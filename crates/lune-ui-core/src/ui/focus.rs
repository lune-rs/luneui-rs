//! Focus (D15): the focused id, focus-visible, focus scopes (the base scope plus one per active
//! overlay, keyed by the overlay's hit tag), Tab order from the previous frame's tab stops, initial
//! focus on open, restore on close and the vanish rule.

use std::collections::BTreeMap;

use crate::input::{FocusEvent, Key, NamedKey};

use super::pointer::HitRecord;
use super::{Ui, UiStore, WidgetId};

/// The window's focus, retained across frames.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FocusState {
    pub(crate) focused: Option<WidgetId>,
    /// Whether the ring shows (keyboard origin).
    pub(crate) visible: bool,
    /// Whether the last input was a key press (true) or a pointer press (false).
    pub(crate) keyboard: bool,
    /// A `request_focus` or `rove_focus` to apply at the next frame's begin.
    pub(crate) pending: Option<WidgetId>,
    /// The ring `pending` leaves: on after `rove_focus`, off after `request_focus`.
    pub(crate) pending_ring: bool,
    /// The children of each roving composite, as `rove_keys` lists them; `begin` reads the
    /// previous frame's.
    pub(crate) roves: Vec<Vec<WidgetId>>,
    /// The keep-alive scopes open when the focused id last registered.
    pub(crate) chain: Vec<WidgetId>,
    /// The id scopes open around the focused widget when it last registered, outermost first,
    /// counted from its overlay's content (the whole stack on the page).
    pub(crate) scope_path: Vec<WidgetId>,
    /// The widget whose IME composition is open (`Ui::set_composing`); only counts while it is
    /// the focused widget.
    pub(crate) composing: Option<WidgetId>,
}

/// Which arrows a roving composite takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoveAxis {
    /// Up and Down.
    Vertical,
    /// Left and Right.
    Horizontal,
    /// All four.
    Both,
}

/// Where a roving key goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoveStep {
    Previous,
    Next,
    First,
    Last,
}

impl RoveStep {
    fn of(key: NamedKey, axis: RoveAxis) -> Option<RoveStep> {
        let (vertical, horizontal) = (axis != RoveAxis::Horizontal, axis != RoveAxis::Vertical);
        match key {
            NamedKey::ArrowUp if vertical => Some(RoveStep::Previous),
            NamedKey::ArrowDown if vertical => Some(RoveStep::Next),
            NamedKey::ArrowLeft if horizontal => Some(RoveStep::Previous),
            NamedKey::ArrowRight if horizontal => Some(RoveStep::Next),
            NamedKey::Home => Some(RoveStep::First),
            NamedKey::End => Some(RoveStep::Last),
            _ => None,
        }
    }

    /// The enabled child this step reaches from `at`, if any.
    fn target(self, items: &[(WidgetId, bool)], at: usize, wrap: bool) -> Option<usize> {
        let n = items.len();
        let on = |i: &usize| items[*i].1;
        match (self, wrap) {
            (RoveStep::First, _) => (0..n).find(on),
            (RoveStep::Last, _) => (0..n).rev().find(on),
            (RoveStep::Next, true) => (1..n).map(|d| (at + d) % n).find(on),
            (RoveStep::Next, false) => (at + 1..n).find(on),
            (RoveStep::Previous, true) => (1..n).map(|d| (at + n - d) % n).find(on),
            (RoveStep::Previous, false) => (0..at).rev().find(on),
        }
    }
}

/// The tab stops of the scope `tag` in the previous frame, each id once: by ascending
/// [`Ui::tab_scope`] order, and in call order among stops of one order.
///
/// `current` (the focus) is listed where its region is, also when that is no stop, so [`step`]
/// continues from it. Focus can reach a child of a roving composite after the frame that
/// registered `hits`, when another child is still the stop there: `roves` names the composite, and
/// `current` stands in for all of it, so the composite stays one stop whichever side of `current`
/// the old stop is on.
pub(crate) fn tab_stops(
    hits: &[HitRecord],
    tag: u32,
    eligible: &dyn Fn(&HitRecord) -> bool,
    current: Option<WidgetId>,
    roves: &[Vec<WidgetId>],
) -> Vec<WidgetId> {
    let group = current.and_then(|c| roves.iter().find(|g| g.contains(&c)));
    let mut stops: Vec<(i32, WidgetId)> = Vec::new();
    for h in hits {
        let stop = if Some(h.id) == current {
            h.enabled || h.sense.skip_tab
        } else {
            h.enabled && !h.sense.skip_tab && !group.is_some_and(|g| g.contains(&h.id))
        };
        if stop
            && h.tag == tag
            && h.sense.focusable
            && eligible(h)
            && !stops.iter().any(|(_, id)| *id == h.id)
        {
            stops.push((h.order, h.id));
        }
    }
    // A stable sort: the stops of one order keep their call order.
    stops.sort_by_key(|(order, _)| *order);
    stops.into_iter().map(|(_, id)| id).collect()
}

/// The stop after (or before) `current` in `stops`, wrapping; the first (or last) when `current`
/// is not a stop.
pub(crate) fn step(stops: &[WidgetId], current: Option<WidgetId>, back: bool) -> Option<WidgetId> {
    let n = stops.len();
    if n == 0 {
        return None;
    }
    let at = current.and_then(|c| stops.iter().position(|s| *s == c));
    let i = match (at, back) {
        (Some(i), false) => (i + 1) % n,
        (Some(i), true) => (i + n - 1) % n,
        (None, false) => 0,
        (None, true) => n - 1,
    };
    Some(stops[i])
}

/// On an overlay's close: when focus is inside it (or gone), focus returns to the id saved at
/// open if it registered this frame, else to the anchor if it did, else nowhere. Focus in a later
/// overlay that is still called is not inside: popping a scope that is not the active one never
/// moves focus. A later overlay that closes this frame too (a child) counts as inside.
/// `registered` maps this frame's ids to their highest hit tag; `live` lists the `(tag, modal)` of
/// the overlays called this frame. A restore target under a live modal is never taken.
pub(crate) fn restore_on_close(
    store: &mut UiStore,
    registered: &BTreeMap<WidgetId, u32>,
    live: &[(u32, bool)],
    tag: u32,
    saved: Option<WidgetId>,
    anchor: Option<WidgetId>,
) {
    let called = |t: u32| live.iter().any(|(l, _)| *l == t);
    let inside = match store.focus.focused {
        None => true,
        Some(f) => registered
            .get(&f)
            .is_none_or(|t| *t == tag || (*t > tag && !called(*t))),
    };
    if !inside {
        return;
    }
    let under_modal = |t: u32| live.iter().any(|(l, modal)| *modal && *l > t);
    store.focus.focused = [saved, anchor]
        .into_iter()
        .flatten()
        .find(|id| registered.get(id).is_some_and(|t| !under_modal(*t)));
}

impl Ui<'_> {
    /// Says whether `id` is composing text (an open IME preedit). While the focused widget
    /// composes, the next frames' Escape and Tab are not handled by the frame: they join the focus
    /// event queue instead. Cleared when focus moves off `id`.
    pub fn set_composing(&mut self, id: WidgetId, composing: bool) {
        let focus = &mut self.store.focus;
        if composing {
            focus.composing = Some(id);
        } else if focus.composing == Some(id) {
            focus.composing = None;
        }
    }

    /// Whether the focused widget is composing, so the keys left in the queue are the input
    /// method's.
    pub(crate) fn composing(&self) -> bool {
        let focus = &self.store.focus;
        focus.composing.is_some() && focus.composing == focus.focused
    }

    /// Focuses `id` at the next frame's begin, without the ring.
    pub fn request_focus(&mut self, id: WidgetId) {
        self.store.focus.pending = Some(id);
        self.store.focus.pending_ring = false;
    }

    /// [`Self::request_focus`] of keyboard origin: the ring shows after it.
    pub fn rove_focus(&mut self, id: WidgetId) {
        self.store.focus.pending = Some(id);
        self.store.focus.pending_ring = true;
    }

    /// The child of a roving composite that is its tab stop this frame, given its `(id, enabled)`
    /// children in order: the focused enabled child, else `active` if enabled, else the first
    /// enabled, else none. Register that child as a stop and the others with [`Sense::CLICK_ROVE`](
    /// super::Sense::CLICK_ROVE).
    pub fn rove_stop(&self, items: &[(WidgetId, bool)], active: Option<usize>) -> Option<usize> {
        let focused = self.store.focus.focused;
        items
            .iter()
            .position(|(id, enabled)| *enabled && Some(*id) == focused)
            .or(active.filter(|a| items.get(*a).is_some_and(|c| c.1)))
            .or_else(|| items.iter().position(|c| c.1))
    }

    /// Moves focus inside a roving composite: while a child is focused, takes the arrows of `axis`,
    /// Home and End (not a Control, Alt or Command chord, which stays a shortcut), skips disabled
    /// children, calls [`Self::rove_focus`] on the new child and returns its index. A key that
    /// finds no move is still taken. Call it every frame: the Tab order reads the list it was
    /// given.
    pub fn rove_keys(
        &mut self,
        items: &[(WidgetId, bool)],
        axis: RoveAxis,
        wrap: bool,
    ) -> Option<usize> {
        self.store
            .focus
            .roves
            .push(items.iter().map(|c| c.0).collect());
        let focused = self.store.focus.focused?;
        let from = items.iter().position(|c| c.0 == focused)?;
        let mut at = from;
        for (index, event) in self.focus_events(focused) {
            let FocusEvent::Key {
                key: Key::Named(key),
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            if modifiers.control || modifiers.alt || modifiers.logo {
                continue;
            }
            if let Some(step) = RoveStep::of(key, axis) {
                self.consume_focus_event(index);
                at = step.target(items, at, wrap).unwrap_or(at);
            }
        }
        if at == from {
            return None;
        }
        self.rove_focus(items[at].0);
        Some(at)
    }

    /// After a newly opened overlay's content: focus its `initial` stop if it registered one,
    /// else its first stop. With no stop, a modal overlay focuses nothing and a non-modal one
    /// leaves focus where it is (D82: a passive overlay such as a toast takes no focus). The ring
    /// follows the last input's origin.
    pub(crate) fn initial_focus(
        &mut self,
        tag: u32,
        first_hit: usize,
        initial: Option<WidgetId>,
        modal: bool,
    ) {
        let meta = self.painter.hit_meta();
        let mut stops = Vec::new();
        for r in &self.registered {
            let stop = r.sense.focusable && !r.sense.skip_tab && r.enabled;
            if r.index >= first_hit && stop && meta[r.index].1 == tag {
                stops.push(self.painter.hits[r.index].1);
            }
        }
        let pick = initial
            .filter(|i| stops.contains(i))
            .or_else(|| stops.first().copied());
        if pick.is_none() && !modal {
            return;
        }
        self.store.focus.focused = pick;
        self.store.focus.visible = self.store.focus.keyboard;
        if pick.is_some() {
            self.store.focus.chain = self.alive.clone();
        }
    }
}

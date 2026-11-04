//! Focus (D15): the focused id, focus-visible, focus scopes (the base scope plus one per active
//! overlay, keyed by the overlay's hit tag), Tab order from the previous frame's tab stops, initial
//! focus on open, restore on close and the vanish rule.

use std::collections::BTreeMap;

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
    /// A `request_focus` to apply at the next frame's begin.
    pub(crate) pending: Option<WidgetId>,
    /// The keep-alive scopes open when the focused id last registered.
    pub(crate) chain: Vec<WidgetId>,
}

/// The tab stops of the scope `tag` in the previous frame, in call order, each id once.
pub(crate) fn tab_stops(
    hits: &[HitRecord],
    tag: u32,
    eligible: &dyn Fn(&HitRecord) -> bool,
) -> Vec<WidgetId> {
    let mut stops: Vec<WidgetId> = Vec::new();
    for h in hits {
        if h.tag == tag && h.sense.focusable && h.enabled && eligible(h) && !stops.contains(&h.id) {
            stops.push(h.id);
        }
    }
    stops
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

/// On an overlay's close: when focus is inside it (or gone with it), focus returns to the id
/// saved at open if it registered this frame, else to the anchor if it did, else nowhere.
/// `registered` maps this frame's ids to their highest hit tag.
pub(crate) fn restore_on_close(
    store: &mut UiStore,
    registered: &BTreeMap<WidgetId, u32>,
    tag: u32,
    saved: Option<WidgetId>,
    anchor: Option<WidgetId>,
) {
    let inside = match store.focus.focused {
        None => true,
        Some(f) => registered.get(&f).is_none_or(|t| *t >= tag),
    };
    if !inside {
        return;
    }
    store.focus.focused = [saved, anchor]
        .into_iter()
        .flatten()
        .find(|id| registered.contains_key(id));
}

impl Ui<'_> {
    /// Focuses `id` at the next frame's begin, without the ring.
    pub fn request_focus(&mut self, id: WidgetId) {
        self.store.focus.pending = Some(id);
    }

    /// After a newly opened overlay's content: focus its `initial` stop if it registered one,
    /// else its first stop, else nothing. The ring follows the last input's origin.
    pub(crate) fn initial_focus(&mut self, tag: u32, first_hit: usize, initial: Option<WidgetId>) {
        let meta = self.painter.hit_meta();
        let mut stops = Vec::new();
        for r in &self.registered {
            if r.index >= first_hit && r.sense.focusable && r.enabled && meta[r.index].1 == tag {
                stops.push(self.painter.hits[r.index].1);
            }
        }
        let pick = initial
            .filter(|i| stops.contains(i))
            .or_else(|| stops.first().copied());
        self.store.focus.focused = pick;
        self.store.focus.visible = self.store.focus.keyboard;
        if pick.is_some() {
            self.store.focus.chain = self.alive.clone();
        }
    }
}

//! The per-window `Ui` frame: stable widget ids under scopes, a retained store with transient and
//! keep-alive state, layered hit testing with pointer capture, [`Response`], animated values, the
//! overlay stack with modal capture, dismissal and exit retention, focus scopes, key routing,
//! semantic nodes with accessibility actions, strings, and the [`FrameOutput`] (with [`Repaint`])
//! a frame returns.
//!
//! A frame: [`Ui::begin`] resolves this frame's events against the previous frame's hit regions,
//! overlays, tab stops and semantic tree (D4), widgets call [`Ui::interact`] and draw through
//! [`Ui::painter`], and [`Ui::finish`] closes the overlays the app stopped calling (drawing their
//! exit frames), turns this frame's regions into the next frame's, drops unused state and returns
//! the `Scene` and the `FrameOutput`. Hits from `interact` and from raw `Painter::hit` calls share
//! one ordered list, so widgets of both APIs mix in one frame; `painter.hover` is the hovered id
//! (D3).

mod access;
mod animate;
mod focus;
mod id;
mod keys;
mod overlay;
mod pointer;
mod response;
mod store;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Display;
use std::ops::Range;

pub use id::WidgetId;
pub use overlay::{Dismissal, OverlayFrame, OverlayOptions, OverlayPhase};
pub use response::{Response, Sense};
pub use store::UiStore;

use crate::access::{AccessValue, SemanticTree};
use crate::input::{ClipboardRequest, Event, FrameOutput, Key, NamedKey, RawInput, Repaint};
use crate::motion::{MotionTier, Time};
use crate::paint::{Painter, Rect, Xform};
use crate::scene::Scene;
use crate::strings::{ArgFormat, StringTable};
use crate::text::TextSystem;
use crate::theme::Theme;

use access::Routed;
use keys::KeyPress;
use overlay::{Dismissal as D, SUPPRESSED};
use pointer::{Ctx, HitRecord, PointerFrame, topmost};

/// The opacity of a disabled subtree.
const DISABLED_ALPHA: f32 = 0.4;

/// What the app hands a frame besides input and text: the look, the motion tier and the strings.
#[derive(Clone, Copy)]
pub struct FrameConfig<'a> {
    pub theme: Theme,
    pub tier: MotionTier,
    pub strings: &'a StringTable,
    pub arg_format: &'a dyn ArgFormat,
}

impl std::fmt::Debug for FrameConfig<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameConfig")
            .field("theme", &self.theme)
            .field("tier", &self.tier)
            .field("strings", &self.strings)
            .finish_non_exhaustive()
    }
}

/// A hit registered through [`Ui::interact`]: its index in the painter's hit list.
#[derive(Debug, Clone, Copy)]
struct Registration {
    index: usize,
    sense: Sense,
    enabled: bool,
}

/// An active overlay as the previous frame left it, for begin's input rules.
#[derive(Debug, Clone, Copy)]
struct Active {
    id: WidgetId,
    tag: u32,
    opts: OverlayOptions,
    /// Modal, or registered a focusable region in the previous frame. A passive overlay (a toast
    /// with no control) is the top for Escape or an outside press only when it asks for that
    /// dismissal (D82); otherwise they go to the overlay below it.
    takes_input: bool,
}

/// The `(tag, layer)` of the topmost active modal overlay, which gates eligibility.
fn modal_gate<'o>(
    overlays: impl DoubleEndedIterator<Item = &'o overlay::OverlayRec>,
) -> Option<(u32, usize)> {
    overlays
        .rev()
        .find(|o| o.active() && o.opts.modal)
        .map(|o| (o.tag, o.opts.layer as usize))
}

/// One window's frame. Built by [`Ui::begin`], consumed by [`Ui::finish`].
pub struct Ui<'a> {
    store: &'a mut UiStore,
    input: &'a RawInput,
    config: FrameConfig<'a>,
    painter: Painter<'a, WidgetId>,
    /// The id scope stack; the last entry is the parent of [`Ui::id`].
    scopes: Vec<WidgetId>,
    /// The keep-alive scopes open around the current call, outermost first.
    alive: Vec<WidgetId>,
    enabled: bool,
    pointer: PointerFrame,
    registered: Vec<Registration>,
    /// Ranges of painter hit indices registered while disabled.
    disabled: Vec<Range<usize>>,
    in_flight: bool,
    repaint_at: Option<Time>,
    clipboard_write: Option<String>,
    clipboard_read: bool,
    /// Dismiss requests found at begin, handed out by `overlay`.
    dismissed: BTreeMap<WidgetId, Dismissal>,
    /// Overlays pushed or reopened this frame.
    opened: Vec<WidgetId>,
    keys: Vec<KeyPress>,
    /// The focused widget of the active scope, which may take keys.
    key_focus: Option<WidgetId>,
    set_values: BTreeMap<WidgetId, AccessValue>,
    scroll_into_view: BTreeSet<WidgetId>,
    tree: SemanticTree,
    /// Indices in `tree` of the open containers.
    containers: Vec<usize>,
    node_meta: BTreeMap<WidgetId, (usize, u32)>,
    /// This frame's input changed state the frame drew before reacting to (a click, a consumed
    /// key, an access action, a dismissal, a scroll or drag): the next frame is needed (D81).
    input_acted: bool,
    /// `(focused, visible)` once begin applied the frame's input.
    focus_at_begin: (Option<WidgetId>, bool),
}

impl std::fmt::Debug for Ui<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ui")
            .field("frame", &self.store.frame)
            .field("time", &self.input.time)
            .field("scopes", &self.scopes)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

/// Whether focus moving to `to` is focus loss for the active overlay tagged `tag`: it asks for
/// that dismissal and `to` has no region in `hits` inside it or in an overlay opened above it.
/// The one containment test, shared by begin (which dismisses) and finish's catch-up (which asks
/// for the frame where begin would), so the two cannot drift.
fn loses_focus(opts: &OverlayOptions, tag: u32, hits: &[HitRecord], to: Option<WidgetId>) -> bool {
    opts.dismiss_on_focus_loss
        && !to.is_some_and(|id| hits.iter().any(|h| h.id == id && h.tag >= tag))
}

/// Dismisses, for focus loss, every active overlay that asks for it and does not hold `to`.
fn focus_loss(
    active: &[Active],
    hits: &[HitRecord],
    to: Option<WidgetId>,
    dismissed: &mut BTreeMap<WidgetId, Dismissal>,
) {
    for o in active
        .iter()
        .filter(|o| loses_focus(&o.opts, o.tag, hits, to))
    {
        dismissed.entry(o.id).or_insert(D::FocusLoss);
    }
}

impl<'a> Ui<'a> {
    /// Starts a frame: resolves `input`'s events against the previous frame's hit regions,
    /// overlays, tab stops and semantic tree.
    pub fn begin(
        store: &'a mut UiStore,
        input: &'a RawInput,
        config: FrameConfig<'a>,
        text: &'a mut dyn TextSystem,
    ) -> Ui<'a> {
        if store.last_tier.is_some_and(|t| t != config.tier) {
            for a in store.anim_f32.values_mut() {
                let t = a.transition.target();
                a.transition.snap(t);
            }
            for a in store.anim_color.values_mut() {
                let t = a.transition.target();
                a.transition.snap(t);
            }
            // D12: everything in flight snaps, overlays too.
            store.overlays.retain(|o| o.active());
            for o in &mut store.overlays {
                o.phase = overlay::RecPhase::Open;
            }
        }
        let active: Vec<Active> = store
            .overlays
            .iter()
            .filter(|o| o.active())
            .map(|o| Active {
                id: o.id,
                tag: o.tag,
                opts: o.opts,
                takes_input: o.opts.modal
                    || store
                        .prev_hits
                        .iter()
                        .any(|h| h.tag == o.tag && h.sense.focusable),
            })
            .collect();
        let top_for = |asks: fn(&OverlayOptions) -> bool| {
            active
                .iter()
                .rev()
                .find(|o| o.takes_input || asks(&o.opts))
                .copied()
        };
        let top_escape = top_for(|o| o.dismiss_on_escape);
        let top_press = top_for(|o| o.dismiss_on_outside_press);
        let modal = modal_gate(store.overlays.iter());
        let eligible = move |h: &HitRecord| modal.is_none_or(|(t, l)| h.tag >= t || h.layer > l);
        let eligible_at =
            move |(layer, tag): (usize, u32)| modal.is_none_or(|(t, l)| tag >= t || layer > l);

        let hits = std::mem::take(&mut store.prev_hits);
        let mut pointer = PointerFrame::default();
        let mut dismissed = BTreeMap::new();
        let mut routed = Routed::default();
        let mut keys = Vec::new();
        let mut acted = false;

        if let Some(id) = store.focus.pending.take() {
            store.focus.focused = Some(id);
            store.focus.visible = false;
            focus_loss(&active, &hits, Some(id), &mut dismissed);
        }
        store
            .pointer
            .drop_capture_if(|id| !hits.iter().any(|h| h.id == id && eligible(h)));
        {
            let cx = Ctx {
                hits: &hits,
                eligible: &eligible,
                time: input.time,
            };
            for ev in &input.events {
                match ev {
                    Event::PointerMoved { pos } => store.pointer.moved(*pos, &cx, &mut pointer),
                    Event::PointerLeft => store.pointer.left(),
                    Event::PointerButton {
                        pos,
                        button: button @ crate::input::PointerButton::Primary,
                        pressed: true,
                        ..
                    } => {
                        let target = topmost(&hits, *pos, &eligible);
                        store.focus.visible = false;
                        store.focus.keyboard = false;
                        if let Some(t) = top_press
                            && t.opts.dismiss_on_outside_press
                            && target.is_none_or(|h| h.tag < t.tag)
                        {
                            // D22: the press that dismisses reaches nothing.
                            dismissed.entry(t.id).or_insert(D::OutsidePress);
                            store.pointer.press_consumed(*pos);
                            continue;
                        }
                        store.pointer.button(*pos, *button, true, &cx, &mut pointer);
                        if let Some(h) = target.filter(|h| h.enabled && h.sense.focusable) {
                            store.focus.focused = Some(h.id);
                        }
                    }
                    Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        ..
                    } => {
                        store
                            .pointer
                            .button(*pos, *button, *pressed, &cx, &mut pointer);
                    }
                    Event::Scroll { pos, delta, .. } => {
                        store.pointer.scroll(*pos, *delta, &cx, &mut pointer);
                    }
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        store.focus.keyboard = true;
                        match key {
                            Key::Named(NamedKey::Escape)
                                if top_escape.is_some_and(|t| t.opts.dismiss_on_escape) =>
                            {
                                if let Some(t) = top_escape {
                                    dismissed.entry(t.id).or_insert(D::Escape);
                                }
                            }
                            Key::Named(NamedKey::Tab) => {
                                let scopes = active.iter().rev().map(|o| (o.tag, o.opts.modal));
                                let mut to = None;
                                for (tag, trap) in scopes.chain([(0, true)]) {
                                    let stops = focus::tab_stops(&hits, tag, &eligible);
                                    if !stops.is_empty() || trap {
                                        to = focus::step(
                                            &stops,
                                            store.focus.focused,
                                            modifiers.shift,
                                        );
                                        break;
                                    }
                                }
                                if to.is_some() {
                                    store.focus.focused = to;
                                }
                                store.focus.visible = true;
                            }
                            Key::Named(k) => keys.push(KeyPress {
                                key: *k,
                                target: None,
                                consumed: false,
                            }),
                            _ => {}
                        }
                    }
                    Event::Focus(false) => {
                        focus_loss(&active, &hits, None, &mut dismissed);
                    }
                    Event::ClipboardText(_) | Event::ClipboardUnavailable => acted = true,
                    Event::Access(a) => {
                        let Some(node) = store.prev_tree.get(a.target) else {
                            continue;
                        };
                        let meta = store.prev_node_meta.get(&a.target).copied();
                        if !meta.is_some_and(eligible_at) {
                            continue;
                        }
                        acted = true;
                        let top_escape = top_escape.map(|t| (t.id, t.opts.dismiss_on_escape));
                        let focus_to = access::route(
                            a,
                            node,
                            top_escape,
                            &mut pointer,
                            &mut routed,
                            &mut dismissed,
                        );
                        if let Some(id) = focus_to {
                            store.focus.focused = Some(id);
                            store.focus.visible = false;
                            focus_loss(&active, &hits, Some(id), &mut dismissed);
                        }
                    }
                    _ => {}
                }
            }
            store.pointer.settle(&cx, &mut pointer);
        }
        acted |= !dismissed.is_empty() || pointer.acted();
        keys.extend(routed.keys);
        let key_focus = store.focus.focused.filter(|id| {
            let mut own = hits.iter().filter(|h| h.id == *id).peekable();
            if own.peek().is_none() {
                modal.is_none()
            } else {
                own.any(eligible)
            }
        });
        store.prev_hits = hits;

        let focus_at_begin = (store.focus.focused, store.focus.visible);
        let window = Rect::new(0.0, 0.0, input.size.0, input.size.1);
        let painter = Painter::new(input.scale, window, text, pointer.hovered, config.theme);
        Ui {
            store,
            input,
            config,
            painter,
            scopes: vec![WidgetId::ROOT],
            alive: Vec::new(),
            enabled: true,
            pointer,
            registered: Vec::new(),
            disabled: Vec::new(),
            in_flight: false,
            repaint_at: None,
            clipboard_write: None,
            clipboard_read: false,
            dismissed,
            opened: Vec::new(),
            keys,
            key_focus,
            set_values: routed.set_values,
            scroll_into_view: routed.scroll_into_view,
            tree: SemanticTree::default(),
            containers: Vec::new(),
            node_meta: BTreeMap::new(),
            input_acted: acted,
            focus_at_begin,
        }
    }

    /// Ends the frame: closes the overlays the app did not call and draws their exit frames,
    /// turns this frame's hit regions into the next frame's and drops unused state.
    pub fn finish(self) -> (Scene, FrameOutput) {
        let Ui {
            store,
            input,
            config,
            mut painter,
            registered,
            disabled,
            in_flight,
            repaint_at,
            clipboard_write,
            clipboard_read,
            mut tree,
            node_meta,
            pointer,
            input_acted,
            focus_at_begin,
            ..
        } = self;
        let now = input.time;
        let mut ids: BTreeMap<WidgetId, u32> = BTreeMap::new();
        for (&(_, id), &(_, tag)) in painter.hits.iter().zip(painter.hit_meta()) {
            let t = ids.entry(id).or_insert(tag);
            *t = (*t).max(tag);
        }
        let moving = Ui::settle_overlays(
            store,
            &mut painter,
            &ids,
            now,
            &config.theme.motion,
            config.tier,
        );

        let mut senses: Vec<Option<(Sense, bool)>> = vec![None; painter.hits.len()];
        for r in &registered {
            senses[r.index] = Some((r.sense, r.enabled));
        }
        let mut hits = Vec::with_capacity(painter.hits.len());
        let mut rects = BTreeMap::new();
        for (i, (&(rect, id), &(layer, tag))) in
            painter.hits.iter().zip(painter.hit_meta()).enumerate()
        {
            if tag == SUPPRESSED {
                continue;
            }
            let (sense, enabled) = senses
                .get(i)
                .copied()
                .flatten()
                .unwrap_or_else(|| (Sense::CLICK, !disabled.iter().any(|d| d.contains(&i))));
            hits.push(HitRecord {
                id,
                rect,
                layer,
                sense,
                tag,
                enabled,
            });
            rects.insert(id, rect);
        }

        // D15: a focused id that did not register clears, unless a live keep-alive scope holds it.
        if let Some(f) = store.focus.focused
            && !ids.contains_key(&f)
            && !store
                .focus
                .chain
                .iter()
                .any(|s| store.live_scopes.contains(s))
        {
            store.focus.focused = None;
        }
        // D81: a frame that acted on input, or whose focus changed after its widgets drew, or
        // whose hover at the still pointer differs against its own new regions, drew a picture
        // the next frame corrects: ask for it. The hover catch-up never repeats on a frame with
        // no events, so a region that moves with its own hover cannot loop.
        let stale_hover = {
            let gate = modal_gate(store.overlays.iter());
            let eligible = move |h: &HitRecord| gate.is_none_or(|(t, l)| h.tag >= t || h.layer > l);
            let cx = Ctx {
                hits: &hits,
                eligible: &eligible,
                time: now,
            };
            let mut next = PointerFrame::default();
            store.pointer.settle(&cx, &mut next);
            next.hovered != pointer.hovered && !(store.hover_catch_up && input.events.is_empty())
        };
        store.hover_catch_up = stale_hover;
        // A pending focus request counts iff applying it at the next begin changes something:
        // another widget, a ring to hide, or a focus-loss overlay to dismiss. The last is begin's
        // own test over the same active set and the regions it will see (`hits` becomes its
        // `prev_hits`).
        let request_acts = store.focus.pending.is_some_and(|p| {
            store.focus.focused != Some(p)
                || store.focus.visible
                || store
                    .overlays
                    .iter()
                    .filter(|o| o.active())
                    .any(|o| loses_focus(&o.opts, o.tag, &hits, Some(p)))
        });
        let catch_up = input_acted
            || stale_hover
            || request_acts
            || (store.focus.focused, store.focus.visible) != focus_at_begin;

        let focused = store.focus.focused;
        access::mark_focus(&mut tree, focused);
        let out_focused = focused.filter(|id| tree.get(*id).is_some());

        store.prev_hits = hits;
        store.prev_rects = rects;
        store.prev_tree = tree.clone();
        store.prev_node_meta = node_meta;
        store.collect(store.frame);
        store.frame += 1;
        store.last_tier = Some(config.tier);

        let repaint = if in_flight || moving || catch_up {
            Repaint::Now
        } else if let Some(t) = repaint_at.filter(|t| *t > now) {
            Repaint::At(t)
        } else {
            Repaint::Idle
        };
        let mut clipboard = Vec::new();
        if let Some(text) = clipboard_write {
            clipboard.push(ClipboardRequest::Write(text));
        }
        if clipboard_read {
            clipboard.push(ClipboardRequest::Read);
        }
        let output = FrameOutput {
            repaint,
            clipboard,
            tree,
            focused: out_focused,
            ..FrameOutput::default()
        };
        (painter.scene, output)
    }

    /// The text for `key`: the string table's, else the toolkit default, else the key. Borrows
    /// the table and the key, not the frame.
    pub fn tr<'k>(&self, key: &'k str) -> &'k str
    where
        'a: 'k,
    {
        let table: &'k StringTable = self.config.strings;
        table.tr(key)
    }

    /// [`Self::tr`] with the placeholders filled from `args` through the frame's `ArgFormat`.
    pub fn tr_fmt(&self, key: &str, args: &[&dyn Display]) -> String {
        self.config
            .strings
            .tr_fmt(key, args, self.config.arg_format)
    }

    /// The painter this frame draws with; old-API widgets take it directly.
    pub fn painter(&mut self) -> &mut Painter<'a, WidgetId> {
        &mut self.painter
    }

    pub fn theme(&self) -> &Theme {
        &self.config.theme
    }

    /// The app's time for this frame.
    pub fn time(&self) -> Time {
        self.input.time
    }

    pub fn tier(&self) -> MotionTier {
        self.config.tier
    }

    /// The frame's raw input (for events a widget handles itself, such as clipboard replies).
    pub fn input(&self) -> &'a RawInput {
        self.input
    }

    /// The id of `key` under the current scope.
    pub fn id(&self, key: impl std::hash::Hash) -> WidgetId {
        self.parent().with(key)
    }

    fn parent(&self) -> WidgetId {
        self.scopes.last().copied().unwrap_or(WidgetId::ROOT)
    }

    /// Runs `f` with ids nested under `key`.
    pub fn scope<R>(&mut self, key: impl std::hash::Hash, f: impl FnOnce(&mut Self) -> R) -> R {
        let id = self.id(key);
        self.scopes.push(id);
        let r = f(self);
        self.scopes.pop();
        r
    }

    /// Opens the keep-alive scope `key` (if it is not open) and runs `f` inside it, with ids
    /// nested under it. The scope stays live across frames, drawn or not, until
    /// [`Self::end_keep_alive`]; persistent state used inside survives while it is live (D6).
    pub fn keep_alive<R>(
        &mut self,
        key: impl std::hash::Hash,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let id = self.id(key);
        self.store.live_scopes.insert(id);
        self.scopes.push(id);
        self.alive.push(id);
        let r = f(self);
        self.alive.pop();
        self.scopes.pop();
        r
    }

    /// Opens the keep-alive scope `key` without drawing inside it.
    pub fn mark_alive(&mut self, key: impl std::hash::Hash) {
        let id = self.id(key);
        self.store.live_scopes.insert(id);
    }

    /// Ends the keep-alive scope `key`: state kept only by it is dropped at this frame's finish
    /// unless used this frame.
    pub fn end_keep_alive(&mut self, key: impl std::hash::Hash) {
        let id = self.id(key);
        self.store.live_scopes.remove(&id);
    }

    /// Registers `rect` as a hit region of `id` on the painter's current layer and returns the
    /// frame's reading of it. The region is clipped and transformed like `Painter::hit`.
    pub fn interact(&mut self, id: WidgetId, rect: Rect, sense: Sense) -> Response {
        let index = self.painter.hits.len();
        self.painter.hit(rect, id);
        let rect = self.painter.hits[index].0;
        self.registered.push(Registration {
            index,
            sense,
            enabled: self.enabled,
        });
        if !self.enabled {
            return Response::inert(id, rect, false);
        }
        let focus = &mut self.store.focus;
        if focus.focused == Some(id) && focus.chain.as_slice() != self.alive.as_slice() {
            focus.chain = self.alive.clone();
        }
        let p = &self.pointer;
        let mut r = Response::inert(id, rect, true);
        r.hovered = p.hovered == Some(id);
        r.pressed = p.pressed == Some(id);
        r.clicked = p.clicked.contains(&id);
        r.double_clicked = p.double_clicked.contains(&id);
        r.secondary_clicked = p.secondary_clicked.contains(&id);
        r.drag_started = p.drag_started.contains(&id);
        r.dragged = p.dragging == Some(id);
        r.drag_delta = p.drag_delta.get(&id).copied().unwrap_or((0.0, 0.0));
        r.drag_stopped = p.drag_stopped.contains(&id);
        r.scroll_lines = p.scroll_lines.get(&id).copied().unwrap_or((0.0, 0.0));
        r.scroll_pixels = p.scroll_pixels.get(&id).copied().unwrap_or((0.0, 0.0));
        r.focused = self.store.focus.focused == Some(id);
        r.focus_visible = r.focused && self.store.focus.visible && self.input.focused;
        r.set_value = self.set_values.get(&id).cloned();
        r.scroll_into_view = self.scroll_into_view.contains(&id);
        r
    }

    /// State of type `T` for `id`, dropped at the finish of the first frame that does not use it.
    pub fn transient<T: Default + 'static>(&mut self, id: WidgetId) -> &mut T {
        let frame = self.store.frame;
        UiStore::entry(&mut self.store.transient, id, frame, &[])
    }

    /// State of type `T` for `id` that also survives undrawn frames while a keep-alive scope open
    /// around its last use is live.
    pub fn persistent<T: Default + 'static>(&mut self, id: WidgetId) -> &mut T {
        let frame = self.store.frame;
        UiStore::entry(&mut self.store.persistent, id, frame, &self.alive)
    }

    /// Asks for a frame at app time `t`; the earliest request later than now wins. A request at
    /// or before the frame's time is due already and is ignored (it never hides a later one).
    pub fn request_repaint_at(&mut self, t: Time) {
        if t <= self.input.time {
            return;
        }
        self.repaint_at = Some(self.repaint_at.map_or(t, |r| r.min(t)));
    }

    /// Runs `f` enabled or not. Disabled, every `interact` inside still registers (and blocks) but
    /// reports nothing, and the painter draws at opacity 0.4.
    pub fn with_enabled<R>(&mut self, enabled: bool, f: impl FnOnce(&mut Self) -> R) -> R {
        if enabled || !self.enabled {
            return f(self);
        }
        self.enabled = false;
        let start = self.painter.hits.len();
        self.painter.push_xform(Xform {
            origin: (0.0, 0.0),
            scale: 1.0,
            dy: 0.0,
            alpha: DISABLED_ALPHA,
        });
        let r = f(self);
        self.painter.pop_xform();
        self.disabled.push(start..self.painter.hits.len());
        self.enabled = true;
        r
    }

    /// Puts `text` on the clipboard; the last write of a frame is the one sent.
    pub fn clipboard_write(&mut self, text: String) {
        self.clipboard_write = Some(text);
    }

    /// Asks for the clipboard's text; the answer arrives as an event on a later frame. Repeated
    /// calls in a frame send one request.
    pub fn clipboard_read(&mut self) {
        self.clipboard_read = true;
    }
}

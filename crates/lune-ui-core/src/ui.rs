//! The per-window `Ui` frame: stable widget ids under scopes, a retained store with transient and
//! keep-alive state, layered hit testing with pointer capture, [`Response`], animated values and
//! the [`FrameOutput`] (with [`Repaint`]) a frame returns.
//!
//! A frame: [`Ui::begin`] resolves this frame's events against the previous frame's hit regions
//! (D4), widgets call [`Ui::interact`] and draw through [`Ui::painter`], and [`Ui::finish`] turns
//! this frame's regions into the next frame's, drops unused state and returns the `Scene` and the
//! `FrameOutput`. Hits from `interact` and from raw `Painter::hit` calls share one ordered list, so
//! widgets of both APIs mix in one frame; `painter.hover` is the hovered id (D3).

mod animate;
mod id;
mod pointer;
mod response;
mod store;

use std::ops::Range;

pub use id::WidgetId;
pub use response::{Response, Sense};
pub use store::UiStore;

use crate::input::{ClipboardRequest, Event, FrameOutput, RawInput, Repaint};
use crate::motion::{MotionTier, Time};
use crate::paint::{Painter, Rect, Xform};
use crate::scene::Scene;
use crate::strings::{ArgFormat, StringTable};
use crate::text::TextSystem;
use crate::theme::Theme;

use pointer::{Ctx, HitRecord, PointerFrame};

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

impl<'a> Ui<'a> {
    /// Starts a frame: resolves `input`'s events against the previous frame's hit regions.
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
        }
        let mut pointer = PointerFrame::default();
        {
            let hits = std::mem::take(&mut store.prev_hits);
            let eligible = |_: &HitRecord| true;
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
                    _ => {}
                }
            }
            store.pointer.settle(&cx, &mut pointer);
            store.prev_hits = hits;
        }
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
        }
    }

    /// Ends the frame: this frame's hit regions become the next frame's, unused state is dropped.
    pub fn finish(self) -> (Scene, FrameOutput) {
        let Ui {
            store,
            input,
            config,
            painter,
            registered,
            disabled,
            in_flight,
            repaint_at,
            clipboard_write,
            clipboard_read,
            ..
        } = self;
        let mut senses = vec![Sense::CLICK; painter.hits.len()];
        for r in &registered {
            senses[r.index] = r.sense;
        }
        let mut hits = Vec::with_capacity(painter.hits.len());
        let mut rects = std::collections::BTreeMap::new();
        for (i, (&(rect, id), &(layer, tag))) in
            painter.hits.iter().zip(painter.hit_meta()).enumerate()
        {
            let enabled = !disabled.iter().any(|d| d.contains(&i));
            hits.push(HitRecord {
                id,
                rect,
                layer,
                sense: senses[i],
                tag,
                enabled,
            });
            rects.insert(id, rect);
        }
        store.prev_hits = hits;
        store.prev_rects = rects;
        store.collect(store.frame);
        store.frame += 1;
        store.last_tier = Some(config.tier);

        let now = input.time;
        let repaint = if in_flight {
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
            ..FrameOutput::default()
        };
        (painter.scene, output)
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
        self.registered.push(Registration { index, sense });
        if !self.enabled {
            return Response::inert(id, rect, false);
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

    /// Asks for a frame at app time `t`; the earliest request later than now wins.
    pub fn request_repaint_at(&mut self, t: Time) {
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

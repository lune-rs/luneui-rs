//! Immediate-mode painter: every frame the UI is described from state into per-layer geometry,
//! text placements and hit regions. Clicks resolve against the previous frame's hit regions.

use crate::color::{Rgba, premul};
use crate::image::{Fit, ImageAlign, ImageId};
use crate::scene::{
    Acrylic, BackdropRequest, CustomItem, ImageItem, ImageSource, Layer, LayerGeometry,
    OverlayLayer, RoundClip, Scene, Shape, StrokeVertex, TextItem,
};
use crate::text::{CaretPos, LineSpan, Spec, TextSystem, Unit};
use crate::theme::Theme;

mod path;

pub use crate::scene::{LAYER_COUNT, LayerId};

/// A rectangle by its top-left corner and size, in logical px unless a call says physical.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// The rect at `x`, `y` of size `w` by `h`.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// The x of the right edge.
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// The y of the bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Whether `p` is inside: the left and top edges count, the right and bottom do not.
    pub fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 < self.right() && p.1 >= self.y && p.1 < self.bottom()
    }

    /// The rect shrunk by `dx` on the left and right and `dy` on the top and bottom.
    pub fn inset(&self, dx: f32, dy: f32) -> Self {
        Self::new(
            self.x + dx,
            self.y + dy,
            (self.w - 2.0 * dx).max(0.0),
            (self.h - 2.0 * dy).max(0.0),
        )
    }

    /// The area both rects cover; a rect with zero size when they do not meet.
    pub fn intersect(&self, o: &Rect) -> Rect {
        let x = self.x.max(o.x);
        let y = self.y.max(o.y);
        Rect::new(
            x,
            y,
            (self.right().min(o.right()) - x).max(0.0),
            (self.bottom().min(o.bottom()) - y).max(0.0),
        )
    }

    /// The rect as `[x, y, w, h]` at scale `s`, in physical px.
    pub fn phys(&self, s: f32) -> [f32; 4] {
        [self.x * s, self.y * s, self.w * s, self.h * s]
    }
}

/// [`StrokeStyle::mode`]: the shader's conic sweep (see [`Painter::stroke_sweep`]).
const MODE_SWEEP: f32 = 1.0;
/// [`StrokeStyle::mode`]: the shader's comet (see [`Painter::stroke_comet`]).
const MODE_COMET: f32 = 2.0;

/// How a stroke is drawn. Every length is in logical px of the drawing space, like the points;
/// the painter turns them into the units the shader reads.
///
/// `mode` 0 is solid. Mode 1, the conic sweep: `a` is the phase in turns, `extra[0..2]` the centre.
/// Mode 2, the comet: `a` the dash length, `b` the distance travelled, `extra[2]` the contour.
#[derive(Debug, Clone, Copy)]
pub struct StrokeStyle {
    pub color: [f32; 4],
    pub width: f32,
    pub mode: f32,
    pub a: f32,
    pub b: f32,
    pub extra: [f32; 4],
}

impl StrokeStyle {
    /// A plain stroke of `color` and `width`, no sweep or comet.
    pub fn solid(color: Rgba, width: f32) -> Self {
        Self {
            color: premul(color),
            width,
            mode: 0.0,
            a: 0.0,
            b: 0.0,
            extra: [0.0; 4],
        }
    }
}

/// Animation transform for a popup: scale about `origin`, then move by `dx`, `dy`, and fade.
///
/// Transforms stack ([`Painter::push_xform`]): the innermost applies first, then the next one out.
#[derive(Debug, Clone, Copy)]
pub struct Xform {
    pub origin: (f32, f32),
    pub scale: f32,
    /// Moves right, after the scale.
    pub dx: f32,
    /// Moves down, after the scale.
    pub dy: f32,
    pub alpha: f32,
}

impl Xform {
    /// A pure translation by `dx`, `dy`: scale 1, alpha 1.
    pub fn translate(dx: f32, dy: f32) -> Self {
        Self {
            origin: (0.0, 0.0),
            scale: 1.0,
            dx,
            dy,
            alpha: 1.0,
        }
    }

    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.origin.0 + (x - self.origin.0) * self.scale + self.dx,
            self.origin.1 + (y - self.origin.1) * self.scale + self.dy,
        )
    }

    fn rect(&self, r: Rect) -> Rect {
        let (x, y) = self.point(r.x, r.y);
        Rect::new(x, y, r.w * self.scale, r.h * self.scale)
    }
}

/// One recorded painter call (see [`Painter::start_recording`]).
#[derive(Debug, Clone)]
enum Op {
    On(LayerId),
    SetClip(Option<Rect>),
    PushClip(Rect),
    PushClipRounded(Rect, f32),
    PopClip,
    SetXform(Option<Xform>),
    PushXform(Xform),
    PopXform,
    PushTranslate(f32, f32),
    PopTranslate,
    FillRadii(Rect, [f32; 4], Rgba),
    FillOver(Rect, f32, [f32; 4]),
    Ring(Rect, f32, f32, Rgba, bool),
    Acrylic(Rect, [f32; 4], [f32; 4], [f32; 4], bool),
    Frost(Rect, [f32; 4], [f32; 4], bool),
    Backdrop(Rect, f32, f32, f32),
    Text(Spec, f32, f32),
    Stroke(Vec<[f32; 2]>, bool, StrokeStyle),
    Dash(Vec<[f32; 2]>, bool, f32, f32, f32, Rgba),
    Arc([f32; 2], f32, f32, f32, StrokeStyle),
    FillPath(Vec<[f32; 2]>, Rgba),
    Sector([f32; 2], f32, f32, f32, Rgba),
    Image(Rect, ImageId, Fit, ImageAlign, f32, f32),
    TextRotated(Spec, (f32, f32), f32),
    Custom(usize, Rect),
}

/// The clip a [`Painter::push_clip`] or [`Painter::push_clip_rounded`] replaced.
#[derive(Debug, Clone, Copy)]
struct ClipFrame {
    clip: Option<Rect>,
    round: Option<(Rect, f32)>,
}

/// Painter calls recorded by [`Painter::start_recording`], owned and `'static`, to replay later
/// with [`Painter::replay`] (an overlay's exit frame replays what the app drew inside it).
#[derive(Debug, Clone, Default)]
pub struct Ops {
    ops: Vec<Op>,
}

impl Ops {
    /// The number of recorded calls.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Whether nothing was recorded.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// The hit tag of exit painters: their hits are dropped.
pub(crate) const SUPPRESSED: u32 = u32::MAX;

/// The painter state an overlay saves and restores around its content.
#[derive(Debug, Clone)]
pub(crate) struct PainterState {
    layer: usize,
    slot: Option<usize>,
    clip: Option<Rect>,
    round: Option<(Rect, f32)>,
    clips: Vec<ClipFrame>,
    xforms: Vec<Xform>,
    hit_tag: u32,
}

impl PainterState {
    /// This state drawing on the fixed layer `layer` instead, with its clip, transforms and hit
    /// tag as they were.
    pub(crate) fn on_fixed(mut self, layer: LayerId) -> Self {
        self.layer = layer as usize;
        self.slot = None;
        self
    }
}

/// Records one frame's drawing into a [`Scene`] and the hit regions of what it drew, at scale `s`.
/// Draw calls are in logical px; the painter turns them into physical px.
pub struct Painter<'a, H> {
    pub s: f32,
    pub window: Rect,
    /// The display list painted so far: every layer and the backdrop request.
    pub scene: Scene,
    /// The fixed layer drawing is on, and what a hit records.
    layer: usize,
    /// The entry of `scene.overlays` drawing goes to; `None` for the fixed layer itself.
    slot: Option<usize>,
    clip: Option<Rect>,
    /// The innermost rounded clip: its rect (also folded into `clip`) and corner radius.
    round: Option<(Rect, f32)>,
    /// The clips [`Self::push_clip`] and [`Self::push_clip_rounded`] replaced, innermost last.
    clips: Vec<ClipFrame>,
    pub hits: Vec<(Rect, H)>,
    pub text: &'a mut dyn TextSystem,
    pub hover: Option<H>,
    /// Tokens the toolkit widgets read.
    pub theme: Theme,
    /// The transform stack, outermost first.
    xforms: Vec<Xform>,
    /// `(layer, tag)` per entry of `hits`, kept in step by [`Self::hit`].
    hit_meta: Vec<(usize, u32)>,
    hit_tag: u32,
    recording: Option<Vec<Op>>,
}

impl<H: std::fmt::Debug> std::fmt::Debug for Painter<'_, H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Painter")
            .field("s", &self.s)
            .field("window", &self.window)
            .field("scene", &self.scene)
            .field("layer", &self.layer)
            .field("slot", &self.slot)
            .field("clip", &self.clip)
            .field("hits", &self.hits)
            .field("hover", &self.hover)
            .field("theme", &self.theme)
            .field("xforms", &self.xforms)
            .finish_non_exhaustive()
    }
}

impl<'a, H: Copy + PartialEq> Painter<'a, H> {
    /// A painter at scale `s` over `window`. `hover` is the hit value under the pointer, which
    /// [`Self::hovered`] compares against.
    pub fn new(
        s: f32,
        window: Rect,
        text: &'a mut dyn TextSystem,
        hover: Option<H>,
        theme: Theme,
    ) -> Self {
        Self {
            s,
            window,
            scene: Scene::default(),
            layer: 0,
            slot: None,
            clip: None,
            round: None,
            clips: Vec::new(),
            hits: Vec::new(),
            text,
            hover,
            theme,
            xforms: Vec::new(),
            hit_meta: Vec::new(),
            hit_tag: 0,
            recording: None,
        }
    }

    fn rec(&mut self, op: impl FnOnce() -> Op) {
        if let Some(ops) = &mut self.recording {
            ops.push(op());
        }
    }

    /// Starts recording the calls that draw or change state (not `hit` and `measure`), dropping
    /// an earlier unfinished recording.
    pub fn start_recording(&mut self) {
        self.recording = Some(Vec::new());
    }

    /// Stops recording and returns what was recorded; empty when not recording.
    pub fn take_recording(&mut self) -> Ops {
        Ops {
            ops: self.recording.take().unwrap_or_default(),
        }
    }

    /// Starts an inner recording, returning the outer one (if any) to hand to
    /// [`Self::unnest_recording`].
    pub(crate) fn nest_recording(&mut self) -> Option<Ops> {
        self.recording.replace(Vec::new()).map(|ops| Ops { ops })
    }

    /// Ends an inner recording: returns it, and resumes `outer` with the inner calls appended.
    pub(crate) fn unnest_recording(&mut self, outer: Option<Ops>) -> Ops {
        let inner = self.take_recording();
        if let Some(Ops { mut ops }) = outer {
            ops.extend(inner.ops.iter().cloned());
            self.recording = Some(ops);
        }
        inner
    }

    /// Stops recording until [`Self::resume_recording`], returning what was recorded so far (if
    /// anything was): what is drawn meanwhile belongs to no recording open now.
    pub(crate) fn pause_recording(&mut self) -> Option<Ops> {
        self.recording.take().map(|ops| Ops { ops })
    }

    /// Ends a [`Self::pause_recording`]: the recording it returned goes on.
    pub(crate) fn resume_recording(&mut self, paused: Option<Ops>) {
        self.recording = paused.map(|o| o.ops);
    }

    /// Replays `ops` through this painter's own methods, so the result depends on its current
    /// state (theme, scale, text system) as if the calls were made here.
    pub fn replay(&mut self, ops: &Ops) {
        for op in &ops.ops {
            match op {
                Op::On(l) => self.on(*l),
                Op::SetClip(c) => self.set_clip(*c),
                Op::PushClip(r) => self.push_clip(*r),
                Op::PushClipRounded(r, radius) => self.push_clip_rounded(*r, *radius),
                Op::PopClip => self.pop_clip(),
                Op::SetXform(x) => self.set_xform(*x),
                Op::PushXform(x) => self.push_xform(*x),
                Op::PopXform => self.pop_xform(),
                Op::PushTranslate(dx, dy) => self.push_translate(*dx, *dy),
                Op::PopTranslate => self.pop_translate(),
                Op::FillRadii(r, radii, c) => self.fill_radii(*r, *radii, *c),
                Op::FillOver(r, radius, c) => self.fill_over(*r, *radius, *c),
                Op::Ring(r, radius, w, c, above) => self.ring(*r, *radius, *w, *c, *above),
                Op::Acrylic(r, radii, c, under, noise) => {
                    self.acrylic_radii(*r, *radii, *c, *under, *noise);
                }
                Op::Frost(r, radii, c, noise) => self.frost_radii(*r, *radii, *c, *noise),
                Op::Backdrop(r, radius, sigma, saturation) => {
                    self.backdrop_with(*r, *radius, *sigma, *saturation);
                }
                Op::Text(spec, x, y) => {
                    self.text(spec, *x, *y);
                }
                Op::Stroke(pts, closed, st) => self.stroke(pts, *closed, *st),
                Op::Dash(pts, closed, width, on, off, c) => {
                    self.dash(pts, *closed, *width, *on, *off, *c);
                }
                Op::Arc(center, radius, start, sweep, st) => {
                    self.arc(*center, *radius, *start, *sweep, *st);
                }
                Op::FillPath(pts, c) => self.fill_path(pts, *c),
                Op::Sector(center, radius, start, sweep, c) => {
                    self.sector(*center, *radius, *start, *sweep, *c);
                }
                Op::Image(dst, id, fit, align, alpha, rotation) => {
                    self.image_rotated(*dst, *id, *fit, *align, *alpha, *rotation);
                }
                Op::TextRotated(spec, center, radians) => {
                    self.text_rotated(spec, *center, *radians);
                }
                Op::Custom(slot, r) => self.custom(*slot, *r),
            }
        }
    }

    /// Sets the tag the next [`Self::hit`] calls record with their layer, for the `Ui`'s hit list.
    #[cfg(test)]
    pub(crate) fn set_hit_tag(&mut self, tag: u32) {
        self.hit_tag = tag;
    }

    /// `(layer index, tag)` for each entry of `hits`, in the same order. Valid while `hits` is only
    /// appended to through [`Self::hit`].
    pub(crate) fn hit_meta(&self) -> &[(usize, u32)] {
        &self.hit_meta
    }

    /// The current layer index and hit tag.
    pub(crate) fn hit_context(&self) -> (usize, u32) {
        (self.layer, self.hit_tag)
    }

    /// The layer, clip, transform stack and hit tag, to restore with [`Self::restore_state`].
    pub(crate) fn save_state(&self) -> PainterState {
        PainterState {
            layer: self.layer,
            slot: self.slot,
            clip: self.clip,
            round: self.round,
            clips: self.clips.clone(),
            xforms: self.xforms.clone(),
            hit_tag: self.hit_tag,
        }
    }

    /// Puts back a [`Self::save_state`] result. Not recorded.
    pub(crate) fn restore_state(&mut self, s: PainterState) {
        self.layer = s.layer;
        self.slot = s.slot;
        self.clip = s.clip;
        self.round = s.round;
        self.clips = s.clips;
        self.xforms = s.xforms;
        self.hit_tag = s.hit_tag;
    }

    /// Switches to `layer` with no clip, an empty transform stack and hit tag `tag`, for the
    /// `Ui`'s overlays. A `tag` above 0 draws into that overlay's own layer on `layer`, made on
    /// first use; tag 0 draws into the fixed layer. Not recorded.
    pub(crate) fn enter(&mut self, layer: LayerId, tag: u32) {
        self.enter_as(layer, tag, tag);
    }

    /// [`Self::enter`] for the exit of the overlay `tag`: it draws in its open order, and its hits
    /// are suppressed.
    pub(crate) fn enter_closing(&mut self, layer: LayerId, tag: u32) {
        self.enter_as(layer, tag, SUPPRESSED);
    }

    fn enter_as(&mut self, layer: LayerId, draw_tag: u32, hit_tag: u32) {
        self.layer = layer as usize;
        self.slot = (draw_tag > 0).then(|| self.overlay_slot(layer, draw_tag));
        self.clip = None;
        self.round = None;
        self.clips.clear();
        self.xforms.clear();
        self.hit_tag = hit_tag;
    }

    /// The index in `scene.overlays` of the layer of overlay `tag` on `on`, made on first use.
    fn overlay_slot(&mut self, on: LayerId, tag: u32) -> usize {
        let overlays = &mut self.scene.overlays;
        overlays
            .iter()
            .position(|o| o.on == on && o.tag == tag)
            .unwrap_or_else(|| {
                overlays.push(OverlayLayer {
                    on,
                    tag,
                    layer: Layer::default(),
                });
                overlays.len() - 1
            })
    }

    /// The layer drawing goes to: the current overlay's, or the fixed layer.
    fn target(&mut self) -> &mut Layer {
        match self.slot {
            Some(i) => &mut self.scene.overlays[i].layer,
            None => &mut self.scene.layers[self.layer],
        }
    }

    /// Draws on `layer` from now on, with no clip. Items of one kind keep the order they were drawn
    /// in; the kinds draw in the renderer's order (acrylic, under shapes, path fills, customs,
    /// images, text, then over shapes and strokes, see [`crate::scene::Layer`]), so a fill drawn
    /// after a text still draws under it. The layers draw back to front ([`LayerId`]). This is the
    /// fixed layer, whichever overlay is drawing: what an overlay draws after it is not the
    /// overlay's.
    pub fn on(&mut self, layer: LayerId) {
        self.rec(|| Op::On(layer));
        self.layer = layer as usize;
        self.slot = None;
        self.clip = None;
        self.round = None;
        self.clips.clear();
    }

    /// Sets the clip to `clip` (logical px), or removes the rect clip with `None`. [`Self::push_clip`]
    /// nests; this replaces the rect. It leaves a rounded clip from [`Self::push_clip_rounded`]
    /// alone: that one stays until its [`Self::pop_clip`].
    pub fn set_clip(&mut self, clip: Option<Rect>) {
        self.rec(|| Op::SetClip(clip));
        self.clip = clip;
    }

    /// Narrows the clip to `rect` intersected with the current clip, until the matching
    /// [`Self::pop_clip`]. The rect is in the space draws are made in now, like [`Self::set_clip`]'s.
    pub fn push_clip(&mut self, rect: Rect) {
        self.rec(|| Op::PushClip(rect));
        self.push_frame();
        self.clip = Some(match self.clip {
            Some(c) => c.intersect(&rect),
            None => rect,
        });
    }

    /// [`Self::push_clip`] with the corners of `rect` rounded by `radius` (logical px, limited to
    /// half the shorter side): shapes, strokes, fills and images are cut along the rounded rect,
    /// antialiased; text and custom draws take its bounding rect only. The innermost rounded clip
    /// holds; a radius of 0 (or one that is negative or not finite) is a plain clip. A `rect` that
    /// is not finite clips everything.
    pub fn push_clip_rounded(&mut self, rect: Rect, radius: f32) {
        self.rec(|| Op::PushClipRounded(rect, radius));
        self.push_frame();
        let finite = [rect.x, rect.y, rect.w, rect.h]
            .iter()
            .all(|v| v.is_finite());
        if !finite {
            self.clip = Some(Rect::default());
            self.round = None;
            return;
        }
        self.clip = Some(match self.clip {
            Some(c) => c.intersect(&rect),
            None => rect,
        });
        let radius = if radius.is_finite() && radius > 0.0 {
            radius.min(rect.w.min(rect.h) / 2.0)
        } else {
            0.0
        };
        if radius > 0.0 {
            self.round = Some((rect, radius));
        }
    }

    fn push_frame(&mut self) {
        self.clips.push(ClipFrame {
            clip: self.clip,
            round: self.round,
        });
    }

    /// Restores the clip that held before the last [`Self::push_clip`] or
    /// [`Self::push_clip_rounded`]; does nothing without one.
    pub fn pop_clip(&mut self) {
        self.rec(|| Op::PopClip);
        if let Some(f) = self.clips.pop() {
            self.clip = f.clip;
            self.round = f.round;
        }
    }

    /// Replaces the transform stack: with `xform` as its only entry, or empty for `None`.
    pub fn set_xform(&mut self, xform: Option<Xform>) {
        self.rec(|| Op::SetXform(xform));
        self.xforms.clear();
        self.xforms.extend(xform);
    }

    /// Adds `xform` inside the current ones: it applies first, the earlier ones after it.
    pub fn push_xform(&mut self, xform: Xform) {
        self.rec(|| Op::PushXform(xform));
        self.xforms.push(xform);
    }

    /// Removes the innermost transform; does nothing on an empty stack.
    pub fn pop_xform(&mut self) {
        self.rec(|| Op::PopXform);
        self.xforms.pop();
    }

    /// Adds a translation by `dx`, `dy` inside the current transforms, like
    /// [`Self::push_xform`] with [`Xform::translate`], and keeps the current clip and rounded clip
    /// where they are on screen (a plain `push_xform` moves them with the new space). Pair it with
    /// [`Self::pop_translate`]; a scroll area draws its content this way.
    pub fn push_translate(&mut self, dx: f32, dy: f32) {
        self.rec(|| Op::PushTranslate(dx, dy));
        self.xforms.push(Xform::translate(dx, dy));
        self.shift_clips(-dx, -dy);
    }

    /// Removes the innermost transform, put there by [`Self::push_translate`], and moves the clips
    /// back; does nothing on an empty stack.
    pub fn pop_translate(&mut self) {
        self.rec(|| Op::PopTranslate);
        if let Some(x) = self.xforms.pop() {
            self.shift_clips(x.dx, x.dy);
        }
    }

    fn shift_clips(&mut self, dx: f32, dy: f32) {
        let moved = |r: Rect| Rect::new(r.x + dx, r.y + dy, r.w, r.h);
        self.clip = self.clip.map(moved);
        self.round = self.round.map(|(r, radius)| (moved(r), radius));
    }

    /// `x, y` through the stack, innermost first.
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        self.xforms
            .iter()
            .rev()
            .fold((x, y), |(x, y), xf| xf.point(x, y))
    }

    /// The product of the stack's scales (1.0 when empty).
    fn scale_k(&self) -> f32 {
        self.xforms.iter().fold(1.0, |k, xf| k * xf.scale)
    }

    /// The product of the stack's alphas (1.0 when empty).
    fn alpha_k(&self) -> f32 {
        self.xforms.iter().fold(1.0, |a, xf| a * xf.alpha)
    }

    fn tr(&self, r: Rect) -> Rect {
        match self.xforms.as_slice() {
            [] => return r,
            [only] => return only.rect(r),
            _ => {}
        }
        let (x, y) = self.point(r.x, r.y);
        let k = self.scale_k();
        Rect::new(x, y, r.w * k, r.h * k)
    }

    /// `r` from drawing space to window space: the transform stack applied, no clip.
    pub fn transformed(&self, r: Rect) -> Rect {
        self.tr(r)
    }

    fn fade(&self, c: [f32; 4]) -> [f32; 4] {
        let a = self.alpha_k();
        c.map(|v| v * a)
    }

    /// The clip in physical px. A clip with no area clips everything (the shader reads a zero
    /// width as "no clip", so it becomes a sliver of zero height).
    fn clip_phys(&self) -> [f32; 4] {
        self.clip.map_or([0.0; 4], |c| {
            let r = self.tr(c).phys(self.s);
            if r[2] > 0.0 && r[3] > 0.0 {
                r
            } else {
                [0.0, 0.0, f32::MIN_POSITIVE, 0.0]
            }
        })
    }

    /// The innermost rounded clip as a physical rect and radius; all zero when there is none.
    fn round_phys(&self) -> ([f32; 4], f32) {
        self.round.map_or(([0.0; 4], 0.0), |(r, radius)| {
            (self.tr(r).phys(self.s), radius * self.scale_k() * self.s)
        })
    }

    /// Ask for the blurred scene inside `r`, drawn under this layer, at the frame's sigma and
    /// saturation. See [`Self::backdrop_with`].
    pub fn backdrop(&mut self, r: Rect, radius: f32) {
        self.backdrop_with(r, radius, 0.0, 0.0);
    }

    /// [`Self::backdrop`] at its own `sigma` and `saturation` (0 means the frame's). Everything
    /// drawn before this layer is blurred, other glass included. Every request stands, and each has
    /// its own numbers; one that shows nothing (alpha 0, no area) records nothing.
    pub fn backdrop_with(&mut self, r: Rect, radius: f32, sigma: f32, saturation: f32) {
        self.rec(|| Op::Backdrop(r, radius, sigma, saturation));
        let (rect, alpha) = (self.tr(r).phys(self.s), self.alpha_k());
        let shows = alpha > 0.0 && rect[2] > 0.0 && rect[3] > 0.0;
        if !shows {
            return;
        }
        let radius = radius * self.scale_k() * self.s;
        self.target().backdrops.push(BackdropRequest {
            rect,
            radius,
            alpha,
            sigma,
            saturation,
        });
    }

    fn geo(&mut self) -> &mut LayerGeometry {
        &mut self.target().geometry
    }

    fn shape(&self, r: Rect, radii: [f32; 4], color: [f32; 4]) -> Shape {
        let k = self.scale_k();
        let (round, round_radius) = self.round_phys();
        Shape {
            rect: self.tr(r).phys(self.s),
            radii: radii.map(|v| v * self.s * k),
            color: self.fade(color),
            under: [0.0; 4],
            params: [0.0; 4],
            clip: self.clip_phys(),
            round,
            round_radius,
        }
    }

    /// Fill below this layer's text.
    pub fn fill(&mut self, r: Rect, radius: f32, color: Rgba) {
        self.fill_radii(r, [radius; 4], color);
    }

    /// [`Self::fill`] with a radius per corner: top-left, top-right, bottom-right, bottom-left.
    pub fn fill_radii(&mut self, r: Rect, radii: [f32; 4], color: Rgba) {
        self.rec(|| Op::FillRadii(r, radii, color));
        let sh = self.shape(r, radii, premul(color));
        self.geo().under.push(sh);
    }

    /// Fill above this layer's text (selection, cursor, overlays).
    pub fn fill_over(&mut self, r: Rect, radius: f32, color: [f32; 4]) {
        self.rec(|| Op::FillOver(r, radius, color));
        let sh = self.shape(r, [radius; 4], color);
        self.geo().over.push(sh);
    }

    /// 1px-style outline inside `r`.
    pub fn ring(&mut self, r: Rect, radius: f32, width: f32, color: Rgba, above_text: bool) {
        self.rec(|| Op::Ring(r, radius, width, color, above_text));
        let mut sh = self.shape(r, [radius; 4], premul(color));
        sh.params = [1.0, width * self.s, 0.0, 0.0];
        if above_text {
            self.geo().over.push(sh);
        } else {
            self.geo().under.push(sh);
        }
    }

    /// The window's own background, on the fixed [`LayerId::Base`] layer: an acrylic layer with
    /// Skia `Src` semantics, which replaces what is below (`under` only feeds the antialiased
    /// edge). Colours premultiplied. The shape keeps `color` unfaded; the transform stack's alpha
    /// travels as the record's `opacity`, which the renderer blends the layer in with. Nothing is
    /// recorded when that opacity is not above zero.
    ///
    /// Anywhere else, on a fixed layer above `Base` or inside any overlay, it draws as
    /// [`Self::frost`] with the same colour and ignores `under`: no draw inside a window replaces
    /// what is below it, so none can open a hole to the desktop.
    pub fn acrylic(&mut self, r: Rect, radius: f32, color: [f32; 4], under: [f32; 4], noise: bool) {
        self.acrylic_radii(r, [radius; 4], color, under, noise);
    }

    /// [`Self::acrylic`] with one radius per corner, in [`Self::fill_radii`]'s order.
    pub fn acrylic_radii(
        &mut self,
        r: Rect,
        radii: [f32; 4],
        color: [f32; 4],
        under: [f32; 4],
        noise: bool,
    ) {
        // The branch comes first: a call above the background records one op, the frost's.
        if self.layer != LayerId::Base as usize || self.slot.is_some() {
            self.frost_radii(r, radii, color, noise);
            return;
        }
        self.rec(|| Op::Acrylic(r, radii, color, under, noise));
        let opacity = self.alpha_k();
        if opacity.is_nan() || opacity <= 0.0 {
            return;
        }
        let mut shape = self.shape(r, radii, color);
        shape.color = color;
        shape.under = under;
        shape.params = [0.0, 0.0, 1.0, if noise { 1.0 } else { 0.0 }];
        self.geo().acrylic.push(Acrylic { shape, opacity });
    }

    /// The tint of a glass surface: premultiplied `color` over what is below, with the noise over
    /// it when `noise` is set. It draws with the shapes under the text, in record order; a
    /// [`Self::backdrop`] asked for first gives it something frosted to tint.
    pub fn frost(&mut self, r: Rect, radius: f32, color: [f32; 4], noise: bool) {
        self.frost_radii(r, [radius; 4], color, noise);
    }

    /// [`Self::frost`] with one radius per corner, in [`Self::fill_radii`]'s order.
    pub fn frost_radii(&mut self, r: Rect, radii: [f32; 4], color: [f32; 4], noise: bool) {
        self.rec(|| Op::Frost(r, radii, color, noise));
        let alpha = self.alpha_k();
        if alpha.is_nan() || alpha <= 0.0 {
            return;
        }
        let mut shape = self.shape(r, radii, color);
        // The colour stays unfaded and the alpha rides beside it, so the noise fades with it.
        shape.color = color;
        shape.params = [0.0, alpha, 2.0, if noise { 1.0 } else { 0.0 }];
        self.geo().under.push(shape);
    }

    /// Records an app-owned draw (the renderer's `CustomDraw` number `slot`) on the
    /// current layer, clipped to `r`, the current clip and the window. Nothing is recorded when
    /// that is empty. The clip is in physical px, left and top floored, right and bottom
    /// ceiled. The popup transform is ignored: custom items are not animated.
    pub fn custom(&mut self, slot: usize, r: Rect) {
        self.rec(|| Op::Custom(slot, r));
        let mut area = r.intersect(&self.window);
        if let Some(c) = self.clip {
            area = area.intersect(&c);
        }
        if area.w <= 0.0 || area.h <= 0.0 {
            return;
        }
        let [x, y, w, h] = area.phys(self.s);
        let (x0, y0) = (x.floor().max(0.0), y.floor().max(0.0));
        let (x1, y1) = ((x + w).ceil().max(0.0), (y + h).ceil().max(0.0));
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        self.target().customs.push(CustomItem {
            slot,
            clip: [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32],
        });
    }

    /// Draws `spec` with its top-left corner at `x`, `y`. Returns its size in logical px.
    pub fn text(&mut self, spec: &Spec, x: f32, y: f32) -> (f32, f32) {
        self.rec(|| Op::Text(spec.clone(), x, y));
        let size = self.text.prepare(spec, self.s).1;
        let (key, scale) = if self.xforms.is_empty() {
            (self.text.prepare(spec, self.s).0, 1.0)
        } else {
            let (alpha, scale) = (self.alpha_k(), self.scale_k());
            if alpha < 0.999 {
                let mut faded = spec.clone();
                for (_, style) in &mut faded.runs {
                    style.color[3] = (f32::from(style.color[3]) * alpha).round() as u8;
                }
                (self.text.prepare(&faded, self.s).0, scale)
            } else {
                (self.text.prepare(spec, self.s).0, scale)
            }
        };
        let clip = self.tr(self.clip.unwrap_or(self.window));
        let (x, y) = self.point(x, y);
        self.target().texts.push(TextItem {
            key,
            x,
            y,
            clip,
            scale,
        });
        size
    }

    /// Draws the picture registered as `id` into `dst`, placed by `fit` and `align`, at opacity
    /// `alpha` (0 to 1) times the transform stack's, after this layer's custom draws and before
    /// its text. An id the renderer does not hold draws nothing; so does a `dst` with no area,
    /// or any number that is not finite.
    pub fn image(&mut self, dst: Rect, id: ImageId, fit: Fit, align: ImageAlign, alpha: f32) {
        self.image_rotated(dst, id, fit, align, alpha, 0.0);
    }

    /// [`Self::image`] turned by `radians` (clockwise on screen) about the centre of the fitted
    /// picture. Clips and the rounded clip stay put; only the picture turns.
    pub fn image_rotated(
        &mut self,
        dst: Rect,
        id: ImageId,
        fit: Fit,
        align: ImageAlign,
        alpha: f32,
        radians: f32,
    ) {
        self.rec(|| Op::Image(dst, id, fit, align, alpha, radians));
        self.push_image(ImageSource::Id(id), dst, fit, align, alpha, radians);
    }

    /// Draws `spec`, centred on `center`, turned by `radians` (clockwise on screen) about that
    /// centre. The renderer rasterises the shaped text once and turns the raster, so unlike
    /// [`Self::text`] the glyphs are resampled; the transform stack's alpha is applied when
    /// drawing. Returns the text's logical size.
    pub fn text_rotated(&mut self, spec: &Spec, center: (f32, f32), radians: f32) -> (f32, f32) {
        self.rec(|| Op::TextRotated(spec.clone(), center, radians));
        let (key, size) = self.text.prepare(spec, self.s);
        let k = self.scale_k();
        let (cx, cy) = self.point(center.0, center.1);
        let (w, h) = (size.0 * k * self.s, size.1 * k * self.s);
        let dst = Rect::new(cx * self.s - w / 2.0, cy * self.s - h / 2.0, w, h);
        let src = ImageSource::Text(key, k);
        self.push_image_phys(src, dst, Fit::Contain, ImageAlign::CENTER, 1.0, radians);
        size
    }

    fn push_image(
        &mut self,
        source: ImageSource,
        dst: Rect,
        fit: Fit,
        align: ImageAlign,
        alpha: f32,
        radians: f32,
    ) {
        if ![dst.x, dst.y, dst.w, dst.h].iter().all(|v| v.is_finite()) {
            return;
        }
        let dst = self.tr(dst);
        let dst = Rect::new(
            dst.x * self.s,
            dst.y * self.s,
            dst.w * self.s,
            dst.h * self.s,
        );
        self.push_image_phys(source, dst, fit, align, alpha, radians);
    }

    /// Records an image whose `dst` is already in physical px.
    fn push_image_phys(
        &mut self,
        source: ImageSource,
        dst: Rect,
        fit: Fit,
        align: ImageAlign,
        alpha: f32,
        radians: f32,
    ) {
        let alpha = alpha.clamp(0.0, 1.0) * self.alpha_k();
        let ok = [dst.x, dst.y, dst.w, dst.h].iter().all(|v| v.is_finite())
            && dst.w > 0.0
            && dst.h > 0.0
            && radians.is_finite()
            && alpha.is_finite()
            && alpha > 0.0;
        if !ok {
            return;
        }
        let (rect, radius) = self.round_phys();
        let item = ImageItem {
            source,
            dst,
            fit,
            align,
            alpha,
            rotation: radians,
            clip: self.clip_phys(),
            round_clip: RoundClip { rect, radius },
        };
        self.target().images.push(item);
    }

    /// Text vertically centred in `r` at its left edge (plus `pad`).
    pub fn text_in(&mut self, spec: &Spec, r: Rect, pad: f32) -> (f32, f32) {
        let (_, (w, h)) = self.text.prepare(spec, self.s);
        self.text(spec, r.x + pad, r.y + (r.h - h) / 2.0);
        (w, h)
    }

    /// Draws `spec` centred in `r`.
    pub fn text_centered(&mut self, spec: &Spec, r: Rect) {
        let (_, (w, h)) = self.text.prepare(spec, self.s);
        self.text(spec, r.x + (r.w - w) / 2.0, r.y + (r.h - h) / 2.0);
    }

    /// The size of `spec` in logical px at this painter's scale.
    pub fn measure(&mut self, spec: &Spec) -> (f32, f32) {
        self.text.measure(spec, self.s)
    }

    /// `spec` cut to `max_w` at this frame's scale: [`crate::text::elide`].
    pub fn elide(&mut self, spec: &Spec, max_w: f32) -> Spec {
        crate::text::elide(&mut *self.text, spec, self.s, max_w)
    }

    /// Where the caret sits at byte `offset` of `spec`'s joined text ([`TextSystem::caret`]).
    pub fn caret(&mut self, spec: &Spec, offset: usize) -> CaretPos {
        self.text.caret(spec, self.s, offset)
    }

    /// The byte offset nearest to `pos`, logical px from the text's top-left corner
    /// ([`TextSystem::hit`]).
    pub fn text_hit(&mut self, spec: &Spec, pos: (f32, f32)) -> usize {
        self.text.hit(spec, self.s, pos)
    }

    /// The visual lines of `spec` ([`TextSystem::lines`]).
    pub fn text_lines(&mut self, spec: &Spec) -> Vec<LineSpan> {
        self.text.lines(spec, self.s)
    }

    /// The boundaries of `unit` in `text` ([`TextSystem::segments`]).
    pub fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
        self.text.segments(text, unit)
    }

    /// `r` as a hit region of it would be: cut to the current clip, then transformed.
    pub(crate) fn hit_rect(&self, r: Rect) -> Rect {
        let r = match self.clip {
            Some(c) => r.intersect(&c),
            None => r,
        };
        self.tr(r)
    }

    /// Adds a hit region: pointer events over `r` (cut to the clip, then transformed) belong to
    /// `h`.
    pub fn hit(&mut self, r: Rect, h: H) {
        let r = self.hit_rect(r);
        self.hits.push((r, h));
        self.hit_meta.push((self.layer, self.hit_tag));
    }

    /// Whether the pointer is over `h`'s region.
    pub fn hovered(&self, h: H) -> bool {
        self.hover == Some(h)
    }

    /// Tessellates a polyline (logical px) into antialiased triangles (physical px).
    pub fn stroke(&mut self, pts: &[[f32; 2]], closed: bool, st: StrokeStyle) {
        self.rec(|| Op::Stroke(pts.to_vec(), closed, st));
        self.stroke_pts(pts, closed, st);
    }

    /// A closed loop through `pts` lit by the conic sweep: alpha runs round `center` (logical px,
    /// where the points are), brightest at `phase` turns clockwise from 12 o'clock. `phase` wraps;
    /// one that is not finite is 0.
    pub fn stroke_sweep(
        &mut self,
        pts: &[[f32; 2]],
        color: Rgba,
        width: f32,
        phase: f32,
        center: [f32; 2],
    ) {
        let style = StrokeStyle {
            mode: MODE_SWEEP,
            a: turns(phase),
            extra: [center[0], center[1], 0.0, 0.0],
            ..StrokeStyle::solid(color, width)
        };
        self.stroke(pts, true, style);
    }

    /// A closed loop through `pts`, lit only along a dash `fraction` of its contour long (clamped
    /// to 0 to 1), which `phase` turns carry once round the loop, against the points' order.
    /// `phase` wraps; one that is not finite is 0.
    pub fn stroke_comet(
        &mut self,
        pts: &[[f32; 2]],
        color: Rgba,
        width: f32,
        fraction: f32,
        phase: f32,
    ) {
        let contour = polyline_length(pts);
        let style = StrokeStyle {
            mode: MODE_COMET,
            a: fraction.clamp(0.0, 1.0) * contour,
            b: turns(phase) * contour,
            extra: [0.0, 0.0, contour, 0.0],
            ..StrokeStyle::solid(color, width)
        };
        self.stroke(pts, true, style);
    }

    /// `st`'s mode parameters in the units the shader reads: the sweep's centre through the
    /// transform stack, in physical px; the comet's lengths in the logical px of the transformed
    /// points, which is what a vertex's `aa.y` measures.
    fn shader_style(&self, st: StrokeStyle) -> StrokeStyle {
        let mut st = st;
        if st.mode == MODE_SWEEP {
            let (x, y) = self.point(st.extra[0], st.extra[1]);
            st.extra[0] = x * self.s;
            st.extra[1] = y * self.s;
        } else if st.mode == MODE_COMET {
            let k = self.scale_k();
            st.a *= k;
            st.b *= k;
            st.extra[2] *= k;
        }
        st
    }

    /// [`Self::stroke`] without recording, for the primitives built on it.
    fn stroke_pts(&mut self, pts: &[[f32; 2]], closed: bool, st: StrokeStyle) {
        let s = self.s;
        let st = self.shader_style(st);
        let clip = self.clip_phys();
        let (round, round_radius) = self.round_phys();
        let (pts, st) = if self.xforms.is_empty() {
            (pts.to_vec(), st)
        } else {
            let (alpha, scale) = (self.alpha_k(), self.scale_k());
            (
                pts.iter()
                    .map(|q| {
                        let (x, y) = self.point(q[0], q[1]);
                        [x, y]
                    })
                    .collect::<Vec<_>>(),
                StrokeStyle {
                    color: st.color.map(|v| v * alpha),
                    width: st.width * scale,
                    ..st
                },
            )
        };
        let pts = pts.as_slice();
        let mut p: Vec<[f32; 2]> = Vec::with_capacity(pts.len());
        for &q in pts {
            if p.last()
                .is_none_or(|l: &[f32; 2]| (l[0] - q[0]).abs() + (l[1] - q[1]).abs() > 1e-3)
            {
                p.push(q);
            }
        }
        if closed && p.len() > 2 && p[0] == p[p.len() - 1] {
            p.pop();
        }
        let n = p.len();
        if n < 2 {
            return;
        }
        let seg_normal = |a: [f32; 2], b: [f32; 2]| {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            [-dy / len, dx / len]
        };
        let normals: Vec<([f32; 2], f32)> = (0..n)
            .map(|i| {
                let prev = if i > 0 {
                    Some(p[i - 1])
                } else if closed {
                    Some(p[n - 1])
                } else {
                    None
                };
                let next = if i + 1 < n {
                    Some(p[i + 1])
                } else if closed {
                    Some(p[0])
                } else {
                    None
                };
                match (prev, next) {
                    (Some(a), Some(b)) => {
                        let n0 = seg_normal(a, p[i]);
                        let n1 = seg_normal(p[i], b);
                        let m = [n0[0] + n1[0], n0[1] + n1[1]];
                        let len = (m[0] * m[0] + m[1] * m[1]).sqrt().max(1e-6);
                        let m = [m[0] / len, m[1] / len];
                        let cos = (m[0] * n0[0] + m[1] * n0[1]).max(0.3);
                        (m, 1.0 / cos)
                    }
                    (None, Some(b)) => (seg_normal(p[i], b), 1.0),
                    (Some(a), None) => (seg_normal(a, p[i]), 1.0),
                    (None, None) => ([0.0, 0.0], 1.0),
                }
            })
            .collect();
        let half = st.width * s / 2.0;
        let expand = half + 1.0;
        let mut along = 0.0;
        let segs = if closed { n } else { n - 1 };
        let out = &mut self.target().geometry.strokes;
        for i in 0..segs {
            let j = (i + 1) % n;
            let len = ((p[j][0] - p[i][0]).powi(2) + (p[j][1] - p[i][1]).powi(2)).sqrt();
            let v = |k: usize, side: f32, d: f32| {
                let (nrm, miter) = normals[k];
                StrokeVertex {
                    pos: [
                        p[k][0] * s + nrm[0] * expand * miter * side,
                        p[k][1] * s + nrm[1] * expand * miter * side,
                    ],
                    aa: [expand * side, d],
                    color: st.color,
                    style: [st.mode, half, st.a, st.b],
                    extra: st.extra,
                    clip,
                    round,
                    round_radius,
                }
            };
            let (a0, a1) = (v(i, -1.0, along), v(i, 1.0, along));
            let (b0, b1) = (v(j, -1.0, along + len), v(j, 1.0, along + len));
            out.extend_from_slice(&[a0, a1, b0, a1, b1, b0]);
            along += len;
        }
    }

    /// A dashed polyline: runs `on` long separated by gaps `off` long (logical px, along the path,
    /// starting with a run at the first point), each stroked `width` wide in `color`. `closed`
    /// also walks the segment from the last point back to the first. An `off` of 0 is a solid
    /// stroke; an `on` or `width` that is not above zero, or any value not finite, draws nothing.
    /// A pattern so fine that it would make over 100 000 runs is drawn solid.
    pub fn dash(
        &mut self,
        pts: &[[f32; 2]],
        closed: bool,
        width: f32,
        on: f32,
        off: f32,
        color: Rgba,
    ) {
        self.rec(|| Op::Dash(pts.to_vec(), closed, width, on, off, color));
        let ok = pts.iter().flatten().all(|v| v.is_finite())
            && [width, on, off].iter().all(|v| v.is_finite())
            && width > 0.0
            && on > 0.0
            && off >= 0.0;
        if !ok {
            return;
        }
        let st = StrokeStyle::solid(color, width);
        let runs = if off > 0.0 {
            path::dash(pts, closed, on, off)
        } else {
            None
        };
        match runs {
            Some(runs) => {
                for run in runs {
                    self.stroke_pts(&run, false, st);
                }
            }
            None => self.stroke_pts(pts, closed, st),
        }
    }

    /// The device radius of a circle of logical `radius` under the current transforms.
    fn device_radius(&self, radius: f32) -> f32 {
        radius * self.scale_k() * self.s
    }

    /// Strokes an arc of the circle of `radius` about `center` (logical px): from angle `start`
    /// over `sweep` radians, where 0 points right and a positive sweep turns clockwise on the
    /// screen. The polyline's chords stay within 0.25 device px of the circle. A `sweep` of 2π
    /// or more (either way) is a closed circle. A `radius` or `sweep` that is zero, or any value
    /// not finite, draws nothing.
    pub fn arc(
        &mut self,
        center: [f32; 2],
        radius: f32,
        start: f32,
        sweep: f32,
        style: StrokeStyle,
    ) {
        self.rec(|| Op::Arc(center, radius, start, sweep, style));
        let styled = style.width.is_finite() && style.color.iter().all(|v| v.is_finite());
        let Some((pts, closed)) =
            path::arc(center, radius, start, sweep, self.device_radius(radius))
        else {
            return;
        };
        if styled {
            self.stroke_pts(&pts, closed, style);
        }
    }

    /// Fills the simple polygon `pts` (logical px) with `color`, below this layer's text. Ear
    /// clipping triangulates it; a one-pixel fringe antialiases the edge. Holes and
    /// self-intersection are not supported: a polygon the clipper cannot finish (or one with over
    /// 4096 points) draws the triangles found, never panics. Points that are not finite draw
    /// nothing.
    pub fn fill_path(&mut self, pts: &[[f32; 2]], color: Rgba) {
        self.rec(|| Op::FillPath(pts.to_vec(), color));
        if !pts.iter().flatten().all(|v| v.is_finite()) {
            return;
        }
        let mut poly: Vec<[f32; 2]> = path::dedup_loop(&self.device_points(pts));
        if poly.len() < 3 {
            return;
        }
        if path::signed_area2(&poly) < 0.0 {
            poly.reverse();
        }
        let tris = path::ear_clip(&poly);
        let boundary: Vec<usize> = (0..poly.len()).collect();
        self.emit_fill(&poly, &tris, &boundary, color);
    }

    /// Fills the circular sector of `radius` about `center` (logical px) from angle `start` over
    /// `sweep`, with the angle convention of [`Self::arc`], as one fan below this layer's text:
    /// one pass, so no seam shows at the centre. A `sweep` of 2π or more is the whole disc. A
    /// `radius` or `sweep` that is zero, or any value not finite, draws nothing.
    pub fn sector(&mut self, center: [f32; 2], radius: f32, start: f32, sweep: f32, color: Rgba) {
        self.rec(|| Op::Sector(center, radius, start, sweep, color));
        let Some((arc, full)) = path::arc(center, radius, start, sweep, self.device_radius(radius))
        else {
            return;
        };
        let mut verts = vec![center];
        verts.extend(arc);
        let verts = self.device_points(&verts);
        let n = verts.len();
        let (tris, boundary): (Vec<[usize; 3]>, Vec<usize>) = if full {
            (
                (1..n)
                    .map(|k| [0, k, if k + 1 < n { k + 1 } else { 1 }])
                    .collect(),
                (1..n).collect(),
            )
        } else {
            (
                (1..n - 1).map(|k| [0, k, k + 1]).collect(),
                (0..n).collect(),
            )
        };
        self.emit_fill(&verts, &tris, &boundary, color);
    }

    /// `pts` through the transform stack, in physical px.
    fn device_points(&self, pts: &[[f32; 2]]) -> Vec<[f32; 2]> {
        pts.iter()
            .map(|q| {
                let (x, y) = self.point(q[0], q[1]);
                [x * self.s, y * self.s]
            })
            .collect()
    }

    /// Records `tris` (indices into `verts`, physical px) as interior triangles and the fringe
    /// of the `boundary` loop into this layer's `fills`.
    fn emit_fill(
        &mut self,
        verts: &[[f32; 2]],
        tris: &[[usize; 3]],
        boundary: &[usize],
        color: Rgba,
    ) {
        const HALF: f32 = 0.5;
        const FRINGE: f32 = 1.0;
        if verts.iter().flatten().any(|v| !v.is_finite()) {
            return;
        }
        let color = self.fade(premul(color));
        let (clip, (round, round_radius)) = (self.clip_phys(), self.round_phys());
        let vertex = |pos: [f32; 2], aa: f32| StrokeVertex {
            pos,
            aa: [aa, 0.0],
            color,
            style: [0.0, HALF, 0.0, 0.0],
            extra: [0.0; 4],
            clip,
            round,
            round_radius,
        };
        let ring: Vec<[f32; 2]> = boundary.iter().map(|&i| verts[i]).collect();
        let made: Vec<StrokeVertex> = tris
            .iter()
            .flatten()
            .map(|&i| vertex(verts[i], 0.0))
            .chain(
                path::fringe(&ring, FRINGE)
                    .into_iter()
                    .map(|(pos, outer)| vertex(pos, if outer { HALF + FRINGE } else { 0.0 })),
            )
            .collect();
        // Coordinates near the f32 limit overflow in the fringe's arithmetic: draw nothing.
        if made.iter().all(|v| v.pos.iter().all(|c| c.is_finite())) {
            self.target().geometry.fills.extend(made);
        }
    }
}

/// `phase` wrapped into one turn; 0 when it is not finite.
fn turns(phase: f32) -> f32 {
    if phase.is_finite() {
        phase.rem_euclid(1.0)
    } else {
        0.0
    }
}

/// The length of the closed loop through `pts`, the closing segment included.
pub fn polyline_length(pts: &[[f32; 2]]) -> f32 {
    let dist = |a: [f32; 2], b: [f32; 2]| ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    pts.windows(2).map(|w| dist(w[0], w[1])).sum::<f32>()
        + pts
            .first()
            .zip(pts.last())
            .map_or(0.0, |(a, b)| dist(*a, *b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeText;
    use crate::theme::lunegit_dark::THEME;

    fn with_painter(f: impl FnOnce(&mut Painter<'_, u8>)) {
        let mut text = FakeText;
        let mut p = Painter::new(
            2.0,
            Rect::new(0.0, 0.0, 200.0, 100.0),
            &mut text,
            None,
            THEME,
        );
        f(&mut p);
    }

    #[test]
    fn custom_records_physical_clip_on_base() {
        with_painter(|p| {
            p.custom(3, Rect::new(10.0, 20.0, 30.25, 40.0));
            assert_eq!(
                p.scene.layers[0].customs,
                [CustomItem {
                    slot: 3,
                    clip: [20, 40, 61, 80]
                }]
            );
        });
    }

    #[test]
    fn custom_is_cut_by_the_current_clip() {
        with_painter(|p| {
            p.set_clip(Some(Rect::new(0.0, 0.0, 15.0, 100.0)));
            p.custom(3, Rect::new(10.0, 20.0, 30.25, 40.0));
            assert_eq!(
                p.scene.layers[0].customs,
                [CustomItem {
                    slot: 3,
                    clip: [20, 40, 10, 80]
                }]
            );
        });
    }

    #[test]
    fn custom_outside_the_window_records_nothing() {
        with_painter(|p| {
            p.custom(0, Rect::new(300.0, 20.0, 30.0, 40.0));
            p.custom(0, Rect::new(-50.0, 20.0, 30.0, 40.0));
            p.custom(0, Rect::new(10.0, 20.0, 0.0, 40.0));
            assert!(p.scene.layers.iter().all(|l| l.customs.is_empty()));
        });
    }

    #[test]
    fn hit_records_layer_and_tag_in_step_with_hits() {
        with_painter(|p| {
            p.hit(Rect::new(0.0, 0.0, 10.0, 10.0), 1);
            p.set_hit_tag(7);
            p.on(LayerId::Popup);
            p.hit(Rect::new(0.0, 0.0, 10.0, 10.0), 2);
            p.set_hit_tag(9);
            p.on(LayerId::Modal);
            p.hit(Rect::new(0.0, 0.0, 10.0, 10.0), 3);
            assert_eq!(p.hits.len(), 3);
            assert_eq!(p.hit_meta(), [(0, 0), (2, 7), (3, 9)]);
        });
    }

    #[test]
    fn custom_lands_on_the_current_layer() {
        with_painter(|p| {
            p.on(LayerId::Popup);
            p.custom(1, Rect::new(10.0, 20.0, 30.0, 40.0));
            assert!(p.scene.layers[0].customs.is_empty());
            assert_eq!(p.scene.layers[2].customs.len(), 1);
            assert_eq!(p.scene.layers[2].customs[0].slot, 1);
        });
    }

    /// The slots of the custom items of `layers`, in order.
    fn slots(layers: &[&crate::scene::Layer]) -> Vec<usize> {
        let items = layers.iter().flat_map(|l| &l.customs);
        items.map(|c| c.slot).collect()
    }

    #[test]
    fn overlays_draw_in_tag_order_whatever_order_they_drew_in() {
        with_painter(|p| {
            let r = Rect::new(0.0, 0.0, 10.0, 10.0);
            p.enter(LayerId::Modal, 7);
            p.custom(7, r);
            p.enter(LayerId::Modal, 5);
            p.custom(5, r);
            p.enter(LayerId::Toast, 0);
            p.custom(9, r);
            p.enter(LayerId::Modal, 0);
            p.custom(1, r);
            assert_eq!(p.scene.overlays.len(), 2);
            let order = p.scene.in_order();
            assert_eq!(order.len(), crate::scene::LAYER_COUNT + 2);
            assert_eq!(slots(&order), [1, 5, 7, 9]);
            let modal = p.scene.merged(LayerId::Modal);
            assert_eq!(slots(&[&modal]), [1, 5, 7]);
            assert_eq!(slots(&[&p.scene.layers[LayerId::Modal as usize]]), [1]);
        });
    }

    #[test]
    fn an_overlay_keeps_one_layer_and_the_painter_state_carries_it() {
        with_painter(|p| {
            let r = Rect::new(0.0, 0.0, 10.0, 10.0);
            p.enter(LayerId::Popup, 3);
            p.custom(1, r);
            let parent = p.save_state();
            p.enter(LayerId::Popup, 4);
            p.custom(2, r);
            p.restore_state(parent);
            p.custom(3, r);
            p.enter(LayerId::Popup, 3);
            p.custom(4, r);
            assert_eq!(p.scene.overlays.len(), 2);
            assert_eq!(slots(&[&p.scene.overlays[0].layer]), [1, 3, 4]);
            assert_eq!(slots(&[&p.scene.overlays[1].layer]), [2]);
            // `on` leaves the overlay for the fixed layer.
            p.on(LayerId::Popup);
            p.custom(5, r);
            assert_eq!(slots(&[&p.scene.layers[LayerId::Popup as usize]]), [5]);
            assert_eq!(slots(&[&p.scene.overlays[0].layer]), [1, 3, 4]);
        });
    }

    #[test]
    fn an_exit_draws_on_its_overlays_layer_with_its_hits_suppressed() {
        with_painter(|p| {
            let r = Rect::new(0.0, 0.0, 10.0, 10.0);
            p.enter_closing(LayerId::Popup, 3);
            p.custom(1, r);
            p.hit(r, 1);
            assert_eq!(p.scene.overlays[0].tag, 3);
            assert_eq!(slots(&[&p.scene.overlays[0].layer]), [1]);
            assert_eq!(p.hit_meta(), [(LayerId::Popup as usize, SUPPRESSED)]);
        });
    }

    #[test]
    fn a_backdrop_stays_with_the_layer_it_was_asked_on() {
        with_painter(|p| {
            let r = Rect::new(0.0, 0.0, 10.0, 10.0);
            p.enter(LayerId::Popup, 7);
            p.backdrop(r, 7.0);
            p.enter(LayerId::Popup, 5);
            p.backdrop(r, 5.0);
            p.on(LayerId::Page);
            p.backdrop(r, 1.0);
            let radii: Vec<f32> = p.scene.backdrops().map(|b| b.radius / 2.0).collect();
            assert_eq!(radii, [1.0, 5.0, 7.0], "draw order");
            assert_eq!(p.scene.floor(), crate::scene::SCENE_LAYERS);
        });
    }
}

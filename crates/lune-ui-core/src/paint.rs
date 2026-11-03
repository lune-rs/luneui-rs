//! Immediate-mode painter: every frame the UI is described from state into per-layer geometry,
//! text placements and hit regions. Clicks resolve against the previous frame's hit regions.

use crate::color::{Rgba, premul};
use crate::scene::{
    Acrylic, BackdropRequest, CustomItem, LayerGeometry, Scene, Shape, StrokeVertex, TextItem,
};
use crate::text::{Spec, TextSystem};
use crate::theme::Theme;

pub use crate::scene::{LAYER_COUNT, LayerId};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 < self.right() && p.1 >= self.y && p.1 < self.bottom()
    }

    pub fn inset(&self, dx: f32, dy: f32) -> Self {
        Self::new(
            self.x + dx,
            self.y + dy,
            (self.w - 2.0 * dx).max(0.0),
            (self.h - 2.0 * dy).max(0.0),
        )
    }

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

    pub fn phys(&self, s: f32) -> [f32; 4] {
        [self.x * s, self.y * s, self.w * s, self.h * s]
    }
}

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

/// Animation transform for a popup: scale about `origin`, then move down `dy`, and fade.
///
/// Transforms stack ([`Painter::push_xform`]): the innermost applies first, then the next one out.
#[derive(Debug, Clone, Copy)]
pub struct Xform {
    pub origin: (f32, f32),
    pub scale: f32,
    pub dy: f32,
    pub alpha: f32,
}

impl Xform {
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.origin.0 + (x - self.origin.0) * self.scale,
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
    SetXform(Option<Xform>),
    PushXform(Xform),
    PopXform,
    FillRadii(Rect, [f32; 4], Rgba),
    FillOver(Rect, f32, [f32; 4]),
    Ring(Rect, f32, f32, Rgba, bool),
    Acrylic(Rect, f32, [f32; 4], [f32; 4], bool),
    Backdrop(Rect, f32),
    Text(Spec, f32, f32),
    Stroke(Vec<[f32; 2]>, bool, StrokeStyle),
    Custom(usize, Rect),
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

pub struct Painter<'a, H> {
    pub s: f32,
    pub window: Rect,
    /// The display list painted so far: every layer and the backdrop request.
    pub scene: Scene,
    layer: usize,
    clip: Option<Rect>,
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
            .field("clip", &self.clip)
            .field("hits", &self.hits)
            .field("hover", &self.hover)
            .field("theme", &self.theme)
            .field("xforms", &self.xforms)
            .finish_non_exhaustive()
    }
}

impl<'a, H: Copy + PartialEq> Painter<'a, H> {
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
            clip: None,
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

    /// Replays `ops` through this painter's own methods, so the result depends on its current
    /// state (theme, scale, text system) as if the calls were made here.
    pub fn replay(&mut self, ops: &Ops) {
        for op in &ops.ops {
            match op {
                Op::On(l) => self.on(*l),
                Op::SetClip(c) => self.set_clip(*c),
                Op::SetXform(x) => self.set_xform(*x),
                Op::PushXform(x) => self.push_xform(*x),
                Op::PopXform => self.pop_xform(),
                Op::FillRadii(r, radii, c) => self.fill_radii(*r, *radii, *c),
                Op::FillOver(r, radius, c) => self.fill_over(*r, *radius, *c),
                Op::Ring(r, radius, w, c, above) => self.ring(*r, *radius, *w, *c, *above),
                Op::Acrylic(r, radius, c, under, noise) => {
                    self.acrylic(*r, *radius, *c, *under, *noise);
                }
                Op::Backdrop(r, radius) => self.backdrop(*r, *radius),
                Op::Text(spec, x, y) => {
                    self.text(spec, *x, *y);
                }
                Op::Stroke(pts, closed, st) => self.stroke(pts, *closed, *st),
                Op::Custom(slot, r) => self.custom(*slot, *r),
            }
        }
    }

    /// Sets the tag the next [`Self::hit`] calls record with their layer, for the `Ui`'s hit list.
    #[allow(dead_code)] // first caller arrives with the Ui frame
    pub(crate) fn set_hit_tag(&mut self, tag: u32) {
        self.hit_tag = tag;
    }

    /// `(layer index, tag)` for each entry of `hits`, in the same order. Valid while `hits` is only
    /// appended to through [`Self::hit`].
    #[allow(dead_code)] // first caller arrives with the Ui frame
    pub(crate) fn hit_meta(&self) -> &[(usize, u32)] {
        &self.hit_meta
    }

    pub fn on(&mut self, layer: LayerId) {
        self.rec(|| Op::On(layer));
        self.layer = layer as usize;
        self.clip = None;
    }

    pub fn set_clip(&mut self, clip: Option<Rect>) {
        self.rec(|| Op::SetClip(clip));
        self.clip = clip;
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

    /// `r` with the transform stack applied and no clip.
    pub fn transformed(&self, r: Rect) -> Rect {
        self.tr(r)
    }

    fn fade(&self, c: [f32; 4]) -> [f32; 4] {
        let a = self.alpha_k();
        c.map(|v| v * a)
    }

    fn clip_phys(&self) -> [f32; 4] {
        self.clip.map_or([0.0; 4], |c| self.tr(c).phys(self.s))
    }

    /// Ask for the blurred scene inside `r` (drawn under this layer).
    pub fn backdrop(&mut self, r: Rect, radius: f32) {
        self.rec(|| Op::Backdrop(r, radius));
        let k = self.scale_k();
        self.scene.backdrop = Some(BackdropRequest {
            rect: self.tr(r).phys(self.s),
            radius: radius * k * self.s,
            alpha: self.alpha_k(),
        });
    }

    fn geo(&mut self) -> &mut LayerGeometry {
        &mut self.scene.layers[self.layer].geometry
    }

    fn shape(&self, r: Rect, radii: [f32; 4], color: [f32; 4]) -> Shape {
        let k = self.scale_k();
        Shape {
            rect: self.tr(r).phys(self.s),
            radii: radii.map(|v| v * self.s * k),
            color: self.fade(color),
            under: [0.0; 4],
            params: [0.0; 4],
            clip: self.clip_phys(),
        }
    }

    /// Fill below this layer's text.
    pub fn fill(&mut self, r: Rect, radius: f32, color: Rgba) {
        self.fill_radii(r, [radius; 4], color);
    }

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

    /// Acrylic layer with Skia `Src` semantics: replaces what is below, `under` only feeds the
    /// antialiased edge. Colours premultiplied. The shape keeps `color` unfaded; the transform
    /// stack's alpha travels as the record's `opacity`, which the renderer blends the layer in
    /// with. Nothing is recorded when that opacity is not above zero.
    pub fn acrylic(&mut self, r: Rect, radius: f32, color: [f32; 4], under: [f32; 4], noise: bool) {
        self.rec(|| Op::Acrylic(r, radius, color, under, noise));
        let opacity = self.alpha_k();
        if opacity.is_nan() || opacity <= 0.0 {
            return;
        }
        let mut shape = self.shape(r, [radius; 4], color);
        shape.color = color;
        shape.under = under;
        shape.params = [0.0, 0.0, 1.0, if noise { 1.0 } else { 0.0 }];
        self.geo().acrylic.push(Acrylic { shape, opacity });
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
        self.scene.layers[self.layer].customs.push(CustomItem {
            slot,
            clip: [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32],
        });
    }

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
        self.scene.layers[self.layer].texts.push(TextItem {
            key,
            x,
            y,
            clip,
            scale,
        });
        size
    }

    /// Text vertically centred in `r` at its left edge (plus `pad`).
    pub fn text_in(&mut self, spec: &Spec, r: Rect, pad: f32) -> (f32, f32) {
        let (_, (w, h)) = self.text.prepare(spec, self.s);
        self.text(spec, r.x + pad, r.y + (r.h - h) / 2.0);
        (w, h)
    }

    pub fn text_centered(&mut self, spec: &Spec, r: Rect) {
        let (_, (w, h)) = self.text.prepare(spec, self.s);
        self.text(spec, r.x + (r.w - w) / 2.0, r.y + (r.h - h) / 2.0);
    }

    pub fn measure(&mut self, spec: &Spec) -> (f32, f32) {
        self.text.measure(spec, self.s)
    }

    pub fn hit(&mut self, r: Rect, h: H) {
        let r = match self.clip {
            Some(c) => r.intersect(&c),
            None => r,
        };
        let r = self.tr(r);
        self.hits.push((r, h));
        self.hit_meta.push((self.layer, self.hit_tag));
    }

    pub fn hovered(&self, h: H) -> bool {
        self.hover == Some(h)
    }

    /// Tessellates a polyline (logical px) into antialiased triangles (physical px).
    pub fn stroke(&mut self, pts: &[[f32; 2]], closed: bool, st: StrokeStyle) {
        self.rec(|| Op::Stroke(pts.to_vec(), closed, st));
        let s = self.s;
        let clip = self.clip_phys();
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
        let out = &mut self.scene.layers[self.layer].geometry.strokes;
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
                }
            };
            let (a0, a1) = (v(i, -1.0, along), v(i, 1.0, along));
            let (b0, b1) = (v(j, -1.0, along + len), v(j, 1.0, along + len));
            out.extend_from_slice(&[a0, a1, b0, a1, b1, b0]);
            along += len;
        }
    }
}

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
}

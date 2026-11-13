//! The display list: plain data the [`Painter`](crate::paint::Painter) builds for one frame and a
//! renderer consumes. Std only; the GPU records are mirrored field by field, in physical pixels.
//!
//! Draw-order contract, which every renderer follows:
//! - layers back to front in [`Scene::in_order`]'s order: each of the seven fixed layers (see
//!   [`LayerId`]), followed by the overlays on it in tag order, which is open order. Everything of
//!   an older overlay draws before anything of a newer one on the same fixed layer, however
//!   their calls fell in the frame, so a kind never interleaves across overlays;
//! - inside a layer: the acrylic shapes (on the fixed `Base` layer only), the under shapes, a frost
//!   among them ([`Shape::is_frost`]), the path fills, the custom items (in
//!   record order), the images (in record order), the text, the over shapes, the strokes;
//! - a layer with [`BackdropRequest`]s is a stop: everything drawn before it is blurred, its
//!   requests draw over that, and the layer follows, so a glass surface blurs everything drawn
//!   before it, other glass included. The layers before [`Scene::floor`] always render offscreen:
//!   a request on one of them stops at the floor instead. Up to the last stop the frame renders
//!   into an offscreen scene; that is then copied to the target and the rest draws there.

use crate::acrylic::NOISE_OPACITY;
use crate::image::{Fit, ImageAlign, ImageId, ImagePart, Sampling};
use crate::paint::Rect;
use crate::text::{TextKey, TextRendering};

/// Draw order, back to front. Each layer interleaves shapes and text correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerId {
    /// The window's own surface: the backdrop of everything else.
    Base = 0,
    /// The app's content.
    Page = 1,
    /// Menus, popovers, dropdowns and tooltips over the page.
    Popup = 2,
    /// Modal dialogs and their scrim.
    Modal = 3,
    /// An overlay opened above an active modal: a popup inside a dialog, or a dialog over a dialog.
    /// The `Ui` picks it at open; nothing asks for it by name. Overlays stacked further share it,
    /// each on a draw layer of its own in open order ([`Scene::in_order`]). Over a modal that is on
    /// `Outside` the overlay is on `Outside` too, so it is above that modal.
    Raised = 4,
    /// A surface that stays outside a modal: above everything a modal dims, blurs or blocks, and
    /// below toasts. The `Ui` puts what [`Ui::outside_modals`](crate::ui::Ui::outside_modals)
    /// scopes here; an overlay opened inside the scope stays on it, above the scope's own drawing
    /// by tag.
    Outside = 5,
    /// Toasts: above modal dialogs, so a toast raised by a dialog stays visible.
    Toast = 6,
}

/// The number of fixed layers: one per [`LayerId`].
pub const LAYER_COUNT: usize = 7;

/// The layers always drawn to the offscreen scene (see [`Scene::floor`]): those under popups, so a
/// glass menu can blur them.
pub const SCENE_LAYERS: usize = 2;

/// One rounded-rect instance (fill, ring or acrylic).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Shape {
    pub rect: [f32; 4],
    pub radii: [f32; 4],
    pub color: [f32; 4],
    pub under: [f32; 4],
    pub params: [f32; 4],
    /// Physical clip rect (x, y, w, h); zero width = no clip.
    pub clip: [f32; 4],
    /// Physical rect (x, y, w, h) of the innermost rounded clip; used only when `round_radius`
    /// is above zero. Cuts coverage along a rounded rect, on top of `clip`.
    pub round: [f32; 4],
    /// Corner radius in physical px of the rounded clip `round`; 0 means none.
    pub round_radius: f32,
}

impl Shape {
    /// Whether this is a frost ([`Painter::frost`](crate::paint::Painter::frost)): the colour is
    /// unfaded, `params[1]` its alpha, `params[3]` the noise flag.
    pub fn is_frost(&self) -> bool {
        self.params[2] > 1.5
    }
}

/// One vertex of a tessellated, antialiased stroke (triangle list).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StrokeVertex {
    pub pos: [f32; 2],
    pub aa: [f32; 2],
    pub color: [f32; 4],
    pub style: [f32; 4],
    pub extra: [f32; 4],
    pub clip: [f32; 4],
    /// As [`Shape::round`].
    pub round: [f32; 4],
    /// As [`Shape::round_radius`].
    pub round_radius: f32,
}

/// One acrylic layer, the window's own background: only the fixed [`LayerId::Base`] holds these,
/// since nothing drawn in a window may replace what is below it. `shape.color` is the unfaded colour; `opacity` (0 to 1) is how much of the
/// layer replaces what is below: the result is `opacity·layer + (1 − opacity)·below`, so 1.0 is a
/// plain replace and a smaller value fades the layer in without punching a hole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Acrylic {
    pub shape: Shape,
    pub opacity: f32,
}

/// One z-layer of the frame: acrylic (replace blend, or a constant blend when faded), shapes under
/// text, path fills, text (drawn by the caller), then shapes and strokes over text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayerGeometry {
    pub acrylic: Vec<Acrylic>,
    pub under: Vec<Shape>,
    pub over: Vec<Shape>,
    pub strokes: Vec<StrokeVertex>,
    /// Filled polygons as triangle lists with an antialiasing fringe (`aa.x` 0 inside, `half + 1`
    /// at the fringe), drawn like strokes but below the text, after the under shapes.
    pub fills: Vec<StrokeVertex>,
}

/// One text draw: the shaped text's key, its position and clip, and a glyph scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextItem {
    pub key: TextKey,
    pub x: f32,
    pub y: f32,
    pub clip: Rect,
    /// Glyph scale relative to the shaped size (popup open/close animation).
    pub scale: f32,
}

/// One recorded custom draw: which supplied implementation, and where it may paint.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CustomItem {
    /// Index into the `customs` slice given to `draw_with`.
    pub slot: usize,
    /// Clip as x, y, width, height in physical pixels.
    pub clip: [u32; 4],
}

/// Where an [`ImageItem`]'s pixels come from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageSource {
    /// A picture the app registered with the renderer's [`ImageStore`](crate::image::ImageStore).
    Id(ImageId),
    /// A shaped text, rasterised once by the renderer and kept while the text is cached. The
    /// number is the glyph scale relative to the shaped size, as [`TextItem::scale`]; the
    /// picture is drawn at its raster size times that, centred on `dst`.
    Text(TextKey, f32),
}

/// A rounded clip: the rect (physical px) and corner radius; radius 0 means none.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RoundClip {
    pub rect: [f32; 4],
    pub radius: f32,
}

/// One recorded image draw. All rects are physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct ImageItem {
    pub source: ImageSource,
    /// The destination rect: for an [`ImageSource::Id`] the rect `fit` and `align` place the
    /// picture in; for text the rect the shaped text would fill, whose centre is the draw's.
    pub dst: Rect,
    pub fit: Fit,
    pub align: ImageAlign,
    /// Opacity multiplier, 0 to 1 (the transform stack's alpha is folded in).
    pub alpha: f32,
    /// Radians, clockwise on screen, about the centre of the drawn picture; 0 for none.
    pub rotation: f32,
    /// Physical clip rect (x, y, w, h); zero width = no clip.
    pub clip: [f32; 4],
    pub round_clip: RoundClip,
    /// The part of the picture to draw, in the picture's own pixels; `None` is the whole picture.
    /// With a part, `fit` and `align` place the part's size in `dst`. Text rasters carry `None`.
    pub part: Option<ImagePart>,
    /// How the renderer samples the picture; text rasters carry [`Sampling::Auto`].
    pub sampling: Sampling,
}

/// One layer's items, grouped by kind, each kind in record order. They draw as: acrylic, under
/// shapes, path fills, customs, images, text, then the over shapes and strokes (the geometry that
/// draws above the text).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layer {
    pub geometry: LayerGeometry,
    pub texts: Vec<TextItem>,
    /// App-owned draws, in record order; drawn after the under shapes, before the images.
    pub customs: Vec<CustomItem>,
    /// Image draws, in record order; drawn after the customs, before the text.
    pub images: Vec<ImageItem>,
    /// Blurred-backdrop regions, in record order; each blurs what is drawn before this layer and
    /// draws under everything of it.
    pub backdrops: Vec<BackdropRequest>,
}

impl Layer {
    /// Adds `other`'s items after this layer's, kind by kind.
    fn append(&mut self, other: &Layer) {
        let (g, o) = (&mut self.geometry, &other.geometry);
        g.acrylic.extend_from_slice(&o.acrylic);
        g.under.extend_from_slice(&o.under);
        g.over.extend_from_slice(&o.over);
        g.strokes.extend_from_slice(&o.strokes);
        g.fills.extend_from_slice(&o.fills);
        self.texts.extend_from_slice(&other.texts);
        self.customs.extend_from_slice(&other.customs);
        self.images.extend_from_slice(&other.images);
        self.backdrops.extend_from_slice(&other.backdrops);
    }
}

/// A blurred-backdrop region to draw, in physical pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BackdropRequest {
    pub rect: [f32; 4],
    pub radius: f32,
    pub alpha: f32,
    /// Gaussian σ of this blur, as [`FrameParams::blur_sigma`]; 0 means the frame's.
    pub sigma: f32,
    /// Saturation boost of this blur, as [`FrameParams::saturation`]; 0 means the frame's.
    pub saturation: f32,
}

/// What one overlay drew: a draw layer of its own, after the fixed layer it sits on.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayLayer {
    /// The fixed layer this one draws after.
    pub on: LayerId,
    /// The overlay's tag: overlays on one fixed layer draw in tag order, which is open order.
    pub tag: u32,
    pub layer: Layer,
}

/// One painted frame: every layer. [`Self::in_order`] is the draw order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    /// What was drawn on each [`LayerId`] outside any overlay, indexed by `id as usize`.
    pub layers: [Layer; LAYER_COUNT],
    /// What each overlay drew, in first-drawn order, which need not be draw order.
    pub overlays: Vec<OverlayLayer>,
}

impl Scene {
    /// Every layer back to front: each fixed layer, then the overlays on it in tag order, whatever
    /// order they drew in. This is the one list a renderer walks.
    pub fn in_order(&self) -> Vec<&Layer> {
        let mut overlays: Vec<&OverlayLayer> = self.overlays.iter().collect();
        overlays.sort_by_key(|o| (o.on as usize, o.tag));
        let mut rest = overlays.as_slice();
        let mut out = Vec::with_capacity(LAYER_COUNT + rest.len());
        for (i, layer) in self.layers.iter().enumerate() {
            out.push(layer);
            let n = rest.iter().take_while(|o| o.on as usize == i).count();
            let (on_this, later) = rest.split_at(n);
            out.extend(on_this.iter().map(|o| &o.layer));
            rest = later;
        }
        out
    }

    /// The fixed layer `id` and the overlays on it as one layer, each kind concatenated in draw
    /// order: for tests and inspection, since the kinds of two overlays do not interleave when
    /// drawn.
    pub fn merged(&self, id: LayerId) -> Layer {
        let mut overlays: Vec<&OverlayLayer> =
            self.overlays.iter().filter(|o| o.on == id).collect();
        overlays.sort_by_key(|o| o.tag);
        let mut out = self.layers[id as usize].clone();
        for o in overlays {
            out.append(&o.layer);
        }
        out
    }

    /// Every backdrop request of the frame, in draw order.
    pub fn backdrops(&self) -> impl Iterator<Item = &BackdropRequest> {
        self.in_order().into_iter().flat_map(|l| &l.backdrops)
    }

    /// How many layers of [`Self::in_order`] always draw to the offscreen scene: those before the
    /// fixed layer [`SCENE_LAYERS`]. The first stop a request can make is this place, so a request
    /// on one of those layers blurs them all and draws over them.
    pub fn floor(&self) -> usize {
        let below = self.overlays.iter();
        SCENE_LAYERS + below.filter(|o| (o.on as usize) < SCENE_LAYERS).count()
    }
}

/// What one frame is drawn at, and how its glass backdrop is blurred.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameParams {
    /// Target size in physical pixels.
    pub size: (u32, u32),
    /// Device pixels per logical pixel.
    pub scale: f32,
    /// Gaussian σ of the backdrop blur, in physical pixels.
    pub blur_sigma: f32,
    /// Saturation boost applied to the blurred backdrop.
    pub saturation: f32,
    /// Opacity of the noise texture on acrylic and frost. [`NOISE_OPACITY`] is Avalonia's; 0 turns
    /// the noise off. A value that is not finite or is negative draws as the default.
    pub noise_opacity: f32,
    /// The limits of the backdrop blur.
    pub blur: BlurLimits,
    /// How glyphs are rasterized this frame: smoothing, hinting and stem darkening. The renderer
    /// reads [`TextRendering::effective`] once per draw; a change takes effect on the next frame
    /// and no cached glyph from another setting is ever drawn.
    pub text: TextRendering,
}

impl Default for FrameParams {
    /// A zero-sized frame with Avalonia's noise, the blur's default limits and
    /// [`TextRendering::SMOOTH`] text.
    fn default() -> Self {
        Self {
            size: (0, 0),
            scale: 0.0,
            blur_sigma: 0.0,
            saturation: 0.0,
            noise_opacity: NOISE_OPACITY,
            blur: BlurLimits::default(),
            text: TextRendering::SMOOTH,
        }
    }
}

impl FrameParams {
    /// [`Self::noise_opacity`] as the renderer draws it: a value that is not finite or is
    /// negative is [`NOISE_OPACITY`], never clamped.
    pub fn noise(&self) -> f32 {
        if self.noise_opacity.is_finite() && self.noise_opacity >= 0.0 {
            self.noise_opacity
        } else {
            NOISE_OPACITY
        }
    }
}

/// The limits of the backdrop blur: how coarse its working picture is and how wide its kernel may
/// grow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlurLimits {
    /// The widest the Gaussian kernel grows, in texels of the working picture on each side of the
    /// centre. A kernel is three σ wide, so a larger σ needs a larger cap to stay a Gaussian; a
    /// capped one is cut off and so spreads less.
    pub kernel_cap: f32,
    /// The working picture is the frame at `1 / downsample` of its size: 2, 4 or 8. A smaller
    /// divisor keeps more detail and costs more.
    pub downsample: u32,
}

impl BlurLimits {
    /// The default kernel cap, in texels.
    pub const KERNEL_CAP: f32 = 48.0;
    /// The widest kernel cap [`Self::effective`] allows, in texels.
    pub const KERNEL_CAP_MAX: f32 = 128.0;
    /// The default divisor of the working picture.
    pub const DOWNSAMPLE: u32 = 8;

    /// The limits as the renderer uses them. A `downsample` other than 2, 4 or 8 is 8; a cap that
    /// is not finite or not positive is 48, and one over 128 is 128.
    pub fn effective(self) -> BlurLimits {
        let downsample = match self.downsample {
            d @ (2 | 4 | 8) => d,
            _ => Self::DOWNSAMPLE,
        };
        let kernel_cap = if self.kernel_cap.is_finite() && self.kernel_cap > 0.0 {
            self.kernel_cap.min(Self::KERNEL_CAP_MAX)
        } else {
            Self::KERNEL_CAP
        };
        BlurLimits {
            kernel_cap,
            downsample,
        }
    }
}

impl Default for BlurLimits {
    /// Cap 48, divisor 8: what the toolkit blurred with before these were values.
    fn default() -> Self {
        Self {
            kernel_cap: Self::KERNEL_CAP,
            downsample: Self::DOWNSAMPLE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_values_the_toolkit_drew_with() {
        let p = FrameParams::default();
        assert_eq!(p.noise_opacity, NOISE_OPACITY);
        assert_eq!(p.noise_opacity, 0.0225);
        assert_eq!(p.text, TextRendering::SMOOTH);
        assert_eq!((p.blur.kernel_cap, p.blur.downsample), (48.0, 8));
        assert_eq!(
            p.blur,
            p.blur.effective(),
            "the default is already effective"
        );
    }

    #[test]
    fn a_noise_that_is_not_a_strength_is_the_default_and_zero_is_none() {
        let with = |noise_opacity| FrameParams {
            noise_opacity,
            ..FrameParams::default()
        };
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.001, -1.0] {
            assert_eq!(with(bad).noise(), NOISE_OPACITY, "noise {bad}");
        }
        assert_eq!(with(0.0).noise(), 0.0);
        assert_eq!(with(0.05).noise(), 0.05);
    }

    #[test]
    fn bad_blur_limits_fail_to_the_default_and_never_truncate() {
        let with = |kernel_cap, downsample| BlurLimits {
            kernel_cap,
            downsample,
        };
        for d in [2, 4, 8] {
            assert_eq!(with(48.0, d).effective().downsample, d);
        }
        for d in [0, 1, 3, 5, 6, 7, 9, 16, u32::MAX] {
            assert_eq!(with(48.0, d).effective().downsample, 8, "downsample {d}");
        }
        for cap in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0, -5.0] {
            assert_eq!(with(cap, 8).effective().kernel_cap, 48.0, "cap {cap}");
        }
        assert_eq!(with(8.0, 8).effective().kernel_cap, 8.0);
        assert_eq!(with(128.0, 8).effective().kernel_cap, 128.0);
        assert_eq!(with(128.5, 8).effective().kernel_cap, 128.0);
        assert_eq!(with(1.0e9, 8).effective().kernel_cap, 128.0);
    }
}

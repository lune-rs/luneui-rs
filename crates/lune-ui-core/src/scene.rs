//! The display list: plain data the [`Painter`](crate::paint::Painter) builds for one frame and a
//! renderer consumes. Std only; the GPU records are mirrored field by field, in physical pixels.
//!
//! Draw-order contract, which every renderer follows:
//! - layers back to front in [`Scene::in_order`]'s order: each of the six fixed layers (see
//!   [`LayerId`]), followed by the overlays on it in tag order, which is open order. Everything of
//!   an older overlay draws before anything of a newer one on the same fixed layer, however
//!   their calls fell in the frame, so a kind never interleaves across overlays;
//! - inside a layer: the acrylic shapes, the under shapes, the path fills, the custom items (in
//!   record order), the images (in record order), the text, the over shapes, the strokes;
//! - the layers before [`Scene::split`] in that order render into an offscreen scene, the one a
//!   backdrop blurs; that scene is then copied to the target, the [`BackdropRequest`] (if any) is
//!   drawn over it, and the remaining layers follow. The split is at the fixed layer
//!   [`SCENE_LAYERS`], or at the backdrop's own layer when that is later, so a glass surface
//!   blurs everything drawn before it.

use crate::image::{Fit, ImageAlign, ImageId};
use crate::paint::Rect;
use crate::text::TextKey;

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
    /// each on a draw layer of its own in open order ([`Scene::in_order`]).
    Raised = 4,
    /// Toasts: above modal dialogs, so a toast raised by a dialog stays visible.
    Toast = 5,
}

pub const LAYER_COUNT: usize = 6;

/// The layers always drawn to the offscreen scene (see [`Scene::split`]): those under popups, so a
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

/// One acrylic layer. `shape.color` is the unfaded colour; `opacity` (0 to 1) is how much of the
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
    }
}

/// A blurred-backdrop region to draw, in physical pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BackdropRequest {
    pub rect: [f32; 4],
    pub radius: f32,
    pub alpha: f32,
    /// The index of the fixed layer the region sits under, with `tag` naming the overlay on it
    /// (0 for the fixed layer itself): everything drawn before that layer is blurred.
    pub layer: usize,
    /// The overlay the region belongs to, as [`OverlayLayer::tag`]; 0 for none.
    pub tag: u32,
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

/// One painted frame: every layer, and the glass backdrop request. [`Self::in_order`] is the draw
/// order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    /// What was drawn on each [`LayerId`] outside any overlay, indexed by `id as usize`.
    pub layers: [Layer; LAYER_COUNT],
    /// What each overlay drew, in first-drawn order, which need not be draw order.
    pub overlays: Vec<OverlayLayer>,
    /// Blurred-backdrop region requested by a glass surface (physical px). One per frame: of two
    /// requests the lower layer keeps it, and on one layer the last wins.
    pub backdrop: Option<BackdropRequest>,
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

    /// How many layers of [`Self::in_order`] draw to the offscreen scene, before the backdrop: those
    /// before the fixed layer [`SCENE_LAYERS`], or before the backdrop's own layer when that is
    /// later.
    pub fn split(&self) -> usize {
        let floor = self.position(SCENE_LAYERS, 0);
        self.backdrop
            .map_or(floor, |b| self.position(b.layer, b.tag).max(floor))
    }

    /// Where the fixed layer `on` (`tag` 0) or its overlay `tag` comes in [`Self::in_order`].
    fn position(&self, on: usize, tag: u32) -> usize {
        let before = self.overlays.iter();
        let before = before.filter(|o| (o.on as usize, o.tag) < (on, tag));
        on + usize::from(tag > 0) + before.count()
    }
}

/// What one frame is drawn at, and how its glass backdrop is blurred.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FrameParams {
    /// Target size in physical pixels.
    pub size: (u32, u32),
    /// Device pixels per logical pixel.
    pub scale: f32,
    /// Gaussian σ of the backdrop blur, in physical pixels.
    pub blur_sigma: f32,
    /// Saturation boost applied to the blurred backdrop.
    pub saturation: f32,
}

//! The display list: plain data the [`Painter`](crate::paint::Painter) builds for one frame and a
//! renderer consumes. Std only; the GPU records are mirrored field by field, in physical pixels.
//!
//! Draw-order contract, which every renderer follows:
//! - layers back to front, `layers[0]` first;
//! - inside a layer: the acrylic shapes, the under shapes, the custom items (in record order),
//!   the text, the over shapes, the strokes;
//! - layers below [`SCENE_LAYERS`] render into an offscreen scene, the one a backdrop blurs; that
//!   scene is then copied to the target, the [`BackdropRequest`] (if any) is drawn over it, and
//!   the remaining layers follow.

use crate::paint::Rect;
use crate::text::TextKey;

/// Draw order, back to front. Each layer interleaves shapes and text correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerId {
    Base = 0,
    Page = 1,
    Popup = 2,
    Modal = 3,
}

pub const LAYER_COUNT: usize = 4;

/// Layers under popups go to an offscreen scene, so a glass menu can blur them.
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
}

/// One z-layer of the frame: acrylic (replace blend), shapes under text, text (drawn by the
/// caller), then shapes and strokes over text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayerGeometry {
    pub acrylic: Vec<Shape>,
    pub under: Vec<Shape>,
    pub over: Vec<Shape>,
    pub strokes: Vec<StrokeVertex>,
}

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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layer {
    pub geometry: LayerGeometry,
    pub texts: Vec<TextItem>,
    /// App-owned draws, in record order; drawn after the under shapes, before the text.
    pub customs: Vec<CustomItem>,
}

/// A blurred-backdrop region to draw, in physical pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BackdropRequest {
    pub rect: [f32; 4],
    pub radius: f32,
    pub alpha: f32,
}

/// One painted frame: every layer, back to front, and the glass backdrop request.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    pub layers: [Layer; LAYER_COUNT],
    /// Blurred-backdrop region requested by a glass menu (physical px).
    pub backdrop: Option<BackdropRequest>,
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

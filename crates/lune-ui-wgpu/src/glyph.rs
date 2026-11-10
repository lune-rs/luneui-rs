//! The glyph renderer: a glyph atlas, the text pipeline and the prepare and render steps that
//! draw shaped text into a render pass.
//!
//! Vendored from glyphon 0.12.0, `src/lib.rs`, `src/error.rs` and the `ContentType` type of
//! `src/custom_glyph.rs`, MIT (`licenses/glyphon-LICENSE-MIT`); changes: items are `pub(crate)`,
//! custom glyphs, the depth callback and the colour-mode choice are removed (the atlas keeps the
//! web colour mode the toolkit always used), `RenderError` and the unused `Default` of `TextBounds`
//! are removed, the cosmic-text re-exports are dropped, the vertex and uniform structs derive
//! `bytemuck` traits (the two raw byte casts of the original are replaced by `bytemuck` calls);
//! atlas entries are keyed by `GlyphKey` (cosmic-text's key and the glyph's `GlyphParams`) and
//! rasterized through `GlyphRasterizer`.

mod atlas;
mod cache;
mod render;
mod viewport;

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use bytemuck::{Pod, Zeroable};
use cosmic_text::{Buffer, CacheKey, Color};
use etagere::AllocId;
use lune_ui_core::text::GlyphParams;
use wgpu::{Device, Queue};

pub(crate) use atlas::TextAtlas;
pub(crate) use cache::Cache;
pub(crate) use render::TextRenderer;
pub(crate) use viewport::Viewport;

/// An error that occurred while preparing text for rendering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PrepareError {
    /// The glyph texture atlas is at its largest size and has no room.
    AtlasFull,
}

impl Display for PrepareError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Prepare error: glyph texture atlas is full")
    }
}

impl Error for PrepareError {}

/// The type of image data contained in a rasterized glyph.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ContentType {
    /// Each pixel contains 32 bits of rgba data.
    Color,
    /// Each pixel contains a single 8 bit channel.
    Mask,
}

/// What an atlas entry is keyed by: cosmic-text's glyph key and the `GlyphParams` the glyph was
/// rasterized with, so two settings coexist in an atlas and a re-upload (`grow`) re-creates each
/// entry from its own key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GlyphKey {
    pub(crate) glyph: CacheKey,
    pub(crate) params: GlyphParams,
}

pub(crate) enum GpuCacheStatus {
    InAtlas {
        x: u16,
        y: u16,
        content_type: ContentType,
    },
    SkipRasterization,
}

pub(crate) struct GlyphDetails {
    width: u16,
    height: u16,
    gpu_cache: GpuCacheStatus,
    atlas_id: Option<AllocId>,
    top: i16,
    left: i16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct GlyphToRender {
    pos: [i32; 2],
    dim: [u16; 2],
    uv: [u16; 2],
    color: u32,
    content_type_with_srgb: [u16; 2],
    depth: f32,
}

/// The screen resolution to use when rendering text.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Pod, Zeroable)]
pub(crate) struct Resolution {
    /// The width of the screen in pixels.
    pub(crate) width: u32,
    /// The height of the screen in pixels.
    pub(crate) height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Pod, Zeroable)]
pub(crate) struct Params {
    screen_resolution: Resolution,
    _pad: [u32; 2],
}

/// Controls the visible area of the text. Any text outside of the visible area will be clipped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TextBounds {
    /// The position of the left edge of the visible area.
    pub(crate) left: i32,
    /// The position of the top edge of the visible area.
    pub(crate) top: i32,
    /// The position of the right edge of the visible area.
    pub(crate) right: i32,
    /// The position of the bottom edge of the visible area.
    pub(crate) bottom: i32,
}

/// A text area containing text to be rendered along with its overflow behavior.
#[derive(Clone)]
pub(crate) struct TextArea<'a> {
    /// The buffer containing the text to be rendered.
    pub(crate) buffer: &'a Buffer,
    /// The left edge of the buffer.
    pub(crate) left: f32,
    /// The top edge of the buffer.
    pub(crate) top: f32,
    /// The scaling to apply to the buffer.
    pub(crate) scale: f32,
    /// The visible bounds of the text area. This is used to clip the text and doesn't have to
    /// match the `left` and `top` values.
    pub(crate) bounds: TextBounds,
    /// The default color of the text area.
    pub(crate) default_color: Color,
}

pub(crate) struct State<'a> {
    pub(crate) device: &'a Device,
    pub(crate) queue: &'a Queue,
}

//! App-agnostic glass UI toolkit: the facade. It re-exports the five crates under the paths the
//! toolkit has always had, so apps write `lune_ui::…` whichever crate holds the item:
//!
//! - `lune-ui-core`: the painter, display list, widgets, theme and seam types (no GPU or window);
//! - `lune-ui-wgpu`: the shared GPU, frame renderers, window surfaces, dumps and glyphon text;
//! - `lune-ui-winit`: window attributes, input translation and output;
//! - `lune-ui-platform`: the OS glass material.
//!
//! The app owns the winit event loop; the toolkit never starts one and never reads the clock or
//! the environment, so frames are deterministic.

#![forbid(unsafe_code)]

pub use lune_ui_core::{Backdrop, BackdropKind, GlassLayer};
pub use lune_ui_core::{
    access, acrylic, backdrop, chrome, code_doc, color, drag, icon, image, json, layout, motion,
    paint, scene, shortcuts, strings, testing, text_edit, theme, ui, widgets,
};
pub use theme::lunegit_dark;

pub use lune_ui_wgpu::{blur, custom, dump, frame, gpu, surface};

pub use lune_ui_platform::material;

/// Text: the core's specs and `TextSystem` seam, and the glyphon `TextCache` that implements it.
pub mod text {
    pub use lune_ui_core::text::*;
    pub use lune_ui_wgpu::text_cache::TextCache;
}

/// Window helpers: the core's edge resize, double click and caption buttons, and the winit glass
/// attributes and resize direction.
pub mod window {
    pub use lune_ui_core::window::*;
    pub use lune_ui_winit::window::*;
}

/// Input: the core's plain-data `RawInput` and `FrameOutput`, and the winit translation and output.
pub mod input {
    pub use lune_ui_core::input::*;
    pub use lune_ui_winit::input::*;
}

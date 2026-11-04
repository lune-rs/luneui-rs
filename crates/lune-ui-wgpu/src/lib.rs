//! The wgpu renderer of the glass UI toolkit: one [`gpu::Gpu`] (device, pipelines, noise texture and
//! glyph atlas per format) shared by any number of targets. Each target is a
//! [`frame::FrameRenderer`] that draws the core's `Scene`; a window adds a [`surface::WindowSurface`],
//! and frame dumps draw into an offscreen texture ([`dump`]). Text shaping and rendering use glyphon
//! ([`text_cache`] implements the core's `TextSystem`).
//!
//! Modules:
//! - [`gpu`] (the GPU every target shares), [`frame`] (one target's renderer: the scene → blur →
//!   target pass sequence), [`blur`] (backdrop blur), [`surface`] (a window's swapchain), [`dump`]
//!   (offscreen targets and PNG readback), [`custom`] (app-owned GPU drawing inside a layer);
//! - [`text_cache`] (the glyphon text system behind the core's `TextSystem` seam).
//!
//! The per-platform choices (the Windows DX12 backend) are in the private `platform` module.

#![forbid(unsafe_code)]

pub mod blur;
pub mod custom;
pub mod dump;
pub mod frame;
pub mod gpu;
mod images;
mod platform;
pub mod surface;
pub mod text_cache;

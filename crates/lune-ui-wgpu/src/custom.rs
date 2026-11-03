//! Custom draw items: app-owned GPU drawing that sits inside a painted layer.
//!
//! The painter records a [`CustomItem`] with [`Painter::custom`](lune_ui_core::paint::Painter::custom).
//! [`FrameRenderer::draw_with`](crate::frame::FrameRenderer::draw_with) draws it after that
//! layer's acrylic and under shapes and before its text, with the scissor set to the item's
//! clip. Because the item lives in a layer, a popup's backdrop blur sees it when the layer is
//! below the popup.

pub use lune_ui_core::scene::CustomItem;

/// What the current frame is drawn at; handed to [`CustomDraw`] on both calls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomFrame {
    /// Target size in physical pixels.
    pub size: (u32, u32),
    /// Device pixels per logical pixel.
    pub scale: f32,
    /// Colour format of the pass the item renders into.
    pub format: wgpu::TextureFormat,
}

/// App-owned drawing. The app passes one implementation per slot to
/// [`FrameRenderer::draw_with`](crate::frame::FrameRenderer::draw_with).
pub trait CustomDraw {
    /// Called once per frame, before the first render pass, for every supplied implementation
    /// (whether or not an item refers to it). Upload buffers and textures here.
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, frame: &CustomFrame);

    /// Records draw calls into `pass`, once for every item that refers to this slot (the scene
    /// pass and the final pass are separate; a slot is normally used by one layer only). The
    /// scissor is already set to the item's clip.
    fn render(&self, pass: &mut wgpu::RenderPass<'_>, frame: &CustomFrame);
}

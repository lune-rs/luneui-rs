//! A window's swapchain: the surface, its configuration, and acquire / present. Windowing-free:
//! the app creates the `wgpu::Surface` from any window handle (`Instance::create_surface`), asks
//! for a redraw when [`acquire`](WindowSurface::acquire) says so, and tells its window a frame is
//! about to be presented before [`present`](WindowSurface::present). [`SurfaceOptions`] sets the
//! present mode and the frame latency ([`WindowSurface::with_options`],
//! [`WindowSurface::set_options`]); the defaults are 0.4.0's.

use crate::gpu::Gpu;

/// What [`WindowSurface::acquire`] got.
#[derive(Debug)]
pub enum Acquired {
    /// The texture to draw this frame into.
    Frame(wgpu::SurfaceTexture),
    /// Skip this frame (hidden or minimised window, or a timeout); the platform sends a redraw
    /// when the window is visible again.
    Skip,
    /// The surface was outdated or lost and has been reconfigured; draw again on a new redraw.
    Redraw,
}

/// How a [`WindowSurface`] presents (D299). The default is `Fifo` with a latency of 2.
///
/// `Fifo` waits for the display, and [`WindowSurface::acquire`] blocks while every drawable is
/// queued, so a run of frames asked for at once waits there for up to a refresh interval each.
/// An app that draws only when a frame is due can pick a mode that does not wait.
/// `AutoNoVsync` becomes the first of `Immediate`, `Mailbox` and `Fifo` that the surface lists
/// (wgpu's order), and a mode that does not wait for the display can tear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceOptions {
    /// The present mode. A mode the surface does not list falls back to `Fifo`; `AutoVsync` and
    /// `AutoNoVsync` are resolved by wgpu.
    pub present_mode: wgpu::PresentMode,
    /// wgpu's `desired_maximum_frame_latency`: how many frames may be queued ahead of the
    /// display. wgpu clamps it to the backend's range (Metal: 1 to 2).
    pub frame_latency: u32,
}

impl Default for SurfaceOptions {
    fn default() -> Self {
        Self {
            present_mode: wgpu::PresentMode::Fifo,
            frame_latency: 2,
        }
    }
}

/// The mode to configure for `want`: `AutoVsync` and `AutoNoVsync` pass (wgpu resolves them), a
/// mode in `supported` passes, any other becomes `Fifo`, which every surface offers.
fn resolve_present_mode(
    want: wgpu::PresentMode,
    supported: &[wgpu::PresentMode],
) -> wgpu::PresentMode {
    use wgpu::PresentMode::{AutoNoVsync, AutoVsync, Fifo};
    if matches!(want, AutoVsync | AutoNoVsync) || supported.contains(&want) {
        want
    } else {
        Fifo
    }
}

/// One window's surface, configured for a [`Gpu`].
pub struct WindowSurface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    /// The present modes the surface lists, read once at creation.
    present_modes: Vec<wgpu::PresentMode>,
}

impl std::fmt::Debug for WindowSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowSurface")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl WindowSurface {
    /// Configures `surface` at `size` physical pixels with [`SurfaceOptions::default`]. The
    /// format prefers `Bgra8Unorm`, then `Rgba8Unorm`; the alpha mode prefers premultiplied, then
    /// post-multiplied.
    pub fn new(
        gpu: &Gpu,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
    ) -> Result<Self, String> {
        Self::with_options(gpu, surface, size, SurfaceOptions::default())
    }

    /// [`Self::new`] with the present mode and frame latency of `options`; a mode the surface
    /// does not list becomes `Fifo` ([`Self::options`] reports what was configured).
    pub fn with_options(
        gpu: &Gpu,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
        options: SurfaceOptions,
    ) -> Result<Self, String> {
        let caps = surface.get_capabilities(&gpu.adapter);
        let (Some(&first_format), Some(&first_alpha)) =
            (caps.formats.first(), caps.alpha_modes.first())
        else {
            return Err("surface: not supported by the adapter".into());
        };
        // Blend in gamma space like Skia/Avalonia, so colours match Avalonia.
        let format = [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
        ]
        .into_iter()
        .find(|f| caps.formats.contains(f))
        .unwrap_or(first_format);
        // Our output is premultiplied. DirectComposition (Windows) offers PreMultiplied; Metal only
        // offers PostMultiplied, which CoreAnimation composites as premultiplied anyway.
        let alpha_mode = [
            wgpu::CompositeAlphaMode::PreMultiplied,
            wgpu::CompositeAlphaMode::PostMultiplied,
        ]
        .into_iter()
        .find(|m| caps.alpha_modes.contains(m))
        .unwrap_or(first_alpha);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: resolve_present_mode(options.present_mode, &caps.present_modes),
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: options.frame_latency,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&gpu.device, &config);
        Ok(Self {
            surface,
            config,
            present_modes: caps.present_modes,
        })
    }

    /// What the surface is configured with: the present mode actually in use (after the fallback
    /// to `Fifo`) and the frame latency requested.
    pub fn options(&self) -> SurfaceOptions {
        SurfaceOptions {
            present_mode: self.config.present_mode,
            frame_latency: self.config.desired_maximum_frame_latency,
        }
    }

    /// Reconfigures the surface with `options` at the current size and format. Call it between
    /// frames, never while a texture from [`Self::acquire`] is alive (present or drop it first):
    /// wgpu refuses to configure a surface with an outstanding texture and its default error
    /// handler panics.
    pub fn set_options(&mut self, gpu: &Gpu, options: SurfaceOptions) {
        self.config.present_mode = resolve_present_mode(options.present_mode, &self.present_modes);
        self.config.desired_maximum_frame_latency = options.frame_latency;
        self.surface.configure(&gpu.device, &self.config);
    }

    /// The surface's colour format: make the window's
    /// [`FrameRenderer`](crate::frame::FrameRenderer) with it.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// The configured size in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// The chosen composite alpha mode.
    pub fn alpha_mode(&self) -> wgpu::CompositeAlphaMode {
        self.config.alpha_mode
    }

    /// Reconfigures the surface to `width` by `height` physical px, each at least 1. Call it when
    /// the window resizes, between frames and never while a texture from [`Self::acquire`] is
    /// alive (present or drop it first: wgpu refuses to configure a surface with an outstanding
    /// texture and its default error handler panics); [`Self::acquire`] does it itself for an
    /// outdated surface.
    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&gpu.device, &self.config);
    }

    /// The next surface texture. An outdated, suboptimal or lost surface is reconfigured (a
    /// suboptimal texture is dropped, not drawn) and [`Acquired::Redraw`] returned; a hidden or
    /// minimised window, or a timeout, gives [`Acquired::Skip`].
    ///
    /// Under `Fifo` a frame started before the display can take one waits here for up to a
    /// refresh interval (see [`SurfaceOptions`]).
    pub fn acquire(&mut self, gpu: &Gpu) -> Result<Acquired, String> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => Ok(Acquired::Frame(t)),
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                // The texture must not be alive when the surface is configured (D11).
                drop(texture);
                let (w, h) = self.size();
                self.resize(gpu, w, h);
                Ok(Acquired::Redraw)
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                let (w, h) = self.size();
                self.resize(gpu, w, h);
                Ok(Acquired::Redraw)
            }
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                Ok(Acquired::Skip)
            }
            other => Err(format!("surface: {other:?}")),
        }
    }

    /// Shows a finished frame. Tell the window a frame is coming first (the windowing library's
    /// pre-present notification).
    pub fn present(&self, gpu: &Gpu, frame: wgpu::SurfaceTexture) {
        gpu.queue.present(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::PresentMode::{AutoNoVsync, AutoVsync, Fifo, Immediate, Mailbox};

    #[test]
    fn the_default_is_fifo_with_a_latency_of_2() {
        let d = SurfaceOptions::default();
        assert_eq!((d.present_mode, d.frame_latency), (Fifo, 2));
    }

    #[test]
    fn a_mode_the_surface_lists_passes_and_any_other_becomes_fifo() {
        assert_eq!(resolve_present_mode(Mailbox, &[Fifo, Immediate]), Fifo);
        assert_eq!(
            resolve_present_mode(Immediate, &[Fifo, Immediate]),
            Immediate
        );
        assert_eq!(resolve_present_mode(Fifo, &[]), Fifo);
    }

    #[test]
    fn the_auto_modes_pass_through_for_wgpu_to_resolve() {
        assert_eq!(resolve_present_mode(AutoNoVsync, &[Fifo]), AutoNoVsync);
        assert_eq!(resolve_present_mode(AutoVsync, &[]), AutoVsync);
    }
}

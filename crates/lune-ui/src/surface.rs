//! A window's swapchain: the surface, its configuration, and acquire / present. Windowing-free:
//! the app creates the `wgpu::Surface` from any window handle (`Instance::create_surface`), asks
//! for a redraw when [`acquire`](WindowSurface::acquire) says so, and tells its window a frame is
//! about to be presented before [`present`](WindowSurface::present).

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

/// One window's surface, configured for a [`Gpu`].
pub struct WindowSurface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

impl std::fmt::Debug for WindowSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowSurface")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl WindowSurface {
    /// Configures `surface` at `size` physical pixels. The format prefers `Bgra8Unorm`, then
    /// `Rgba8Unorm`; the alpha mode prefers premultiplied, then post-multiplied.
    pub fn new(
        gpu: &Gpu,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
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
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&gpu.device, &config);
        Ok(Self { surface, config })
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

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&gpu.device, &self.config);
    }

    /// The next surface texture. An outdated, suboptimal or lost surface is reconfigured (a
    /// suboptimal texture is dropped, not drawn) and [`Acquired::Redraw`] returned; a hidden or
    /// minimised window, or a timeout, gives [`Acquired::Skip`].
    pub fn acquire(&mut self, gpu: &Gpu) -> Result<Acquired, String> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => Ok(Acquired::Frame(t)),
            wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Suboptimal(_)
            | wgpu::CurrentSurfaceTexture::Lost => {
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

//! The frame renderer: owns the glyphon state and the scene → blur → target pass sequence.
//!
//! The app paints into a [`Painter`](crate::paint::Painter) and hands the finished [`Scene`] to
//! [`FrameRenderer::draw`], which renders it in the scene module's documented draw order into any target view at an explicit size and scale:
//! the window surface (see [`Gpu::acquire`] and [`Gpu::present`]) or an offscreen texture (frame dumps).

use glyphon::{
    Cache, Color, Resolution, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

use crate::blur::Blur;
use crate::custom::{CustomDraw, CustomFrame, CustomItem};
use crate::gpu::Gpu;
use crate::scene::{LAYER_COUNT, SCENE_LAYERS, Scene};
use crate::text::TextCache;

pub use crate::scene::FrameParams;

pub struct FrameRenderer {
    viewport: Viewport,
    atlas: TextAtlas,
    /// One text renderer per paint layer, so text interleaves correctly with shapes.
    layers: Vec<TextRenderer>,
    swash: SwashCache,
    /// Offscreen scene + blur for glass menus.
    blur: Blur,
}

impl std::fmt::Debug for FrameRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameRenderer").finish_non_exhaustive()
    }
}

impl FrameRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        let cache = Cache::new(&gpu.device);
        let viewport = Viewport::new(&gpu.device, &cache);
        let mut atlas = TextAtlas::with_color_mode(
            &gpu.device,
            &gpu.queue,
            &cache,
            gpu.config.format,
            glyphon::ColorMode::Web,
        );
        let layers = (0..LAYER_COUNT)
            .map(|_| {
                TextRenderer::new(
                    &mut atlas,
                    &gpu.device,
                    wgpu::MultisampleState::default(),
                    None,
                )
            })
            .collect();
        let blur = Blur::new(&gpu.device, gpu.config.format);
        Self {
            viewport,
            atlas,
            layers,
            swash: SwashCache::new(),
            blur,
        }
    }

    /// The blur stage, for debug dumps of its intermediate textures.
    pub fn blur(&self) -> &Blur {
        &self.blur
    }

    /// Draws one painted `scene` into `target`: the scene layers go to the offscreen scene,
    /// which is blitted to `target` (blurred under the backdrop request if one is set), then the
    /// popup layers are drawn over it.
    pub fn draw(
        &mut self,
        gpu: &Gpu,
        text: &mut TextCache,
        scene: &Scene,
        target: &wgpu::TextureView,
        params: FrameParams,
    ) -> Result<(), String> {
        self.draw_with(gpu, text, scene, target, params, &mut [])
    }

    /// [`draw`](Self::draw) with app-owned draws: each layer's [`CustomItem`]s render after the
    /// layer's acrylic and under shapes and before its text, scissored to the item's clip.
    /// `customs[slot]` draws the items recorded with that slot. Every element of `customs` gets
    /// one `prepare` call before the first render pass.
    pub fn draw_with(
        &mut self,
        gpu: &Gpu,
        text: &mut TextCache,
        scene: &Scene,
        target: &wgpu::TextureView,
        params: FrameParams,
        customs: &mut [&mut dyn CustomDraw],
    ) -> Result<(), String> {
        let layers = &scene.layers;
        let backdrop = scene.backdrop;
        if let Some(bad) = layers
            .iter()
            .flat_map(|l| &l.customs)
            .find(|i| i.slot >= customs.len())
        {
            return Err(format!("custom draw slot {} not supplied", bad.slot));
        }
        let custom_frame = CustomFrame {
            size: params.size,
            scale: params.scale,
            format: gpu.config.format,
        };
        for c in customs.iter_mut() {
            c.prepare(&gpu.device, &gpu.queue, &custom_frame);
        }
        let customs: &[&mut dyn CustomDraw] = customs;
        let FrameParams {
            size,
            scale: s,
            blur_sigma: sigma,
            saturation,
        } = params;
        self.viewport.update(
            &gpu.queue,
            Resolution {
                width: size.0,
                height: size.1,
            },
        );
        for (layer, renderer) in layers.iter().zip(&mut self.layers) {
            let (fonts, buffer_of) = text.split();
            let areas = layer.texts.iter().filter_map(|t| {
                let buffer = buffer_of(t.key)?;
                let c = t.clip;
                Some(TextArea {
                    buffer,
                    left: t.x * s,
                    top: t.y * s,
                    scale: t.scale,
                    bounds: TextBounds {
                        left: (c.x * s) as i32,
                        top: (c.y * s) as i32,
                        right: (c.right() * s) as i32,
                        bottom: (c.bottom() * s) as i32,
                    },
                    default_color: Color::rgb(0xCC, 0xCC, 0xCC),
                    custom_glyphs: &[],
                })
            });
            renderer
                .prepare(
                    &gpu.device,
                    &gpu.queue,
                    fonts,
                    &mut self.atlas,
                    &self.viewport,
                    areas,
                    &mut self.swash,
                )
                .map_err(|e| e.to_string())?;
        }

        let uploaded = gpu.upload(scene, size, s);
        self.blur.prepare(&gpu.device, &gpu.queue, size, s, sigma);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let pass_desc = |target: &wgpu::TextureView,
                         f: &mut dyn FnMut(&mut wgpu::RenderPass<'_>) -> Result<(), String>,
                         encoder: &mut wgpu::CommandEncoder| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            f(&mut pass)
        };
        let scene_view = self.blur.scene_view().ok_or("no scene target")?;
        pass_desc(
            scene_view,
            &mut |pass| {
                for (i, layer) in layers.iter().enumerate().take(SCENE_LAYERS) {
                    gpu.draw_below_text(pass, &uploaded, i);
                    draw_customs(pass, &layer.customs, customs, &custom_frame);
                    self.layers[i]
                        .render(&self.atlas, &self.viewport, pass)
                        .map_err(|e| e.to_string())?;
                    gpu.draw_above_text(pass, &uploaded, i);
                }
                Ok(())
            },
            &mut encoder,
        )?;
        if let Some(b) = backdrop {
            self.blur.set_backdrop(&gpu.queue, b, saturation);
            self.blur.run(&mut encoder);
        }
        pass_desc(
            target,
            &mut |pass| {
                self.blur.blit_scene(pass);
                if backdrop.is_some() {
                    self.blur.draw_backdrop(pass);
                }
                for (i, layer) in layers.iter().enumerate().skip(SCENE_LAYERS) {
                    gpu.draw_below_text(pass, &uploaded, i);
                    draw_customs(pass, &layer.customs, customs, &custom_frame);
                    self.layers[i]
                        .render(&self.atlas, &self.viewport, pass)
                        .map_err(|e| e.to_string())?;
                    gpu.draw_above_text(pass, &uploaded, i);
                }
                Ok(())
            },
            &mut encoder,
        )?;
        gpu.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    /// Per-frame housekeeping after the frame is finished: trims the glyph atlas and ages the
    /// text cache.
    pub fn end_frame(&mut self, text: &mut TextCache) {
        self.atlas.trim();
        text.end_frame();
    }
}

/// Renders one layer's custom items in record order, each under a scissor of its clip clamped to
/// the target; the scissor goes back to the full target afterwards.
fn draw_customs(
    pass: &mut wgpu::RenderPass<'_>,
    items: &[CustomItem],
    customs: &[&mut dyn CustomDraw],
    frame: &CustomFrame,
) {
    let (tw, th) = frame.size;
    for item in items {
        let [x, y, w, h] = item.clip;
        let (x, y) = (x.min(tw), y.min(th));
        let (w, h) = (w.min(tw - x), h.min(th - y));
        if w == 0 || h == 0 {
            continue;
        }
        pass.set_scissor_rect(x, y, w, h);
        customs[item.slot].render(pass, frame);
        pass.set_scissor_rect(0, 0, tw, th);
    }
}

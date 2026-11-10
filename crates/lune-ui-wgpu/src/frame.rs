//! The frame renderer of one target: its uniforms, viewport, text renderers and blur textures,
//! and the scene → blur → target pass sequence.
//!
//! The app paints into a [`Painter`](lune_ui_core::paint::Painter) and hands the finished [`Scene`] to
//! [`FrameRenderer::draw`], which renders it in the [`scene`](lune_ui_core::scene) module's draw order
//! into any target view at an explicit size and scale: a window surface (see
//! [`WindowSurface`](crate::surface::WindowSurface)) or an offscreen texture (frame dumps). Any
//! number of renderers share one [`Gpu`]; everything a draw writes belongs to its renderer, except
//! the shared glyph atlas of its format and the glyph rasterizer cache.

use cosmic_text::Color;

use crate::blur::{Blur, BlurPlan};
use crate::custom::{CustomDraw, CustomFrame, CustomItem};
use crate::glyph::{Resolution, TextArea, TextBounds, TextRenderer, Viewport};
use crate::gpu::{Globals, Gpu};
use crate::text_cache::TextCache;
use lune_ui_core::scene::{LAYER_COUNT, Layer, Scene};

pub use lune_ui_core::scene::FrameParams;

/// Draws scenes into targets of one colour format. Use it only with the [`Gpu`] that made it.
pub struct FrameRenderer {
    format: wgpu::TextureFormat,
    /// Index of the format's shared state in the [`Gpu`].
    format_index: usize,
    globals: wgpu::Buffer,
    /// Globals, the shared noise texture and sampler (the shape and stroke pipelines' group 0).
    bind_group: wgpu::BindGroup,
    viewport: Viewport,
    /// One text renderer per layer of [`Scene::in_order`], so text interleaves correctly with
    /// shapes; grows to the longest list a frame has had.
    layers: Vec<TextRenderer>,
    /// Offscreen scene + blur for glass menus.
    blur: Blur,
}

impl std::fmt::Debug for FrameRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameRenderer")
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}

impl FrameRenderer {
    /// A renderer for targets of `format`; creates the format's shared pipelines and glyph atlas
    /// in `gpu` on first use.
    pub fn new(gpu: &mut Gpu, format: wgpu::TextureFormat) -> Self {
        let format_index = gpu.format_index(format);
        let device = &gpu.device;
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &gpu.globals_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gpu.noise_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&gpu.noise_sampler),
                },
            ],
        });
        let viewport = Viewport::new(device, &gpu.glyph_cache);
        let atlas = &mut gpu.formats[format_index].atlas;
        let layers = (0..LAYER_COUNT)
            .map(|_| TextRenderer::new(atlas, device, wgpu::MultisampleState::default(), None))
            .collect();
        let blur = Blur::new(device, format);
        Self {
            format,
            format_index,
            globals,
            bind_group,
            viewport,
            layers,
            blur,
        }
    }

    /// The colour format of the targets this renderer draws into.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// The blur stage, for debug dumps of its intermediate textures.
    pub fn blur(&self) -> &Blur {
        &self.blur
    }

    /// Draws one painted `scene` into `target`, in [`Scene::in_order`]'s order. Each layer with
    /// backdrop requests is a stop: what is drawn so far is blurred, the requests draw and the
    /// layer follows. Up to the last stop, and at least to [`Scene::floor`], the frame renders
    /// offscreen; that is blitted to `target` and the rest draws over it.
    pub fn draw(
        &mut self,
        gpu: &mut Gpu,
        text: &mut TextCache,
        scene: &Scene,
        target: &wgpu::TextureView,
        params: FrameParams,
    ) -> Result<(), String> {
        self.draw_with(gpu, text, scene, target, params, &mut [])
    }

    /// [`draw`](Self::draw) with app-owned draws: each layer's [`CustomItem`]s render after the
    /// layer's acrylic and under shapes and before its images and text, scissored to the item's clip.
    /// `customs[slot]` draws the items recorded with that slot. Every element of `customs` gets
    /// one `prepare` call before the first render pass. One queue submit per call.
    pub fn draw_with(
        &mut self,
        gpu: &mut Gpu,
        text: &mut TextCache,
        scene: &Scene,
        target: &wgpu::TextureView,
        params: FrameParams,
        customs: &mut [&mut dyn CustomDraw],
    ) -> Result<(), String> {
        if gpu
            .formats
            .get(self.format_index)
            .is_none_or(|f| f.format != self.format)
        {
            return Err("frame renderer used with a Gpu that did not make it".into());
        }
        let layers = scene.in_order();
        let floor = scene.floor();
        if let Some(bad) = layers
            .iter()
            .flat_map(|l| &l.customs)
            .find(|i| i.slot >= customs.len())
        {
            return Err(format!("custom draw slot {} not supplied", bad.slot));
        }
        let rendering = params.text.effective();
        let custom_frame = CustomFrame {
            size: params.size,
            scale: params.scale,
            format: self.format,
            text: rendering,
        };
        for c in customs.iter_mut() {
            c.prepare(&gpu.device, &gpu.queue, &custom_frame);
        }
        gpu.ensure_text_images(&layers, text, rendering);
        let customs: &[&mut dyn CustomDraw] = customs;
        let FrameParams {
            size,
            scale: s,
            blur_sigma,
            saturation,
            ..
        } = params;
        let noise = params.noise();
        let limits = params.blur.effective();
        let (stops, plans) = stops_of(&layers, floor, blur_sigma, saturation);
        // Disjoint borrows: the format's atlas and the glyph rasterizer are the only shared state
        // this draw writes.
        let Gpu {
            device,
            queue,
            formats,
            glyphs,
            ..
        } = gpu;
        let atlas = &mut formats[self.format_index].atlas;
        while self.layers.len() < layers.len() {
            let multisample = wgpu::MultisampleState::default();
            let renderer = TextRenderer::new(atlas, device, multisample, None);
            self.layers.push(renderer);
        }
        self.viewport.update(
            queue,
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
                })
            });
            renderer
                .prepare(
                    device,
                    queue,
                    fonts,
                    atlas,
                    &self.viewport,
                    areas,
                    glyphs,
                    rendering,
                )
                .map_err(|e| e.to_string())?;
        }

        queue.write_buffer(
            &self.globals,
            0,
            bytemuck::bytes_of(&Globals {
                size: [size.0 as f32, size.1 as f32],
                scale: s,
                noise,
            }),
        );
        let gpu: &Gpu = gpu;
        let fs = &gpu.formats[self.format_index];
        let uploaded = gpu.upload(&layers, rendering);
        self.blur
            .prepare(&gpu.device, &gpu.blur, size, limits.downsample);
        self.blur
            .plan(&gpu.device, &gpu.queue, &gpu.blur, s, limits, &plans);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let pass_desc = |target: &wgpu::TextureView,
                         load: wgpu::LoadOp<wgpu::Color>,
                         f: &mut dyn FnMut(&mut wgpu::RenderPass<'_>) -> Result<(), String>,
                         encoder: &mut wgpu::CommandEncoder| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
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
        let globals = &self.bind_group;
        let scene_view = self.blur.scene_view().ok_or("no scene target")?;
        let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        let span = |pass: &mut wgpu::RenderPass<'_>, from: usize, to: usize| {
            for (i, layer) in layers.iter().enumerate().take(to).skip(from) {
                fs.draw_below_text(pass, globals, &uploaded, i);
                draw_customs(pass, &layer.customs, customs, &custom_frame);
                fs.draw_images(pass, globals, &uploaded, i);
                self.layers[i].render(&fs.atlas, &self.viewport, pass);
                fs.draw_above_text(pass, globals, &uploaded, i);
            }
            Ok::<(), String>(())
        };
        let requests = |pass: &mut wgpu::RenderPass<'_>, stop: &Stop| {
            for &(blur, request) in &stop.draws {
                self.blur.draw_request(pass, &fs.blur, blur, request);
            }
        };
        let first = stops.first().map_or(floor, |s| s.at);
        pass_desc(
            scene_view,
            clear,
            &mut |pass| span(pass, 0, first),
            &mut encoder,
        )?;
        for (k, stop) in stops.iter().enumerate() {
            self.blur.downsample(&mut encoder, &fs.blur);
            for blur in stop.blurs.clone() {
                self.blur.blur(&mut encoder, &fs.blur, blur);
            }
            // The last stop's requests draw on the target, after the blit.
            let Some(next) = stops.get(k + 1) else { break };
            pass_desc(
                scene_view,
                wgpu::LoadOp::Load,
                &mut |pass| {
                    requests(pass, stop);
                    span(pass, stop.at, next.at)
                },
                &mut encoder,
            )?;
        }
        pass_desc(
            target,
            clear,
            &mut |pass| {
                self.blur.blit_scene(pass, &fs.blur);
                if let Some(stop) = stops.last() {
                    requests(pass, stop);
                }
                span(pass, stops.last().map_or(floor, |s| s.at), layers.len())
            },
            &mut encoder,
        )?;
        gpu.queue.submit(Some(encoder.finish()));
        Ok(())
    }
}

/// One place in the layer list where a frame blurs what it has drawn.
#[derive(Debug)]
struct Stop {
    /// The index of the layer the requests draw under.
    at: usize,
    /// This stop's entries of the frame's [`BlurPlan`]s.
    blurs: std::ops::Range<usize>,
    /// (blur, request in it) in record order.
    draws: Vec<(usize, usize)>,
}

/// The frame's stops, and one blur per distinct sigma of each. A request's own numbers win, 0 means
/// the frame's; a request below `floor` stops there.
fn stops_of(
    layers: &[&Layer],
    floor: usize,
    frame_sigma: f32,
    frame_saturation: f32,
) -> (Vec<Stop>, Vec<BlurPlan>) {
    let own = |v: f32, frame: f32| if v > 0.0 { v } else { frame };
    let (mut stops, mut plans) = (Vec::<Stop>::new(), Vec::<BlurPlan>::new());
    for (i, layer) in layers.iter().enumerate() {
        for b in &layer.backdrops {
            let at = i.max(floor);
            if stops.last().is_none_or(|s| s.at != at) {
                stops.push(Stop {
                    at,
                    blurs: plans.len()..plans.len(),
                    draws: Vec::new(),
                });
            }
            let Some(stop) = stops.last_mut() else {
                continue;
            };
            let sigma = own(b.sigma, frame_sigma);
            let blur = stop
                .blurs
                .clone()
                .find(|&p| plans[p].sigma == sigma)
                .unwrap_or_else(|| {
                    plans.push(BlurPlan {
                        sigma,
                        requests: Vec::new(),
                    });
                    stop.blurs.end = plans.len();
                    plans.len() - 1
                });
            let requests = &mut plans[blur].requests;
            stop.draws.push((blur, requests.len()));
            requests.push((*b, own(b.saturation, frame_saturation)));
        }
    }
    (stops, plans)
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

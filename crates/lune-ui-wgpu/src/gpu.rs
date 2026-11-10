//! The shared GPU: device, queue, the noise texture and, per colour target format, the shape and
//! stroke pipelines, the blur pipelines and one glyph atlas.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bytemuck::{Pod, Zeroable};
use cosmic_text::SwashCache;
use wgpu::util::DeviceExt;

use crate::blur;
use crate::glyph::{Cache, TextAtlas};
use crate::images::{GpuImageInstance, ImageResources};
use crate::platform;
use crate::text_cache::TextCache;
use lune_ui_core::image::{ImageData, ImageError, ImageId, ImageStore};
use lune_ui_core::scene::{ImageSource, Layer, Shape, StrokeVertex};

/// The uniform block of the shape and stroke pipelines: the target size in physical px, the scale
/// and the opacity of the noise texture.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Globals {
    pub size: [f32; 2],
    pub scale: f32,
    pub noise: f32,
}

/// GPU copy of [`Shape`]: same fields, same order.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuShape {
    rect: [f32; 4],
    radii: [f32; 4],
    color: [f32; 4],
    under: [f32; 4],
    params: [f32; 4],
    /// Physical clip rect (x, y, w, h); zero width = no clip.
    clip: [f32; 4],
    round: [f32; 4],
    round_radius: f32,
}

/// GPU copy of [`StrokeVertex`]: same fields, same order.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuStrokeVertex {
    pos: [f32; 2],
    aa: [f32; 2],
    color: [f32; 4],
    style: [f32; 4],
    extra: [f32; 4],
    clip: [f32; 4],
    round: [f32; 4],
    round_radius: f32,
}

const _: () = assert!(size_of::<GpuShape>() == 116);
const _: () = assert!(size_of::<GpuStrokeVertex>() == 100);

impl From<&Shape> for GpuShape {
    fn from(s: &Shape) -> Self {
        Self {
            rect: s.rect,
            radii: s.radii,
            color: s.color,
            under: s.under,
            params: s.params,
            clip: s.clip,
            round: s.round,
            round_radius: s.round_radius,
        }
    }
}

impl From<&StrokeVertex> for GpuStrokeVertex {
    fn from(v: &StrokeVertex) -> Self {
        Self {
            pos: v.pos,
            aa: v.aa,
            color: v.color,
            style: v.style,
            extra: v.extra,
            clip: v.clip,
            round: v.round,
            round_radius: v.round_radius,
        }
    }
}

/// The pipelines and the glyph atlas of one colour target format, shared by every
/// [`FrameRenderer`](crate::frame::FrameRenderer) that draws in that format.
pub(crate) struct FormatState {
    pub(crate) format: wgpu::TextureFormat,
    acrylic_pipeline: wgpu::RenderPipeline,
    /// The acrylic shader blended in at a constant opacity (`src·c + dst·(1 − c)`).
    acrylic_fade_pipeline: wgpu::RenderPipeline,
    shape_pipeline: wgpu::RenderPipeline,
    stroke_pipeline: wgpu::RenderPipeline,
    image_pipeline: wgpu::RenderPipeline,
    pub(crate) blur: blur::Pipelines,
    pub(crate) atlas: TextAtlas,
}

/// The GPU shared by every target of the app: one device and queue, the noise texture, and per
/// colour target format the pipelines and one glyph atlas. Holds no window and no surface; each
/// target is a [`FrameRenderer`](crate::frame::FrameRenderer) (plus a
/// [`WindowSurface`](crate::surface::WindowSurface) for a window).
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// Globals (binding 0), noise texture (1) and sampler (2) of the shape and stroke pipelines.
    pub(crate) globals_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    image_shader: wgpu::ShaderModule,
    image_pipeline_layout: wgpu::PipelineLayout,
    /// Registered pictures and rasterised texts, shared by every target on the device.
    pub(crate) images: ImageResources,
    pub(crate) noise_view: wgpu::TextureView,
    pub(crate) noise_sampler: wgpu::Sampler,
    pub(crate) blur: blur::Shared,
    pub(crate) glyph_cache: Cache,
    pub(crate) swash: SwashCache,
    /// Created on first use, looked up linearly (a handful of formats at most).
    pub(crate) formats: Vec<FormatState>,
    /// Set by the device's lost callback; read by [`Gpu::is_lost`].
    lost: Arc<AtomicBool>,
}

impl std::fmt::Debug for Gpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formats: Vec<_> = self.formats.iter().map(|s| s.format).collect();
        f.debug_struct("Gpu")
            .field("adapter", &self.adapter.get_info().name)
            .field("formats", &formats)
            .finish_non_exhaustive()
    }
}

impl ImageStore for Gpu {
    /// Uploads `data` as a premultiplied texture. A side above the device's maximum texture
    /// size, or a length other than `4 * width * height`, is refused before any GPU call. A
    /// picture with no pixels is accepted and draws nothing.
    fn register(&mut self, id: ImageId, data: ImageData) -> Result<(), ImageError> {
        let limit = self.device.limits().max_texture_dimension_2d;
        crate::images::check(&data, limit)?;
        if data.width == 0 || data.height == 0 {
            self.images.ids.remove(&id);
            return Ok(());
        }
        let img = self.images.upload(
            &self.device,
            &self.queue,
            data.width,
            data.height,
            &data.rgba,
        );
        self.images.ids.insert(id, img);
        Ok(())
    }

    fn release(&mut self, id: ImageId) {
        self.images.ids.remove(&id);
    }
}

type Range = std::ops::Range<u32>;

/// A run of acrylic shapes drawn with one pipeline: opacity 1.0 is the replace pipeline for the
/// whole run, anything lower is a single shape through the constant-blend pipeline.
type AcrylicRun = (Range, f32);

/// Where one layer's records sit in the uploaded buffers.
#[derive(Debug, Clone)]
struct LayerRanges {
    acrylic: Vec<AcrylicRun>,
    under: Range,
    over: Range,
    /// Path-fill vertices, in the stroke buffer.
    fills: Range,
    strokes: Range,
}

#[derive(Debug)]
pub(crate) struct Uploaded {
    shapes: wgpu::Buffer,
    strokes: Option<wgpu::Buffer>,
    /// Image instances of every layer, in layer order.
    images: Option<wgpu::Buffer>,
    /// Per layer: the instance index and bind group of each image to draw, in record order.
    image_draws: Vec<Vec<(u32, wgpu::BindGroup)>>,
    /// Per layer: the acrylic runs, the under and over shape ranges and the stroke vertex range.
    ranges: Vec<LayerRanges>,
}

/// The wgpu instance. `display` is the windowing system's display handle (an event loop's owned
/// display handle, say); `None` for a headless instance. On Windows this is DX12 presented
/// through a DirectComposition visual, the wgpu path that supports a transparent window.
pub fn instance(display: Option<Box<dyn wgpu::wgt::WgpuHasDisplayHandle>>) -> wgpu::Instance {
    let mut desc = match display {
        Some(display) => wgpu::InstanceDescriptor::new_with_display_handle(display),
        None => wgpu::InstanceDescriptor::new_without_display_handle(),
    };
    platform::configure(&mut desc);
    wgpu::Instance::new(desc)
}

impl Gpu {
    /// Opens the device. `compatible` is a surface the adapter must be able to present to;
    /// `None` opens a headless device (offscreen targets only).
    pub async fn new(
        instance: wgpu::Instance,
        compatible: Option<&wgpu::Surface<'_>>,
    ) -> Result<Gpu, String> {
        Self::new_with_features(instance, compatible, wgpu::Features::empty()).await
    }

    /// [`new`](Self::new) that also asks for the optional device `wanted` features. Only those
    /// the adapter offers are requested; read what was granted from `device.features()`.
    pub async fn new_with_features(
        instance: wgpu::Instance,
        compatible: Option<&wgpu::Surface<'_>>,
        wanted: wgpu::Features,
    ) -> Result<Gpu, String> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: compatible,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wanted & adapter.features(),
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let lost = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&lost);
        device.set_device_lost_callback(move |_reason, _message| {
            flag.store(true, Ordering::Release);
        });

        let (noise_view, noise_sampler) = noise_texture(&device, &queue)?;
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shapes"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shapes"),
            bind_group_layouts: &[Some(&globals_layout)],
            ..Default::default()
        });
        let images = ImageResources::new(&device);
        let image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("images"),
            source: wgpu::ShaderSource::Wgsl(include_str!("image.wgsl").into()),
        });
        let image_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("images"),
                bind_group_layouts: &[Some(&globals_layout), Some(&images.layout)],
                ..Default::default()
            });
        let blur = blur::Shared::new(&device);
        let glyph_cache = Cache::new(&device);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            globals_layout,
            pipeline_layout,
            shader,
            image_shader,
            image_pipeline_layout,
            images,
            noise_view,
            noise_sampler,
            blur,
            glyph_cache,
            swash: SwashCache::new(),
            formats: Vec::new(),
            lost,
        })
    }

    /// Whether the device is lost: the driver reset or dropped it, the adapter went away, or
    /// `device.destroy()` was called. wgpu reports a loss from the device's poll (every frame's
    /// submission makes one, and a destroy is reported once the queue has drained), so a check
    /// right after the event may still read `false`; ask once a frame, before drawing. A lost
    /// device draws nothing, and wgpu panics inside a draw that creates a buffer on one (seen with
    /// [`FrameRenderer::draw`](crate::frame::FrameRenderer::draw)): check this first and skip the
    /// draw.
    ///
    /// The toolkit does not recover. The app owns everything built on the device, so on `true` it
    /// drops and rebuilds all of it: this `Gpu`, the [`TextCache`], every
    /// [`FrameRenderer`](crate::frame::FrameRenderer), every
    /// [`WindowSurface`](crate::surface::WindowSurface) and every registered image (register them
    /// again with the new `Gpu`).
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    /// The index of `format`'s shared state in `formats`, created on first use.
    pub(crate) fn format_index(&mut self, format: wgpu::TextureFormat) -> usize {
        if let Some(i) = self.formats.iter().position(|f| f.format == format) {
            return i;
        }
        let state = self.format_state(format);
        self.formats.push(state);
        self.formats.len() - 1
    }

    fn format_state(&self, format: wgpu::TextureFormat) -> FormatState {
        let device = &self.device;
        let shape_attrs = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4,
            5 => Float32x4, 6 => Float32x4, 7 => Float32
        ];
        let shape_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuShape>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &shape_attrs,
        };
        let stroke_attrs = wgpu::vertex_attr_array![
            0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4,
            5 => Float32x4, 6 => Float32x4, 7 => Float32
        ];
        let stroke_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuStrokeVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &stroke_attrs,
        };

        let premultiplied_over = wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING;
        let make = |label: &str,
                    vs: &str,
                    fs: &str,
                    buffer: wgpu::VertexBufferLayout<'_>,
                    topology: wgpu::PrimitiveTopology,
                    blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some(vs),
                    buffers: &[Some(buffer)],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            })
        };
        let strip = wgpu::PrimitiveTopology::TriangleStrip;
        let acrylic_pipeline = make(
            "acrylic",
            "vs_shape",
            "fs_shape",
            shape_layout.clone(),
            strip,
            Some(wgpu::BlendState::REPLACE),
        );
        // Faded acrylic: the shader's colour and alpha blended with what is below, by a constant.
        let constant = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Constant,
            dst_factor: wgpu::BlendFactor::OneMinusConstant,
            operation: wgpu::BlendOperation::Add,
        };
        let acrylic_fade_pipeline = make(
            "acrylic fade",
            "vs_shape",
            "fs_shape",
            shape_layout.clone(),
            strip,
            Some(wgpu::BlendState {
                color: constant,
                alpha: constant,
            }),
        );
        let shape_pipeline = make(
            "shapes",
            "vs_shape",
            "fs_shape",
            shape_layout,
            strip,
            Some(premultiplied_over),
        );
        let stroke_pipeline = make(
            "strokes",
            "vs_stroke",
            "fs_stroke",
            stroke_layout,
            wgpu::PrimitiveTopology::TriangleList,
            Some(premultiplied_over),
        );
        let image_attrs = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4
        ];
        let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("images"),
            layout: Some(&self.image_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.image_shader,
                entry_point: Some("vs_image"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<GpuImageInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &image_attrs,
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.image_shader,
                entry_point: Some("fs_image"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(premultiplied_over),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: strip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });
        let atlas = TextAtlas::new(device, &self.queue, &self.glyph_cache, format);
        FormatState {
            format,
            acrylic_pipeline,
            acrylic_fade_pipeline,
            shape_pipeline,
            stroke_pipeline,
            image_pipeline,
            blur: self.blur.pipelines(device, format),
            atlas,
        }
    }

    /// Per-app-frame housekeeping, once after every target of the frame drew: trims every glyph
    /// atlas and ages the text cache.
    pub fn end_frame(&mut self, text: &mut TextCache) {
        for f in &mut self.formats {
            f.atlas.trim();
        }
        text.end_frame();
        self.images.texts.retain(|key, _| text.contains(*key));
    }

    /// How many pictures the app has registered (rasterised texts are not counted).
    pub fn image_count(&self) -> usize {
        self.images.ids.len()
    }

    /// How many rasterised texts (from `Painter::text_rotated`) the GPU holds.
    pub fn text_image_count(&self) -> usize {
        self.images.texts.len()
    }

    /// Rasterises and uploads every text of `layers`' image items that has no raster yet.
    pub(crate) fn ensure_text_images(&mut self, layers: &[&Layer], text: &mut TextCache) {
        let limit = self.device.limits().max_texture_dimension_2d;
        for layer in layers {
            for item in &layer.images {
                let ImageSource::Text(key, _) = item.source else {
                    continue;
                };
                if self.images.texts.contains_key(&key) {
                    continue;
                }
                let Some(raster) = text.rasterize(key, &mut self.swash) else {
                    continue;
                };
                if raster.width.max(raster.height) > limit {
                    continue;
                }
                let img = self.images.upload(
                    &self.device,
                    &self.queue,
                    raster.width,
                    raster.height,
                    &raster.rgba,
                );
                self.images.texts.insert(key, img);
            }
        }
    }

    /// Copies the shapes and stroke vertices of `layers`, a frame's [`Scene::in_order`] list, into
    /// fresh vertex buffers, created for this draw. The result's ranges index that list.
    ///
    /// [`Scene::in_order`]: lune_ui_core::scene::Scene::in_order
    pub(crate) fn upload(&self, layers: &[&Layer]) -> Uploaded {
        let mut shapes: Vec<GpuShape> = Vec::new();
        let mut strokes: Vec<GpuStrokeVertex> = Vec::new();
        let mut ranges = Vec::with_capacity(layers.len());
        for layer in layers.iter().map(|l| &l.geometry) {
            let mut push = |items: &[Shape]| {
                let start = shapes.len() as u32;
                shapes.extend(items.iter().map(GpuShape::from));
                start..shapes.len() as u32
            };
            let mut acrylic: Vec<AcrylicRun> = Vec::new();
            for a in &layer.acrylic {
                let range = push(std::slice::from_ref(&a.shape));
                let opaque = a.opacity >= 1.0;
                match acrylic.last_mut() {
                    Some((run, opacity)) if opaque && *opacity >= 1.0 => run.end = range.end,
                    _ => acrylic.push((range, if opaque { 1.0 } else { a.opacity })),
                }
            }
            let under = push(&layer.under);
            let over = push(&layer.over);
            let start = strokes.len() as u32;
            strokes.extend(layer.fills.iter().map(GpuStrokeVertex::from));
            let mid = strokes.len() as u32;
            strokes.extend(layer.strokes.iter().map(GpuStrokeVertex::from));
            ranges.push(LayerRanges {
                acrylic,
                under,
                over,
                fills: start..mid,
                strokes: mid..strokes.len() as u32,
            });
        }
        let mut instances: Vec<GpuImageInstance> = Vec::new();
        let image_draws: Vec<Vec<(u32, wgpu::BindGroup)>> = layers
            .iter()
            .map(|layer| {
                layer
                    .images
                    .iter()
                    .filter_map(|item| self.images.instance(item))
                    .map(|(inst, group)| {
                        instances.push(inst);
                        (instances.len() as u32 - 1, group)
                    })
                    .collect()
            })
            .collect();
        if shapes.is_empty() {
            shapes.push(GpuShape::zeroed());
        }
        let shapes = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shapes"),
                contents: bytemuck::cast_slice(&shapes),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let strokes = (!strokes.is_empty()).then(|| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("strokes"),
                    contents: bytemuck::cast_slice(&strokes),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        let images = (!instances.is_empty()).then(|| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("images"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        Uploaded {
            shapes,
            strokes,
            images,
            image_draws,
            ranges,
        }
    }
}

impl FormatState {
    /// Everything of `layer` that goes below its text; `globals` is the target's bind group.
    pub(crate) fn draw_below_text(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        globals: &wgpu::BindGroup,
        up: &Uploaded,
        layer: usize,
    ) {
        let LayerRanges {
            acrylic,
            under,
            fills,
            ..
        } = up.ranges[layer].clone();
        for (range, opacity) in acrylic {
            if opacity >= 1.0 {
                draw_shapes(pass, globals, up, &self.acrylic_pipeline, range);
            } else {
                pass.set_blend_constant(wgpu::Color {
                    r: f64::from(opacity),
                    g: f64::from(opacity),
                    b: f64::from(opacity),
                    a: f64::from(opacity),
                });
                draw_shapes(pass, globals, up, &self.acrylic_fade_pipeline, range);
            }
        }
        draw_shapes(pass, globals, up, &self.shape_pipeline, under);
        self.draw_vertices(pass, globals, up, fills);
    }

    fn draw_vertices(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        globals: &wgpu::BindGroup,
        up: &Uploaded,
        range: Range,
    ) {
        if let Some(strokes) = &up.strokes
            && !range.is_empty()
        {
            pass.set_pipeline(&self.stroke_pipeline);
            pass.set_bind_group(0, globals, &[]);
            pass.set_vertex_buffer(0, strokes.slice(..));
            pass.draw(range, 0..1);
        }
    }

    /// The image items of `layer`, in record order; they go after its customs and before its text.
    pub(crate) fn draw_images(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        globals: &wgpu::BindGroup,
        up: &Uploaded,
        layer: usize,
    ) {
        let Some(buffer) = &up.images else {
            return;
        };
        for (index, group) in &up.image_draws[layer] {
            pass.set_pipeline(&self.image_pipeline);
            pass.set_bind_group(0, globals, &[]);
            pass.set_bind_group(1, group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..4, *index..*index + 1);
        }
    }

    /// Everything of `layer` that goes above its text; `globals` is the target's bind group.
    pub(crate) fn draw_above_text(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        globals: &wgpu::BindGroup,
        up: &Uploaded,
        layer: usize,
    ) {
        let LayerRanges { over, strokes, .. } = up.ranges[layer].clone();
        draw_shapes(pass, globals, up, &self.shape_pipeline, over);
        self.draw_vertices(pass, globals, up, strokes);
    }
}

fn draw_shapes(
    pass: &mut wgpu::RenderPass<'_>,
    globals: &wgpu::BindGroup,
    up: &Uploaded,
    pipeline: &wgpu::RenderPipeline,
    range: Range,
) {
    if range.is_empty() {
        return;
    }
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, globals, &[]);
    pass.set_vertex_buffer(0, up.shapes.slice(..));
    pass.draw(0..4, range);
}

/// Avalonia's 256×256 acrylic noise (MIT, `licenses/avalonia-LICENSE`), as straight-alpha RGBA.
fn noise_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(wgpu::TextureView, wgpu::Sampler), String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!(
        "../assets/acrylic-noise.png"
    )));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("png size")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px
            .chunks(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        png::ColorType::Indexed => return Err("png palette not expanded".into()),
    };
    let size = wgpu::Extent3d {
        width: info.width,
        height: info.height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("acrylic noise"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &rgba,
    );
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("noise"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    Ok((texture.create_view(&Default::default()), sampler))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pairwise distinct values, starting with the awkward ones: -0.0, a subnormal, f32::MAX and
    /// a NaN with a payload.
    fn values(n: usize) -> Vec<f32> {
        let mut v = vec![
            -0.0,
            f32::from_bits(1),
            f32::MAX,
            f32::from_bits(0x7fc0_1234),
        ];
        v.extend((0..n).map(|i| i as f32 * 1.25 + 3.0));
        v.truncate(n);
        v
    }

    fn bytes(fields: &[f32]) -> Vec<u8> {
        fields.iter().flat_map(|f| f.to_ne_bytes()).collect()
    }

    fn quad(v: &[f32]) -> [f32; 4] {
        [v[0], v[1], v[2], v[3]]
    }

    #[test]
    fn gpu_records_are_bit_exact() {
        let v = values(29);
        let distinct: std::collections::HashSet<u32> = v.iter().map(|f| f.to_bits()).collect();
        assert_eq!(distinct.len(), 29);
        let shape = Shape {
            rect: quad(&v[0..]),
            radii: quad(&v[4..]),
            color: quad(&v[8..]),
            under: quad(&v[12..]),
            params: quad(&v[16..]),
            clip: quad(&v[20..]),
            round: quad(&v[24..]),
            round_radius: v[28],
        };
        assert_eq!(
            bytemuck::bytes_of(&GpuShape::from(&shape)),
            bytes(&v).as_slice()
        );

        let v = values(25);
        let vertex = StrokeVertex {
            pos: [v[0], v[1]],
            aa: [v[2], v[3]],
            color: quad(&v[4..]),
            style: quad(&v[8..]),
            extra: quad(&v[12..]),
            clip: quad(&v[16..]),
            round: quad(&v[20..]),
            round_radius: v[24],
        };
        assert_eq!(
            bytemuck::bytes_of(&GpuStrokeVertex::from(&vertex)),
            bytes(&v).as_slice()
        );
    }
}

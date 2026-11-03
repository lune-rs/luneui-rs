//! The shared GPU: device, queue, the noise texture and, per colour target format, the shape and
//! stroke pipelines, the blur pipelines and one glyph atlas.

use bytemuck::{Pod, Zeroable};
use glyphon::{Cache, SwashCache, TextAtlas};
use wgpu::util::DeviceExt;

use crate::blur;
use crate::scene::{Scene, Shape, StrokeVertex};
use crate::text::TextCache;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Globals {
    pub size: [f32; 2],
    pub scale: f32,
    pub _pad: f32,
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
}

const _: () = assert!(size_of::<GpuShape>() == 96);
const _: () = assert!(size_of::<GpuStrokeVertex>() == 80);

impl From<&Shape> for GpuShape {
    fn from(s: &Shape) -> Self {
        Self {
            rect: s.rect,
            radii: s.radii,
            color: s.color,
            under: s.under,
            params: s.params,
            clip: s.clip,
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
        }
    }
}

/// The pipelines and the glyph atlas of one colour target format, shared by every
/// [`FrameRenderer`](crate::frame::FrameRenderer) that draws in that format.
pub(crate) struct FormatState {
    pub(crate) format: wgpu::TextureFormat,
    acrylic_pipeline: wgpu::RenderPipeline,
    shape_pipeline: wgpu::RenderPipeline,
    stroke_pipeline: wgpu::RenderPipeline,
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
    pub(crate) noise_view: wgpu::TextureView,
    pub(crate) noise_sampler: wgpu::Sampler,
    pub(crate) blur: blur::Shared,
    pub(crate) glyph_cache: Cache,
    pub(crate) swash: SwashCache,
    /// Created on first use, looked up linearly (a handful of formats at most).
    pub(crate) formats: Vec<FormatState>,
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

type Range = std::ops::Range<u32>;

#[derive(Debug)]
pub struct Uploaded {
    shapes: wgpu::Buffer,
    strokes: Option<wgpu::Buffer>,
    /// Per layer: acrylic, under, over shape ranges and the stroke vertex range.
    ranges: Vec<(Range, Range, Range, Range)>,
}

/// The wgpu instance. `display` is the windowing system's display handle (an event loop's owned
/// display handle, say); `None` for a headless instance. On Windows this is DX12 presented
/// through a DirectComposition visual, the wgpu path that supports a transparent window.
pub fn instance(display: Option<Box<dyn wgpu::wgt::WgpuHasDisplayHandle>>) -> wgpu::Instance {
    #[allow(unused_mut)]
    let mut desc = match display {
        Some(display) => wgpu::InstanceDescriptor::new_with_display_handle(display),
        None => wgpu::InstanceDescriptor::new_without_display_handle(),
    };
    #[cfg(target_os = "windows")]
    {
        desc.backends = wgpu::Backends::DX12;
        desc.backend_options.dx12.presentation_system = wgpu::Dx12SwapchainKind::DxgiFromVisual;
    }
    wgpu::Instance::new(desc)
}

impl Gpu {
    /// Opens the device. `compatible` is a surface the adapter must be able to present to;
    /// `None` opens a headless device (offscreen targets only).
    pub async fn new(
        instance: wgpu::Instance,
        compatible: Option<&wgpu::Surface<'_>>,
    ) -> Result<Gpu, String> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: compatible,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|e| e.to_string())?;

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
            noise_view,
            noise_sampler,
            blur,
            glyph_cache,
            swash: SwashCache::new(),
            formats: Vec::new(),
        })
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
            5 => Float32x4
        ];
        let shape_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuShape>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &shape_attrs,
        };
        let stroke_attrs = wgpu::vertex_attr_array![
            0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4,
            5 => Float32x4
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
        let atlas = TextAtlas::with_color_mode(
            device,
            &self.queue,
            &self.glyph_cache,
            format,
            glyphon::ColorMode::Web,
        );
        FormatState {
            format,
            acrylic_pipeline,
            shape_pipeline,
            stroke_pipeline,
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
    }

    /// Copies every layer's shapes and stroke vertices of `scene` into fresh vertex buffers,
    /// created for this draw.
    pub fn upload(&self, scene: &Scene) -> Uploaded {
        let mut shapes: Vec<GpuShape> = Vec::new();
        let mut strokes: Vec<GpuStrokeVertex> = Vec::new();
        let mut ranges = Vec::with_capacity(scene.layers.len());
        for layer in scene.layers.iter().map(|l| &l.geometry) {
            let mut push = |items: &[Shape]| {
                let start = shapes.len() as u32;
                shapes.extend(items.iter().map(GpuShape::from));
                start..shapes.len() as u32
            };
            let acrylic = push(&layer.acrylic);
            let under = push(&layer.under);
            let over = push(&layer.over);
            let start = strokes.len() as u32;
            strokes.extend(layer.strokes.iter().map(GpuStrokeVertex::from));
            ranges.push((acrylic, under, over, start..strokes.len() as u32));
        }
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
        Uploaded {
            shapes,
            strokes,
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
        let (acrylic, under, _, _) = up.ranges[layer].clone();
        draw_shapes(pass, globals, up, &self.acrylic_pipeline, acrylic);
        draw_shapes(pass, globals, up, &self.shape_pipeline, under);
    }

    /// Everything of `layer` that goes above its text; `globals` is the target's bind group.
    pub(crate) fn draw_above_text(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        globals: &wgpu::BindGroup,
        up: &Uploaded,
        layer: usize,
    ) {
        let (_, _, over, stroke_range) = up.ranges[layer].clone();
        draw_shapes(pass, globals, up, &self.shape_pipeline, over);
        if let Some(strokes) = &up.strokes
            && !stroke_range.is_empty()
        {
            pass.set_pipeline(&self.stroke_pipeline);
            pass.set_bind_group(0, globals, &[]);
            pass.set_vertex_buffer(0, strokes.slice(..));
            pass.draw(stroke_range, 0..1);
        }
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
        let v = values(24);
        let distinct: std::collections::HashSet<u32> = v.iter().map(|f| f.to_bits()).collect();
        assert_eq!(distinct.len(), 24);
        let shape = Shape {
            rect: quad(&v[0..]),
            radii: quad(&v[4..]),
            color: quad(&v[8..]),
            under: quad(&v[12..]),
            params: quad(&v[16..]),
            clip: quad(&v[20..]),
        };
        assert_eq!(
            bytemuck::bytes_of(&GpuShape::from(&shape)),
            bytes(&v).as_slice()
        );

        let v = values(20);
        let vertex = StrokeVertex {
            pos: [v[0], v[1]],
            aa: [v[2], v[3]],
            color: quad(&v[4..]),
            style: quad(&v[8..]),
            extra: quad(&v[12..]),
            clip: quad(&v[16..]),
        };
        assert_eq!(
            bytemuck::bytes_of(&GpuStrokeVertex::from(&vertex)),
            bytes(&v).as_slice()
        );
    }
}

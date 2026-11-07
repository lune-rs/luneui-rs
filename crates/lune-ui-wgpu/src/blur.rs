//! Offscreen scene + backdrop blur, so a menu can show a blurred copy of what is under it
//! (Luneium's context-menu glass: blur 30px, saturate 1.8, then acrylic on top).

use bytemuck::{Pod, Zeroable};

use lune_ui_core::scene::BackdropRequest;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Params {
    src_texel: [f32; 2],
    dir: [f32; 2],
    sigma: f32,
    radius: f32,
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct BackdropUniform {
    rect: [f32; 4],
    target_size: [f32; 2],
    radius: f32,
    alpha: f32,
    saturation: f32,
    _pad: [f32; 3],
}

// The blur strength comes from Settings (`BlurLevel::sigma`). Luneium's CSS says `blur(30px)`, but
// taken literally that erases text to flat grey; its real menus use the lighter native macOS blur.
/// Blur runs at 1/8 resolution.
const DOWNSCALE: u32 = 8;

/// One blur a frame asks for: its sigma, and the requests that draw from it, each with its
/// saturation.
#[derive(Debug)]
pub(crate) struct BlurPlan {
    pub(crate) sigma: f32,
    pub(crate) requests: Vec<(BackdropRequest, f32)>,
}

/// One request's uniforms and the bind group that draws it from its blur's result.
#[derive(Debug)]
struct RequestSlot {
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
}

/// One blur's uniforms and result. A buffer write lands before the frame's submit, so every blur
/// of a frame needs buffers of its own: a shared one would hold the last value for all of them.
#[derive(Debug)]
struct BlurSlot {
    h_params: wgpu::Buffer,
    v_params: wgpu::Buffer,
    /// Blur-h (level2 → pong), blur-v (pong → `result`).
    groups: [wgpu::BindGroup; 2],
    result: wgpu::TextureView,
    requests: Vec<RequestSlot>,
}

#[derive(Debug)]
struct Targets {
    scene: wgpu::TextureView,
    levels: [wgpu::TextureView; 3],
    pong: wgpu::TextureView,
    /// Bind groups: scene, level0, level1.
    groups: [wgpu::BindGroup; 3],
    /// Bound to these views, so a new size drops them; grown to the most blurs a frame has had.
    blurs: Vec<BlurSlot>,
}

/// The blur state every target shares: shader, bind group layout, pipeline layout, sampler.
#[derive(Debug)]
pub(crate) struct Shared {
    shader: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
}

/// The blur pipelines of one colour target format (shared by every target of that format).
#[derive(Debug)]
pub(crate) struct Pipelines {
    copy: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    backdrop: wgpu::RenderPipeline,
}

impl Shared {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blur.wgsl").into()),
        });
        let uniform = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blur"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                uniform(2),
                uniform(3),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blur"),
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blur"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            shader,
            layout,
            pipeline_layout,
            sampler,
        }
    }

    /// The copy, blur and backdrop pipelines for targets of `format`.
    pub(crate) fn pipelines(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Pipelines {
        let make = |label: &str, vs: &str, fs: &str, blend: wgpu::BlendState, strip: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some(vs),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: if strip {
                        wgpu::PrimitiveTopology::TriangleStrip
                    } else {
                        wgpu::PrimitiveTopology::TriangleList
                    },
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                cache: None,
                multiview_mask: None,
            })
        };
        let copy = make(
            "blur copy",
            "vs_full",
            "fs_copy",
            wgpu::BlendState::REPLACE,
            false,
        );
        let blur = make(
            "blur gauss",
            "vs_full",
            "fs_blur",
            wgpu::BlendState::REPLACE,
            false,
        );
        let backdrop = make(
            "blur backdrop",
            "vs_backdrop",
            "fs_backdrop",
            wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            true,
        );
        Pipelines {
            copy,
            blur,
            backdrop,
        }
    }
}

/// One target's offscreen scene and blur: its textures, bind groups and uniform buffers. The
/// pipelines, layout and sampler are shared ([`Gpu`](crate::gpu::Gpu)).
#[derive(Debug)]
pub struct Blur {
    format: wgpu::TextureFormat,
    /// Fill the bindings the copy pipelines do not read.
    copy_params: wgpu::Buffer,
    copy_backdrop: wgpu::Buffer,
    size: (u32, u32),
    targets: Option<Targets>,
}

impl Blur {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let buffer = |label, size: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        Self {
            format,
            copy_params: buffer("blur copy params", size_of::<Params>()),
            copy_backdrop: buffer("blur copy backdrop", size_of::<BackdropUniform>()),
            size: (0, 0),
            targets: None,
        }
    }

    /// (Re)creates the offscreen targets for the surface size; cheap when unchanged.
    pub(crate) fn prepare(&mut self, device: &wgpu::Device, shared: &Shared, size: (u32, u32)) {
        let (w, h) = (size.0.max(1), size.1.max(1));
        let small = ((w / DOWNSCALE).max(1), (h / DOWNSCALE).max(1));
        if self.size == (w, h) && self.targets.is_some() {
            return;
        }
        self.size = (w, h);
        let texture = |label, tw, th| target_view(device, self.format, label, tw, th);
        let scene = texture("scene", w, h);
        let levels = [
            texture("blur 1/2", (w / 2).max(1), (h / 2).max(1)),
            texture("blur 1/4", (w / 4).max(1), (h / 4).max(1)),
            texture("blur 1/8", small.0, small.1),
        ];
        let pong = texture("blur pong", small.0, small.1);
        let group = |src| bind_group(device, shared, src, &self.copy_params, &self.copy_backdrop);
        let groups = [group(&scene), group(&levels[0]), group(&levels[1])];
        self.targets = Some(Targets {
            scene,
            levels,
            pong,
            groups,
            blurs: Vec::new(),
        });
    }

    /// Writes the uniforms of a frame's `plans`, growing the slots to fit. Call after
    /// [`Self::prepare`]; [`Self::blur`] and [`Self::draw_request`] then take indices into them.
    pub(crate) fn plan(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        shared: &Shared,
        scale: f32,
        plans: &[BlurPlan],
    ) {
        let (w, h) = self.size;
        let small = ((w / DOWNSCALE).max(1), (h / DOWNSCALE).max(1));
        let format = self.format;
        let copy_params = &self.copy_params;
        let Some(t) = &mut self.targets else { return };
        for (i, plan) in plans.iter().enumerate() {
            if t.blurs.len() == i {
                let slot = BlurSlot::new(
                    device,
                    shared,
                    format,
                    small,
                    (&t.levels[2], &t.pong),
                    (copy_params, &self.copy_backdrop),
                );
                t.blurs.push(slot);
            }
            let slot = &mut t.blurs[i];
            let sigma = plan.sigma * scale / DOWNSCALE as f32;
            for (buf, dir) in [(&slot.h_params, [1.0, 0.0]), (&slot.v_params, [0.0, 1.0])] {
                let params = Params {
                    src_texel: [1.0 / small.0 as f32, 1.0 / small.1 as f32],
                    dir,
                    sigma,
                    radius: (sigma * 3.0).ceil().min(48.0),
                    _pad: [0.0; 2],
                };
                queue.write_buffer(buf, 0, bytemuck::bytes_of(&params));
            }
            for (j, (b, saturation)) in plan.requests.iter().enumerate() {
                if slot.requests.len() == j {
                    let request = RequestSlot::new(device, shared, &slot.result, copy_params);
                    slot.requests.push(request);
                }
                let u = BackdropUniform {
                    rect: b.rect,
                    target_size: [w as f32, h as f32],
                    radius: b.radius,
                    alpha: b.alpha,
                    saturation: *saturation,
                    _pad: [0.0; 3],
                };
                queue.write_buffer(&slot.requests[j].uniform, 0, bytemuck::bytes_of(&u));
            }
        }
    }

    /// The texture the scene pass draws into, which the blur reads; `None` before the first size is
    /// set.
    pub fn scene_view(&self) -> Option<&wgpu::TextureView> {
        self.targets.as_ref().map(|t| &t.scene)
    }

    /// Downsamples the scene to 1/8, the source of [`Self::blur`]. Call after a scene pass.
    pub(crate) fn downsample(&self, encoder: &mut wgpu::CommandEncoder, p: &Pipelines) {
        let Some(t) = &self.targets else { return };
        step(encoder, &t.levels[0], &t.groups[0], &p.copy);
        step(encoder, &t.levels[1], &t.groups[1], &p.copy);
        step(encoder, &t.levels[2], &t.groups[2], &p.copy);
    }

    /// Runs blur `i` of the frame's plans on what [`Self::downsample`] left, into its own result.
    pub(crate) fn blur(&self, encoder: &mut wgpu::CommandEncoder, p: &Pipelines, i: usize) {
        let Some(t) = &self.targets else { return };
        let slot = &t.blurs[i];
        step(encoder, &t.pong, &slot.groups[0], &p.blur);
        step(encoder, &slot.result, &slot.groups[1], &p.blur);
    }

    /// Copy the scene into the current pass (first draw of the main pass).
    pub(crate) fn blit_scene(&self, pass: &mut wgpu::RenderPass<'_>, p: &Pipelines) {
        let Some(t) = &self.targets else { return };
        pass.set_pipeline(&p.copy);
        pass.set_bind_group(0, &t.groups[0], &[]);
        pass.draw(0..3, 0..1);
    }

    /// Draws request `request` of blur `blur` of the frame's plans inside its rect.
    pub(crate) fn draw_request(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        p: &Pipelines,
        blur: usize,
        request: usize,
    ) {
        let Some(t) = &self.targets else { return };
        pass.set_pipeline(&p.backdrop);
        pass.set_bind_group(0, &t.blurs[blur].requests[request].group, &[]);
        pass.draw(0..4, 0..1);
    }
}

/// A texture the blur renders into and reads, as a view.
fn target_view(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    label: &str,
    width: u32,
    height: u32,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn uniform_buffer(device: &wgpu::Device, label: &str, size: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn bind_group(
    device: &wgpu::Device,
    shared: &Shared,
    src: &wgpu::TextureView,
    params: &wgpu::Buffer,
    backdrop: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blur"),
        layout: &shared.layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(src),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&shared.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: backdrop.as_entire_binding(),
            },
        ],
    })
}

/// One full-target pass of the blur chain: `group` through `pipeline` into `target`.
fn step(
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    group: &wgpu::BindGroup,
    pipeline: &wgpu::RenderPipeline,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("blur step"),
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
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.draw(0..3, 0..1);
}

impl BlurSlot {
    /// A slot reading `source.0` through `source.1`, with its own 1/8-size result; `fill` holds
    /// the buffers for the bindings the blur pipeline does not read.
    fn new(
        device: &wgpu::Device,
        shared: &Shared,
        format: wgpu::TextureFormat,
        small: (u32, u32),
        source: (&wgpu::TextureView, &wgpu::TextureView),
        fill: (&wgpu::Buffer, &wgpu::Buffer),
    ) -> Self {
        let (level, pong) = source;
        let h_params = uniform_buffer(device, "blur h params", size_of::<Params>());
        let v_params = uniform_buffer(device, "blur v params", size_of::<Params>());
        let groups = [
            bind_group(device, shared, level, &h_params, fill.1),
            bind_group(device, shared, pong, &v_params, fill.1),
        ];
        Self {
            h_params,
            v_params,
            groups,
            result: target_view(device, format, "blur result", small.0, small.1),
            requests: Vec::new(),
        }
    }
}

impl RequestSlot {
    fn new(
        device: &wgpu::Device,
        shared: &Shared,
        result: &wgpu::TextureView,
        copy_params: &wgpu::Buffer,
    ) -> Self {
        let uniform = uniform_buffer(device, "blur backdrop", size_of::<BackdropUniform>());
        let group = bind_group(device, shared, result, copy_params, &uniform);
        Self { uniform, group }
    }
}

impl Blur {
    /// Debug: write the scene and the blurred 1/8 texture as PNGs (premultiplied, over black).
    pub fn dump(&self, device: &wgpu::Device, queue: &wgpu::Queue, dir: &std::path::Path) {
        let Some(t) = &self.targets else { return };
        let blurred = t.blurs.first().map_or(&t.levels[2], |b| &b.result);
        for (name, view) in [("scene", &t.scene), ("blurred", blurred)] {
            let tex = view.texture();
            let path = dir.join(format!("{name}.png"));
            let mut rgba = match crate::dump::read_rgba(device, queue, tex) {
                Ok(rgba) => rgba,
                Err(e) => {
                    eprintln!("dump {name}: {e}");
                    return;
                }
            };
            for px in rgba.chunks_exact_mut(4) {
                px[3] = 255;
            }
            let size = (tex.width(), tex.height());
            match crate::dump::write_png(&path, size, &rgba) {
                Ok(()) => eprintln!("dumped {} ({}x{})", path.display(), size.0, size.1),
                Err(e) => eprintln!("dump {name}: {e}"),
            }
        }
    }
}

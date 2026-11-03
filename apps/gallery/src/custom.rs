//! The gallery's `CustomDraw`: fills its slot with one fixed opaque colour through a minimal
//! pipeline, so a scene can prove that app-owned drawing lands inside its clip.

use lune_ui::custom::{CustomDraw, CustomFrame};

const SHADER: &str = "
@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    // One oversized triangle; the scissor (the slot's clip) cuts it to the slot.
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    return vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs() -> @location(0) vec4<f32> {
    return vec4<f32>(0.85, 0.40, 0.10, 1.0);
}
";

#[derive(Debug)]
pub(crate) struct Fill {
    pipeline: wgpu::RenderPipeline,
}

impl Fill {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gallery fill"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gallery fill"),
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gallery fill"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });
        Self { pipeline }
    }
}

impl CustomDraw for Fill {
    fn prepare(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue, _frame: &CustomFrame) {}

    fn render(&self, pass: &mut wgpu::RenderPass<'_>, _frame: &CustomFrame) {
        pass.set_pipeline(&self.pipeline);
        pass.draw(0..3, 0..1);
    }
}

//! Registered pictures and rasterised texts on the GPU: textures, the two samplers, the bind
//! group layout, and the per-draw instance records the image pipeline reads.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use lune_ui_core::image::{ImageData, ImageId, image_rect};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{ImageItem, ImageSource};
use lune_ui_core::text::TextKey;

/// One image instance for the image shader: the picture's rect, the plain clip, the rounded clip
/// rect, and `[rotation, alpha, rounded clip radius, 0]`. Physical pixels.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuImageInstance {
    pub(crate) rect: [f32; 4],
    pub(crate) clip: [f32; 4],
    pub(crate) round: [f32; 4],
    pub(crate) params: [f32; 4],
}

const _: () = assert!(size_of::<GpuImageInstance>() == 64);

/// One picture on the GPU, with a bind group for each sampler.
#[derive(Debug)]
pub(crate) struct GpuImage {
    pub(crate) size: (u32, u32),
    nearest: wgpu::BindGroup,
    linear: wgpu::BindGroup,
    _texture: wgpu::Texture,
}

/// Everything the image pipeline shares: layout, samplers and the registered pictures.
#[derive(Debug)]
pub(crate) struct ImageResources {
    pub(crate) layout: wgpu::BindGroupLayout,
    nearest: wgpu::Sampler,
    linear: wgpu::Sampler,
    /// Pictures the app registered.
    pub(crate) ids: HashMap<ImageId, GpuImage>,
    /// Rasterised texts, dropped when the text leaves the cache.
    pub(crate) texts: HashMap<TextKey, GpuImage>,
}

impl ImageResources {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image layout"),
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
            ],
        });
        let sampler = |label, filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        Self {
            layout,
            nearest: sampler("image nearest", wgpu::FilterMode::Nearest),
            linear: sampler("image linear", wgpu::FilterMode::Linear),
            ids: HashMap::new(),
            texts: HashMap::new(),
        }
    }

    /// Uploads straight-alpha `rgba` (premultiplying it) as a `width` by `height` texture.
    pub(crate) fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> GpuImage {
        let mut px = rgba.to_vec();
        for p in px.chunks_exact_mut(4) {
            let a = u32::from(p[3]);
            for c in &mut p[..3] {
                *c = ((u32::from(*c) * a + 127) / 255) as u8;
            }
        }
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("image"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &px,
        );
        let view = texture.create_view(&Default::default());
        let group = |sampler: &wgpu::Sampler| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("image"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            })
        };
        GpuImage {
            size: (width, height),
            nearest: group(&self.nearest),
            linear: group(&self.linear),
            _texture: texture,
        }
    }

    /// The picture behind `source`, if it is registered or rasterised.
    pub(crate) fn get(&self, source: ImageSource) -> Option<&GpuImage> {
        match source {
            ImageSource::Id(id) => self.ids.get(&id),
            ImageSource::Text(key, _) => self.texts.get(&key),
        }
    }

    /// The instance record and bind group for drawing `item`, or `None` when its picture is not
    /// held or nothing of it would show.
    pub(crate) fn instance(&self, item: &ImageItem) -> Option<(GpuImageInstance, wgpu::BindGroup)> {
        let img = self.get(item.source)?;
        let (tw, th) = (img.size.0 as f32, img.size.1 as f32);
        let rect = match item.source {
            ImageSource::Id(_) => image_rect(img.size, item.dst, item.fit, item.align)?,
            ImageSource::Text(_, k) => {
                let (w, h) = (tw * k, th * k);
                let (cx, cy) = (item.dst.x + item.dst.w / 2.0, item.dst.y + item.dst.h / 2.0);
                Rect::new(cx - w / 2.0, cy - h / 2.0, w, h)
            }
        };
        let integer = |ratio: f32| ratio >= 0.999 && (ratio - ratio.round()).abs() < 1e-3;
        let crisp = item.rotation == 0.0 && integer(rect.w / tw) && integer(rect.h / th);
        let group = if crisp { &img.nearest } else { &img.linear };
        let instance = GpuImageInstance {
            rect: [rect.x, rect.y, rect.w, rect.h],
            clip: item.clip,
            round: item.round_clip.rect,
            params: [item.rotation, item.alpha, item.round_clip.radius, 0.0],
        };
        Some((instance, group.clone()))
    }
}

/// Checks `data` against `limit` and the length rule, as the store contract asks.
pub(crate) fn check(data: &ImageData, limit: u32) -> Result<(), lune_ui_core::image::ImageError> {
    use lune_ui_core::image::ImageError;
    if !data.length_ok() {
        return Err(ImageError::BadLength);
    }
    if data.width.max(data.height) > limit {
        return Err(ImageError::TooLarge { limit });
    }
    Ok(())
}

//! Registered pictures and rasterised texts on the GPU: textures, the two samplers, the bind
//! group layout, and the per-draw instance records the image pipeline reads.

use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use lune_ui_core::image::{ImageData, ImageError, ImageId, Sampling, image_rect};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{ImageItem, ImageSource};
use lune_ui_core::text::{TextKey, TextRendering};

/// A text setting as the bits that tell two settings apart: what a rasterised text is keyed by, so
/// no raster is drawn under a setting other than the one it was made with.
pub(crate) type RenderingBits = [u32; 7];

/// The bits of `rendering`'s effective value (so `-0.0`, an invalid factor or a setting with
/// smoothing off never count as another setting than the one they draw as).
pub(crate) fn rendering_bits(rendering: TextRendering) -> RenderingBits {
    let r = rendering.effective();
    let d = r.stem_darkening;
    [
        u32::from(r.smoothing),
        u32::from(r.hinting),
        d.factors[0].to_bits(),
        d.factors[1].to_bits(),
        d.max_px[0].to_bits(),
        d.max_px[1].to_bits(),
        d.max_ppem.to_bits(),
    ]
}

/// One image instance for the image shader: the picture's rect, the plain clip, the rounded clip
/// rect, `[rotation, alpha, rounded clip radius, part]` (`part` is 1 when a part of the picture is
/// drawn, 0 for the whole picture), and the drawn part as `[u0, v0, du, dv]` in texture units.
/// Rects are physical pixels.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuImageInstance {
    pub(crate) rect: [f32; 4],
    pub(crate) clip: [f32; 4],
    pub(crate) round: [f32; 4],
    pub(crate) params: [f32; 4],
    pub(crate) uv: [f32; 4],
}

const _: () = assert!(size_of::<GpuImageInstance>() == 80);

/// One picture on the GPU, with a bind group for each sampler.
#[derive(Debug)]
pub(crate) struct GpuImage {
    pub(crate) size: (u32, u32),
    nearest: wgpu::BindGroup,
    linear: wgpu::BindGroup,
    texture: wgpu::Texture,
}

/// Everything the image pipeline shares: layout, samplers and the registered pictures.
#[derive(Debug)]
pub(crate) struct ImageResources {
    pub(crate) layout: wgpu::BindGroupLayout,
    nearest: wgpu::Sampler,
    linear: wgpu::Sampler,
    /// Pictures the app registered.
    pub(crate) ids: HashMap<ImageId, GpuImage>,
    /// Rasterised texts by text and text setting, dropped when the text leaves the cache or its
    /// setting is no longer drawn.
    pub(crate) texts: HashMap<(TextKey, RenderingBits), GpuImage>,
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
        let px = premultiplied(rgba);
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
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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
            texture,
        }
    }

    /// Writes the straight-alpha `data` (premultiplied as `upload` does) into the picture `id`
    /// with its top-left corner at `at`, in place: the texture and its bind groups stay. The
    /// caller has checked the length and that the rect lies inside the picture.
    pub(crate) fn write(
        &self,
        queue: &wgpu::Queue,
        id: ImageId,
        at: (u32, u32),
        data: &ImageData,
    ) -> Result<(), ImageError> {
        if !data.length_ok() {
            return Err(ImageError::BadLength);
        }
        let img = self.ids.get(&id).ok_or(ImageError::UnknownId)?;
        if u64::from(at.0) + u64::from(data.width) > u64::from(img.size.0)
            || u64::from(at.1) + u64::from(data.height) > u64::from(img.size.1)
        {
            return Err(ImageError::OutOfBounds);
        }
        if data.width == 0 || data.height == 0 {
            return Ok(());
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &img.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: at.0,
                    y: at.1,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &premultiplied(&data.rgba),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * data.width),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: data.width,
                height: data.height,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    /// The picture behind `source`, if it is registered or rasterised under the setting `bits`.
    pub(crate) fn get(&self, source: ImageSource, bits: RenderingBits) -> Option<&GpuImage> {
        match source {
            ImageSource::Id(id) => self.ids.get(&id),
            ImageSource::Text(key, _) => self.texts.get(&(key, bits)),
        }
    }

    /// The instance record and bind group for drawing `item`, or `None` when its picture is not
    /// held or nothing of it would show.
    pub(crate) fn instance(
        &self,
        item: &ImageItem,
        bits: RenderingBits,
    ) -> Option<(GpuImageInstance, wgpu::BindGroup)> {
        let img = self.get(item.source, bits)?;
        let (tw, th) = img.size;
        // The picture's pixels the draw shows: its part, or the whole texture.
        let part = match item.part {
            Some(p) if matches!(item.source, ImageSource::Id(_)) => {
                if u64::from(p.x) + u64::from(p.w) > u64::from(tw)
                    || u64::from(p.y) + u64::from(p.h) > u64::from(th)
                {
                    return None;
                }
                Some(p)
            }
            _ => None,
        };
        let (sw, sh) = part.map_or((tw, th), |p| (p.w, p.h));
        let rect = match item.source {
            ImageSource::Id(_) => image_rect((sw, sh), item.dst, item.fit, item.align)?,
            ImageSource::Text(_, k) => {
                let (w, h) = (tw as f32 * k, th as f32 * k);
                let (cx, cy) = (item.dst.x + item.dst.w / 2.0, item.dst.y + item.dst.h / 2.0);
                Rect::new(cx - w / 2.0, cy - h / 2.0, w, h)
            }
        };
        let integer = |ratio: f32| ratio >= 0.999 && (ratio - ratio.round()).abs() < 1e-3;
        let crisp = match item.sampling {
            Sampling::Nearest => true,
            Sampling::Linear => false,
            Sampling::Auto => {
                item.rotation == 0.0 && integer(rect.w / sw as f32) && integer(rect.h / sh as f32)
            }
        };
        let group = if crisp { &img.nearest } else { &img.linear };
        let uv = part.map_or([0.0, 0.0, 1.0, 1.0], |p| {
            let (tw, th) = (tw as f32, th as f32);
            [
                p.x as f32 / tw,
                p.y as f32 / th,
                p.w as f32 / tw,
                p.h as f32 / th,
            ]
        });
        let instance = GpuImageInstance {
            rect: [rect.x, rect.y, rect.w, rect.h],
            clip: item.clip,
            round: item.round_clip.rect,
            params: [
                item.rotation,
                item.alpha,
                item.round_clip.radius,
                if part.is_some() { 1.0 } else { 0.0 },
            ],
            uv,
        };
        Some((instance, group.clone()))
    }
}

/// `rgba` (straight alpha) with each colour channel scaled by its alpha, rounded.
fn premultiplied(rgba: &[u8]) -> Vec<u8> {
    let mut px = rgba.to_vec();
    for p in px.chunks_exact_mut(4) {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
    px
}

/// Checks `data` against `limit` and the length rule, as the store contract asks.
pub(crate) fn check(data: &ImageData, limit: u32) -> Result<(), ImageError> {
    if !data.length_ok() {
        return Err(ImageError::BadLength);
    }
    if data.width.max(data.height) > limit {
        return Err(ImageError::TooLarge { limit });
    }
    Ok(())
}

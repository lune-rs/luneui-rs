//! The glyph atlas: one mask texture and one colour texture of rasterized glyphs, packed by
//! etagere, with least-recently-used eviction and growth.
//!
//! Vendored from glyphon 0.12.0, `src/text_atlas.rs`, MIT (`licenses/glyphon-LICENSE-MIT`);
//! changes: items are `pub(crate)`, imports point at the parent module, custom glyphs are removed
//! (the cache key is `GlyphKey`: cosmic-text's `CacheKey` and the glyph's `GlyphParams`, and a
//! re-upload rasterizes through `GlyphRasterizer`), the colour mode is fixed to the web one (a
//! non-sRGB colour texture), `TextAtlas::new` replaces `with_color_mode`, two `unwrap` calls
//! became `expect` and a `let ... else` that skips a glyph that cannot be re-rasterized.

use cosmic_text::FontSystem;
use etagere::{Allocation, BucketedAtlasAllocator, size2};
use lru::LruCache;
use rustc_hash::FxHasher;
use std::{collections::HashSet, hash::BuildHasherDefault};
use wgpu::{
    BindGroup, DepthStencilState, Device, Extent3d, MultisampleState, Origin3d, Queue,
    RenderPipeline, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

use super::{Cache, ContentType, GlyphDetails, GlyphKey, GpuCacheStatus, State};
use crate::raster::GlyphRasterizer;

type Hasher = BuildHasherDefault<FxHasher>;

pub(crate) struct InnerAtlas {
    pub(crate) kind: Kind,
    pub(crate) texture: Texture,
    pub(crate) texture_view: TextureView,
    pub(crate) packer: BucketedAtlasAllocator,
    pub(crate) size: u32,
    pub(crate) glyph_cache: LruCache<GlyphKey, GlyphDetails, Hasher>,
    pub(crate) glyphs_in_use: HashSet<GlyphKey, Hasher>,
    pub(crate) max_texture_dimension_2d: u32,
}

impl InnerAtlas {
    const INITIAL_SIZE: u32 = 256;

    fn new(state: &State, kind: Kind) -> Self {
        let max_texture_dimension_2d = state.device.limits().max_texture_dimension_2d;
        let size = Self::INITIAL_SIZE.min(max_texture_dimension_2d);

        let packer = BucketedAtlasAllocator::new(size2(size as i32, size as i32));

        // Create a texture to use for our atlas
        let texture = state.device.create_texture(&TextureDescriptor {
            label: Some("glyphon atlas"),
            size: Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: kind.texture_format(),
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        let glyph_cache = LruCache::unbounded_with_hasher(Hasher::default());
        let glyphs_in_use = HashSet::with_hasher(Hasher::default());

        Self {
            kind,
            texture,
            texture_view,
            packer,
            size,
            glyph_cache,
            glyphs_in_use,
            max_texture_dimension_2d,
        }
    }

    pub(crate) fn try_allocate(&mut self, width: usize, height: usize) -> Option<Allocation> {
        let size = size2(width as i32, height as i32);

        loop {
            let allocation = self.packer.allocate(size);

            if allocation.is_some() {
                return allocation;
            }

            // Try to free least recently used allocation
            let (mut key, mut value) = self.glyph_cache.peek_lru()?;

            // Find a glyph with an actual size
            while value.atlas_id.is_none() {
                // All sized glyphs are in use, cache is full
                if self.glyphs_in_use.contains(key) {
                    return None;
                }

                let _ = self.glyph_cache.pop_lru();

                (key, value) = self.glyph_cache.peek_lru()?;
            }

            // All sized glyphs are in use, cache is full
            if self.glyphs_in_use.contains(key) {
                return None;
            }

            let (_, value) = self
                .glyph_cache
                .pop_lru()
                .expect("peek_lru returned an entry just above");
            self.packer.deallocate(
                value
                    .atlas_id
                    .expect("the loop above stopped at a sized glyph"),
            );
        }
    }

    /// The side in px of this atlas texture.
    pub(crate) fn side(&self) -> u32 {
        self.size
    }

    pub(crate) fn num_channels(&self) -> usize {
        self.kind.num_channels()
    }

    pub(crate) fn grow(
        &mut self,
        state: &State,
        font_system: &mut FontSystem,
        glyphs: &mut GlyphRasterizer,
    ) -> bool {
        if self.size >= self.max_texture_dimension_2d {
            return false;
        }

        // Grow each dimension by a factor of 2. The growth factor was chosen to match the growth
        // factor of `Vec`.`
        const GROWTH_FACTOR: u32 = 2;
        let new_size = (self.size * GROWTH_FACTOR).min(self.max_texture_dimension_2d);

        self.packer.grow(size2(new_size as i32, new_size as i32));

        // Create a texture to use for our atlas
        self.texture = state.device.create_texture(&TextureDescriptor {
            label: Some("glyphon atlas"),
            size: Extent3d {
                width: new_size,
                height: new_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.kind.texture_format(),
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Re-upload glyphs
        for (&key, glyph) in &self.glyph_cache {
            let (x, y) = match glyph.gpu_cache {
                GpuCacheStatus::InAtlas { x, y, .. } => (x, y),
                GpuCacheStatus::SkipRasterization => continue,
            };

            // A glyph that was rasterized once rasterizes again, with the key's own params, not
            // the frame's; if the font is gone, skip it.
            let Some(image) = glyphs.image(font_system, key.glyph, key.params) else {
                continue;
            };
            let width = image.placement.width as usize;
            let height = image.placement.height as usize;
            let image_data = image.data;

            state.queue.write_texture(
                TexelCopyTextureInfo {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: Origin3d {
                        x: x as u32,
                        y: y as u32,
                        z: 0,
                    },
                    aspect: TextureAspect::All,
                },
                &image_data,
                TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width as u32 * self.kind.num_channels() as u32),
                    rows_per_image: None,
                },
                Extent3d {
                    width: width as u32,
                    height: height as u32,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.texture_view = self.texture.create_view(&TextureViewDescriptor::default());
        self.size = new_size;

        true
    }

    fn trim(&mut self) {
        self.glyphs_in_use.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Mask,
    Color,
}

impl Kind {
    fn num_channels(self) -> usize {
        match self {
            Kind::Mask => 1,
            Kind::Color => 4,
        }
    }

    fn texture_format(self) -> wgpu::TextureFormat {
        match self {
            Kind::Mask => TextureFormat::R8Unorm,
            // The web colour mode: sRGB colours stored in a linear texture, blended as is.
            Kind::Color => TextureFormat::Rgba8Unorm,
        }
    }
}

/// An atlas containing a cache of rasterized glyphs that can be rendered.
pub(crate) struct TextAtlas {
    cache: Cache,
    pub(crate) bind_group: BindGroup,
    pub(crate) color_atlas: InnerAtlas,
    pub(crate) mask_atlas: InnerAtlas,
    pub(crate) format: TextureFormat,
}

impl TextAtlas {
    /// Creates a new [`TextAtlas`].
    pub(crate) fn new(
        device: &Device,
        queue: &Queue,
        cache: &Cache,
        format: TextureFormat,
    ) -> Self {
        let state = State { device, queue };
        let color_atlas = InnerAtlas::new(&state, Kind::Color);
        let mask_atlas = InnerAtlas::new(&state, Kind::Mask);

        let bind_group = cache.create_atlas_bind_group(
            device,
            &color_atlas.texture_view,
            &mask_atlas.texture_view,
        );

        Self {
            cache: cache.clone(),
            bind_group,
            color_atlas,
            mask_atlas,
            format,
        }
    }

    /// The larger side in px of the two atlas textures.
    pub(crate) fn max_side(&self) -> u32 {
        self.color_atlas.side().max(self.mask_atlas.side())
    }

    pub(crate) fn trim(&mut self) {
        self.mask_atlas.trim();
        self.color_atlas.trim();
    }

    pub(crate) fn grow(
        &mut self,
        state: &State,
        font_system: &mut FontSystem,
        glyphs: &mut GlyphRasterizer,
        content_type: ContentType,
    ) -> bool {
        let did_grow = match content_type {
            ContentType::Mask => self.mask_atlas.grow(state, font_system, glyphs),
            ContentType::Color => self.color_atlas.grow(state, font_system, glyphs),
        };

        if did_grow {
            self.rebind(state.device);
        }

        did_grow
    }

    pub(crate) fn inner_for_content_mut(&mut self, content_type: ContentType) -> &mut InnerAtlas {
        match content_type {
            ContentType::Color => &mut self.color_atlas,
            ContentType::Mask => &mut self.mask_atlas,
        }
    }

    pub(crate) fn get_or_create_pipeline(
        &self,
        device: &Device,
        multisample: MultisampleState,
        depth_stencil: Option<DepthStencilState>,
    ) -> RenderPipeline {
        self.cache
            .get_or_create_pipeline(device, self.format, multisample, depth_stencil)
    }

    fn rebind(&mut self, device: &wgpu::Device) {
        self.bind_group = self.cache.create_atlas_bind_group(
            device,
            &self.color_atlas.texture_view,
            &self.mask_atlas.texture_view,
        );
    }
}

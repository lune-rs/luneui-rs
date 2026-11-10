//! The glyph rasterizer: one glyph's coverage image for a cosmic-text cache key and the
//! [`GlyphParams`] (hinting and stem darkening) it is drawn with. The toolkit's text paths rasterize
//! through it, and an app's own glyph renderer can too, so both draw the same pixels.

use std::fmt;

use cosmic_text::{CacheKey, CacheKeyFlags, FontSystem, SwashCache, SwashImage};
use lune_ui_core::text::GlyphParams;
use swash::scale::image::Content;
use swash::scale::{ScaleContext, Source};
use swash::zeno::{Angle, Fill, Format, Mask, Origin, Style, Transform, Vector};

/// Rasterizes single glyphs with a [`GlyphParams`]: hinting on or off, and the outline grown by a
/// stem-darkening dilation on each side.
///
/// [`image`](Self::image) follows three rules:
/// 1. `GlyphParams::OFF` returns cosmic-text's own image of the key, unchanged
///    (`SwashCache::get_image_uncached`): the look without smoothing.
/// 2. Otherwise hinting off adds `CacheKeyFlags::DISABLE_HINTING` to the key (a key that already
///    carries it stays unhinted) and cosmic-text picks the glyph's source as it always does; a
///    colour outline or colour bitmap (an emoji, say) is returned as cosmic-text drew it, never
///    dilated.
/// 3. An outline glyph is filled again from its outline, grown by `params.dilation()` on each
///    side and moved back so it stays centred where it was, through the same scaler settings and
///    mask steps cosmic-text uses. At zero dilation that gives cosmic-text's bytes exactly.
///
/// An app's own glyph cache must key each image by the glyph's `GlyphParams` as well as its
/// `CacheKey` (two settings give two images), and must leave `DISABLE_HINTING` to this rasterizer
/// rather than set it itself. Hinting here is the rasterization hinting of the glyph outline; it is
/// distinct from the text shaper's metrics hinting (cosmic-text's `Buffer::set_hinting`, which
/// rounds the layout, is off by default, and the toolkit never sets it).
///
/// The `FontSystem` and `CacheKey` passed in must come from the toolkit's cosmic-text version
/// (0.19): an app's dependency tree must hold that one version (`cargo tree -i cosmic-text` lists
/// one).
///
/// Deterministic: the same inputs give the same bytes on any instance; nothing is cached between
/// calls.
pub struct GlyphRasterizer {
    swash: SwashCache,
    scale: ScaleContext,
}

impl GlyphRasterizer {
    /// A rasterizer with empty scaling contexts.
    pub fn new() -> Self {
        Self {
            swash: SwashCache::new(),
            scale: ScaleContext::new(),
        }
    }

    /// The coverage image of `key` drawn with `params` (the rules are on the type), or `None` when
    /// the font or the glyph is missing.
    pub fn image(
        &mut self,
        fonts: &mut FontSystem,
        key: CacheKey,
        params: GlyphParams,
    ) -> Option<SwashImage> {
        if params == GlyphParams::OFF {
            return self.swash.get_image_uncached(fonts, key);
        }
        let mut key = key;
        if !params.hinting() {
            key.flags |= CacheKeyFlags::DISABLE_HINTING;
        }
        let image = self.swash.get_image_uncached(fonts, key)?;
        if !matches!(image.source, Source::Outline) {
            return Some(image);
        }
        Some(self.outline(fonts, key, params.dilation()).unwrap_or(image))
    }

    /// The cosmic-text cache this rasterizer draws rule 1 and 2 images with.
    pub(crate) fn swash(&mut self) -> &mut SwashCache {
        &mut self.swash
    }

    /// Rule 3: the outline of `key`, grown by `dx`, `dy` a side and re-centred, filled as swash's
    /// `Render` fills an outline with cosmic-text's settings.
    fn outline(
        &mut self,
        fonts: &mut FontSystem,
        key: CacheKey,
        [dx, dy]: [f32; 2],
    ) -> Option<SwashImage> {
        let font = fonts.get_font(key.font_id, key.font_weight)?;
        let face = font.as_swash();
        let wght = swash::Tag::from_be_bytes(*b"wght");
        let mut builder = self
            .scale
            .builder(face)
            .size(f32::from_bits(key.font_size_bits))
            .hint(!key.flags.contains(CacheKeyFlags::DISABLE_HINTING));
        if let Some(axis) = face.variations().find_by_tag(wght) {
            builder = builder.normalized_coords(face.variations().normalized_coords([(
                wght,
                f32::from(key.font_weight.0).clamp(axis.min_value(), axis.max_value()),
            )]));
        }
        let mut scaler = builder.build();
        let mut outline = scaler.scale_outline(key.glyph_id)?;
        if dx != 0.0 || dy != 0.0 {
            // `embolden` grows the box by twice the strength with its min corner fixed; the
            // translate moves it back so the growth is `dx`, `dy` on each side.
            outline.embolden(dx, dy);
            outline.transform(&Transform::translation(-dx, -dy));
        }
        if key.flags.contains(CacheKeyFlags::FAKE_ITALIC) {
            outline.transform(&Transform::skew(
                Angle::from_degrees(14.0),
                Angle::from_degrees(0.0),
            ));
        }
        let offset = if key.flags.contains(CacheKeyFlags::PIXEL_FONT) {
            Vector::new(key.x_bin.as_float().round(), key.y_bin.as_float().round())
        } else {
            Vector::new(key.x_bin.as_float(), key.y_bin.as_float())
        };
        let mut image = SwashImage::new();
        // `inspect` sizes the buffer, and also sets the mask height that `placement.top` reads.
        let placement = Mask::new(outline.path())
            .format(Format::Alpha)
            .origin(Origin::BottomLeft)
            .style(Style::Fill(Fill::NonZero))
            .offset(offset)
            .render_offset(offset)
            .inspect(|format, width, height| {
                image.data.resize(format.buffer_size(width, height), 0);
            })
            .render_into(&mut image.data[..], None);
        image.placement = placement;
        image.content = Content::Mask;
        image.source = Source::Outline;
        Some(image)
    }
}

impl Default for GlyphRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for GlyphRasterizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlyphRasterizer").finish_non_exhaustive()
    }
}

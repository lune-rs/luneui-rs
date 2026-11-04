//! The glyphon text system: a shaped-buffer cache behind [`TextSystem`].

use std::collections::HashMap;

use glyphon::{
    Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap,
};

use lune_ui_core::text::{Font, Spec, Style, TextKey, TextSystem};

/// A shaped text drawn into pixels by [`TextCache::rasterize`]: straight-alpha RGBA, row by row
/// from the top left, with a margin of [`RASTER_MARGIN`] px around the text's own box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRaster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Transparent pixels around a rasterised text, so glyphs that overhang their advance box are
/// not cut.
pub const RASTER_MARGIN: u32 = 2;

#[derive(Debug)]
struct Entry {
    buffer: Buffer,
    size: (f32, f32),
    last_used: u64,
}

/// The glyphon [`TextSystem`]: shaped buffers keyed by [`TextKey`], so the UI can be drawn
/// immediate-mode. One cache serves any number of targets and scales.
#[derive(Debug)]
pub struct TextCache {
    pub fonts: FontSystem,
    ui_family: String,
    mono_family: String,
    entries: HashMap<TextKey, Entry>,
    frame: u64,
}

impl TextCache {
    /// `font_data` is loaded in order; `ui_family` and `mono_family` are the family names
    /// that [`Font::Ui`] and [`Font::Mono`] resolve to. The caller supplies all of it; this
    /// crate embeds no fonts; with none, every text is an empty layout of zero size. The locale is `en-US`; see [`TextCache::with_locale`].
    pub fn new(font_data: &[&[u8]], ui_family: &str, mono_family: &str) -> Self {
        Self::with_locale(font_data, ui_family, mono_family, "en-US")
    }

    /// As [`TextCache::new`], with the shaping locale given. No system font is loaded and the
    /// system locale is not read: the app supplies every font, so the result depends only on
    /// its arguments.
    pub fn with_locale(
        font_data: &[&[u8]],
        ui_family: &str,
        mono_family: &str,
        locale: &str,
    ) -> Self {
        let mut db = glyphon::fontdb::Database::new();
        for data in font_data {
            db.load_font_data(data.to_vec());
        }
        let fonts = FontSystem::new_with_locale_and_db(locale.to_owned(), db);
        Self {
            fonts,
            ui_family: ui_family.to_owned(),
            mono_family: mono_family.to_owned(),
            entries: HashMap::new(),
            frame: 0,
        }
    }

    /// The font system (for glyphon's `prepare`) alongside a lookup of shaped buffers.
    pub fn split<'a>(
        &'a mut self,
    ) -> (
        &'a mut FontSystem,
        impl Fn(TextKey) -> Option<&'a Buffer> + 'a,
    ) {
        let entries = &self.entries;
        (&mut self.fonts, move |key| {
            entries.get(&key).map(|e| &e.buffer)
        })
    }

    /// Whether a shaped text is cached under `key`.
    pub fn contains(&self, key: TextKey) -> bool {
        self.entries.contains_key(&key)
    }

    /// Draws the cached text `key` into straight-alpha RGBA pixels at the scale it was shaped at
    /// (its own colours, on transparent), with [`RASTER_MARGIN`] px around its box. `None` for a
    /// key that is not cached, an empty layout (no fonts, no text) or a side above 8192 px.
    pub fn rasterize(&mut self, key: TextKey, swash: &mut SwashCache) -> Option<TextRaster> {
        let entry = self.entries.get_mut(&key)?;
        let (mut w, mut h) = (0.0_f32, 0.0_f32);
        for run in entry.buffer.layout_runs() {
            w = w.max(run.line_w);
            h = h.max(run.line_top + run.line_height);
        }
        let m = RASTER_MARGIN;
        let (width, height) = (w.ceil() as u32 + 2 * m, h.ceil() as u32 + 2 * m);
        if w <= 0.0 || h <= 0.0 || width > 8192 || height > 8192 {
            return None;
        }
        let mut rgba = vec![0u8; 4 * width as usize * height as usize];
        entry.buffer.draw(
            &mut self.fonts,
            swash,
            Color::rgb(0xCC, 0xCC, 0xCC),
            |x, y, rw, rh, color| {
                let src = f32::from(color.a()) / 255.0;
                if src <= 0.0 {
                    return;
                }
                for py in y.max(-(m as i32))..(y + rh as i32).min(height as i32 - m as i32) {
                    for px in x.max(-(m as i32))..(x + rw as i32).min(width as i32 - m as i32) {
                        let i = 4
                            * ((py + m as i32) as usize * width as usize
                                + (px + m as i32) as usize);
                        let dst = f32::from(rgba[i + 3]) / 255.0;
                        let out = src + dst * (1.0 - src);
                        let mix = |s: u8, d: u8| {
                            (f32::from(s) * src + f32::from(d) * dst * (1.0 - src)) / out
                        };
                        rgba[i] = mix(color.r(), rgba[i]).round() as u8;
                        rgba[i + 1] = mix(color.g(), rgba[i + 1]).round() as u8;
                        rgba[i + 2] = mix(color.b(), rgba[i + 2]).round() as u8;
                        rgba[i + 3] = (out * 255.0).round() as u8;
                    }
                }
            },
        );
        Some(TextRaster {
            width,
            height,
            rgba,
        })
    }

    /// Drop buffers unused for a while; call once per frame.
    pub fn end_frame(&mut self) {
        let frame = self.frame;
        self.entries
            .retain(|_, e| frame.saturating_sub(e.last_used) < 180);
        self.frame += 1;
    }
}

impl TextSystem for TextCache {
    /// Shapes `spec` at `scale` if needed; returns its cache key and logical size.
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
        let key = TextKey(spec.cache_key(scale));
        let frame = self.frame;
        if let Some(e) = self.entries.get_mut(&key) {
            e.last_used = frame;
            return (key, e.size);
        }
        if self.fonts.db().faces().next().is_none() {
            // No font face was supplied: shaping would panic inside cosmic-text, so the text
            // is an empty, zero-sized layout and draws nothing.
            let buffer = Buffer::new_empty(Metrics::new(1.0, 1.0));
            self.entries.insert(
                key,
                Entry {
                    buffer,
                    size: (0.0, 0.0),
                    last_used: frame,
                },
            );
            return (key, (0.0, 0.0));
        }
        let s = scale;
        let first = spec
            .runs
            .first()
            .map(|r| r.1)
            .unwrap_or(Style::ui(13.0, [255; 4]));
        let mut buffer = Buffer::new(
            &mut self.fonts,
            Metrics::new(first.size * s, first.line * s),
        );
        buffer.set_size(spec.wrap.map(|w| w * s), None);
        buffer.set_wrap(if spec.wrap.is_some() {
            Wrap::WordOrGlyph
        } else {
            Wrap::None
        });
        let attrs = |st: &Style| {
            let family = match st.font {
                Font::Ui => Family::Name(&self.ui_family),
                Font::Mono => Family::Name(&self.mono_family),
            };
            let [r, g, b, a] = st.color;
            Attrs::new()
                .family(family)
                .weight(Weight(st.weight))
                .color(Color::rgba(r, g, b, a))
        };
        let default = attrs(&first);
        buffer.set_rich_text(
            spec.runs.iter().map(|(t, st)| (t.as_str(), attrs(st))),
            &default,
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.fonts, false);
        let mut w: f32 = 0.0;
        let mut lines = 0;
        for run in buffer.layout_runs() {
            w = w.max(run.line_w);
            lines += 1;
        }
        let size = (w / s, lines.max(1) as f32 * first.line);
        self.entries.insert(
            key,
            Entry {
                buffer,
                size,
                last_used: frame,
            },
        );
        (key, size)
    }
}

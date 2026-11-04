//! The glyphon text system: a shaped-buffer cache behind [`TextSystem`].

use std::borrow::Cow;
use std::collections::HashMap;

use glyphon::{
    Attrs, Buffer, Color, Cursor, Family, FontSystem, LayoutRun, Metrics, Shaping, SwashCache,
    Weight, Wrap,
};
use unicode_segmentation::UnicodeSegmentation;

use lune_ui_core::text::{
    CaretPos, Font, LineSpan, Spec, Style, TextKey, TextSystem, Unit, clamp_to_boundary,
    joined_text,
};

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

/// The runs as cosmic-text gets them: each line break of the joined text as one `\n`.
///
/// `BidiParagraphs` splits plain ASCII at `\n` and strips one `\r`, and any other text through
/// `BidiInfo`, where `\r` and `\n` are a break each, so the same CRLF would be one line or two
/// depending on the rest of the text. With no `\r` in it, both paths agree. A CRLF split across
/// two runs is one break too. The queries keep their offsets in the spec's text: [`line_bases`]
/// walks the joined text, where a CRLF is still two bytes, and a line's own text has no terminator.
fn shaped_runs(spec: &Spec) -> Vec<Cow<'_, str>> {
    let runs = &spec.runs;
    let next_is_lf = |i: usize| {
        let mut later = runs[i + 1..].iter().filter(|(t, _)| !t.is_empty());
        later.next().is_some_and(|(t, _)| t.starts_with('\n'))
    };
    runs.iter()
        .enumerate()
        .map(|(i, (t, _))| {
            if !t.contains('\r') {
                return Cow::Borrowed(t.as_str());
            }
            let mut text = t.replace("\r\n", "\n");
            if text.ends_with('\r') && next_is_lf(i) {
                text.pop();
            }
            Cow::Owned(text.replace('\r', "\n"))
        })
        .collect()
}

/// Where each buffer line starts in the spec's joined text, and whether the text ends in a line
/// terminator that opens one more, empty line (cosmic-text has no buffer line for it).
///
/// The buffer keeps each paragraph's text without its terminator and does not record which
/// terminator it was (`set_rich_text` gives every line `LineEnding::Lf`), so the offsets are found
/// by walking the joined text: a line's text, then one terminator (`\r\n` counts once).
fn line_bases(buffer: &Buffer, joined: &str) -> (Vec<usize>, bool) {
    let mut at = 0;
    let mut bases = Vec::with_capacity(buffer.lines.len());
    let mut open = false;
    for line in &buffer.lines {
        bases.push(at);
        at = (at + line.text().len()).min(joined.len());
        let rest = joined.get(at..).unwrap_or("");
        let term = if rest.starts_with("\r\n") {
            2
        } else {
            rest.chars().next().map_or(0, char::len_utf8)
        };
        at += term;
        open = term > 0 && at == joined.len();
    }
    (bases, open)
}

/// The caret's x for a byte `local` of the row's line that is in none of the row's glyphs: at a
/// wrap cosmic-text drops the whitespace that does not fit, so such bytes come after the row's
/// glyphs, or before them for a leading space ahead of a word wider than the wrap. The answer is
/// the row's edge on that side.
fn off_the_glyphs(run: &LayoutRun<'_>, local: usize) -> f32 {
    let left = run.glyphs.first().map_or(0.0, |g| g.x);
    let right = run.glyphs.last().map_or(0.0, |g| g.x + g.w);
    let before = local <= glyph_range(run.glyphs).0;
    if before != run.rtl { left } else { right }
}

/// The byte range of a layout run's glyphs in its buffer line (empty for an empty line).
fn glyph_range(glyphs: &[glyphon::LayoutGlyph]) -> (usize, usize) {
    let start = glyphs.iter().map(|g| g.start).min().unwrap_or(0);
    let end = glyphs.iter().map(|g| g.end).max().unwrap_or(0);
    (start, end)
}

impl TextCache {
    /// The height an unshaped query answers with: the first run's line height.
    fn fallback_height(spec: &Spec) -> f32 {
        spec.runs.first().map_or(17.0, |(_, st)| st.line)
    }

    /// The shaped buffer of `spec` at `scale`, after `prepare`.
    fn buffer_of(&mut self, spec: &Spec, scale: f32) -> Option<&Buffer> {
        let (key, _) = self.prepare(spec, scale);
        self.entries.get(&key).map(|e| &e.buffer)
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
        let shaped = shaped_runs(spec);
        buffer.set_rich_text(
            shaped
                .iter()
                .zip(&spec.runs)
                .map(|(t, (_, st))| (t.as_ref(), attrs(st))),
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
        // `caret` and `lines` count the empty line a final terminator opens, so the height does.
        lines += usize::from(line_bases(&buffer, &joined_text(spec)).1);
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

    fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos {
        let joined = joined_text(spec);
        let off = clamp_to_boundary(&joined, offset);
        let fallback = CaretPos {
            x: 0.0,
            y: 0.0,
            height: Self::fallback_height(spec),
            line: 0,
        };
        let Some(buffer) = self.buffer_of(spec, scale) else {
            return fallback;
        };
        let (bases, open) = line_bases(buffer, &joined);
        let runs: Vec<_> = buffer.layout_runs().collect();
        if open
            && off == joined.len()
            && let Some(last) = runs.last()
        {
            // After a final line terminator: the start of the empty line below the last row.
            return CaretPos {
                x: 0.0,
                y: (last.line_top + last.line_height) / scale,
                height: last.line_height / scale,
                line: runs.len(),
            };
        }
        // The visual line that shows `off`: the last run whose first glyph is at or before it, so
        // the offset where a wrapped line breaks belongs to the later row. A buffer line's first
        // run owns the line's start, even when a dropped space puts its first glyph later.
        let mut chosen = None;
        let mut line = None;
        for (row, run) in runs.iter().enumerate() {
            let base = bases.get(run.line_i).copied().unwrap_or(0);
            let first = line.replace(run.line_i) != Some(run.line_i);
            if base + if first { 0 } else { glyph_range(run.glyphs).0 } <= off {
                chosen = Some((row, run, base));
            }
        }
        let Some((row, run, base)) = chosen else {
            return fallback;
        };
        let local = (off - base).min(run.text.len());
        let mut x = run
            .cursor_position(&Cursor::new(run.line_i, local))
            .unwrap_or_else(|| off_the_glyphs(run, local));
        // Positions are sums of glyph advances; where the caret is at the end of the line, take
        // the line's own width, so the two agree to the last bit.
        if (x - run.line_w).abs() < 0.01 {
            x = run.line_w;
        }
        CaretPos {
            x: x / scale,
            y: run.line_top / scale,
            height: run.line_height / scale,
            line: row,
        }
    }

    fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize {
        if !(pos.0.is_finite() && pos.1.is_finite()) {
            return 0;
        }
        let text = joined_text(spec);
        let Some(buffer) = self.buffer_of(spec, scale) else {
            return 0;
        };
        let (bases, open) = line_bases(buffer, &text);
        let (x, y) = (pos.0 * scale, pos.1 * scale);
        let Some(top) = buffer.layout_runs().next().map(|r| r.line_top) else {
            return 0;
        };
        let (row_top, below) = buffer
            .layout_runs()
            .take_while(|r| r.line_top <= y)
            .last()
            .map_or((top, top), |r| (r.line_top, r.line_top + r.line_height));
        if open && y >= below {
            return text.len();
        }
        // The top of the row under `y`, or of the nearest one: `Buffer::hit` drops `x` outside the
        // rows (offset 0 above them, the end of the text below them), and the trait keeps the
        // column on the first or last line.
        let y = row_top;
        let Some(cursor) = buffer.hit(x, y) else {
            return 0;
        };
        let base = bases.get(cursor.line).copied().unwrap_or(0);
        clamp_to_boundary(&text, base + cursor.index)
    }

    fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan> {
        let joined = joined_text(spec);
        let len = joined.len();
        let height = Self::fallback_height(spec);
        let one = LineSpan {
            start: 0,
            end: len,
            y: 0.0,
            height,
        };
        let Some(buffer) = self.buffer_of(spec, scale) else {
            return vec![one];
        };
        let (bases, open) = line_bases(buffer, &joined);
        let mut lines: Vec<LineSpan> = buffer
            .layout_runs()
            .map(|run| {
                let base = bases.get(run.line_i).copied().unwrap_or(0);
                let (start, end) = glyph_range(run.glyphs);
                LineSpan {
                    start: (base + start).min(len),
                    end: (base + end).min(len),
                    y: run.line_top / scale,
                    height: run.line_height / scale,
                }
            })
            .collect();
        if lines.is_empty() {
            return vec![one];
        }
        if open && let Some(last) = lines.last().copied() {
            lines.push(LineSpan {
                start: len,
                end: len,
                y: last.y + last.height,
                height: last.height,
            });
        }
        lines
    }

    fn segments(&self, text: &str, unit: Unit) -> Vec<usize> {
        let mut out: Vec<usize> = match unit {
            Unit::Grapheme => text.grapheme_indices(true).map(|(i, _)| i).collect(),
            Unit::Word => text.split_word_bound_indices().map(|(i, _)| i).collect(),
        };
        out.push(text.len());
        out.dedup();
        out
    }
}

//! The glyphon text system: a shaped-buffer cache behind [`TextSystem`].

use std::borrow::Cow;
use std::collections::HashMap;

use glyphon::{
    Attrs, Buffer, Color, Cursor, Family, FontSystem, Metrics, Shaping, Style as FontStyle,
    SwashCache, Weight, Wrap,
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
    rows: Vec<Row>,
    size: (f32, f32),
    last_used: u64,
}

/// One visual row of a shaped text, as every query sees it: `lines` lists these, `caret` and `hit`
/// pick one from this list, so the three cannot name different rows. Offsets are bytes of the
/// spec's joined text; `top`, `height`, `left` and `right` are device px.
#[derive(Debug, Clone, Copy)]
struct Row {
    /// The index of the row's layout run. `None` for a row cosmic-text has no run for: the empty
    /// line a final terminator opens, or the one row of a text with no layout.
    run: Option<usize>,
    /// Where the row's buffer line starts.
    base: usize,
    /// The first offset the row owns: its first glyph, or the line's start on a line's first row,
    /// where the first glyph comes later when a space was dropped.
    start: usize,
    /// Where the row's first glyph starts.
    glyphs_start: usize,
    /// Where the row's glyphs end.
    end: usize,
    /// The last offset the caret shows on this row: `end`, or the grapheme before it where the row
    /// runs straight into the next, whose first offset that is.
    last: usize,
    top: f32,
    height: f32,
    /// The left edge of the row's leftmost glyph. `run.glyphs` is not in visual order on a
    /// right-to-left row, so this is a min over the row.
    left: f32,
    /// The right edge of the row's rightmost glyph.
    right: f32,
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

/// The rows of a shaped `buffer`, never empty: one per layout run, then the open line, or one row
/// of `height` device px holding all of `joined` when there is no layout.
fn rows_of(buffer: &Buffer, joined: &str, height: f32) -> Vec<Row> {
    let len = joined.len();
    let (bases, open) = line_bases(buffer, joined);
    let mut rows: Vec<Row> = Vec::new();
    let mut line = None;
    for (i, run) in buffer.layout_runs().enumerate() {
        let base = bases.get(run.line_i).copied().unwrap_or(0);
        let (first, last) = glyph_range(run.glyphs);
        let opens_line = line.replace(run.line_i) != Some(run.line_i);
        // Rows stay in text order: a soft row with no glyphs has no offsets to give, and a cluster
        // of several glyphs split over two rows keeps its bytes on the first.
        let floor = rows.last().map_or(0, |r| r.end);
        let glyphs_start = (base + first).min(len).max(floor);
        let end = (base + last).min(len).max(floor);
        let edges = run.glyphs.iter().map(|g| (g.x, g.x + g.w));
        let (left, right) = edges
            .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
            .unwrap_or((0.0, 0.0));
        rows.push(Row {
            run: Some(i),
            base,
            start: if opens_line {
                base.min(len)
            } else {
                glyphs_start
            },
            glyphs_start,
            end,
            last: end,
            top: run.line_top,
            height: run.line_height,
            left,
            right,
        });
    }
    let empty = |top, height| Row {
        run: None,
        base: len,
        start: len,
        glyphs_start: len,
        end: len,
        last: len,
        top,
        height,
        left: 0.0,
        right: 0.0,
    };
    match rows.last().copied() {
        Some(last) if open => rows.push(empty(last.top + last.height, last.height)),
        Some(_) => {}
        None => rows.push(Row {
            base: 0,
            start: 0,
            glyphs_start: 0,
            ..empty(0.0, height)
        }),
    }
    for i in 1..rows.len() {
        let (row, next) = (rows[i - 1], rows[i].start);
        if row.start < row.end && row.end == next {
            let shown = joined.get(row.start..row.end).unwrap_or("");
            let before = shown.grapheme_indices(true).next_back();
            rows[i - 1].last = row.start + before.map_or(0, |(at, _)| at);
        }
    }
    rows
}

/// The byte range of a layout run's glyphs in its buffer line (empty for an empty line).
fn glyph_range(glyphs: &[glyphon::LayoutGlyph]) -> (usize, usize) {
    let start = glyphs.iter().map(|g| g.start).min().unwrap_or(0);
    let end = glyphs.iter().map(|g| g.end).max().unwrap_or(0);
    (start, end)
}

impl Row {
    /// The caret's x for `off`, an offset this row owns. At a wrap cosmic-text drops the whitespace
    /// that does not fit, so an offset can be in none of the row's glyphs: after them, or before
    /// them for a leading space ahead of a word that fills the row. The answer is the row's edge on
    /// that side.
    fn caret_x(&self, buffer: &Buffer, off: usize) -> f32 {
        let Some(run) = self.run.and_then(|i| buffer.layout_runs().nth(i)) else {
            return 0.0;
        };
        let local = (off - self.base).min(run.text.len());
        let x = run
            .cursor_position(&Cursor::new(run.line_i, local))
            .unwrap_or(if (off <= self.glyphs_start) != run.rtl {
                self.left
            } else {
                self.right
            });
        // cosmic-text stops at the first glyph with this end; a cluster can have several.
        let x = if run.glyphs.iter().any(|g| g.start == local) {
            x
        } else {
            run.glyphs
                .iter()
                .filter(|g| g.end == local)
                .map(|g| (g.level.is_rtl(), g.x, g.x + g.w))
                .reduce(|a, b| (a.0, a.1.min(b.1), a.2.max(b.2)))
                .map_or(x, |(rtl, l, r)| if rtl { l } else { r })
        };
        // Positions are sums of glyph advances; where the caret is at the end of the line, take
        // the line's own width, so the two agree to the last bit.
        if (x - run.line_w).abs() < 0.01 {
            run.line_w
        } else {
            x
        }
    }
}

impl TextCache {
    /// The height an unshaped query answers with: the first run's line height.
    fn fallback_height(spec: &Spec) -> f32 {
        spec.runs.first().map_or(17.0, |(_, st)| st.line)
    }

    /// The entry of `spec` at `scale`, after `prepare`.
    fn entry_of(&mut self, spec: &Spec, scale: f32) -> Option<&Entry> {
        let (key, _) = self.prepare(spec, scale);
        self.entries.get(&key)
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
            let rows = rows_of(
                &buffer,
                &joined_text(spec),
                Self::fallback_height(spec) * scale,
            );
            self.entries.insert(
                key,
                Entry {
                    buffer,
                    rows,
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
                .style(if st.italic {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                })
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
        let w = buffer.layout_runs().fold(0.0_f32, |w, r| w.max(r.line_w));
        let rows = rows_of(&buffer, &joined_text(spec), first.line * s);
        let size = (w / s, rows.len() as f32 * first.line);
        self.entries.insert(
            key,
            Entry {
                buffer,
                rows,
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
        let Some(entry) = self.entry_of(spec, scale) else {
            return fallback;
        };
        // The last row that owns `off`, so the offset where a wrapped line breaks is the later row's.
        let Some(line) = entry.rows.iter().rposition(|r| r.start <= off) else {
            return fallback;
        };
        let row = entry.rows[line];
        CaretPos {
            x: row.caret_x(&entry.buffer, off) / scale,
            y: row.top / scale,
            height: row.height / scale,
            line,
        }
    }

    fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize {
        if !(pos.0.is_finite() && pos.1.is_finite()) {
            return 0;
        }
        let text = joined_text(spec);
        let Some(entry) = self.entry_of(spec, scale) else {
            return 0;
        };
        let (x, y) = (pos.0 * scale, pos.1 * scale);
        // The row under `y`, or the nearest one: `Buffer::hit` drops `x` outside the rows (offset 0
        // above them, the end of the text below them), and the trait keeps the column on the first
        // or last line. So it is asked at the row's own top, and its answer stays on that row.
        let rows = &entry.rows;
        let Some(row) = rows.iter().rfind(|r| r.top <= y).or(rows.first()) else {
            return 0;
        };
        let cursor = row.run.and_then(|_| entry.buffer.hit(x, row.top));
        let at = cursor.map_or(row.start, |c| row.base + c.index);
        clamp_to_boundary(&text, at.clamp(row.start, row.last))
    }

    fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan> {
        let Some(entry) = self.entry_of(spec, scale) else {
            return vec![LineSpan {
                start: 0,
                end: joined_text(spec).len(),
                y: 0.0,
                height: Self::fallback_height(spec),
            }];
        };
        entry
            .rows
            .iter()
            .map(|r| LineSpan {
                start: r.start,
                end: r.end,
                y: r.top / scale,
                height: r.height / scale,
            })
            .collect()
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

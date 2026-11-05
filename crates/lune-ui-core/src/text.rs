//! Text: what to draw ([`Spec`] runs of [`Style`]d text) and the [`TextSystem`] seam that shapes
//! it. Sizes are logical pixels; the scale is an argument of every call, so one text system can
//! serve windows of different scales.

use std::hash::{Hash, Hasher};

use crate::color::Rgba;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Font {
    Ui,
    Mono,
}

/// Font, size and colour for a run of text. Sizes are logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub font: Font,
    pub size: f32,
    pub line: f32,
    pub weight: u16,
    pub color: Rgba,
    /// Slanted. No italic face ships, so the shaper skews the roman one (D13).
    pub italic: bool,
}

impl Style {
    pub fn ui(size: f32, color: Rgba) -> Self {
        Self {
            font: Font::Ui,
            size,
            line: (size * 1.3).round(),
            weight: 400,
            color,
            italic: false,
        }
    }

    pub fn mono(size: f32, line: f32, color: Rgba) -> Self {
        Self {
            font: Font::Mono,
            size,
            line,
            weight: 400,
            color,
            italic: false,
        }
    }

    pub fn medium(self) -> Self {
        Self {
            weight: 500,
            ..self
        }
    }

    pub fn bold(self) -> Self {
        Self {
            weight: 700,
            ..self
        }
    }

    pub fn italic(self) -> Self {
        Self {
            italic: true,
            ..self
        }
    }

    fn hash_into(&self, h: &mut impl Hasher) {
        self.font.hash(h);
        self.size.to_bits().hash(h);
        self.line.to_bits().hash(h);
        self.weight.hash(h);
        self.color.hash(h);
        self.italic.hash(h);
    }
}

/// Text to draw: one or more styled runs; `wrap` is a width in logical px.
#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    pub runs: Vec<(String, Style)>,
    pub wrap: Option<f32>,
}

impl Spec {
    pub fn plain(text: impl Into<String>, style: Style) -> Self {
        Self {
            runs: vec![(text.into(), style)],
            wrap: None,
        }
    }

    pub fn wrapped(mut self, width: f32) -> Self {
        self.wrap = Some(width);
        self
    }

    /// A hash of everything that changes the shaped result at `scale`: the scale's bits, the
    /// wrap width, and every run's text and style (colour included).
    pub fn cache_key(&self, scale: f32) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        scale.to_bits().hash(&mut h);
        self.wrap.map(f32::to_bits).hash(&mut h);
        for (text, style) in &self.runs {
            text.hash(&mut h);
            style.hash_into(&mut h);
        }
        h.finish()
    }
}

/// Identifies one prepared (shaped) text: equal spec and scale give an equal key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextKey(pub u64);

/// Where the caret goes at a byte offset of a spec's text, in logical px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretPos {
    /// The caret's x within its line, from the line's left edge.
    pub x: f32,
    /// The top of the caret's line.
    pub y: f32,
    /// The line's height.
    pub height: f32,
    /// The visual line (a wrapped line counts once per row), counted from 0.
    pub line: usize,
}

/// One visual line of a shaped spec: the byte range of the runs' joined text it shows (a line
/// ending is not part of it) and its place, in logical px. An offset belongs to the last line whose
/// `start` is at or before it, so the rows are in text order and a line's first row starts at the
/// line's start, also when a leading space was dropped at a wrap and its first glyph comes later.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSpan {
    pub start: usize,
    pub end: usize,
    pub y: f32,
    pub height: f32,
}

/// A kind of text boundary for [`TextSystem::segments`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Extended grapheme clusters (UAX #29): what a caret steps over.
    Grapheme,
    /// Word segments (UAX #29 word bounds): words, runs of whitespace and single other
    /// characters, in order, so they tile the text.
    Word,
}

/// The text seam: shapes specs and measures them. Sizes are logical pixels; `scale` is device
/// pixels per logical pixel. Preparing the same spec at the same scale gives the same key and
/// size; entries of several scales may coexist.
///
/// The queries after `measure` read the layout `prepare` makes. Offsets are bytes of the spec's
/// runs joined into one string; one that is past the end, or inside a character, clamps down to
/// the end or to the character's start. Nothing panics, and a system with nothing to lay out
/// (empty text, no fonts) answers with finite values. A line break is `\n`, `\r\n` (even across
/// two runs) or a lone `\r`, whatever else the text holds, and the offsets count it as the bytes
/// it is in the spec.
pub trait TextSystem {
    /// Shapes `spec` at `scale` if needed; returns its key and logical size (width, height).
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32));

    /// The logical size `prepare` would return.
    fn measure(&mut self, spec: &Spec, scale: f32) -> (f32, f32) {
        self.prepare(spec, scale).1
    }

    /// Where the caret sits at `offset`, on the line [`LineSpan`] gives it to. For one unwrapped
    /// line, the caret at the end of the text has exactly the width `prepare` returns, and the
    /// caret at 0 has `x == 0`.
    fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos;

    /// The offset nearest to `pos`, a point in logical px from the spec's top-left corner:
    /// the nearer side of the character under it, the first or last offset of the line when it is
    /// left or right of the text, the first or last line when it is above or below. The answer's
    /// caret is on the line under `pos`: where a soft-wrapped row runs into the next, the row's end
    /// is the next row's, so the row's last offset is the grapheme before it. A `pos` that is not
    /// finite gives 0.
    fn hit(&mut self, spec: &Spec, scale: f32, pos: (f32, f32)) -> usize;

    /// The visual lines, top to bottom; there is always at least one.
    fn lines(&mut self, spec: &Spec, scale: f32) -> Vec<LineSpan>;

    /// The sorted boundaries of `unit` in `text`, including 0 and `text.len()`.
    fn segments(&self, text: &str, unit: Unit) -> Vec<usize>;
}

/// The runs of `spec` joined into one string: the text the offsets of [`TextSystem`] index.
pub fn joined_text(spec: &Spec) -> String {
    spec.runs.iter().map(|(t, _)| t.as_str()).collect()
}

/// `offset` clamped into `text` and down to a character boundary.
pub fn clamp_to_boundary(text: &str, offset: usize) -> usize {
    let mut at = offset.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// The mark an elided text ends in.
const ELLIPSIS: &str = "\u{2026}";

/// The style of the run holding the byte before `at`, else of the first run with text.
fn style_before(spec: &Spec, at: usize) -> Option<Style> {
    let mut end = 0;
    for (text, style) in &spec.runs {
        end += text.len();
        if !text.is_empty() && end >= at {
            return Some(*style);
        }
    }
    None
}

/// `spec`'s text up to `bounds[i]` with trailing whitespace dropped and the ellipsis after it, in
/// the last kept run's style, or in `first` alone when nothing is kept. The runs before the cut
/// stay as they are.
fn cut_at(spec: &Spec, text: &str, bounds: &[usize], mut i: usize, first: Style) -> Spec {
    while i > 0
        && text[bounds[i - 1]..bounds[i]]
            .chars()
            .all(char::is_whitespace)
    {
        i -= 1;
    }
    let mut left = bounds[i];
    if left == 0 {
        return Spec::plain(ELLIPSIS, first);
    }
    let mut runs = Vec::new();
    for (run, style) in &spec.runs {
        if left == 0 {
            break;
        }
        let keep = left.min(run.len());
        runs.push((run[..keep].to_string(), *style));
        left -= keep;
    }
    if let Some((last, _)) = runs.last_mut() {
        last.push_str(ELLIPSIS);
    }
    Spec { runs, wrap: None }
}

/// `spec` cut to one line at most `max_w` wide, ending in an ellipsis in the last kept run's style.
///
/// A spec that fits on one line comes back equal, and nothing but it is shaped. A line break cuts
/// there, also when the first line fits. Only the first line counts, so the result is one line; the
/// cut falls on a grapheme boundary and trailing whitespace goes with it. The result's width is at
/// most `max_w` when the ellipsis alone fits; otherwise the result is the ellipsis alone. A spec
/// with `wrap` set, an empty text or a `max_w` that is not finite comes back as it is.
///
/// The search reads carets of the shaped `spec`, so it shapes no prefix. A caret ignores what the
/// cut changes (kerning across it, joining, the order of mixed-direction text), so the result is
/// measured, and only a result that is over `max_w` searches again by measuring.
pub fn elide(system: &mut dyn TextSystem, spec: &Spec, scale: f32, max_w: f32) -> Spec {
    let text = joined_text(spec);
    if !max_w.is_finite() || spec.wrap.is_some() || text.is_empty() {
        return spec.clone();
    }
    let head = text
        .find(['\n', '\r'])
        .map_or(text.as_str(), |at| &text[..at]);
    if head.len() == text.len() && system.measure(spec, scale).0 <= max_w {
        return spec.clone();
    }
    let Some(first) = style_before(spec, 0) else {
        return spec.clone();
    };
    let alone = Spec::plain(ELLIPSIS, first);
    if system.measure(&alone, scale).0 > max_w {
        return alone;
    }
    let bounds = system.segments(head, Unit::Grapheme);

    let mut marks: Vec<(Style, f32)> = Vec::new();
    let (mut lo, mut hi) = (0, bounds.len());
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        let style = style_before(spec, bounds[mid]).unwrap_or(first);
        let mark = match marks.iter().find(|m| m.0 == style) {
            Some(m) => m.1,
            None => {
                let w = system.measure(&Spec::plain(ELLIPSIS, style), scale).0;
                marks.push((style, w));
                w
            }
        };
        if system.caret(spec, scale, bounds[mid]).x + mark <= max_w {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return alone;
    }
    let cut = cut_at(spec, &text, &bounds, lo, first);
    if system.measure(&cut, scale).0 <= max_w {
        return cut;
    }
    let mut best = alone;
    let (mut lo, mut hi) = (0, lo);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        let cut = cut_at(spec, &text, &bounds, mid, first);
        if system.measure(&cut, scale).0 <= max_w {
            lo = mid;
            best = cut;
        } else {
            hi = mid;
        }
    }
    best
}

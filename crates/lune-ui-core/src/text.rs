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
}

impl Style {
    pub fn ui(size: f32, color: Rgba) -> Self {
        Self {
            font: Font::Ui,
            size,
            line: (size * 1.3).round(),
            weight: 400,
            color,
        }
    }

    pub fn mono(size: f32, line: f32, color: Rgba) -> Self {
        Self {
            font: Font::Mono,
            size,
            line,
            weight: 400,
            color,
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

    fn hash_into(&self, h: &mut impl Hasher) {
        self.font.hash(h);
        self.size.to_bits().hash(h);
        self.line.to_bits().hash(h);
        self.weight.hash(h);
        self.color.hash(h);
    }
}

/// Text to draw: one or more styled runs; `wrap` is a width in logical px.
#[derive(Debug, Clone)]
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
/// ending is not part of it) and its place, in logical px.
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
/// (empty text, no fonts) answers with finite values.
pub trait TextSystem {
    /// Shapes `spec` at `scale` if needed; returns its key and logical size (width, height).
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32));

    /// The logical size `prepare` would return.
    fn measure(&mut self, spec: &Spec, scale: f32) -> (f32, f32) {
        self.prepare(spec, scale).1
    }

    /// Where the caret sits at `offset`. For one unwrapped line, the caret at the end of the
    /// text has exactly the width `prepare` returns, and the caret at 0 has `x == 0`.
    fn caret(&mut self, spec: &Spec, scale: f32, offset: usize) -> CaretPos;

    /// The offset nearest to `pos`, a point in logical px from the spec's top-left corner:
    /// the nearer side of the character under it, the first or last offset of the line when it is
    /// left or right of the text, the first or last line when it is above or below. A `pos` that
    /// is not finite gives 0.
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

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

/// The text seam: shapes specs and measures them. Sizes are logical pixels; `scale` is device
/// pixels per logical pixel. Preparing the same spec at the same scale gives the same key and
/// size; entries of several scales may coexist.
pub trait TextSystem {
    /// Shapes `spec` at `scale` if needed; returns its key and logical size (width, height).
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32));

    /// The logical size `prepare` would return.
    fn measure(&mut self, spec: &Spec, scale: f32) -> (f32, f32) {
        self.prepare(spec, scale).1
    }
}

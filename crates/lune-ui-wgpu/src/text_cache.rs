//! The glyphon text system: a shaped-buffer cache behind [`TextSystem`].

use std::collections::HashMap;

use glyphon::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, Weight, Wrap};

use lune_ui_core::text::{Font, Spec, Style, TextKey, TextSystem};

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
    /// crate embeds no fonts.
    pub fn new(font_data: &[&[u8]], ui_family: &str, mono_family: &str) -> Self {
        let mut fonts = FontSystem::new();
        let db = fonts.db_mut();
        for data in font_data {
            db.load_font_data(data.to_vec());
        }
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

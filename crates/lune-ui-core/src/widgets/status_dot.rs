//! A small filled dot in a status tone (`widget.status-dot`): 8 px, no glow, ring or pulse. A dot
//! never stands alone: the word beside it carries the meaning, so [`Ui::status_dot`] draws the
//! dot only together with its word and [`paint`] is for widgets that embed a dot (the badge).

use std::hash::Hash;

use super::tone::Tone;
use crate::access::{Node, Role, State, Value};
use crate::paint::{Painter, Rect};
use crate::text::{Spec, Style};
use crate::ui::Ui;

/// The dot's diameter.
pub const SIZE: f32 = 8.0;
/// The gap between the dot and its word.
pub const GAP: f32 = 8.0;

/// Draws the dot centred on `center`. No node: the embedding widget names the whole.
pub fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, center: (f32, f32), tone: Tone) {
    let r = Rect::new(center.0 - SIZE / 2.0, center.1 - SIZE / 2.0, SIZE, SIZE);
    let color = tone.color(&p.theme);
    p.fill(r, SIZE / 2.0, color);
}

impl Ui<'_> {
    /// A dot in `tone` with `word` beside it, top left at `at`: the dot is vertically centred on
    /// the word's line, the word in the base size. Returns the rect of both. The nodes are an
    /// `Image` under `key` marked `Hidden` (the word carries the meaning) and a `StaticText`
    /// under `key` and `"word"`. An empty `word` draws nothing and emits nothing, because a dot
    /// can never stand alone; the returned rect is empty.
    pub fn status_dot(&mut self, key: impl Hash, at: (f32, f32), tone: Tone, word: &str) -> Rect {
        if word.is_empty() {
            return Rect::new(at.0, at.1, 0.0, 0.0);
        }
        let id = self.id(key);
        let theme = *self.theme();
        let style = Style::ui(theme.font_base, theme.fg_primary);
        let line = style.line;
        let dot = Rect::new(at.0, at.1 + (line - SIZE) / 2.0, SIZE, SIZE);
        paint(
            self.painter(),
            (dot.x + SIZE / 2.0, dot.y + SIZE / 2.0),
            tone,
        );
        let (w, h) = self
            .painter()
            .text(&Spec::plain(word, style), at.0 + SIZE + GAP, at.1);

        let mut node = Node::new(id, Role::Image, "", dot);
        node.states.insert(State::Hidden);
        self.node(node);
        let mut text = Node::new(
            id.with("word"),
            Role::StaticText,
            word,
            Rect::new(at.0 + SIZE + GAP, at.1, w, h),
        );
        text.value = Some(Value::Text(word.to_owned()));
        self.node(text);
        Rect::new(at.0, at.1, SIZE + GAP + w, h)
    }
}

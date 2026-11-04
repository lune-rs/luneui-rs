//! A static tag (`widget.badge`): 18 px high, radius 8, a one px tone border, 11 px Medium text
//! on a 16 px line, an optional leading status dot. Colours come from [`Tone`], the one recipe of
//! the status widgets. The word carries the meaning; the colour is not the only channel.

use std::hash::Hash;

use super::status_dot;
use super::tone::Tone;
use crate::access::{Node, Role, Value};
use crate::paint::{Painter, Rect};
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::Ui;

/// The badge's height.
pub const HEIGHT: f32 = 18.0;
/// The corner radius.
pub const RADIUS: f32 = 8.0;
/// The border's width.
pub const BORDER: f32 = 1.0;
/// The padding left and right of the content, inside the border.
pub const PAD: f32 = 6.0;
/// The gap between the leading dot and the text.
pub const GAP: f32 = 6.0;
/// The text's line height.
pub const LINE: f32 = 16.0;

fn style(theme: &Theme, tone: Tone) -> Style {
    Style {
        line: LINE,
        ..Style::ui(theme.font_s, tone.text(theme)).medium()
    }
}

/// The badge's width for `text`, with or without the leading dot.
pub fn width<H: Copy + PartialEq>(p: &mut Painter<'_, H>, text: &str, dot: bool) -> f32 {
    let spec = Spec::plain(text, style(&p.theme, Tone::Neutral));
    let lead = if dot { status_dot::SIZE + GAP } else { 0.0 };
    2.0 * (BORDER + PAD) + lead + p.measure(&spec).0
}

/// Draws the badge with its top left at `at`; returns its rect.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    at: (f32, f32),
    text: &str,
    tone: Tone,
    dot: bool,
) -> Rect {
    let theme = p.theme;
    let r = Rect::new(at.0, at.1, width(p, text, dot), HEIGHT);
    p.fill(r, RADIUS, tone.fill(&theme));
    p.ring(r, RADIUS, BORDER, tone.border(&theme), false);
    let mut x = r.x + BORDER + PAD;
    if dot {
        status_dot::paint(p, (x + status_dot::SIZE / 2.0, r.y + HEIGHT / 2.0), tone);
        x += status_dot::SIZE + GAP;
    }
    p.text(
        &Spec::plain(text, style(&theme, tone)),
        x,
        r.y + (HEIGHT - LINE) / 2.0,
    );
    r
}

impl Ui<'_> {
    /// A badge under `key` with its top left at `at`. Static: it takes no input. The node is a
    /// `StaticText` named and valued by `text`. Returns the badge's rect.
    pub fn badge(
        &mut self,
        key: impl Hash,
        at: (f32, f32),
        text: &str,
        tone: Tone,
        dot: bool,
    ) -> Rect {
        let id = self.id(key);
        let r = paint(self.painter(), at, text, tone, dot);
        let mut node = Node::new(id, Role::StaticText, text, r);
        node.value = Some(Value::Text(text.to_owned()));
        self.node(node);
        r
    }
}

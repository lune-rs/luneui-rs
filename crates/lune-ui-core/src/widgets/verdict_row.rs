//! A check row (`widget.verdict-row`): a mark word (OK, WARN or NO) in its tone, a name and a
//! text. Three columns of 40, 92 and the rest, or stacked for a narrow column: mark and name on
//! the first line, the text below in the small size, wrapping, with a hairline under the row.
//! Static: the tone is on the mark only, and the mark's word, not its colour, carries the meaning.
//! The mark words are the toolkit strings `luneui.verdict-row.{ok,warn,no}`.

use std::hash::Hash;

use super::tone::Tone;
use crate::access::{Node, Role, Value};
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::ui::Ui;

/// The mark column's width.
pub const MARK_W: f32 = 40.0;
/// The name column's width in the three column layout.
pub const NAME_W: f32 = 92.0;
/// The gap kept between the name and what follows it.
pub const NAME_GAP: f32 = 8.0;
/// The shortest a row is.
pub const MIN_HEIGHT: f32 = 24.0;
/// A line of the mark, the name and the wide layout's text.
pub const LINE: f32 = 20.0;
/// Vertical padding of the wide layout (24 = 20 + 2 + 2).
pub const PAD: f32 = 2.0;
/// Vertical padding of the stacked layout.
pub const STACKED_PAD: f32 = 4.0;
/// The stacked text's line.
pub const STACKED_LINE: f32 = 14.0;

/// A verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    Ok,
    Warn,
    No,
}

impl Verdict {
    /// The tone of the mark: `No` is the error tone.
    pub fn tone(self) -> Tone {
        match self {
            Verdict::Ok => Tone::Ok,
            Verdict::Warn => Tone::Warn,
            Verdict::No => Tone::Err,
        }
    }

    /// The string key of the mark word.
    pub fn key(self) -> &'static str {
        match self {
            Verdict::Ok => "luneui.verdict-row.ok",
            Verdict::Warn => "luneui.verdict-row.warn",
            Verdict::No => "luneui.verdict-row.no",
        }
    }
}

impl Ui<'_> {
    /// A verdict row under `key` at the top left and width of `rect` (its height is ignored).
    /// `stacked` picks the narrow layout. Returns the height drawn: at least 24 wide, the text's
    /// lines plus the padding stacked. The nodes are a `Row` named `name` and valued by the mark
    /// word, over three `Cell`s (mark, name, text).
    pub fn verdict_row(
        &mut self,
        key: impl Hash,
        rect: Rect,
        verdict: Verdict,
        name: &str,
        text: &str,
        stacked: bool,
    ) -> f32 {
        let id = self.id(key);
        let theme = *self.theme();
        let word = self.tr(verdict.key());
        let mark = Spec::plain(
            word,
            Style::mono(theme.font_s, LINE, verdict.tone().color(&theme)).medium(),
        );
        let name_style = Style {
            line: LINE,
            ..Style::ui(theme.font_base, theme.fg_tertiary)
        };
        let (name_x, name_max) = (
            rect.x + MARK_W,
            if stacked {
                rect.w - MARK_W
            } else {
                NAME_W - NAME_GAP
            },
        );
        let name_spec = self
            .painter()
            .elide(&Spec::plain(name, name_style), name_max);

        let (text_spec, text_x) = if stacked {
            let style = Style {
                line: STACKED_LINE,
                ..Style::ui(theme.font_s, theme.fg_secondary)
            };
            (Spec::plain(text, style).wrapped(rect.w - MARK_W), name_x)
        } else {
            let style = Style {
                line: LINE,
                ..Style::ui(theme.font_base, theme.fg_primary)
            };
            let x = rect.x + MARK_W + NAME_W;
            (
                Spec::plain(text, style).wrapped((rect.w - MARK_W - NAME_W).max(0.0)),
                x,
            )
        };
        let text_h = if text.is_empty() {
            0.0
        } else {
            self.painter().measure(&text_spec).1
        };
        let height = if stacked {
            STACKED_PAD + LINE + text_h + STACKED_PAD + 1.0
        } else {
            (text_h + 2.0 * PAD).max(MIN_HEIGHT)
        };

        let bounds = Rect::new(rect.x, rect.y, rect.w, height);
        let mut row = Node::new(id, Role::Row, name, bounds);
        row.value = Some(Value::Text(word.to_owned()));
        let top = rect.y + if stacked { STACKED_PAD } else { PAD };
        self.node_container(row, |ui| {
            let p = ui.painter();
            let mark_size = p.text(&mark, rect.x, top);
            let name_size = p.text(&name_spec, name_x, top);
            let text_size = if text.is_empty() {
                (0.0, 0.0)
            } else {
                let y = if stacked { top + LINE } else { top };
                p.text(&text_spec, text_x, y)
            };
            if stacked {
                p.fill(
                    Rect::new(rect.x, rect.y + height - 1.0, rect.w, 1.0),
                    0.0,
                    theme.border_subtle,
                );
            }
            let cells = [
                (
                    "mark",
                    word,
                    Rect::new(rect.x, top, mark_size.0, mark_size.1),
                ),
                (
                    "name",
                    name,
                    Rect::new(name_x, top, name_size.0, name_size.1),
                ),
                (
                    "text",
                    text,
                    Rect::new(
                        text_x,
                        if stacked { top + LINE } else { top },
                        text_size.0,
                        text_size.1,
                    ),
                ),
            ];
            for (k, s, r) in cells {
                let mut cell = Node::new(id.with(k), Role::Cell, s, r);
                cell.value = Some(Value::Text(s.to_owned()));
                ui.node(cell);
            }
        });
        height
    }
}

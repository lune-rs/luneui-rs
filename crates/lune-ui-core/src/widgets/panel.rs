//! The panel (`layout.panel`, D148): an outlined group box with a title and a meta line, a body in
//! one of five kinds and an optional action footer.
//!
//! [`Ui::panel`] draws no fill, only the 1 px `border_subtle` outline at radius 8, and does it last
//! so nothing the body drew covers it. `content` is called for the body and then the footer, in tab
//! order, and returns the body's height.
//!
//! - `fit: true` hugs the body and returns the panel's height; `fit: false` fills the rect with the
//!   footer pinned to its bottom, which is how panels in a row share a bottom edge.
//! - An empty header (no title, no meta) and an empty footer (no note, no footer buttons) take no
//!   height. A kind with no header gets a top padding of its own (see [`PanelKind`]).
//! - `unavailable` replaces the body with one `fg_tertiary` line (node `Status`, polite); the body
//!   slot is not called. The footer stays.
//!
//! Text cannot be cut by a rounded clip (D12), so the toolkit keeps its own text at least the
//! radius from the edges (16 for the header, the note and the unavailable line). The body is
//! clipped to the rounded rect, which cuts fills and images; a flush body's text is the caller's
//! to keep [`TEXT_INSET`] in. With `fit: true` the clip is the rect given, since the final height
//! is known only after the body ran.

use std::hash::Hash;

use crate::access::{Live, Node, Role, Value};
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::ui::Ui;

/// The corner radius.
pub const RADIUS: f32 = 8.0;
/// The least distance a flush body's text keeps from the panel's edge: the radius.
pub const TEXT_INSET: f32 = 8.0;
/// The header's padding: vertical and horizontal. A card's header pads 12 each side.
pub const HEADER_PAD: (f32, f32) = (12.0, 16.0);
/// The title's line (its size is the theme's `font_m`) and the meta's (`font_s`).
pub const TITLE_LINE: f32 = 20.0;
pub const META_LINE: f32 = 16.0;
/// Between the title and the meta.
pub const META_GAP: f32 = 12.0;
/// How far the meta's top sits below the title's, so the two share a baseline.
pub const META_DROP: f32 = 3.0;
/// The footer's padding (vertical, horizontal), its note's line and its buttons' row height.
pub const FOOTER_PAD: (f32, f32) = (12.0, 16.0);
pub const NOTE_LINE: f32 = 16.0;
pub const BUTTON_ROW: f32 = 32.0;
/// The padding of a [`PanelKind::Card`].
pub const CARD_PAD: f32 = 12.0;
/// The unavailable line's height.
pub const UNAVAILABLE_LINE: f32 = 20.0;

/// How the body is padded and what edges it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelKind {
    /// Body padding 0 16 16; 16 above when there is no header.
    #[default]
    Plain,
    /// Rows of a list: padding 0 4 8; 8 above when there is no header.
    Rows,
    /// A table or list edge to edge, with a hairline under the header. No padding.
    Flush,
    /// A row of [`Ui::stat_strip`] cells, whose hairlines divide it. No padding.
    Strip,
    /// A panel inside a panel: padding 12 all round, and the footer's buttons sit 12 from the
    /// bottom and right with no hairline above them.
    Card,
}

impl PanelKind {
    /// The body's padding: left and right, below, and above when there is no header.
    fn pad(self) -> (f32, f32, f32) {
        match self {
            PanelKind::Plain => (16.0, 16.0, 16.0),
            PanelKind::Rows => (4.0, 8.0, 8.0),
            PanelKind::Flush | PanelKind::Strip => (0.0, 0.0, 0.0),
            PanelKind::Card => (CARD_PAD, CARD_PAD, CARD_PAD),
        }
    }
}

/// What [`Ui::panel`] is given besides its rect.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PanelOptions<'a> {
    /// The title (`font_m` on a 20 px line, medium, `fg_primary`).
    pub title: Option<&'a str>,
    /// The meta at the header's right (`font_s` on 16, `fg_tertiary`).
    pub meta: Option<&'a str>,
    pub kind: PanelKind,
    /// Replaces the body with this one line; the body slot is not called.
    pub unavailable: Option<&'a str>,
    /// The footer's note at the left (`font_s` on 16, `fg_tertiary`), elided.
    pub note: Option<&'a str>,
    /// The width of the footer's buttons, right-aligned. 0 draws and calls none.
    pub footer_w: f32,
    /// Hug the body and return the height, instead of filling the rect.
    pub fit: bool,
    /// The group's accessible name; empty falls back to the title.
    pub name: &'a str,
}

/// The slot a [`Ui::panel`] closure is called for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanelSlot {
    /// The body's padded box; the closure returns the content's height. Its height is the room
    /// left in the rect.
    Body(Rect),
    /// The footer's buttons: [`BUTTON_ROW`] high, `footer_w` wide, at the right. Called only when
    /// `footer_w` is above 0.
    Footer(Rect),
}

fn clean(v: f32) -> f32 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

impl Ui<'_> {
    /// A panel under `key` in `rect`. Returns its height: `rect.h` when it fills, the hugged
    /// height with `fit`. The node is a `Group` named by the title (`opts.name` when set)
    /// described by the meta, over the body's nodes, the footer note (`StaticText`) and the
    /// footer's buttons.
    pub fn panel(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &PanelOptions<'_>,
        mut content: impl FnMut(&mut Ui<'_>, PanelSlot) -> f32,
    ) -> f32 {
        let id = self.id(&key);
        let t = *self.theme();
        let title = opts.title.filter(|s| !s.is_empty());
        let meta = opts.meta.filter(|s| !s.is_empty());
        let note = opts.note.filter(|s| !s.is_empty());
        let footer_w = clean(opts.footer_w);
        let card = opts.kind == PanelKind::Card;
        let (pad_x, pad_bottom, pad_top) = opts.kind.pad();

        let header_h = match (title, meta) {
            (None, None) => 0.0,
            _ => {
                let line = if title.is_some() {
                    TITLE_LINE
                } else {
                    META_LINE
                };
                2.0 * HEADER_PAD.0 + line
            }
        };
        let hairline = if opts.kind == PanelKind::Flush && header_h > 0.0 {
            1.0
        } else {
            0.0
        };
        let body_top = if header_h > 0.0 { 0.0 } else { pad_top };
        let footer_h = match (note, footer_w > 0.0) {
            (None, false) => 0.0,
            (_, true) if card => BUTTON_ROW + CARD_PAD,
            (_, true) => 2.0 * FOOTER_PAD.0 + BUTTON_ROW,
            (Some(_), false) => 2.0 * FOOTER_PAD.0 + NOTE_LINE,
        };
        let foot_pad_x = if card { CARD_PAD } else { FOOTER_PAD.1 };
        let head_pad_x = if card { CARD_PAD } else { HEADER_PAD.1 };

        // The body's room, then where the footer goes.
        let room = (rect.h - header_h - hairline - body_top - pad_bottom - footer_h).max(0.0);
        let body = Rect::new(
            rect.x + pad_x,
            rect.y + header_h + hairline + body_top,
            (rect.w - 2.0 * pad_x).max(0.0),
            room,
        );

        let name = if opts.name.is_empty() {
            title.unwrap_or("")
        } else {
            opts.name
        };
        let mut group = Node::new(id, Role::Group, name, rect);
        group.description = meta.map(str::to_owned);

        self.scope(&key, |ui| {
            ui.node_container(group, |ui| {
                // The header.
                if header_h > 0.0 {
                    let p = ui.painter();
                    let meta_spec = meta.map(|m| {
                        p.elide(
                            &Spec::plain(
                                m,
                                Style {
                                    line: META_LINE,
                                    ..Style::ui(t.font_s, t.fg_tertiary)
                                },
                            ),
                            (rect.w - 2.0 * head_pad_x).max(0.0),
                        )
                    });
                    let meta_w = meta_spec.as_ref().map_or(0.0, |s| p.measure(s).0);
                    if let Some(title) = title {
                        let gap = if meta_w > 0.0 { META_GAP } else { 0.0 };
                        let room = (rect.w - 2.0 * head_pad_x - meta_w - gap).max(0.0);
                        let style = Style {
                            line: TITLE_LINE,
                            ..Style::ui(t.font_m, t.fg_primary).medium()
                        };
                        let spec = p.elide(&Spec::plain(title, style), room);
                        p.text(&spec, rect.x + head_pad_x, rect.y + HEADER_PAD.0);
                    }
                    if let Some(spec) = meta_spec {
                        let drop = if title.is_some() { META_DROP } else { 0.0 };
                        p.text(
                            &spec,
                            rect.right() - head_pad_x - meta_w,
                            rect.y + HEADER_PAD.0 + drop,
                        );
                    }
                    if hairline > 0.0 {
                        p.fill(
                            Rect::new(
                                rect.x + 1.0,
                                rect.y + header_h,
                                (rect.w - 2.0).max(0.0),
                                1.0,
                            ),
                            0.0,
                            t.border_subtle,
                        );
                    }
                }

                // The body, cut by the rounded rect.
                ui.painter().push_clip_rounded(rect, RADIUS);
                let content_h = if let Some(text) = opts.unavailable {
                    let x = rect.x + pad_x.max(16.0);
                    let w = (rect.w - 2.0 * pad_x.max(16.0)).max(0.0);
                    let spec = ui.painter().elide(
                        &Spec::plain(
                            text,
                            Style {
                                line: UNAVAILABLE_LINE,
                                ..Style::ui(t.font_base, t.fg_tertiary)
                            },
                        ),
                        w,
                    );
                    let size = ui.painter().text(&spec, x, body.y);
                    let at = Rect::new(x, body.y, size.0, UNAVAILABLE_LINE);
                    let mut status = Node::new(ui.id("unavailable"), Role::Status, text, at);
                    status.value = Some(Value::Text(text.to_owned()));
                    status.live = Some(Live::Polite);
                    ui.node(status);
                    UNAVAILABLE_LINE
                } else {
                    clean(content(ui, PanelSlot::Body(body)))
                };
                ui.painter().pop_clip();

                let height = if opts.fit {
                    header_h + hairline + body_top + content_h + pad_bottom + footer_h
                } else {
                    rect.h
                };

                // The footer, pinned to the bottom of the panel.
                if footer_h > 0.0 {
                    let top = rect.y + height - footer_h;
                    let p = ui.painter();
                    if !card {
                        p.fill(
                            Rect::new(rect.x + 1.0, top, (rect.w - 2.0).max(0.0), 1.0),
                            0.0,
                            t.border_subtle,
                        );
                    }
                    let row_y = if card { top } else { top + FOOTER_PAD.0 };
                    if let Some(note) = note {
                        let gap = if footer_w > 0.0 { META_GAP } else { 0.0 };
                        let w = (rect.w - 2.0 * foot_pad_x - footer_w - gap).max(0.0);
                        let row_h = if footer_w > 0.0 {
                            BUTTON_ROW
                        } else {
                            NOTE_LINE
                        };
                        let spec = p.elide(
                            &Spec::plain(
                                note,
                                Style {
                                    line: NOTE_LINE,
                                    ..Style::ui(t.font_s, t.fg_tertiary)
                                },
                            ),
                            w,
                        );
                        let x = rect.x + foot_pad_x;
                        let y = row_y + (row_h - NOTE_LINE) / 2.0;
                        let size = p.text(&spec, x, y);
                        let mut node = Node::new(
                            ui.id("note"),
                            Role::StaticText,
                            note,
                            Rect::new(x, y, size.0, NOTE_LINE),
                        );
                        node.value = Some(Value::Text(note.to_owned()));
                        ui.node(node);
                    }
                    if footer_w > 0.0 {
                        let at = Rect::new(
                            rect.right() - foot_pad_x - footer_w,
                            row_y,
                            footer_w,
                            BUTTON_ROW,
                        );
                        content(ui, PanelSlot::Footer(at));
                    }
                }

                // The outline last, so a flush body's fill cannot cover it.
                ui.painter().ring(
                    Rect::new(rect.x, rect.y, rect.w, height),
                    RADIUS,
                    1.0,
                    t.border_subtle,
                    true,
                );
                let bounds = ui
                    .painter()
                    .transformed(Rect::new(rect.x, rect.y, rect.w, height));
                ui.patch_node(id, |n| n.bounds = bounds);
                height
            })
        })
    }
}

//! A numbered sequence of steps with a done state (`widget.steps`).
//!
//! Rows 32 high with radius 6, 2 px apart, padding 0 8, gap 8: a 20 px circle, then a label in a
//! 148 px column that never wraps (a long one is elided). Upcoming: a 1 px `border` ring and a
//! tertiary number and label. Current: `surface_selected` fill, a bright number and a bright
//! Medium label. Done: an `accent` circle with a 12 px check in `on_accent` and a secondary label.
//! Steps are not clickable and take no focus: the owner's Back and Continue move through them.
//! An optional lead line sits 12 px below the last row.

use std::hash::Hash;

use crate::access::{Node, Role, State, Value};
use crate::icon;
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::ui::Ui;

/// A row's height and the gap between rows.
pub const ROW: f32 = 32.0;
pub const ROW_GAP: f32 = 2.0;
/// The row's corner radius, its side padding and the gap inside it.
pub const RADIUS: f32 = 6.0;
pub const PAD_X: f32 = 8.0;
pub const GAP: f32 = 8.0;
/// The number circle's diameter.
pub const CIRCLE: f32 = 20.0;
/// The check's size.
pub const CHECK: f32 = 12.0;
/// The label column's width.
pub const LABEL_W: f32 = 148.0;
/// The label's and the lead's line.
pub const LINE: f32 = 20.0;
/// Between the last row and the lead line.
pub const LEAD_GAP: f32 = 12.0;

impl Ui<'_> {
    /// Steps under `key` with their top left and width of `rect` (its height is ignored): one row
    /// per label, the one at `current` selected, every one before it done. Returns the height
    /// drawn. See [`Ui::steps_with_lead`].
    pub fn steps(&mut self, key: impl Hash, rect: Rect, labels: &[&str], current: usize) -> f32 {
        self.steps_with_lead(key, rect, labels, current, None)
    }

    /// [`Ui::steps`] with a lead line below the rows (13 on a 20 px line, tertiary, wrapping).
    /// The node is a `Navigation` over a `List` of one `ListItem` per step named by its full
    /// label: `Selected` and `Current` on the current one, `Checked` on a done one.
    pub fn steps_with_lead(
        &mut self,
        key: impl Hash,
        rect: Rect,
        labels: &[&str],
        current: usize,
        lead: Option<&str>,
    ) -> f32 {
        let t = *self.theme();
        let id = self.id(&key);
        let n = labels.len();
        let rows_h = if n == 0 {
            0.0
        } else {
            n as f32 * ROW + (n - 1) as f32 * ROW_GAP
        };
        let lead_spec = lead.map(|l| {
            Spec::plain(
                l,
                Style {
                    line: LINE,
                    ..Style::ui(t.font_base, t.fg_tertiary)
                },
            )
            .wrapped(rect.w)
        });
        let lead_h = lead_spec
            .as_ref()
            .map_or(0.0, |s| LEAD_GAP + self.painter().measure(s).1.max(LINE));
        let height = rows_h + lead_h;
        let label_w = LABEL_W.min((rect.w - 2.0 * PAD_X - CIRCLE - GAP).max(0.0));

        self.scope(&key, |ui| {
            let nav = Node::new(
                id,
                Role::Navigation,
                "",
                Rect::new(rect.x, rect.y, rect.w, rows_h),
            );
            ui.node_container(nav, |ui| {
                let list = Node::new(
                    id.with("list"),
                    Role::List,
                    "",
                    Rect::new(rect.x, rect.y, rect.w, rows_h),
                );
                ui.node_container(list, |ui| {
                    for (i, label) in labels.iter().enumerate() {
                        let row =
                            Rect::new(rect.x, rect.y + i as f32 * (ROW + ROW_GAP), rect.w, ROW);
                        let done = i < current;
                        let is_current = i == current;
                        ui.scope(i, |ui| {
                            let item_id = ui.id("item");
                            draw_row(ui, row, label, i, done, is_current, label_w);
                            let mut node = Node::new(item_id, Role::ListItem, label, row);
                            if is_current {
                                node.states.insert(State::Selected);
                                node.states.insert(State::Current);
                            }
                            if done {
                                node.states.insert(State::Checked);
                            }
                            ui.node(node);
                        });
                    }
                });
            });
            if let Some(spec) = &lead_spec {
                let y = rect.y + rows_h + LEAD_GAP;
                let (_, h) = ui.painter().text(spec, rect.x, y);
                let mut node = Node::new(
                    id.with("lead"),
                    Role::StaticText,
                    lead.unwrap_or(""),
                    Rect::new(rect.x, y, rect.w, h),
                );
                node.value = Some(Value::Text(lead.unwrap_or("").to_owned()));
                ui.node(node);
            }
        });
        height
    }
}

fn draw_row(
    ui: &mut Ui<'_>,
    row: Rect,
    label: &str,
    index: usize,
    done: bool,
    current: bool,
    label_w: f32,
) {
    let t = *ui.theme();
    let p = ui.painter();
    if current {
        p.fill(row, RADIUS, t.surface_selected);
    }
    let circle = Rect::new(row.x + PAD_X, row.y + (ROW - CIRCLE) / 2.0, CIRCLE, CIRCLE);
    if done {
        p.fill(circle, CIRCLE / 2.0, t.accent);
        p.text_centered(&icon::glyph(icon::CHECK, CHECK, t.on_accent), circle);
    } else {
        let ring = if current { t.fg_primary } else { t.border };
        p.ring(circle, CIRCLE / 2.0, 1.0, ring, false);
        let color = if current { t.fg_bright } else { t.fg_tertiary };
        let number = Spec::plain(
            (index + 1).to_string(),
            Style {
                line: 16.0,
                ..Style::ui(t.font_s, color).medium()
            },
        );
        p.text_centered(&number, circle);
    }
    let style = if current {
        Style::ui(t.font_base, t.fg_bright).medium()
    } else if done {
        Style::ui(t.font_base, t.fg_secondary)
    } else {
        Style::ui(t.font_base, t.fg_tertiary)
    };
    let spec = p.elide(
        &Spec::plain(
            label,
            Style {
                line: LINE,
                ..style
            },
        ),
        label_w,
    );
    p.text(&spec, circle.right() + GAP, row.y + (ROW - LINE) / 2.0);
}

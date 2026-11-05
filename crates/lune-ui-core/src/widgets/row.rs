//! A hover-lit row with a leading, a body and a trailing part (`widget.row`).
//!
//! Two layouts. The setting row: radius 6, padding 12, at least 56 high, a title (13 on a 20 px
//! line) over an optional description (11 on a 16 px line, `fg_muted`, wrapping) with the control
//! at the right behind a 16 px gap. The item row: 32 high, padding 0 12, a grid of 40 / 1fr / 88 /
//! 176 with 8 px gaps whose cells the caller fills. A row is not clickable and registers no hit
//! region: its hover reads [`Ui::pointer_in`], so the control inside keeps every press and the row
//! stays lit while the pointer is on the control.

use std::hash::Hash;

use crate::access::{Node, Role, State};
use crate::color::Rgba;
use crate::motion::{Kind, Motion};
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::ui::{Ui, WidgetId};

/// Background fade between rest and hover.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Title colour between on and off.
pub const SELECTION: Motion = Motion::of(Kind::Selection);

/// The corner radius of both rows.
pub const RADIUS: f32 = 6.0;
/// The setting row's padding on every side.
pub const PAD: f32 = 12.0;
/// The setting row's least height.
pub const MIN_HEIGHT: f32 = 56.0;
/// Between the text and the control.
pub const CONTROL_GAP: f32 = 16.0;
/// The title's line and the description's line.
pub const TITLE_LINE: f32 = 20.0;
pub const DESC_LINE: f32 = 16.0;
/// Between the title and the description.
pub const DESC_GAP: f32 = 2.0;
/// The item row's height, side padding and gap between cells.
pub const ITEM_HEIGHT: f32 = 32.0;
pub const ITEM_PAD_X: f32 = 12.0;
pub const ITEM_GAP: f32 = 8.0;
/// The item row's fixed cells: the first, the third and the fourth (the second is `1fr`).
pub const ITEM_COLS: [f32; 3] = [40.0, 88.0, 176.0];

/// What a setting row shows. The `off` flag is the app's: it dims the title.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SettingRow<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub off: bool,
    /// The width of the control's slot at the right; 0 when the row has no control.
    pub control_w: f32,
}

/// What an item row is told: its accessible name (the cells are the app's).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ItemRow<'a> {
    pub name: &'a str,
}

/// `c` with alpha 0, so a fade keeps its hue.
fn clear(c: Rgba) -> Rgba {
    [c[0], c[1], c[2], 0]
}

impl Ui<'_> {
    /// Draws the hover fill of a row at `rect` under the current scope.
    fn row_hover_fill(&mut self, rect: Rect) {
        let t = *self.theme();
        let hovered = self.is_enabled() && self.pointer_in(rect);
        let target = if hovered {
            t.surface_hover
        } else {
            clear(t.surface_hover)
        };
        let id = self.id("hover");
        let fill = self.animate_color(id, target, &HOVER);
        if fill[3] > 0 {
            self.painter().fill(rect, RADIUS, fill);
        }
    }

    /// A setting row under `key` with its top left and width of `rect` (its height is ignored): the
    /// title and description at the left, `control` at the right in a slot of `row.control_w` that
    /// spans the row's height (the control centres itself). Returns the height drawn. The node is
    /// a `Group` named by the title, described by the description, `Disabled` inside
    /// `with_enabled(false, ..)`; the control's node is its child.
    pub fn setting_row(
        &mut self,
        key: impl Hash,
        rect: Rect,
        row: &SettingRow<'_>,
        control: impl FnOnce(&mut Ui<'_>, Rect),
    ) -> f32 {
        let t = *self.theme();
        let id = self.id(&key);
        let text_w = {
            let side = if row.control_w > 0.0 {
                row.control_w + CONTROL_GAP
            } else {
                0.0
            };
            (rect.w - 2.0 * PAD - side).max(0.0)
        };
        let title_color = if row.off {
            t.fg_secondary
        } else {
            t.fg_primary
        };
        let title_style = Style {
            line: TITLE_LINE,
            ..Style::ui(t.font_base, title_color)
        };
        let title = self
            .painter()
            .elide(&Spec::plain(row.title, title_style), text_w);
        let desc = row.description.map(|d| {
            Spec::plain(
                d,
                Style {
                    line: DESC_LINE,
                    ..Style::ui(t.font_s, t.fg_muted)
                },
            )
            .wrapped(text_w)
        });
        let desc_h = desc.as_ref().map_or(0.0, |d| {
            let h = self.painter().measure(d).1.max(DESC_LINE);
            DESC_GAP + h
        });
        let block_h = TITLE_LINE + desc_h;
        let height = (block_h + 2.0 * PAD).max(MIN_HEIGHT);
        let bounds = Rect::new(rect.x, rect.y, rect.w, height);
        let enabled = self.is_enabled();

        self.scope(&key, |ui| {
            ui.row_hover_fill(bounds);
            let title_id = ui.id("title");
            let color = ui.animate_color(title_id, title_color, &SELECTION);
            let mut title = title;
            for (_, style) in &mut title.runs {
                style.color = color;
            }
            let top = bounds.y + (height - block_h) / 2.0;
            let x = bounds.x + PAD;
            ui.painter().text(&title, x, top);
            if let Some(d) = &desc {
                ui.painter().text(d, x, top + TITLE_LINE + DESC_GAP);
            }

            let mut node = Node::new(id, Role::Group, row.title, bounds);
            node.description = row.description.map(str::to_owned);
            if !enabled {
                node.states.insert(State::Disabled);
            }
            ui.node_container(node, |ui| {
                let slot = Rect::new(
                    bounds.right() - PAD - row.control_w,
                    bounds.y,
                    row.control_w,
                    height,
                );
                control(ui, slot);
            });
        });
        height
    }

    /// An item row under `key` with its top left and width of `rect`: the four cells of the grid
    /// (40 / `1fr` / 88 / 176, 8 px apart, inside 12 px of side padding, each as high as the row)
    /// go to `cols`. Returns the height, 32. The node is a `Group` named by `row.name`.
    pub fn item_row(
        &mut self,
        key: impl Hash,
        rect: Rect,
        row: &ItemRow<'_>,
        cols: impl FnOnce(&mut Ui<'_>, [Rect; 4]),
    ) -> f32 {
        let id: WidgetId = self.id(&key);
        let bounds = Rect::new(rect.x, rect.y, rect.w, ITEM_HEIGHT);
        let enabled = self.is_enabled();
        let [a, c, d] = ITEM_COLS;
        let inner = (rect.w - 2.0 * ITEM_PAD_X).max(0.0);
        let flex = (inner - a - c - d - 3.0 * ITEM_GAP).max(0.0);
        let x0 = rect.x + ITEM_PAD_X;
        let x1 = x0 + a + ITEM_GAP;
        let x2 = x1 + flex + ITEM_GAP;
        let x3 = x2 + c + ITEM_GAP;
        let cells = [
            Rect::new(x0, rect.y, a, ITEM_HEIGHT),
            Rect::new(x1, rect.y, flex, ITEM_HEIGHT),
            Rect::new(x2, rect.y, c, ITEM_HEIGHT),
            Rect::new(x3, rect.y, d, ITEM_HEIGHT),
        ];
        self.scope(&key, |ui| {
            ui.row_hover_fill(bounds);
            let mut node = Node::new(id, Role::Group, row.name, bounds);
            if !enabled {
                node.states.insert(State::Disabled);
            }
            ui.node_container(node, |ui| cols(ui, cells));
        });
        ITEM_HEIGHT
    }
}

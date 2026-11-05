//! The sidebar (`layout.sidebar`, D149): a column on the window glass holding navigation rows or
//! wizard steps, and a status footer pinned to its bottom.
//!
//! [`Ui::sidebar`] draws no card and no fill: it sits on the layer the app drew. The caller's rect
//! sets the column (its width should be [`WIDTH`], 208, and its height is where the footer lands:
//! the footer's bottom is [`MARGIN_BOTTOM`] above the rect's bottom). The body is either
//! [`Ui::nav_list`] (one roving stop; keys as `widget.nav-item`) or [`Ui::steps`] (not interactive),
//! inset [`NAV_PAD`]. The footer is static text with a status dot over a hairline and takes no
//! input.
//!
//! The nodes: a `Group` named `name` over the nav list, or the steps' own `Navigation` named
//! `name`, then the footer as a polite `Status` after them.

use std::hash::Hash;

use super::nav_item::{self, NavEntry};
use super::status_dot;
use super::tone::Tone;
use crate::access::{Live, Node, Role, Value};
use crate::paint::Rect;
use crate::text::{Spec, Style};
use crate::ui::Ui;

/// The column's width (`--sidebar-w`).
pub const WIDTH: f32 = 208.0;
/// The padding around the body.
pub const NAV_PAD: f32 = 8.0;
/// The footer's padding: left and right, above and below its line.
pub const FOOTER_PAD_X: f32 = 16.0;
pub const FOOTER_PAD_Y: f32 = 12.0;
/// The footer text's line (the size is the theme's `font_s`).
pub const FOOTER_LINE: f32 = 16.0;
/// The footer's height: padding, line, padding.
pub const FOOTER_H: f32 = 2.0 * FOOTER_PAD_Y + FOOTER_LINE;
/// Between the footer's bottom and the rect's bottom (`--gutter`).
pub const MARGIN_BOTTOM: f32 = 10.0;

/// What fills the column above the footer.
#[derive(Debug)]
pub enum SidebarBody<'a> {
    /// Navigation rows; `selected` holds the current screen's key and takes a navigation.
    Nav {
        entries: &'a [NavEntry<'a>],
        selected: &'a mut u64,
    },
    /// The wizard's steps: one row per label, the one at `current` selected, every one before it
    /// done. No input.
    Steps {
        labels: &'a [&'a str],
        current: usize,
    },
}

/// The footer line: a dot in `tone` and `text`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidebarFooter<'a> {
    pub tone: Tone,
    pub text: &'a str,
    /// The text is in the mono face (addresses, paths).
    pub mono: bool,
}

/// What [`Ui::sidebar`] draws.
#[derive(Debug)]
pub struct Sidebar<'a> {
    pub body: SidebarBody<'a>,
    pub footer: Option<SidebarFooter<'a>>,
    /// The accessible name of the group (nav) or of the navigation (steps).
    pub name: &'a str,
}

/// The rect the footer occupies in a sidebar at `rect`.
pub fn footer_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x,
        rect.bottom() - MARGIN_BOTTOM - FOOTER_H,
        rect.w,
        FOOTER_H,
    )
}

impl Ui<'_> {
    /// A sidebar under `key` in `rect`. Returns the key of the nav row that was activated this
    /// frame (a click, Enter or Space, also when it is already current); `None` for the steps
    /// body and when nothing was.
    pub fn sidebar(&mut self, key: impl Hash, rect: Rect, sidebar: Sidebar<'_>) -> Option<u64> {
        let id = self.id(&key);
        let Sidebar { body, footer, name } = sidebar;
        let inner = Rect::new(
            rect.x + NAV_PAD,
            rect.y + NAV_PAD,
            (rect.w - 2.0 * NAV_PAD).max(0.0),
            0.0,
        );
        self.scope(&key, |ui| {
            let activated = match body {
                SidebarBody::Nav { entries, selected } => {
                    let h = match entries.len() {
                        0 => 0.0,
                        n => (n - 1) as f32 * nav_item::PITCH + nav_item::ROW_HEIGHT,
                    };
                    let group = Node::new(id, Role::Group, name, Rect { h, ..inner });
                    ui.node_container(group, |ui| {
                        ui.nav_list("nav", inner, entries, selected, "").activated
                    })
                }
                SidebarBody::Steps { labels, current } => {
                    ui.steps("steps", inner, labels, current);
                    let nav = ui.id("steps");
                    ui.patch_node(nav, |n| n.name = name.to_owned());
                    None
                }
            };
            if let Some(f) = footer {
                draw_footer(ui, id.with("footer"), footer_rect(rect), &f);
            }
            activated
        })
    }
}

/// Draws the footer in `at` and emits its `Status` node.
fn draw_footer(ui: &mut Ui<'_>, id: crate::ui::WidgetId, at: Rect, f: &SidebarFooter<'_>) {
    let t = *ui.theme();
    let p = ui.painter();
    p.fill(Rect::new(at.x, at.y, at.w, 1.0), 0.0, t.border_subtle);
    let line_mid = at.y + FOOTER_PAD_Y + FOOTER_LINE / 2.0;
    status_dot::paint(
        p,
        (at.x + FOOTER_PAD_X + status_dot::SIZE / 2.0, line_mid),
        f.tone,
    );
    let x = at.x + FOOTER_PAD_X + status_dot::SIZE + status_dot::GAP;
    let style = if f.mono {
        Style::mono(t.font_s, FOOTER_LINE, t.fg_tertiary)
    } else {
        Style {
            line: FOOTER_LINE,
            ..Style::ui(t.font_s, t.fg_tertiary)
        }
    };
    let spec = p.elide(
        &Spec::plain(f.text, style),
        (at.right() - FOOTER_PAD_X - x).max(0.0),
    );
    p.text(&spec, x, at.y + FOOTER_PAD_Y);

    let mut node = Node::new(id, Role::Status, f.text, at);
    node.live = Some(Live::Polite);
    node.value = Some(Value::Text(f.text.to_owned()));
    ui.node(node);
}

//! The icon rail (`layout.icon-rail`, D149): a narrow column of 40 px icon buttons that switch the
//! side panels, a group at the top and a group docked at the bottom, with an uncommitted-changes
//! dot and a count badge.
//!
//! [`Ui::icon_rail`] draws only the buttons: the glass behind the rail is the app's. The rail is
//! one tab stop and one roving group (`core.focus`'s composite rule; the entry cites the
//! source's stop per button): Up and Down move focus through the top group then the bottom, wrapping at the ends,
//! skipping a disabled item; Home and End go to the ends. Tab enters it once. Enter, Space and a
//! click activate.
//!
//! Each button is the ghost icon button of the toolbar. An item the app marks `active` (the panel
//! that is open) takes the `surface_selected` fill and the `fg_primary` glyph and reads `Pressed`
//! to assistive technology: the source has no active look and a rail needs one to say which panel is
//! open (D149). The dot and the badge are never animated; they scale with the button's press so
//! they stay on it.
//!
//! Keys are unique across both groups. The top group runs from the rect's top, the bottom group
//! ends at the rect's bottom, its items listed top to bottom.

use std::hash::Hash;

use super::button::{self, IconButton, PRESS_SCALE};
use super::nav_item::SELECTION;
use super::toolbar::{ACTION_GLYPH, ACTION_SIZE};
use crate::access::{Node, Role, State};
use crate::paint::{Rect, Xform};
use crate::text::{Spec, Style};
use crate::ui::{RoveAxis, Sense, Ui, WidgetId};

/// The rail's padding: left and right, above the top group and below the bottom one.
pub const PAD_X: f32 = 4.0;
pub const PAD_Y: f32 = 10.0;
/// The gap between two buttons of the top group (the `StackPanel`'s spacing). The bottom group
/// has none: its buttons are docked children of a `DockPanel` with no spacing
/// (`IconRailView.axaml:22-24,83`).
pub const GAP: f32 = 4.0;
/// A button's side (`toolbar::ACTION_SIZE`) and the distance between two tops of the top group
/// and of the bottom group.
pub const SIZE: f32 = ACTION_SIZE;
pub const PITCH: f32 = ACTION_SIZE + GAP;
pub const BOTTOM_PITCH: f32 = ACTION_SIZE;
/// The dot: 8 by 8, its top left from the button's.
pub const DOT: f32 = 8.0;
pub const DOT_X: f32 = 27.0;
pub const DOT_Y: f32 = 7.0;
/// The badge: at least this wide, this high, the right edge this far from the button's left and
/// the top this far from the button's top; its text is 9 px, padded [`BADGE_PAD`] each side.
pub const BADGE_MIN_W: f32 = 16.0;
pub const BADGE_H: f32 = 16.0;
pub const BADGE_RIGHT: f32 = 34.0;
pub const BADGE_TOP: f32 = 10.0;
pub const BADGE_PAD: f32 = 2.0;
pub const BADGE_FONT: f32 = 9.0;
/// The badge shows this many and "99+" above it.
pub const BADGE_MAX: u32 = 99;

/// One button of the rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailItem<'a> {
    /// The value [`Ui::icon_rail`] returns when it is activated; unique across the rail.
    pub key: u64,
    /// A codicon glyph (`icon::*`).
    pub glyph: &'a str,
    /// The accessible name and the tooltip.
    pub name: &'a str,
    /// This item's panel is open.
    pub active: bool,
    /// A disabled item is skipped by the arrows, drawn at 0.4 and inert.
    pub enabled: bool,
    /// The dot (`dirty_indicator`) is shown.
    pub dot: bool,
    /// The count badge; `None` and `Some(0)` show none.
    pub badge: Option<u32>,
    /// The description of the dot and the badge ("3 unread"), so the count has a name.
    pub note: Option<&'a str>,
}

/// The height of the top group of `n` buttons, `GAP` apart.
pub fn group_height(n: usize) -> f32 {
    match n {
        0 => 0.0,
        n => n as f32 * SIZE + (n - 1) as f32 * GAP,
    }
}

/// The height of the bottom group of `n` buttons, touching.
pub fn bottom_height(n: usize) -> f32 {
    n as f32 * BOTTOM_PITCH
}

/// The text a badge of `count` reads.
pub fn badge_text(count: u32) -> String {
    if count > BADGE_MAX {
        format!("{BADGE_MAX}+")
    } else {
        count.to_string()
    }
}

impl Ui<'_> {
    /// A rail under `key` in `rect`: `top` from the top, `bottom` docked to the bottom. Returns the
    /// key of the item activated this frame. The node is a `Toolbar` named `name`; each item is a
    /// `Button` named by its `name`, `Pressed` when `active`, described by its `note`.
    pub fn icon_rail(
        &mut self,
        key: impl Hash,
        rect: Rect,
        top: &[RailItem<'_>],
        bottom: &[RailItem<'_>],
        name: &str,
    ) -> Option<u64> {
        let id = self.id(&key);
        let group_on = self.is_enabled();
        let all: Vec<&RailItem<'_>> = top.iter().chain(bottom).collect();
        let items: Vec<(WidgetId, bool)> = all
            .iter()
            .map(|i| (id.with(i.key), i.enabled && group_on))
            .collect();
        let stop = self.rove_stop(&items, all.iter().position(|i| i.active));
        let x = rect.x + PAD_X;
        let top_y = rect.y + PAD_Y;
        let bottom_y = rect.bottom() - PAD_Y - bottom_height(bottom.len());

        let mut activated = None;
        let node = Node::new(id, Role::Toolbar, name, rect);
        self.scope(&key, |ui| {
            ui.node_container(node, |ui| {
                for (i, item) in all.iter().enumerate() {
                    let y = if i < top.len() {
                        top_y + i as f32 * PITCH
                    } else {
                        bottom_y + (i - top.len()) as f32 * BOTTOM_PITCH
                    };
                    let sense = if stop == Some(i) {
                        Sense::CLICK_FOCUS
                    } else {
                        Sense::CLICK_ROVE
                    };
                    let r = rail_button(ui, Rect::new(x, y, SIZE, SIZE), item, sense);
                    if r.enabled && r.clicked && activated.is_none() {
                        activated = Some(item.key);
                    }
                }
            });
        });
        self.rove_keys(&items, RoveAxis::Vertical, true);
        activated
    }
}

/// One button with its fill, dot, badge and node.
fn rail_button(
    ui: &mut Ui<'_>,
    rect: Rect,
    item: &RailItem<'_>,
    sense: Sense,
) -> crate::ui::Response {
    let t = *ui.theme();
    let kid = ui.id(item.key);
    let r = ui.with_enabled(item.enabled, |ui| {
        let [sr, sg, sb, _] = t.surface_selected;
        let fill = ui.animate_color(
            kid.with("sel"),
            if item.active {
                t.surface_selected
            } else {
                [sr, sg, sb, 0]
            },
            &SELECTION,
        );
        let glyph = ui.animate_color(
            kid.with("glyph"),
            if item.active {
                t.fg_primary
            } else {
                t.fg_muted
            },
            &SELECTION,
        );
        if fill[3] > 0 {
            ui.painter().fill(rect, t.item_radius, fill);
        }
        let button = IconButton {
            glyph: item.glyph,
            glyph_size: ACTION_GLYPH,
            color: Some(glyph),
            radius: t.item_radius,
            name: item.name,
        };
        let r = ui.icon_button_sensed(item.key, rect, &button, sense);
        // The dot and the badge follow the button's press scale (the same tween, read again).
        let target = if r.pressed { PRESS_SCALE } else { 1.0 };
        let scale = ui.animate_f32(kid.with("press"), target, &button::PRESS);
        let badge = item.badge.filter(|n| *n > 0);
        if item.dot || badge.is_some() {
            let p = ui.painter();
            let scaled = scale != 1.0;
            if scaled {
                p.push_xform(Xform {
                    origin: (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
                    scale,
                    dx: 0.0,
                    dy: 0.0,
                    alpha: 1.0,
                });
            }
            if item.dot {
                let at = Rect::new(rect.x + DOT_X, rect.y + DOT_Y, DOT, DOT);
                p.fill(at, DOT / 2.0, t.dirty_indicator);
            }
            if let Some(n) = badge {
                let spec = Spec::plain(badge_text(n), Style::ui(BADGE_FONT, t.on_accent).bold());
                let w = (p.measure(&spec).0 + 2.0 * BADGE_PAD).max(BADGE_MIN_W);
                let at = Rect::new(rect.x + BADGE_RIGHT - w, rect.y + BADGE_TOP, w, BADGE_H);
                p.fill(at, BADGE_H / 2.0, t.count_badge);
                p.text_centered(&spec, at);
            }
            if scaled {
                p.pop_xform();
            }
        }
        r
    });
    ui.tooltip(&r, item.name);
    ui.patch_node(kid, |n| {
        if item.active {
            n.states.insert(State::Pressed);
        }
        n.description = item.note.map(str::to_owned);
    });
    r
}

//! Luneium's glass context menu (`ContextMenu.css`, `Popup.css`) and its motion.

use std::hash::Hash;

use crate::access::{Action, Node, Role, State};
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Easing, Kind, Motion, Reduce, Token};
use crate::paint::{LayerId, Painter, Rect, Xform};
use crate::text::{Spec, Style};
use crate::ui::{OverlayOptions, OverlayPhase, Sense, Ui, WidgetId};

/// One row of a menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuEntry<'s, H> {
    Item {
        glyph: Option<&'s str>,
        text: &'s str,
        shortcut: &'s str,
        hit: H,
    },
    Separator,
}

/// Luneium's menu motion (`Popup.css`): in 0.18 s ease-out from 4 px lower at 97 %; out 0.12 s
/// ease-in to 2 px lower at 98 %. Both about the top-left corner.
pub const MENU_IN: f32 = 0.18;
pub const MENU_OUT: f32 = 0.12;

/// Menu opacity, vertical offset and scale at one moment of its animation.
pub type MenuAnim = (f32, f32, f32);

/// CSS `cubic-bezier(x1, y1, x2, y2)` at time `t`; [`motion::cubic_bezier`](crate::motion::cubic_bezier).
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    crate::motion::cubic_bezier(x1, y1, x2, y2, t)
}

/// Opening motion at `t` = elapsed / [`MENU_IN`].
pub fn menu_open_anim(t: f32) -> MenuAnim {
    let e = cubic_bezier(0.215, 0.61, 0.355, 1.0, t);
    (e, 4.0 * (1.0 - e), 0.97 + 0.03 * e)
}

/// Closing motion at `t` = elapsed / [`MENU_OUT`]; `None` once it has finished (`t` ≥ 1).
pub fn menu_close_anim(t: f32) -> Option<MenuAnim> {
    if t >= 1.0 {
        return None;
    }
    let e = cubic_bezier(0.42, 0.0, 1.0, 1.0, t);
    Some((1.0 - e, 2.0 * e, 1.0 - 0.02 * e))
}

/// One row of a menu as the layout and the paint see it (no hit value).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuRow<'s> {
    Item {
        glyph: Option<&'s str>,
        text: &'s str,
        shortcut: &'s str,
    },
    Separator,
}

impl<'s, H> From<&MenuEntry<'s, H>> for MenuRow<'s> {
    fn from(e: &MenuEntry<'s, H>) -> Self {
        match e {
            MenuEntry::Item {
                glyph,
                text,
                shortcut,
                ..
            } => MenuRow::Item {
                glyph: *glyph,
                text,
                shortcut,
            },
            MenuEntry::Separator => MenuRow::Separator,
        }
    }
}

const PAD: f32 = 6.0;
const ROW: f32 = 28.0;
const SEP: f32 = 11.0;
const RADIUS: f32 = 9.0;

/// Where a menu's panel and rows are.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuLayout {
    pub panel: Rect,
    /// One rect per row, in order: an item's 28 px hover and hit rect, or a separator's 11 px strip.
    pub rows: Vec<Rect>,
}

/// Sizes a menu of `rows` and places it at `at`, kept inside the window (flipped above `at` near
/// the bottom edge).
pub fn menu_layout<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rows: &[MenuRow<'_>],
    at: (f32, f32),
) -> MenuLayout {
    let t = p.theme;
    let label_style = Style::ui(12.0, t.menu_text_primary);
    let hint_style = Style::ui(t.font_s, t.menu_text_quaternary);
    let mut w: f32 = 190.0;
    let mut h = 2.0 * PAD;
    for e in rows {
        match e {
            MenuRow::Item { text, shortcut, .. } => {
                let tw = p.measure(&Spec::plain(*text, label_style)).0;
                let sw = if shortcut.is_empty() {
                    0.0
                } else {
                    16.0 + p.measure(&Spec::plain(*shortcut, hint_style)).0
                };
                w = w.max(2.0 * PAD + 8.0 + 14.0 + 8.0 + tw + sw + 8.0);
                h += ROW;
            }
            MenuRow::Separator => h += SEP,
        }
    }
    let x = at.0.min(p.window.w - w - 8.0).max(8.0);
    let y = if at.1 + h > p.window.h - 8.0 {
        (at.1 - h).max(8.0)
    } else {
        at.1
    };
    let panel = Rect::new(x, y, w, h);
    let mut cy = panel.y + PAD;
    let rects = rows
        .iter()
        .map(|e| {
            let (height, r) = match e {
                MenuRow::Item { .. } => {
                    (ROW, Rect::new(panel.x + PAD, cy, panel.w - 2.0 * PAD, ROW))
                }
                MenuRow::Separator => (SEP, Rect::new(panel.x + PAD, cy, panel.w - 2.0 * PAD, SEP)),
            };
            cy += height;
            r
        })
        .collect();
    MenuLayout { panel, rows: rects }
}

/// Draws the glass panel and its rows, untransformed and without hit regions. `hover[i]` fills
/// item `i` (a missing entry is false).
pub fn menu_paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    rows: &[MenuRow<'_>],
    layout: &MenuLayout,
    hover: &[bool],
) {
    let t = p.theme;
    let label_style = Style::ui(12.0, t.menu_text_primary);
    let hint_style = Style::ui(t.font_s, t.menu_text_quaternary);
    let panel = layout.panel;
    p.backdrop(panel, RADIUS);
    p.fill(panel, RADIUS, t.menu_glass);
    p.ring(panel, RADIUS, 1.0, t.menu_popup_border, false);
    for (i, (e, r)) in rows.iter().zip(&layout.rows).enumerate() {
        match e {
            MenuRow::Item {
                glyph,
                text,
                shortcut,
            } => {
                let r = *r;
                if hover.get(i).copied().unwrap_or(false) {
                    p.fill(r, 4.0, t.menu_item_hover);
                }
                let icon_slot = Rect::new(r.x + 8.0, r.y, 14.0, r.h);
                if let Some(g) = glyph {
                    p.text_centered(&icon::glyph(g, 13.0, t.menu_text_tertiary), icon_slot);
                }
                p.text_in(
                    &Spec::plain(*text, label_style),
                    Rect::new(icon_slot.right() + 8.0, r.y, 0.0, r.h),
                    0.0,
                );
                if !shortcut.is_empty() {
                    let spec = Spec::plain(*shortcut, hint_style);
                    let sw = p.measure(&spec).0;
                    p.text_in(&spec, Rect::new(r.right() - 8.0 - sw, r.y, sw, r.h), 0.0);
                }
            }
            MenuRow::Separator => {
                p.fill(
                    Rect::new(
                        panel.x + PAD + 2.0,
                        r.y + 5.0,
                        panel.w - 2.0 * PAD - 4.0,
                        1.0,
                    ),
                    0.0,
                    t.menu_separator,
                );
            }
        }
    }
}

/// A glass menu at `at` (kept inside the window), the content under it blurred (30 px,
/// saturate 1.8), black 45 % over grey 165 @ 30 %, radius 9, inset 1 px ring, 6 px padding,
/// 28 px rows with radius 4, 12 px labels, 14 px icon slot, 11 px hints. `interactive` is off
/// while the menu fades out.
pub fn menu<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    entries: &[MenuEntry<'_, H>],
    at: (f32, f32),
    anim: MenuAnim,
    interactive: bool,
) {
    let rows: Vec<MenuRow<'_>> = entries.iter().map(MenuRow::from).collect();
    let layout = menu_layout(p, &rows, at);
    let hover: Vec<bool> = entries
        .iter()
        .map(|e| match e {
            MenuEntry::Item { hit, .. } => interactive && p.hovered(*hit),
            MenuEntry::Separator => false,
        })
        .collect();
    let (alpha, dy, scale) = anim;
    p.set_xform(Some(Xform {
        origin: (layout.panel.x, layout.panel.y),
        scale,
        dx: 0.0,
        dy,
        alpha,
    }));
    menu_paint(p, &rows, &layout, &hover);
    if interactive {
        for (e, r) in entries.iter().zip(&layout.rows) {
            if let MenuEntry::Item { hit, .. } = e {
                p.hit(*r, *hit);
            }
        }
    }
    p.set_xform(None);
}

/// Open: pop, fast, cubic-out; Reduced fades opacity only, still at Fast (menu.md's override).
pub const OPEN: Motion = Motion {
    kind: Kind::Pop,
    token: Token::Fast,
    easing: Easing::CUBIC_OUT,
    reduced: Reduce::Fade,
    reduced_token: Some(Token::Fast),
};
/// Close: dismiss, instant (the moved 120 ms), ease-in; Reduced fades opacity only.
pub const CLOSE: Motion = Motion {
    kind: Kind::Dismiss,
    token: Token::Instant,
    easing: Easing::EASE_IN,
    reduced: Reduce::Fade,
    reduced_token: Some(Token::Instant),
};

/// What a menu frame reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuResponse {
    /// The index in `rows` of the item picked this frame.
    pub picked: Option<usize>,
    /// The Ui asks the app to close the menu (Escape, an outside press, an accessibility dismiss).
    pub dismissed: bool,
}

/// An owned [`MenuRow`], for the exit painter.
#[derive(Debug, Clone)]
enum OwnedRow {
    Item {
        glyph: Option<String>,
        text: String,
        shortcut: String,
    },
    Separator,
}

impl OwnedRow {
    fn of(r: &MenuRow<'_>) -> OwnedRow {
        match r {
            MenuRow::Item {
                glyph,
                text,
                shortcut,
            } => OwnedRow::Item {
                glyph: glyph.map(str::to_owned),
                text: (*text).to_owned(),
                shortcut: (*shortcut).to_owned(),
            },
            MenuRow::Separator => OwnedRow::Separator,
        }
    }

    fn borrow(&self) -> MenuRow<'_> {
        match self {
            OwnedRow::Item {
                glyph,
                text,
                shortcut,
            } => MenuRow::Item {
                glyph: glyph.as_deref(),
                text,
                shortcut,
            },
            OwnedRow::Separator => MenuRow::Separator,
        }
    }
}

/// The transform of a menu at one moment: the moved curves, or opacity only when `transforms`
/// is off (the Reduced fade).
fn xform_of(anim: MenuAnim, origin: (f32, f32), transforms: bool) -> Xform {
    let (alpha, dy, scale) = anim;
    if transforms {
        Xform {
            origin,
            scale,
            dx: 0.0,
            dy,
            alpha,
        }
    } else {
        Xform {
            origin,
            scale: 1.0,
            dx: 0.0,
            dy: 0.0,
            alpha,
        }
    }
}

impl<'a> Ui<'a> {
    /// A glass menu of `rows` at `at` on the popup layer, non-modal: Escape and a press outside
    /// dismiss it (`dismissed`), a click on an item, or Enter with the pointer over one, picks it
    /// (`picked`). Call it every frame it is open; stop calling to close it, and it fades out
    /// through its exit painter. The look follows D7: opening at progress `p` is
    /// [`menu_open_anim`]`(p)`, open is `menu_open_anim(1.0)`, closing at `p` is
    /// [`menu_close_anim`]`(p)`.
    pub fn menu(&mut self, key: impl Hash, rows: &[MenuRow<'_>], at: (f32, f32)) -> MenuResponse {
        let layout = menu_layout(self.painter(), rows, at);
        let options = OverlayOptions {
            layer: LayerId::Popup,
            modal: false,
            dismiss_on_escape: true,
            dismiss_on_outside_press: true,
            dismiss_on_focus_loss: false,
            anchor: None,
            initial_focus: None,
            open: OPEN,
            close: CLOSE,
        };
        let owned: Vec<OwnedRow> = rows.iter().map(OwnedRow::of).collect();
        let exit_layout = layout.clone();
        let exit = move |p: &mut Painter<'_, WidgetId>, progress: f32, transforms: bool| {
            let Some(anim) = menu_close_anim(progress) else {
                return;
            };
            let rows: Vec<MenuRow<'_>> = owned.iter().map(OwnedRow::borrow).collect();
            let origin = (exit_layout.panel.x, exit_layout.panel.y);
            p.push_xform(xform_of(anim, origin, transforms));
            menu_paint(p, &rows, &exit_layout, &[]);
            p.pop_xform();
        };
        let (frame, picked) = self.overlay(&key, &options, exit, |ui, frame| {
            let origin = (layout.panel.x, layout.panel.y);
            let anim = match frame.phase {
                OverlayPhase::Opening(p) => menu_open_anim(p),
                OverlayPhase::Open => menu_open_anim(1.0),
            };
            let menu_id = ui.id(&key);
            let mut menu_node = Node::new(menu_id, Role::Menu, "", layout.panel);
            menu_node.states.insert(State::Expanded);
            let (picked, hover) = ui.node_container(menu_node, |ui| {
                let mut responses = Vec::with_capacity(rows.len());
                ui.painter()
                    .push_xform(xform_of(anim, origin, frame.transforms));
                // The panel blocks what is under it, so a press on a separator or the padding is
                // not an outside press.
                ui.interact(menu_id.with("panel"), layout.panel, Sense::HOVER);
                for (i, (row, r)) in rows.iter().zip(&layout.rows).enumerate() {
                    let MenuRow::Item { text, shortcut, .. } = row else {
                        responses.push(None);
                        continue;
                    };
                    let id = menu_id.with(i);
                    let resp = ui.interact(id, *r, Sense::CLICK_FOCUS);
                    let mut node = Node::new(id, Role::MenuItem, text, *r);
                    if !shortcut.is_empty() {
                        node.description = Some((*shortcut).to_owned());
                    }
                    node.states.insert(State::Focusable);
                    node.actions.insert(Action::Default);
                    ui.node(node);
                    responses.push(Some(resp));
                }
                // Enter picks the item under the pointer, else the focused one.
                let mut enter_on = None;
                let mut hovered = None;
                let mut clicked = None;
                for (i, resp) in responses.iter().enumerate() {
                    let Some(resp) = resp else { continue };
                    if resp.hovered {
                        hovered = Some(i);
                    }
                    if resp.clicked {
                        clicked = Some(i);
                    }
                    if ui.consume_key(resp.id, NamedKey::Enter) {
                        enter_on = Some(i);
                    }
                }
                let entered = enter_on.and_then(|focused| hovered.or(Some(focused)));
                let picked = clicked.or(entered);
                let hover: Vec<bool> = (0..rows.len()).map(|i| hovered == Some(i)).collect();
                (picked, hover)
            });
            let p = ui.painter();
            menu_paint(p, rows, &layout, &hover);
            p.pop_xform();
            picked
        });
        MenuResponse {
            picked,
            dismissed: frame.dismissed.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_bezier_endpoints() {
        for (x1, y1, x2, y2) in [(0.215, 0.61, 0.355, 1.0), (0.42, 0.0, 1.0, 1.0)] {
            assert!(cubic_bezier(x1, y1, x2, y2, -1.0).abs() < 1e-5);
            assert!(cubic_bezier(x1, y1, x2, y2, 0.0).abs() < 1e-5);
            assert!((cubic_bezier(x1, y1, x2, y2, 1.0) - 1.0).abs() < 1e-5);
            assert!((cubic_bezier(x1, y1, x2, y2, 2.0) - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn open_curve_ends_at_rest() {
        let (alpha, dy, scale) = menu_open_anim(1.0);
        assert!((alpha - 1.0).abs() < 1e-5);
        assert!(dy.abs() < 1e-4);
        assert!((scale - 1.0).abs() < 1e-5);
    }

    #[test]
    fn close_curve_ends_at_none() {
        assert!(menu_close_anim(1.0).is_none());
        assert!(menu_close_anim(3.0).is_none());
        let (alpha, ..) = menu_close_anim(0.0).expect("closing at t=0");
        assert!((alpha - 1.0).abs() < 1e-5);
    }
}

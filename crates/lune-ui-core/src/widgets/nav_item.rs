//! A navigation item and the list that owns the group (`widget.nav-item`, D121). A 32 px row with
//! a 16 px icon and a label: secondary text at rest, the hover fill on hover, the selected fill
//! and bright text for the current screen, no accent bar.
//!
//! [`Ui::nav_list`] owns the group: one roving stop, Up and Down move focus without wrapping,
//! Enter, Space and a click navigate (once each). A Control, Alt or Command chord on Enter or Space
//! stays a shortcut. Focus alone navigates nothing.

use std::hash::Hash;

use super::focus_ring::{self, RingKind};
use crate::access::{Action, Node, Role, State};
use crate::color::Rgba;
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Kind, Motion};
use crate::paint::{Painter, Rect};
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::{Response, RoveAxis, Sense, Ui, WidgetId};

use super::radio_group::group_response;

/// The fill and label between inactive and active.
pub const SELECTION: Motion = Motion::of(Kind::Selection);
/// The fill on hover and press.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Focus ring opacity.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// A row's height (`--nav-h`).
pub const ROW_HEIGHT: f32 = 32.0;
/// The distance between the tops of two rows: the height and a 2 px gap.
pub const PITCH: f32 = 34.0;
/// The row's corner radius.
pub const RADIUS: f32 = 6.0;
/// Horizontal padding.
pub const PAD: f32 = 8.0;
/// Space between the icon and the label.
pub const GAP: f32 = 8.0;
/// The icon's size.
pub const ICON_SIZE: f32 = 16.0;

/// One item of a [`Ui::nav_list`]. `key` is the value `selected` holds when it is the current
/// screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavEntry<'a> {
    pub key: u64,
    pub label: &'a str,
    /// A codicon glyph (`icon::*`).
    pub icon: Option<&'a str>,
    pub enabled: bool,
}

/// What [`Ui::nav_list`] reports.
#[derive(Debug, Clone, PartialEq)]
pub struct NavResponse {
    /// The group: `changed` when the current screen changed this frame.
    pub response: Response,
    /// The key an item navigated to this frame (a click, Enter or Space), also when it is already
    /// the current one.
    pub activated: Option<u64>,
}

/// What the pointer and keyboard are doing to an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NavState {
    /// The current screen.
    pub active: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub focus_visible: bool,
}

/// The continuous look of an item. A colour with alpha 0 draws nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavLook {
    pub fill: Rgba,
    pub label: Rgba,
    /// The label's weight is 500 (the current screen).
    pub medium: bool,
    /// Focus ring opacity.
    pub focus: f32,
}

/// `a` to `b` by `t` (0 to 1), channel by channel; exactly `a` at 0 and `b` at 1.
fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let ch = |i: usize| {
        let (x, y) = (f32::from(a[i]), f32::from(b[i]));
        (x + (y - x) * t).round().clamp(0.0, 255.0) as u8
    };
    [ch(0), ch(1), ch(2), ch(3)]
}

impl NavLook {
    /// The look at the continuous levels `active`, `hover` and `press` (each 0 to 1): the four
    /// settled fills (rest, hover, active, active and hover) mixed bilinearly, so each settled
    /// look is exact and the pressed fill is `surface_active` for either.
    pub fn mixed(theme: &Theme, active: f32, hover: f32, press: f32, focus: f32) -> NavLook {
        let [r, g, b, _] = theme.surface_hover;
        let clear = [r, g, b, 0];
        let inactive = mix(
            mix(clear, theme.surface_hover, hover),
            theme.surface_active,
            press,
        );
        let current = mix(
            mix(theme.surface_selected, theme.surface_active, hover),
            theme.surface_active,
            press,
        );
        let rest = mix(theme.fg_secondary, theme.fg_primary, hover);
        NavLook {
            fill: mix(inactive, current, active),
            label: mix(rest, theme.fg_bright, active),
            medium: active >= 0.5,
            focus,
        }
    }

    /// The look `state` settles to.
    pub fn of(theme: &Theme, state: NavState) -> NavLook {
        let level = |on: bool| if on { 1.0 } else { 0.0 };
        NavLook::mixed(
            theme,
            level(state.active),
            level(state.hovered),
            level(state.pressed),
            level(state.focus_visible),
        )
    }
}

/// The label spec of an item with `look`.
fn label_spec(theme: &Theme, text: &str, look: &NavLook) -> Spec {
    let style = Style::ui(theme.font_base, look.label);
    Spec::plain(text, if look.medium { style.medium() } else { style })
}

/// Draws one item in `r` with `look`; the label is cut with an ellipsis to fit.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    glyph: Option<&str>,
    look: &NavLook,
) {
    let t = p.theme;
    if look.fill[3] > 0 {
        p.fill(r, RADIUS, look.fill);
    }
    let mut x = r.x + PAD;
    if let Some(g) = glyph {
        let spec = icon::glyph(g, ICON_SIZE, look.label);
        p.text_centered(&spec, Rect::new(x, r.y, ICON_SIZE, r.h));
        x += ICON_SIZE + GAP;
    }
    let spec = p.elide(&label_spec(&t, text, look), (r.right() - PAD - x).max(0.0));
    p.text_in(&spec, Rect::new(x, r.y, r.right() - x, r.h), 0.0);
    focus_ring::paint(p, r, RingKind::Offset { radius: RADIUS }, look.focus);
}

impl Ui<'_> {
    /// A navigation list under `key`: one row per entry, stacked from the top left of `rect` with
    /// its width (its height is ignored), `selected` being the current screen's key. A click,
    /// Enter or Space on the focused row navigates: `selected` takes its key and the response says
    /// so. `name` is the list's accessible name.
    pub fn nav_list(
        &mut self,
        key: impl Hash,
        rect: Rect,
        entries: &[NavEntry<'_>],
        selected: &mut u64,
        name: &str,
    ) -> NavResponse {
        let t = *self.theme();
        let id = self.id(key);
        let group_on = self.is_enabled();
        let items: Vec<(WidgetId, bool)> = entries
            .iter()
            .map(|e| (id.with(e.key), e.enabled && group_on))
            .collect();
        let rows: Vec<Rect> = (0..entries.len())
            .map(|i| Rect::new(rect.x, rect.y + i as f32 * PITCH, rect.w, ROW_HEIGHT))
            .collect();
        let active = entries.iter().position(|e| e.key == *selected);
        let stop = self.rove_stop(&items, active);

        let mut activated = None;
        let mut kids = Vec::with_capacity(entries.len());
        for (i, e) in entries.iter().enumerate() {
            let sense = if stop == Some(i) {
                Sense::CLICK_FOCUS
            } else {
                Sense::CLICK_ROVE
            };
            let kid = items[i].0;
            let r = self.with_enabled(e.enabled, |ui| {
                let r = ui.interact(kid, rows[i], sense);
                let enter = ui.consume_plain_key(kid, NamedKey::Enter);
                let space = ui.consume_plain_key(kid, NamedKey::Space);
                if r.enabled && (r.clicked || enter || space) {
                    activated = Some(e.key);
                }
                r
            });
            kids.push(r);
        }
        self.rove_keys(&items, RoveAxis::Vertical, false);
        let height = match entries.len() {
            0 => 0.0,
            n => (n - 1) as f32 * PITCH + ROW_HEIGHT,
        };
        let mut group = group_response(
            id,
            Rect::new(rect.x, rect.y, rect.w, height),
            group_on,
            &kids,
        );
        if let Some(k) = activated
            && k != *selected
        {
            *selected = k;
            group.mark_changed();
        }

        let mut node = Node::new(id, Role::List, name, group.rect);
        if !group_on {
            node.states.insert(State::Disabled);
        }
        self.node_container(node, |ui| {
            for (i, e) in entries.iter().enumerate() {
                let kid = items[i].0;
                let on = e.key == *selected;
                let level = |v: bool| if v { 1.0 } else { 0.0 };
                ui.with_enabled(e.enabled, |ui| {
                    let sel = ui.animate_f32(kid.with("sel"), level(on), &SELECTION);
                    let hov = ui.animate_f32(kid.with("hov"), level(kids[i].hovered), &HOVER);
                    let press = ui.animate_f32(kid.with("press"), level(kids[i].pressed), &HOVER);
                    let focus =
                        ui.animate_f32(kid.with("focus"), level(kids[i].focus_visible), &FOCUS);
                    let look = NavLook::mixed(&t, sel, hov, press, focus);
                    paint(ui.painter(), rows[i], e.label, e.icon, &look);
                });
                let mut item = Node::new(kid, Role::ListItem, e.label, rows[i]);
                if on {
                    item.states.insert(State::Selected);
                    item.states.insert(State::Current);
                }
                if items[i].1 {
                    item.states.insert(State::Focusable);
                    item.actions.insert(Action::Default);
                } else {
                    item.states.insert(State::Disabled);
                }
                ui.node(item);
            }
        });
        NavResponse {
            response: group,
            activated,
        }
    }
}

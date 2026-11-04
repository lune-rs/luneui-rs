//! Checkbox control with label. Built on the same four layers as the button (see `button.rs`): a
//! [`CheckboxLook`] and its settled values, [`paint`], the Painter-API [`checkbox`] and
//! [`Ui::checkbox`].

use std::hash::Hash;

use super::focus_ring::{self, RingKind};
use super::label;
use crate::access::{Action, Node, Role, State};
use crate::color::{Rgba, with_alpha};
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Kind, Motion};
use crate::paint::{Painter, Rect};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui};

/// Fill, check mark and outline colour between checked and unchecked.
pub const SELECTION: Motion = Motion::of(Kind::Selection);
/// Box fill on hover while unchecked.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Focus ring opacity.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// What the pointer and keyboard are doing to a checkbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CheckboxState {
    pub checked: bool,
    pub hovered: bool,
    pub focus_visible: bool,
}

/// The continuous look of a checkbox. A colour with alpha 0 draws nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckboxLook {
    /// Box fill: `accent` when checked.
    pub fill: Rgba,
    /// Box fill on hover while unchecked.
    pub hover_fill: Rgba,
    /// The outline while unchecked.
    pub ring: Rgba,
    /// The check mark.
    pub check: Rgba,
    /// Focus ring opacity.
    pub focus: f32,
}

impl CheckboxLook {
    /// The look `state` settles to.
    pub fn of(theme: &Theme, state: CheckboxState) -> CheckboxLook {
        let focus = if state.focus_visible { 1.0 } else { 0.0 };
        if state.checked {
            CheckboxLook {
                fill: theme.accent,
                hover_fill: with_alpha(theme.surface_hover, 0.0),
                ring: with_alpha(theme.fg_secondary, 0.0),
                check: theme.on_accent,
                focus,
            }
        } else {
            CheckboxLook {
                fill: with_alpha(theme.accent, 0.0),
                hover_fill: if state.hovered {
                    theme.surface_hover
                } else {
                    with_alpha(theme.surface_hover, 0.0)
                },
                ring: theme.fg_secondary,
                check: with_alpha(theme.on_accent, 0.0),
                focus,
            }
        }
    }
}

/// The 16 by 16 box, vertically centred in `r` at its left edge.
pub fn box_rect(r: Rect) -> Rect {
    Rect::new(r.x, r.y + (r.h - 16.0) / 2.0, 16.0, 16.0)
}

/// The hit area: the box and the label (`label_w` wide) after it, the full rect height.
pub fn hit_rect(r: Rect, label_w: f32) -> Rect {
    Rect::new(r.x, r.y, 24.0 + label_w, r.h)
}

/// Draws a checkbox with `text` in `r` with `look`; returns the label's width.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    look: &CheckboxLook,
) -> f32 {
    let t = p.theme;
    let b = box_rect(r);
    if look.hover_fill[3] > 0 {
        p.fill(b, 3.0, look.hover_fill);
    }
    if look.fill[3] > 0 {
        p.fill(b, 3.0, look.fill);
    }
    if look.ring[3] > 0 {
        p.ring(b, 3.0, 1.0, look.ring, false);
    }
    if look.check[3] > 0 {
        p.text_centered(&icon::glyph(icon::CHECK, 12.0, look.check), b);
    }
    focus_ring::paint(p, b, RingKind::Offset { radius: 3.0 }, look.focus);
    let (w, _) = p.text_in(
        &label(&t, text, t.fg_secondary),
        Rect::new(b.right() + 8.0, r.y, r.w, r.h),
        0.0,
    );
    w
}

/// The Painter-API checkbox: the discrete look of `checked` (unhovered), then its hit region.
pub fn checkbox<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    checked: bool,
    text: &str,
    hit: H,
) {
    let state = CheckboxState {
        checked,
        ..CheckboxState::default()
    };
    let look = CheckboxLook::of(&p.theme, state);
    let w = paint(p, r, text, &look);
    p.hit(hit_rect(r, w), hit);
}

impl Ui<'_> {
    /// A checkbox under `key` in `rect`, flipping `checked` on a click, Space while focused or an
    /// accessibility default action (and marking the response `changed`). `text` is its label and
    /// its accessible name.
    pub fn checkbox(
        &mut self,
        key: impl Hash,
        rect: Rect,
        checked: &mut bool,
        text: &str,
    ) -> Response {
        let t = *self.theme();
        let id = self.id(key);
        let w = self.painter().measure(&label(&t, text, t.fg_secondary)).0;
        let hit = hit_rect(rect, w);
        let mut r = self.interact(id, hit, Sense::CLICK_FOCUS);
        if r.enabled && (r.clicked || self.consume_key(id, NamedKey::Space)) {
            *checked = !*checked;
            r.mark_changed();
        }

        let state = CheckboxState {
            checked: *checked,
            hovered: r.hovered,
            focus_visible: r.focus_visible,
        };
        let target = CheckboxLook::of(&t, state);
        let look = CheckboxLook {
            fill: self.animate_color(id.with("fill"), target.fill, &SELECTION),
            hover_fill: self.animate_color(id.with("hover"), target.hover_fill, &HOVER),
            ring: self.animate_color(id.with("ring"), target.ring, &SELECTION),
            check: self.animate_color(id.with("check"), target.check, &SELECTION),
            focus: self.animate_f32(id.with("focus"), target.focus, &FOCUS),
        };
        paint(self.painter(), rect, text, &look);

        let mut node = Node::new(id, Role::Checkbox, text, hit);
        if *checked {
            node.states.insert(State::Checked);
        }
        if r.enabled {
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Default);
        } else {
            node.states.insert(State::Disabled);
        }
        self.node(node);
        r
    }
}

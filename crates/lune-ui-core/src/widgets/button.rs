//! Button control and its label width.
//!
//! The pattern every widget follows (D2):
//! 1. [`ButtonLook`], plain data for the continuous look of one frame, and [`ButtonLook::of`], the
//!    look a discrete [`ButtonState`] settles to;
//! 2. [`paint`], which draws a look (specimen sheets call it directly);
//! 3. [`button`], the Painter-API function: the discrete look, `paint`, then the hit region;
//! 4. [`Ui::button`], which interacts, animates each field of the look toward its settled value
//!    through the rows of [`HOVER`], [`PRESS`] and [`FOCUS`], paints and emits the semantic node.
//!
//! At rest the animated look equals `ButtonLook::of`, so the `Ui` draws exactly what `button` does.

use std::hash::Hash;

use super::focus_ring::{self, RingKind};
use super::label;
use crate::access::{Action, Node, Role, State};
use crate::color::Rgba;
use crate::input::NamedKey;
use crate::motion::{Kind, Motion};
use crate::paint::{Painter, Rect, Xform};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui};

/// The press scale about the rect's centre.
pub const PRESS_SCALE: f32 = 0.98;

/// Fill colour change on hover and press.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Press scale.
pub const PRESS: Motion = Motion::of(Kind::Press);
/// Focus ring opacity.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// Which button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
}

impl ButtonKind {
    fn ring(self) -> RingKind {
        match self {
            ButtonKind::Primary => RingKind::PrimaryButton,
            ButtonKind::Secondary => RingKind::Button,
        }
    }
}

/// What the pointer and keyboard are doing to a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ButtonState {
    pub hovered: bool,
    pub pressed: bool,
    pub focus_visible: bool,
}

/// The continuous look of a button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonLook {
    pub fill: Rgba,
    pub label: Rgba,
    /// Scale about the rect's centre; 1.0 pushes no transform.
    pub scale: f32,
    /// Focus ring opacity; 0.0 draws no ring.
    pub ring: f32,
}

impl ButtonLook {
    /// The look `state` settles to.
    pub fn of(theme: &Theme, kind: ButtonKind, state: ButtonState) -> ButtonLook {
        let fill = match (kind, state.pressed, state.hovered) {
            (ButtonKind::Primary, false, false) => theme.accent,
            (ButtonKind::Primary, _, _) => theme.accent_hover,
            (ButtonKind::Secondary, true, _) => theme.surface_active,
            (ButtonKind::Secondary, false, true) => theme.surface_selected,
            (ButtonKind::Secondary, false, false) => theme.surface_hover,
        };
        ButtonLook {
            fill,
            label: match kind {
                ButtonKind::Primary => theme.on_accent_text,
                ButtonKind::Secondary => theme.fg_primary,
            },
            scale: if state.pressed { PRESS_SCALE } else { 1.0 },
            ring: if state.focus_visible { 1.0 } else { 0.0 },
        }
    }
}

/// Draws a button in `r` with `look`.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    kind: ButtonKind,
    look: &ButtonLook,
) {
    let t = p.theme;
    let scaled = look.scale != 1.0;
    if scaled {
        p.push_xform(Xform {
            origin: (r.x + r.w / 2.0, r.y + r.h / 2.0),
            scale: look.scale,
            dx: 0.0,
            dy: 0.0,
            alpha: 1.0,
        });
    }
    p.fill(r, t.item_radius, look.fill);
    p.text_centered(&label(&t, text, look.label), r);
    focus_ring::paint(p, r, kind.ring(), look.ring);
    if scaled {
        p.pop_xform();
    }
}

pub fn button<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    primary: bool,
    hit: H,
) {
    let kind = if primary {
        ButtonKind::Primary
    } else {
        ButtonKind::Secondary
    };
    let state = ButtonState {
        hovered: p.hovered(hit),
        ..ButtonState::default()
    };
    let look = ButtonLook::of(&p.theme, kind, state);
    paint(p, r, text, kind, &look);
    p.hit(r, hit);
}

pub fn button_width<H: Copy + PartialEq>(p: &mut Painter<'_, H>, text: &str) -> f32 {
    let t = p.theme;
    p.measure(&label(&t, text, t.fg_primary)).0 + 32.0
}

impl Ui<'_> {
    /// A button under `key` in `rect`. `clicked` on a pointer click, Enter or Space while focused,
    /// or an accessibility default action. Disabled (see [`Ui::with_enabled`]) it reports nothing.
    pub fn button(&mut self, key: impl Hash, rect: Rect, text: &str, kind: ButtonKind) -> Response {
        let id = self.id(key);
        let mut r = self.interact(id, rect, Sense::CLICK_FOCUS);
        if r.enabled
            && (self.consume_key(id, NamedKey::Enter) || self.consume_key(id, NamedKey::Space))
        {
            r.clicked = true;
        }

        let state = ButtonState {
            hovered: r.hovered,
            pressed: r.pressed,
            focus_visible: r.focus_visible,
        };
        let target = ButtonLook::of(self.theme(), kind, state);
        let look = ButtonLook {
            fill: self.animate_color(id.with("fill"), target.fill, &HOVER),
            label: target.label,
            scale: self.animate_f32(id.with("press"), target.scale, &PRESS),
            ring: self.animate_f32(id.with("ring"), target.ring, &FOCUS),
        };
        paint(self.painter(), rect, text, kind, &look);

        let mut node = Node::new(id, Role::Button, text, rect);
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

//! Fluent toggle switch. Built on the same four layers as the button (see `button.rs`): a
//! [`ToggleLook`] and its settled values, [`paint`], the Painter-API [`toggle`] and [`Ui::toggle`].

use std::hash::Hash;

use super::focus_ring::{self, RingKind};
use crate::access::{Action, Node, Role, State};
use crate::color::{Rgba, clear};
use crate::input::NamedKey;
use crate::motion::{Kind, Motion, Reduce};
use crate::paint::{Painter, Rect};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui};

/// Knob x between the ends.
pub const MOVE: Motion = Motion::of(Kind::Move);
/// Track fill, ring and knob colour between on and off.
pub const SELECTION: Motion = Motion::of(Kind::Selection);
/// Hover track fill.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Knob size 12 to 14 on hover: the fill keeps its tween under Reduced, the size snaps.
pub const HOVER_SIZE: Motion = Motion {
    reduced: Reduce::Snap,
    ..Motion::of(Kind::Hover)
};
/// Focus ring opacity.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// What the switch and the input are doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToggleState {
    pub on: bool,
    pub hovered: bool,
    pub focus_visible: bool,
}

/// The continuous look of a toggle. A colour with alpha 0 draws nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToggleLook {
    /// Track fill while off and hovered.
    pub hover_fill: Rgba,
    /// Track fill while on.
    pub on_fill: Rgba,
    /// The off outline.
    pub ring: Rgba,
    pub knob: Rgba,
    pub knob_size: f32,
    /// Knob position: 0 off end, 1 on end.
    pub knob_pos: f32,
    /// Focus ring opacity.
    pub focus: f32,
}

impl ToggleLook {
    /// The look `state` settles to.
    pub fn of(theme: &Theme, state: ToggleState) -> ToggleLook {
        let knob_size = if state.hovered { 14.0 } else { 12.0 };
        let focus = if state.focus_visible { 1.0 } else { 0.0 };
        if state.on {
            ToggleLook {
                hover_fill: clear(theme.surface_hover),
                on_fill: theme.accent,
                ring: clear(theme.fg_secondary),
                knob: theme.on_accent,
                knob_size,
                knob_pos: 1.0,
                focus,
            }
        } else {
            ToggleLook {
                hover_fill: if state.hovered {
                    theme.surface_hover
                } else {
                    clear(theme.surface_hover)
                },
                on_fill: clear(theme.accent),
                ring: theme.fg_secondary,
                knob: theme.fg_primary,
                knob_size,
                knob_pos: 0.0,
                focus,
            }
        }
    }
}

/// The 40 by 20 track, vertically centred in `r`.
pub fn track(r: Rect) -> Rect {
    Rect::new(r.x, r.y + (r.h - 20.0) / 2.0, 40.0, 20.0)
}

/// The hit area: the track grown 4 px left and right and 6 px up and down.
pub fn hit_rect(r: Rect) -> Rect {
    track(r).inset(-4.0, -6.0)
}

/// Draws a toggle in `r` with `look`.
pub fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, r: Rect, look: &ToggleLook) {
    let track = track(r);
    if look.hover_fill[3] > 0 {
        p.fill(track, 10.0, look.hover_fill);
    }
    if look.on_fill[3] > 0 {
        p.fill(track, 10.0, look.on_fill);
    }
    if look.ring[3] > 0 {
        p.ring(track, 10.0, 1.0, look.ring, false);
    }
    let knob = look.knob_size;
    let off_x = track.x + 4.0;
    let on_x = track.right() - 4.0 - knob;
    // The ends are computed exactly as the settled looks always were.
    let x = if look.knob_pos <= 0.0 {
        off_x
    } else if look.knob_pos >= 1.0 {
        on_x
    } else {
        off_x + (on_x - off_x) * look.knob_pos
    };
    let k = Rect::new(x, track.y + (20.0 - knob) / 2.0, knob, knob);
    p.fill(k, knob / 2.0, look.knob);
    focus_ring::paint(p, track, RingKind::Offset { radius: 10.0 }, look.focus);
}

/// Fluent ToggleSwitch: 40×20 track, accent fill when on.
pub fn toggle<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    on: bool,
    hovered: bool,
    hit: H,
) {
    let state = ToggleState {
        on,
        hovered,
        focus_visible: false,
    };
    let look = ToggleLook::of(&p.theme, state);
    paint(p, r, &look);
    p.hit(hit_rect(r), hit);
}

impl Ui<'_> {
    /// A toggle under `key` in `rect`, flipping `on` on a click, Space while focused or an
    /// accessibility default action (and marking the response `changed`). `name` is its
    /// accessible name.
    pub fn toggle(&mut self, key: impl Hash, rect: Rect, on: &mut bool, name: &str) -> Response {
        let id = self.id(key);
        let mut r = self.interact(id, hit_rect(rect), Sense::CLICK_FOCUS);
        if (r.clicked || self.activation_keys(id, r.enabled, &[NamedKey::Space])) && r.enabled {
            *on = !*on;
            r.mark_changed();
        }

        let state = ToggleState {
            on: *on,
            hovered: r.hovered,
            focus_visible: r.focus_visible,
        };
        let target = ToggleLook::of(self.theme(), state);
        let look = ToggleLook {
            hover_fill: self.animate_color(id.with("hover"), target.hover_fill, &HOVER),
            on_fill: self.animate_color(id.with("on"), target.on_fill, &SELECTION),
            ring: self.animate_color(id.with("ring"), target.ring, &SELECTION),
            knob: self.animate_color(id.with("knob"), target.knob, &SELECTION),
            knob_size: self.animate_f32(id.with("size"), target.knob_size, &HOVER_SIZE),
            knob_pos: self.animate_f32(id.with("pos"), target.knob_pos, &MOVE),
            focus: self.animate_f32(id.with("focus"), target.focus, &FOCUS),
        };
        paint(self.painter(), rect, &look);

        let mut node = Node::new(id, Role::Switch, name, track(rect));
        if *on {
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

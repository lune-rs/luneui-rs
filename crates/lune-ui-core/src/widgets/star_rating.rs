//! A row of stars that sets a whole-number value (`widget.star-rating`, D120): `max` star buttons
//! (default [`MAX`]), the first `value` of them filled in the warning tone, the rest empty in the
//! tertiary tone. A click on star N sets N; a click on the current value clears it to 0.
//!
//! The row is one roving stop (role `radio-group`, each star a `radio`): Tab enters once, the
//! arrows move focus only (no wrap), Enter, Space and a click set the value, and so does the
//! accessibility default action. A change snaps; only the hover fill and the focus ring animate.

use std::hash::Hash;

use super::focus_ring::{self, RingKind};
use super::radio_group::group_response;
use crate::access::{Action, Node, Role, State};
use crate::color::{Rgba, with_alpha};
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Kind, Motion};
use crate::paint::{Painter, Rect};
use crate::theme::Theme;
use crate::ui::{Response, RoveAxis, Sense, Ui, WidgetId};

/// The star buttons' fill on hover.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Focus ring opacity.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// The default number of stars.
pub const MAX: u32 = 5;
/// The glyph's size.
pub const GLYPH: f32 = 20.0;
/// A star button's side: the glyph and 2 px of padding each side.
pub const BUTTON: f32 = 24.0;
/// Space between two buttons.
pub const GAP: f32 = 2.0;
/// The corner radius of a button's fill and ring.
pub const RADIUS: f32 = 4.0;

/// What the pointer and keyboard are doing to a star button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StarState {
    /// Within the value: drawn with the full glyph in the warning tone.
    pub filled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub focus_visible: bool,
}

/// The continuous look of a star button. A colour with alpha 0 draws nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StarLook {
    /// The button's fill: `surface_hover` on hover, `surface_active` pressed.
    pub fill: Rgba,
    pub glyph: Rgba,
    /// Whether the glyph is the full star.
    pub filled: bool,
    /// Focus ring opacity.
    pub focus: f32,
}

impl StarLook {
    /// The look `state` settles to.
    pub fn of(theme: &Theme, state: StarState) -> StarLook {
        StarLook {
            fill: StarLook::fill_for(theme, state),
            glyph: if state.filled {
                theme.status_warning
            } else {
                theme.fg_tertiary
            },
            filled: state.filled,
            focus: if state.focus_visible { 1.0 } else { 0.0 },
        }
    }

    /// The fill `state` settles to: transparent, in the hover colour so a fade has no dark middle.
    fn fill_for(theme: &Theme, state: StarState) -> Rgba {
        if state.pressed {
            theme.surface_active
        } else if state.hovered {
            theme.surface_hover
        } else {
            with_alpha(theme.surface_hover, 0.0)
        }
    }
}

/// The button of star `n` (0-based) of a row whose first star is at `origin`.
pub fn button_rect(origin: (f32, f32), n: u32) -> Rect {
    Rect::new(
        origin.0 + n as f32 * (BUTTON + GAP),
        origin.1,
        BUTTON,
        BUTTON,
    )
}

/// The width of a row of `max` stars.
pub fn row_width(max: u32) -> f32 {
    match max {
        0 => 0.0,
        n => n as f32 * BUTTON + (n - 1) as f32 * GAP,
    }
}

/// Draws one star button in `r` with `look`.
pub fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, r: Rect, look: &StarLook) {
    if look.fill[3] > 0 {
        p.fill(r, RADIUS, look.fill);
    }
    let glyph = if look.filled {
        icon::STAR_FULL
    } else {
        icon::STAR_EMPTY
    };
    p.text_centered(&icon::glyph(glyph, GLYPH, look.glyph), r);
    focus_ring::paint(
        p,
        r,
        RingKind::Edge {
            radius: RADIUS,
            on_accent: false,
        },
        look.focus,
    );
}

impl Ui<'_> {
    /// A star rating under `key` with its first star's top left at `at`: `max` stars (use
    /// [`MAX`]), `value` of them filled. A click on star N, or Enter or Space on the focused one,
    /// sets `value` to N, or to 0 when N is the current value; the response is `changed` when it
    /// did. A `value` above `max` is drawn as `max` and left alone until a star is chosen. `name`
    /// is the group's accessible name; each star is named "N of max".
    pub fn star_rating(
        &mut self,
        key: impl Hash,
        at: (f32, f32),
        value: &mut u32,
        max: u32,
        name: &str,
    ) -> Response {
        let t = *self.theme();
        let id = self.id(key);
        let group_on = self.is_enabled();
        let shown = (*value).min(max);
        let items: Vec<(WidgetId, bool)> = (1..=max).map(|n| (id.with(n), group_on)).collect();
        let rects: Vec<Rect> = (0..max).map(|n| button_rect(at, n)).collect();
        let active = shown.checked_sub(1).map(|i| i as usize);
        let stop = self.rove_stop(&items, active);

        let mut chosen = None;
        let mut kids = Vec::with_capacity(items.len());
        for (i, &(kid, _)) in items.iter().enumerate() {
            let sense = if stop == Some(i) {
                Sense::CLICK_FOCUS
            } else {
                Sense::CLICK_ROVE
            };
            let r = self.interact(kid, rects[i], sense);
            let keyed = self.activation_keys(kid, r.enabled, &[NamedKey::Enter, NamedKey::Space]);
            if r.enabled && (r.clicked || keyed) && chosen.is_none() {
                chosen = Some(i as u32 + 1);
            }
            kids.push(r);
        }
        self.rove_keys(&items, RoveAxis::Both, false);
        let mut group = group_response(
            id,
            Rect::new(
                at.0,
                at.1,
                row_width(max),
                if max == 0 { 0.0 } else { BUTTON },
            ),
            group_on,
            &kids,
        );
        if let Some(n) = chosen {
            *value = if n == shown { 0 } else { n };
            group.mark_changed();
        }
        let shown = (*value).min(max);

        let mut node = Node::new(id, Role::RadioGroup, name, group.rect);
        if !group_on {
            node.states.insert(State::Disabled);
        }
        self.node_container(node, |ui| {
            for (i, &(kid, _)) in items.iter().enumerate() {
                let n = i as u32 + 1;
                let state = StarState {
                    filled: n <= shown,
                    hovered: kids[i].hovered,
                    pressed: kids[i].pressed,
                    focus_visible: kids[i].focus_visible,
                };
                let target = StarLook::of(&t, state);
                let look = StarLook {
                    fill: ui.animate_color(kid.with("fill"), target.fill, &HOVER),
                    focus: ui.animate_f32(kid.with("focus"), target.focus, &FOCUS),
                    ..target
                };
                paint(ui.painter(), rects[i], &look);
                let mut star = Node::new(kid, Role::Radio, &format!("{n} of {max}"), rects[i]);
                if n == shown {
                    star.states.insert(State::Checked);
                }
                if group_on {
                    star.states.insert(State::Focusable);
                    star.actions.insert(Action::Default);
                } else {
                    star.states.insert(State::Disabled);
                }
                ui.node(star);
            }
        });
        group
    }
}

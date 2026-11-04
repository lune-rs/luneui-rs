//! A group of radio options with one roving stop (`widget.radio-group`, D120). A vertical column of
//! rows, each a 16 px circle and a label; the selected one has an `accent` fill and a 6 px dot.
//! Built on the layers of the checkbox: a [`RadioLook`] and its settled values, [`paint`] and
//! [`Ui::radio_group`].
//!
//! Tab enters the group once, on the selected option (or the first enabled one); an arrow moves
//! focus and the selection together, wrapping, and skips disabled options. Space selects the
//! focused option and a click selects the one under the pointer.

use std::hash::Hash;

use super::focus_ring::{self, RingKind};
use super::label;
use crate::access::{Action, Node, Role, State};
use crate::color::{Rgba, with_alpha};
use crate::input::NamedKey;
use crate::motion::{Kind, Motion};
use crate::paint::{Painter, Rect};
use crate::theme::Theme;
use crate::ui::{Response, RoveAxis, Sense, Ui, WidgetId};

/// Fill, ring and dot between selected and not.
pub const SELECTION: Motion = Motion::of(Kind::Selection);
/// The row's fill on hover.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Focus ring opacity.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// A row is at least this high; the group stacks its rows with no gap.
pub const ROW_HEIGHT: f32 = 28.0;
/// The circle's diameter.
pub const CIRCLE: f32 = 16.0;
/// The dot's diameter.
pub const DOT: f32 = 6.0;
/// Space between the row's left edge and the circle (and its right edge and the label's end).
pub const PAD: f32 = 8.0;
/// Space between the circle and the label.
pub const GAP: f32 = 8.0;
/// The corner radius of the row's hover fill.
pub const ROW_RADIUS: f32 = 4.0;

/// One option of a group. `key` is the value `selected` holds when it is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioOption<'a> {
    pub key: u64,
    pub label: &'a str,
    pub enabled: bool,
}

/// What the pointer and keyboard are doing to a radio row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RadioState {
    pub selected: bool,
    pub hovered: bool,
    pub focus_visible: bool,
}

/// The continuous look of a radio row. A colour with alpha 0 draws nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadioLook {
    /// The row's fill: `surface_hover` on hover.
    pub row_fill: Rgba,
    /// The circle's fill: `accent` when selected.
    pub fill: Rgba,
    /// The circle's outline while not selected.
    pub ring: Rgba,
    /// The dot.
    pub dot: Rgba,
    /// Focus ring opacity.
    pub focus: f32,
}

impl RadioLook {
    /// The look `state` settles to.
    pub fn of(theme: &Theme, state: RadioState) -> RadioLook {
        let row_fill = if state.hovered {
            theme.surface_hover
        } else {
            with_alpha(theme.surface_hover, 0.0)
        };
        let focus = if state.focus_visible { 1.0 } else { 0.0 };
        if state.selected {
            RadioLook {
                row_fill,
                fill: theme.accent,
                ring: with_alpha(theme.fg_secondary, 0.0),
                dot: theme.on_accent,
                focus,
            }
        } else {
            RadioLook {
                row_fill,
                fill: with_alpha(theme.accent, 0.0),
                ring: theme.fg_secondary,
                dot: with_alpha(theme.on_accent, 0.0),
                focus,
            }
        }
    }
}

/// The circle of the row `r`: 16 by 16, [`PAD`] from the left edge, centred vertically.
pub fn circle_rect(r: Rect) -> Rect {
    Rect::new(r.x + PAD, r.y + (r.h - CIRCLE) / 2.0, CIRCLE, CIRCLE)
}

/// Draws one row in `r` with `look`; the label is cut with an ellipsis to fit.
pub fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, r: Rect, text: &str, look: &RadioLook) {
    let t = p.theme;
    let c = circle_rect(r);
    if look.row_fill[3] > 0 {
        p.fill(r, ROW_RADIUS, look.row_fill);
    }
    if look.fill[3] > 0 {
        p.fill(c, CIRCLE / 2.0, look.fill);
    }
    if look.ring[3] > 0 {
        p.ring(c, CIRCLE / 2.0, 1.0, look.ring, false);
    }
    if look.dot[3] > 0 {
        let dot = Rect::new(
            c.x + (CIRCLE - DOT) / 2.0,
            c.y + (CIRCLE - DOT) / 2.0,
            DOT,
            DOT,
        );
        p.fill(dot, DOT / 2.0, look.dot);
    }
    focus_ring::paint(
        p,
        c,
        RingKind::Offset {
            radius: CIRCLE / 2.0,
        },
        look.focus,
    );
    let x = c.right() + GAP;
    let spec = p.elide(
        &label(&t, text, t.fg_secondary),
        (r.right() - PAD - x).max(0.0),
    );
    p.text_in(&spec, Rect::new(x, r.y, r.right() - x, r.h), 0.0);
}

/// The response of a composite: the union of its children's readings, over `rect`.
pub(crate) fn group_response(
    id: WidgetId,
    rect: Rect,
    enabled: bool,
    kids: &[Response],
) -> Response {
    let mut g = Response::inert(id, rect, enabled);
    for k in kids {
        g.hovered |= k.hovered;
        g.pressed |= k.pressed;
        g.clicked |= k.clicked;
        g.focused |= k.focused;
        g.focus_visible |= k.focus_visible;
    }
    g
}

impl Ui<'_> {
    /// A radio group under `key`: one row per option, stacked from the top left of `rect` with its
    /// width (its height is ignored), choosing a key into `selected`. A click, Space on the focused
    /// row or an arrow key (which moves focus too) selects; the response is `changed` once when
    /// the selection did. `name` is the group's accessible name. Disabled (see
    /// [`Ui::with_enabled`]) the group changes nothing and reports nothing; a disabled option is
    /// skipped by the arrows.
    pub fn radio_group(
        &mut self,
        key: impl Hash,
        rect: Rect,
        options: &[RadioOption<'_>],
        selected: &mut u64,
        name: &str,
    ) -> Response {
        let t = *self.theme();
        let id = self.id(key);
        let group_on = self.is_enabled();
        let items: Vec<(WidgetId, bool)> = options
            .iter()
            .map(|o| (id.with(o.key), o.enabled && group_on))
            .collect();
        let rows: Vec<Rect> = (0..options.len())
            .map(|i| Rect::new(rect.x, rect.y + i as f32 * ROW_HEIGHT, rect.w, ROW_HEIGHT))
            .collect();
        let active = options.iter().position(|o| o.key == *selected);
        let stop = self.rove_stop(&items, active);

        let mut chosen = None;
        let mut kids = Vec::with_capacity(options.len());
        for (i, o) in options.iter().enumerate() {
            let sense = if stop == Some(i) {
                Sense::CLICK_FOCUS
            } else {
                Sense::CLICK_ROVE
            };
            let kid = items[i].0;
            let r = self.with_enabled(o.enabled, |ui| {
                let r = ui.interact(kid, rows[i], sense);
                if r.enabled && (r.clicked || ui.consume_key(kid, NamedKey::Space)) {
                    chosen = Some(i);
                }
                r
            });
            kids.push(r);
        }
        if let Some(at) = self.rove_keys(&items, RoveAxis::Both, true) {
            chosen = Some(at);
        }
        let mut group = group_response(
            id,
            Rect::new(rect.x, rect.y, rect.w, options.len() as f32 * ROW_HEIGHT),
            group_on,
            &kids,
        );
        if let Some(i) = chosen
            && options[i].key != *selected
        {
            *selected = options[i].key;
            group.mark_changed();
        }

        let mut node = Node::new(id, Role::RadioGroup, name, group.rect);
        if !group_on {
            node.states.insert(State::Disabled);
        }
        self.node_container(node, |ui| {
            for (i, o) in options.iter().enumerate() {
                let kid = items[i].0;
                let state = RadioState {
                    selected: o.key == *selected,
                    hovered: kids[i].hovered,
                    focus_visible: kids[i].focus_visible,
                };
                let target = RadioLook::of(&t, state);
                ui.with_enabled(o.enabled, |ui| {
                    let look = RadioLook {
                        row_fill: ui.animate_color(kid.with("row"), target.row_fill, &HOVER),
                        fill: ui.animate_color(kid.with("fill"), target.fill, &SELECTION),
                        ring: ui.animate_color(kid.with("ring"), target.ring, &SELECTION),
                        dot: ui.animate_color(kid.with("dot"), target.dot, &SELECTION),
                        focus: ui.animate_f32(kid.with("focus"), target.focus, &FOCUS),
                    };
                    paint(ui.painter(), rows[i], o.label, &look);
                });
                let mut radio = Node::new(kid, Role::Radio, o.label, rows[i]);
                if state.selected {
                    radio.states.insert(State::Checked);
                }
                if items[i].1 {
                    radio.states.insert(State::Focusable);
                    radio.actions.insert(Action::Default);
                } else {
                    radio.states.insert(State::Disabled);
                }
                ui.node(radio);
            }
        });
        group
    }
}

//! NumericUpDown control: a value with − and + buttons in an input box. Built on the same four
//! layers as the button (see `button.rs`): a [`StepperLook`] and its settled values, [`paint`], the
//! Painter-API [`stepper`] and [`Ui::stepper`]. The app owns the value; the response reports the
//! step or the value the request asks for.

use std::hash::Hash;

use super::input_box::{self, InputBoxLook};
use super::label;
use crate::access::{AccessValue, Action, Node, Role, State, Value};
use crate::color::{Rgba, with_alpha};
use crate::input::NamedKey;
use crate::motion::{Kind, Lerp, Motion};
use crate::paint::{Painter, Rect};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui};

/// Button hover fill.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Focus-within ring colour (`border` to `accent`).
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// The value range and its step: the app's clamping rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepRange {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

/// What the pointer and focus are doing to a stepper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StepperState {
    pub minus_hovered: bool,
    pub plus_hovered: bool,
    pub focus_within: bool,
}

/// The continuous look of a stepper. A colour with alpha 0 draws nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepperLook {
    /// The minus button's fill.
    pub minus_hover: Rgba,
    /// The plus button's fill.
    pub plus_hover: Rgba,
    /// Focus-within ring: 0 is `border`, 1 is `accent`.
    pub focus: f32,
}

impl StepperLook {
    /// The look `state` settles to.
    pub fn of(theme: &Theme, state: StepperState) -> StepperLook {
        let fill = |hovered: bool| {
            if hovered {
                theme.surface_hover
            } else {
                with_alpha(theme.surface_hover, 0.0)
            }
        };
        StepperLook {
            minus_hover: fill(state.minus_hovered),
            plus_hover: fill(state.plus_hovered),
            focus: if state.focus_within { 1.0 } else { 0.0 },
        }
    }
}

/// The minus button: 24 wide, 4 px in from the top and bottom, its right edge 32 px from `r`'s.
pub fn minus_rect(r: Rect) -> Rect {
    Rect::new(r.right() - 56.0, r.y + 4.0, 24.0, r.h - 8.0)
}

/// The plus button, 4 px from `r`'s right edge.
pub fn plus_rect(r: Rect) -> Rect {
    Rect::new(r.right() - 28.0, r.y + 4.0, 24.0, r.h - 8.0)
}

/// Draws a stepper in `r` showing `value` with `look`.
pub fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, r: Rect, value: u32, look: &StepperLook) {
    let t = p.theme;
    // The border is `border` at focus 0 and `accent` at 1; both ends are exact.
    let border = t
        .border
        .map(f32::from)
        .lerp(t.accent.map(f32::from), look.focus)
        .map(|c| c.round() as u8);
    input_box::paint(
        p,
        r,
        &InputBoxLook {
            fill: t.surface_input,
            border,
        },
    );
    p.text_in(&label(&t, &value.to_string(), t.fg_primary), r, 10.0);
    for (b, glyph, hover) in [
        (minus_rect(r), "−", look.minus_hover),
        (plus_rect(r), "+", look.plus_hover),
    ] {
        if hover[3] > 0 {
            p.fill(b, 4.0, hover);
        }
        p.text_centered(&label(&t, glyph, t.fg_secondary), b);
    }
}

/// The Painter-API stepper: the unfocused look with the buttons' hover from `minus` and `plus`,
/// then their hit regions.
pub fn stepper<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    value: u32,
    minus: H,
    plus: H,
) {
    let state = StepperState {
        minus_hovered: p.hovered(minus),
        plus_hovered: p.hovered(plus),
        focus_within: false,
    };
    let look = StepperLook::of(&p.theme, state);
    paint(p, r, value, &look);
    p.hit(minus_rect(r), minus);
    p.hit(plus_rect(r), plus);
}

/// `v` clamped to `range`, snapped to its step from `min`, then clamped again (a snap can round past
/// `max`). `None` for NaN; an infinity clamps to the nearest end before the snap. A step that is not
/// a positive finite number does not snap. Never panics.
pub fn settle(v: f64, range: StepRange) -> Option<f64> {
    if v.is_nan() {
        return None;
    }
    let clamp = |x: f64| x.max(range.min).min(range.max);
    let x = clamp(v);
    if range.step.is_finite() && range.step > 0.0 {
        Some(clamp(
            range.min + ((x - range.min) / range.step).round() * range.step,
        ))
    } else {
        Some(x)
    }
}

/// What a stepper reports this frame.
#[derive(Debug, Clone, PartialEq)]
pub struct StepperResponse {
    pub response: Response,
    /// `+1` or `-1` per activation (a plus or minus click, Up or Down while focused, or the
    /// increment and decrement actions), summed; `None` when there was none.
    pub step: Option<i32>,
    /// The value an assistive technology asked for, clamped and snapped by [`settle`]; `None`
    /// when there was no request or it was NaN.
    pub set: Option<f64>,
}

impl Ui<'_> {
    /// A stepper under `key` in `rect` showing `value`. `name` is its accessible name. The
    /// buttons and Up or Down while focused report one step each; the app applies it.
    pub fn stepper(
        &mut self,
        key: impl Hash,
        rect: Rect,
        value: u32,
        range: StepRange,
        name: &str,
    ) -> StepperResponse {
        let t = *self.theme();
        let id = self.id(key);
        // The whole box first, so the buttons registered after it lie on top of it.
        let r = self.interact(id, rect, Sense::CLICK_FOCUS);
        let minus = self.interact(id.with("minus"), minus_rect(rect), Sense::CLICK);
        let plus = self.interact(id.with("plus"), plus_rect(rect), Sense::CLICK);

        let mut step = 0_i32;
        if minus.enabled && minus.clicked {
            step -= 1;
        }
        if plus.enabled && plus.clicked {
            step += 1;
        }
        if r.enabled {
            if self.consume_key(id, NamedKey::ArrowUp) {
                step += 1;
            }
            if self.consume_key(id, NamedKey::ArrowDown) {
                step -= 1;
            }
        }
        let set = match r.set_value.clone() {
            Some(AccessValue::Number(v)) => settle(v, range),
            _ => None,
        };

        let state = StepperState {
            minus_hovered: minus.hovered,
            plus_hovered: plus.hovered,
            focus_within: r.focused,
        };
        let target = StepperLook::of(&t, state);
        let look = StepperLook {
            minus_hover: self.animate_color(id.with("minus-fill"), target.minus_hover, &HOVER),
            plus_hover: self.animate_color(id.with("plus-fill"), target.plus_hover, &HOVER),
            focus: self.animate_f32(id.with("focus"), target.focus, &FOCUS),
        };
        paint(self.painter(), rect, value, &look);

        let mut node = Node::new(id, Role::SpinButton, name, rect);
        node.value = Some(Value::Number {
            value: f64::from(value),
            min: range.min,
            max: range.max,
            step: range.step,
        });
        if r.enabled {
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Increment);
            node.actions.insert(Action::Decrement);
            node.actions.insert(Action::SetValue);
        } else {
            node.states.insert(State::Disabled);
        }
        self.node(node);
        StepperResponse {
            response: r,
            step: (step != 0).then_some(step),
            set,
        }
    }
}

//! TextBox control with placeholder and static caret. Built on the same four layers as the button
//! (see `button.rs`): a [`FieldLook`] and its settled values, [`paint`], the Painter-API [`field`]
//! and [`Ui::text_field`]. M4 has focus, the node and the transitions; typing, selection and IME are
//! M5 (the engine), so a `Text` key does nothing here.

use std::hash::Hash;

use super::input_box::{self, InputBoxLook};
use super::label;
use crate::access::{AccessValue, Action, Node, Role, State, Value};
use crate::color::{Rgba, premul};
use crate::motion::{Kind, Lerp, Motion};
use crate::paint::{Painter, Rect};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui};

/// Box fill on hover (not while focused).
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Ring colour: `border` to `accent` on focus.
pub const FOCUS: Motion = Motion::of(Kind::Focus);
/// Ring colour: to `err` on an error.
pub const ERROR: Motion = Motion::of(Kind::Selection);

/// What the app says about the field: an error flag (the form around it sets it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FieldState {
    pub error: bool,
}

/// The continuous look of a text field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldLook {
    /// Box fill: `surface_hover` when hovered and not focused, else `surface_input`.
    pub fill: Rgba,
    /// Ring from `border` (0) to `accent` (1).
    pub focus: f32,
    /// Ring from the colour the focus gives to `err` (1); 0 keeps it.
    pub error: f32,
    /// The static caret, drawn while focused.
    pub caret: bool,
}

impl FieldLook {
    /// The look a field settles to for these flags.
    pub fn of(theme: &Theme, hovered: bool, focused: bool, state: FieldState) -> FieldLook {
        FieldLook {
            fill: if hovered && !focused {
                theme.surface_hover
            } else {
                theme.surface_input
            },
            focus: if focused { 1.0 } else { 0.0 },
            error: if state.error { 1.0 } else { 0.0 },
            caret: focused,
        }
    }
}

/// Draws a field showing `text` (or `placeholder` while empty) in `r` with `look`.
pub fn paint<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    placeholder: &str,
    look: &FieldLook,
) {
    let t = p.theme;
    // The ring is `border` at focus 0 and `accent` at 1; `err` replaces it at error 1. Each end is
    // exact, so the rest look is the old `input_box` look.
    let ring = t
        .border
        .map(f32::from)
        .lerp(t.accent.map(f32::from), look.focus)
        .map(|c| c.round() as u8);
    let border = ring
        .map(f32::from)
        .lerp(t.err.map(f32::from), look.error)
        .map(|c| c.round() as u8);
    input_box::paint(
        p,
        r,
        &InputBoxLook {
            fill: look.fill,
            border,
        },
    );
    p.push_clip(r.inset(6.0, 0.0));
    let (w, h) = if text.is_empty() {
        p.text_in(&label(&t, placeholder, t.fg_tertiary), r, 10.0);
        (0.0, 0.0)
    } else {
        p.text_in(&label(&t, text, t.fg_primary), r, 10.0)
    };
    if look.caret {
        let ch = if h > 0.0 { h } else { 16.0 };
        let caret = Rect::new(r.x + 10.0 + w + 1.0, r.y + (r.h - ch) / 2.0, 1.5, ch);
        p.fill_over(caret, 0.0, premul(t.fg_primary));
    }
    p.pop_clip();
}

/// The Painter-API field: the discrete look (`focused` given, hover from the painter), then its
/// hit region.
pub fn field<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    placeholder: &str,
    focused: bool,
    hit: H,
) {
    let look = FieldLook::of(&p.theme, p.hovered(hit), focused, FieldState::default());
    paint(p, r, text, placeholder, &look);
    p.hit(r, hit);
}

impl Ui<'_> {
    /// A text field under `key` in `rect` showing `text`. A click focuses it; a set-value access
    /// request replaces `text` and marks the response `changed`. `placeholder` is its accessible
    /// name (M4 has no label). Disabled (see [`Ui::with_enabled`]) it ignores the pointer.
    pub fn text_field(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: &mut String,
        placeholder: &str,
        state: FieldState,
    ) -> Response {
        let t = *self.theme();
        let id = self.id(key);
        let mut r = self.interact(id, rect, Sense::CLICK_FOCUS);
        if let Some(AccessValue::Text(value)) = r.set_value.clone() {
            *text = value;
            r.mark_changed();
        }

        let target = FieldLook::of(&t, r.hovered, r.focused, state);
        let look = FieldLook {
            fill: self.animate_color(id.with("fill"), target.fill, &HOVER),
            focus: self.animate_f32(id.with("focus"), target.focus, &FOCUS),
            error: self.animate_f32(id.with("error"), target.error, &ERROR),
            caret: target.caret,
        };
        paint(self.painter(), rect, text.as_str(), placeholder, &look);

        let mut node = Node::new(id, Role::TextInput, placeholder, rect);
        node.value = Some(Value::Text(text.clone()));
        if state.error {
            node.states.insert(State::Invalid);
        }
        if r.enabled {
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Focus);
            node.actions.insert(Action::SetValue);
        } else {
            node.states.insert(State::Disabled);
        }
        self.node(node);
        r
    }
}

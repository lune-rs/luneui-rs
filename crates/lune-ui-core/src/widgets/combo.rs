//! ComboBox control, on the same four layers as the button (see `button.rs`): a [`ComboLook`] and
//! its settled values, [`paint`], the Painter-API [`combo`] and [`Ui::combo`]. The list itself is
//! the dropdown's ([`Ui::dropdown`]); the combo only asks for it to open.

use std::hash::Hash;

use super::input_box::{self, InputBoxLook};
use super::label;
use crate::access::{Action, Node, Role, State, Value};
use crate::color::Rgba;
use crate::icon;
use crate::input::NamedKey;
use crate::motion::{Kind, Motion};
use crate::paint::{Painter, Rect};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui};

/// Fill colour change on hover.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Border colour change between closed and open (or focused).
pub const BORDER: Motion = Motion::of(Kind::Focus);

/// What the pointer, the keyboard and the app are doing to a combo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ComboState {
    pub hovered: bool,
    /// The app's list is open.
    pub open: bool,
    pub focus_visible: bool,
}

/// The continuous look of a combo: the input box's fill and border colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComboLook {
    pub fill: Rgba,
    /// `border` at rest, `accent` while open or keyboard-focused.
    pub border: Rgba,
}

impl ComboLook {
    /// The look `state` settles to.
    pub fn of(theme: &Theme, state: ComboState) -> ComboLook {
        let b = InputBoxLook::of(theme, state.hovered, state.open || state.focus_visible);
        ComboLook {
            fill: b.fill,
            border: b.border,
        }
    }
}

/// Draws a combo in `r` with `look`: the box, the clipped value and the chevron.
pub fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, r: Rect, value: &str, look: &ComboLook) {
    let t = p.theme;
    input_box::paint(
        p,
        r,
        &InputBoxLook {
            fill: look.fill,
            border: look.border,
        },
    );
    let text_clip = Rect::new(r.x, r.y, r.w - 30.0, r.h);
    p.push_clip(text_clip);
    p.text_in(&label(&t, value, t.fg_primary), r, 10.0);
    p.pop_clip();
    let chevron = Rect::new(r.right() - 28.0, r.y, 20.0, r.h);
    p.text_centered(
        &icon::glyph(icon::CHEVRON_DOWN, 12.0, t.fg_secondary),
        chevron,
    );
}

/// ComboBox: current value plus a chevron.
pub fn combo<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    value: &str,
    hovered: bool,
    open: bool,
    hit: H,
) {
    let state = ComboState {
        hovered,
        open,
        focus_visible: false,
    };
    let look = ComboLook::of(&p.theme, state);
    paint(p, r, value, &look);
    p.hit(r, hit);
}

impl Ui<'_> {
    /// A combo under `key` in `rect` showing `value`. A click, Enter, Space or Down while
    /// focused, or an accessibility default or expand action, sets `*open = true` (and `changed`)
    /// and reports `clicked`. It never closes itself: the list's dismissal does, through the app.
    /// `name` is its accessible name.
    pub fn combo(
        &mut self,
        key: impl Hash,
        rect: Rect,
        value: &str,
        open: &mut bool,
        name: &str,
    ) -> Response {
        let id = self.id(key);
        let mut r = self.interact(id, rect, Sense::CLICK_FOCUS);
        // Disabled, the keys are taken and nothing opens; a chord on them stays a shortcut.
        let key = self.activation_keys(
            id,
            r.enabled,
            &[NamedKey::Enter, NamedKey::Space, NamedKey::ArrowDown],
        );
        if r.enabled {
            if key {
                r.clicked = true;
            }
            if r.clicked && !*open {
                *open = true;
                r.mark_changed();
            }
        }

        let state = ComboState {
            hovered: r.hovered,
            open: *open,
            focus_visible: r.focus_visible,
        };
        let target = ComboLook::of(self.theme(), state);
        let look = ComboLook {
            fill: self.animate_color(id.with("fill"), target.fill, &HOVER),
            border: self.animate_color(id.with("border"), target.border, &BORDER),
        };
        paint(self.painter(), rect, value, &look);

        let mut node = Node::new(id, Role::ComboBox, name, rect);
        node.value = Some(Value::Text(value.to_owned()));
        if *open {
            node.states.insert(State::Expanded);
        }
        if r.enabled {
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Default);
            // No collapse: the combo never closes itself; the open list's dismiss closes it.
            node.actions.insert(Action::Expand);
        } else {
            node.states.insert(State::Disabled);
        }
        self.node(node);
        r
    }
}

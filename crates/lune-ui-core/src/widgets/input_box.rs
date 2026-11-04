//! Input-box chrome shared by the combo, stepper and field.

use crate::color::Rgba;
use crate::paint::{Painter, Rect};
use crate::theme::Theme;

/// The continuous look of an input box: fill and border colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct InputBoxLook {
    pub(crate) fill: Rgba,
    pub(crate) border: Rgba,
}

impl InputBoxLook {
    /// The look the box settles to.
    pub(crate) fn of(theme: &Theme, hovered: bool, focused: bool) -> InputBoxLook {
        InputBoxLook {
            fill: if hovered {
                theme.surface_hover
            } else {
                theme.surface_input
            },
            border: if focused { theme.accent } else { theme.border },
        }
    }
}

/// Draws the box in `r` with `look`.
pub(crate) fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, r: Rect, look: &InputBoxLook) {
    let radius = p.theme.item_radius;
    p.fill(r, radius, look.fill);
    p.ring(r, radius, 1.0, look.border, false);
}

pub(crate) fn input_box<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    hovered: bool,
    focused: bool,
) {
    let look = InputBoxLook::of(&p.theme, hovered, focused);
    paint(p, r, &look);
}

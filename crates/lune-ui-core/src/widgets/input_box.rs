//! Input-box chrome shared by the combo, stepper and field.

use crate::paint::{Painter, Rect};

pub(crate) fn input_box<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    hovered: bool,
    focused: bool,
) {
    let t = p.theme;
    p.fill(
        r,
        t.item_radius,
        if hovered {
            t.surface_hover
        } else {
            t.surface_input
        },
    );
    p.ring(
        r,
        t.item_radius,
        1.0,
        if focused { t.accent } else { t.border },
        false,
    );
}

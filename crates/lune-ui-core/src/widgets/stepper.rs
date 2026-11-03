//! NumericUpDown control.

use super::input_box;
use super::label;
use crate::paint::{Painter, Rect};

/// NumericUpDown: value with − / + buttons.
pub fn stepper<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    value: u32,
    minus: H,
    plus: H,
) {
    let t = p.theme;
    input_box(p, r, false, false);
    p.text_in(&label(&t, &value.to_string(), t.fg_primary), r, 10.0);
    for (k, (glyph, hit)) in [("−", minus), ("+", plus)].into_iter().enumerate() {
        let b = Rect::new(
            r.right() - 56.0 + k as f32 * 28.0,
            r.y + 4.0,
            24.0,
            r.h - 8.0,
        );
        if p.hovered(hit) {
            p.fill(b, 4.0, t.surface_hover);
        }
        p.text_centered(&label(&t, glyph, t.fg_secondary), b);
        p.hit(b, hit);
    }
}

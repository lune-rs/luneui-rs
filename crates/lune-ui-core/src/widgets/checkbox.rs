//! Checkbox control with label.

use super::label;
use crate::icon;
use crate::paint::{Painter, Rect};

pub fn checkbox<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    checked: bool,
    text: &str,
    hit: H,
) {
    let t = p.theme;
    let b = Rect::new(r.x, r.y + (r.h - 16.0) / 2.0, 16.0, 16.0);
    if checked {
        p.fill(b, 3.0, t.accent);
        p.text_centered(&icon::glyph(icon::CHECK, 12.0, t.on_accent), b);
    } else {
        p.ring(b, 3.0, 1.0, t.fg_secondary, false);
    }
    let (w, _) = p.text_in(
        &label(&t, text, t.fg_secondary),
        Rect::new(b.right() + 8.0, r.y, r.w, r.h),
        0.0,
    );
    p.hit(Rect::new(r.x, r.y, 24.0 + w, r.h), hit);
}

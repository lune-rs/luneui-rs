//! TextBox control with placeholder and static caret.

use super::input_box;
use super::label;
use crate::color::premul;
use crate::paint::{Painter, Rect};

/// TextBox with placeholder; draws a caret when focused.
pub fn field<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    placeholder: &str,
    focused: bool,
    hit: H,
) {
    let t = p.theme;
    input_box(p, r, p.hovered(hit) && !focused, focused);
    p.set_clip(Some(r.inset(6.0, 0.0)));
    let (w, h) = if text.is_empty() {
        p.text_in(&label(&t, placeholder, t.fg_tertiary), r, 10.0);
        (0.0, 0.0)
    } else {
        p.text_in(&label(&t, text, t.fg_primary), r, 10.0)
    };
    if focused {
        let ch = if h > 0.0 { h } else { 16.0 };
        let caret = Rect::new(r.x + 10.0 + w + 1.0, r.y + (r.h - ch) / 2.0, 1.5, ch);
        p.fill_over(caret, 0.0, premul(t.fg_primary));
    }
    p.set_clip(None);
    p.hit(r, hit);
}

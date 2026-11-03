//! ComboBox control.

use super::input_box;
use super::label;
use crate::icon;
use crate::paint::{Painter, Rect};

/// ComboBox: current value plus a chevron.
pub fn combo<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    value: &str,
    hovered: bool,
    open: bool,
    hit: H,
) {
    let t = p.theme;
    input_box(p, r, hovered, open);
    let text_clip = Rect::new(r.x, r.y, r.w - 30.0, r.h);
    p.set_clip(Some(text_clip));
    p.text_in(&label(&t, value, t.fg_primary), r, 10.0);
    p.set_clip(None);
    let chevron = Rect::new(r.right() - 28.0, r.y, 20.0, r.h);
    p.text_centered(
        &icon::glyph(icon::CHEVRON_DOWN, 12.0, t.fg_secondary),
        chevron,
    );
    p.hit(r, hit);
}

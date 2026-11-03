//! Button control and its label width.

use super::label;
use crate::paint::{Painter, Rect};

pub fn button<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    text: &str,
    primary: bool,
    hit: H,
) {
    let t = p.theme;
    let hovered = p.hovered(hit);
    let bg = match (primary, hovered) {
        (true, false) => t.accent,
        (true, true) => t.accent_hover,
        (false, false) => t.surface_hover,
        (false, true) => t.surface_selected,
    };
    p.fill(r, t.item_radius, bg);
    let fg = if primary {
        t.on_accent_text
    } else {
        t.fg_primary
    };
    p.text_centered(&label(&t, text, fg), r);
    p.hit(r, hit);
}

pub fn button_width<H: Copy + PartialEq>(p: &mut Painter<'_, H>, text: &str) -> f32 {
    let t = p.theme;
    p.measure(&label(&t, text, t.fg_primary)).0 + 32.0
}

//! Menu and dropdown surface.

use crate::color::Rgba;
use crate::paint::{Painter, Rect};

/// Menu / dropdown surface.
pub fn popup_panel<H: Copy + PartialEq>(p: &mut Painter<'_, H>, r: Rect, bg: Rgba, border: Rgba) {
    p.fill(r, 8.0, bg);
    p.ring(r, 8.0, 1.0, border, false);
}

//! Fluent toggle switch.

use crate::paint::{Painter, Rect};

/// Fluent ToggleSwitch: 40×20 track, accent fill when on.
pub fn toggle<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    r: Rect,
    on: bool,
    hovered: bool,
    hit: H,
) {
    let t = p.theme;
    let track = Rect::new(r.x, r.y + (r.h - 20.0) / 2.0, 40.0, 20.0);
    let knob = if hovered { 14.0 } else { 12.0 };
    if on {
        p.fill(track, 10.0, t.accent);
        let k = Rect::new(
            track.right() - 4.0 - knob,
            track.y + (20.0 - knob) / 2.0,
            knob,
            knob,
        );
        p.fill(k, knob / 2.0, t.on_accent);
    } else {
        if hovered {
            p.fill(track, 10.0, t.surface_hover);
        }
        p.ring(track, 10.0, 1.0, t.fg_secondary, false);
        let k = Rect::new(track.x + 4.0, track.y + (20.0 - knob) / 2.0, knob, knob);
        p.fill(k, knob / 2.0, t.fg_primary);
    }
    p.hit(track.inset(-4.0, -6.0), hit);
}

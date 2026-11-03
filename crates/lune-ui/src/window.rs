//! Helpers for a borderless glass window: edge resizing, title-bar double click and Windows 11
//! style caption buttons. The caller owns the event loop and calls these from it. The winit side
//! (window attributes, edge → resize direction) is in `winit_window`.

use std::time::{Duration, Instant};

use crate::paint::{Painter, Rect};

/// Width of the resize border of a borderless window, in logical pixels.
pub const EDGE_BORDER: f32 = 6.0;

/// An edge or corner of a window, for resizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

/// Which edge of a borderless window of size `w` × `h` the point `p` is on, for resizing.
pub fn edge_at(p: (f32, f32), w: f32, h: f32) -> Option<ResizeEdge> {
    use ResizeEdge as D;
    const B: f32 = EDGE_BORDER;
    let (l, r, t, b) = (p.0 < B, p.0 >= w - B, p.1 < B, p.1 >= h - B);
    Some(match (l, r, t, b) {
        (true, _, true, _) => D::NorthWest,
        (_, true, true, _) => D::NorthEast,
        (true, _, _, true) => D::SouthWest,
        (_, true, _, true) => D::SouthEast,
        (true, ..) => D::West,
        (_, true, ..) => D::East,
        (_, _, true, _) => D::North,
        (.., true) => D::South,
        _ => return None,
    })
}

/// Two clicks closer together than this are a double click.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// Detects double clicks from click times the caller supplies.
#[derive(Debug, Clone, Copy, Default)]
pub struct DoubleClick {
    last: Option<Instant>,
}

impl DoubleClick {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a click at `now`. True when the gap since the previous click is strictly less than
    /// [`DOUBLE_CLICK`].
    pub fn click(&mut self, now: Instant) -> bool {
        let double = self
            .last
            .is_some_and(|t| now.duration_since(t) < DOUBLE_CLICK);
        self.last = Some(now);
        double
    }
}

/// Windows 11 style caption buttons (minimize, maximize or restore, close) along the top right of
/// `win`, each `height` tall. `hits` are the hit values for [minimize, maximize, close].
pub fn caption_buttons<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    win: Rect,
    height: f32,
    maximized: bool,
    hits: [H; 3],
) {
    use crate::icon;
    let t = p.theme;
    let restore = if maximized {
        icon::CHROME_RESTORE
    } else {
        icon::CHROME_MAXIMIZE
    };
    let glyphs = [icon::CHROME_MINIMIZE, restore, icon::CHROME_CLOSE];
    for (k, (glyph, hit)) in glyphs.into_iter().zip(hits).enumerate() {
        let is_close = k == 2;
        let r = Rect::new(win.w - 46.0 * (3 - k) as f32, 0.0, 46.0, height);
        let hovered = p.hovered(hit);
        let mut fg = t.fg_primary;
        if hovered && is_close {
            p.fill(r, 0.0, t.caption_close_hover);
            fg = t.caption_close_fg;
        } else if hovered {
            p.fill(r, 0.0, t.surface_hover);
        }
        p.text_centered(&icon::glyph(glyph, 10.0, fg), r);
        p.hit(r, hit);
    }
}

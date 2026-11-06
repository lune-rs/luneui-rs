//! The tray's pure logic (`platform.tray-icon`, M10): the icon rasterised on the CPU, where the
//! flyout and the menu go from the icon's rect, and the click rules that open and close them.
//! Nothing here talks to an OS: the tray crate of the platform adapter hands in rects and times
//! and takes bitmaps and actions back.
//!
//! The icon is drawn by signed distance, not by the painter (D13): a crescent stroke, then a
//! transparent ring cut round the status dot, then the dot. The painter cannot erase, so only a
//! CPU raster has a ring that is truly see-through on a light and on a dark bar.

use std::time::Duration;

use crate::color::Rgba;
use crate::paint::Rect;
use crate::theme::Theme;
use crate::widgets::popover::{PlaceAlign, PlaceRequest, PopoverOptions, Side, Surface, place};

/// What the tray reports. Only the dot's colour follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrayState {
    /// `Theme::ok`.
    Serving,
    /// `Theme::warn`.
    Degraded,
    /// `Theme::paused`.
    Paused,
}

/// The glyph's colour, chosen to read on the bar it sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphTone {
    /// White, for a dark bar.
    Light,
    /// Black, for a light bar.
    Dark,
}

/// A straight-alpha RGBA bitmap, `width * height * 4` bytes, row-major from the top left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayBitmap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The icon's side at scale 1: a 16 px glyph box and 2 px for the dot's overhang.
const SIDE: u32 = 18;
/// The glyph box at scale 1.
const BOX: f32 = 16.0;
/// The crescent's stroke, the dot's radius and the ring's outer radius at scale 1.
const STROKE: f32 = 1.5;
const DOT_R: f32 = 3.0;
const RING_R: f32 = 4.5;
/// The dot's centre, from the top left, at scale 1 and at scale 2 (`right: -2px; bottom: -2px` of
/// the glyph box, `-4px` at 2x). At scale 1 it is on a pixel corner, so the 6 px dot covers whole
/// pixels (11 to 16) and not a soft 7 px disc; its ring's outer circle then runs half a pixel past
/// the bitmap's edge, which cuts nothing. At scale 2 it is 27, the 2x variant of the same design.
const DOT_AT: [f32; 2] = [14.0, 27.0];
/// The crescent's view box.
const VIEW: f32 = 24.0;
/// The longest straight piece of a flattened arc, in pixels (`chrome.rs` uses the same step).
const ARC_STEP: f32 = 0.75;

/// A pixel's coverage by a shape whose signed distance from the pixel centre is `sd`: 1 inside
/// by half a pixel, 0 outside by half a pixel.
fn coverage(sd: f32) -> f32 {
    (0.5 - sd).clamp(0.0, 1.0)
}

/// Appends the arc from `p0` to `p1` of radius `r` in SVG's terms (`large` and `sweep` are its
/// flags; sweep is clockwise on screen, y down), without `p0` itself, scaled by `s`.
fn svg_arc(
    out: &mut Vec<[f32; 2]>,
    p0: [f32; 2],
    p1: [f32; 2],
    r: f32,
    large: bool,
    sweep: bool,
    s: f32,
) {
    let (mx, my) = ((p0[0] + p1[0]) / 2.0, (p0[1] + p1[1]) / 2.0);
    let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
    let d = (dx * dx + dy * dy).sqrt();
    if d < 1e-4 || r < 1e-4 {
        out.push([p1[0] * s, p1[1] * s]);
        return;
    }
    let h = (r * r - d * d / 4.0).max(0.0).sqrt();
    let (nx, ny) = (-dy / d, dx / d);
    // Of the two centres, the one whose arc runs the asked way round is `large` or not.
    let mut best = None;
    for sign in [1.0_f32, -1.0] {
        let c = [mx + nx * h * sign, my + ny * h * sign];
        let a0 = (p0[1] - c[1]).atan2(p0[0] - c[0]);
        let a1 = (p1[1] - c[1]).atan2(p1[0] - c[0]);
        let mut delta = a1 - a0;
        if sweep && delta < 0.0 {
            delta += std::f32::consts::TAU;
        } else if !sweep && delta > 0.0 {
            delta -= std::f32::consts::TAU;
        }
        if (delta.abs() > std::f32::consts::PI) == large || best.is_none() {
            best = Some((c, a0, delta));
        }
    }
    let Some((c, a0, delta)) = best else { return };
    let steps = ((delta.abs() * r * s) / ARC_STEP).ceil().max(4.0) as usize;
    for i in 1..=steps {
        let a = a0 + delta * i as f32 / steps as f32;
        out.push([(c[0] + r * a.cos()) * s, (c[1] + r * a.sin()) * s]);
    }
}

/// The crescent's outline, a closed polyline in pixels (the last point joins the first). The
/// provisional mark of the consuming app's design (`M15.5 3.5a8.5 8.5 0 1 0 5 15.4A9 9 0 0 1 15.5 3.5Z` in a
/// 24 unit box), scaled into the glyph box.
fn crescent(scale: f32) -> Vec<[f32; 2]> {
    let s = BOX * scale / VIEW;
    let (top, bottom) = ([15.5, 3.5], [20.5, 18.9]);
    let mut pts = vec![[top[0] * s, top[1] * s]];
    svg_arc(&mut pts, top, bottom, 8.5, true, false, s);
    svg_arc(&mut pts, bottom, top, 9.0, false, true, s);
    pts
}

/// The distance from `p` to the closed polyline `pts`.
fn distance_to(pts: &[[f32; 2]], p: (f32, f32)) -> f32 {
    let mut best = f32::MAX;
    for (i, a) in pts.iter().enumerate() {
        let b = pts[(i + 1) % pts.len()];
        let (abx, aby) = (b[0] - a[0], b[1] - a[1]);
        let len2 = abx * abx + aby * aby;
        let t = if len2 < 1e-12 {
            0.0
        } else {
            (((p.0 - a[0]) * abx + (p.1 - a[1]) * aby) / len2).clamp(0.0, 1.0)
        };
        let (qx, qy) = (a[0] + abx * t - p.0, a[1] + aby * t - p.1);
        best = best.min((qx * qx + qy * qy).sqrt());
    }
    best
}

/// The tray icon for `state` as an RGBA bitmap, 18 px square at `scale` 1 and 36 at 2 (any other
/// scale, 0 included, is 2). The crescent sits in a 16 px (32) glyph box at the top left; the
/// status dot is 6 px (12) with a 1.5 px (3) ring cut round it, its centre at (14, 14) ((27, 27)) so
/// the dot sits on the pixel grid. Drawn in this order: the glyph in `tone`, every pixel inside the
/// ring's outer circle made transparent (the ring is alpha 0, not a colour), then the dot in
/// `theme.ok`, `warn` or `paused`. Edges are anti-aliased from coverage; a pixel wholly inside
/// the dot is exactly its colour. Pure and deterministic.
pub fn icon_bitmap(state: TrayState, tone: GlyphTone, scale: u32, theme: &Theme) -> TrayBitmap {
    let k = if scale == 1 { 1_u32 } else { 2 };
    let kf = k as f32;
    let side = SIDE * k;
    let glyph = match tone {
        GlyphTone::Light => 1.0_f32,
        GlyphTone::Dark => 0.0,
    };
    let dot: Rgba = match state {
        TrayState::Serving => theme.ok,
        TrayState::Degraded => theme.warn,
        TrayState::Paused => theme.paused,
    };
    let dot_a = f32::from(dot[3]) / 255.0;
    let outline = crescent(kf);
    let half = STROKE * kf / 2.0;
    let at = DOT_AT[(k - 1) as usize];
    let centre = (at, at);
    let mut rgba = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            // Premultiplied, so the three cuts below are plain products.
            let cg = coverage(distance_to(&outline, p) - half);
            let mut px = [glyph * cg, glyph * cg, glyph * cg, cg];
            let dc = ((p.0 - centre.0).powi(2) + (p.1 - centre.1).powi(2)).sqrt();
            let keep = 1.0 - coverage(dc - RING_R * kf);
            for v in &mut px {
                *v *= keep;
            }
            let over = coverage(dc - DOT_R * kf);
            for (v, c) in px.iter_mut().zip(&dot[..3]) {
                *v = f32::from(*c) / 255.0 * dot_a * over + *v * (1.0 - dot_a * over);
            }
            px[3] = dot_a * over + px[3] * (1.0 - dot_a * over);
            let alpha = (px[3] * 255.0).round().clamp(0.0, 255.0) as u8;
            // A pixel that rounds to see-through has no colour either.
            if alpha == 0 {
                rgba.extend_from_slice(&[0; 4]);
                continue;
            }
            for v in &px[..3] {
                rgba.push(((v / px[3]) * 255.0).round().clamp(0.0, 255.0) as u8);
            }
            rgba.push(alpha);
        }
    }
    TrayBitmap {
        width: side,
        height: side,
        rgba,
    }
}

/// What the tray opens, and so where it goes and how far it keeps from the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraySurface {
    /// The menu 4 px past the bar, flush with the screen's edge (macOS).
    MenuUnderBar,
    /// The flyout 12 px from the bar and 12 px from the screen's edge (Windows, left click).
    FlyoutOverBar,
    /// The menu 8 px from the bar and 12 px from the screen's edge (Windows, right click).
    MenuOverBar,
}

/// The bar the tray icon sits in, from the icon's rect and the screen (D16): the strip along the
/// screen edge the icon is nearest, reaching through the icon. On macOS this is the menu bar
/// exactly; on Windows the icon's rect may sit a few px inside the taskbar, so the strip may
/// start a little inside it. Ties go to the top, then the bottom, the left, the right.
///
/// Logical pixels in one coordinate space: `screen` and `tray` are in the same unit and share an
/// origin. A tray item's rect from the OS is physical, so the caller divides it (and the screen) by
/// the monitor's scale first (the offsets from the monitor's origin), as `place_surface`'s gaps are
/// logical.
pub fn bar_from_tray(screen: Rect, tray: Rect) -> Rect {
    let top = tray.y - screen.y;
    let bottom = screen.bottom() - tray.bottom();
    let left = tray.x - screen.x;
    let right = screen.right() - tray.right();
    let nearest = top.min(bottom).min(left).min(right);
    if nearest == top {
        Rect::new(
            screen.x,
            screen.y,
            screen.w,
            (tray.bottom() - screen.y).max(0.0),
        )
    } else if nearest == bottom {
        let y = tray.y.min(screen.bottom());
        Rect::new(screen.x, y, screen.w, screen.bottom() - y)
    } else if nearest == left {
        Rect::new(
            screen.x,
            screen.y,
            (tray.right() - screen.x).max(0.0),
            screen.h,
        )
    } else {
        let x = tray.x.min(screen.right());
        Rect::new(x, screen.y, screen.right() - x, screen.h)
    }
}

/// Where `surface` of `size` goes for a bar: on the screen side of the bar (below a bar on the
/// top edge, above one on the bottom, beside one on a side), its end edge on the screen's end
/// edge, kept on screen by [`place`]. The gap to the bar is 4, 12 and 8 px and the margin to the
/// screen's edges 0, 12 and 12 px, in the order of [`TraySurface`]. Logical pixels, one coordinate
/// space: `screen`, `bar` and `size` are in the same unit, which is the unit of the gaps, so a
/// caller on a monitor at scale 2 passes physical rects divided by 2 and multiplies the answer
/// back, or its gaps come out halved. A surface larger than the
/// screen keeps its size and sits on the margin, as `place` has it.
pub fn place_surface(surface: TraySurface, screen: Rect, bar: Rect, size: (f32, f32)) -> Rect {
    let (gap, margin) = match surface {
        TraySurface::MenuUnderBar => (4.0, 0.0),
        TraySurface::FlyoutOverBar => (12.0, 12.0),
        TraySurface::MenuOverBar => (8.0, 12.0),
    };
    // The side of the anchor the surface prefers: away from the screen edge the bar is on.
    let side = if bar.w >= bar.h {
        if bar.y - screen.y <= screen.bottom() - bar.bottom() {
            Side::Bottom
        } else {
            Side::Top
        }
    } else if bar.x - screen.x <= screen.right() - bar.right() {
        Side::Right
    } else {
        Side::Left
    };
    place(&PlaceRequest {
        anchor: bar,
        size,
        side,
        align: PlaceAlign::End,
        gap,
        margin,
        shift: true,
        window: screen,
    })
    .rect
}

/// The OS whose tray rules apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrayPlatform {
    MacOs,
    Windows,
}

/// What a click opens: on macOS the menu under the bar, on Windows a left click the flyout and a
/// right click the menu.
pub fn tray_surface(platform: TrayPlatform, left: bool) -> TraySurface {
    match (platform, left) {
        (TrayPlatform::MacOs, _) => TraySurface::MenuUnderBar,
        (TrayPlatform::Windows, true) => TraySurface::FlyoutOverBar,
        (TrayPlatform::Windows, false) => TraySurface::MenuOverBar,
    }
}

/// What the app does after a tray event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// Show this surface (swapping a different one that is open).
    Open(TraySurface),
    /// Hide the open surface.
    Close,
    /// Nothing.
    None,
}

/// How long after a focus-loss close a click on the icon is ignored.
const REOPEN_GUARD: Duration = Duration::from_millis(250);

/// The click rules, as a state machine over app-passed time (D18).
///
/// A click on the icon takes focus from the open surface first, so the surface closes from focus
/// loss a moment before the click arrives; reading the click as "open" would flicker the surface
/// shut and open. A click within 250 ms after a focus-loss close is ignored when it asks for the
/// surface that closed; a click for another surface (the right button with the flyout open, on
/// Windows) opens it, since it cannot be that reopen.
///
/// The app calls [`Self::focus_lost`] when the surface's window loses focus, from its event
/// handler before the frame runs, and [`Self::closed`] for every other way the surface went away
/// (Escape, a picked row).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrayToggle {
    open: Option<TraySurface>,
    /// The surface a focus loss closed and when: the click that took the focus is for this one.
    focus_closed: Option<(TraySurface, Duration)>,
}

impl TrayToggle {
    pub fn new() -> Self {
        Self::default()
    }

    /// The surface the toggle thinks is open.
    pub fn open(&self) -> Option<TraySurface> {
        self.open
    }

    /// A click on the icon that asks for `surface` at `now`: the same surface open closes, a
    /// different one swaps, and with none open it opens, unless the click is the one that took
    /// focus from that same surface a moment ago.
    pub fn click(&mut self, surface: TraySurface, now: Duration) -> TrayAction {
        match self.open {
            Some(open) if open == surface => {
                self.open = None;
                TrayAction::Close
            }
            Some(_) => {
                self.open = Some(surface);
                TrayAction::Open(surface)
            }
            None => {
                let recent = self.focus_closed.take().is_some_and(|(closed, at)| {
                    closed == surface && now.saturating_sub(at) < REOPEN_GUARD
                });
                if recent {
                    TrayAction::None
                } else {
                    self.open = Some(surface);
                    TrayAction::Open(surface)
                }
            }
        }
    }

    /// The surface's window lost focus at `now`: it closes, and the click that may follow is
    /// not a reopen. With none open, nothing.
    pub fn focus_lost(&mut self, now: Duration) -> TrayAction {
        if let Some(surface) = self.open.take() {
            self.focus_closed = Some((surface, now));
            TrayAction::Close
        } else {
            TrayAction::None
        }
    }

    /// The surface went away another way (Escape, a pick): none is open.
    pub fn closed(&mut self) {
        self.open = None;
    }
}

/// A popover whose rect is the window `window` (D17): `popover` places it flush at the window's
/// origin at `size` with no margin and no shift, so the card is the window. The widgets of
/// `overlay.flyout` and `overlay.menu` run through this to fill a window of their own.
pub(crate) fn window_popover(window: Rect, size: (f32, f32), surface: Surface) -> PopoverOptions {
    PopoverOptions {
        gap: 0.0,
        margin: 0.0,
        shift: false,
        surface,
        ..PopoverOptions::new(Rect::new(window.x, window.y, 0.0, 0.0), size)
    }
}

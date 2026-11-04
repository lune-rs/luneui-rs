//! The `motion` page: the seven easings plotted, one swatch per animation kind, a `move`
//! retargeted in flight, and the kind swatches under each tier. Swatches are driven by
//! `Ui::animate_f32` (so the page exercises the same transitions widgets use), plots by
//! `Easing::eval`, and the spinner by `spin_angle`. There is no rotation primitive: spin shows a
//! dot travelling a circle.

use std::time::Duration;

use lune_ui::lunegit_dark::THEME;
use lune_ui::motion::{
    Easing, Kind, MOTION, Motion, MotionTier, Reduce, Token, Transition, spin_angle,
};
use lune_ui::paint::{Painter, Rect, StrokeStyle, Xform};
use lune_ui::text::{Spec, Style};
use lune_ui::ui::{Ui, WidgetId};

use super::{caption, card, heading};
use crate::page::{Expect, Page, PageKind, PageState, Step, UiPage, UiScene};

pub(crate) static PAGE: Page = Page {
    id: "motion",
    title: "Motion",
    kind: PageKind::Ui(UiPage {
        state: live,
        scenes: SCENES,
        bench: &[],
    }),
};

type P<'a> = Painter<'a, WidgetId>;

// ---------------------------------------------------------------------------------------------
// Names

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Hover => "hover",
        Kind::Press => "press",
        Kind::Focus => "focus",
        Kind::Appear => "appear",
        Kind::Dismiss => "dismiss",
        Kind::PageSwap => "page-swap",
        Kind::Drawer => "drawer",
        Kind::Pop => "pop",
        Kind::Selection => "selection",
        Kind::Move => "move",
        Kind::Expand => "expand",
        Kind::Value => "value",
        Kind::Spin => "spin",
        Kind::Scroll => "scroll",
        Kind::Settle => "settle",
    }
}

fn token_name(t: Token) -> &'static str {
    match t {
        Token::Zero => "zero",
        Token::Instant => "instant",
        Token::Fast => "fast",
        Token::Base => "base",
        Token::Moderate => "moderate",
        Token::Slow => "slow",
    }
}

fn easing_name(e: Easing) -> &'static str {
    match e {
        Easing::Linear => "linear",
        Easing::QuadOut => "quad-out",
        Easing::QuadIn => "quad-in",
        Easing::SineInOut => "sine-in-out",
        Easing::BackOut => "back-out",
        e if e == Easing::CUBIC_OUT => "cubic-out",
        e if e == Easing::EASE_IN => "ease-in",
        Easing::CubicBezier(..) => "cubic-bezier",
    }
}

/// The seven easings of `docs/motion.md`.
const EASINGS: [Easing; 7] = [
    Easing::Linear,
    Easing::QuadOut,
    Easing::QuadIn,
    Easing::SineInOut,
    Easing::BackOut,
    Easing::CUBIC_OUT,
    Easing::EASE_IN,
];

// ---------------------------------------------------------------------------------------------
// Small helpers

fn lerp(a: f32, b: f32, v: f32) -> f32 {
    a + (b - a) * v
}

fn mix(a: [u8; 4], b: [u8; 4], v: f32) -> [u8; 4] {
    let ch = |i: usize| {
        lerp(f32::from(a[i]), f32::from(b[i]), v)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    [ch(0), ch(1), ch(2), ch(3)]
}

fn label(ui: &mut Ui<'_>, text: &str, x: f32, y: f32, size: f32, color: [u8; 4]) {
    ui.painter()
        .text(&Spec::plain(text, Style::ui(size, color)), x, y);
}

fn alpha(a: f32) -> Xform {
    Xform {
        origin: (0.0, 0.0),
        scale: 1.0,
        dy: 0.0,
        alpha: a,
    }
}

/// Runs `f` under an alpha-only transform; nothing is drawn at zero opacity.
fn faded(p: &mut P<'_>, a: f32, f: impl FnOnce(&mut P<'_>)) {
    if a <= 0.0 {
        return;
    }
    p.push_xform(alpha(a));
    f(p);
    p.pop_xform();
}

fn scaled(p: &mut P<'_>, r: Rect, scale: f32, f: impl FnOnce(&mut P<'_>)) {
    p.push_xform(Xform {
        origin: (r.x + r.w / 2.0, r.y + r.h / 2.0),
        scale,
        dy: 0.0,
        alpha: 1.0,
    });
    f(p);
    p.pop_xform();
}

// ---------------------------------------------------------------------------------------------
// Easing plots

/// `e` over 0..1 in `area`: the unit square and the curve (back-out overshoots the top).
fn plot(p: &mut P<'_>, area: Rect, e: Easing) {
    p.ring(area, 4.0, 1.0, THEME.border_subtle, false);
    let n = 48;
    let pts: Vec<[f32; 2]> = (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            [area.x + t * area.w, area.bottom() - e.eval(t) * area.h]
        })
        .collect();
    p.stroke(&pts, false, StrokeStyle::solid(THEME.accent, 2.0));
}

fn easings_row(ui: &mut Ui<'_>, x: f32, y: f32) {
    heading(ui, "EASINGS", x, y);
    for (i, e) in EASINGS.into_iter().enumerate() {
        let cx = x + i as f32 * 136.0;
        plot(ui.painter(), Rect::new(cx, y + 30.0, 96.0, 96.0), e);
        label(
            ui,
            easing_name(e),
            cx,
            y + 136.0,
            THEME.font_s,
            THEME.fg_secondary,
        );
    }
}

struct Easings;

impl PageState for Easings {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Motion: easings");
        easings_row(ui, card.x + 24.0, card.y + 24.0);
    }
}

/// One curve, large, beside its token name.
struct Curve(Easing);

impl PageState for Curve {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        let card = card(ui, "Motion: curve");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "CURVE", x, y);
        plot(ui.painter(), Rect::new(x, y + 30.0, 240.0, 240.0), self.0);
        let name = easing_name(self.0);
        label(
            ui,
            name,
            x + 264.0,
            y + 30.0,
            THEME.font_m,
            THEME.fg_primary,
        );
        let note = match self.0 {
            e if e == Easing::CUBIC_OUT => {
                "The menu's open curve: cubic-bezier(0.215, 0.61, 0.355, 1)."
            }
            _ => "The menu's close curve: cubic-bezier(0.42, 0, 1, 1).",
        };
        label(
            ui,
            note,
            x + 264.0,
            y + 56.0,
            THEME.font_base,
            THEME.fg_secondary,
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Kind swatches

/// The progress of `kind`'s change toward `on` under `tier`, and whether transforms (slide,
/// scale) apply. Under the frame's own tier this is `Ui::animate_f32`; for another tier the row is
/// rebuilt from what `MotionTokens::resolve` makes of it there, so one frame can show three tiers.
fn drive(ui: &mut Ui<'_>, id: WidgetId, kind: Kind, tier: MotionTier, on: bool) -> (f32, bool) {
    let target = if on { 1.0 } else { 0.0 };
    let motion = Motion::of(kind);
    let Some(r) = ui.theme().motion.resolve(&motion, tier) else {
        // A snap shows the end state at once, transforms included.
        return (target, true);
    };
    let row = if tier == ui.tier() || ui.tier() != MotionTier::Full {
        motion
    } else {
        let token = Token::ALL
            .into_iter()
            .find(|t| ui.theme().motion.duration(*t) == r.duration)
            .unwrap_or(Token::Base);
        Motion {
            kind,
            token,
            easing: r.easing,
            reduced: Reduce::Keep,
            reduced_token: None,
        }
    };
    (ui.animate_f32(id, target, &row), r.transforms)
}

/// Draws `kind`'s change at progress `v` in `area`. Without `transforms` (Reduced) only the
/// opacity and colour parts move; the slide and scale parts stay at rest.
fn swatch(p: &mut P<'_>, area: Rect, kind: Kind, v: f32, transforms: bool, spin_deg: f32) {
    p.ring(area, 6.0, 1.0, THEME.border_subtle, false);
    let s = (area.h * 0.5).round();
    let e = Rect::new(
        area.x + ((area.w - s) / 2.0).round(),
        area.y + ((area.h - s) / 2.0).round(),
        s,
        s,
    );
    let (dim, hot) = (THEME.surface_selected, THEME.accent);
    let t = |moving: f32| if transforms { moving } else { 0.0 };
    match kind {
        Kind::Hover => p.fill(e, 4.0, mix(dim, hot, v)),
        Kind::Press => {
            let k = if transforms { lerp(1.0, 0.98, v) } else { 1.0 };
            let big = e.inset(-6.0, -6.0);
            scaled(p, big, k, |p| p.fill(big, 4.0, hot));
        }
        Kind::Focus => {
            p.fill(e, 4.0, dim);
            faded(p, v, |p| p.ring(e.inset(-3.0, -3.0), 6.0, 2.0, hot, false));
        }
        Kind::Appear => faded(p, v, |p| {
            p.fill(Rect::new(e.x, e.y + t((1.0 - v) * 8.0), e.w, e.h), 4.0, hot);
        }),
        Kind::Dismiss => faded(p, 1.0 - v, |p| {
            p.fill(Rect::new(e.x, e.y + t(v * 6.0), e.w, e.h), 4.0, hot);
        }),
        Kind::PageSwap => {
            faded(p, 1.0 - v, |p| p.fill(e, 4.0, dim));
            faded(p, v, |p| {
                p.fill(Rect::new(e.x, e.y + t((1.0 - v) * 6.0), e.w, e.h), 4.0, hot);
            });
        }
        Kind::Drawer => faded(p, v, |p| {
            p.fill(Rect::new(e.x + t((1.0 - v) * 6.0), e.y, e.w, e.h), 4.0, hot);
        }),
        Kind::Pop => {
            let k = if transforms { lerp(0.96, 1.0, v) } else { 1.0 };
            faded(p, v, |p| scaled(p, e, k, |p| p.fill(e, 4.0, hot)));
        }
        Kind::Selection => {
            p.fill(e, 4.0, dim);
            faded(p, v, |p| p.fill(e, 4.0, hot));
        }
        Kind::Move => {
            let d = (s * 0.5).round();
            let (a, b) = (area.x + 6.0, area.right() - 6.0 - d);
            p.fill(
                Rect::new(a, area.y + area.h - 8.0, 2.0, 4.0),
                0.0,
                THEME.fg_faint,
            );
            p.fill(
                Rect::new(b + d - 2.0, area.y + area.h - 8.0, 2.0, 4.0),
                0.0,
                THEME.fg_faint,
            );
            p.fill(Rect::new(lerp(a, b, v), e.y, d, d), 4.0, hot);
        }
        Kind::Expand => {
            let h = lerp(10.0, area.h - 12.0, v);
            let r = Rect::new(area.x + 10.0, area.y + 6.0, area.w - 20.0, h);
            p.fill(r, 4.0, dim);
            p.ring(r, 4.0, 1.0, hot, false);
        }
        Kind::Value => {
            let track = Rect::new(
                area.x + 8.0,
                area.y + (area.h - 8.0) / 2.0,
                area.w - 16.0,
                8.0,
            );
            p.fill(track, 4.0, dim);
            p.fill(Rect::new(track.x, track.y, track.w * v, track.h), 4.0, hot);
        }
        Kind::Spin => {
            let (c, r) = (
                (area.x + area.w / 2.0, area.y + area.h / 2.0),
                area.h * 0.32,
            );
            let ring: Vec<[f32; 2]> = (0..=32)
                .map(|i| {
                    let a = i as f32 / 32.0 * std::f32::consts::TAU;
                    [c.0 + r * a.sin(), c.1 - r * a.cos()]
                })
                .collect();
            p.stroke(&ring, true, StrokeStyle::solid(THEME.border_subtle, 1.5));
            let a = spin_deg.to_radians();
            let dot = Rect::new(c.0 + r * a.sin() - 3.0, c.1 - r * a.cos() - 3.0, 6.0, 6.0);
            p.fill(dot, 3.0, hot);
        }
        Kind::Scroll => {
            p.set_clip(Some(area.inset(2.0, 2.0)));
            let off = v * 18.0;
            for i in 0..4 {
                let r = Rect::new(
                    area.x + 8.0,
                    area.y + 6.0 + i as f32 * 18.0 - off,
                    area.w - 16.0,
                    8.0,
                );
                p.fill(r, 3.0, if i == 1 { hot } else { dim });
            }
            p.set_clip(None);
        }
        Kind::Settle => {
            let slot = (area.h * 0.3).round();
            let (x, w, h) = (area.x + 10.0, area.w - 20.0, slot - 4.0);
            p.fill(Rect::new(x, area.y + 4.0, w, h), 3.0, dim);
            p.fill(
                Rect::new(x, area.y + 4.0 + lerp(0.0, slot, v), w, h),
                3.0,
                hot,
            );
        }
    }
}

/// How the swatches are driven.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Drive {
    /// Every change already happened (frame 0): the resting look, no spin.
    Settled,
    /// Each kind's change starts so that it is at half its own duration at `at` ms.
    HalfwayAt(u32),
    /// All changes start at the page's second frame (0 ms in the scenes).
    Start,
    /// Window mode: every 1.2 s all flip, so a change interrupts the previous one.
    Loop,
}

/// When `kind`'s change starts in `Drive::HalfwayAt(at)`: `at` minus half its default duration.
fn start_ms(kind: Kind, at: u32) -> u32 {
    let d = MOTION.duration(Motion::of(kind).token).as_millis() as u32;
    at.saturating_sub(d / 2)
}

/// The kinds whose half-way instant `at` makes a swatch start strictly after 0 ms.
const HALFWAY_MS: u32 = 300;

/// Where the swatches sit and how they are driven this frame.
struct Grid {
    drive: Drive,
    /// Frames drawn so far (the second frame is when `Drive::Start` starts).
    frames: u32,
}

impl Grid {
    fn on(&self, ui: &Ui<'_>, kind: Kind) -> bool {
        let now = ui.time();
        match self.drive {
            Drive::Settled => true,
            Drive::HalfwayAt(at) => now >= Duration::from_millis(u64::from(start_ms(kind, at))),
            Drive::Start => self.frames >= 2,
            Drive::Loop => (now.as_millis() / 1200) % 2 == 1,
        }
    }

    fn spin_deg(&self, ui: &Ui<'_>) -> f32 {
        match self.drive {
            Drive::Settled => 0.0,
            _ => spin_angle(ui.time()),
        }
    }

    /// Keeps a window-mode page alive: the next flip, and a frame for the spinner.
    fn keep_alive(&self, ui: &mut Ui<'_>) {
        if self.drive == Drive::Loop {
            let now = ui.time();
            let next = Duration::from_millis((now.as_millis() as u64 / 1200 + 1) * 1200);
            ui.request_repaint_at(next.min(now + Duration::from_millis(16)));
        }
    }
}

/// Fifteen kinds in a grid of `cols` columns: `area` swatches, `pitch` apart, `tier` applied.
#[allow(clippy::too_many_arguments)]
fn kinds_grid(
    ui: &mut Ui<'_>,
    g: &Grid,
    scope: &str,
    tier: MotionTier,
    origin: (f32, f32),
    cols: usize,
    area: (f32, f32),
    pitch: (f32, f32),
    captions: bool,
) {
    let spin = g.spin_deg(ui);
    for (i, kind) in Kind::ALL.into_iter().enumerate() {
        let (cx, cy) = (
            origin.0 + (i % cols) as f32 * pitch.0,
            origin.1 + (i / cols) as f32 * pitch.1,
        );
        let id = ui.id((scope, kind_name(kind)));
        let (v, transforms) = if kind == Kind::Spin {
            (1.0, true)
        } else {
            let on = g.on(ui, kind);
            drive(ui, id, kind, tier, on)
        };
        swatch(
            ui.painter(),
            Rect::new(cx, cy, area.0, area.1),
            kind,
            v,
            transforms,
            spin,
        );
        let y = cy + area.1 + 4.0;
        label(ui, kind_name(kind), cx, y, THEME.font_s, THEME.fg_primary);
        if captions {
            let m = Motion::of(kind);
            let text = if kind == Kind::Spin {
                "1200 ms, linear".to_owned()
            } else {
                let ms = MOTION.duration(m.token).as_millis();
                format!("{} {ms} ms, {}", token_name(m.token), easing_name(m.easing))
            };
            label(ui, &text, cx, y + 14.0, THEME.font_s, THEME.fg_tertiary);
        }
    }
}

/// The kinds page: all fifteen swatches.
struct Kinds(Grid);

impl PageState for Kinds {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        self.0.frames += 1;
        let card = card(ui, "Motion: kinds");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "KINDS", x, y);
        let tier = ui.tier();
        kinds_grid(
            ui,
            &self.0,
            "kinds",
            tier,
            (x, y + 30.0),
            5,
            (64.0, 56.0),
            (190.0, 112.0),
            true,
        );
        self.0.keep_alive(ui);
    }
}

// ---------------------------------------------------------------------------------------------
// Retarget

/// `move` (180 ms, sine-in-out) retargeted back at 60 ms, drawn at 120 ms: a plot of the path
/// beside the live swatch. The path leaves the retarget point with its position and speed.
struct Retarget {
    frames: u32,
}

/// When the reversal happens, in ms after the start.
const REVERSE_MS: u64 = 60;

impl PageState for Retarget {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        self.frames += 1;
        let card = card(ui, "Motion: retarget");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "MOVE, RETARGETED AT 60 MS OF ITS 180 MS RUN", x, y);
        let now = ui.time();
        let started = self.frames >= 2;
        let target = if started && now < Duration::from_millis(REVERSE_MS) {
            1.0
        } else {
            0.0
        };
        let motion = Motion::of(Kind::Move);
        let live = ui.animate_f32(ui.id("move"), target, &motion);

        // The path, from the same closed form the Ui uses: out and back, and the run that was
        // not interrupted for comparison.
        let run = MOTION.duration(motion.token);
        let mut full = Transition::settled(0.0f32);
        full.retarget(Duration::ZERO, 1.0, run, motion.easing);
        let mut cut = full;
        cut.retarget(Duration::from_millis(REVERSE_MS), 0.0, run, motion.easing);

        // Before the reversal the run is the uninterrupted one; after it, the retargeted one.
        let value = |t: Duration| {
            if t < Duration::from_millis(REVERSE_MS) {
                full.sample(t).0
            } else {
                cut.sample(t).0
            }
        };
        let area = Rect::new(x, y + 30.0, 360.0, 200.0);
        p_plot_frame(ui, area);
        let at = |t: Duration, v: f32| {
            let u = t.as_secs_f32() / 0.24;
            [
                area.x + u * area.w,
                area.bottom() - 8.0 - v * (area.h - 16.0),
            ]
        };
        let n = 48;
        let ghost: Vec<[f32; 2]> = (0..=n)
            .map(|i| {
                let t = Duration::from_micros(240_000 * i / n);
                at(t, full.sample(t).0)
            })
            .collect();
        let shown = now.min(Duration::from_millis(240));
        let path: Vec<[f32; 2]> = (0..=n)
            .map(|i| {
                let t = shown.mul_f32(i as f32 / n as f32);
                at(t, value(t))
            })
            .collect();
        let p = ui.painter();
        p.stroke(&ghost, false, StrokeStyle::solid(THEME.fg_faint, 1.0));
        p.stroke(&path, false, StrokeStyle::solid(THEME.accent, 2.0));
        let r = at(
            Duration::from_millis(REVERSE_MS),
            value(Duration::from_millis(REVERSE_MS)),
        );
        p.fill(
            Rect::new(r[0] - 3.0, r[1] - 3.0, 6.0, 6.0),
            3.0,
            THEME.fg_primary,
        );
        let c = at(shown, value(shown));
        p.fill(
            Rect::new(c[0] - 4.0, c[1] - 4.0, 8.0, 8.0),
            4.0,
            THEME.accent,
        );
        label(
            ui,
            "reversed here",
            r[0] + 8.0,
            r[1] - 18.0,
            THEME.font_s,
            THEME.fg_secondary,
        );
        label(
            ui,
            "uninterrupted run",
            area.x + 8.0,
            area.y + 6.0,
            THEME.font_s,
            THEME.fg_faint,
        );

        // The live swatch: a dot between two stops, driven by `animate_f32`.
        let track = Rect::new(x + 400.0, y + 30.0 + 90.0, 240.0, 20.0);
        let p = ui.painter();
        p.fill(track, 10.0, THEME.surface_selected);
        p.fill(
            Rect::new(
                lerp(track.x + 2.0, track.right() - 18.0, live),
                track.y + 2.0,
                16.0,
                16.0,
            ),
            8.0,
            THEME.accent,
        );
        label(
            ui,
            "live (Ui::animate_f32)",
            track.x,
            track.bottom() + 8.0,
            THEME.font_s,
            THEME.fg_secondary,
        );
        caption(
            ui,
            "The path leaves the reversal point at the speed it arrived with: no jump, no restart.",
            x,
            y + 250.0,
        );
        if started && now < Duration::from_millis(240) {
            ui.request_repaint_at(now + Duration::from_millis(16));
        }
    }
}

fn p_plot_frame(ui: &mut Ui<'_>, area: Rect) {
    ui.painter()
        .ring(area, 4.0, 1.0, THEME.border_subtle, false);
}

// ---------------------------------------------------------------------------------------------
// Tiers

/// The kind swatches three times, one panel per tier.
struct Tiers(Grid);

const PANEL_COLS: usize = 3;
const PANEL_PITCH: (f32, f32) = (96.0, 62.0);
const PANEL_AREA: (f32, f32) = (80.0, 40.0);
/// Where a panel's swatches start inside the page card, per panel index.
const PANEL_PAGE_X: f32 = 320.0;

/// One tier's panel of swatches at `origin` (no heading, so a panel is comparable on its own).
fn panel(ui: &mut Ui<'_>, g: &Grid, tier: MotionTier, origin: (f32, f32)) {
    kinds_grid(
        ui,
        g,
        tier_name(tier),
        tier,
        origin,
        PANEL_COLS,
        PANEL_AREA,
        PANEL_PITCH,
        false,
    );
}

fn tier_name(t: MotionTier) -> &'static str {
    match t {
        MotionTier::Full => "full",
        MotionTier::Reduced => "reduced",
        MotionTier::Off => "off",
    }
}

impl PageState for Tiers {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        self.0.frames += 1;
        let card = card(ui, "Motion: tiers");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        heading(ui, "FULL, REDUCED, OFF", x, y);
        for (i, tier) in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off]
            .into_iter()
            .enumerate()
        {
            let px = x + i as f32 * PANEL_PAGE_X;
            label(
                ui,
                tier_name(tier),
                px,
                y + 24.0,
                THEME.font_base,
                THEME.fg_primary,
            );
            panel(ui, &self.0, tier, (px, y + 52.0));
        }
        self.0.keep_alive(ui);
    }
}

// ---------------------------------------------------------------------------------------------
// Window mode

/// Everything, looping: the easings and the kinds, flipping every 1.2 s.
struct Live(Grid);

fn live() -> Box<dyn PageState> {
    Box::new(Live(Grid {
        drive: Drive::Loop,
        frames: 0,
    }))
}

impl PageState for Live {
    fn draw(&mut self, ui: &mut Ui<'_>) {
        self.0.frames += 1;
        let card = card(ui, "Motion");
        let (x, y) = (card.x + 24.0, card.y + 24.0);
        easings_row(ui, x, y);
        let ky = y + 168.0;
        heading(ui, "KINDS (FLIP EVERY 1.2 S)", x, ky);
        let tier = ui.tier();
        kinds_grid(
            ui,
            &self.0,
            "kinds",
            tier,
            (x, ky + 26.0),
            5,
            (64.0, 48.0),
            (190.0, 100.0),
            false,
        );
        self.0.keep_alive(ui);
    }
}

// ---------------------------------------------------------------------------------------------
// Scenes

fn grid(drive: Drive) -> Grid {
    Grid { drive, frames: 0 }
}

fn easings() -> Box<dyn PageState> {
    Box::new(Easings)
}

fn curve_cubic_out() -> Box<dyn PageState> {
    Box::new(Curve(Easing::CUBIC_OUT))
}

fn curve_ease_in() -> Box<dyn PageState> {
    Box::new(Curve(Easing::EASE_IN))
}

fn kinds_halfway() -> Box<dyn PageState> {
    Box::new(Kinds(grid(Drive::HalfwayAt(HALFWAY_MS))))
}

fn kinds_settled() -> Box<dyn PageState> {
    Box::new(Kinds(grid(Drive::Settled)))
}

fn retarget() -> Box<dyn PageState> {
    Box::new(Retarget { frames: 0 })
}

fn tiers_moving() -> Box<dyn PageState> {
    Box::new(Tiers(grid(Drive::Start)))
}

fn tiers_settled() -> Box<dyn PageState> {
    Box::new(Tiers(grid(Drive::Settled)))
}

const fn at(ms: u32) -> Step {
    Step {
        at_ms: ms,
        events: &[],
    }
}

/// A frame at each distinct start time of `Drive::HalfwayAt(300)`: 300 minus half of 120, 180,
/// 250 and 350 ms.
const KIND_STARTS: &[Step] = &[at(125), at(175), at(210), at(240)];

const SCENES: &[UiScene] = &[
    UiScene {
        id: "motion-easings",
        scale: 2,
        tier: MotionTier::Full,
        state: easings,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "motion-curve-cubic-out",
        scale: 2,
        tier: MotionTier::Full,
        state: curve_cubic_out,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    UiScene {
        id: "motion-curve-ease-in",
        scale: 2,
        tier: MotionTier::Full,
        state: curve_ease_in,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // Each swatch is at half its own default duration at 300 ms; spin shows 300 ms.
    UiScene {
        id: "motion-kinds",
        scale: 2,
        tier: MotionTier::Full,
        state: kinds_halfway,
        script: KIND_STARTS,
        capture_ms: HALFWAY_MS,
        expect: Expect::Moving,
    },
    UiScene {
        id: "motion-kinds-settled",
        scale: 2,
        tier: MotionTier::Full,
        state: kinds_settled,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
    // The run starts at 0 ms (the second frame), reverses at 60 ms, is drawn at 120 ms.
    UiScene {
        id: "motion-retarget",
        scale: 2,
        tier: MotionTier::Full,
        state: retarget,
        script: &[at(0), at(60)],
        capture_ms: 120,
        expect: Expect::Moving,
    },
    // Every change starts at 0 ms and is drawn 90 ms in (half of `fast`).
    UiScene {
        id: "motion-tiers",
        scale: 2,
        tier: MotionTier::Full,
        state: tiers_moving,
        script: &[at(0)],
        capture_ms: 90,
        expect: Expect::Moving,
    },
    UiScene {
        id: "motion-tiers-settled",
        scale: 2,
        tier: MotionTier::Full,
        state: tiers_settled,
        script: &[],
        capture_ms: 0,
        expect: Expect::Settled,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::Player;
    use crate::scenes::{H, W};
    use lune_ui::scene::Scene;
    use lune_ui::testing::FakeText;

    /// One tier's panel alone, at a fixed origin.
    struct Alone(Grid, MotionTier);

    impl PageState for Alone {
        fn draw(&mut self, ui: &mut Ui<'_>) {
            self.0.frames += 1;
            panel(ui, &self.0, self.1, (40.0, 40.0));
        }
    }

    fn settled_panel(tier: MotionTier) -> Scene {
        let state = Box::new(Alone(grid(Drive::Settled), tier));
        let mut player = Player::new("motion", state, (W as f32, H as f32), 2.0, MotionTier::Full);
        player.frame(&mut FakeText, 0, &[]).expect("frame 0");
        player.frame(&mut FakeText, 1000, &[]).expect("settled").0
    }

    #[test]
    fn a_tier_never_changes_the_resting_look() {
        let full = settled_panel(MotionTier::Full);
        assert_eq!(full, settled_panel(MotionTier::Reduced), "Reduced");
        assert_eq!(full, settled_panel(MotionTier::Off), "Off");
    }

    #[test]
    fn the_scripted_start_times_are_the_halfway_starts() {
        let mut starts: Vec<u32> = Kind::ALL
            .into_iter()
            .filter(|k| *k != Kind::Spin)
            .map(|k| start_ms(k, HALFWAY_MS))
            .collect();
        starts.sort_unstable();
        starts.dedup();
        let scripted: Vec<u32> = KIND_STARTS.iter().map(|s| s.at_ms).collect();
        assert_eq!(starts, scripted);
        assert!(
            starts.iter().all(|s| *s > 0),
            "a start at 0 would not animate"
        );
    }

    #[test]
    fn the_tiers_differ_while_moving() {
        let scene = SCENES
            .iter()
            .find(|s| s.id == "motion-tiers")
            .expect("scene");
        let (moving, _) = crate::page::run_scene("motion", scene, &mut FakeText).expect("runs");
        let settled = SCENES
            .iter()
            .find(|s| s.id == "motion-tiers-settled")
            .expect("scene");
        let (rest, _) = crate::page::run_scene("motion", settled, &mut FakeText).expect("runs");
        assert_ne!(moving, rest);
    }
}

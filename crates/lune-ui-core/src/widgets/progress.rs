//! A progress track (`data.progress`): 4 px high, radius 4, `surface_overlay` track, a fill in a
//! tone (neutral is `accent`; square at its leading edge, clipped by the track), an optional
//! head (name left, value right, 13 on a 20 px line, 4 px below). Determinate never animates (a progress value updates continuously, no tween).
//! Indeterminate (D118) is a 3 px track: a segment 30% of the track travelling left to right,
//! linear, period 2400 ms, clipped by the track. Under Reduced the full-width fill breathes
//! between 0.38 and 1.0 over 1800 ms (opacity only); under Off it holds at 0.38.

use std::hash::Hash;
use std::time::Duration;

use super::tone::Tone;
use crate::access::{Node, Role, Value};
use crate::color::{Rgba, with_alpha};
use crate::motion::{MotionTier, Time};
use crate::paint::{Painter, Rect};
use crate::text::{Font, Spec, Style};
use crate::theme::Theme;
use crate::ui::Ui;

/// The determinate track's height.
pub const HEIGHT: f32 = 4.0;
/// The indeterminate track's height (thinner than a measured bar).
pub const HEIGHT_INDETERMINATE: f32 = 3.0;
/// The track's corner radius: half the height, a semicircular end (D135).
pub const RADIUS: f32 = HEIGHT / 2.0;
/// The indeterminate track's corner radius, half its own height.
pub const RADIUS_INDETERMINATE: f32 = HEIGHT_INDETERMINATE / 2.0;
/// The head's line height.
pub const HEAD_LINE: f32 = 20.0;
/// The gap below the head.
pub const HEAD_GAP: f32 = 4.0;
/// The travelling segment's share of the track.
pub const SEGMENT: f32 = 0.3;
/// One trip of the segment: the accepted busy-loop period, outside the motion scale.
pub const PERIOD: Duration = Duration::from_millis(2400);
/// One breath of the Reduced fill.
pub const BREATH_PERIOD: Duration = Duration::from_millis(1800);
/// The lowest opacity of the breath, and the static opacity under Off.
pub const BREATH_LOW: f32 = 0.38;

/// What a progress bar shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressOptions<'a> {
    /// The fraction 0 to 1, clamped (NaN is 0); `None` is indeterminate.
    pub value: Option<f32>,
    /// The fill's tone; neutral is `accent`.
    pub tone: Tone,
    /// The head: the name at the left and the value text at the right (in the mono face).
    pub head: Option<(&'a str, &'a str)>,
    /// The accessible name; empty falls back to the head's name.
    pub name: &'a str,
}

impl Default for ProgressOptions<'_> {
    fn default() -> Self {
        Self {
            value: Some(0.0),
            tone: Tone::Neutral,
            head: None,
            name: "",
        }
    }
}

/// How an indeterminate bar is drawn at one instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Indeterminate {
    /// The segment travels: 0 is just off the left end, 1 just off the right end.
    Travel(f32),
    /// The full-width fill at this opacity (Reduced breathes, Off holds).
    Fade(f32),
}

/// The fill colour of `tone`: neutral is `accent`.
pub fn fill_color(theme: &Theme, tone: Tone) -> Rgba {
    match tone {
        Tone::Neutral => theme.accent,
        t => t.color(theme),
    }
}

/// `fraction` as a clamped 0 to 1 value; NaN is 0.
pub fn clamp(fraction: f32) -> f32 {
    if fraction.is_nan() {
        0.0
    } else {
        fraction.clamp(0.0, 1.0)
    }
}

/// The segment's position at `t`, closed in time: `frac(t / 2400 ms)`.
pub fn travel_phase(t: Time) -> f32 {
    let into = t.as_nanos() % PERIOD.as_nanos();
    (into as f64 / PERIOD.as_nanos() as f64) as f32
}

/// The Reduced fill's opacity at `t`: 0.38 to 1.0 and back over 1800 ms, sine in and out.
pub fn breath_opacity(t: Time) -> f32 {
    let ph = (t.as_nanos() % BREATH_PERIOD.as_nanos()) as f64 / BREATH_PERIOD.as_nanos() as f64;
    let tri = if ph < 0.5 { ph * 2.0 } else { (1.0 - ph) * 2.0 };
    let eased = 0.5 - 0.5 * (std::f64::consts::PI * tri).cos();
    BREATH_LOW + eased as f32 * (1.0 - BREATH_LOW)
}

/// What an indeterminate bar draws under `tier` at `t`.
pub fn indeterminate_at(tier: MotionTier, t: Time) -> Indeterminate {
    match tier {
        MotionTier::Full => Indeterminate::Travel(travel_phase(t)),
        MotionTier::Reduced => Indeterminate::Fade(breath_opacity(t)),
        MotionTier::Off => Indeterminate::Fade(BREATH_LOW),
    }
}

/// Draws a determinate bar in `track` at `fraction` (clamped): the track, then the fill from the
/// left, whose width is the fraction of the track's. No fill op at 0. The ends are semicircles of
/// the track's own height (4 px gives `RADIUS`). Public for the stat meter.
pub fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, track: Rect, fraction: f32, tone: Tone) {
    let theme = p.theme;
    let radius = track.h / 2.0;
    p.fill(track, radius, theme.surface_overlay);
    let f = clamp(fraction);
    if f > 0.0 {
        // Square leading edge, clipped by the rounded track (a full bar keeps the track's ends).
        let fill = Rect::new(track.x, track.y, track.w * f, track.h);
        p.push_clip_rounded(track, radius);
        p.fill(fill, 0.0, fill_color(&theme, tone));
        p.pop_clip();
    }
}

/// Draws an indeterminate bar in `track`: the track, then the segment clipped by it, or the
/// faded full fill.
pub fn paint_indeterminate<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    track: Rect,
    tone: Tone,
    how: Indeterminate,
) {
    let theme = p.theme;
    p.fill(track, RADIUS_INDETERMINATE, theme.surface_overlay);
    let color = fill_color(&theme, tone);
    match how {
        Indeterminate::Travel(phase) => {
            let seg = track.w * SEGMENT;
            let x = track.x - seg + (track.w + seg) * phase.clamp(0.0, 1.0);
            p.push_clip_rounded(track, RADIUS_INDETERMINATE);
            p.fill(
                Rect::new(x, track.y, seg, track.h),
                RADIUS_INDETERMINATE,
                color,
            );
            p.pop_clip();
        }
        Indeterminate::Fade(opacity) => {
            p.fill(track, RADIUS_INDETERMINATE, with_alpha(color, opacity));
        }
    }
}

impl Ui<'_> {
    /// A progress bar under `key`, the top left and width of `rect` (its height is ignored). The
    /// head, when set, takes its line first. Returns the height drawn. The node is `Progress`
    /// valued 0 to 100 (none when indeterminate), named by `opts.name` or the head's name.
    pub fn progress(&mut self, key: impl Hash, rect: Rect, opts: &ProgressOptions<'_>) -> f32 {
        self.progress_sized(key, rect, opts, HEIGHT)
    }

    /// `progress` with the determinate track `determinate_h` high (the toast's bar is 3 px in
    /// both states).
    pub(super) fn progress_sized(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &ProgressOptions<'_>,
        determinate_h: f32,
    ) -> f32 {
        let id = self.id(key);
        let theme = *self.theme();
        let mut y = rect.y;
        if let Some((name, value)) = opts.head {
            let name_style = Style {
                line: HEAD_LINE,
                ..Style::ui(theme.font_base, theme.fg_secondary)
            };
            let value_style = Style {
                font: Font::Mono,
                color: theme.fg_primary,
                ..name_style
            };
            let p = self.painter();
            p.text(&Spec::plain(name, name_style), rect.x, y);
            let w = p.measure(&Spec::plain(value, value_style)).0;
            p.text(&Spec::plain(value, value_style), rect.right() - w, y);
            y += HEAD_LINE + HEAD_GAP;
        }
        let track_h = if opts.value.is_some() {
            determinate_h
        } else {
            HEIGHT_INDETERMINATE
        };
        let track = Rect::new(rect.x, y, rect.w, track_h);
        let mut node = Node::new(
            id,
            Role::Progress,
            if opts.name.is_empty() {
                opts.head.map_or("", |h| h.0)
            } else {
                opts.name
            },
            Rect::new(rect.x, rect.y, rect.w, y + track_h - rect.y),
        );
        match opts.value {
            Some(v) => {
                paint(self.painter(), track, v, opts.tone);
                node.value = Some(Value::Number {
                    value: f64::from(clamp(v) * 100.0),
                    min: 0.0,
                    max: 100.0,
                    step: 0.0,
                });
            }
            None => {
                let (tier, now) = (self.tier(), self.time());
                paint_indeterminate(
                    self.painter(),
                    track,
                    opts.tone,
                    indeterminate_at(tier, now),
                );
                if tier != MotionTier::Off {
                    self.mark_moving();
                }
            }
        }
        self.node(node);
        y + track_h - rect.y
    }
}

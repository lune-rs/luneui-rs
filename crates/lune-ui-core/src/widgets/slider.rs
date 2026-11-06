//! A horizontal value slider (`widget.slider`): a 4 px track, an accent fill up to the value and a
//! 12 px knob (14 hovered) whose centre sits at the value's position. Built on the layers of the
//! button (see `button.rs`): a [`SliderLook`] and its settled values, [`paint`] and [`Ui::slider`].
//!
//! The value is the app's; the widget writes it only when the user acts (D119): a press anywhere
//! in the hit area sets it and starts following, the keys and the accessibility steps move it one
//! stop, a set-value request jumps. Every written value lies in `[min, max]` on a stop. The stops
//! are `min + k * step` and `max`, which stays reachable when `step` does not divide the range. A
//! `step` of 0 or less is continuous, with a key step of 1/100 of the range.
//!
//! Only a key or an accessibility step tweens the knob. It follows the pointer exactly, and any
//! other change (the app's, a set-value) snaps.

use std::hash::Hash;

use super::focus_ring::{self, RingKind};
use crate::access::{AccessValue, Action, Node, Role, State, Value};
use crate::color::Rgba;
use crate::input::{FocusEvent, Key, NamedKey};
use crate::motion::{Easing, Kind, Motion, Reduce};
use crate::paint::{Painter, Rect};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui};

/// The track's height.
pub const HEIGHT: f32 = 4.0;
/// The track's and the fill's corner radius: half the height, a semicircular end. A CSS radius
/// larger than that clamps to it; `sd_round_rect` does not, so the value is the clamped one (D135).
pub const RADIUS: f32 = HEIGHT / 2.0;
/// Knob position after a key or an accessibility step. Snaps under Reduced.
pub const VALUE: Motion = Motion {
    easing: Easing::SineInOut,
    ..Motion::of(Kind::Value)
};
/// Knob size 12 to 14 on hover: a size, so it snaps under Reduced.
pub const HOVER_SIZE: Motion = Motion {
    reduced: Reduce::Snap,
    ..Motion::of(Kind::Hover)
};
/// Focus ring opacity.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// A press captures at once and the value follows it, so the hit region senses both.
const SENSE: Sense = Sense {
    focusable: true,
    ..Sense::DRAG
};

/// What a slider ranges over. The widget treats a range it cannot use (`min >= max`, anything not
/// finite) as inert: fraction 0, no change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderOptions<'a> {
    pub min: f32,
    pub max: f32,
    /// The distance between stops from `min`; 0 or less is continuous.
    pub step: f32,
    /// The accessible name.
    pub name: &'a str,
}

/// What a slider shows and what the input is doing.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SliderState {
    /// The value's place on the track, 0 to 1.
    pub fraction: f32,
    pub hovered: bool,
    pub focus_visible: bool,
}

/// The continuous look of a slider.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderLook {
    pub track: Rgba,
    pub fill: Rgba,
    pub knob: Rgba,
    pub knob_size: f32,
    /// The knob centre's place on the track: 0 start, 1 end.
    pub fraction: f32,
    /// Focus ring opacity.
    pub focus: f32,
}

impl SliderLook {
    /// The look `state` settles to.
    pub fn of(theme: &Theme, state: SliderState) -> SliderLook {
        SliderLook {
            track: theme.surface_overlay,
            fill: theme.accent,
            knob: theme.on_accent,
            knob_size: if state.hovered { 14.0 } else { 12.0 },
            fraction: state.fraction,
            focus: if state.focus_visible { 1.0 } else { 0.0 },
        }
    }
}

/// The track: the full width of `r`, vertically centred in it.
pub fn track(r: Rect) -> Rect {
    Rect::new(r.x, r.y + (r.h - HEIGHT) / 2.0, r.w, HEIGHT)
}

/// The hit area: the track grown 4 px left and right and 6 px up and down.
pub fn hit_rect(r: Rect) -> Rect {
    track(r).inset(-4.0, -6.0)
}

/// The knob of `look` on the track of `r`: centred on the track at the value's place.
pub fn knob_rect(r: Rect, look: &SliderLook) -> Rect {
    let t = track(r);
    let size = look.knob_size;
    Rect::new(
        t.x + t.w * look.fraction - size / 2.0,
        t.y + (t.h - size) / 2.0,
        size,
        size,
    )
}

/// Draws a slider in `r` with `look`.
pub fn paint<H: Copy + PartialEq>(p: &mut Painter<'_, H>, r: Rect, look: &SliderLook) {
    let t = track(r);
    if t.w > 0.0 {
        p.fill(t, RADIUS, look.track);
        if look.fraction > 0.0 {
            let fill = Rect::new(t.x, t.y, t.w * look.fraction, t.h);
            p.fill(fill, RADIUS, look.fill);
        }
    }
    let knob = knob_rect(r, look);
    p.fill(knob, look.knob_size / 2.0, look.knob);
    focus_ring::paint(
        p,
        knob,
        RingKind::Offset {
            radius: look.knob_size / 2.0,
        },
        look.focus,
    );
}

/// A usable range and its stops, in `f64` so `max - min` cannot overflow.
#[derive(Debug, Clone, Copy)]
struct Stops {
    min: f64,
    max: f64,
    /// 0 is continuous.
    step: f64,
    /// The index of `max`, the last stop.
    last: f64,
    /// How far an `f32` near the range's ends can sit from what it stands for.
    slack: f64,
}

impl Stops {
    /// `None` for a range the slider cannot use.
    fn of(opts: &SliderOptions<'_>) -> Option<Stops> {
        if !(opts.min.is_finite() && opts.max.is_finite() && opts.step.is_finite()) {
            return None;
        }
        if opts.min >= opts.max {
            return None;
        }
        let (min, max) = (f64::from(opts.min), f64::from(opts.max));
        let step = f64::from(opts.step.max(0.0));
        let slack = min.abs().max(max.abs()) * f64::from(f32::EPSILON);
        let last = if step > 0.0 {
            let n = (max - min) / step;
            // A step that divides the range only up to `f32` rounding adds no stop just below max.
            let k = if (n - n.round()).abs() * step <= slack {
                n.round()
            } else {
                n.ceil()
            };
            k.max(1.0)
        } else {
            1.0
        };
        Some(Stops {
            min,
            max,
            step,
            last,
            slack,
        })
    }

    /// Stop `k`.
    fn at(&self, k: f64) -> f64 {
        if k >= self.last {
            self.max
        } else {
            (self.min + k * self.step).min(self.max)
        }
    }

    /// The stop nearest `x`, which may lie outside the range. Not NaN.
    fn snap(&self, x: f64) -> f32 {
        let x = x.clamp(self.min, self.max);
        if self.step <= 0.0 {
            return x as f32;
        }
        let k = ((x - self.min) / self.step)
            .floor()
            .clamp(0.0, self.last - 1.0);
        let (below, above) = (self.at(k), self.at(k + 1.0));
        let nearest = if above - x <= x - below { above } else { below };
        nearest as f32
    }

    /// One key step from `v`: the next stop up or down, so an off-step value lands on the stop
    /// beside it.
    fn next(&self, v: f32, up: bool) -> f32 {
        let x = f64::from(v).clamp(self.min, self.max);
        if self.step <= 0.0 {
            let d = (self.max - self.min) / 100.0;
            return (if up { x + d } else { x - d }).clamp(self.min, self.max) as f32;
        }
        let i = (x - self.min) / self.step;
        let on_stop = (i - i.round()).abs() * self.step <= self.slack;
        let k = match (on_stop, up) {
            (true, true) => i.round() + 1.0,
            (true, false) => i.round() - 1.0,
            (false, true) => i.ceil(),
            (false, false) => i.floor(),
        };
        self.at(k.clamp(0.0, self.last)) as f32
    }

    fn fraction(&self, v: f32) -> f32 {
        ((f64::from(v) - self.min) / (self.max - self.min)).clamp(0.0, 1.0) as f32
    }
}

/// The stops of a slider in `rect` holding `value`; `None` when it is inert.
fn usable(rect: Rect, value: f32, opts: &SliderOptions<'_>) -> Option<Stops> {
    Stops::of(opts).filter(|_| rect.w > 0.0 && value.is_finite())
}

/// The place of `value` on a slider in `rect`, 0 to 1: 0 when the slider is inert.
pub fn fraction(rect: Rect, value: f32, opts: &SliderOptions<'_>) -> f32 {
    usable(rect, value, opts).map_or(0.0, |s| s.fraction(value))
}

/// The fraction the knob last aimed at: what tells an app-driven change from the widget's own.
#[derive(Debug, Default)]
struct Aimed(f32);

impl Ui<'_> {
    /// A slider under `key` in `rect`, holding `value` per `opts`. A press sets the value from
    /// where it lands and following frames track the pointer until the release; Right and Up step
    /// up, Left and Down down, as do the increment and decrement actions; set-value jumps. Marks
    /// the response `changed` when it wrote a different value. While the pointer holds the slider
    /// it alone moves the value: keys and actions are dropped.
    pub fn slider(
        &mut self,
        key: impl Hash,
        rect: Rect,
        value: &mut f32,
        opts: &SliderOptions<'_>,
    ) -> Response {
        let id = self.id(key);
        let pos = id.with("pos");
        let mut r = self.interact(id, hit_rect(rect), SENSE);

        // A value that is not the one this widget left behind is the app's: it snaps, first, so a
        // key step in the same frame starts from it.
        let held = fraction(rect, *value, opts);
        if self.transient::<Aimed>(id).0 != held {
            self.snap_f32(pos, held);
        }

        let stops = usable(rect, *value, opts);
        // The release frame counts: the event carries the pointer's last move.
        let following = r.pressed || r.press_started || r.clicked || r.drag_stopped;
        let mut next = *value;
        if following {
            let at = if r.press_started {
                r.press_pos
            } else {
                self.pointer_pos()
            };
            // Through the transform stack: `r.rect` is cut by the clip, the track is not.
            let t = self.painter().transformed(track(rect));
            if let (Some(s), Some((x, _))) = (stops, at)
                && t.w > 0.0
            {
                let f = f64::from((x - t.x) / t.w).clamp(0.0, 1.0);
                next = s.snap(s.min + f * (s.max - s.min));
            }
        }
        // The arrows are taken whether or not the slider is enabled (D140); a Control, Alt or
        // Command chord on them is left to the shortcuts (D157); only an enabled slider acts.
        for (index, event) in self.focus_events(id) {
            let FocusEvent::Key {
                key: Key::Named(k),
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            if modifiers.control || modifiers.alt || modifiers.logo {
                continue;
            }
            let up = match k {
                NamedKey::ArrowRight | NamedKey::ArrowUp => true,
                NamedKey::ArrowLeft | NamedKey::ArrowDown => false,
                _ => continue,
            };
            self.consume_focus_event(index);
            if let (true, Some(s), false) = (r.enabled, stops, following) {
                next = s.next(next, up);
            }
        }
        let mut jumped = following;
        if let (Some(AccessValue::Number(v)), Some(s), false) = (&r.set_value, stops, following)
            && !v.is_nan()
        {
            next = s.snap(*v);
            jumped = true;
        }

        if stops.is_some() && next != *value {
            *value = next;
            r.mark_changed();
            // Whatever the frame drew before this call showed the old value.
            self.request_catch_up();
        }
        let aim = fraction(rect, *value, opts);
        self.transient::<Aimed>(id).0 = aim;

        let state = SliderState {
            fraction: aim,
            hovered: r.hovered || r.pressed,
            focus_visible: r.focus_visible,
        };
        let target = SliderLook::of(self.theme(), state);
        let look = SliderLook {
            fraction: if jumped {
                self.snap_f32(pos, aim)
            } else {
                self.animate_f32(pos, aim, &VALUE)
            },
            knob_size: self.animate_f32(id.with("size"), target.knob_size, &HOVER_SIZE),
            focus: self.animate_f32(id.with("focus"), target.focus, &FOCUS),
            ..target
        };
        paint(self.painter(), rect, &look);

        // A node's numbers are compared and serialised: nothing not finite goes in.
        let number = |v: f32| if v.is_finite() { f64::from(v) } else { 0.0 };
        let mut node = Node::new(id, Role::Slider, opts.name, track(rect));
        node.value = Some(Value::Number {
            value: number(*value),
            min: number(opts.min),
            max: number(opts.max),
            step: number(opts.step),
        });
        if r.enabled {
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Increment);
            node.actions.insert(Action::Decrement);
            node.actions.insert(Action::SetValue);
        } else {
            node.states.insert(State::Disabled);
        }
        self.node(node);
        r
    }
}

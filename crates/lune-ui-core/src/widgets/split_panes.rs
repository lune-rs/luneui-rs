//! Resizable split panes (`layout.split-panes`, D150): two panes in a rect with a draggable,
//! keyboard-operable strip between them. [`Ui::split_panes`] lays the panes out, runs the app's
//! closure once per pane and draws the strip last, so its hit is on top.
//!
//! One pane is the *sized* one ([`SplitOptions::sized`]): its size along the axis is the value the
//! user changes and the app stores; the other takes what is left. The strip is a 4 px gutter
//! between them with an 8 px hit (2 px into each pane), transparent at rest and with no hover look;
//! only focus-visible fills it with `accent`.
//!
//! The size follows the pointer at once (no drag threshold): size = the size at the press plus the
//! travel, clamped to `[min, max]` and to the room that leaves the other pane its `min_other`.
//! When both cannot fit, `min` wins. The stored size changes only on user input or the app's own
//! write ([`Ui::split_state`]); a smaller window clamps what is *drawn*, not what is stored, so the
//! size comes back when the window grows. Nothing here is animated: the size snaps in every tier
//! (layout on a large surface), and only the focus fill fades.
//!
//! Keys, on the strip as the one tab stop: the arrows along the axis move the strip 16 px (Left and
//! Right for [`Axis::Horizontal`], Up and Down for [`Axis::Vertical`]); Home and End set the sized
//! pane to its smallest and largest size; Escape during a drag restores the size at the press and
//! is consumed. For [`Axis::Horizontal`], Up and Down grow and shrink the sized pane by 16. A
//! Control, Alt or Command chord on the arrows, Home and End stays a shortcut. An accessibility
//! increment and decrement raise and lower the value, the sized pane, by 16 on both axes and both sides,
//! whichever way that moves the strip.

use std::hash::Hash;

use crate::access::{AccessValue, Action, Node, Role, State, Value};
use crate::input::{CursorIcon, FocusEvent, Key, NamedKey};
use crate::layout::Axis;
use crate::motion::{Kind, Motion};
use crate::paint::Rect;
use crate::ui::{Response, Sense, Ui};

/// The painted strip's thickness, and the gutter between the panes.
pub const STRIP: f32 = 4.0;
/// The hit strip's thickness: the paint grown 2 px on each side.
pub const HIT: f32 = 8.0;
/// What an arrow key moves the strip by.
pub const KEY_STEP: f32 = 16.0;
/// Focus-visible fill opacity.
pub const FOCUS: Motion = Motion::of(Kind::Focus);

/// A press captures at once and the size follows it, so the hit region senses both.
const SENSE: Sense = Sense {
    focusable: true,
    ..Sense::DRAG
};

/// Which pane [`SplitOptions::sized`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitSide {
    /// The pane at the start of the axis (left, or top).
    First,
    /// The pane at the end of the axis (right, or bottom). It grows as the strip moves toward the
    /// start.
    Second,
}

/// A pane, as the draw closure is told which one it draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    First,
    Second,
}

/// What a split is and how far it may be dragged. Sizes are along the axis, in logical px. A bound
/// that is NaN or not usable counts as no bound (`min` and `min_other` as 0, `max` as the room).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitOptions<'a> {
    /// [`Axis::Horizontal`]: two columns side by side, the strip vertical. [`Axis::Vertical`]: two
    /// rows, the strip horizontal.
    pub axis: Axis,
    /// The pane whose size is the value.
    pub sized: SplitSide,
    /// The sized pane's size until the app or the user sets one.
    pub default: f32,
    /// The sized pane's smallest size. It wins over `max` and `min_other`.
    pub min: f32,
    /// The sized pane's largest size.
    pub max: f32,
    /// The room the other pane keeps, taken from the sized pane's largest size.
    pub min_other: f32,
    /// The sized pane is hidden: the other takes the whole rect, with no strip and no node. Snaps
    /// in every tier.
    pub hidden: bool,
    /// The accessible name of the strip.
    pub name: &'a str,
}

/// A split's persistent state, owned by the toolkit and readable and writable by the app
/// ([`Ui::split_state`]) so it can load and store the size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SplitState {
    /// The sized pane's stored size; `None` until set, then [`SplitOptions::default`] holds. What
    /// is drawn is this clamped to the current room.
    pub size: Option<f32>,
}

/// What [`Ui::split_panes`] reports.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitResponse {
    /// The strip's response; inert when the split is hidden.
    pub response: Response,
    /// The sized pane's size now, as drawn (after this frame's input).
    pub size: f32,
    /// This frame's input changed it.
    pub changed: bool,
}

/// The pieces of a split in window space (the space of the rect it was given).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitGeometry {
    pub first: Rect,
    pub second: Rect,
    /// The painted strip.
    pub strip: Rect,
    /// The hit strip, cut to the split's rect.
    pub hit: Rect,
}

/// The effective `(min, max)` of the sized pane in a split `extent` px long: `min` is the option's,
/// `max` is the option's cut to the room that keeps `min_other` for the other pane, and never below
/// `min`.
pub fn bounds(extent: f32, opts: &SplitOptions<'_>) -> (f32, f32) {
    let len = if extent.is_finite() {
        extent.max(0.0)
    } else {
        0.0
    };
    let room = (len - STRIP).max(0.0);
    let lo = if opts.min.is_finite() {
        opts.min.max(0.0)
    } else {
        0.0
    };
    let other = if opts.min_other.is_finite() {
        opts.min_other.max(0.0)
    } else {
        0.0
    };
    let top = if opts.max.is_nan() { room } else { opts.max };
    (lo, top.min(room - other).max(lo))
}

/// `size` cut to `[lo, hi]`, `lo` winning; not NaN for a finite `lo`.
fn clamp(size: f32, lo: f32, hi: f32) -> f32 {
    size.min(hi.max(lo)).max(lo)
}

/// The extent of `rect` along `axis`.
fn extent(rect: Rect, axis: Axis) -> f32 {
    match axis {
        Axis::Horizontal => rect.w,
        Axis::Vertical => rect.h,
    }
}

/// The size drawn for `stored` (or the default) in a split of `extent`: never NaN.
fn drawn(extent: f32, stored: Option<f32>, opts: &SplitOptions<'_>) -> f32 {
    let (lo, hi) = bounds(extent, opts);
    let start = stored
        .filter(|s| s.is_finite())
        .or(Some(opts.default).filter(|d| d.is_finite()))
        .unwrap_or(lo);
    clamp(start, lo, hi)
}

/// The panes and the strip for a sized pane of `size` in `rect`. A size larger than the room is
/// cut to it, so nothing leaves the rect.
pub fn geometry(rect: Rect, axis: Axis, sized: SplitSide, size: f32) -> SplitGeometry {
    // A rect that is not finite counts as empty; a negative side as 0.
    let rect = if [rect.x, rect.y, rect.w, rect.h]
        .iter()
        .all(|v| v.is_finite())
    {
        Rect::new(rect.x, rect.y, rect.w.max(0.0), rect.h.max(0.0))
    } else {
        Rect::default()
    };
    let len = if extent(rect, axis).is_finite() {
        extent(rect, axis).max(0.0)
    } else {
        0.0
    };
    let room = (len - STRIP).max(0.0);
    let s = if size.is_nan() {
        0.0
    } else {
        size.clamp(0.0, room)
    };
    let at = match sized {
        SplitSide::First => s,
        SplitSide::Second => room - s,
    };
    // `from` and `to` are offsets along the axis.
    let piece = |from: f32, to: f32| match axis {
        Axis::Horizontal => Rect::new(rect.x + from, rect.y, (to - from).max(0.0), rect.h),
        Axis::Vertical => Rect::new(rect.x, rect.y + from, rect.w, (to - from).max(0.0)),
    };
    SplitGeometry {
        first: piece(0.0, at),
        second: piece(at + STRIP, len),
        strip: piece(at, at + STRIP).intersect(&rect),
        hit: piece(at - (HIT - STRIP) / 2.0, at + (HIT + STRIP) / 2.0).intersect(&rect),
    }
}

/// The pointer's hold on the strip: where it began.
#[derive(Debug, Clone, Copy)]
struct Grab {
    /// The pointer's place along the axis at the press, in window space.
    origin: f32,
    /// The drawn size at the press: what the travel is added to.
    size: f32,
    /// The stored size at the press: what Escape puts back.
    stored: Option<f32>,
    /// Escape ended it; the pointer is still down but no longer moves the size.
    cancelled: bool,
}

#[derive(Debug, Default)]
struct Hold(Option<Grab>);

impl Ui<'_> {
    /// The persistent state of the split under `key`, in the current scope: the sized pane's
    /// stored size, for the app to load and to store. It lives while a split is drawn under `key`
    /// (also hidden) or a keep-alive scope around the last one is.
    pub fn split_state(&mut self, key: impl Hash) -> &mut SplitState {
        let id = self.id(key);
        self.persistent::<SplitState>(id)
    }

    /// A split under `key` in `rect`. `draw` is called for each pane with its rect, `First` then
    /// `Second`, clipped to that rect and under a scope of its own; the strip is drawn and
    /// registered after both. The sized pane is hidden when `opts.hidden`: only the other is drawn,
    /// on the whole rect.
    ///
    /// The panes are drawn with the size the frame began with; a size this frame's input changed
    /// shows in the strip and the response at once and in the panes on the catch-up frame it
    /// asks for.
    pub fn split_panes(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &SplitOptions<'_>,
        mut draw: impl FnMut(&mut Ui<'_>, Pane, Rect),
    ) -> SplitResponse {
        let id = self.id(&key);
        let len = extent(rect, opts.axis);
        let stored = self.split_state(&key).size;
        let before = drawn(len, stored, opts);
        let other = match opts.sized {
            SplitSide::First => Pane::Second,
            SplitSide::Second => Pane::First,
        };

        if opts.hidden {
            self.scope(&key, |ui| pane(ui, other, rect, &mut draw));
            return SplitResponse {
                response: Response::inert(id, rect, self.is_enabled()),
                size: before,
                changed: false,
            };
        }

        let at = geometry(rect, opts.axis, opts.sized, before);
        self.scope(&key, |ui| {
            pane(ui, Pane::First, at.first, &mut draw);
            pane(ui, Pane::Second, at.second, &mut draw);
        });

        let mut r = self.interact(id, at.hit, SENSE);
        let hold = id.with("hold");
        if !r.enabled {
            self.transient::<Hold>(hold).0 = None;
        }
        if r.press_started {
            let origin = r.press_pos.map_or(0.0, |p| along(p, opts.axis));
            self.transient::<Hold>(hold).0 = Some(Grab {
                origin,
                size: before,
                stored,
                cancelled: false,
            });
        }
        let (lo, hi) = bounds(len, opts);
        let mut next = before;
        let mut restore = None;
        // The release frame counts: the event carries the pointer's last move.
        let following = r.pressed || r.press_started || r.clicked || r.drag_stopped;
        if let Some(mut grab) = self.transient::<Hold>(hold).0.filter(|_| r.enabled) {
            if r.pressed && !grab.cancelled && self.consume_key_while_pressed(id, NamedKey::Escape)
            {
                grab.cancelled = true;
                restore = Some(grab.stored);
                next = drawn(len, grab.stored, opts);
            } else if following && !grab.cancelled {
                let now = if r.press_started {
                    r.press_pos.map(|p| along(p, opts.axis))
                } else {
                    self.pointer_pos().map(|p| along(p, opts.axis))
                };
                // Window travel to logical: the drawing space may be scaled.
                let drawn_len = extent(self.painter().transformed(rect), opts.axis);
                let scale = if len > 0.0 && drawn_len > 0.0 {
                    drawn_len / len
                } else {
                    1.0
                };
                if let Some(now) = now {
                    let travel = (now - grab.origin) / scale;
                    let signed = match opts.sized {
                        SplitSide::First => travel,
                        SplitSide::Second => -travel,
                    };
                    let moved = grab.size + signed;
                    if moved.is_finite() {
                        next = clamp(moved, lo, hi);
                    }
                }
            }
            self.transient::<Hold>(hold).0 = (r.pressed || r.press_started).then_some(grab);
        }

        // Keys are taken whatever the state of the strip, then applied only when it may act and
        // the pointer does not hold it: the pointer alone moves the size while it is down.
        let (toward_start, toward_end) = match opts.axis {
            Axis::Horizontal => (NamedKey::ArrowLeft, NamedKey::ArrowRight),
            Axis::Vertical => (NamedKey::ArrowUp, NamedKey::ArrowDown),
        };
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
            // An accessibility increment or decrement arrives as Up or Down.
            let action = self.focus_event_is_synthetic(index);
            // The strip's move along the axis, or the sized pane's direct change.
            let by = |strip: f32| match opts.sized {
                SplitSide::First => strip,
                SplitSide::Second => -strip,
            };
            let to = match k {
                NamedKey::ArrowUp if action => next + KEY_STEP,
                NamedKey::ArrowDown if action => next - KEY_STEP,
                k if k == toward_start => next - by(KEY_STEP),
                k if k == toward_end => next + by(KEY_STEP),
                NamedKey::ArrowUp if opts.axis == Axis::Horizontal => next + KEY_STEP,
                NamedKey::ArrowDown if opts.axis == Axis::Horizontal => next - KEY_STEP,
                NamedKey::Home => lo,
                NamedKey::End => hi,
                _ => continue,
            };
            self.consume_focus_event(index);
            if r.enabled && !following {
                next = clamp(to, lo, hi);
            }
        }
        if let (Some(AccessValue::Number(v)), true, false) = (&r.set_value, r.enabled, following)
            && v.is_finite()
        {
            next = clamp(*v as f32, lo, hi);
        }

        let changed = next != before;
        if changed || restore.is_some() {
            self.split_state(&key).size = match restore {
                Some(stored) => stored,
                None => Some(next),
            };
        }
        if changed {
            r.mark_changed();
            // The panes drawn above showed the old size.
            self.request_catch_up();
        }

        // The strip, in its new place.
        let at = geometry(rect, opts.axis, opts.sized, next);
        if r.hovered || r.pressed {
            self.set_cursor(match opts.axis {
                Axis::Horizontal => CursorIcon::ColResize,
                Axis::Vertical => CursorIcon::RowResize,
            });
        }
        let accent = self.theme().accent;
        let [ar, ag, ab, _] = accent;
        let fill = self.animate_color(
            id.with("focus"),
            if r.focus_visible {
                accent
            } else {
                [ar, ag, ab, 0]
            },
            &FOCUS,
        );
        if fill[3] > 0 {
            self.painter().fill(at.strip, 0.0, fill);
        }

        let mut node = Node::new(id, Role::Separator, opts.name, at.hit);
        node.value = Some(Value::Number {
            value: f64::from(next),
            min: f64::from(lo),
            max: f64::from(hi),
            step: f64::from(KEY_STEP),
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
        SplitResponse {
            response: r,
            size: next,
            changed,
        }
    }
}

/// `p` along `axis`.
fn along(p: (f32, f32), axis: Axis) -> f32 {
    match axis {
        Axis::Horizontal => p.0,
        Axis::Vertical => p.1,
    }
}

/// One pane: its own id scope and a clip to its rect.
fn pane(ui: &mut Ui<'_>, which: Pane, rect: Rect, draw: &mut impl FnMut(&mut Ui<'_>, Pane, Rect)) {
    ui.scope(
        match which {
            Pane::First => "first",
            Pane::Second => "second",
        },
        |ui| {
            ui.painter().push_clip(rect);
            draw(ui, which, rect);
            ui.painter().pop_clip();
        },
    );
}

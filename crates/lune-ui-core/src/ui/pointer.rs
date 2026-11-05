//! The pointer state machine: presses, capture, clicks, double clicks, drags and scroll, resolved
//! against the previous frame's layered hit list (D4).

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::input::{Modifiers, PointerButton, ScrollDelta};
use crate::motion::Time;
use crate::paint::Rect;

use super::{Sense, WidgetId};

/// Travel from the press point at which a drag starts.
pub(crate) const DRAG_THRESHOLD: f32 = 4.0;
/// Two clicks on one id closer than this (frame time) are a double click.
pub(crate) const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// One hit region of a finished frame, in draw order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HitRecord {
    pub(crate) id: WidgetId,
    pub(crate) rect: Rect,
    pub(crate) layer: usize,
    pub(crate) sense: Sense,
    pub(crate) tag: u32,
    pub(crate) enabled: bool,
    /// The axes a scroll region is live on, `(x, y)`: a wheel delta on an axis goes only to a region
    /// live on it (D178). Both for every other region.
    pub(crate) scroll_axes: (bool, bool),
}

/// The topmost region of `hits` at `pos` that `eligible` admits: highest layer, then latest call.
pub(crate) fn topmost<'h>(
    hits: &'h [HitRecord],
    pos: (f32, f32),
    eligible: &dyn Fn(&HitRecord) -> bool,
) -> Option<&'h HitRecord> {
    hits.iter()
        .enumerate()
        .filter(|(_, h)| eligible(h) && h.rect.contains(pos))
        .max_by_key(|(i, h)| (h.layer, *i))
        .map(|(_, h)| h)
}

fn senses_capture(h: &HitRecord) -> bool {
    h.enabled && (h.sense.click || h.sense.drag)
}

fn senses_drag(hits: &[HitRecord], id: WidgetId) -> bool {
    hits.iter().any(|h| h.id == id && h.sense.drag && h.enabled)
}

/// A primary press a region captured this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PressInfo {
    pub(crate) pos: (f32, f32),
    pub(crate) count: u8,
    pub(crate) modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Press {
    /// The captured id; `None` when the press landed on nothing that captures.
    id: Option<WidgetId>,
    at: (f32, f32),
    last: (f32, f32),
    dragging: bool,
}

/// Pointer state that outlives a frame (held in the store).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PointerState {
    pos: Option<(f32, f32)>,
    press: Option<Press>,
    /// The id a secondary press landed on, until its release.
    secondary: Option<Option<WidgetId>>,
    last_click: Option<(WidgetId, Time)>,
    /// The id, time and count of the last captured primary press, for `press_count`.
    last_press: Option<(WidgetId, Time, u8)>,
}

/// One frame's pointer reading.
#[derive(Debug, Clone, Default)]
pub(crate) struct PointerFrame {
    pub(crate) hovered: Option<WidgetId>,
    pub(crate) pressed: Option<WidgetId>,
    pub(crate) dragging: Option<WidgetId>,
    pub(crate) clicked: BTreeSet<WidgetId>,
    pub(crate) double_clicked: BTreeSet<WidgetId>,
    pub(crate) secondary_clicked: BTreeSet<WidgetId>,
    pub(crate) drag_started: BTreeSet<WidgetId>,
    pub(crate) drag_stopped: BTreeSet<WidgetId>,
    pub(crate) press_started: BTreeMap<WidgetId, PressInfo>,
    pub(crate) drag_delta: BTreeMap<WidgetId, (f32, f32)>,
    pub(crate) scroll_lines: BTreeMap<WidgetId, (f32, f32)>,
    pub(crate) scroll_pixels: BTreeMap<WidgetId, (f32, f32)>,
}

impl PointerFrame {
    /// Whether the frame's pointer input resolved to something a widget reacts to after it drew:
    /// a click of any kind, a drag start, movement or stop, or a scroll.
    pub(crate) fn acted(&self) -> bool {
        !(self.clicked.is_empty()
            && self.double_clicked.is_empty()
            && self.secondary_clicked.is_empty()
            && self.drag_started.is_empty()
            && self.drag_stopped.is_empty()
            && self.drag_delta.is_empty()
            && self.scroll_lines.is_empty()
            && self.scroll_pixels.is_empty())
    }
}

fn add(m: &mut BTreeMap<WidgetId, (f32, f32)>, id: WidgetId, d: (f32, f32)) {
    let e = m.entry(id).or_insert((0.0, 0.0));
    e.0 += d.0;
    e.1 += d.1;
}

/// What the events of one frame resolve against.
pub(crate) struct Ctx<'c> {
    pub(crate) hits: &'c [HitRecord],
    pub(crate) eligible: &'c dyn Fn(&HitRecord) -> bool,
    pub(crate) time: Time,
}

impl PointerState {
    /// The pointer moved to `pos`.
    pub(crate) fn moved(&mut self, pos: (f32, f32), cx: &Ctx<'_>, out: &mut PointerFrame) {
        self.pos = Some(pos);
        let Some(p) = &mut self.press else { return };
        if let Some(id) = p.id {
            if p.dragging {
                add(
                    &mut out.drag_delta,
                    id,
                    (pos.0 - p.last.0, pos.1 - p.last.1),
                );
            } else {
                let (dx, dy) = (pos.0 - p.at.0, pos.1 - p.at.1);
                if (dx * dx + dy * dy).sqrt() >= DRAG_THRESHOLD && senses_drag(cx.hits, id) {
                    p.dragging = true;
                    out.drag_started.insert(id);
                    add(&mut out.drag_delta, id, (dx, dy));
                }
            }
        }
        p.last = pos;
    }

    /// The pointer's last known position; `None` outside the window.
    pub(crate) fn pos(&self) -> Option<(f32, f32)> {
        self.pos
    }

    /// The pointer left the window.
    pub(crate) fn left(&mut self) {
        self.pos = None;
    }

    /// The topmost eligible hit at `pos`.
    pub(crate) fn target<'h>(&self, pos: (f32, f32), cx: &Ctx<'h>) -> Option<&'h HitRecord> {
        topmost(cx.hits, pos, cx.eligible)
    }

    /// A button changed state at `pos`. Returns the id a primary press captured, if any.
    pub(crate) fn button(
        &mut self,
        pos: (f32, f32),
        button: PointerButton,
        pressed: bool,
        modifiers: Modifiers,
        cx: &Ctx<'_>,
        out: &mut PointerFrame,
    ) -> Option<WidgetId> {
        self.moved(pos, cx, out);
        let target = self.target(pos, cx);
        match (button, pressed) {
            (PointerButton::Primary, true) => {
                let id = target.filter(|h| senses_capture(h)).map(|h| h.id);
                self.press = Some(Press {
                    id,
                    at: pos,
                    last: pos,
                    dragging: false,
                });
                self.last_press = match id {
                    Some(id) => {
                        let count = match self.last_press {
                            Some((last, t0, n))
                                if last == id && cx.time.saturating_sub(t0) < DOUBLE_CLICK =>
                            {
                                n % 3 + 1
                            }
                            _ => 1,
                        };
                        out.press_started.insert(
                            id,
                            PressInfo {
                                pos,
                                count,
                                modifiers,
                            },
                        );
                        Some((id, cx.time, count))
                    }
                    None => None,
                };
                return id;
            }
            (PointerButton::Primary, false) => {
                let p = self.press.take()?;
                let id = p.id?;
                if p.dragging {
                    out.drag_stopped.insert(id);
                    return None;
                }
                if target.map(|h| h.id) == Some(id) {
                    out.clicked.insert(id);
                    match self.last_click {
                        Some((last, t0))
                            if last == id && cx.time.saturating_sub(t0) < DOUBLE_CLICK =>
                        {
                            out.double_clicked.insert(id);
                            self.last_click = None;
                        }
                        _ => self.last_click = Some((id, cx.time)),
                    }
                }
            }
            (PointerButton::Secondary, true) => {
                self.secondary = Some(target.filter(|h| senses_capture(h)).map(|h| h.id));
            }
            (PointerButton::Secondary, false) => {
                if let Some(Some(id)) = self.secondary.take()
                    && target.map(|h| h.id) == Some(id)
                {
                    out.secondary_clicked.insert(id);
                }
            }
            _ => {}
        }
        None
    }

    /// A scroll gesture at `pos`: each axis of the delta goes to the topmost scroll region live on
    /// that axis that is not on a layer below the topmost region's (D178). A region live on both
    /// axes takes both components, as one live on one takes its own.
    pub(crate) fn scroll(
        &mut self,
        pos: (f32, f32),
        delta: ScrollDelta,
        cx: &Ctx<'_>,
        out: &mut PointerFrame,
    ) {
        self.pos = Some(pos);
        let Some(top) = self.target(pos, cx) else {
            return;
        };
        let layer = top.layer;
        let region = |on: fn(&HitRecord) -> bool| {
            topmost(cx.hits, pos, &|h| {
                (cx.eligible)(h) && h.enabled && h.sense.scroll && h.layer >= layer && on(h)
            })
        };
        let (map, x, y) = match delta {
            ScrollDelta::Lines { x, y } => (&mut out.scroll_lines, x, y),
            ScrollDelta::Pixels { x, y } => (&mut out.scroll_pixels, x, y),
        };
        if x != 0.0
            && let Some(to) = region(|h| h.scroll_axes.0)
        {
            add(map, to.id, (x, 0.0));
        }
        if y != 0.0
            && let Some(to) = region(|h| h.scroll_axes.1)
        {
            add(map, to.id, (0.0, y));
        }
    }

    /// Fills hover, capture and drag state after the frame's events.
    pub(crate) fn settle(&self, cx: &Ctx<'_>, out: &mut PointerFrame) {
        let captured = self.press.and_then(|p| p.id);
        out.pressed = captured;
        out.dragging = self.press.filter(|p| p.dragging).and_then(|p| p.id);
        out.hovered = match (captured, self.pos) {
            (_, None) => None,
            (Some(id), Some(pos)) => self.target(pos, cx).filter(|h| h.id == id).map(|h| h.id),
            (None, Some(pos)) => self
                .target(pos, cx)
                .filter(|h| h.enabled && h.sense.hover)
                .map(|h| h.id),
        };
    }

    /// Drops a primary press without capturing (an outside press that dismissed an overlay).
    pub(crate) fn press_consumed(&mut self, pos: (f32, f32)) {
        self.pos = Some(pos);
        self.press = Some(Press {
            id: None,
            at: pos,
            last: pos,
            dragging: false,
        });
    }

    /// Whether a press holds a widget's capture now, at this point in the frame's event batch.
    pub(crate) fn captured(&self) -> bool {
        self.press.is_some_and(|p| p.id.is_some())
    }

    /// Releases a capture whose id `lost` says can no longer hold it (it fell under a modal).
    pub(crate) fn drop_capture_if(&mut self, lost: impl Fn(WidgetId) -> bool) {
        if let Some(p) = &mut self.press
            && p.id.is_some_and(&lost)
        {
            p.id = None;
            p.dragging = false;
        }
    }
}

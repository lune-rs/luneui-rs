//! In-window drag and drop (`infra.drag-and-drop`): pointer tracking only, never the OS drag API.
//!
//! Two pure state machines and the glue that feeds them from a frame:
//!
//! - [`ReorderDrag`] reorders items along one axis (a strip of tabs, a row of chips). A press arms
//!   it, 4 px of horizontal travel starts the drag, the dragged item follows the pointer while a
//!   target slot and every other item's offset are recomputed, and an edge zone auto-scrolls the
//!   strip. The shape is a port of the reference tab-strip controller: indices and widths in, an
//!   offset per item out.
//! - [`FreeDrag`] is the two-axis form (a board of columns of cards): 4 px of Euclidean travel
//!   starts it and the target is a column and an insertion index.
//!
//! Both are plain data with no clock: the auto-scroll takes the frame's `dt`, never a timer.
//! Totality is a contract: NaN or infinite coordinates, zero or negative widths, an empty list and an
//! out-of-range index never panic and answer "not dragging, all offsets zero, delta zero".
//!
//! The glue is [`Ui::drag_gesture`] (and [`Ui::free_drag_gesture`]): it reads the item's
//! [`Response`], feeds the machine, cancels on Escape (through
//! [`Ui::consume_key_while_pressed`], so Escape with nothing in flight is not consumed), on capture
//! loss, on a button up that reported no release and on a change of the item count, and asks for a
//! frame while the auto-scroll delta is non-zero. The item's hit region must sense a drag
//! ([`Sense::DRAG`](crate::ui::Sense::DRAG)). Coordinates in the glue are window coordinates in
//! logical pixels, as [`Response::press_pos`] and [`Ui::pointer_pos`] report them.
//!
//! Motion (`Ui::drag_offset`): a neighbour slides to its new slot (`move`, `fast`, `sine-in-out`),
//! the dragged item has no transition, and after a release it settles into its slot (`settle`, 180
//! ms in the Full tier, a snap in Reduced and Off). Auto-scroll is never eased.

use std::time::Duration;

use crate::access::State;
use crate::input::NamedKey;
use crate::motion::{Easing, Kind, Motion, Reduce, Time, Token};
use crate::paint::{Painter, Rect};
use crate::scene::LayerId;
use crate::theme::Theme;
use crate::ui::{Response, Ui, WidgetId};

/// Horizontal travel (reorder) or Euclidean travel (free) before a press becomes a drag.
pub const DRAG_THRESHOLD: f64 = 4.0;
/// Distance from a viewport edge within which the auto-scroll arms.
pub const AUTO_SCROLL_ZONE: f64 = 24.0;
/// Auto-scroll at full ramp per [`AUTO_SCROLL_TICK`], about 750 px/s.
pub const AUTO_SCROLL_MAX: f64 = 12.0;
/// The tick [`AUTO_SCROLL_MAX`] is per.
pub const AUTO_SCROLL_TICK: Duration = Duration::from_millis(16);
/// The drop line's thickness, logical px.
pub const DROP_LINE_THICKNESS: f32 = 2.0;
/// The opacity of a dimmed source card.
pub const DIM_ALPHA: f32 = 0.4;

/// A neighbour sliding into its slot: `move`, `fast`, `sine-in-out`, snapped in Reduced and Off.
pub const SLIDE: Motion = Motion::of(Kind::Move);
/// The released item settling into its slot: `settle`, snapped in Reduced and Off.
pub const SETTLE: Motion = Motion::of(Kind::Settle);
/// The source card dimming: `selection`, `fast`, `linear`.
pub const DIM: Motion = Motion::of(Kind::Selection);
/// No transition at all: the dragged item tracks the pointer exactly.
const FOLLOW: Motion = Motion {
    kind: Kind::Move,
    token: Token::Zero,
    easing: Easing::Linear,
    reduced: Reduce::Snap,
    reduced_token: None,
};

/// Coordinates and widths are limited to this, so sums of them stay finite.
const MAX_COORD: f64 = 1.0e12;

fn finite(v: f64) -> bool {
    v.is_finite()
}

/// `v` limited to `[lo, hi]`; `hi` below `lo` gives `lo` (never panics).
fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi.max(lo))
}

/// One measurement of the strip, passed to every call (widths change during a gesture).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics<'a> {
    /// The items' widths in strip order.
    pub widths: &'a [f64],
    /// The gap between neighbours.
    pub spacing: f64,
    /// The strip viewport's width.
    pub viewport: f64,
    /// The strip's scroll offset.
    pub offset: f64,
    /// The strip's scrollable extent.
    pub extent: f64,
}

/// What a finished gesture means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropResult {
    /// Nothing was in flight.
    None,
    /// The gesture ended without a reorder (armed and never dragged, or dragged back to its start).
    /// The item already activated on press; there is nothing further to do.
    Click,
    /// The item at `from` moves to index `to`.
    Commit { from: usize, to: usize },
}

/// Glue state that lives with the machine: which item owns the gesture, the last frame's time and
/// the item count it started with, and the running settle.
#[derive(Debug, Clone, Default)]
struct Glue {
    owner: Option<WidgetId>,
    last_time: Option<Time>,
    count: usize,
    /// The released item and the app time its settle ends.
    settle: Option<(WidgetId, Time)>,
}

/// The reorder gesture's state machine.
#[derive(Debug, Clone, Default)]
pub struct ReorderDrag {
    armed: bool,
    dragging: bool,
    press_index: Option<usize>,
    target_index: Option<usize>,
    press_x: f64,
    grab: f64,
    cx: f64,
    vx: f64,
    delta: f64,
    step: f64,
    left: f64,
    spacing: f64,
    offsets: Vec<f64>,
    widths: Vec<f64>,
    x: Vec<f64>,
    nx: Vec<f64>,
    order: Vec<usize>,
    glue: Glue,
}

impl ReorderDrag {
    /// An idle machine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pressed on an item but still under the threshold.
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// The threshold has been crossed and the drag is live.
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// Armed or dragging.
    pub fn is_active(&self) -> bool {
        self.armed || self.dragging
    }

    /// The index the drag started from, while dragging.
    pub fn dragged_index(&self) -> Option<usize> {
        if self.dragging {
            self.press_index
        } else {
            None
        }
    }

    /// The index the dragged item would land in on release, while dragging.
    pub fn target_index(&self) -> Option<usize> {
        if self.dragging {
            self.target_index
        } else {
            None
        }
    }

    /// Every item's horizontal offset, index-aligned with the widths; all zero when not dragging.
    /// The dragged item's entry tracks the pointer; every other entry is the exact difference
    /// between that item's slot now and its slot in the reordered layout.
    pub fn offsets(&self) -> &[f64] {
        &self.offsets
    }

    /// Signed scroll to add to the strip this tick (per [`AUTO_SCROLL_TICK`]); 0 when idle.
    pub fn auto_scroll_delta(&self) -> f64 {
        self.delta
    }

    /// The scroll to apply this frame: [`Self::auto_scroll_delta`] scaled by the `dt` of the last
    /// [`Self::tick`]; 0 before any tick.
    pub fn auto_scroll_step(&self) -> f64 {
        self.step
    }

    /// The dragged item's clamped left edge in strip content coordinates; 0 when not dragging.
    pub fn dragged_left(&self) -> f64 {
        self.left
    }

    /// Arms a gesture on item `index`. Records the threshold origin and the grab offset, but does
    /// not start a drag; activating the item is the caller's job.
    pub fn press(&mut self, index: usize, content_x: f64, viewport_x: f64, m: &Metrics<'_>) {
        self.cancel();
        let n = self.load(m);
        if n == 0 || index >= n || !finite(content_x) || !finite(viewport_x) {
            return;
        }
        self.press_index = Some(index);
        self.press_x = content_x;
        self.cx = content_x;
        self.vx = viewport_x;
        // A seed only: `update` re-takes it while the gesture is armed (see `rebaseline`).
        self.grab = clamp(content_x - self.x[index], 0.0, self.widths[index]);
        self.armed = true;
    }

    /// Feeds a new pointer position (`content_x` in strip content coordinates, `viewport_x` in the
    /// strip viewport's). Crosses the threshold when the horizontal travel reaches
    /// [`DRAG_THRESHOLD`] (a vertical wobble stays a click), then recomputes the target, the offsets
    /// and the auto-scroll delta. Returns whether a drag is live.
    pub fn update(&mut self, content_x: f64, viewport_x: f64, m: &Metrics<'_>) -> bool {
        if !self.armed && !self.dragging {
            return false;
        }
        if finite(content_x) {
            self.cx = content_x;
        }
        if finite(viewport_x) {
            self.vx = viewport_x;
        }
        if !self.dragging {
            // Before the threshold test, so the last baseline is the one taken on the very update
            // that starts the drag, in that update's own layout frame.
            self.rebaseline(m);
            if (self.cx - self.press_x).abs() >= DRAG_THRESHOLD {
                self.dragging = true;
                self.armed = false;
            }
        }
        self.recompute(m);
        self.dragging
    }

    /// Re-takes the grab offset against the current layout while armed: a layout shift between the
    /// press and the crossing (the pressed item's width changing) must not show as a jump.
    fn rebaseline(&mut self, m: &Metrics<'_>) {
        let n = self.load(m);
        let Some(p) = self.press_index.filter(|&p| p < n) else {
            return;
        };
        self.grab = clamp(self.cx - self.x[p], 0.0, self.widths[p]);
    }

    /// An auto-scroll tick over `dt` of frame time: the pointer has not moved but the content under
    /// it has, so the content position is re-derived as `viewport_x + metrics.offset`. The scroll
    /// to apply (`auto_scroll_step`) is the per-tick delta scaled by `dt / 16 ms`. Returns whether
    /// a drag is live.
    pub fn tick(&mut self, dt: Duration, m: &Metrics<'_>) -> bool {
        if !self.dragging {
            self.delta = 0.0;
            self.step = 0.0;
            return false;
        }
        if finite(m.offset) && finite(self.vx) {
            self.cx = self.vx + m.offset;
        }
        self.recompute(m);
        self.step = self.delta * dt.as_secs_f64() / AUTO_SCROLL_TICK.as_secs_f64();
        true
    }

    /// Ends the gesture and reports what it meant; all state is cleared before it returns, so the
    /// indices travel in the result.
    pub fn release(&mut self) -> DropResult {
        let result = if self.dragging {
            match (self.press_index, self.target_index) {
                (Some(from), Some(to)) if from != to => DropResult::Commit { from, to },
                _ => DropResult::Click,
            }
        } else if self.armed {
            DropResult::Click
        } else {
            DropResult::None
        };
        self.reset();
        result
    }

    /// Clears every trace of the gesture, the offsets and any running settle. Idempotent.
    pub fn cancel(&mut self) {
        self.reset();
        self.glue.settle = None;
    }

    fn reset(&mut self) {
        self.armed = false;
        self.dragging = false;
        self.press_index = None;
        self.target_index = None;
        self.press_x = 0.0;
        self.grab = 0.0;
        self.cx = 0.0;
        self.vx = 0.0;
        self.delta = 0.0;
        self.step = 0.0;
        self.left = 0.0;
        self.offsets.iter_mut().for_each(|o| *o = 0.0);
        self.glue.owner = None;
        self.glue.last_time = None;
    }

    fn recompute(&mut self, m: &Metrics<'_>) {
        self.delta = 0.0;
        self.step = 0.0;
        let n = self.load(m);
        let Some(d) = self.press_index.filter(|&d| self.dragging && d < n) else {
            // Degenerate but total: no offsets, and a target equal to the source so a release
            // reports a click, never an out-of-range commit.
            self.offsets.iter_mut().for_each(|o| *o = 0.0);
            self.left = 0.0;
            self.target_index = self.press_index;
            return;
        };
        let wd = self.widths[d];
        let total = self.x[n - 1] + self.widths[n - 1];
        let max_left = (total - wd).max(0.0);
        // Re-clamped against the current width, so it stays inside an item that has shrunk.
        let grab = clamp(self.grab, 0.0, wd);
        let left = clamp(self.cx - grab, 0.0, max_left);
        self.left = left;
        let target = self.nearest_slot(n, d, left);
        self.target_index = Some(target);
        self.build_offsets(n, d, left, target);
        self.delta = self.auto_scroll(m);
    }

    /// Of the `n` insertion positions, the one whose left edge is nearest the dragged item's left
    /// edge, the lowest index on a tie. Correct for unequal widths, which comparing against
    /// neighbour midpoints is not.
    fn nearest_slot(&self, n: usize, d: usize, left: f64) -> usize {
        let mut best = f64::MAX;
        let mut best_index = 0;
        let mut slot_x = 0.0;
        for slot in 0..n {
            let distance = (slot_x - left).abs();
            if distance < best {
                best = distance;
                best_index = slot;
            }
            if slot + 1 < n {
                let other = if slot < d { slot } else { slot + 1 };
                slot_x += self.widths[other] + self.spacing;
            }
        }
        best_index
    }

    /// Every item's offset, derived from the reordered layout rather than a fixed shift, which is
    /// what makes unequal widths exact.
    fn build_offsets(&mut self, n: usize, d: usize, left: f64, target: usize) {
        let mut consumed = 0;
        for slot in 0..n {
            if slot == target {
                self.order[slot] = d;
                continue;
            }
            self.order[slot] = if consumed < d { consumed } else { consumed + 1 };
            consumed += 1;
        }
        let mut position = 0.0;
        for slot in 0..n {
            self.nx[slot] = position;
            position += self.widths[self.order[slot]] + self.spacing;
        }
        for slot in 0..n {
            let item = self.order[slot];
            self.offsets[item] = self.nx[slot] - self.x[item];
        }
        // The dragged item tracks the pointer, not its would-be slot.
        self.offsets[d] = left - self.x[d];
    }

    fn auto_scroll(&self, m: &Metrics<'_>) -> f64 {
        let (viewport, offset, extent) = (m.viewport, m.offset, m.extent);
        if !finite(viewport) || !finite(offset) || !finite(extent) || !finite(self.vx) {
            return 0.0;
        }
        if viewport <= 0.0 {
            return 0.0;
        }
        let can_left = offset > 0.0;
        let can_right = offset < extent - viewport - 0.5;
        edge_step(self.vx, 0.0, viewport, can_left, can_right)
    }

    /// Sanitises the metrics into the scratch buffers and returns the usable item count (0 when
    /// there is nothing to work with). Widths and spacing are forced finite and non-negative here.
    fn load(&mut self, m: &Metrics<'_>) -> usize {
        let n = m.widths.len();
        if n == 0 {
            return 0;
        }
        if self.offsets.len() != n {
            self.offsets = vec![0.0; n];
            self.widths = vec![0.0; n];
            self.x = vec![0.0; n];
            self.nx = vec![0.0; n];
            self.order = vec![0; n];
        }
        self.spacing = if finite(m.spacing) && m.spacing > 0.0 {
            m.spacing.min(MAX_COORD)
        } else {
            0.0
        };
        let mut position = 0.0;
        for (i, &w) in m.widths.iter().enumerate() {
            self.widths[i] = if finite(w) && w > 0.0 {
                w.min(MAX_COORD)
            } else {
                0.0
            };
            self.x[i] = position;
            position += self.widths[i] + self.spacing;
        }
        n
    }
}

/// Linear and clamped to `[0, 1]`, deliberately not eased: an auto-scroll wants to be predictable.
fn ramp(u: f64) -> f64 {
    if finite(u) { u.clamp(0.0, 1.0) } else { 0.0 }
}

/// The auto-scroll delta per [`AUTO_SCROLL_TICK`] of a pointer at `p` over the viewport `lo..hi`:
/// negative within [`AUTO_SCROLL_ZONE`] of `lo` while `can_lo`, positive within it of `hi` while
/// `can_hi`, ramping linearly to [`AUTO_SCROLL_MAX`] at the edge and past it; else 0, also for a
/// coordinate that is not finite.
pub fn edge_step(p: f64, lo: f64, hi: f64, can_lo: bool, can_hi: bool) -> f64 {
    if !finite(p) || !finite(lo) || !finite(hi) {
        return 0.0;
    }
    if p < lo + AUTO_SCROLL_ZONE && can_lo {
        return -AUTO_SCROLL_MAX * ramp((lo + AUTO_SCROLL_ZONE - p) / AUTO_SCROLL_ZONE);
    }
    if p > hi - AUTO_SCROLL_ZONE && can_hi {
        return AUTO_SCROLL_MAX * ramp((p - (hi - AUTO_SCROLL_ZONE)) / AUTO_SCROLL_ZONE);
    }
    0.0
}

/// One column of a board: its horizontal extent and its cards' vertical midpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column<'a> {
    pub x: f64,
    pub w: f64,
    /// The cards' vertical midpoints, in any order.
    pub card_mids: &'a [f64],
}

/// Where a free drag would drop: a column and the insertion index within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropTarget {
    pub column: usize,
    pub index: usize,
}

/// The insertion index for a pointer at `pointer_y`: the number of `card_mids` above it. NaN mids
/// and a non-finite pointer count nothing.
pub fn drop_index(card_mids: &[f64], pointer_y: f64) -> usize {
    card_mids.iter().filter(|&&m| m < pointer_y).count()
}

/// The first column whose range `[x, x + w)` holds `pointer_x`, with the insertion index at
/// `pointer_y`; `None` outside every column or for a non-finite pointer.
pub fn drop_target(columns: &[Column<'_>], pointer_x: f64, pointer_y: f64) -> Option<DropTarget> {
    if !finite(pointer_x) || !finite(pointer_y) {
        return None;
    }
    columns
        .iter()
        .position(|c| pointer_x >= c.x && pointer_x < c.x + c.w)
        .map(|column| DropTarget {
            column,
            index: drop_index(columns[column].card_mids, pointer_y),
        })
}

/// What a finished free gesture means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeResult {
    /// Nothing was in flight.
    None,
    /// Armed and never dragged.
    Click,
    /// Dropped outside every column: a no-op.
    NoTarget,
    /// The item `source` was dropped on `target`.
    Drop { source: u64, target: DropTarget },
}

/// The free two-axis drag's state machine (a card between columns): the card stays in place,
/// dimmed, while a drop line shows the insertion slot.
#[derive(Debug, Clone, Default)]
pub struct FreeDrag {
    armed: bool,
    dragging: bool,
    source: u64,
    press: (f64, f64),
    pos: (f64, f64),
    target: Option<DropTarget>,
    owner: Option<WidgetId>,
    count: usize,
}

impl FreeDrag {
    /// An idle machine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pressed but still under the threshold.
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// The threshold has been crossed.
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// Armed or dragging.
    pub fn is_active(&self) -> bool {
        self.armed || self.dragging
    }

    /// The item being dragged, while dragging.
    pub fn source(&self) -> Option<u64> {
        self.dragging.then_some(self.source)
    }

    /// The item pressed, armed or dragging.
    pub fn pressed(&self) -> Option<u64> {
        self.is_active().then_some(self.source)
    }

    /// The column and insertion index under the pointer, while dragging; `None` outside every
    /// column.
    pub fn target(&self) -> Option<DropTarget> {
        if self.dragging { self.target } else { None }
    }

    /// Arms a gesture on `source` at `(x, y)`.
    pub fn press(&mut self, source: u64, x: f64, y: f64) {
        self.cancel();
        if !finite(x) || !finite(y) {
            return;
        }
        self.source = source;
        self.press = (x, y);
        self.pos = (x, y);
        self.armed = true;
    }

    /// Feeds a pointer position. The drag starts at `dx*dx + dy*dy >= 16`; while it is live the
    /// target follows the pointer. Returns whether a drag is live.
    pub fn update(&mut self, x: f64, y: f64, columns: &[Column<'_>]) -> bool {
        if !self.is_active() {
            return false;
        }
        if finite(x) && finite(y) {
            self.pos = (x, y);
        }
        if !self.dragging {
            let (dx, dy) = (self.pos.0 - self.press.0, self.pos.1 - self.press.1);
            if dx * dx + dy * dy < DRAG_THRESHOLD * DRAG_THRESHOLD {
                return false;
            }
            self.dragging = true;
            self.armed = false;
        }
        self.target = drop_target(columns, self.pos.0, self.pos.1);
        true
    }

    /// Ends the gesture and reports what it meant.
    pub fn release(&mut self) -> FreeResult {
        let result = if self.dragging {
            match self.target {
                Some(target) => FreeResult::Drop {
                    source: self.source,
                    target,
                },
                None => FreeResult::NoTarget,
            }
        } else if self.armed {
            FreeResult::Click
        } else {
            FreeResult::None
        };
        self.cancel();
        result
    }

    /// Clears every trace of the gesture. Idempotent.
    pub fn cancel(&mut self) {
        self.armed = false;
        self.dragging = false;
        self.target = None;
        self.owner = None;
        self.press = (0.0, 0.0);
        self.pos = (0.0, 0.0);
    }
}

/// The strip a reorder item sits in, for [`Ui::drag_gesture`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strip<'a> {
    /// The item's index in strip order.
    pub index: usize,
    /// The strip viewport's left edge in window coordinates.
    pub origin: f64,
    pub metrics: Metrics<'a>,
}

/// One frame of a reorder gesture, from [`Ui::drag_gesture`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragFrame {
    /// What a release this frame meant; [`DropResult::None`] on every other frame.
    pub result: DropResult,
    /// A drag is live after this frame.
    pub dragging: bool,
    /// A drag was cancelled this frame (Escape, capture loss, a change of the item count).
    pub cancelled: bool,
    /// The scroll to add to the strip this frame, in px (the auto-scroll delta scaled by `dt`).
    pub scroll_by: f64,
}

impl Default for DragFrame {
    fn default() -> Self {
        DragFrame {
            result: DropResult::None,
            dragging: false,
            cancelled: false,
            scroll_by: 0.0,
        }
    }
}

/// One frame of a free gesture, from [`Ui::free_drag_gesture`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeFrame {
    /// What a release this frame meant; [`FreeResult::None`] on every other frame.
    pub result: FreeResult,
    /// A drag is live after this frame.
    pub dragging: bool,
    /// A drag was cancelled this frame.
    pub cancelled: bool,
}

impl Default for FreeFrame {
    fn default() -> Self {
        FreeFrame {
            result: FreeResult::None,
            dragging: false,
            cancelled: false,
        }
    }
}

impl Ui<'_> {
    /// Feeds `drag` from item `id`'s `resp` (`Sense::DRAG`): call it for every item every frame, and
    /// before [`Self::drag_offset`] so the offsets are this frame's. A primary press on an item arms
    /// the machine (this item then owns the gesture; other items' calls only report `dragging`);
    /// while it is held the pointer feeds `update` and `tick`; a release reports
    /// [`DropResult`]. The gesture cancels on Escape (taken only while dragging), when the
    /// item count of `strip.metrics` changes, and when the capture ends without a release. While the
    /// auto-scroll delta is non-zero the frame asks for another. The owner's node, if already
    /// emitted, gets `State::Pressed` while armed or dragging.
    ///
    /// The app applies a `Commit` to its order, and applies `scroll_by` to the strip's scroll
    /// offset.
    pub fn drag_gesture(
        &mut self,
        id: WidgetId,
        resp: &Response,
        drag: &mut ReorderDrag,
        strip: &Strip<'_>,
    ) -> DragFrame {
        let now = self.time();
        let mut out = DragFrame::default();
        let m = &strip.metrics;
        if resp.press_started && resp.enabled {
            if let Some((px, _)) = resp.press_pos {
                let vx = f64::from(px) - strip.origin;
                drag.press(strip.index, vx + m.offset, vx, m);
                if drag.armed {
                    drag.glue.owner = Some(id);
                    drag.glue.last_time = Some(now);
                    drag.glue.count = m.widths.len();
                }
            }
            return self.pressed_node(id, drag, out);
        }
        if drag.glue.owner != Some(id) {
            out.dragging = drag.dragging;
            return out;
        }
        let was_dragging = drag.dragging;
        let cancelled = |drag: &mut ReorderDrag, mut out: DragFrame| {
            drag.cancel();
            out.cancelled = was_dragging;
            out
        };
        if m.widths.len() != drag.glue.count {
            return cancelled(drag, out);
        }
        if was_dragging && self.consume_key_while_pressed(id, NamedKey::Escape) {
            return cancelled(drag, out);
        }
        if !resp.pressed {
            if resp.drag_stopped || resp.clicked {
                out.result = drag.release();
                if was_dragging {
                    let settle = self
                        .theme()
                        .motion
                        .resolve(&SETTLE, self.tier())
                        .map(|r| (id, now + r.duration));
                    drag.glue.settle = settle;
                }
                return out;
            }
            return cancelled(drag, out);
        }
        if let Some((px, _)) = self.pointer_pos() {
            let vx = f64::from(px) - strip.origin;
            drag.update(vx + m.offset, vx, m);
        }
        let dt = drag
            .glue
            .last_time
            .map_or(Duration::ZERO, |t| now.saturating_sub(t));
        drag.glue.last_time = Some(now);
        if drag.dragging {
            drag.tick(dt, m);
        }
        if drag.delta != 0.0 {
            self.mark_moving();
        }
        out.scroll_by = drag.step;
        self.pressed_node(id, drag, out)
    }

    /// Marks `id`'s node pressed while `drag` is active, and fills `out.dragging`.
    fn pressed_node(&mut self, id: WidgetId, drag: &ReorderDrag, mut out: DragFrame) -> DragFrame {
        out.dragging = drag.dragging;
        if drag.dragging {
            self.note_live_gesture();
        }
        if drag.is_active() && drag.glue.owner == Some(id) {
            self.patch_node(id, |n| {
                n.states.insert(State::Pressed);
            });
        }
        out
    }

    /// The horizontal offset to draw item `index` (id `id`, unshifted layout position `x` in strip
    /// content coordinates) with while `drag` runs, and for the settle after its release; 0
    /// otherwise. The dragged item follows the pointer with no transition; every other item slides
    /// to its slot (`move`); after a release the released item settles (`settle`). Both are
    /// animations of the item's absolute position keyed by `id`, so applying a `Commit` to the
    /// layout changes nothing the animation sees. Call it for every item every frame, after
    /// [`Self::drag_gesture`].
    pub fn drag_offset(&mut self, id: WidgetId, drag: &ReorderDrag, index: usize, x: f32) -> f32 {
        let key = id.with("drag-offset");
        if drag.dragging {
            if drag.press_index == Some(index) {
                let left = drag.left as f32;
                self.animate_f32(key, left, &FOLLOW);
                return left - x;
            }
            let o = drag.offsets.get(index).copied().unwrap_or(0.0) as f32;
            let at = self.animate_f32(key, x + o, &SLIDE);
            return at - x;
        }
        let now = self.time();
        match drag.glue.settle {
            Some((owner, until)) if now < until => {
                let motion = if owner == id { &SETTLE } else { &SLIDE };
                self.animate_f32(key, x, motion) - x
            }
            _ => 0.0,
        }
    }

    /// The opacity of a source card: [`DIM_ALPHA`] while it is the one being dragged, else 1, with
    /// the `selection` transition.
    pub fn drag_dim(&mut self, id: WidgetId, dragged: bool) -> f32 {
        let target = if dragged { DIM_ALPHA } else { 1.0 };
        self.animate_f32(id.with("drag-dim"), target, &DIM)
    }

    /// Runs `f` to draw a dragged item's ghost above the current layer group (`Base` and `Page` to
    /// `Popup`, `Popup` and `Modal` to `Raised`, `Raised` to `Toast`, so text under it cannot show
    /// through): no clip or transform applies, `f` draws in window coordinates, and what it draws
    /// belongs to no open recording. The caller's painter state is back afterwards.
    pub(crate) fn drag_ghost(&mut self, f: impl FnOnce(&mut Ui<'_>)) {
        let (layer, tag) = self.painter().hit_context();
        let above = match layer {
            l if l <= LayerId::Page as usize => LayerId::Popup,
            l if l <= LayerId::Modal as usize => LayerId::Raised,
            _ => LayerId::Toast,
        };
        let paused = self.painter().pause_recording();
        let saved = self.painter().save_state();
        self.painter().enter(above, tag);
        f(self);
        self.painter().restore_state(saved);
        self.painter().resume_recording(paused);
    }

    /// [`Self::drag_gesture`] for a [`FreeDrag`]: item `id` carries `source`; `columns` are in
    /// window coordinates. Escape, a change of the column count and a capture that ends without a
    /// release cancel it. A press and its release inside one frame are a [`FreeResult::Click`].
    ///
    /// The gesture only sees an item that still calls it: when `source` leaves the items, the
    /// caller cancels `drag` itself, or it stays live with nobody to end it.
    pub fn free_drag_gesture(
        &mut self,
        id: WidgetId,
        resp: &Response,
        drag: &mut FreeDrag,
        source: u64,
        columns: &[Column<'_>],
    ) -> FreeFrame {
        let mut out = FreeFrame::default();
        if resp.press_started && resp.enabled {
            if let Some((px, py)) = resp.press_pos {
                drag.press(source, f64::from(px), f64::from(py));
                if drag.armed {
                    drag.owner = Some(id);
                    drag.count = columns.len();
                }
            }
            if !resp.pressed {
                if resp.clicked {
                    out.result = drag.release();
                } else {
                    drag.cancel();
                }
            }
            out.dragging = drag.dragging;
            return out;
        }
        if drag.owner != Some(id) {
            out.dragging = drag.dragging;
            return out;
        }
        let was_dragging = drag.dragging;
        if columns.len() != drag.count
            || (was_dragging && self.consume_key_while_pressed(id, NamedKey::Escape))
        {
            drag.cancel();
            out.cancelled = was_dragging;
            return out;
        }
        if !resp.pressed {
            if resp.drag_stopped || resp.clicked {
                out.result = drag.release();
            } else {
                drag.cancel();
                out.cancelled = was_dragging;
            }
            return out;
        }
        if let Some((px, py)) = self.pointer_pos() {
            drag.update(f64::from(px), f64::from(py), columns);
        }
        out.dragging = drag.dragging;
        if drag.dragging {
            self.note_live_gesture();
        }
        if drag.is_active() {
            self.patch_node(id, |n| {
                n.states.insert(State::Pressed);
            });
        }
        out
    }
}

/// Paints the drop line: [`DROP_LINE_THICKNESS`] tall, `accent`, across `rect`'s width and centred
/// vertically in it (`rect` is the gap between two cards).
pub fn paint_drop_line<H: Copy + PartialEq>(p: &mut Painter<'_, H>, rect: Rect, theme: &Theme) {
    let y = rect.y + (rect.h - DROP_LINE_THICKNESS) / 2.0;
    p.fill(
        Rect::new(rect.x, y, rect.w, DROP_LINE_THICKNESS),
        0.0,
        theme.accent,
    );
}

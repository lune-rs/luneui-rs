//! The scroll area (`layout.scroll-area`): a clipped viewport over taller or wider content, with a
//! per-id offset kept by the `Ui` (persistent state, so it survives a hidden tab under
//! `keep_alive`), smooth wheel stepping, trackpad scrolling 1:1, scroll-into-view, follow-tail,
//! offset preservation across replaced content and the overlay thumb of
//! [`scrollbar`](super::scrollbar).
//!
//! Offsets and extents are `f64` (D6): at a million 45 px rows an `f32` offset steps 4 px. An
//! animated offset is kept as an exact `f64` base plus a small `f32` [`Transition`] relative to
//! it, so it lands on the target exactly and an `f32` never carries a large offset.
//!
//! The pure functions ([`clamp_offset`], [`should_restore`], [`restore_offset`],
//! [`centre_target`], [`reveal_offset`]) port the reference app's offset preserver and its centring
//! formula; the frame logic lives in [`Ui::scroll_area`].
//!
//! Frame order: the wheel and key input of the frame moves the offset, the content draws at it,
//! then everything that needs the content's size runs (the measured clamp, follow-tail, reveals,
//! page keys, the thumb). When one of those moves the offset after the content drew, the frame asks
//! for a catch-up frame (`Repaint::Now`), so the stale picture lasts one frame.

use std::hash::Hash;

use crate::access::{Action, Node, Role, State, Value};
use crate::input::{FocusEvent, Key, NamedKey};
use crate::layout::Axis;
use crate::motion::{Easing, Kind, Motion, Resolved, Time, Transition};
use crate::paint::Rect;
use crate::ui::{Response, Sense, Ui, WidgetId};

use super::scrollbar::thumb_geometry;

/// Logical px per wheel line (D9).
pub const LINE: f64 = 48.0;
/// The wheel step: `scroll` kind, `fast`, `quad-out`, snapped in Reduced.
pub const WHEEL: Motion = Motion::of(Kind::Scroll);
/// Scroll-into-view within two viewports: `fast`, `sine-in-out`, snapped in Reduced.
pub const INTO_VIEW: Motion = Motion {
    easing: Easing::SineInOut,
    ..Motion::of(Kind::Scroll)
};
/// A reveal farther than this many viewports snaps.
pub const SNAP_DISTANCE: f64 = 2.0;

/// Whether an axis shows its bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bars {
    /// The thumb shows (while the pointer is over the area) when the content overflows.
    Auto,
    /// No thumb; the wheel and the keys still scroll.
    Hidden,
    /// No thumb and no scrolling on this axis.
    Disabled,
}

impl Bars {
    /// This bar while `extent` overflows `viewport`, else [`Bars::Disabled`]. A composite that nests
    /// an area and knows its extent before the area runs passes its axes through this: an area with
    /// no live axis registers no scroll sense, so the delta reaches the area around it, which the
    /// single-topmost rule would otherwise end here (D176). The offset still clamps on a disabled
    /// axis, so an area that shrinks to fit returns to 0.
    pub(crate) fn unless_fits(self, extent: f64, viewport: f64) -> Self {
        if extent > viewport {
            self
        } else {
            Self::Disabled
        }
    }
}

/// What a scroll area is given besides its rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollOptions {
    /// The horizontal and the vertical bar.
    pub bars: (Bars, Bars),
    /// The app's "same document" key. A new key starts at the top; the same key keeps the offset
    /// through frames with a shorter extent (a reload) and restores it. `None` never restores.
    pub identity: Option<u64>,
    /// When the extent grows, go to the end (the vertical offset).
    pub follow_tail: bool,
    /// The content's extent when the app knows it before drawing: the clamp then runs before the
    /// content (D7); otherwise the content returns it and the clamp runs after.
    pub extent: Option<(f64, f64)>,
    /// A vertical wheel or pixel delta scrolls the horizontal axis, so a plain wheel works on a
    /// strip that only scrolls sideways. The larger of a gesture's two components wins.
    pub wheel_x: bool,
}

impl Default for ScrollOptions {
    fn default() -> Self {
        Self {
            bars: (Bars::Auto, Bars::Auto),
            identity: None,
            follow_tail: false,
            extent: None,
            wheel_x: false,
        }
    }
}

/// What the content closure is told: the viewport (in the area's own space) and the offset the
/// content is drawn at. Content coordinates have their origin at the viewport's top-left corner
/// (offset 0): a box drawn at `viewport.y + 100` is at `y = 100` for [`Ui::scroll_to`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollView {
    pub viewport: Rect,
    pub offset: (f64, f64),
}

/// The size of the content, as the content measured it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Extent {
    pub width: f64,
    pub height: f64,
}

impl Extent {
    /// An extent of `width` by `height`.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

/// What [`Ui::scroll_area`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollResponse<R> {
    /// What the content closure returned.
    pub value: R,
    /// The area's own region (the wheel target).
    pub response: Response,
    /// The offset after this frame's input, clamped to the extent.
    pub offset: (f64, f64),
    /// The extent this frame used.
    pub extent: (f64, f64),
    pub viewport: Rect,
}

/// How a target is brought into view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reveal {
    /// Centre it: `start - (viewport - len) / 2` (the spec's formula).
    Center,
    /// Move the least that shows it all.
    Nearest,
}

/// A box in content coordinates, per axis a `(start, length)` or nothing (that axis stays put).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Target {
    pub x: Option<(f64, f64)>,
    pub y: Option<(f64, f64)>,
}

impl Target {
    /// A box on both axes.
    pub fn rect(r: Rect) -> Self {
        Self {
            x: Some((f64::from(r.x), f64::from(r.w))),
            y: Some((f64::from(r.y), f64::from(r.h))),
        }
    }

    /// A span on the vertical axis only, such as a list row: `index * row_height`, `row_height`.
    pub fn span_y(start: f64, len: f64) -> Self {
        Self {
            x: None,
            y: Some((start, len)),
        }
    }

    /// A span on the horizontal axis only.
    pub fn span_x(start: f64, len: f64) -> Self {
        Self {
            x: Some((start, len)),
            y: None,
        }
    }
}

/// The offset range's end: `extent - viewport`, at least 0 (0 for anything not finite).
pub fn max_offset(extent: f64, viewport: f64) -> f64 {
    if !extent.is_finite() || !viewport.is_finite() {
        return 0.0;
    }
    (extent - viewport).max(0.0)
}

/// `offset` clamped to `[0, max(0, extent - viewport)]`; 0 for anything not finite or not
/// positive (the reference app's `ClampRestoreOffset`).
pub fn clamp_offset(offset: f64, extent: f64, viewport: f64) -> f64 {
    if !offset.is_finite() || offset <= 0.0 {
        return 0.0;
    }
    offset.min(max_offset(extent, viewport))
}

/// Whether a rebind restores the previous offset: the area has rendered, the new key is `Some` and
/// equals the last one (`None` never restores). The reference app's `ShouldRestore`.
pub fn should_restore(has_rendered: bool, last_key: Option<u64>, new_key: Option<u64>) -> bool {
    has_rendered && new_key.is_some() && new_key == last_key
}

/// The offset a restore writes, or `None` for no write: a saved offset of 0 or less is not
/// restored, and a change under 0.5 px from `current` is not written.
pub fn restore_offset(saved: f64, extent: f64, viewport: f64, current: f64) -> Option<f64> {
    if !saved.is_finite() || saved <= 0.0 {
        return None;
    }
    let target = clamp_offset(saved, extent, viewport);
    ((current - target).abs() > 0.5).then_some(target)
}

/// The offset that centres the row `index` of height `row` in a viewport, clamped to the range:
/// `index * row - (viewport - row) / 2`. The reference app's `ComputeCenteredOffsetY`.
pub fn centre_target(index: usize, row: f64, viewport: f64, extent: f64) -> f64 {
    let raw = index as f64 * row - (viewport - row) / 2.0;
    if raw.is_nan() {
        return 0.0;
    }
    raw.clamp(0.0, max_offset(extent, viewport))
}

/// The offset that shows the span `start .. start + len` in a viewport that is at `current`,
/// before the clamp. A span taller than the viewport aligns its start unless it already covers it.
pub fn reveal_offset(start: f64, len: f64, viewport: f64, current: f64, how: Reveal) -> f64 {
    match how {
        Reveal::Center => start - (viewport - len) / 2.0,
        Reveal::Nearest => {
            let end = start + len;
            if len >= viewport {
                if start <= current && end >= current + viewport {
                    current
                } else {
                    start
                }
            } else if start < current {
                start
            } else if end > current + viewport {
                end - viewport
            } else {
                current
            }
        }
    }
}

/// One axis' offset: where the user or app wants it (`wanted`, never lowered by a short frame of
/// a preserved document), and the animation towards it.
#[derive(Debug, Clone, Copy)]
struct AxisState {
    wanted: f64,
    /// The exact base the animation is relative to.
    base: f64,
    /// The animated part: the shown offset is `base + rel`.
    rel: Transition<f32>,
}

impl Default for AxisState {
    fn default() -> Self {
        Self {
            wanted: 0.0,
            base: 0.0,
            rel: Transition::settled(0.0),
        }
    }
}

impl AxisState {
    /// Rebases onto `wanted` once the animation has ended, so a settled offset is exact.
    fn settle(&mut self, now: Time) {
        if !self.rel.in_flight(now) {
            self.base = self.wanted;
            self.rel = Transition::settled(0.0);
        }
    }

    fn value(&self, now: Time) -> (f64, bool) {
        let (rel, moving) = self.rel.sample(now);
        (self.base + f64::from(rel), moving)
    }

    /// Moves the whole offset by `by`, keeping any animation's course relative to it.
    fn shift(&mut self, by: f64) {
        self.wanted += by;
        self.base += by;
    }

    /// Sets the offset at once, cancelling any animation.
    fn jump(&mut self, to: f64) {
        self.wanted = to;
        self.base = to;
        self.rel = Transition::settled(0.0);
    }

    /// Moves to `to` per `motion` (retargeting keeps the velocity), or at once without one.
    fn step(&mut self, now: Time, to: f64, motion: Option<Resolved>) {
        match motion {
            Some(r) => {
                self.wanted = to;
                self.rel
                    .retarget(now, (to - self.base) as f32, r.duration, r.easing);
            }
            None => self.jump(to),
        }
    }
}

/// A scroll link's retained state (D8), under the group's id: the value the panes share, the
/// value each pane last took, and how many panes drew this frame.
#[derive(Debug, Default)]
struct LinkState {
    value: Option<f64>,
    taken: std::collections::BTreeMap<WidgetId, f64>,
    frame: u64,
    drawn: u32,
}

/// The retained state of one scroll area.
#[derive(Debug, Default)]
struct ScrollState {
    /// Horizontal, then vertical.
    a: [AxisState; 2],
    /// The extent the previous frame's content had.
    extent: [f64; 2],
    key: Option<u64>,
    rendered: bool,
    /// The focused child the last frame saw inside the area.
    last_focus: Option<WidgetId>,
}

/// `Ui::scroll_to` and `Ui::scroll_shift` requests, taken by the area on its next call.
#[derive(Debug, Default)]
struct Requests {
    reveals: Vec<(Target, Reveal)>,
    shift_x: f64,
    shift_y: f64,
    /// What the area left behind when it last ran.
    ran: Option<Ran>,
}

/// One area's axes as its last call ended, so a request made later in the same frame can tell
/// whether it changes the target.
#[derive(Debug, Clone, Copy)]
struct Ran {
    frame: u64,
    live: [bool; 2],
    wanted: [f64; 2],
    ext: [f64; 2],
    vp: [f64; 2],
}

impl Ran {
    /// Whether `target` moves either axis from where the area is heading.
    fn moved_by(&self, target: &Target, how: Reveal) -> bool {
        [target.x, target.y].iter().enumerate().any(|(ax, span)| {
            let cur = clamp_offset(self.wanted[ax], self.ext[ax], self.vp[ax]);
            self.live[ax] && reveal_axis(*span, cur, self.ext[ax], self.vp[ax], how).is_some()
        })
    }
}

/// The offset a reveal of `span` (start and length, in content coordinates) sends an axis to from
/// `cur`, or `None` when it changes nothing: no span, one outside the content, or already showing.
fn reveal_axis(span: Option<(f64, f64)>, cur: f64, ext: f64, vp: f64, how: Reveal) -> Option<f64> {
    let (start, len) = span?;
    if !start.is_finite() || !len.is_finite() || len < 0.0 || start >= ext || start + len < 0.0 {
        return None;
    }
    let to = clamp_offset(reveal_offset(start, len, vp, cur, how), ext, vp);
    (to != cur).then_some(to)
}

/// `r`, drawn in the area's space, as a box in content coordinates: the content's origin is the
/// viewport's top-left corner at offset 0.
fn content_rect(r: Rect, viewport: Rect) -> Rect {
    Rect::new(r.x - viewport.x, r.y - viewport.y, r.w, r.h)
}

/// Whether `inner` lies wholly inside `outer`, both in one space.
fn within(inner: Rect, outer: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.right() <= outer.right()
        && inner.bottom() <= outer.bottom()
}

/// What an area hands on to the area around it for a child at `child`: its viewport on the axes it
/// scrolls itself. On a translated axis it cannot move (its offset there is 0), only the outer area
/// can bring the child into view, so the child's own span goes on (a tall table that scrolls only
/// sideways, in a document). An untranslated area's child is a region of its own, not content.
fn handed_on(child: Rect, viewport: Rect, live: [bool; 2], translate: bool) -> Rect {
    let span = |axis_live: bool, own: (f32, f32), child: (f32, f32)| {
        if axis_live || !translate { own } else { child }
    };
    let (x, w) = span(live[0], (viewport.x, viewport.w), (child.x, child.w));
    let (y, h) = span(live[1], (viewport.y, viewport.h), (child.y, child.h));
    Rect::new(x, y, w, h)
}

fn clean(v: f64) -> f64 {
    if v.is_finite() && v > 0.0 { v } else { 0.0 }
}

impl Ui<'_> {
    /// A scroll area under `key` over `rect`. `content` draws in content coordinates (the area
    /// translates it by the negative offset, clipped to `rect`) and returns its value and its
    /// [`Extent`]; with [`ScrollOptions::extent`] given, the returned extent is ignored.
    pub fn scroll_area<R>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &ScrollOptions,
        content: impl FnOnce(&mut Self, &ScrollView) -> (R, Extent),
    ) -> ScrollResponse<R> {
        self.scroll_area_with(key, rect, opts, true, content)
    }

    /// [`Self::scroll_area`] without the translation: the content places itself from
    /// [`ScrollView::offset`] (a virtual list subtracts in `f64` and casts the small result, which
    /// an `f32` translation could not do at a million rows). The clip, the input, the thumb and the
    /// node are the same. A keyboard focus move to a child that shows only in part brings it into
    /// view by the least movement, as in a translated area; a child placed over the whole viewport
    /// (a list's own region) has nothing to reveal, and a scroll-into-view stays the content's
    /// business.
    pub fn scroll_area_untranslated<R>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &ScrollOptions,
        content: impl FnOnce(&mut Self, &ScrollView) -> (R, Extent),
    ) -> ScrollResponse<R> {
        self.scroll_area_with(key, rect, opts, false, content)
    }

    /// Asks the scroll area under `key` (of the current scope) to bring `target`, a box in its
    /// content coordinates, into view. Applied when the area is next called (this frame if it
    /// comes later, or from inside its content). A target outside the content changes nothing.
    ///
    /// Made after the area in the frame, it asks for the frame that applies it, unless the target
    /// already shows, so an app may call this every frame and still settle to `Repaint::Idle`.
    pub fn scroll_to(&mut self, key: impl Hash, target: Target, how: Reveal) {
        let id = self.id(key);
        let frame = self.frame_number();
        let requests = self.transient::<Requests>(id);
        requests.reveals.push((target, how));
        let late = requests
            .ran
            .is_some_and(|r| r.frame == frame && r.moved_by(&target, how));
        if late {
            self.request_catch_up();
        }
    }

    /// Asks the area under `key` (of the current scope) to move its vertical offset by `by` when
    /// it is next called, before anything else: the content above the viewport changed height by
    /// that much, so what is on screen stays put. Any animation keeps its course.
    pub(crate) fn scroll_shift(&mut self, key: impl Hash, by: f64) {
        let id = self.id(key);
        self.transient::<Requests>(id).shift_y += by;
    }

    /// The vertical shift [`Self::scroll_shift`] has queued for the area under `key` and the area
    /// has not taken yet. A pane of a scroll link adds it to the offset it drew, since the area
    /// moves by it first: asked of the group's value without it, a pane that measured the same
    /// rows as the pane that published would move twice (D8).
    pub(crate) fn scroll_pending_shift(&mut self, key: impl Hash) -> f64 {
        let id = self.id(key);
        self.transient::<Requests>(id).shift_y
    }

    /// Drops the shifts queued for the area under `key`. A host calls it on a new identity, before
    /// it queues anything of its own: what the last frame left there measured the last document.
    pub(crate) fn scroll_forget_shift(&mut self, key: impl Hash) {
        let id = self.id(key);
        let requests = self.transient::<Requests>(id);
        requests.shift_x = 0.0;
        requests.shift_y = 0.0;
    }

    /// [`Self::scroll_shift`] on the horizontal axis: an edit moves its caret into view at once,
    /// since typing and caret motion never animate.
    pub(crate) fn scroll_shift_x(&mut self, key: impl Hash, by: f64) {
        let id = self.id(key);
        self.transient::<Requests>(id).shift_x += by;
    }

    /// The state of the scroll link `group`, with the asking pane counted among the frame's.
    fn link_drawn(&mut self, group: WidgetId) -> &mut LinkState {
        let frame = self.frame_number();
        let link = self.persistent::<LinkState>(group);
        if link.frame != frame {
            link.frame = frame;
            link.drawn = 0;
        }
        link.drawn += 1;
        link
    }

    /// A pane of a scroll link (D8) asks what to apply before its area runs: the shift that takes
    /// `own_offset`, where the last frame left it, to the group's value. Zero when the group has
    /// no value yet, when the pane already took that value (a pane that clamps does not ask every
    /// frame), or when they are equal. A `first` draw (the pane's state is new: its first frame, or
    /// the first after it was not drawn and its state swept) ignores what it took before, since
    /// `taken` lives under the group and outlasts the pane: it joins where the group stands. The
    /// pane is the current scope.
    pub(crate) fn scroll_link_pull(
        &mut self,
        group: WidgetId,
        own_offset: f64,
        first: bool,
    ) -> f64 {
        let pane = self.scope_id();
        let link = self.link_drawn(group);
        let Some(value) = link.value else {
            return 0.0;
        };
        if !first && link.taken.get(&pane) == Some(&value) {
            return 0.0;
        }
        link.taken.insert(pane, value);
        if value.is_finite() {
            value - own_offset
        } else {
            0.0
        }
    }

    /// A pane that opened another document asks this in place of [`Self::scroll_link_pull`]. It
    /// takes nothing, since the group's value measured the last document, and that value goes,
    /// wherever the pane stood: the group starts over at the top with it (D203). A pane that drew
    /// the old value before this one asks a catch-up frame. A first draw opens nothing; such a
    /// pane pulls, as any pane that joins.
    pub(crate) fn scroll_link_open(&mut self, group: WidgetId) {
        let link = self.link_drawn(group);
        let left = link.value.replace(0.0);
        let stale = link.drawn > 1 && left.is_some_and(|v| v != 0.0);
        if stale {
            self.request_catch_up();
        }
    }

    /// A pane tells the group where it ended: `before` is what it drew last frame, `after` what it
    /// is at now. Only a move publishes; equal values do nothing, so a pane that took the group's
    /// value does not echo it. A pane that moved after another one already drew this frame asks a
    /// catch-up frame, since that one drew the old value.
    pub(crate) fn scroll_link_push(&mut self, group: WidgetId, before: f64, after: f64) {
        if before == after {
            return;
        }
        let pane = self.scope_id();
        let link = self.persistent::<LinkState>(group);
        link.value = Some(after);
        link.taken.insert(pane, after);
        let stale = link.drawn > 1;
        if stale {
            self.request_catch_up();
        }
    }

    fn scroll_area_with<R>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &ScrollOptions,
        translate: bool,
        content: impl FnOnce(&mut Self, &ScrollView) -> (R, Extent),
    ) -> ScrollResponse<R> {
        let id = self.id(key);
        let now = self.time();
        let live = [opts.bars.0 != Bars::Disabled, opts.bars.1 != Bars::Disabled];
        let resp = self.interact(
            id,
            rect,
            if live[0] || live[1] {
                Sense::SCROLL
            } else {
                Sense::default()
            },
        );
        // A sideways-wheel strip reads the whole delta as its own axis (`sideways`), so it takes
        // both; any other area takes an axis only where it is live (D178).
        self.set_scroll_axes(if opts.wheel_x {
            (live[0], live[0])
        } else {
            (live[0], live[1])
        });
        let vp = [f64::from(rect.w), f64::from(rect.h)];
        let mut st = std::mem::take(self.persistent::<ScrollState>(id));
        let Requests {
            mut reveals,
            shift_x,
            shift_y,
            ran: _,
        } = std::mem::take(self.transient::<Requests>(id));

        // Identity: a new key starts at the top; the same key keeps `wanted` through short frames.
        if st.rendered && opts.identity != st.key {
            st = ScrollState::default();
        }
        let preserve = should_restore(st.rendered, st.key, opts.identity);
        st.key = opts.identity;
        st.rendered = true;
        st.a[0].shift(shift_x);
        st.a[1].shift(shift_y);

        let known = opts.extent.map(|(w, h)| [clean(w), clean(h)]);
        let ext_pre = known.unwrap_or(st.extent);
        let wheel = self.theme().motion.resolve(&WHEEL, self.tier());
        let into_view = self.theme().motion.resolve(&INTO_VIEW, self.tier());

        // Input: wanted offsets are clamped here, against the extent last known (a wheel step at
        // the end must not leave `wanted` past it).
        if resp.enabled {
            let gesture = |d: (f32, f32)| if opts.wheel_x { sideways(d) } else { d };
            let (scroll_lines, scroll_pixels) =
                (gesture(resp.scroll_lines), gesture(resp.scroll_pixels));
            for ax in 0..2 {
                if !live[ax] {
                    continue;
                }
                let lines = [scroll_lines.0, scroll_lines.1][ax];
                let pixels = [scroll_pixels.0, scroll_pixels.1][ax];
                let a = &mut st.a[ax];
                let mut cur = clamp_offset(a.wanted, ext_pre[ax], vp[ax]);
                if lines.is_finite() && lines != 0.0 {
                    cur = clamp_offset(cur - f64::from(lines) * LINE, ext_pre[ax], vp[ax]);
                    a.step(now, cur, wheel);
                }
                if pixels.is_finite() && pixels != 0.0 {
                    cur = clamp_offset(cur - f64::from(pixels), ext_pre[ax], vp[ax]);
                    a.jump(cur);
                }
            }
            let pax = primary_axis(&live, &ext_pre, &vp);
            for (nk, dir) in [(NamedKey::ArrowUp, 1.0), (NamedKey::ArrowDown, -1.0)] {
                if live[pax] && self.consume_synthetic_key(id, nk) {
                    let a = &mut st.a[pax];
                    let cur = clamp_offset(a.wanted, ext_pre[pax], vp[pax]);
                    let to = clamp_offset(cur + dir * LINE, ext_pre[pax], vp[pax]);
                    a.step(now, to, wheel);
                }
            }
        }
        for a in &mut st.a {
            a.settle(now);
        }
        // A preserved document keeps `wanted` through a short frame, but not through a vertical
        // shift: that is the host placing the area, and a `wanted` past the end would hold it
        // there when the next shift brings it back (D203).
        let keep = [preserve, preserve && shift_y == 0.0];
        if let Some(k) = known {
            for ax in 0..2 {
                let c = clamp_offset(st.a[ax].wanted, k[ax], vp[ax]);
                if !keep[ax] && c != st.a[ax].wanted {
                    st.a[ax].jump(c);
                }
            }
        }
        let shown = |st: &ScrollState, ext: &[f64; 2]| -> [f64; 2] {
            [0, 1].map(|ax| clamp_offset(st.a[ax].value(now).0, ext[ax], vp[ax]))
        };
        let pre = shown(&st, &ext_pre);

        // The content, in the clip, translated by the negative offset.
        let view = ScrollView {
            viewport: rect,
            offset: (pre[0], pre[1]),
        };
        let watch_mark = self.watch_mark();
        let reg_mark = self.registration_mark();
        let mut node = Node::new(id, Role::ScrollArea, "", rect);
        if !resp.enabled {
            node.states.insert(State::Disabled);
        }
        let (value, measured) = self.node_container(node, |ui| {
            ui.painter().push_clip(rect);
            if translate {
                ui.painter()
                    .push_translate(-(pre[0] as f32), -(pre[1] as f32));
            }
            let out = content(ui, &view);
            if translate {
                ui.painter().pop_translate();
            }
            ui.painter().pop_clip();
            out
        });
        let focusable_child = self.focusable_since(reg_mark);
        let watched = self.watched_since(watch_mark);
        // Requests the content made: nothing else would run them on a quiet frame.
        reveals.append(&mut self.transient::<Requests>(id).reveals);

        // The measured extent: follow-tail, then the clamp (a preserved document keeps `wanted`).
        let ext = known.unwrap_or([clean(measured.width), clean(measured.height)]);
        if opts.follow_tail && live[1] && ext[1] > st.extent[1] {
            st.a[1].jump(max_offset(ext[1], vp[1]));
        }
        st.extent = ext;
        if !preserve {
            for ax in 0..2 {
                let c = clamp_offset(st.a[ax].wanted, ext[ax], vp[ax]);
                if c != st.a[ax].wanted {
                    st.a[ax].jump(c);
                }
            }
        }

        // Reveals: the app's requests, an accessibility scroll-into-view on a child (centred) and
        // a keyboard focus move to a child (the least movement).
        let (focused, ring) = self.focus_state();
        let inside = focused.and_then(|f| watched.iter().rev().find(|(w, _)| *w == f).copied());
        if translate {
            for (w, r) in &watched {
                if self.is_scroll_target(*w) {
                    reveals.push((Target::rect(content_rect(*r, rect)), Reveal::Center));
                }
            }
            if let Some((f, r)) = inside
                && ring
                && st.last_focus != Some(f)
            {
                reveals.push((Target::rect(content_rect(r, rect)), Reveal::Nearest));
            }
        } else if let Some((f, r)) = inside
            && ring
            && st.last_focus != Some(f)
            && !within(r, rect)
        {
            // Drawn at the offset the content was given, in the viewport's own space.
            let at = |start: f32, origin: f32, ax: usize| f64::from(start - origin) + pre[ax];
            reveals.push((
                Target {
                    x: Some((at(r.x, rect.x, 0), f64::from(r.w))),
                    y: Some((at(r.y, rect.y, 1), f64::from(r.h))),
                },
                Reveal::Nearest,
            ));
        }
        st.last_focus = inside.map(|(f, _)| f);
        for (target, how) in reveals {
            for ax in 0..2 {
                let a = &mut st.a[ax];
                let cur = clamp_offset(a.wanted, ext[ax], vp[ax]);
                let span = [target.x, target.y][ax];
                let Some(to) = reveal_axis(span, cur, ext[ax], vp[ax], how).filter(|_| live[ax])
                else {
                    continue;
                };
                if (to - cur).abs() > SNAP_DISTANCE * vp[ax] {
                    a.jump(to);
                } else {
                    a.step(now, to, into_view);
                }
            }
        }

        // Page keys and Home and End, when the focused child inside the area declined them;
        // while it composes they are the input method's.
        // A focused child in a disabled scope is not watched (it registered inert), but its keys
        // are still the area's to take: they are consumed and nothing scrolls.
        let keyed = inside.map(|(f, _)| (f, true)).or_else(|| {
            focused
                .filter(|f| self.disabled_registered_since(reg_mark, *f))
                .map(|f| (f, false))
        });
        if let Some((f, acts)) = keyed.filter(|_| !self.composing()) {
            let pax = primary_axis(&live, &ext, &vp);
            if live[pax] && ext[pax] > vp[pax] {
                for (index, ev) in self.focus_events(f) {
                    let FocusEvent::Key {
                        key: Key::Named(k),
                        modifiers,
                        ..
                    } = ev
                    else {
                        continue;
                    };
                    if modifiers.control || modifiers.alt || modifiers.logo {
                        continue;
                    }
                    let a = &mut st.a[pax];
                    let cur = clamp_offset(a.wanted, ext[pax], vp[pax]);
                    let to = match k {
                        NamedKey::PageUp => cur - vp[pax],
                        NamedKey::PageDown => cur + vp[pax],
                        NamedKey::Home => 0.0,
                        NamedKey::End => max_offset(ext[pax], vp[pax]),
                        _ => continue,
                    };
                    if acts {
                        a.jump(clamp_offset(to, ext[pax], vp[pax]));
                    }
                    self.consume_focus_event(index);
                }
            }
        }

        // The overlay bars, hit after the content so a press on a thumb reaches no child.
        let now_off = shown(&st, &ext);
        let bar = [
            opts.bars.0 == Bars::Auto && live[0],
            opts.bars.1 == Bars::Auto && live[1],
        ];
        let has_y =
            bar[1] && thumb_geometry(Axis::Vertical, rect, now_off[1], ext[1], false).is_some();
        let has_x =
            bar[0] && thumb_geometry(Axis::Horizontal, rect, now_off[0], ext[0], false).is_some();
        let reveal = self.pointer_in(rect);
        if has_y && let Some(g) = thumb_geometry(Axis::Vertical, rect, now_off[1], ext[1], has_x) {
            let drag = self.scrollbar(id.with("bar-y"), Axis::Vertical, &g, reveal);
            if drag != 0.0 {
                let cur = clamp_offset(st.a[1].wanted, ext[1], vp[1]);
                st.a[1].jump(clamp_offset(cur + g.offset_delta(drag), ext[1], vp[1]));
            }
        }
        if has_x && let Some(g) = thumb_geometry(Axis::Horizontal, rect, now_off[0], ext[0], has_y)
        {
            let drag = self.scrollbar(id.with("bar-x"), Axis::Horizontal, &g, reveal);
            if drag != 0.0 {
                let cur = clamp_offset(st.a[0].wanted, ext[0], vp[0]);
                st.a[0].jump(clamp_offset(cur + g.offset_delta(drag), ext[0], vp[0]));
            }
        }

        // Settled or moving; a change after the content drew needs a catch-up frame.
        if st.a.iter().any(|a| a.value(now).1) {
            self.mark_moving();
        }
        let fin = shown(&st, &ext);
        if fin != pre {
            self.request_catch_up();
        }

        let nax = usize::from(ext[1] > vp[1] || ext[0] <= vp[0]);
        let max = max_offset(ext[nax], vp[nax]);
        let scrollable = resp.enabled && (0..2).any(|ax| live[ax] && ext[ax] > vp[ax]);
        let off = fin[nax];
        self.patch_node(id, |n| {
            n.value = Some(Value::Number {
                value: off,
                min: 0.0,
                max,
                step: LINE,
            });
            if scrollable {
                n.actions.insert(Action::Increment);
                n.actions.insert(Action::Decrement);
            }
            if focusable_child {
                n.states.insert(State::Focusable);
            }
        });
        // A nested area hands the widgets it dealt with to the outer one as its own viewport.
        let keep: Vec<(WidgetId, Rect)> = watched
            .iter()
            .map(|(w, r)| (*w, handed_on(*r, rect, live, translate)))
            .collect();
        self.watch_replace(watch_mark, keep);

        let ran = Ran {
            frame: self.frame_number(),
            live,
            wanted: [st.a[0].wanted, st.a[1].wanted],
            ext,
            vp,
        };
        self.transient::<Requests>(id).ran = Some(ran);
        *self.persistent::<ScrollState>(id) = st;
        ScrollResponse {
            value,
            response: resp,
            offset: (fin[0], fin[1]),
            extent: (ext[0], ext[1]),
            viewport: rect,
        }
    }
}

/// A gesture `d` turned onto the horizontal axis: its larger component, so a plain wheel scrolls
/// sideways and a trackpad's small vertical drift does not fight a horizontal swipe.
fn sideways(d: (f32, f32)) -> (f32, f32) {
    (if d.1.abs() > d.0.abs() { d.1 } else { d.0 }, 0.0)
}

/// The axis keys and the node's actions scroll: the vertical one unless only the horizontal one
/// overflows.
fn primary_axis(live: &[bool; 2], ext: &[f64; 2], vp: &[f64; 2]) -> usize {
    usize::from(!(live[0] && ext[0] > vp[0] && !(live[1] && ext[1] > vp[1])))
}

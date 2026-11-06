//! The virtualized list (`layout.virtualized-list`): only the rows in view are built, on a
//! [`scroll area`](super::scroll_area) that knows the whole extent. Selection is a set of
//! [`RowKey`]s, so a row keeps it when it scrolls away or the app replaces the data.
//!
//! Rows are placed with an `f64` subtraction and a cast of the small result (D6): `top(i)` and the
//! offset can be tens of millions of pixels, where an `f32` steps in whole pixels. That is why the
//! list sits on [`Ui::scroll_area_untranslated`].
//!
//! Heights ([`RowHeights`]): fixed rows are arithmetic; measured rows are a Fenwick tree of height
//! differences over an estimate, so `top(i)` and the row at an offset are O(log n). A frame's work
//! depends on the viewport, not on the row count: the only pass over the rows is the tree's when
//! the estimate changes or rows are added (amortised O(1) a row).
//!
//! Frame order: the keys move the cursor before anything draws, so the reveal request reaches the
//! area this frame. Inside the area, every built row registers its region and takes its input, and
//! only then do the rows draw: a click moves the selection of all rows in the same frame, and a
//! child widget's region lands above its row's. A measured height takes effect on the next frame
//! (one catch-up frame); a change above the first visible row shifts the offset by as much, so
//! that row stays where it is on screen.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use crate::access::{Action, Node, Role, State};
use crate::color::{Rgba, with_alpha};
use crate::input::{FocusEvent, Key, Modifiers, NamedKey};
use crate::motion::{Kind, Motion};
use crate::paint::Rect;
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui, WidgetId};

use super::focus_ring::{self, RingKind};
use super::scroll_area::{Bars, Extent, Reveal, ScrollOptions, Target};

/// Row fill on hover, while the list is still.
pub const HOVER: Motion = Motion::of(Kind::Hover);
/// Row fill on a selection change, while the list is still.
pub const SELECTION: Motion = Motion::of(Kind::Selection);
/// Rows built beyond the viewport at each end (D10).
pub const MARGIN: usize = 2;
/// The highlight's inset inside the row slot, left and right, then top and bottom.
pub const HIGHLIGHT_INSET: (f32, f32) = (4.0, 1.0);
/// The smallest height a row counts as: a row of no height could never be reached to measure it.
pub const MIN_ROW: f32 = 1.0;

/// The inner scroll area's key, under the list's scope.
const AREA: &str = "area";

/// What distinguishes a list frame that is not the plain list (the table runs on it).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ListStyle {
    /// The role of the frame's node.
    pub(crate) role: Role,
    /// The role of each row's node.
    pub(crate) row_role: Role,
    /// Rows fill their slot edge to edge at radius 0, and hover reads the pointer over the slot
    /// (so a control in a cell keeps its row lit).
    pub(crate) flat: bool,
    /// The width of the rows' content when it can exceed the viewport: the x axis is then live
    /// (a bar that shows only when it overflows) and [`RowCtx::scroll_x`] says how far the rows'
    /// content has scrolled. `None`: no horizontal scroll.
    pub(crate) content_w: Option<f64>,
    /// Whether the list paints the hover and selection fill. `false`: the row's closure draws it
    /// (from [`RowCtx::fill`]); the focus ring stays the list's.
    pub(crate) fill: bool,
    /// The app's "same document" key, given to the area; a new key clears the list's offset,
    /// measured heights and selection.
    pub(crate) identity: Option<u64>,
    /// A scroll link (D8): lists with the same group share their vertical offset.
    pub(crate) link: Option<WidgetId>,
}

impl ListStyle {
    /// The virtualized list as it is.
    pub(crate) const LIST: Self = Self {
        role: Role::List,
        row_role: Role::ListItem,
        flat: false,
        content_w: None,
        fill: true,
        identity: None,
        link: None,
    };
}

/// A row's identity: stable across scrolling and replaced data, unlike its index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowKey(pub u64);

/// How tall the rows are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Heights {
    /// Every row is this tall; what the row's `draw` returns is ignored.
    Fixed(f32),
    /// A row is `estimate` tall until it has been drawn, then what its `draw` returned.
    Measured { estimate: f32 },
}

/// What the pointer and keys may select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// Nothing: the cursor still moves.
    None,
    /// One row.
    Single,
    /// A set of rows: the primary modifier toggles, Shift and a drag select a range.
    Multi,
}

/// What a list is given besides its rect.
#[derive(Clone, Copy)]
pub struct ListOptions<'a> {
    pub count: usize,
    pub heights: Heights,
    pub selection: SelectionMode,
    /// Finds a key's row after the data changed. Without it, a range from an anchor that is no
    /// longer at its last known index falls back to a single row.
    pub index_of: Option<&'a dyn Fn(RowKey) -> Option<usize>>,
    /// The list's accessible name.
    pub label: &'a str,
}

impl std::fmt::Debug for ListOptions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListOptions")
            .field("count", &self.count)
            .field("heights", &self.heights)
            .field("selection", &self.selection)
            .field("index_of", &self.index_of.is_some())
            .field("label", &self.label)
            .finish()
    }
}

/// What a row's `draw` is told. The list has already painted the highlight (`fill`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowCtx {
    pub index: usize,
    pub key: RowKey,
    /// The row's slot.
    pub rect: Rect,
    pub hovered: bool,
    pub selected: bool,
    /// The keyboard cursor is on this row.
    pub cursor: bool,
    /// The cursor is here and the focus ring shows.
    pub focus_visible: bool,
    /// The highlight colour this frame.
    pub fill: Rgba,
    /// How far the rows' content has scrolled sideways, in px: 0 unless the frame has a content
    /// width (the slot itself does not move).
    pub scroll_x: f32,
}

/// What [`Ui::virtual_list`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct ListResponse {
    /// The list's own region, the Tab stop. `changed` when the selection changed this frame.
    pub response: Response,
    /// The row a secondary click landed on.
    pub context: Option<RowKey>,
    /// The primary modifier with C: the selected keys in key order, for the app to format.
    pub copy: Option<Vec<RowKey>>,
    /// The offset after this frame's input.
    pub offset: f64,
    /// The height of all rows this frame used.
    pub extent: f64,
}

/// A list's retained selection: persistent, so it outlives the rows it names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListState {
    pub selection: BTreeSet<RowKey>,
    /// Where a range starts.
    pub anchor: Option<RowKey>,
    /// The keyboard cursor's row.
    pub cursor: Option<RowKey>,
    /// The index each of them had when last seen.
    anchor_at: usize,
    cursor_at: usize,
}

impl ListState {
    /// Keeps only the keys `keep` admits: the app calls this when rows left its data. An anchor or
    /// cursor on a dropped key clears.
    pub fn retain(&mut self, mut keep: impl FnMut(RowKey) -> bool) {
        self.selection.retain(|k| keep(*k));
        self.anchor = self.anchor.filter(|k| keep(*k));
        self.cursor = self.cursor.filter(|k| keep(*k));
    }

    /// Replaces the selection; whether that changed it.
    fn select(&mut self, set: BTreeSet<RowKey>) -> bool {
        let changed = set != self.selection;
        self.selection = set;
        changed
    }

    fn set_anchor(&mut self, index: usize, key: RowKey) {
        self.anchor = Some(key);
        self.anchor_at = index;
    }

    fn set_cursor(&mut self, index: usize, key: RowKey) {
        self.cursor = Some(key);
        self.cursor_at = index;
    }

    /// The anchor's row this frame: its last known index while that still holds its key, else what
    /// `index_of` says.
    fn anchor_index(&mut self, rows: &Rows<'_>) -> Option<usize> {
        let at = rows.locate(self.anchor?, self.anchor_at)?;
        self.anchor_at = at;
        Some(at)
    }

    /// The cursor's row this frame; a cursor that cannot be found stays at its old position.
    fn cursor_index(&mut self, rows: &Rows<'_>) -> Option<usize> {
        let key = self.cursor?;
        let at = rows
            .locate(key, self.cursor_at)
            .unwrap_or_else(|| self.cursor_at.min(rows.count().saturating_sub(1)));
        self.cursor_at = at;
        Some(at)
    }

    /// Selects only `index` and anchors there.
    fn select_one(&mut self, index: usize, rows: &Rows<'_>) -> bool {
        let key = (rows.key)(index);
        self.set_anchor(index, key);
        rows.mode() != SelectionMode::None && self.select(BTreeSet::from([key]))
    }

    /// Selects from the anchor to `index`, replacing the set; a single row when the anchor is gone.
    /// Costs the range's length, so it runs when the range changes, never per frame.
    fn select_range(&mut self, index: usize, rows: &Rows<'_>) -> bool {
        let Some(anchor) = self.anchor_index(rows) else {
            return self.select_one(index, rows);
        };
        let span = anchor.min(index)..=anchor.max(index);
        self.select(span.map(rows.key).collect())
    }

    /// A press on `index`, or an accessibility default action (no modifiers).
    fn press(&mut self, index: usize, m: Modifiers, primary: bool, rows: &Rows<'_>) -> bool {
        let key = (rows.key)(index);
        self.set_cursor(index, key);
        if rows.mode() == SelectionMode::Multi {
            if m.shift {
                return self.select_range(index, rows);
            }
            if primary {
                self.set_anchor(index, key);
                if !self.selection.remove(&key) {
                    self.selection.insert(key);
                }
                return true;
            }
        }
        self.select_one(index, rows)
    }

    /// The cursor moved to `index` by a key; `extend` is Shift.
    fn key_move(&mut self, index: usize, extend: bool, rows: &Rows<'_>) -> bool {
        self.set_cursor(index, (rows.key)(index));
        if extend && rows.mode() == SelectionMode::Multi {
            self.select_range(index, rows)
        } else {
            self.select_one(index, rows)
        }
    }
}

/// The rows' geometry: fixed arithmetic, or measured heights over an estimate. Positions are `f64`.
///
/// Measured rows keep each row's height (`own`, 0 until measured) and a Fenwick tree (`tree`, node
/// `i` sums the height differences from the estimate of the rows `(i - lowbit(i), i]`), so
/// [`Self::top`], [`Self::row_at`] and [`Self::set`] are O(log n). Rows `0..m` only touch nodes up
/// to `m`, which is why dropping rows is a truncation and adding them only computes the new nodes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RowHeights {
    count: usize,
    /// The fixed height or the estimate.
    unit: f64,
    measured: bool,
    own: Vec<f32>,
    tree: Vec<f64>,
}

fn lowbit(i: usize) -> usize {
    i & i.wrapping_neg()
}

/// A usable row height from what the app passed.
fn usable(h: f32) -> f64 {
    f64::from(if h.is_finite() {
        h.max(MIN_ROW)
    } else {
        MIN_ROW
    })
}

impl RowHeights {
    /// Brings the geometry up to this frame's `count` and `heights`. Free when neither changed.
    pub fn sync(&mut self, count: usize, heights: Heights) {
        let (unit, measured) = match heights {
            Heights::Fixed(h) => (usable(h), false),
            Heights::Measured { estimate } => (usable(estimate), true),
        };
        if measured != self.measured || unit != self.unit {
            self.measured = measured;
            self.unit = unit;
            self.tree.clear();
            if !measured {
                self.own = Vec::new();
                self.tree = Vec::new();
            }
        }
        self.count = count;
        if measured {
            self.own.resize(count, 0.0);
            self.tree.truncate(count + 1);
            self.grow_tree();
        }
    }

    /// Computes the tree's missing nodes: each adds its row's difference and the child nodes it
    /// covers, an amortised O(1) each.
    fn grow_tree(&mut self) {
        if self.tree.is_empty() {
            self.tree.push(0.0);
        }
        self.tree.reserve(self.own.len() + 1 - self.tree.len());
        for i in self.tree.len()..=self.own.len() {
            let mut sum = self.difference(i - 1);
            let stop = i - lowbit(i);
            let mut j = i - 1;
            while j > stop {
                sum += self.tree[j];
                j -= lowbit(j);
            }
            self.tree.push(sum);
        }
    }

    fn difference(&self, row: usize) -> f64 {
        match self.own[row] {
            0.0 => 0.0,
            h => f64::from(h) - self.unit,
        }
    }

    /// The summed height differences of rows `0..end`.
    fn prefix(&self, mut end: usize) -> f64 {
        let mut sum = 0.0;
        if self.measured {
            while end > 0 {
                sum += self.tree[end];
                end -= lowbit(end);
            }
        }
        sum
    }

    /// The number of rows.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Where row `i` starts; `top(count)` is the end of the last row.
    pub fn top(&self, i: usize) -> f64 {
        i as f64 * self.unit + self.prefix(i)
    }

    /// Row `i`'s height.
    pub fn height(&self, i: usize) -> f64 {
        match self.own.get(i) {
            Some(h) if *h > 0.0 => f64::from(*h),
            _ => self.unit,
        }
    }

    /// The height of all rows.
    pub fn total(&self) -> f64 {
        self.top(self.count)
    }

    /// The row holding `y`: the last row with `top <= y`, in `0..count` (0 for no rows or a `y`
    /// above the first).
    pub fn row_at(&self, y: f64) -> usize {
        if self.count == 0 || y.is_nan() || y <= 0.0 {
            return 0;
        }
        let last = self.count - 1;
        if !self.measured {
            let mut i = ((y / self.unit) as usize).min(last);
            while i > 0 && self.top(i) > y {
                i -= 1;
            }
            while i < last && self.top(i + 1) <= y {
                i += 1;
            }
            return i;
        }
        let (mut at, mut sum) = (0, 0.0);
        let mut step = 1 << self.count.ilog2();
        while step > 0 {
            let next = at + step;
            if next <= self.count {
                let top = next as f64 * self.unit + sum + self.tree[next];
                if top <= y {
                    at = next;
                    sum += self.tree[next];
                }
            }
            step >>= 1;
        }
        at.min(last)
    }

    /// Records what row `i` measured and returns the change in its height: 0 for fixed rows, a
    /// row out of range or a height that is not finite.
    pub fn set(&mut self, i: usize, h: f32) -> f64 {
        if !self.measured || i >= self.count || !h.is_finite() {
            return 0.0;
        }
        let h = h.max(MIN_ROW);
        let change = f64::from(h) - self.height(i);
        self.own[i] = h;
        if change != 0.0 {
            let mut j = i + 1;
            while j <= self.count {
                self.tree[j] += change;
                j += lowbit(j);
            }
        }
        change
    }

    /// The rows to build for a viewport of `viewport` at `offset`: those in view and [`MARGIN`]
    /// above and below, one fewer below when the first row is cut, so uniform rows of height `h`
    /// never build more than `ceil(viewport / h) + 2 * MARGIN`.
    pub fn window(&self, offset: f64, viewport: f64) -> std::ops::Range<usize> {
        if self.count == 0 {
            return 0..0;
        }
        let first = self.row_at(offset);
        let top = self.top(first);
        let (mut end, mut y) = (first, top);
        while end < self.count && y < offset + viewport {
            y += self.height(end);
            end += 1;
        }
        let below = if top == offset { MARGIN } else { MARGIN - 1 };
        first.saturating_sub(MARGIN)..(end + below).min(self.count)
    }
}

/// This call of the list: its id and rect, and the app's rows.
struct Rows<'r> {
    id: WidgetId,
    rect: Rect,
    opts: &'r ListOptions<'r>,
    style: ListStyle,
    key: &'r dyn Fn(usize) -> RowKey,
}

impl Rows<'_> {
    fn count(&self) -> usize {
        self.opts.count
    }

    fn mode(&self) -> SelectionMode {
        self.opts.selection
    }

    /// `key`'s row: `last` while it still holds that key, else what `index_of` finds.
    fn locate(&self, key: RowKey, last: usize) -> Option<usize> {
        if last < self.count() && (self.key)(last) == key {
            return Some(last);
        }
        let found = self.opts.index_of.and_then(|f| f(key))?;
        (found < self.count()).then_some(found)
    }
}

/// The settled fill of a row.
pub fn row_fill(theme: &Theme, hovered: bool, selected: bool) -> Rgba {
    match (selected, hovered) {
        (true, true) => theme.surface_active,
        (true, false) => theme.surface_selected,
        (false, true) => theme.surface_hover,
        (false, false) => with_alpha(theme.surface_hover, 0.0),
    }
}

/// What a list keeps besides its selection.
#[derive(Debug, Default)]
struct ListMem {
    heights: RowHeights,
    /// The offset and rect the last frame drew at.
    offset: f64,
    rect: Option<Rect>,
    /// The offset the last frame's area ended at: past `offset` when a reveal landed after the
    /// rows drew.
    ended: f64,
    /// A height changed last frame, so this frame's rows move.
    relaid: bool,
    /// The content moved last frame: this frame's hover is still that movement's result.
    was_moved: bool,
    /// The row whose press this list already handled, until the release.
    press: Option<RowKey>,
    /// The row a drag selection last reached.
    drag_end: Option<usize>,
    /// The identity the rows belong to.
    identity: Option<u64>,
    /// How many identities followed the first this list drew: a row's id holds it, so what a row
    /// and its children kept (a focused button) goes with the document.
    epoch: u64,
}

/// What a row's id and its children's scope are keyed by: the key alone until a new identity, so a
/// list's ids do not depend on it having one.
#[derive(Clone, Copy)]
struct RowScope {
    epoch: u64,
    key: RowKey,
}

impl Hash for RowScope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.epoch != 0 {
            self.epoch.hash(state);
        }
        self.key.hash(state);
    }
}

impl ListMem {
    /// Records this frame's position; whether row state may tween: the content did not move this
    /// frame or the one before (a hover is read against the last frame's regions, so it lags the
    /// move by a frame).
    fn still(&mut self, offset: f64, rect: Rect) -> bool {
        let moved = offset != self.offset || self.rect.is_some_and(|r| r != rect) || self.relaid;
        let still = !moved && !self.was_moved;
        self.offset = offset;
        self.rect = Some(rect);
        self.relaid = false;
        self.was_moved = moved;
        still
    }
}

/// A built row between registering and drawing.
struct Built {
    id: WidgetId,
    ctx: RowCtx,
    selected_before: bool,
}

/// The row's last drawn selection, which picks the motion of a fill change.
#[derive(Debug, Default)]
struct RowMem {
    selected: bool,
}

/// What the frame's input produced, for the response.
#[derive(Debug, Default)]
struct Outcome {
    changed: bool,
    context: Option<RowKey>,
    copy: Option<Vec<RowKey>>,
}

impl Ui<'_> {
    /// The retained selection of the list under `key` (of the current scope). The app may change
    /// it; [`ListState::retain`] prunes keys that left its data.
    pub fn list_state(&mut self, key: impl Hash) -> &mut ListState {
        let id = self.id(key);
        self.persistent::<ListState>(id)
    }

    /// A list under `key` over `rect`, building only the rows in view. `row_key` names the row at
    /// an index; `draw` draws a row inside its slot, under the row's own id scope and semantic
    /// node, and returns its height (ignored for [`Heights::Fixed`]). The row's accessible name
    /// comes from what `draw` emits inside it.
    pub fn virtual_list(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &ListOptions<'_>,
        row_key: impl Fn(usize) -> RowKey,
        draw: impl FnMut(&mut Self, &RowCtx) -> f32,
    ) -> ListResponse {
        self.list_styled(key, rect, opts, ListStyle::LIST, row_key, draw)
    }

    /// [`Self::virtual_list`] in another [`ListStyle`].
    pub(crate) fn list_styled(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &ListOptions<'_>,
        style: ListStyle,
        row_key: impl Fn(usize) -> RowKey,
        mut draw: impl FnMut(&mut Self, &RowCtx) -> f32,
    ) -> ListResponse {
        self.scope(key, |ui| {
            ui.list_frame(rect, opts, style, &row_key, &mut draw)
        })
    }

    fn list_frame(
        &mut self,
        rect: Rect,
        opts: &ListOptions<'_>,
        style: ListStyle,
        row_key: &dyn Fn(usize) -> RowKey,
        draw: &mut dyn FnMut(&mut Self, &RowCtx) -> f32,
    ) -> ListResponse {
        let id = self.scope_id();
        let mut mem = std::mem::take(self.persistent::<ListMem>(id));
        let mut st = std::mem::take(self.persistent::<ListState>(id));
        mem.heights.sync(opts.count, opts.heights);
        let rows = Rows {
            id,
            rect,
            opts,
            style,
            key: row_key,
        };
        let mut out = Outcome::default();
        let mut opened = false;
        if style.identity != mem.identity {
            // Another document: nothing of the last one carries over. A first draw replaces none.
            opened = mem.rect.is_some();
            self.scroll_forget_shift(AREA);
            mem = ListMem {
                identity: style.identity,
                epoch: mem.epoch + u64::from(opened),
                ..ListMem::default()
            };
            mem.heights.sync(opts.count, opts.heights);
            st = ListState::default();
        }

        // Always: a list focused and then disabled takes its keys and acts on none.
        let enabled = self.is_enabled();
        self.list_keys(&mut st, &mem, &rows, enabled, &mut out);

        // A linked pane takes the group's offset before its area runs, from where it stands: the
        // last frame's end and this pane's own queued shift (a measured height above the first
        // row). One that opened a document takes nothing, and the group starts over with it.
        let drawn_at = mem.offset;
        let pending = self.scroll_pending_shift(AREA);
        let pulled = match style.link {
            Some(group) if opened => {
                self.scroll_link_open(group);
                0.0
            }
            Some(group) => self.scroll_link_pull(group, mem.ended + pending, mem.rect.is_none()),
            None => 0.0,
        };
        if pulled != 0.0 {
            self.scroll_shift(AREA, pulled);
        }
        let area = ScrollOptions {
            bars: (
                style.content_w.map_or(Bars::Disabled, |w| {
                    Bars::Auto.unless_fits(w, f64::from(rect.w))
                }),
                Bars::Auto.unless_fits(mem.heights.total(), f64::from(rect.h)),
            ),
            extent: Some((
                style.content_w.unwrap_or(f64::from(rect.w)),
                mem.heights.total(),
            )),
            identity: style.identity,
            ..ScrollOptions::default()
        };
        let scroll = self.scroll_area_untranslated(AREA, rect, &area, |ui, view| {
            let mut node = Node::new(id, style.role, opts.label, rect);
            let enabled = ui.is_enabled();
            node.states.insert(if enabled {
                State::Focusable
            } else {
                State::Disabled
            });
            let response = ui.node_container(node, |ui| {
                ui.list_rows(view.offset, st, &mut mem, &rows, draw, &mut out)
            });
            (response, Extent::default())
        });
        mem.ended = scroll.offset.1;
        *self.persistent::<ListMem>(id) = mem;
        // Only a move of its own publishes; a pane that was just pulled follows, and says nothing.
        if let Some(group) = style.link
            && pulled == 0.0
        {
            self.scroll_link_push(group, drawn_at, scroll.offset.1);
        }

        let mut response = scroll.value;
        if out.changed {
            response.mark_changed();
        }
        ListResponse {
            response,
            context: out.context,
            copy: out.copy,
            offset: scroll.offset.1,
            extent: scroll.extent.1,
        }
    }

    /// Takes the frame's keys: the cursor moves before the area runs, so its reveal lands this
    /// frame.
    ///
    /// Every key the list reads is consumed; the cursor, the selection, the reveal and the copy
    /// happen only when `enabled`.
    fn list_keys(
        &mut self,
        st: &mut ListState,
        mem: &ListMem,
        rows: &Rows<'_>,
        enabled: bool,
        out: &mut Outcome,
    ) {
        let last = match rows.count() {
            0 => return,
            n => n - 1,
        };
        let h = &mem.heights;
        let viewport = f64::from(rows.rect.h);
        let mut moved = None;
        for (index, ev) in self.focus_events(rows.id) {
            let FocusEvent::Key { key, modifiers, .. } = ev else {
                continue;
            };
            let primary = self.keymap().is_primary(modifiers);
            if let Key::Character(c) = &key {
                if primary
                    && !modifiers.shift
                    && !modifiers.alt
                    && c.eq_ignore_ascii_case("c")
                    && !st.selection.is_empty()
                {
                    if enabled {
                        out.copy = Some(st.selection.iter().copied().collect());
                    }
                    self.consume_focus_event(index);
                }
                continue;
            }
            let Key::Named(k) = key else { continue };
            if modifiers.control || modifiers.alt || modifiers.logo {
                continue;
            }
            // With no cursor yet, the first move lands on the first visible row.
            let cursor = st.cursor_index(rows);
            let from = cursor.unwrap_or_else(|| h.row_at(mem.offset));
            let page = |down: bool| {
                let y = if down {
                    h.top(from) + viewport
                } else {
                    h.top(from) - viewport
                };
                let to = h.row_at(y);
                match (to == from, down) {
                    (true, true) => (from + 1).min(last),
                    (true, false) => from.saturating_sub(1),
                    _ => to,
                }
            };
            let to = match k {
                NamedKey::ArrowUp if cursor.is_some() => from.saturating_sub(1),
                NamedKey::ArrowDown if cursor.is_some() => (from + 1).min(last),
                NamedKey::ArrowUp | NamedKey::ArrowDown => from,
                NamedKey::PageUp => page(false),
                NamedKey::PageDown => page(true),
                NamedKey::Home => 0,
                NamedKey::End => last,
                _ => continue,
            };
            self.consume_focus_event(index);
            if enabled {
                out.changed |= st.key_move(to, modifiers.shift, rows);
                moved = Some(to);
            }
        }
        if let Some(i) = moved {
            self.scroll_to(AREA, Target::span_y(h.top(i), h.height(i)), Reveal::Nearest);
        }
    }

    /// One frame of the rows inside the area: registers them, applies their input, then draws.
    fn list_rows(
        &mut self,
        view_offset: (f64, f64),
        mut st: ListState,
        mem: &mut ListMem,
        rows: &Rows<'_>,
        draw: &mut dyn FnMut(&mut Self, &RowCtx) -> f32,
        out: &mut Outcome,
    ) -> Response {
        let (id, rect, style) = (rows.id, rows.rect, rows.style);
        let (scroll_x, offset) = (view_offset.0 as f32, view_offset.1);
        let response = self.interact(id, rect, Sense::CLICK_FOCUS);
        let still = mem.still(offset, rect);
        let viewport = f64::from(rect.h);
        let window = mem.heights.window(offset, viewport);
        let first = mem.heights.row_at(offset);
        let keymap = self.keymap();

        // Register and take input, so the whole window draws the frame's final selection.
        let mut slots = Vec::with_capacity(window.len());
        let mut pressing = false;
        for index in window {
            let key = (rows.key)(index);
            let row_id = self.id(RowScope {
                epoch: mem.epoch,
                key,
            });
            let y = (mem.heights.top(index) - offset) as f32 + rect.y;
            let slot = Rect::new(rect.x, y, rect.w, mem.heights.height(index) as f32);
            let r = self.interact(row_id, slot, Sense::DRAG);
            if r.press_started {
                let primary = keymap.is_primary(r.press_modifiers);
                out.changed |= st.press(index, r.press_modifiers, primary, rows);
                mem.press = Some(key);
                mem.drag_end = Some(index);
                self.request_focus(id);
            }
            // A click with no press of ours is an accessibility default action.
            if r.clicked && mem.press != Some(key) {
                out.changed |= st.press(index, Modifiers::default(), false, rows);
            }
            pressing |= r.pressed;
            if r.dragged && rows.mode() == SelectionMode::Multi {
                // The pointer is in window coordinates, `rect` in the caller's space.
                let top = self.painter().transformed(rect).y;
                let to = self.pointer_pos().map_or(index, |(_, y)| {
                    let inside = f64::from(y - top).clamp(0.0, viewport);
                    mem.heights.row_at(offset + inside)
                });
                if mem.drag_end != Some(to) {
                    mem.drag_end = Some(to);
                    st.set_cursor(to, (rows.key)(to));
                    out.changed |= st.select_range(to, rows);
                }
            }
            if r.secondary_clicked {
                out.context = Some(key);
            }
            if r.focused {
                // An accessibility focus action: the list holds the focus, the cursor moves.
                st.set_cursor(index, key);
                self.request_focus(id);
            }
            if r.scroll_into_view {
                let (top, height) = (mem.heights.top(index), mem.heights.height(index));
                self.scroll_to(AREA, Target::span_y(top, height), Reveal::Center);
            }
            let hovered = if style.flat {
                r.enabled && self.pointer_in(slot)
            } else {
                r.hovered
            };
            slots.push((index, key, row_id, slot, hovered));
        }
        if !pressing {
            mem.press = None;
        }

        // The rows' final state.
        let (focused, ring) = self.focus_state();
        let ring = ring && focused == Some(id);
        let theme = *self.theme();
        let mut built = Vec::with_capacity(slots.len());
        for (index, key, row_id, slot, hovered) in slots {
            let selected = st.selection.contains(&key);
            let cursor = st.cursor.map_or(ring && index == first, |c| c == key);
            let selected_before =
                std::mem::replace(&mut self.transient::<RowMem>(row_id).selected, selected);
            built.push(Built {
                id: row_id,
                selected_before,
                ctx: RowCtx {
                    index,
                    key,
                    rect: slot,
                    hovered,
                    selected,
                    cursor,
                    focus_visible: cursor && ring,
                    fill: row_fill(&theme, hovered, selected),
                    scroll_x,
                },
            });
        }
        // A row's `draw` may read the state; everything after this only reads `built`.
        *self.persistent::<ListState>(id) = st;

        let enabled = response.enabled;
        let (mut relaid, mut shift) = (false, 0.0);
        for b in built {
            let mut ctx = b.ctx;
            // While the content moves a row snaps: skipping the animation also drops any in flight.
            if still {
                let motion = if ctx.selected != b.selected_before {
                    &SELECTION
                } else {
                    &HOVER
                };
                ctx.fill = self.animate_color(b.id.with("fill"), ctx.fill, motion);
            }
            let (highlight, radius) = if style.flat {
                (ctx.rect, 0.0)
            } else {
                (
                    ctx.rect.inset(HIGHLIGHT_INSET.0, HIGHLIGHT_INSET.1),
                    theme.item_radius,
                )
            };
            if style.fill && ctx.fill[3] != 0 {
                self.painter().fill(highlight, radius, ctx.fill);
            }

            let mut node = Node::new(b.id, style.row_role, "", ctx.rect);
            if ctx.selected {
                node.states.insert(State::Selected);
            }
            if enabled {
                for action in [
                    Action::Default,
                    Action::Focus,
                    Action::ScrollIntoView,
                    Action::ShowMenu,
                ] {
                    node.actions.insert(action);
                }
            } else {
                node.states.insert(State::Disabled);
            }
            let scope = RowScope {
                epoch: mem.epoch,
                key: ctx.key,
            };
            // A margin row is built and clipped: nothing on it is a Tab stop.
            let hidden = ctx.rect.bottom() <= rect.y || ctx.rect.y >= rect.bottom();
            let height = self.off_tab_order(hidden, |ui| {
                ui.node_container(node, |ui| ui.scope(scope, |ui| draw(ui, &ctx)))
            });
            if ctx.focus_visible {
                focus_ring::paint(self.painter(), highlight, RingKind::Button, 1.0);
            }

            let change = mem.heights.set(ctx.index, height);
            if change != 0.0 {
                relaid = true;
                if ctx.index < first {
                    shift += change;
                }
            }
        }
        if relaid {
            mem.relaid = true;
            if shift != 0.0 {
                self.scroll_shift(AREA, shift);
            }
            self.request_catch_up();
        }
        response
    }
}

/// The frame's crate-private fields (`ListStyle`'s `content_w`, `fill`, `identity` and `link`,
/// `RowCtx::scroll_x`) at non-default values. The suites in `tests/` hold
/// the defaults; the diff view proves these again from outside.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::premul;
    use crate::input::{Key, PointerButton, Repaint};
    use crate::scene::Scene;
    use crate::testing::UiHarness;
    use crate::theme::DARK;

    const WIN: (f32, f32) = (500.0, 300.0);
    const RECT: Rect = Rect::new(20.0, 20.0, 200.0, 100.0);
    const RIGHT: Rect = Rect::new(260.0, 20.0, 200.0, 100.0);
    const ROW: f32 = 20.0;
    const COUNT: usize = 60;

    fn opts(count: usize) -> ListOptions<'static> {
        ListOptions {
            count,
            heights: Heights::Fixed(ROW),
            selection: SelectionMode::Single,
            index_of: None,
            label: "rows",
        }
    }

    struct Seen {
        rows: Vec<RowCtx>,
        resp: ListResponse,
    }

    /// One pane of a frame.
    fn pane(ui: &mut Ui<'_>, rect: Rect, count: usize, style: ListStyle) -> Seen {
        let mut rows = Vec::new();
        let resp = ui.list_styled(
            ("l", rect.x as i32),
            rect,
            &opts(count),
            style,
            |i| RowKey(i as u64),
            |_, c| {
                rows.push(*c);
                ROW
            },
        );
        Seen { rows, resp }
    }

    /// One frame with the left pane.
    fn frame(h: &mut UiHarness, style: ListStyle) -> (Seen, Scene) {
        let mut seen = None;
        let (scene, _) = h.frame(|ui| seen = Some(pane(ui, RECT, COUNT, style)));
        (seen.expect("the frame ran"), scene)
    }

    /// One frame with both panes, left drawn first; the repaint request comes with it.
    fn both(
        h: &mut UiHarness,
        styles: (ListStyle, ListStyle),
        counts: (usize, usize),
    ) -> (Seen, Seen, Repaint) {
        let mut seen = (None, None);
        let (_, out) = h.frame(|ui| {
            seen.0 = Some(pane(ui, RECT, counts.0, styles.0));
            seen.1 = Some(pane(ui, RIGHT, counts.1, styles.1));
        });
        (
            seen.0.expect("left drew"),
            seen.1.expect("right drew"),
            out.repaint,
        )
    }

    fn shapes(scene: &Scene) -> usize {
        scene
            .layers
            .iter()
            .map(|l| l.geometry.under.len() + l.geometry.over.len())
            .sum()
    }

    fn fills_of(scene: &Scene, colors: &[Rgba]) -> usize {
        let cs: Vec<_> = colors.iter().map(|c| premul(*c)).collect();
        scene
            .layers
            .iter()
            .flat_map(|l| l.geometry.under.iter())
            .filter(|s| cs.contains(&s.color))
            .count()
    }

    #[test]
    fn a_content_width_makes_the_x_axis_live_with_its_bar() {
        let wide = ListStyle {
            content_w: Some(600.0),
            ..ListStyle::LIST
        };
        let mut seen = Vec::new();
        for style in [ListStyle::LIST, wide] {
            let mut h = UiHarness::new(WIN);
            frame(&mut h, style);
            h.input.pointer_to(100.0, 60.0);
            frame(&mut h, style);
            h.input.scroll_pixels(-50.0, 0.0);
            frame(&mut h, style);
            // Past the bar's fade-in.
            h.at_ms(300);
            let (s, scene) = frame(&mut h, style);
            seen.push((s.rows[0].scroll_x, shapes(&scene)));
        }
        assert_eq!(seen[0].0, 0.0, "no content width, no sideways scroll");
        assert_eq!(seen[1].0, 50.0, "the wheel moved the rows' content");
        assert!(seen[1].1 > seen[0].1, "the x bar draws: {seen:?}");
    }

    #[test]
    fn a_content_that_fits_the_width_has_no_x_scroll() {
        let fits = ListStyle {
            content_w: Some(150.0),
            ..ListStyle::LIST
        };
        let mut h = UiHarness::new(WIN);
        frame(&mut h, fits);
        h.input.pointer_to(100.0, 60.0);
        frame(&mut h, fits);
        h.input.scroll_pixels(-50.0, 0.0);
        assert_eq!(frame(&mut h, fits).0.rows[0].scroll_x, 0.0);
    }

    #[test]
    fn no_fill_paints_no_highlight_but_keeps_the_fill_value_and_the_ring() {
        let quiet = ListStyle {
            fill: false,
            ..ListStyle::LIST
        };
        let tones = [DARK.surface_selected, DARK.surface_active];
        let mut result = Vec::new();
        for style in [ListStyle::LIST, quiet] {
            let mut h = UiHarness::new(WIN);
            frame(&mut h, style);
            h.input
                .pointer_to(100.0, 30.0)
                .click(PointerButton::Primary);
            frame(&mut h, style);
            h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
            frame(&mut h, style);
            h.at_ms(500);
            let (s, scene) = frame(&mut h, style);
            let selected = s.rows.iter().find(|r| r.selected).expect("a selected row");
            assert_ne!(selected.fill[3], 0, "the row is still given its fill");
            assert!(s.rows.iter().any(|r| r.focus_visible), "the ring shows");
            let strokes: usize = scene.layers.iter().map(|l| l.geometry.over.len()).sum();
            result.push((fills_of(&scene, &tones), strokes));
        }
        assert!(result[0].0 > 0, "the instrument sees the fill: {result:?}");
        assert_eq!(result[1].0, 0, "no fill: {result:?}");
        assert_eq!(result[0].1, result[1].1, "the ring stays: {result:?}");
        assert!(result[1].1 > 0, "and there is a ring: {result:?}");
    }

    fn selected(h: &mut UiHarness) -> usize {
        let mut n = 0;
        h.frame(|ui| n = ui.list_state(("l", RECT.x as i32)).selection.len());
        n
    }

    #[test]
    fn a_new_identity_clears_the_offset_and_the_selection() {
        let at = |id| ListStyle {
            identity: Some(id),
            ..ListStyle::LIST
        };
        let mut h = UiHarness::new(WIN);
        frame(&mut h, at(1));
        h.input
            .pointer_to(100.0, 30.0)
            .click(PointerButton::Primary);
        frame(&mut h, at(1));
        h.input.scroll_pixels(0.0, -200.0);
        assert_eq!(frame(&mut h, at(1)).0.resp.offset, 200.0);
        assert_eq!(
            frame(&mut h, at(1)).0.resp.offset,
            200.0,
            "the same key keeps it"
        );
        assert_eq!(selected(&mut h), 1, "the selection is kept with the key");
        assert_eq!(
            frame(&mut h, at(2)).0.resp.offset,
            0.0,
            "a new key starts at the top"
        );
        assert_eq!(selected(&mut h), 0, "and unselected");
    }

    #[test]
    fn tab_onto_a_child_past_the_right_edge_moves_the_list_sideways() {
        let style = ListStyle {
            content_w: Some(800.0),
            ..ListStyle::LIST
        };
        let mut child = None;
        let mut frame = |h: &mut UiHarness| {
            let mut x = 0.0;
            let (_, out) = h.frame(|ui| {
                ui.list_styled(
                    "l",
                    RECT,
                    &opts(COUNT),
                    style,
                    |i| RowKey(i as u64),
                    |ui, row| {
                        x = row.scroll_x;
                        if row.index == 1 {
                            let id = ui.id("child");
                            child = Some(id);
                            let left = row.rect.x + 600.0 - row.scroll_x;
                            let at = Rect::new(left, row.rect.y, 40.0, ROW);
                            ui.interact(id, at, Sense::CLICK_FOCUS);
                            ui.node(Node::new(id, Role::Button, "child", at));
                        }
                        ROW
                    },
                );
            });
            (x, out)
        };
        let mut h = UiHarness::new(WIN);
        frame(&mut h);
        frame(&mut h);
        for _ in 0..2 {
            h.input.key_down(Key::Named(NamedKey::Tab), None);
            frame(&mut h);
        }
        let (mut x, mut out) = frame(&mut h);
        for ms in 1..30 {
            if out.repaint == Repaint::Idle {
                break;
            }
            h.at_ms(ms * 100);
            (x, out) = frame(&mut h);
        }
        assert_eq!(out.focused, child, "the child took the focus");
        assert_eq!(x, 640.0 - RECT.w, "its right edge at the view's");
    }

    fn linked() -> ListStyle {
        ListStyle {
            link: Some(WidgetId::ROOT.with("group")),
            ..ListStyle::LIST
        }
    }

    fn offsets(h: &mut UiHarness, styles: (ListStyle, ListStyle)) -> (f64, f64, Repaint) {
        let (a, b, repaint) = both(h, styles, (COUNT, COUNT));
        (a.resp.offset, b.resp.offset, repaint)
    }

    #[test]
    fn a_wheel_on_the_first_pane_moves_the_second_in_the_same_frame() {
        let mut h = UiHarness::new(WIN);
        let styles = (linked(), linked());
        offsets(&mut h, styles);
        h.input.pointer_to(100.0, 60.0);
        offsets(&mut h, styles);
        h.input.scroll_pixels(0.0, -120.0);
        let (a, b, _) = offsets(&mut h, styles);
        assert_eq!((a, b), (120.0, 120.0));
    }

    #[test]
    fn a_wheel_on_the_second_pane_moves_the_first_on_the_catch_up_frame() {
        let mut h = UiHarness::new(WIN);
        let styles = (linked(), linked());
        offsets(&mut h, styles);
        h.input.pointer_to(300.0, 60.0);
        offsets(&mut h, styles);
        h.input.scroll_pixels(0.0, -120.0);
        let (a, b, repaint) = offsets(&mut h, styles);
        assert_eq!(b, 120.0);
        assert_eq!(a, 0.0, "the first drew before the move");
        assert_eq!(repaint, Repaint::Now, "a catch-up frame was asked");
        let (a, b, _) = offsets(&mut h, styles);
        assert_eq!((a, b), (120.0, 120.0));
        h.at_ms(500);
        let (_, _, repaint) = offsets(&mut h, styles);
        assert_eq!(repaint, Repaint::Idle, "equal values ask nothing");
    }

    #[test]
    fn a_pane_not_drawn_for_three_frames_catches_up_when_drawn() {
        let mut h = UiHarness::new(WIN);
        let styles = (linked(), linked());
        offsets(&mut h, styles);
        h.input.pointer_to(100.0, 60.0);
        offsets(&mut h, styles);
        h.input.scroll_pixels(0.0, -120.0);
        // The right pane drops out of the page for the moves that follow.
        let alone = |h: &mut UiHarness| {
            let mut seen = None;
            h.frame(|ui| seen = Some(pane(ui, RECT, COUNT, linked())));
            seen.expect("drew").resp.offset
        };
        assert_eq!(alone(&mut h), 120.0);
        h.input.scroll_pixels(0.0, -40.0);
        assert_eq!(alone(&mut h), 160.0);
        assert_eq!(alone(&mut h), 160.0);
        let (a, b, _) = offsets(&mut h, styles);
        assert_eq!((a, b), (160.0, 160.0), "it caught up when drawn again");
    }

    #[test]
    fn a_line_wheel_on_the_second_pane_rests_with_both_panes_equal() {
        let mut h = UiHarness::new(WIN);
        let styles = (linked(), linked());
        offsets(&mut h, styles);
        h.input.pointer_to(300.0, 60.0);
        offsets(&mut h, styles);
        h.input.scroll_lines(0.0, -3.0);
        let mut ms = 0;
        for _ in 0..200 {
            ms += 16;
            h.at_ms(ms);
            let (a, b, repaint) = offsets(&mut h, styles);
            if repaint == Repaint::Idle {
                assert!(b > 0.0, "the wheel moved it");
                assert_eq!(a, b, "the panes rest equal");
                return;
            }
        }
        panic!("the page did not settle");
    }

    /// One frame of a list whose rows `with_child` carry a focusable box; the rows' offset, the
    /// focus and the child's box when it is focused.
    fn child_frame(
        h: &mut UiHarness,
        with_child: &[usize],
    ) -> (f64, Option<WidgetId>, Option<Rect>) {
        let mut seen = (0.0, None);
        let (_, out) = h.frame(|ui| {
            let resp = ui.virtual_list(
                "l",
                RECT,
                &opts(COUNT),
                |i| RowKey(i as u64),
                |ui, c| {
                    if with_child.contains(&c.index) {
                        let id = ui.id("child");
                        let at = Rect::new(c.rect.x + 10.0, c.rect.y + 2.0, 40.0, 16.0);
                        let r = ui.interact(id, at, Sense::CLICK_FOCUS);
                        if r.focused {
                            seen.1 = Some(at);
                        }
                    }
                    ROW
                },
            );
            seen.0 = resp.offset;
        });
        (seen.0, out.focused, seen.1)
    }

    fn resting(
        h: &mut UiHarness,
        ms: &mut u64,
        rows: &[usize],
    ) -> (f64, Option<WidgetId>, Option<Rect>) {
        child_frame(h, rows);
        *ms += 2_000;
        h.at_ms(*ms);
        child_frame(h, rows)
    }

    #[test]
    fn tab_reveals_a_child_on_a_row_and_a_margin_rows_child_is_no_stop() {
        let list = WidgetId::ROOT.with("l");
        let rows = [5, 6];
        let mut h = UiHarness::new(WIN);
        let mut ms = 0;
        resting(&mut h, &mut ms, &rows);
        // At offset 10 of rows of 20 in a view of 100: row 5 shows in part, row 6 is in the margin.
        h.input.pointer_to(100.0, 60.0).scroll_pixels(0.0, -10.0);
        resting(&mut h, &mut ms, &rows);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        assert_eq!(resting(&mut h, &mut ms, &rows).1, Some(list));
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        let (offset, _, child) = resting(&mut h, &mut ms, &rows);
        let child = child.expect("the child on row 5 is a stop");
        assert!(child.bottom() <= RECT.bottom(), "{child:?}");
        assert_eq!(offset, 18.0, "the least movement");
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        assert_eq!(resting(&mut h, &mut ms, &rows).1, Some(list), "not row 6's");
    }

    #[test]
    fn tab_onto_the_list_in_a_wheels_flight_leaves_the_flight_alone() {
        let mut h = UiHarness::new(WIN);
        let s = ListStyle::LIST;
        frame(&mut h, s);
        h.input.pointer_to(100.0, 60.0);
        frame(&mut h, s);
        h.input.scroll_lines(0.0, -3.0);
        frame(&mut h, s);
        h.at_ms(30);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        frame(&mut h, s);
        h.at_ms(5_000);
        frame(&mut h, s);
        assert_eq!(
            frame(&mut h, s).0.resp.offset,
            144.0,
            "the wheel's whole step"
        );
    }

    #[test]
    fn panes_without_a_group_scroll_alone() {
        let mut h = UiHarness::new(WIN);
        let styles = (ListStyle::LIST, ListStyle::LIST);
        offsets(&mut h, styles);
        h.input.pointer_to(100.0, 60.0);
        offsets(&mut h, styles);
        h.input.scroll_pixels(0.0, -120.0);
        let (a, b, _) = offsets(&mut h, styles);
        assert_eq!((a, b), (120.0, 0.0));
    }

    fn document(identity: u64) -> ListStyle {
        ListStyle {
            identity: Some(identity),
            ..linked()
        }
    }

    /// Both panes at 200 by a wheel on the left, each on document 1.
    fn apart() -> UiHarness {
        let mut h = UiHarness::new(WIN);
        let one = (document(1), document(1));
        offsets(&mut h, one);
        h.input.pointer_to(100.0, 60.0);
        offsets(&mut h, one);
        h.input.scroll_pixels(0.0, -200.0);
        let (a, b, _) = offsets(&mut h, one);
        assert_eq!((a, b), (200.0, 200.0));
        h
    }

    #[test]
    fn a_later_pane_alone_on_a_new_identity_asks_the_frame_that_tops_the_first() {
        let mut h = apart();
        let styles = (document(1), document(2));
        // Long after the wheel: nothing else asks a frame.
        h.at_ms(5_000);
        assert_eq!(offsets(&mut h, (document(1), document(1))).2, Repaint::Idle);
        let (a, b, repaint) = offsets(&mut h, styles);
        assert_eq!((a, b), (200.0, 0.0), "the first drew before it");
        assert_eq!(repaint, Repaint::Now);
        let (a, b, _) = offsets(&mut h, styles);
        assert_eq!((a, b), (0.0, 0.0));
        h.at_ms(500);
        assert_eq!(offsets(&mut h, styles).2, Repaint::Idle);
    }

    #[test]
    fn an_earlier_pane_alone_on_a_new_identity_tops_the_later_one_in_the_frame() {
        let mut h = apart();
        let (a, b, _) = offsets(&mut h, (document(2), document(1)));
        assert_eq!((a, b), (0.0, 0.0));
    }

    #[test]
    fn both_panes_on_a_new_identity_ask_no_frame_of_each_other() {
        let mut h = apart();
        let styles = (document(2), document(2));
        h.at_ms(500);
        let (a, b, repaint) = offsets(&mut h, styles);
        assert_eq!((a, b, repaint), (0.0, 0.0, Repaint::Idle));
    }

    #[test]
    fn a_far_jump_then_the_other_pane_on_a_new_identity_tops_the_pane_that_jumped() {
        // End lands after the content, so the right pane drew 0 and stands at the end: the group's
        // top must be asked of where it stands.
        let mut h = UiHarness::new(WIN);
        let one = (document(1), document(1));
        offsets(&mut h, one);
        h.input
            .pointer_to(300.0, 60.0)
            .click(PointerButton::Primary);
        offsets(&mut h, one);
        h.input.key_down(Key::Named(NamedKey::End), None);
        let (a, b, _) = offsets(&mut h, one);
        assert_eq!((a, b), (0.0, 1100.0), "the left pane drew first");
        let styles = (document(2), document(1));
        let (a, b, _) = offsets(&mut h, styles);
        assert_eq!((a, b), (0.0, 0.0));
        let (a, b, _) = offsets(&mut h, styles);
        assert_eq!(
            (a, b),
            (0.0, 0.0),
            "and the right pane published nothing back"
        );
    }

    #[test]
    fn a_pane_drawn_first_with_an_identity_takes_the_groups_value() {
        // A first draw opens no document: a pane joining a moved group joins it.
        let mut h = UiHarness::new(WIN);
        let alone = |h: &mut UiHarness| {
            let mut seen = None;
            h.frame(|ui| seen = Some(pane(ui, RECT, COUNT, document(1))));
            seen.expect("drew").resp.offset
        };
        alone(&mut h);
        h.input.pointer_to(100.0, 60.0);
        alone(&mut h);
        h.input.scroll_pixels(0.0, -120.0);
        assert_eq!(alone(&mut h), 120.0);
        let (a, b, _) = offsets(&mut h, (document(1), document(1)));
        assert_eq!((a, b), (120.0, 120.0), "in its first frame");
        let (a, b, _) = offsets(&mut h, (document(1), document(1)));
        assert_eq!((a, b), (120.0, 120.0), "and the first pane stays");
    }

    #[test]
    fn a_pane_hidden_while_its_group_stands_still_comes_back_with_the_group() {
        // `taken` lives under the group and outlasts the pane's own state: a first draw joins
        // where the group stands, whether it moved meanwhile or not.
        let styles = (document(1), document(1));
        let mut h = UiHarness::new(WIN);
        offsets(&mut h, styles);
        h.input.pointer_to(100.0, 60.0);
        offsets(&mut h, styles);
        h.input.scroll_pixels(0.0, -120.0);
        assert_eq!(offsets(&mut h, styles).0, 120.0);
        for _ in 0..3 {
            let mut seen = None;
            h.frame(|ui| seen = Some(pane(ui, RECT, COUNT, document(1))));
            assert_eq!(seen.expect("drew").resp.offset, 120.0);
        }
        let (a, b, _) = offsets(&mut h, styles);
        assert_eq!((a, b), (120.0, 120.0), "in the frame it returns");
        h.at_ms(5_000);
        assert_eq!(
            offsets(&mut h, styles).2,
            Repaint::Idle,
            "and the page rests"
        );
    }

    #[test]
    fn a_short_pane_with_an_identity_comes_back_with_its_group() {
        // An area keeps `wanted` for its document: asked past its end, it would stay there.
        let styles = (document(1), document(1));
        let mut h = UiHarness::new(WIN);
        let two = |h: &mut UiHarness| {
            let (a, b, _) = both(h, styles, (COUNT, 8));
            (a.resp.offset, b.resp.offset)
        };
        two(&mut h);
        h.input.pointer_to(100.0, 60.0);
        two(&mut h);
        h.input.scroll_pixels(0.0, -500.0);
        assert_eq!(two(&mut h), (500.0, 60.0));
        h.input.scroll_pixels(0.0, 460.0);
        assert_eq!(two(&mut h), (40.0, 40.0));
    }

    #[test]
    fn a_document_keeps_its_place_through_a_short_frame() {
        let style = ListStyle {
            identity: Some(1),
            ..ListStyle::LIST
        };
        let mut h = UiHarness::new(WIN);
        let at = |h: &mut UiHarness, count: usize| {
            let mut seen = None;
            h.frame(|ui| seen = Some(pane(ui, RECT, count, style)));
            seen.expect("drew").resp.offset
        };
        at(&mut h, COUNT);
        h.input.pointer_to(100.0, 60.0);
        at(&mut h, COUNT);
        h.input.scroll_pixels(0.0, -500.0);
        assert_eq!(at(&mut h, COUNT), 500.0);
        assert_eq!(at(&mut h, 3), 0.0, "the short frame shows its top");
        assert_eq!(at(&mut h, COUNT), 500.0, "and the place comes back");
    }

    #[test]
    fn a_pane_that_cannot_reach_the_value_does_not_ask_every_frame() {
        // The right pane has fewer rows, so it stops below the group's value; it must settle.
        let styles = (linked(), linked());
        let mut h = UiHarness::new(WIN);
        let two = |h: &mut UiHarness| {
            let (a, b, repaint) = both(h, styles, (COUNT, 8));
            (a.resp.offset, b.resp.offset, repaint)
        };
        two(&mut h);
        h.input.pointer_to(100.0, 60.0);
        two(&mut h);
        h.input.scroll_pixels(0.0, -500.0);
        let (a, _, _) = two(&mut h);
        assert_eq!(a, 500.0);
        let (_, b, _) = two(&mut h);
        assert_eq!(b, 60.0, "the short pane stops at its end");
        h.at_ms(500);
        let (_, _, repaint) = two(&mut h);
        assert_eq!(repaint, Repaint::Idle);
        let (a, b, repaint) = two(&mut h);
        assert_eq!((a, b, repaint), (500.0, 60.0, Repaint::Idle));
    }
}

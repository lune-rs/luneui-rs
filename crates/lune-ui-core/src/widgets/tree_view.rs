//! The tree view (`layout.tree-view`, D151): the app's pre-order rows with depths, a keyboard
//! cursor, single selection, guide lines, a disclosure mark and one block of rows in expand
//! motion, on a
//! [`scroll area`](super::scroll_area) that knows the whole extent.
//!
//! State is sets of keys ([`TreeState`], persistent), so expansion and selection outlive a rebuilt
//! row list and rows that scroll away. Each frame one walk over the rows gives the visible ones;
//! everything after it is index arithmetic, and only the rows in view are built. Rows are placed
//! with an `f64` subtraction and a cast of the small result, as in the virtual list, which is why
//! the tree sits on [`Ui::scroll_area_untranslated`].
//!
//! Frame order: the walk reads what changed in `expanded` since the last frame, whoever changed it
//! (a press, an accessibility action, the app), and fixes the cursor; the keys run, each toggle
//! walking again; the area and the rows follow. A press changes `expanded` after the walk, so its
//! block moves from the next frame; a key's moves in the same frame.
//!
//! Expand motion: one key changed between two walks starts the block of its children, a change of
//! the moving key retargets it from where it is, and any other change snaps it. The rows of the
//! block keep their height: they draw under a clip of the block's current height, with no hit
//! regions and no nodes, and `cells` is not called for them, since it could add both. The rows
//! below sit at a closed-form distance, and the extent follows. When the block settles it stops
//! existing, so the frame after is the frame of the same state with no motion. Under a tier with
//! no expand motion it never exists.
//!
//! The disclosure mark (a solid triangle, or a thin chevron: [`ChevronStyle`]) is drawn by the
//! tree, not as a glyph, and turns about its centre from 0 deg (closed, pointing right) to 90 deg
//! (open, pointing down). Its angle is the block's own fraction, so it lands with the rows and
//! needs no second animation: with no block (Reduced, Off, an app change of several rows) it is
//! exactly 0 or 90 deg and nothing is rotated. A press changes `expanded` after the walk, so the
//! mark turns from the next frame with the block. The guides ([`GuideStyle`]) are quiet
//! (`Theme::tree_guide`); `OnHover` fades them in while the pointer is over the tree.

use std::collections::BTreeSet;
use std::hash::Hash;

use super::focus_ring::{self, RingKind};
use super::scroll_area::{Bars, Extent, Reveal, ScrollOptions, Target};
use super::virtual_list::{HOVER, SELECTION, row_fill};
use crate::access::{Action, Node, Role, State};
use crate::color::Rgba;
use crate::icon;
use crate::input::{Event, FocusEvent, Key, NamedKey};
use crate::motion::{Kind, Motion};
use crate::paint::{Painter, Rect, StrokeStyle};
use crate::text::{Spec, Style};
use crate::theme::Theme;
use crate::ui::{Response, Sense, Ui, WidgetId};

/// A row's height.
pub const ROW: f32 = 28.0;
/// A row's corner radius.
pub const ROW_RADIUS: f32 = 6.0;
/// A row's left padding, before its depth's indent.
pub const PAD_LEFT: f32 = 8.0;
/// The indent of one depth.
pub const INDENT: f32 = 16.0;
/// The mark's slot, present on every row when any row has children; also a glyph's size.
pub const CHEVRON: f32 = 14.0;
/// The space after the chevron slot, and after a glyph.
pub const CHEVRON_GAP: f32 = 4.0;
pub const GLYPH_GAP: f32 = 6.0;
/// Where a guide column's line sits inside its indent.
pub const COLUMN_CENTRE: f32 = 8.0;
/// The guide's width.
pub const GUIDE_WIDTH: f32 = 1.0;
/// The elbow's corner radius.
pub const ELBOW_RADIUS: f32 = 3.5;
/// The triangle's ink box when closed (pointing right): width, height.
pub const TRIANGLE: (f32, f32) = (6.0, 8.0);
/// The chevron's arm when closed: half its width and half its height, to the stroke's centre line.
pub const CHEVRON_ARM: (f32, f32) = (2.5, 4.0);
/// The chevron's stroke width.
pub const CHEVRON_STROKE: f32 = 1.25;
/// How far below the row's centre the mark's centre sits: the label's x-height centre line.
pub const MARK_DROP: f32 = 1.25;
/// The child block's clip height: `expand` kind, `base`, `sine-in-out`, snapped in Reduced.
pub const EXPAND: Motion = Motion::of(Kind::Expand);

/// Segments of an elbow's curve.
const ELBOW_STEPS: usize = 6;
/// The inner scroll area's key, under the tree's scope.
const AREA: &str = "area";
/// The expand motion's key, under the tree's id.
const MOTION: &str = "expand";
/// The `OnHover` guides' fade key, under the tree's id.
const FADE: &str = "guides";
/// The `OnHover` guides' fade in and out: `hover` kind, instant, linear; Reduced keeps the fade
/// and Off snaps (`docs/motion.md`, "tree guide fade").
pub const GUIDE_FADE: Motion = Motion::of(Kind::Hover);
/// The `Chevron` mark's brightening on row hover: the same `hover` kind.
pub const MARK_HOVER: Motion = Motion::of(Kind::Hover);

/// One row, in pre-order: a row's children follow it, each one depth deeper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow<'a> {
    /// Stable across rebuilds; unique in the tree.
    pub key: u64,
    /// 0 at the top. A row that rises by more than 1 draws at its depth.
    pub depth: u16,
    pub text: &'a str,
    /// A codicon glyph (`icon::*`) before the text.
    pub glyph: Option<&'a str>,
    /// It can open: the rows after it that are deeper are its children, shown when it is in
    /// [`TreeState::expanded`].
    pub has_children: bool,
}

/// How the guide lines of the rows are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GuideStyle {
    /// A trunk through each ancestor's column and a rounded elbow into the row.
    #[default]
    Elbow,
    /// One faint vertical line per depth, whether or not the ancestor is the last child.
    Straight,
    /// The elbow lines, at an opacity that fades in while the pointer is over the tree.
    OnHover,
}

/// How the disclosure mark of a row with children is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChevronStyle {
    /// A small solid triangle, as a file manager's list view.
    #[default]
    Triangle,
    /// A thin `>` stroke, brighter while the row is hovered.
    Chevron,
}

/// What a tree is given besides its rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeOptions<'a> {
    /// Every row, shown or not. They must be pre-order with depths that rise by at most 1.
    pub rows: &'a [TreeRow<'a>],
    /// The tree's accessible name.
    pub label: &'a str,
    /// Draw the guide lines; off wins over `guide_style`.
    pub guides: bool,
    /// How the guide lines look.
    pub guide_style: GuideStyle,
    /// How the disclosure mark looks.
    pub chevron: ChevronStyle,
    /// A press anywhere on a row with children toggles it, not only on its chevron.
    pub toggle_on_row: bool,
}

/// What `cells` is told, for a row the tree has already drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeRowCtx {
    /// The row's index in [`TreeOptions::rows`].
    pub row: usize,
    pub key: u64,
    /// The row's slot; its right side is `cells`' to draw on.
    pub rect: Rect,
    pub depth: u16,
    pub hovered: bool,
    pub selected: bool,
    /// The keyboard cursor is on this row.
    pub cursor: bool,
}

/// What [`Ui::tree_view`] returns.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeResponse {
    /// The tree's own region, the Tab stop. `changed` when the selection changed this frame.
    pub response: Response,
    /// The row Enter or a double click activated.
    pub activated: Option<u64>,
    /// The row a key, a press or an accessibility action opened or closed this frame.
    pub toggled: Option<u64>,
    /// The row a secondary click landed on.
    pub context: Option<u64>,
}

/// A tree's retained state, in sets of keys: persistent, so it outlives the rows it names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeState {
    /// The open rows. The app may change it: one key changed moves, more snap.
    pub expanded: BTreeSet<u64>,
    /// The selected row, as a set of at most one key. A key that left the rows stays here until
    /// [`Self::retain`].
    pub selected: BTreeSet<u64>,
    /// The keyboard cursor's row, `None` until the tree first holds the focus or a row is pressed.
    /// The tree keeps it on a visible row while any exists: a row hidden or removed gives it to
    /// its nearest visible ancestor.
    pub cursor: Option<u64>,
}

impl TreeState {
    /// Keeps only the keys `keep` admits in `expanded` and `selected`: the app calls this when
    /// rows left its data. The cursor needs no pruning: it looks after itself.
    pub fn retain(&mut self, mut keep: impl FnMut(u64) -> bool) {
        self.expanded.retain(|k| keep(*k));
        self.selected.retain(|k| keep(*k));
    }

    fn toggle(&mut self, key: u64) {
        if !self.expanded.remove(&key) {
            self.expanded.insert(key);
        }
    }

    /// Selects only `key`; whether that changed the selection.
    fn select(&mut self, key: u64) -> bool {
        let one = BTreeSet::from([key]);
        let changed = self.selected != one;
        self.selected = one;
        changed
    }
}

/// The lines of one row's guide, each a polyline in px from the row's top-left corner: a full
/// height line per column in `ancestors` (column `k` is the line of the rows at depth `k + 1`),
/// then the row's own. Not last: a trunk through the row and a branch off it. Last: one line that
/// turns into the stub. A row at depth 0 has no guide of its own.
pub fn guide_paths(depth: u16, last: bool, ancestors: &[u16], height: f32) -> Vec<Vec<[f32; 2]>> {
    let column = |k: u16| (f32::from(k) * INDENT + COLUMN_CENTRE).floor() + 0.5;
    let mut paths: Vec<Vec<[f32; 2]>> = ancestors
        .iter()
        .map(|k| vec![[column(*k), 0.0], [column(*k), height]])
        .collect();
    if depth == 0 {
        return paths;
    }
    let x = column(depth - 1);
    let stub = f32::from(depth) * INDENT;
    let cy = height / 2.0;
    let r = elbow_radius(x, stub, cy);
    let mut branch: Vec<[f32; 2]> = (0..=ELBOW_STEPS)
        .map(|i| elbow_point(x, cy, r, i as f32 / ELBOW_STEPS as f32))
        .collect();
    branch.push([stub, cy]);
    if last {
        branch.insert(0, [x, 0.0]);
    } else {
        paths.push(vec![[x, 0.0], [x, height]]);
    }
    paths.push(branch);
    paths
}

/// The lines of one row's guide in [`GuideStyle::Straight`]: a full height vertical line in the
/// column of every depth above the row's own (column `k` is the line of the rows at depth
/// `k + 1`, as in [`guide_paths`]). A row at depth 0 has none.
pub fn straight_paths(depth: u16, height: f32) -> Vec<Vec<[f32; 2]>> {
    (0..depth)
        .map(|k| {
            let x = (f32::from(k) * INDENT + COLUMN_CENTRE).floor() + 0.5;
            vec![[x, 0.0], [x, height]]
        })
        .collect()
}

fn elbow_radius(x: f32, stub: f32, cy: f32) -> f32 {
    ELBOW_RADIUS.min(((stub - x) / 2.0).max(0.0)).min(cy)
}

/// The elbow's quadratic curve from `(x, cy - r)` by the control point `(x, cy)` to `(x + r, cy)`.
fn elbow_point(x: f32, cy: f32, r: f32, t: f32) -> [f32; 2] {
    [x + r * t * t, cy - r * (1.0 - t) * (1.0 - t)]
}

/// `branch` from the edge of the trunk at `x`: the guide's colour is translucent, so a line drawn
/// over the trunk would show as a bright spot at the junction.
fn clear_of_trunk(branch: &[[f32; 2]], x: f32) -> Vec<[f32; 2]> {
    let edge = x + GUIDE_WIDTH / 2.0;
    let Some(i) = branch.iter().position(|p| p[0] >= edge).filter(|i| *i > 0) else {
        return branch.to_vec();
    };
    let (a, b) = (branch[i - 1], branch[i]);
    let k = (edge - a[0]) / (b[0] - a[0]);
    let mut out = vec![[edge, a[1] + (b[1] - a[1]) * k]];
    out.extend_from_slice(&branch[i..]);
    out
}

/// How a row's guide is drawn: the lines' shape and their colour (the opacity of a fade is in it).
#[derive(Debug, Clone, Copy)]
struct GuideLook {
    straight: bool,
    color: Rgba,
}

/// Draws the guide of a row in `slot`. The stroke has square ends, so the elbow's stub gets its
/// round cap as a half disc past it.
fn paint_guide<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    slot: Rect,
    depth: u16,
    guide: &RowGuide,
    look: GuideLook,
) {
    let color = look.color;
    let style = StrokeStyle::solid(color, GUIDE_WIDTH);
    let at = |q: &[f32; 2]| [slot.x + q[0], slot.y + q[1]];
    if look.straight {
        for path in straight_paths(depth, slot.h) {
            let pts: Vec<[f32; 2]> = path.iter().map(at).collect();
            p.stroke(&pts, false, style);
        }
        return;
    }
    let paths = guide_paths(depth, guide.last, &guide.ancestors, slot.h);
    let own_branch = (depth > 0 && !guide.last).then(|| paths.len() - 1);
    for (i, path) in paths.iter().enumerate() {
        let path = if own_branch == Some(i) {
            clear_of_trunk(path, path[0][0])
        } else {
            path.clone()
        };
        let pts: Vec<[f32; 2]> = path.iter().map(at).collect();
        p.stroke(&pts, false, style);
    }
    if depth > 0 {
        let half = GUIDE_WIDTH / 2.0;
        let (x, y) = (slot.x + f32::from(depth) * INDENT, slot.y + slot.h / 2.0);
        let cap = Rect::new(x, y - half, half, GUIDE_WIDTH);
        p.fill_radii(cap, [0.0, half, half, 0.0], color);
    }
}

/// `pts`, offsets from a centre, turned `turn` of a quarter turn clockwise (0 closed, 1 open). The
/// two ends are exact, with no trigonometry: a settled mark is never a rotated one.
fn turned(pts: &[[f32; 2]], turn: f32) -> Vec<[f32; 2]> {
    if turn <= 0.0 {
        return pts.to_vec();
    }
    if turn >= 1.0 {
        return pts.iter().map(|q| [-q[1], q[0]]).collect();
    }
    let (sin, cos) = (turn * std::f32::consts::FRAC_PI_2).sin_cos();
    pts.iter()
        .map(|q| [q[0] * cos - q[1] * sin, q[0] * sin + q[1] * cos])
        .collect()
}

/// The disclosure mark of a row with children, centred in the slot at `centre`: a solid triangle
/// or a thin chevron, pointing right at `turn` 0 and down at 1.
fn paint_mark<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    centre: [f32; 2],
    style: ChevronStyle,
    turn: f32,
    color: Rgba,
) {
    let at = |pts: Vec<[f32; 2]>| -> Vec<[f32; 2]> {
        pts.iter()
            .map(|q| [centre[0] + q[0], centre[1] + q[1]])
            .collect()
    };
    match style {
        ChevronStyle::Triangle => {
            let (w, h) = (TRIANGLE.0 / 2.0, TRIANGLE.1 / 2.0);
            let pts = [[-w, -h], [w, 0.0], [-w, h]];
            p.fill_path(&at(turned(&pts, turn)), color);
        }
        ChevronStyle::Chevron => {
            let (w, h) = CHEVRON_ARM;
            let pts = [[-w, -h], [w, 0.0], [-w, h]];
            let stroke = StrokeStyle::solid(color, CHEVRON_STROKE);
            p.stroke(&at(turned(&pts, turn)), false, stroke);
        }
    }
}

/// Where a row's content starts, from its left edge.
fn content_x(depth: u16) -> f32 {
    PAD_LEFT + f32::from(depth) * INDENT
}

/// What a row draws besides its fill and guide.
#[derive(Debug, Clone, Copy)]
struct RowLook<'a> {
    row: &'a TreeRow<'a>,
    /// The tree has a mark slot.
    slot: bool,
    /// How far the mark has turned, 0 closed to 1 open.
    turn: f32,
    /// The mark's style and colour.
    mark: (ChevronStyle, Rgba),
    selected: bool,
}

/// Draws the mark, the glyph and the text of a row in `slot`, the text cut to fit.
fn paint_content<H: Copy + PartialEq>(
    p: &mut Painter<'_, H>,
    theme: &Theme,
    slot: Rect,
    look: &RowLook<'_>,
) {
    let mut x = slot.x + content_x(look.row.depth);
    if look.slot {
        if look.row.has_children {
            let centre = [x + CHEVRON / 2.0, slot.y + slot.h / 2.0 + MARK_DROP];
            paint_mark(p, centre, look.mark.0, look.turn, look.mark.1);
        }
        x += CHEVRON + CHEVRON_GAP;
    }
    if let Some(g) = look.row.glyph {
        let spec = icon::glyph(g, CHEVRON, theme.fg_secondary);
        p.text_centered(&spec, Rect::new(x, slot.y, CHEVRON, slot.h));
        x += CHEVRON + GLYPH_GAP;
    }
    let color = if look.selected {
        theme.fg_bright
    } else {
        theme.fg_primary
    };
    let spec = Spec::plain(look.row.text, Style::ui(theme.font_base, color));
    let spec = p.elide(&spec, (slot.right() - x).max(0.0));
    p.text_in(&spec, Rect::new(x, slot.y, slot.right() - x, slot.h), 0.0);
}

/// The block in motion, as it is kept between frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Block {
    key: u64,
    /// Opening; closing otherwise.
    open: bool,
}

/// The block in motion, in this frame's shown rows.
#[derive(Debug, Clone, Copy)]
struct Moving {
    /// The shown index of the row that owns the block.
    at: usize,
    /// One past the block's last shown index.
    end: usize,
    /// How much of the block's height shows, 0 to 1.
    f: f64,
    closing: bool,
}

impl Moving {
    fn height(&self) -> f64 {
        (self.end - self.at - 1) as f64 * f64::from(ROW)
    }

    /// How far the rows below the block sit above their settled place.
    fn shift(&self) -> f64 {
        (1.0 - self.f) * self.height()
    }

    /// Where the block's clip starts.
    fn top(&self) -> f64 {
        (self.at + 1) as f64 * f64::from(ROW)
    }
}

/// A row in view.
#[derive(Debug, Clone, Copy)]
struct InView {
    /// The shown index.
    at: usize,
    /// Its top in content coordinates.
    top: f64,
    /// It belongs to the block in motion.
    block: bool,
}

/// The frame's rows: those shown (a closing block's are, until it settles) and the block.
#[derive(Debug, Clone)]
struct Layout {
    /// Indices into the app's rows.
    shown: Vec<usize>,
    moving: Option<Moving>,
}

impl Layout {
    /// One walk over `rows`. The row of `block` counts as open, so a closing block is still here.
    fn new(rows: &[TreeRow<'_>], expanded: &BTreeSet<u64>, block: Option<Block>) -> Self {
        let hold = block.map(|b| b.key);
        let mut shown = Vec::with_capacity(rows.len());
        let mut hide_below = None;
        for (i, r) in rows.iter().enumerate() {
            if let Some(d) = hide_below {
                if r.depth > d {
                    continue;
                }
                hide_below = None;
            }
            shown.push(i);
            if r.has_children && !expanded.contains(&r.key) && hold != Some(r.key) {
                hide_below = Some(r.depth);
            }
        }
        let moving = block.and_then(|b| {
            let at = shown.iter().position(|&i| rows[i].key == b.key)?;
            let depth = rows[shown[at]].depth;
            let end = shown[at + 1..]
                .iter()
                .position(|&i| rows[i].depth <= depth)
                .map_or(shown.len(), |n| at + 1 + n);
            (rows[shown[at]].has_children && end > at + 1).then_some(Moving {
                at,
                end,
                f: if b.open { 0.0 } else { 1.0 },
                closing: !b.open,
            })
        });
        Self { shown, moving }
    }

    /// The block's length, when its rows are out of the cursor's reach (closing).
    fn closing_len(&self) -> usize {
        match self.moving {
            Some(m) if m.closing => m.end - m.at - 1,
            _ => 0,
        }
    }

    /// The rows the cursor can stand on: the shown ones less a closing block.
    fn nav_len(&self) -> usize {
        self.shown.len() - self.closing_len()
    }

    /// The shown index of the `i`-th row the cursor can stand on.
    fn shown_at(&self, i: usize) -> usize {
        match self.moving {
            Some(m) if i > m.at => i + self.closing_len(),
            _ => i,
        }
    }

    /// The app's row of the `i`-th row the cursor can stand on.
    fn row_at<'r, 'a>(&self, rows: &'r [TreeRow<'a>], i: usize) -> &'r TreeRow<'a> {
        &rows[self.shown[self.shown_at(i)]]
    }

    /// Where `key` is among the rows the cursor can stand on.
    fn find(&self, rows: &[TreeRow<'_>], key: u64) -> Option<usize> {
        let at = self.shown.iter().position(|&i| rows[i].key == key)?;
        match self.moving {
            Some(m) if m.closing && at > m.at => (at >= m.end).then(|| at - self.closing_len()),
            _ => Some(at),
        }
    }

    /// The top of shown row `at`, in content coordinates.
    fn top(&self, at: usize) -> f64 {
        let y = at as f64 * f64::from(ROW);
        match self.moving {
            Some(m) if at >= m.end => y - m.shift(),
            _ => y,
        }
    }

    /// The height of what shows.
    fn extent(&self) -> f64 {
        self.top(self.shown.len())
    }

    /// The rows that reach into `offset .. offset + viewport`, in order; those of the block only
    /// inside its clip.
    fn in_view(&self, offset: f64, viewport: f64) -> Vec<InView> {
        let (low, high) = (offset, offset + viewport);
        let row = f64::from(ROW);
        let mut out = Vec::new();
        // The rows `from..to`, whose tops sit `shift` above `index * ROW`, inside `low..high`.
        let mut span = |from: usize, to: usize, shift: f64, low: f64, high: f64, block: bool| {
            let first = (((low + shift) / row).floor().max(0.0) as usize).max(from);
            let end = (((high + shift) / row).ceil().max(0.0) as usize).min(to);
            out.extend((first..end).map(|at| InView {
                at,
                top: at as f64 * row - shift,
                block,
            }));
        };
        match self.moving {
            None => span(0, self.shown.len(), 0.0, low, high, false),
            Some(m) => {
                span(0, m.at + 1, 0.0, low, high, false);
                let clip_end = m.top() + m.f * m.height();
                span(
                    m.at + 1,
                    m.end,
                    0.0,
                    low.max(m.top()),
                    high.min(clip_end),
                    true,
                );
                span(m.end, self.shown.len(), m.shift(), low, high, false);
            }
        }
        out
    }

    /// Whether each row of `view` is the last among its siblings and which ancestor columns run
    /// through it: one pass from the end, down to the first row in view.
    fn guides(&self, rows: &[TreeRow<'_>], view: &[InView]) -> Vec<RowGuide> {
        let mut out = vec![RowGuide::default(); view.len()];
        // The depths with a later sibling under the current parent, rising.
        let mut open: Vec<u16> = Vec::new();
        let mut left = view.len();
        for at in (0..self.shown.len()).rev() {
            if left == 0 {
                break;
            }
            let depth = rows[self.shown[at]].depth;
            while open.last().is_some_and(|d| *d > depth) {
                open.pop();
            }
            let next = open.last() == Some(&depth);
            if view[left - 1].at == at {
                left -= 1;
                out[left] = RowGuide {
                    last: !next,
                    ancestors: open
                        .iter()
                        .filter(|d| **d >= 1 && **d < depth)
                        .map(|d| d - 1)
                        .collect(),
                };
            }
            if !next {
                open.push(depth);
            }
        }
        out
    }
}

/// A row's place in the guide lines.
#[derive(Debug, Clone, Default)]
struct RowGuide {
    last: bool,
    /// The columns of the ancestors that have a later sibling.
    ancestors: Vec<u16>,
}

/// The keys of `rows[index]`'s ancestors, nearest first.
fn ancestors_of(rows: &[TreeRow<'_>], index: usize) -> Vec<u64> {
    let mut depth = rows[index].depth;
    let mut out = Vec::new();
    for r in rows[..index].iter().rev() {
        if depth == 0 {
            break;
        }
        if r.depth < depth {
            out.push(r.key);
            depth = r.depth;
        }
    }
    out
}

/// What a tree keeps besides its [`TreeState`].
#[derive(Debug, Default)]
struct TreeMem {
    /// `expanded` as the last walk saw it; `None` before the first.
    seen: Option<BTreeSet<u64>>,
    block: Option<Block>,
    /// The cursor must be brought into view, again while a block moves under it.
    reveal: bool,
    /// The cursor's ancestors and its index the last frame, for when its row goes.
    cursor_path: Vec<u64>,
    cursor_at: usize,
    /// The offset, rect and shown count the last frame drew at.
    offset: f64,
    rect: Option<Rect>,
    shown: usize,
    /// The content moved last frame: this frame's hover is still that movement's result.
    was_moved: bool,
    /// The row whose press this tree already handled, until the release, and whether it was on
    /// the chevron.
    press: Option<(u64, bool)>,
}

impl TreeMem {
    /// Records this frame's position; whether row fills may tween: the content did not move this
    /// frame or the one before (a hover is read against the last frame's regions).
    fn still(&mut self, offset: f64, rect: Rect, lay: &Layout) -> bool {
        let moved = offset != self.offset
            || self.rect.is_some_and(|r| r != rect)
            || lay.shown.len() != self.shown
            || lay.moving.is_some();
        let still = !moved && !self.was_moved;
        self.offset = offset;
        self.rect = Some(rect);
        self.shown = lay.shown.len();
        self.was_moved = moved;
        still
    }

    /// Puts the cursor back on a row it can stand on. A focused tree with no cursor yet takes the
    /// selected row, else the first row in view.
    fn fix_cursor(
        &mut self,
        st: &mut TreeState,
        rows: &[TreeRow<'_>],
        lay: &Layout,
        focused: bool,
    ) {
        let count = lay.nav_len();
        if count == 0 {
            return;
        }
        let Some(cursor) = st.cursor else {
            if focused {
                let top = (self.offset / f64::from(ROW)) as usize;
                let at = st
                    .selected
                    .iter()
                    .find_map(|k| lay.find(rows, *k))
                    .unwrap_or(top.min(count - 1));
                st.cursor = Some(lay.row_at(rows, at).key);
            }
            return;
        };
        if lay.find(rows, cursor).is_some() {
            return;
        }
        let path = match rows.iter().position(|r| r.key == cursor) {
            Some(i) => ancestors_of(rows, i),
            None => std::mem::take(&mut self.cursor_path),
        };
        let at = path
            .iter()
            .find_map(|k| lay.find(rows, *k))
            .unwrap_or(self.cursor_at.min(count - 1));
        st.cursor = Some(lay.row_at(rows, at).key);
    }

    /// Remembers where the cursor stands, for a frame in which its row is gone.
    fn note_cursor(&mut self, st: &TreeState, rows: &[TreeRow<'_>], lay: &Layout) {
        let Some(cursor) = st.cursor else { return };
        if let Some(i) = rows.iter().position(|r| r.key == cursor) {
            self.cursor_path = ancestors_of(rows, i);
        }
        if let Some(at) = lay.find(rows, cursor) {
            self.cursor_at = at;
        }
    }
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
    activated: Option<u64>,
    toggled: Option<u64>,
    context: Option<u64>,
}

/// A row between registering and drawing.
#[derive(Debug, Clone, Copy)]
struct Built {
    id: WidgetId,
    index: usize,
    slot: Rect,
    /// `index` in the view list, for its guide.
    view: usize,
    hovered: bool,
    selected: bool,
    selected_before: bool,
    open: bool,
    cursor: bool,
}

/// This call of the tree: its id, rect and options.
struct Frame<'r, 'a> {
    id: WidgetId,
    rect: Rect,
    opts: &'r TreeOptions<'a>,
    /// How much of the guides' colour shows: 1, or the `OnHover` fade.
    fade: f32,
}

impl<'r, 'a> Frame<'r, 'a> {
    fn rows(&self) -> &'r [TreeRow<'a>] {
        self.opts.rows
    }
}

impl Ui<'_> {
    /// The retained state of the tree under `key` (of the current scope). The app may change it;
    /// [`TreeState::retain`] prunes keys that left its data.
    pub fn tree_state(&mut self, key: impl Hash) -> &mut TreeState {
        let id = self.id(key);
        self.persistent::<TreeState>(id)
    }

    /// A tree under `key` over `rect`, building only the rows in view. It draws each row's fill,
    /// guide, chevron, glyph and text, then calls `cells` for what goes on its right side, under
    /// the row's own id scope and semantic node.
    pub fn tree_view(
        &mut self,
        key: impl Hash,
        rect: Rect,
        opts: &TreeOptions<'_>,
        mut cells: impl FnMut(&mut Self, &TreeRowCtx),
    ) -> TreeResponse {
        self.scope(key, |ui| ui.tree_frame(rect, opts, &mut cells))
    }

    fn tree_frame(
        &mut self,
        rect: Rect,
        opts: &TreeOptions<'_>,
        cells: &mut dyn FnMut(&mut Self, &TreeRowCtx),
    ) -> TreeResponse {
        let id = self.scope_id();
        let fade = self.guide_fade(id, rect, opts);
        let frame = Frame {
            id,
            rect,
            opts,
            fade,
        };
        let mut mem = std::mem::take(self.persistent::<TreeMem>(id));
        let mut st = std::mem::take(self.persistent::<TreeState>(id));
        let mut out = Outcome::default();

        let mut lay = self.tree_walk(&frame, &mut st, &mut mem);
        self.tree_keys(&frame, &mut st, &mut mem, &mut lay, &mut out);
        self.tree_sample(&frame, &st, &mut mem, &mut lay);
        if mem.reveal {
            if let Some(at) = st.cursor.and_then(|k| lay.find(opts.rows, k)) {
                let top = lay.top(lay.shown_at(at));
                let span = Target::span_y(top, f64::from(ROW));
                self.scroll_to(AREA, span, Reveal::Nearest);
            }
            mem.reveal = lay.moving.is_some();
        }

        let area = ScrollOptions {
            bars: (
                Bars::Disabled,
                Bars::Auto.unless_fits(lay.extent(), f64::from(rect.h)),
            ),
            extent: Some((f64::from(rect.w), lay.extent())),
            ..ScrollOptions::default()
        };
        let scroll = self.scroll_area_untranslated(AREA, rect, &area, |ui, view| {
            let mut node = Node::new(id, Role::Tree, opts.label, rect);
            node.states.insert(State::Focusable);
            node.actions.insert(Action::Focus);
            let response = ui.node_container(node, |ui| {
                ui.tree_rows(&frame, view.offset.1, st, &mut mem, &lay, cells, &mut out)
            });
            (response, Extent::default())
        });

        mem.note_cursor(self.persistent::<TreeState>(id), opts.rows, &lay);
        *self.persistent::<TreeMem>(id) = mem;

        let mut response = scroll.value;
        if out.changed {
            response.mark_changed();
        }
        TreeResponse {
            response,
            activated: out.activated,
            toggled: out.toggled,
            context: out.context,
        }
    }

    /// How much of the guides' colour shows this frame: 1, except in `OnHover`, where it follows
    /// the pointer over the tree's rect (the rows are inside it) and is read against the last
    /// frame's regions. A disabled tree has no pointer.
    fn guide_fade(&mut self, id: WidgetId, rect: Rect, opts: &TreeOptions<'_>) -> f32 {
        if opts.guide_style != GuideStyle::OnHover || !opts.guides {
            return 1.0;
        }
        let over = self.is_enabled() && self.pointer_in(rect);
        let target = if over { 1.0 } else { 0.0 };
        self.animate_f32(id.with(FADE), target, &GUIDE_FADE)
    }

    /// The walk: reads what changed in `expanded` since the last one, decides the block's fate and
    /// keeps the cursor on a row it can stand on.
    fn tree_walk(&mut self, f: &Frame<'_, '_>, st: &mut TreeState, mem: &mut TreeMem) -> Layout {
        let rows = f.rows();
        if mem.seen.as_ref() != Some(&st.expanded) {
            let change: Vec<u64> = mem.seen.as_ref().map_or(Vec::new(), |seen| {
                let diff = st.expanded.symmetric_difference(seen);
                diff.take(2).copied().collect()
            });
            mem.seen = Some(st.expanded.clone());
            let block = match change[..] {
                [key] => Some(Block {
                    key,
                    open: st.expanded.contains(&key),
                }),
                _ => None,
            };
            mem.block = match (block, mem.block) {
                (Some(new), Some(old)) if new.key == old.key => Some(new),
                (Some(new), _) => {
                    let lay = Layout::new(rows, &st.expanded, Some(new));
                    lay.moving.map(|_| {
                        let from = if new.open { 0.0 } else { 1.0 };
                        self.snap_f32(f.id.with(MOTION), from);
                        new
                    })
                }
                (None, _) => None,
            };
        }
        let mut lay = Layout::new(rows, &st.expanded, mem.block);
        if mem.block.is_some() && lay.moving.is_none() {
            // Its row left, was hidden or lost its children.
            mem.block = None;
            lay = Layout::new(rows, &st.expanded, None);
        }
        let (focused, _) = self.focus_state();
        mem.fix_cursor(st, rows, &lay, focused == Some(f.id));
        lay
    }

    /// Reads the block's height for this frame; a settled block stops existing.
    fn tree_sample(
        &mut self,
        f: &Frame<'_, '_>,
        st: &TreeState,
        mem: &mut TreeMem,
        lay: &mut Layout,
    ) {
        let Some(block) = mem.block else { return };
        let target = if block.open { 1.0 } else { 0.0 };
        let value = self.animate_f32(f.id.with(MOTION), target, &EXPAND);
        if value == target {
            mem.block = None;
            *lay = Layout::new(f.rows(), &st.expanded, None);
        } else if let Some(m) = &mut lay.moving {
            m.f = f64::from(value).clamp(0.0, 1.0);
        }
    }

    /// Takes the frame's keys: the cursor moves before the area runs, so its reveal lands with it.
    /// Disabled, the same keys are taken and do nothing.
    fn tree_keys(
        &mut self,
        f: &Frame<'_, '_>,
        st: &mut TreeState,
        mem: &mut TreeMem,
        lay: &mut Layout,
        out: &mut Outcome,
    ) {
        let (id, rows) = (f.id, f.rows());
        let enabled = self.is_enabled();
        let page = ((f.rect.h / ROW).floor() as usize).max(1);
        for (index, ev) in self.focus_events(id) {
            let FocusEvent::Key {
                key: Key::Named(k),
                modifiers,
                ..
            } = ev
            else {
                continue;
            };
            let arrow = matches!(
                k,
                NamedKey::ArrowUp
                    | NamedKey::ArrowDown
                    | NamedKey::ArrowLeft
                    | NamedKey::ArrowRight
                    | NamedKey::Home
                    | NamedKey::End
                    | NamedKey::PageUp
                    | NamedKey::PageDown
            );
            if !arrow || modifiers.control || modifiers.alt || modifiers.logo {
                continue;
            }
            self.consume_focus_event(index);
            if !enabled {
                continue;
            }
            let Some(at) = st.cursor.and_then(|c| lay.find(rows, c)) else {
                continue;
            };
            let last = lay.nav_len() - 1;
            let row = lay.row_at(rows, at);
            let open = row.has_children && st.expanded.contains(&row.key);
            let to = match k {
                NamedKey::ArrowUp => Some(at.saturating_sub(1)),
                NamedKey::ArrowDown => Some((at + 1).min(last)),
                NamedKey::Home => Some(0),
                NamedKey::End => Some(last),
                NamedKey::PageUp => Some(at.saturating_sub(page)),
                NamedKey::PageDown => Some((at + page).min(last)),
                NamedKey::ArrowRight if open => {
                    (at < last && lay.row_at(rows, at + 1).depth > row.depth).then_some(at + 1)
                }
                NamedKey::ArrowLeft if !open => (0..at)
                    .rev()
                    .find(|i| lay.row_at(rows, *i).depth < row.depth),
                _ => None,
            };
            if let Some(to) = to {
                st.cursor = Some(lay.row_at(rows, to).key);
                mem.reveal = true;
            } else if row.has_children && (k == NamedKey::ArrowRight) != open {
                // Only Left and Right reach here: Right on a closed row, Left on an open one.
                let key = row.key;
                st.toggle(key);
                out.toggled = Some(key);
                *lay = self.tree_walk(f, st, mem);
            }
        }
        while self.activation_keys(id, enabled, &[NamedKey::Enter]) {
            out.activated = st.cursor.filter(|c| lay.find(rows, *c).is_some());
        }
        while self.activation_keys(id, enabled, &[NamedKey::Space]) {
            if let Some(c) = st.cursor.filter(|c| lay.find(rows, *c).is_some()) {
                out.changed |= st.select(c);
            }
        }
    }

    /// Whether this frame's click on `id` came from an Expand or Collapse action: they and Default
    /// all arrive as a click.
    fn tree_toggle_action(&self, id: WidgetId) -> bool {
        self.input().events.iter().any(|e| {
            matches!(e, Event::Access(a)
                if a.target == id && matches!(a.action, Action::Expand | Action::Collapse))
        })
    }

    /// One frame of the rows inside the area: registers them, applies their input, then draws.
    #[allow(clippy::too_many_arguments)]
    fn tree_rows(
        &mut self,
        f: &Frame<'_, '_>,
        offset: f64,
        mut st: TreeState,
        mem: &mut TreeMem,
        lay: &Layout,
        cells: &mut dyn FnMut(&mut Self, &TreeRowCtx),
        out: &mut Outcome,
    ) -> Response {
        let (id, rect, rows) = (f.id, f.rect, f.rows());
        let response = self.interact(id, rect, Sense::CLICK_FOCUS);
        let still = mem.still(offset, rect, lay);
        let view = lay.in_view(offset, f64::from(rect.h));
        let slot_of = |v: &InView| {
            let y = (v.top - offset) as f32 + rect.y;
            Rect::new(rect.x, y, rect.w, ROW)
        };

        // Register and take input, so every row draws the frame's final selection.
        let mut pressing = false;
        let mut toggled = false;
        let mut slots = Vec::with_capacity(view.len());
        for (n, v) in view.iter().enumerate().filter(|(_, v)| !v.block) {
            let index = lay.shown[v.at];
            let row = &rows[index];
            let row_id = self.id(row.key);
            let slot = slot_of(v);
            let r = self.interact(row_id, slot, Sense::CLICK);
            if r.press_started {
                self.request_focus(id);
                let chevron = Rect::new(slot.x + content_x(row.depth), slot.y, CHEVRON, ROW);
                let chevron = self.painter().transformed(chevron);
                let on_chevron =
                    row.has_children && r.press_pos.is_some_and(|(x, y)| chevron.contains((x, y)));
                mem.press = Some((row.key, on_chevron));
                if !on_chevron {
                    st.cursor = Some(row.key);
                    out.changed |= st.select(row.key);
                }
                let by_row = f.opts.toggle_on_row && row.has_children && r.press_count <= 1;
                if on_chevron || by_row {
                    st.toggle(row.key);
                    out.toggled = Some(row.key);
                    toggled = true;
                }
            }
            let press = mem.press.filter(|(k, _)| *k == row.key);
            // A click with no press of ours is an accessibility action.
            if r.clicked && press.is_none() {
                let by_row = f.opts.toggle_on_row && row.has_children;
                let action = self.tree_toggle_action(row_id);
                if !action {
                    st.cursor = Some(row.key);
                    out.changed |= st.select(row.key);
                }
                if action || by_row {
                    st.toggle(row.key);
                    out.toggled = Some(row.key);
                    toggled = true;
                }
            }
            if r.double_clicked && !press.is_some_and(|(_, on_chevron)| on_chevron) {
                out.activated = Some(row.key);
            }
            if r.secondary_clicked {
                out.context = Some(row.key);
            }
            if r.scroll_into_view {
                let span = Target::span_y(v.top, f64::from(ROW));
                self.scroll_to(AREA, span, Reveal::Center);
            }
            pressing |= r.pressed;
            slots.push((n, index, row_id, slot, r.hovered));
        }
        if !pressing {
            mem.press = None;
        }
        if toggled {
            // The block moves from the next walk; the cursor leaves a hidden row now.
            let (focused, _) = self.focus_state();
            let logical = Layout::new(rows, &st.expanded, None);
            mem.fix_cursor(&mut st, rows, &logical, focused == Some(id));
            self.request_catch_up();
        }

        // The rows' final state.
        let (focused, ring) = self.focus_state();
        let ring = ring && focused == Some(id);
        let theme = *self.theme();
        let any_children = rows.iter().any(|r| r.has_children);
        let guides = if f.opts.guides && f.fade > 0.0 {
            lay.guides(rows, &view)
        } else {
            Vec::new()
        };
        let guide_look = GuideLook {
            straight: f.opts.guide_style == GuideStyle::Straight,
            color: {
                let mut c = theme.tree_guide;
                c[3] = (f32::from(c[3]) * f.fade.clamp(0.0, 1.0)).round() as u8;
                c
            },
        };
        // The mark's turn: the block's own fraction on its row, else the state the last walk saw
        // (a press changes `expanded` after the walk, so its mark turns from the next frame, with
        // its block).
        let turn_of = |index: usize, key: u64| -> f32 {
            match lay.moving {
                Some(m) if lay.shown[m.at] == index => m.f as f32,
                _ if mem.seen.as_ref().is_some_and(|s| s.contains(&key)) => 1.0,
                _ => 0.0,
            }
        };
        let style = f.opts.chevron;
        let mut built = Vec::with_capacity(slots.len());
        for (n, index, row_id, slot, hovered) in slots {
            let key = rows[index].key;
            let selected = st.selected.contains(&key);
            let selected_before =
                std::mem::replace(&mut self.transient::<RowMem>(row_id).selected, selected);
            built.push(Built {
                id: row_id,
                index,
                slot,
                view: n,
                hovered,
                selected,
                selected_before,
                open: rows[index].has_children && st.expanded.contains(&key),
                cursor: st.cursor == Some(key),
            });
        }

        // The block first, under its clip: no hits, no nodes, no cells.
        if let Some(m) = lay.moving {
            let top = (m.top() - offset) as f32 + rect.y;
            let clip = Rect::new(rect.x, top, rect.w, (m.f * m.height()) as f32);
            self.painter().push_clip(clip);
            for (n, v) in view.iter().enumerate().filter(|(_, v)| v.block) {
                let slot = slot_of(v);
                let row = &rows[lay.shown[v.at]];
                let l = RowLook {
                    row,
                    slot: any_children,
                    turn: turn_of(lay.shown[v.at], row.key),
                    mark: (style, theme.fg_tertiary),
                    selected: st.selected.contains(&row.key),
                };
                if l.selected {
                    self.painter()
                        .fill(slot, ROW_RADIUS, row_fill(&theme, false, true));
                }
                if let Some(g) = guides.get(n) {
                    paint_guide(self.painter(), slot, row.depth, g, guide_look);
                }
                paint_content(self.painter(), &theme, slot, &l);
            }
            self.painter().pop_clip();
        }

        // `cells` may read the state; everything after this only reads what was taken from it.
        *self.persistent::<TreeState>(id) = st;

        for b in &built {
            let row = &rows[b.index];
            let mut fill: Rgba = row_fill(&theme, b.hovered, b.selected);
            // While the content moves a row snaps: skipping the animation also drops any in flight.
            if still {
                let motion = if b.selected != b.selected_before {
                    &SELECTION
                } else {
                    &HOVER
                };
                fill = self.animate_color(b.id.with("fill"), fill, motion);
            }
            if fill[3] != 0 {
                self.painter().fill(b.slot, ROW_RADIUS, fill);
            }
            if let Some(g) = guides.get(b.view) {
                paint_guide(self.painter(), b.slot, row.depth, g, guide_look);
            }
            let mut mark = theme.fg_tertiary;
            if style == ChevronStyle::Chevron && row.has_children {
                let target = if b.hovered {
                    theme.fg_secondary
                } else {
                    theme.fg_tertiary
                };
                mark = if still {
                    self.animate_color(b.id.with("mark"), target, &MARK_HOVER)
                } else {
                    target
                };
            }
            let l = RowLook {
                row,
                slot: any_children,
                turn: turn_of(b.index, row.key),
                mark: (style, mark),
                selected: b.selected,
            };
            paint_content(self.painter(), &theme, b.slot, &l);

            let mut node = Node::new(b.id, Role::TreeItem, row.text, b.slot);
            node.level = Some(u32::from(row.depth) + 1);
            if b.selected {
                node.states.insert(State::Selected);
            }
            if b.open {
                node.states.insert(State::Expanded);
            }
            node.actions.insert(Action::Default);
            node.actions.insert(Action::ScrollIntoView);
            if row.has_children {
                node.actions.insert(if b.open {
                    Action::Collapse
                } else {
                    Action::Expand
                });
            }
            let ctx = TreeRowCtx {
                row: b.index,
                key: row.key,
                rect: b.slot,
                depth: row.depth,
                hovered: b.hovered,
                selected: b.selected,
                cursor: b.cursor,
            };
            self.node_container(node, |ui| ui.scope(row.key, |ui| cells(ui, &ctx)));
            if b.cursor && ring {
                let kind = RingKind::Edge { radius: ROW_RADIUS };
                focus_ring::paint(self.painter(), b.slot, kind, 1.0);
            }
        }
        response
    }
}

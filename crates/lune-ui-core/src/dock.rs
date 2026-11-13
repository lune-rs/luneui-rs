//! Docking maths for windows in desktop units: which rects touch, the sets they form, how a moving
//! set snaps to screen areas and to other windows, and the drag that keeps a set together. Pure
//! and integer: nothing here knows a window type, an OS or a move policy, so which windows move
//! together is the app's choice (D310).
//!
//! Every sum saturates, so a rect at the edge of the `i32` range never panics a debug build. An
//! empty rect ([`DesktopRect::is_empty`]) takes part in nothing: it touches nothing, snaps to
//! nothing and nothing snaps to it.

use crate::desktop::{DesktopPoint, DesktopRect};

/// Whether `a` and `b` share an edge segment of positive length: one's right edge is the other's
/// left edge (or bottom and top) and their extents along that edge overlap. Exact: no tolerance,
/// and corner contact does not count. Empty rects never touch.
pub fn touching(a: DesktopRect, b: DesktopRect) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let x_overlap = a.x < b.right() && b.x < a.right();
    let y_overlap = a.y < b.bottom() && b.y < a.bottom();
    ((a.right() == b.x || b.right() == a.x) && y_overlap)
        || ((a.bottom() == b.y || b.bottom() == a.y) && x_overlap)
}

/// The docked sets of `rects`: touching is transitive, so each index maps to the lowest index of
/// its set. Two rects are in one set when a chain of touching rects joins them.
pub fn docked_sets(rects: &[DesktopRect]) -> Vec<usize> {
    let mut parent: Vec<usize> = (0..rects.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            if touching(rects[i], rects[j]) {
                let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                // The lower index stays the root, so a set is named by its lowest member.
                parent[ri.max(rj)] = ri.min(rj);
            }
        }
    }
    (0..rects.len()).map(|i| root(&mut parent, i)).collect()
}

/// The indices in the docked set of `i`, ascending and including `i`. Empty when `i` is out of
/// range.
pub fn docked_with(i: usize, rects: &[DesktopRect]) -> Vec<usize> {
    if i >= rects.len() {
        return Vec::new();
    }
    let sets = docked_sets(rects);
    (0..rects.len()).filter(|&j| sets[j] == sets[i]).collect()
}

/// A rect's extent along one axis and along the other.
struct Extent {
    start: i32,
    end: i32,
    other_start: i32,
    other_end: i32,
}

fn extent(r: DesktopRect, horizontal: bool) -> Extent {
    if horizontal {
        Extent {
            start: r.x,
            end: r.right(),
            other_start: r.y,
            other_end: r.bottom(),
        }
    } else {
        Extent {
            start: r.y,
            end: r.bottom(),
            other_start: r.x,
            other_end: r.right(),
        }
    }
}

/// The best offset along one axis: the smallest move that lines an edge of a moving rect up with an
/// edge of an area or a fixed window, within reach. Candidates come in a fixed order and one wins
/// only when strictly nearer than the best so far, so a tie goes to the first offered.
fn axis_offset(
    moving: &[DesktopRect],
    fixed: &[DesktopRect],
    areas: &[DesktopRect],
    within: i32,
    horizontal: bool,
) -> i32 {
    let mut best: Option<i32> = None;
    let mut offer = |d: i32| {
        let mag = d.unsigned_abs();
        if mag <= within.unsigned_abs() && best.is_none_or(|b| mag < b.unsigned_abs()) {
            best = Some(d);
        }
    };
    for &m in moving.iter().filter(|m| !m.is_empty()) {
        let me = extent(m, horizontal);
        for &a in areas.iter().filter(|a| !a.is_empty()) {
            if m.intersects(a) {
                let ae = extent(a, horizontal);
                offer(ae.start.saturating_sub(me.start));
                offer(ae.end.saturating_sub(me.end));
            }
        }
        for &f in fixed.iter().filter(|f| !f.is_empty()) {
            let fe = extent(f, horizontal);
            // Beside on the other axis: the ranges overlap or are within reach of each other.
            if me.other_start <= fe.other_end.saturating_add(within)
                && fe.other_start <= me.other_end.saturating_add(within)
            {
                offer(fe.end.saturating_sub(me.start));
                offer(fe.start.saturating_sub(me.end));
            }
            // Stacked on the other axis: one's far edge is within reach of the other's near edge.
            if me.other_start.saturating_sub(fe.other_end).unsigned_abs() <= within.unsigned_abs()
                || fe.other_start.saturating_sub(me.other_end).unsigned_abs()
                    <= within.unsigned_abs()
            {
                offer(fe.start.saturating_sub(me.start));
                offer(fe.end.saturating_sub(me.end));
            }
        }
    }
    best.unwrap_or(0)
}

/// The offset `(dx, dy)` that snaps the `moving` rects, as one set, to the edges of `fixed`
/// windows and of `areas` (work areas), when an edge is within `within` units.
///
/// The axes are independent and both read the rects as given. On an axis the candidates are
/// offered in this order: for each moving rect in turn, the areas it overlaps (near edge, then far
/// edge), then each fixed window (the two abut offsets when the windows are beside each other
/// within reach, then the two align offsets when they are stacked within reach). The smallest
/// `|d| <= within` wins and the first offered wins a tie. `within <= 0` gives `(0, 0)`.
pub fn snap(
    moving: &[DesktopRect],
    fixed: &[DesktopRect],
    areas: &[DesktopRect],
    within: i32,
) -> (i32, i32) {
    if within <= 0 {
        return (0, 0);
    }
    (
        axis_offset(moving, fixed, areas, within, true),
        axis_offset(moving, fixed, areas, within, false),
    )
}

/// A drag of a set of windows, kept as the travel since the press: the pointer's movement is added
/// to where the windows started and the result snaps, so a snap never accumulates drift and the
/// windows follow the pointer again once it leaves the snap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockDrag {
    from: DesktopPoint,
    items: Vec<(usize, DesktopRect)>,
}

impl DockDrag {
    /// Starts a drag with the pointer at `pointer` on the windows `moving` (indices into `rects`).
    /// An index out of range is dropped, and each index counts once, in ascending order.
    pub fn start(pointer: DesktopPoint, rects: &[DesktopRect], moving: &[usize]) -> Self {
        let mut indices: Vec<usize> = moving
            .iter()
            .copied()
            .filter(|&i| i < rects.len())
            .collect();
        indices.sort_unstable();
        indices.dedup();
        Self {
            from: pointer,
            items: indices.into_iter().map(|i| (i, rects[i])).collect(),
        }
    }

    /// The new origin of each moving window with the pointer at `pointer`, ascending by index. The
    /// pointer's travel since the press is added to the start rects, which then snap as one set to
    /// the windows of `rects` that are not moving and to `areas`, within `within` units.
    pub fn update(
        &self,
        pointer: DesktopPoint,
        rects: &[DesktopRect],
        areas: &[DesktopRect],
        within: i32,
    ) -> Vec<(usize, DesktopPoint)> {
        let dx = pointer.x.saturating_sub(self.from.x);
        let dy = pointer.y.saturating_sub(self.from.y);
        let moved: Vec<DesktopRect> = self
            .items
            .iter()
            .map(|&(_, r)| r.translated(dx, dy))
            .collect();
        let fixed: Vec<DesktopRect> = rects
            .iter()
            .enumerate()
            .filter(|(j, _)| !self.items.iter().any(|&(i, _)| i == *j))
            .map(|(_, &r)| r)
            .collect();
        let (sx, sy) = snap(&moved, &fixed, areas, within);
        self.items
            .iter()
            .zip(&moved)
            .map(|(&(i, _), r)| (i, r.translated(sx, sy).origin()))
            .collect()
    }
}

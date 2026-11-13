//! Docking maths: touching, docked sets, snapping and the group drag, known answers.

use lune_ui_core::desktop::{DesktopPoint, DesktopRect};
use lune_ui_core::dock::{DockDrag, docked_sets, docked_with, snap, touching};

const A: DesktopRect = DesktopRect::new(100, 100, 275, 116);
const B: DesktopRect = DesktopRect::new(100, 216, 275, 116);
const C: DesktopRect = DesktopRect::new(600, 100, 275, 116);

fn p(x: i32, y: i32) -> DesktopPoint {
    DesktopPoint::new(x, y)
}

#[test]
fn an_exact_shared_edge_touches() {
    assert!(touching(A, B));
    assert!(touching(B, A));
    assert!(!touching(A, C));
    assert!(touching(A, A.translated(275, 0)));
    assert!(touching(A, DesktopRect::new(375, 150, 50, 20)));
}

#[test]
fn corner_contact_gaps_and_overlap_do_not_touch() {
    assert!(!touching(
        DesktopRect::new(0, 0, 10, 10),
        DesktopRect::new(10, 10, 10, 10)
    ));
    assert!(!touching(A, B.translated(0, 1)), "a one unit gap");
    assert!(!touching(A, B.translated(0, -1)), "overlap is not touching");
    assert!(!touching(A, A.translated(1, 0)));
    assert!(!touching(
        DesktopRect::new(0, 0, 0, 10),
        DesktopRect::new(0, 0, 10, 10)
    ));
    assert!(!touching(
        DesktopRect::new(0, 0, 10, 10),
        DesktopRect::new(10, 0, 10, 0)
    ));
}

#[test]
fn docked_sets_are_transitive_and_named_by_the_lowest_index() {
    assert_eq!(docked_sets(&[A, B, C]), vec![0, 0, 2]);
    // A bridge between A and C, listed last, joins all four.
    let bridge = DesktopRect::new(375, 100, 225, 116);
    assert_eq!(docked_sets(&[A, B, C, bridge]), vec![0, 0, 0, 0]);
    assert_eq!(docked_sets(&[C, B, A]), vec![0, 1, 1]);
    assert_eq!(docked_sets(&[]), Vec::<usize>::new());
}

#[test]
fn docked_with_is_ascending_and_includes_itself() {
    assert_eq!(docked_with(1, &[A, B, C]), vec![0, 1]);
    assert_eq!(docked_with(2, &[A, B, C]), vec![2]);
    assert_eq!(docked_with(3, &[A, B, C]), Vec::<usize>::new());
    assert_eq!(docked_with(0, &[]), Vec::<usize>::new());
}

#[test]
fn a_small_drag_moves_the_set_without_snapping() {
    let rects = [A, B, C];
    let drag = DockDrag::start(p(150, 110), &rects, &[0, 1]);
    let out = drag.update(p(160, 115), &rects, &[], 10);
    assert_eq!(out, vec![(0, p(110, 105)), (1, p(110, 221))]);
}

#[test]
fn a_drag_snaps_to_a_window_and_docks() {
    let rects = [A, B, C];
    let drag = DockDrag::start(p(150, 110), &rects, &[0, 1]);
    let out = drag.update(p(370, 110), &rects, &[], 10);
    assert_eq!(out, vec![(0, p(325, 100)), (1, p(325, 216))]);
    let moved = [A.moved_to(out[0].1), B.moved_to(out[1].1), C];
    assert_eq!(docked_sets(&moved), vec![0, 0, 0]);
}

#[test]
fn a_drag_that_leaves_the_snap_follows_the_pointer_again() {
    let rects = [A, B, C];
    let drag = DockDrag::start(p(150, 110), &rects, &[0, 1]);
    let snapped = drag.update(p(370, 110), &rects, &[], 10);
    assert_eq!(snapped[0].1, p(325, 100));
    let free = drag.update(p(372, 110), &rects, &[], 1);
    assert_eq!(free[0].1, p(322, 100), "no drift from the earlier snap");
}

#[test]
fn start_keeps_in_range_indices_once_in_order() {
    let rects = [A, B, C];
    let drag = DockDrag::start(p(0, 0), &rects, &[2, 9, 0, 2]);
    let out = drag.update(p(0, 0), &rects, &[], 0);
    assert_eq!(out, vec![(0, A.origin()), (2, C.origin())]);
}

#[test]
fn an_area_edge_pulls_a_rect() {
    let work = DesktopRect::new(0, 0, 3840, 2088);
    let a = DesktopRect::new(7, 100, 275, 116);
    assert_eq!(snap(&[a], &[], &[work], 10), (-7, 0));
    let far = DesktopRect::new(3840 - 275 - 4, 100, 275, 116);
    assert_eq!(snap(&[far], &[], &[work], 10), (4, 0));
}

#[test]
fn an_area_the_rect_does_not_overlap_does_not_pull() {
    let other = DesktopRect::new(5000, 0, 1920, 1080);
    let a = DesktopRect::new(4995 - 275, 100, 275, 116);
    assert_eq!(snap(&[a], &[], &[other], 10), (0, 0));
}

#[test]
fn a_tie_goes_to_the_area() {
    let work = DesktopRect::new(0, 0, 3840, 2088);
    let a = DesktopRect::new(5, 100, 275, 116);
    // The area's near edge is 5 to the left; a window 5 to the right of a's far edge pulls the
    // other way. Equal reach: the area's offer comes first and stays.
    let right = DesktopRect::new(285, 100, 100, 116);
    assert_eq!(snap(&[a], &[right], &[work], 10), (-5, 0));
}

#[test]
fn within_zero_gives_no_snap() {
    let work = DesktopRect::new(0, 0, 3840, 2088);
    let a = DesktopRect::new(7, 100, 275, 116);
    assert_eq!(snap(&[a], &[C], &[work], 0), (0, 0));
    assert_eq!(snap(&[a], &[C], &[work], -5), (0, 0));
}

#[test]
fn a_window_below_another_abuts_it_and_aligns_a_near_edge() {
    // 4 under C's bottom edge, 20 right of its left edge.
    let m = DesktopRect::new(620, 220, 100, 50);
    assert_eq!(snap(&[m], &[C], &[], 25), (-20, -4));
}

#[test]
fn a_set_snaps_by_any_member_edge() {
    // `b` hangs below `a`; only b's right edge is near c's left edge.
    let a = DesktopRect::new(100, 100, 200, 100);
    let b = DesktopRect::new(100, 200, 380, 100);
    let c = DesktopRect::new(485, 200, 100, 100);
    assert_eq!(snap(&[a, b], &[c], &[], 10), (5, 0));
}

#[test]
fn empty_rects_take_no_part() {
    let empty = DesktopRect::new(7, 7, 0, 0);
    let area = DesktopRect::new(0, 0, 100, 100);
    assert_eq!(snap(&[empty], &[C], &[area], 10), (0, 0));
    let empty_fixed = DesktopRect::new(376, 100, 0, 116);
    assert_eq!(snap(&[A], &[empty_fixed], &[], 10), (0, 0));
}

#[test]
fn rects_at_the_edge_of_the_range_do_not_panic() {
    let a = DesktopRect::new(i32::MAX - 10, 5, 100, 100);
    assert_eq!(a.right(), i32::MAX);
    let work = DesktopRect::new(0, 0, i32::MAX, i32::MAX);
    let _ = snap(&[a], &[a.translated(-50, 0)], &[work], i32::MAX);
    let _ = snap(&[a], &[a], &[work], 10);
    let rects = [a, DesktopRect::new(i32::MIN, i32::MIN, 100, 100)];
    let drag = DockDrag::start(p(i32::MIN, i32::MIN), &rects, &[0]);
    let out = drag.update(p(i32::MAX, i32::MAX), &rects, &[work], 10);
    assert_eq!(out.len(), 1);
    let _ = docked_sets(&rects);
}

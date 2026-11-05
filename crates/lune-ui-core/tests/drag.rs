//! In-window drag and drop (`infra.drag-and-drop` acceptance): the reorder machine against the
//! reference controller's own known answers (each of its test cases, by name), the catalogue's
//! numbers, the free machine, totality, and the `Ui` glue: press, threshold, cancel paths,
//! auto-scroll, motion per tier, the drop line and the dim at scale 2, and the semantic node.

use std::collections::BTreeMap;
use std::time::Duration;

use lune_ui_core::access::{Node, Role, SemanticTree, State};
use lune_ui_core::drag::{
    AUTO_SCROLL_MAX, AUTO_SCROLL_TICK, AUTO_SCROLL_ZONE, Column, DIM, DIM_ALPHA, DRAG_THRESHOLD,
    DragFrame, DropResult, DropTarget, FreeDrag, FreeFrame, FreeResult, Metrics, ReorderDrag,
    SETTLE, SLIDE, Strip, drop_index, drop_target, paint_drop_line,
};
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Rect, Xform};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Sense, Ui, WidgetId};
use lune_ui_core::widgets::dialog::{Cancel, Dialog, DialogGlass};
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};

const SPACING: f64 = 5.0;
const TICK: Duration = AUTO_SCROLL_TICK;

fn metrics(widths: &[f64]) -> Metrics<'_> {
    Metrics {
        widths,
        spacing: SPACING,
        viewport: 400.0,
        offset: 0.0,
        extent: 400.0,
    }
}

/// The strip at a scroll position.
fn scrolled(widths: &[f64], viewport: f64, offset: f64, extent: f64) -> Metrics<'_> {
    Metrics {
        widths,
        spacing: SPACING,
        viewport,
        offset,
        extent,
    }
}

/// Unshifted left edges, the layout the machine builds.
fn layout_x(widths: &[f64]) -> Vec<f64> {
    let mut x = Vec::new();
    let mut p = 0.0;
    for w in widths {
        x.push(p);
        p += w + SPACING;
    }
    x
}

/// Presses item `index` and crosses the threshold the way a real gesture does (a small move), with
/// the grab point `grab_in` px inside the item and the item at rest.
fn grab(widths: &[f64], index: usize, grab_in: f64, viewport_x: f64) -> ReorderDrag {
    let target = layout_x(widths)[index] + grab_in;
    let mut c = ReorderDrag::new();
    c.press(index, target - DRAG_THRESHOLD, viewport_x, &metrics(widths));
    c.update(target, viewport_x, &metrics(widths));
    c
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} vs {b}");
}

// The 17 cases (18 with the theory's three inputs) of the reference controller's test file.

#[test]
fn equal_widths_dragging_right_targets_the_slot_under_the_tab() {
    let widths = [100.0, 100.0, 100.0];
    let x = layout_x(&widths);
    let mut c = ReorderDrag::new();
    c.press(0, x[0] + 50.0, 200.0, &metrics(&widths));
    // One slot right: the dragged left edge lands exactly on the second slot's left edge.
    c.update(x[0] + 50.0 + 105.0, 250.0, &metrics(&widths));
    assert!(c.is_dragging());
    assert_eq!(c.dragged_index(), Some(0));
    assert_eq!(c.target_index(), Some(1));
    // Two slots right.
    c.update(x[0] + 50.0 + 210.0, 300.0, &metrics(&widths));
    assert_eq!(c.target_index(), Some(2));
}

#[test]
fn unequal_widths_nearest_slot_is_correct_in_both_directions() {
    let widths = [108.0, 270.0, 140.0];
    let x = layout_x(&widths); // 0, 113, 388
    let mut right = grab(&widths, 0, 10.0, 200.0);
    right.update(300.0, 200.0, &metrics(&widths)); // L = 290: nearest of {0, 275, 420}
    assert_eq!(right.target_index(), Some(1));
    right.update(420.0, 200.0, &metrics(&widths)); // L = 410: nearest 420
    assert_eq!(right.target_index(), Some(2));

    let mut left = grab(&widths, 2, 10.0, 200.0);
    near(left.dragged_left(), x[2]); // the drag starts at rest
    left.update(100.0, 200.0, &metrics(&widths)); // L = 90: nearest of {0, 113, 388}
    assert_eq!(left.target_index(), Some(1));
    left.update(20.0, 200.0, &metrics(&widths)); // L = 10: nearest 0
    assert_eq!(left.target_index(), Some(0));
}

#[test]
fn equal_widths_only_the_displaced_neighbours_move_by_exactly_one_slot() {
    let widths = [100.0, 100.0, 100.0];
    let x = layout_x(&widths);
    let slot = widths[0] + SPACING; // 105
    let mut c = ReorderDrag::new();
    c.press(0, x[0] + 50.0, 200.0, &metrics(&widths));
    c.update(x[0] + 50.0 + 105.0, 200.0, &metrics(&widths)); // target 1
    assert_eq!(c.target_index(), Some(1));
    near(c.offsets()[0], c.dragged_left() - x[0]); // the dragged item tracks the pointer
    near(c.offsets()[1], -slot); // the item it passed slides left
    near(c.offsets()[2], 0.0); // untouched

    c.update(x[0] + 50.0 + 210.0, 200.0, &metrics(&widths)); // target 2
    assert_eq!(c.target_index(), Some(2));
    near(c.offsets()[1], -slot);
    near(c.offsets()[2], -slot);
    near(c.offsets()[0], c.dragged_left() - x[0]);
}

#[test]
fn unequal_widths_each_displaced_tab_lands_on_its_reordered_layout_position() {
    let widths = [108.0, 270.0, 140.0];
    let x = layout_x(&widths); // 0, 113, 388
    let mut c = grab(&widths, 2, 10.0, 200.0);
    c.update(5.0, 200.0, &metrics(&widths)); // drag the last item to the front
    assert_eq!(c.target_index(), Some(0));
    // Reordered list is [2, 0, 1]: positions 0, 145, 258. The resulting left edges are asserted,
    // not a "shift by one width" shortcut, which is wrong here.
    near(x[2] + c.offsets()[2], 0.0);
    near(x[0] + c.offsets()[0], 140.0 + SPACING);
    near(x[1] + c.offsets()[1], 140.0 + SPACING + 108.0 + SPACING);
}

#[test]
fn dragging_past_either_end_clamps_the_left_edge_and_saturates_the_target() {
    let widths = [100.0, 100.0, 100.0];
    let x = layout_x(&widths);
    let total = 300.0 + 2.0 * SPACING; // 310

    let mut far_left = ReorderDrag::new();
    far_left.press(1, x[1] + 50.0, 200.0, &metrics(&widths));
    far_left.update(-1000.0, 200.0, &metrics(&widths));
    near(far_left.dragged_left(), 0.0);
    assert_eq!(far_left.target_index(), Some(0));

    let mut far_right = ReorderDrag::new();
    far_right.press(1, x[1] + 50.0, 200.0, &metrics(&widths));
    far_right.update(10_000.0, 200.0, &metrics(&widths));
    near(far_right.dragged_left(), total - widths[1]);
    assert_eq!(far_right.target_index(), Some(2));
}

/// The frame-mismatch trap: the pressed item's width changes between the press and the first move
/// (press-to-activate), which must not show as a jump at the threshold crossing.
#[test]
fn press_to_activate_width_change_does_not_jump_the_tab_at_the_threshold_crossing() {
    let before = [128.0, 100.0, 100.0]; // x = 0, 133, 238
    let after = [100.0, 128.0, 100.0]; // x = 0, 105, 238
    let after_x = layout_x(&after);
    let press_x = layout_x(&before)[1] + 50.0; // 183, grabbed 50 px into the item
    let mut c = ReorderDrag::new();
    c.press(1, press_x, 200.0, &metrics(&before));
    c.update(press_x + 5.0, 200.0, &metrics(&after)); // 5 px > the 4 px threshold
    assert!(c.is_dragging());
    // The item starts following the pointer from where it currently sits: no snap.
    near(c.offsets()[1], 0.0);
    near(c.dragged_left(), after_x[1]);
    near(c.dragged_left(), 105.0);
    // 138 is what a press-frame baseline produces: the 28 px frame delta plus 5 px of travel.
    assert!((c.dragged_left() - 138.0).abs() > 1e-6);
}

/// Once the drag is live the grab offset is frozen: a later layout shift moves the slot under the
/// pointer-anchored item without moving it on screen.
#[test]
fn once_dragging_the_tab_is_pointer_anchored_and_absorbs_a_later_layout_shift() {
    let widths = [100.0, 100.0, 100.0]; // x = 0, 105, 210
    let mut c = grab(&widths, 1, 50.0, 200.0);
    c.update(layout_x(&widths)[1] + 50.0 + 30.0, 200.0, &metrics(&widths));
    let rendered = c.dragged_left();
    near(rendered, 135.0); // 105 + 30 of travel
    let shifted = [128.0, 100.0, 100.0]; // x = 0, 133, 238
    c.update(
        layout_x(&widths)[1] + 50.0 + 30.0,
        200.0,
        &metrics(&shifted),
    );
    near(c.dragged_left(), rendered); // on screen it has not budged
    near(c.offsets()[1], rendered - layout_x(&shifted)[1]); // the offset absorbed the shift: 2, not 30
}

#[test]
fn below_the_threshold_stays_a_click_above_it_becomes_a_drag() {
    let widths = [100.0, 100.0, 100.0];
    let x = layout_x(&widths);

    let mut under = ReorderDrag::new();
    under.press(1, x[1] + 50.0, 200.0, &metrics(&widths));
    assert!(under.is_armed());
    assert!(!under.update(x[1] + 50.0 + 3.9, 200.0, &metrics(&widths)));
    assert!(!under.is_dragging());
    assert_eq!(under.release(), DropResult::Click);

    let mut over = ReorderDrag::new();
    over.press(1, x[1] + 50.0, 200.0, &metrics(&widths));
    assert!(over.update(x[1] + 50.0 + 4.1, 200.0, &metrics(&widths)));
    assert!(over.is_dragging());
    // On into the next slot so the release has something to commit.
    over.update(x[1] + 50.0 + 105.0, 200.0, &metrics(&widths));
    assert_eq!(over.release(), DropResult::Commit { from: 1, to: 2 });
    assert!(!over.is_dragging());
}

#[test]
fn releasing_on_the_original_slot_reports_click_not_commit() {
    let widths = [100.0, 100.0, 100.0];
    let x = layout_x(&widths);
    let mut c = ReorderDrag::new();
    c.press(1, x[1] + 50.0, 200.0, &metrics(&widths));
    c.update(x[1] + 50.0 + 6.0, 200.0, &metrics(&widths)); // dragging, but still nearest to slot 1
    assert!(c.is_dragging());
    assert_eq!(c.target_index(), Some(1));
    assert_eq!(c.release(), DropResult::Click);
}

#[test]
fn cancel_zeroes_everything_and_is_idempotent() {
    let widths = [100.0, 100.0, 100.0];
    let x = layout_x(&widths);
    let mut c = ReorderDrag::new();
    c.press(0, x[0] + 50.0, 200.0, &metrics(&widths));
    c.update(x[0] + 50.0 + 210.0, 200.0, &metrics(&widths));
    assert!(c.is_dragging());
    assert!(c.offsets().iter().any(|&o| o != 0.0));
    for _ in 0..2 {
        c.cancel();
        assert!(!c.is_dragging());
        assert!(!c.is_armed());
        assert_eq!(c.dragged_index(), None);
        assert_eq!(c.target_index(), None);
        assert_eq!(c.dragged_left(), 0.0);
        assert_eq!(c.auto_scroll_delta(), 0.0);
        assert!(c.offsets().iter().all(|&o| o == 0.0));
    }
    assert_eq!(c.release(), DropResult::None);
}

#[test]
fn auto_scroll_is_zero_in_the_middle_and_only_runs_where_there_is_room() {
    let widths = [100.0, 100.0, 100.0];
    let x = layout_x(&widths);
    let (viewport, extent) = (400.0, 1000.0);
    let drag = |viewport_x: f64, offset: f64| {
        let mut c = ReorderDrag::new();
        let m = scrolled(&widths, viewport, offset, extent);
        c.press(0, x[0] + 10.0, 200.0, &m);
        c.update(x[0] + 60.0, viewport_x, &m);
        c
    };
    // Middle of the viewport: idle.
    assert_eq!(drag(200.0, 100.0).auto_scroll_delta(), 0.0);
    // Left edge with room to scroll left: negative, ramped linearly.
    near(
        drag(10.0, 100.0).auto_scroll_delta(),
        -AUTO_SCROLL_MAX * (14.0 / 24.0),
    );
    // Same position, already scrolled fully left: no room, no delta.
    assert_eq!(drag(10.0, 0.0).auto_scroll_delta(), 0.0);
    // Right edge with remaining extent: positive.
    near(
        drag(395.0, 100.0).auto_scroll_delta(),
        AUTO_SCROLL_MAX * (19.0 / 24.0),
    );
    // Already scrolled fully right: no room, no delta.
    assert_eq!(drag(395.0, extent - viewport).auto_scroll_delta(), 0.0);
    // Beyond the edge entirely: the ramp saturates at the cap.
    near(drag(-500.0, 100.0).auto_scroll_delta(), -AUTO_SCROLL_MAX);
    near(drag(5000.0, 100.0).auto_scroll_delta(), AUTO_SCROLL_MAX);
}

#[test]
fn single_tab_cannot_move_at_all() {
    let widths = [100.0];
    let mut c = ReorderDrag::new();
    c.press(0, 50.0, 200.0, &metrics(&widths));
    c.update(5000.0, 200.0, &metrics(&widths));
    assert!(c.is_dragging());
    assert_eq!(c.target_index(), Some(0));
    near(c.dragged_left(), 0.0); // total - w[0] == 0, so the clamp pins it
    assert!(c.offsets().iter().all(|&o| o == 0.0));
    assert_eq!(c.release(), DropResult::Click);
}

#[test]
fn non_finite_coordinates_and_metrics_never_throw_and_never_move() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let widths = [100.0, 100.0, 100.0];
        let bad_widths = [bad, bad, bad];
        let bad_metrics = Metrics {
            widths: &bad_widths,
            spacing: bad,
            viewport: bad,
            offset: bad,
            extent: bad,
        };

        // Non-finite press coordinates: never arms.
        let mut pressed = ReorderDrag::new();
        pressed.press(0, bad, bad, &metrics(&widths));
        assert!(!pressed.is_armed());
        assert!(!pressed.update(bad, bad, &metrics(&widths)));
        assert!(pressed.offsets().iter().all(|&o| o == 0.0));
        assert_eq!(pressed.auto_scroll_delta(), 0.0);

        // Non-finite metrics on a live drag: no panic, no auto-scroll.
        let mut live = ReorderDrag::new();
        live.press(0, 50.0, 200.0, &metrics(&widths));
        live.update(200.0, 200.0, &bad_metrics);
        assert_eq!(live.auto_scroll_delta(), 0.0);
        live.tick(TICK, &bad_metrics);
        assert_eq!(live.auto_scroll_delta(), 0.0);
        live.release();

        // Non-finite coordinates fed into a live drag keep the last good position.
        let mut poisoned = ReorderDrag::new();
        poisoned.press(0, 50.0, 200.0, &metrics(&widths));
        poisoned.update(260.0, 200.0, &metrics(&widths));
        let target = poisoned.target_index();
        poisoned.update(bad, bad, &metrics(&widths));
        assert_eq!(poisoned.target_index(), target);
        assert!(poisoned.offsets().iter().all(|o| !o.is_nan()));
    }
}

#[test]
fn degenerate_structural_inputs_never_throw() {
    let widths = [100.0, 100.0];
    // Empty width list.
    let m = metrics(&[]);
    let mut c = ReorderDrag::new();
    c.press(0, 10.0, 10.0, &m);
    assert!(!c.is_armed());
    assert!(!c.update(500.0, 500.0, &m));
    assert!(!c.tick(TICK, &m));
    assert_eq!(c.release(), DropResult::None);

    // Out-of-range press indices.
    for index in [2, 99, usize::MAX] {
        let mut c = ReorderDrag::new();
        c.press(index, 10.0, 10.0, &metrics(&widths));
        assert!(!c.is_armed());
        assert!(!c.update(500.0, 500.0, &metrics(&widths)));
    }

    // Negative spacing and zero or negative widths degrade to zero.
    let odd_widths = [0.0, -50.0, 100.0];
    let odd_metrics = Metrics {
        widths: &odd_widths,
        spacing: -12.0,
        viewport: 400.0,
        offset: 0.0,
        extent: 400.0,
    };
    let mut odd = ReorderDrag::new();
    odd.press(0, 0.0, 0.0, &odd_metrics);
    odd.update(500.0, 500.0, &odd_metrics);
    assert!(odd.offsets().iter().all(|o| !o.is_nan()));
    assert!(odd.target_index().is_some_and(|t| t <= 2));

    // The item count shrinking under a live drag.
    let mut shrunk = ReorderDrag::new();
    shrunk.press(1, 150.0, 150.0, &metrics(&widths));
    shrunk.update(400.0, 400.0, &metrics(&widths));
    shrunk.update(400.0, 400.0, &metrics(&[100.0]));
    assert!(shrunk.offsets().iter().all(|&o| o == 0.0));
    assert_eq!(shrunk.release(), DropResult::Click);

    // Huge finite widths stay finite.
    let huge = [f64::MAX, f64::MAX];
    let mut c = ReorderDrag::new();
    c.press(0, 1.0, 1.0, &metrics(&huge));
    c.update(1e300, 1.0, &metrics(&huge));
    assert!(c.offsets().iter().all(|o| o.is_finite()));
}

#[test]
fn constants_match_the_documented_tokens() {
    assert_eq!(DRAG_THRESHOLD, 4.0);
    assert_eq!(AUTO_SCROLL_ZONE, 24.0);
    assert_eq!(AUTO_SCROLL_MAX, 12.0);
    assert_eq!(AUTO_SCROLL_TICK, Duration::from_millis(16));
    // The neighbour slide and the settle run at `fast`, 180 ms in the Full tier.
    assert_eq!(DARK.motion.fast, Duration::from_millis(180));
    assert_eq!(
        DARK.motion
            .resolve(&SLIDE, MotionTier::Full)
            .map(|r| r.duration),
        Some(Duration::from_millis(180))
    );
    assert_eq!(
        DARK.motion
            .resolve(&SETTLE, MotionTier::Full)
            .map(|r| r.duration),
        Some(Duration::from_millis(180))
    );
}

#[test]
fn before_any_press_nothing_is_dragging_and_every_offset_is_zero() {
    let mut c = ReorderDrag::new();
    assert!(!c.is_armed());
    assert!(!c.is_dragging());
    assert_eq!(c.dragged_index(), None);
    assert_eq!(c.target_index(), None);
    assert_eq!(c.auto_scroll_delta(), 0.0);
    assert!(c.offsets().is_empty());
    assert_eq!(c.release(), DropResult::None);
}

#[test]
fn tick_re_derives_the_content_position_from_the_new_scroll_offset() {
    let widths = [100.0, 100.0, 100.0];
    let x = layout_x(&widths);
    let mut c = ReorderDrag::new();
    c.press(0, x[0] + 46.0, 46.0, &scrolled(&widths, 200.0, 0.0, 310.0));
    c.update(x[0] + 50.0, 50.0, &scrolled(&widths, 200.0, 0.0, 310.0));
    assert!(c.is_dragging());
    let before = c.dragged_left();
    near(before, 0.0); // the grab is baselined at the crossing: at rest
    // The strip scrolled 100 px right while the pointer stayed at viewport x 50.
    c.tick(TICK, &scrolled(&widths, 200.0, 100.0, 310.0));
    near(c.dragged_left() - before, 100.0);
    near(c.dragged_left(), 100.0); // (50 + 100) - 50
    assert_eq!(c.target_index(), Some(1));
}

// The catalogue's acceptance numbers.

#[test]
fn threshold_3_9_stays_armed_and_4_0_starts_a_drag_and_vertical_travel_never_does() {
    let widths = [100.0, 100.0, 100.0];
    let mut c = ReorderDrag::new();
    c.press(1, 50.0, 50.0, &metrics(&widths));
    assert!(!c.update(53.9, 53.9, &metrics(&widths)));
    assert!(c.is_armed() && !c.is_dragging());
    assert!(c.update(54.0, 54.0, &metrics(&widths)));
    assert!(c.is_dragging() && !c.is_armed());

    // The reorder axis is horizontal: the x passed does not move, however far the pointer goes.
    let mut v = ReorderDrag::new();
    v.press(1, 50.0, 50.0, &metrics(&widths));
    assert!(!v.update(50.0, 50.0, &metrics(&widths)));
    assert_eq!(v.release(), DropResult::Click);
    assert_eq!(ReorderDrag::new().release(), DropResult::None);
}

#[test]
fn nearest_slot_numbers_for_widths_100_200_100() {
    let widths = [100.0, 200.0, 100.0];
    let mut c = grab(&widths, 0, 50.0, 200.0);
    c.update(300.0 + 50.0, 200.0, &metrics(&widths));
    near(c.dragged_left(), 300.0);
    assert_eq!(c.target_index(), Some(2));
    near(c.offsets()[0], 300.0);
    near(c.offsets()[1], -105.0);
    near(c.offsets()[2], -105.0);

    c.update(250.0 + 50.0, 200.0, &metrics(&widths));
    assert_eq!(c.target_index(), Some(1));

    // The clamp is [0, 310].
    c.update(-1000.0, 200.0, &metrics(&widths));
    near(c.dragged_left(), 0.0);
    c.update(10_000.0, 200.0, &metrics(&widths));
    near(c.dragged_left(), 310.0);
}

#[test]
fn auto_scroll_numbers_at_viewport_500() {
    let widths = [100.0, 100.0, 100.0];
    let delta = |viewport_x: f64, offset: f64, extent: f64| {
        let mut c = ReorderDrag::new();
        let m = scrolled(&widths, 500.0, offset, extent);
        c.press(0, 10.0, 200.0, &m);
        c.update(60.0, viewport_x, &m);
        c.auto_scroll_delta()
    };
    near(delta(0.0, 100.0, 2000.0), -12.0);
    near(delta(12.0, 100.0, 2000.0), -6.0);
    near(delta(24.0, 100.0, 2000.0), 0.0);
    near(delta(500.0, 100.0, 2000.0), 12.0);
    assert_eq!(delta(0.0, 0.0, 2000.0), 0.0);
    // At offset >= extent - viewport - 0.5 the right delta is 0; just below it is not.
    assert_eq!(delta(500.0, 1499.5, 2000.0), 0.0);
    assert!(delta(500.0, 1499.0, 2000.0) > 0.0);
}

#[test]
fn tick_scales_the_per_tick_delta_by_dt_over_16_ms() {
    let widths = [100.0, 100.0, 100.0];
    let m = scrolled(&widths, 500.0, 100.0, 2000.0);
    let mut c = ReorderDrag::new();
    c.press(0, 10.0, 200.0, &m);
    c.update(60.0, 500.0, &m);
    near(c.auto_scroll_delta(), 12.0);
    c.tick(TICK, &m);
    near(c.auto_scroll_step(), 12.0);
    c.tick(Duration::from_millis(32), &m);
    near(c.auto_scroll_step(), 24.0);
    c.tick(Duration::from_millis(8), &m);
    near(c.auto_scroll_step(), 6.0);
    // About 750 px/s.
    c.tick(Duration::from_secs(1), &m);
    near(c.auto_scroll_step(), 750.0);
    // The per-tick delta itself is unscaled.
    near(c.auto_scroll_delta(), 12.0);
    // Nothing in flight: no step.
    c.cancel();
    assert!(!c.tick(TICK, &m));
    assert_eq!(c.auto_scroll_step(), 0.0);
}

#[test]
fn tick_re_derives_the_position_so_a_scrolling_strip_moves_the_target() {
    let widths = [100.0, 100.0, 100.0];
    let mut c = ReorderDrag::new();
    let m = scrolled(&widths, 200.0, 0.0, 310.0);
    c.press(0, 46.0, 46.0, &m);
    c.update(50.0, 50.0, &m);
    for offset in [0.0, 50.0, 105.0, 200.0] {
        c.tick(TICK, &scrolled(&widths, 200.0, offset, 310.0));
        near(c.dragged_left(), offset.min(310.0 - 100.0));
    }
}

// The free machine.

fn board() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    (vec![20.0, 60.0, 100.0], vec![], vec![30.0])
}

fn columns<'a>(mids: &'a (Vec<f64>, Vec<f64>, Vec<f64>)) -> [Column<'a>; 3] {
    [
        Column {
            x: 0.0,
            w: 100.0,
            card_mids: &mids.0,
        },
        Column {
            x: 110.0,
            w: 100.0,
            card_mids: &mids.1,
        },
        Column {
            x: 220.0,
            w: 100.0,
            card_mids: &mids.2,
        },
    ]
}

#[test]
fn free_drag_threshold_is_euclidean() {
    let b = board();
    let cols = columns(&b);
    let mut d = FreeDrag::new();
    d.press(7, 50.0, 50.0);
    assert!(d.is_armed());
    // 2.8 px on each axis is 3.96: still a click. (3, 3) is 4.24: a drag.
    assert!(!d.update(52.8, 52.8, &cols));
    assert!(!d.is_dragging());
    assert!(d.update(53.0, 53.0, &cols));
    assert!(d.is_dragging());
    assert_eq!(d.source(), Some(7));

    // Exactly 4 on one axis starts it (the test is `< 16`); 3.99 does not.
    let mut e = FreeDrag::new();
    e.press(1, 0.0, 0.0);
    assert!(!e.update(3.99, 0.0, &cols));
    assert!(e.update(4.0, 0.0, &cols));
    let mut f = FreeDrag::new();
    f.press(1, 0.0, 0.0);
    assert!(f.update(0.0, 4.0, &cols));
    assert_eq!(FreeDrag::new().release(), FreeResult::None);
}

#[test]
fn drop_index_counts_the_midpoints_above_the_pointer() {
    let mids = [20.0, 60.0, 100.0];
    assert_eq!(drop_index(&mids, 0.0), 0);
    assert_eq!(drop_index(&mids, 20.0), 0); // strictly above
    assert_eq!(drop_index(&mids, 20.5), 1);
    assert_eq!(drop_index(&mids, 80.0), 2);
    assert_eq!(drop_index(&mids, 1000.0), 3);
    assert_eq!(drop_index(&[], 50.0), 0);
    assert_eq!(drop_index(&[f64::NAN, 10.0], 50.0), 1);
    assert_eq!(drop_index(&mids, f64::NAN), 0);
    // Any order.
    assert_eq!(drop_index(&[100.0, 20.0, 60.0], 80.0), 2);
}

#[test]
fn free_drag_targets_the_column_under_the_pointer_and_none_outside_every_column() {
    let b = board();
    let cols = columns(&b);
    assert_eq!(
        drop_target(&cols, 50.0, 80.0),
        Some(DropTarget {
            column: 0,
            index: 2
        })
    );
    assert_eq!(
        drop_target(&cols, 150.0, 80.0),
        Some(DropTarget {
            column: 1,
            index: 0
        })
    );
    assert_eq!(
        drop_target(&cols, 300.0, 80.0),
        Some(DropTarget {
            column: 2,
            index: 1
        })
    );
    // The range is [x, x + w): the right edge belongs to the gap; the gap holds nothing.
    assert_eq!(drop_target(&cols, 100.0, 80.0), None);
    assert_eq!(drop_target(&cols, 105.0, 80.0), None);
    assert_eq!(drop_target(&cols, 110.0, 80.0).map(|t| t.column), Some(1));
    assert_eq!(drop_target(&cols, -1.0, 80.0), None);
    assert_eq!(drop_target(&cols, f64::NAN, 80.0), None);
    assert_eq!(drop_target(&[], 5.0, 5.0), None);

    let mut d = FreeDrag::new();
    d.press(9, 10.0, 10.0);
    d.update(300.0, 80.0, &cols);
    assert_eq!(
        d.target(),
        Some(DropTarget {
            column: 2,
            index: 1
        })
    );
    // Between the columns: no target, and a drop is a no-op.
    d.update(105.0, 80.0, &cols);
    assert_eq!(d.target(), None);
    assert_eq!(d.release(), FreeResult::NoTarget);
    assert!(!d.is_dragging());
}

#[test]
fn free_drag_release_cancel_and_totality() {
    let b = board();
    let cols = columns(&b);
    let mut d = FreeDrag::new();
    d.press(9, 10.0, 10.0);
    assert_eq!(d.release(), FreeResult::Click);

    d.press(9, 10.0, 10.0);
    d.update(150.0, 80.0, &cols);
    assert_eq!(
        d.release(),
        FreeResult::Drop {
            source: 9,
            target: DropTarget {
                column: 1,
                index: 0
            }
        }
    );
    assert_eq!(d.release(), FreeResult::None);

    d.press(9, 10.0, 10.0);
    d.update(150.0, 80.0, &cols);
    d.cancel();
    d.cancel();
    assert!(!d.is_active());
    assert_eq!(d.target(), None);
    assert_eq!(d.release(), FreeResult::None);

    for bad in [f64::NAN, f64::INFINITY] {
        let mut d = FreeDrag::new();
        d.press(1, bad, 0.0);
        assert!(!d.is_armed());
        d.press(1, 0.0, 0.0);
        assert!(d.update(300.0, 80.0, &cols));
        assert!(d.update(bad, bad, &cols)); // the last good position stays
        assert_eq!(
            d.target(),
            Some(DropTarget {
                column: 2,
                index: 1
            })
        );
        assert!(!FreeDrag::new().update(1.0, 1.0, &[]));
    }
}

// The glue, over a three-item strip: widths 100, 200, 100, spacing 5, viewport 300 at window x 50.

const ORIGIN: f64 = 50.0;
const ROW_Y: f32 = 100.0;
const ITEM_H: f32 = 30.0;
const WIN: (f32, f32) = (600.0, 400.0);
const P: PointerButton = PointerButton::Primary;

fn item(key: u32) -> WidgetId {
    WidgetId::ROOT.with("item").with(key)
}

struct Model {
    drag: ReorderDrag,
    /// The items' keys in strip order.
    keys: Vec<u32>,
    widths: BTreeMap<u32, f64>,
    offset: f64,
    viewport: f64,
    extent: f64,
    /// Items drawn disabled this frame.
    disabled: bool,
    /// The items are the body of a dialog.
    dialog: bool,
    /// The dialog reported a cancel in the last frame.
    dialog_cancelled: bool,
}

impl Model {
    fn new() -> Self {
        Model {
            drag: ReorderDrag::new(),
            keys: vec![0, 1, 2],
            widths: BTreeMap::from([(0, 100.0), (1, 200.0), (2, 100.0)]),
            offset: 0.0,
            viewport: 300.0,
            extent: 410.0,
            disabled: false,
            dialog: false,
            dialog_cancelled: false,
        }
    }

    fn widths(&self) -> Vec<f64> {
        self.keys.iter().map(|k| self.widths[k]).collect()
    }

    fn apply(&mut self, from: usize, to: usize) {
        let k = self.keys.remove(from);
        self.keys.insert(to, k);
    }
}

struct Run {
    gestures: Vec<DragFrame>,
    offsets: Vec<f32>,
    repaint: Repaint,
    tree: SemanticTree,
    escape_left: Vec<bool>,
}

/// One frame of the strip: the items' hit regions and nodes, then every gesture, then every offset.
fn run(h: &mut UiHarness, m: &mut Model, ms: u64) -> Run {
    h.at_ms(ms);
    let widths = m.widths();
    let x = layout_x(&widths);
    let mut gestures = Vec::new();
    let mut offsets = Vec::new();
    let mut escape_left = Vec::new();
    let mut cancelled = false;
    let in_dialog = m.dialog;
    let (_, fo) = h.frame(|ui| {
        let mut body = |ui: &mut Ui<'_>| {
            let mut resps = Vec::new();
            for (i, key) in m.keys.iter().enumerate() {
                let rect = Rect::new(
                    (ORIGIN + x[i] - m.offset) as f32,
                    ROW_Y,
                    widths[i] as f32,
                    ITEM_H,
                );
                let id = item(*key);
                let resp = ui.with_enabled(!m.disabled, |ui| {
                    let r = ui.interact(id, rect, Sense::DRAG);
                    ui.node(Node::new(id, Role::Tab, "tab", rect));
                    r
                });
                resps.push(resp);
            }
            let strip_metrics = Metrics {
                widths: &widths,
                spacing: SPACING,
                viewport: m.viewport,
                offset: m.offset,
                extent: m.extent,
            };
            for (i, key) in m.keys.iter().enumerate() {
                let strip = Strip {
                    index: i,
                    origin: ORIGIN,
                    metrics: strip_metrics,
                };
                gestures.push(ui.drag_gesture(item(*key), &resps[i], &mut m.drag, &strip));
            }
            for (i, key) in m.keys.iter().enumerate() {
                offsets.push(ui.drag_offset(item(*key), &m.drag, i, x[i] as f32));
            }
            // Whether an Escape press is still unconsumed (taken by `consume_key_while_pressed`).
            for key in &m.keys {
                escape_left.push(ui.consume_key_while_pressed(item(*key), NamedKey::Escape));
            }
        };
        if in_dialog {
            cancelled = ui
                .dialog("dlg", &DIALOG, &dialog_glass(), |ui, _, _, _| body(ui))
                .cancelled;
        } else {
            body(ui);
        }
    });
    m.dialog_cancelled = cancelled;
    Run {
        gestures,
        offsets,
        repaint: fo.repaint,
        tree: fo.tree,
        escape_left,
    }
}

const DIALOG: Dialog<'static> = Dialog {
    title: "Tabs",
    message: "m",
    extra_h: 100.0,
    confirm: "OK",
    cancel: Cancel::Default,
};

fn dialog_glass() -> DialogGlass {
    DialogGlass {
        panel: [0.0; 4],
        fill: [0.0; 4],
        noise: false,
    }
}

/// A harness with the strip drawn once, so the next frame's presses resolve against its hits.
fn start(m: &mut Model) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    run(&mut h, m, 0);
    h
}

/// The window point `grab_in` px into the item `index` of the unshifted strip.
fn point(m: &Model, index: usize, grab_in: f64) -> (f32, f32) {
    let x = layout_x(&m.widths());
    (
        (ORIGIN + x[index] - m.offset + grab_in) as f32,
        ROW_Y + 15.0,
    )
}

fn press_at(h: &mut UiHarness, p: (f32, f32)) {
    h.input.pointer_to(p.0, p.1);
    h.input.press(P);
}

fn result_of(r: &Run) -> DropResult {
    r.gestures
        .iter()
        .map(|g| g.result)
        .find(|r| *r != DropResult::None)
        .unwrap_or(DropResult::None)
}

fn dragging(r: &Run) -> bool {
    r.gestures.iter().any(|g| g.dragging)
}

fn cancelled(r: &Run) -> bool {
    r.gestures.iter().any(|g| g.cancelled)
}

#[test]
fn a_press_arms_travel_3_9_stays_armed_and_4_0_starts_the_drag() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    let p = point(&m, 1, 50.0);
    press_at(&mut h, p);
    let r = run(&mut h, &mut m, 10);
    assert!(m.drag.is_armed());
    assert!(!dragging(&r));
    assert!(r.offsets.iter().all(|&o| o == 0.0));

    h.input.pointer_to(p.0 + 3.9, p.1);
    let r = run(&mut h, &mut m, 20);
    assert!(m.drag.is_armed(), "3.9 stays armed");
    assert!(!dragging(&r));

    h.input.pointer_to(p.0 + 4.0, p.1);
    let r = run(&mut h, &mut m, 30);
    assert!(m.drag.is_dragging(), "4.0 starts a drag");
    assert!(dragging(&r));
    assert_eq!(m.drag.dragged_index(), Some(1));
}

#[test]
fn vertical_travel_never_starts_a_drag() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    let p = point(&m, 1, 50.0);
    press_at(&mut h, p);
    run(&mut h, &mut m, 10);
    h.input.pointer_to(p.0, p.1 + 10.0);
    let r = run(&mut h, &mut m, 20);
    assert!(m.drag.is_armed());
    assert!(!dragging(&r));
    h.input.release(P);
    let r = run(&mut h, &mut m, 30);
    assert!(!m.drag.is_active());
    assert!(matches!(
        result_of(&r),
        DropResult::Click | DropResult::None
    ));
}

/// Drags item 0 to left edge 300 (grab 50 in) and returns the pointer's window x.
fn drag_first_to_the_end(h: &mut UiHarness, m: &mut Model, ms: u64) -> f32 {
    let p = point(m, 0, 50.0);
    press_at(h, p);
    run(h, m, ms);
    h.input.pointer_to(p.0 + 5.0, p.1);
    run(h, m, ms + 10);
    // the crossing re-takes the grab at 55 px, so the left edge is 55 px behind the pointer
    let to = (ORIGIN + 300.0 + 55.0) as f32;
    h.input.pointer_to(to, p.1);
    run(h, m, ms + 20);
    to
}

#[test]
fn dragging_reports_the_offsets_and_a_release_commits() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    let to = drag_first_to_the_end(&mut h, &mut m, 10);
    assert_eq!(m.drag.target_index(), Some(2));
    near(m.drag.dragged_left(), 300.0);
    near(m.drag.offsets()[1], -105.0);
    near(m.drag.offsets()[2], -105.0);
    // The node of the dragged item carries Pressed during the gesture.
    let r = run(&mut h, &mut m, 40);
    assert!(
        r.tree
            .get(item(0))
            .is_some_and(|n| n.states.contains(State::Pressed))
    );
    assert!(
        r.tree
            .get(item(1))
            .is_some_and(|n| !n.states.contains(State::Pressed))
    );
    // The dragged item follows the pointer exactly, with no transition.
    near(f64::from(r.offsets[0]), 300.0);

    h.input.pointer_to(to, ROW_Y + 15.0);
    h.input.release(P);
    let r = run(&mut h, &mut m, 50);
    assert_eq!(result_of(&r), DropResult::Commit { from: 0, to: 2 });
    assert!(!m.drag.is_active());
    assert!(m.drag.offsets().iter().all(|&o| o == 0.0));
    // The node is not pressed once it ends.
    assert!(
        r.tree
            .get(item(0))
            .is_some_and(|n| !n.states.contains(State::Pressed))
    );
}

#[test]
fn a_release_where_it_started_is_a_click() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    let p = point(&m, 1, 50.0);
    press_at(&mut h, p);
    run(&mut h, &mut m, 10);
    h.input.pointer_to(p.0 + 6.0, p.1);
    run(&mut h, &mut m, 20);
    assert!(m.drag.is_dragging());
    h.input.release(P);
    let r = run(&mut h, &mut m, 30);
    assert_eq!(result_of(&r), DropResult::Click);
}

#[test]
fn escape_cancels_a_drag_and_is_taken_only_then() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    drag_first_to_the_end(&mut h, &mut m, 10);
    assert!(m.drag.is_dragging());
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let r = run(&mut h, &mut m, 50);
    assert!(cancelled(&r));
    assert!(!m.drag.is_active());
    assert!(m.drag.offsets().iter().all(|&o| o == 0.0));
    assert_eq!(result_of(&r), DropResult::None);
    assert!(r.escape_left.iter().all(|left| !left), "Escape was taken");
    // The still-held button's release later commits nothing.
    h.input.release(P);
    let r = run(&mut h, &mut m, 60);
    assert_eq!(result_of(&r), DropResult::None);
}

#[test]
fn in_a_dialog_the_first_escape_cancels_a_reorder_drag_and_only_the_second_closes_the_dialog() {
    let mut m = Model::new();
    m.dialog = true;
    let mut h = start(&mut m);
    // The dialog opens before the strip takes a press.
    run(&mut h, &mut m, 1000);
    let end = drag_first_to_the_end(&mut h, &mut m, 1010);
    assert!(m.drag.is_dragging(), "dragging to {end}");
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let r = run(&mut h, &mut m, 1100);
    assert!(cancelled(&r), "the drag is cancelled");
    assert!(!m.dialog_cancelled, "and the dialog stays");
    assert!(!m.drag.is_active());
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    run(&mut h, &mut m, 1110);
    assert!(m.dialog_cancelled, "the second Escape closes the dialog");
}

#[test]
fn in_a_dialog_an_armed_press_does_not_take_escape_from_the_dialog() {
    let mut m = Model::new();
    m.dialog = true;
    let mut h = start(&mut m);
    run(&mut h, &mut m, 1000);
    // Armed, not dragging: Escape is the dialog's (the flag means dragging).
    press_at(&mut h, point(&m, 1, 50.0));
    run(&mut h, &mut m, 1010);
    assert!(m.drag.is_armed() && !m.drag.is_dragging());
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    run(&mut h, &mut m, 1020);
    assert!(m.dialog_cancelled);
}

#[test]
fn in_a_dialog_the_first_escape_cancels_a_free_drag_and_only_the_second_closes_the_dialog() {
    let b = board();
    let cols = columns(&b);
    let card = |k: u32| WidgetId::ROOT.with("card").with(k);
    let rect = Rect::new(10.0, 60.0, 80.0, 40.0);
    let mut drag = FreeDrag::new();
    let mut h = UiHarness::new(WIN);
    let frame = |h: &mut UiHarness, ms: u64, drag: &mut FreeDrag| {
        let mut out = Vec::new();
        let mut cancelled = false;
        h.at_ms(ms).frame(|ui: &mut Ui<'_>| {
            cancelled = ui
                .dialog("dlg", &DIALOG, &dialog_glass(), |ui, _, _, _| {
                    let r = ui.interact(card(5), rect, Sense::DRAG);
                    ui.node(Node::new(card(5), Role::ListItem, "card", rect));
                    out.push(ui.free_drag_gesture(card(5), &r, drag, 5, &cols));
                })
                .cancelled;
        });
        (out[0], cancelled)
    };
    frame(&mut h, 0, &mut drag);
    frame(&mut h, 1000, &mut drag);
    h.input.pointer_to(50.0, 80.0);
    h.input.press(P);
    frame(&mut h, 1010, &mut drag);
    h.input.pointer_to(300.0, 80.0);
    frame(&mut h, 1020, &mut drag);
    assert!(drag.is_dragging());
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (f, cancelled) = frame(&mut h, 1030, &mut drag);
    assert!(f.cancelled && !drag.is_active(), "the drag is cancelled");
    assert!(!cancelled, "and the dialog stays");
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (_, cancelled) = frame(&mut h, 1040, &mut drag);
    assert!(cancelled, "the second Escape closes the dialog");
}

#[test]
fn escape_with_nothing_in_flight_is_not_consumed() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    // Idle: no press at all.
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let r = run(&mut h, &mut m, 10);
    assert!(!cancelled(&r));
    // Nothing holds the capture, so the key is not a cancel for anyone either.
    assert!(r.escape_left.iter().all(|left| !left));

    // Armed but not dragging: the press holds the capture, and Escape is left alone by the gesture.
    let p = point(&m, 1, 50.0);
    press_at(&mut h, p);
    run(&mut h, &mut m, 20);
    assert!(m.drag.is_armed());
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let r = run(&mut h, &mut m, 30);
    assert!(m.drag.is_armed(), "armed is not cancelled by Escape");
    assert!(
        r.escape_left.contains(&true),
        "the gesture did not take Escape"
    );
}

#[test]
fn losing_the_capture_cancels() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    drag_first_to_the_end(&mut h, &mut m, 10);
    assert!(m.drag.is_dragging());
    // The items stop reporting (disabled): no release arrives, the capture is gone.
    m.disabled = true;
    let r = run(&mut h, &mut m, 50);
    assert!(cancelled(&r));
    assert!(!m.drag.is_active());
    assert!(m.drag.offsets().iter().all(|&o| o == 0.0));
    assert_eq!(result_of(&r), DropResult::None);
}

#[test]
fn a_change_of_the_item_count_cancels() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    drag_first_to_the_end(&mut h, &mut m, 10);
    assert!(m.drag.is_dragging());
    m.keys.pop();
    let r = run(&mut h, &mut m, 50);
    assert!(cancelled(&r));
    assert!(!m.drag.is_active());
    assert_eq!(result_of(&r), DropResult::None);
}

#[test]
fn a_secondary_press_arms_nothing() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    let p = point(&m, 1, 50.0);
    h.input.pointer_to(p.0, p.1);
    h.input.press(PointerButton::Secondary);
    run(&mut h, &mut m, 10);
    assert!(!m.drag.is_active());
}

#[test]
fn auto_scroll_asks_for_frames_and_steps_by_dt() {
    let mut m = Model::new();
    m.viewport = 200.0;
    m.extent = 410.0;
    let mut h = start(&mut m);
    let p = point(&m, 0, 50.0);
    press_at(&mut h, p);
    run(&mut h, &mut m, 10);
    // Drag to 2 px from the right viewport edge (window x 250): ramp (198 - 176) / 24.
    let edge = (ORIGIN + 200.0 - 2.0) as f32;
    h.input.pointer_to(edge, p.1);
    let r = run(&mut h, &mut m, 26);
    assert!(m.drag.is_dragging());
    let ramp = (198.0 - 176.0) / 24.0;
    near(m.drag.auto_scroll_delta(), 12.0 * ramp);
    assert_eq!(
        r.repaint,
        Repaint::Now,
        "a non-zero delta keeps frames coming"
    );
    let step16 = r.gestures.iter().map(|g| g.scroll_by).fold(0.0, f64::max);
    near(step16, 12.0 * ramp);

    // A 32 ms frame doubles the step; the pointer has not moved.
    let r = run(&mut h, &mut m, 58);
    let step32 = r.gestures.iter().map(|g| g.scroll_by).fold(0.0, f64::max);
    near(step32, 24.0 * ramp);

    // The app applies the scroll; the content moves under the parked pointer.
    m.offset += step32;
    let before = m.drag.dragged_left();
    run(&mut h, &mut m, 74);
    assert!(
        m.drag.dragged_left() > before,
        "the dragged item moves with the content"
    );

    // No room to scroll: no delta, no repaint demand from the gesture.
    m.offset = 210.0;
    let r = run(&mut h, &mut m, 90);
    assert_eq!(m.drag.auto_scroll_delta(), 0.0);
    assert!(r.gestures.iter().all(|g| g.scroll_by == 0.0));
}

#[test]
fn an_idle_strip_settles_to_idle() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    let r = run(&mut h, &mut m, 10);
    let r2 = run(&mut h, &mut m, 20);
    assert_eq!(r2.repaint, Repaint::Idle);
    assert!(r.offsets.iter().all(|&o| o == 0.0));
}

// Motion.

fn at_neighbour(r: &Run) -> f32 {
    r.offsets[1]
}

/// Drags item 0 across item 1 (one slot) at `ms` and returns the harness.
fn cross_one_slot(m: &mut Model, tier: MotionTier) -> UiHarness {
    let mut h = start(m);
    h.tier = tier;
    let p = point(m, 0, 50.0);
    press_at(&mut h, p);
    run(&mut h, m, 10);
    h.input.pointer_to(p.0 + 5.0, p.1);
    run(&mut h, m, 20);
    h
}

#[test]
fn a_neighbour_slides_over_fast_with_sine_in_out_and_the_dragged_item_does_not() {
    let mut m = Model::new();
    let mut h = cross_one_slot(&mut m, MotionTier::Full);
    // Move the dragged item one full slot right: item 1 (width 200) is passed.
    let p = point(&m, 0, 50.0);
    h.input.pointer_to(p.0 + 205.0 + 5.0, p.1);
    let first = run(&mut h, &mut m, 100);
    assert_eq!(m.drag.target_index(), Some(1));
    // The dragged item is at the pointer at once; the neighbour has barely begun.
    near(f64::from(first.offsets[0]), m.drag.dragged_left());
    assert!(at_neighbour(&first).abs() < 1.0, "{}", at_neighbour(&first));
    // Halfway through `fast` (90 ms) sine-in-out is at 0.5 of the 100 + 5 slot shift.
    let mid = run(&mut h, &mut m, 190);
    near(f64::from(at_neighbour(&mid)), -52.5);
    // The end is exact, and the frame asks for another while it moves.
    assert_eq!(mid.repaint, Repaint::Now);
    let end = run(&mut h, &mut m, 280);
    near(f64::from(at_neighbour(&end)), -105.0);
    let after = run(&mut h, &mut m, 290);
    assert_eq!(after.repaint, Repaint::Idle);
}

#[test]
fn reduced_and_off_snap_the_slide() {
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let mut m = Model::new();
        let mut h = cross_one_slot(&mut m, tier);
        let p = point(&m, 0, 50.0);
        h.input.pointer_to(p.0 + 205.0 + 5.0, p.1);
        let r = run(&mut h, &mut m, 100);
        near(f64::from(at_neighbour(&r)), -105.0);
    }
}

#[test]
fn slide_and_settle_pass_the_60_hz_conformance_harness() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for motion in [SLIDE, SETTLE] {
            conformance(&motion, &DARK.motion, tier, 0.0f32, -105.0f32).expect("conforms");
        }
    }
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&DIM, &DARK.motion, tier, 1.0f32, DIM_ALPHA).expect("conforms");
    }
}

/// Release over the end slot at `ms`, apply the commit, and return the frame that drew after it.
fn release_and_commit(h: &mut UiHarness, m: &mut Model, to_x: f32, ms: u64) -> Run {
    h.input.pointer_to(to_x, ROW_Y + 15.0);
    h.input.release(P);
    let r = run(h, m, ms);
    let DropResult::Commit { from, to } = result_of(&r) else {
        panic!("expected a commit");
    };
    m.apply(from, to);
    r
}

#[test]
fn the_released_item_settles_over_fast_into_its_slot() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    let to = drag_first_to_the_end(&mut h, &mut m, 10);
    // Let the neighbours finish their slides before the release.
    run(&mut h, &mut m, 300);
    release_and_commit(&mut h, &mut m, to, 400);
    // New order [1, 2, 0]: the released item is at index 2, layout x 310; it was drawn at 300.
    assert_eq!(m.keys, vec![1, 2, 0]);
    let r = run(&mut h, &mut m, 400);
    near(f64::from(r.offsets[2]), -10.0);
    // The neighbours sit in their slots already.
    near(f64::from(r.offsets[0]), 0.0);
    near(f64::from(r.offsets[1]), 0.0);
    // 90 ms in, halfway.
    let mid = run(&mut h, &mut m, 490);
    near(f64::from(mid.offsets[2]), -5.0);
    assert_eq!(mid.repaint, Repaint::Now);
    // 180 ms in, settled exactly; the retention ends and the frame goes idle.
    let end = run(&mut h, &mut m, 580);
    near(f64::from(end.offsets[2]), 0.0);
    let idle = run(&mut h, &mut m, 600);
    assert_eq!(idle.repaint, Repaint::Idle);
    assert!(idle.offsets.iter().all(|&o| o == 0.0));
}

#[test]
fn reduced_and_off_have_no_settle() {
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let mut m = Model::new();
        let mut h = start(&mut m);
        h.tier = tier;
        let to = drag_first_to_the_end(&mut h, &mut m, 10);
        run(&mut h, &mut m, 300);
        release_and_commit(&mut h, &mut m, to, 400);
        let r = run(&mut h, &mut m, 400);
        assert!(
            r.offsets.iter().all(|&o| o == 0.0),
            "{tier:?}: {:?}",
            r.offsets
        );
    }
}

#[test]
fn a_cancel_clears_the_offsets_at_once() {
    let mut m = Model::new();
    let mut h = start(&mut m);
    drag_first_to_the_end(&mut h, &mut m, 10);
    run(&mut h, &mut m, 300);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let r = run(&mut h, &mut m, 310);
    assert!(r.offsets.iter().all(|&o| o == 0.0), "{:?}", r.offsets);
}

// The painted look, at scale 2.

fn shapes(scene: &Scene) -> Vec<Shape> {
    scene
        .layers
        .iter()
        .flat_map(|l| l.geometry.under.iter().chain(&l.geometry.over))
        .copied()
        .collect()
}

fn premul(c: [u8; 4]) -> [f32; 4] {
    let a = f32::from(c[3]) / 255.0;
    [
        f32::from(c[0]) / 255.0 * a,
        f32::from(c[1]) / 255.0 * a,
        f32::from(c[2]) / 255.0 * a,
        a,
    ]
}

#[test]
fn the_drop_line_is_2_px_accent_4_device_px_at_scale_2() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let (scene, _) = h.at_ms(0).frame(|ui| {
        ui.painter().on(LayerId::Page);
        let theme = *ui.theme();
        paint_drop_line(ui.painter(), Rect::new(20.0, 50.0, 120.0, 8.0), &theme);
    });
    let s = shapes(&scene);
    assert_eq!(s.len(), 1);
    // 2 logical px tall, centred in the 8 px gap: y = 50 + 3.
    assert_eq!(s[0].rect, [40.0, 106.0, 240.0, 4.0]);
    let want = premul(DARK.accent);
    for i in 0..4 {
        assert!(
            (s[0].color[i] - want[i]).abs() < 1e-4,
            "{:?} vs {want:?}",
            s[0].color
        );
    }
}

#[test]
fn the_dimmed_source_card_is_drawn_at_alpha_0_4() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let id = WidgetId::ROOT.with("card");
    let draw = |h: &mut UiHarness, ms: u64, dragged: bool| {
        h.at_ms(ms)
            .frame(|ui| {
                let a = ui.drag_dim(id, dragged);
                ui.painter().push_xform(Xform {
                    origin: (0.0, 0.0),
                    scale: 1.0,
                    dx: 0.0,
                    dy: 0.0,
                    alpha: a,
                });
                ui.painter().fill(
                    Rect::new(10.0, 10.0, 100.0, 40.0),
                    4.0,
                    [255, 255, 255, 255],
                );
                ui.painter().pop_xform();
            })
            .0
    };
    let alpha = |scene: &Scene| shapes(scene)[0].color[3];
    near(f64::from(alpha(&draw(&mut h, 0, false))), 1.0);
    // The dim runs over `fast`, linear: halfway at 90 ms, 0.4 at 180 ms.
    draw(&mut h, 10, true);
    near(f64::from(alpha(&draw(&mut h, 100, true))), 0.7);
    near(f64::from(alpha(&draw(&mut h, 190, true))), 0.4);
    // Off snaps.
    let mut o = UiHarness::new(WIN);
    o.tier = MotionTier::Off;
    o.input.set_scale(2.0);
    draw(&mut o, 0, false);
    near(f64::from(alpha(&draw(&mut o, 10, true))), 0.4);
}

// The free glue.

#[test]
fn the_free_glue_drops_on_a_column_and_cancels_on_escape() {
    let b = board();
    let cols = columns(&b);
    let card = |k: u32| WidgetId::ROOT.with("card").with(k);
    let rect = Rect::new(10.0, 60.0, 80.0, 40.0);
    let mut drag = FreeDrag::new();
    let mut h = UiHarness::new(WIN);
    let frame = |h: &mut UiHarness, ms: u64, drag: &mut FreeDrag| {
        let mut out = Vec::new();
        let (_, fo) = h.at_ms(ms).frame(|ui: &mut Ui<'_>| {
            let r = ui.interact(card(5), rect, Sense::DRAG);
            ui.node(Node::new(card(5), Role::ListItem, "card", rect));
            out.push(ui.free_drag_gesture(card(5), &r, drag, 5, &cols));
        });
        (out[0], fo)
    };
    frame(&mut h, 0, &mut drag);
    h.input.pointer_to(50.0, 80.0);
    h.input.press(P);
    let (f, _) = frame(&mut h, 10, &mut drag);
    assert!(drag.is_armed() && !f.dragging);
    h.input.pointer_to(53.0, 83.0);
    let (f, fo) = frame(&mut h, 20, &mut drag);
    assert!(f.dragging);
    assert!(
        fo.tree
            .get(card(5))
            .is_some_and(|n| n.states.contains(State::Pressed))
    );
    h.input.pointer_to(300.0, 80.0);
    frame(&mut h, 30, &mut drag);
    assert_eq!(
        drag.target(),
        Some(DropTarget {
            column: 2,
            index: 1
        })
    );
    h.input.release(P);
    let (f, _) = frame(&mut h, 40, &mut drag);
    assert_eq!(
        f.result,
        FreeResult::Drop {
            source: 5,
            target: DropTarget {
                column: 2,
                index: 1
            }
        }
    );

    // Escape cancels a second drag.
    h.input.pointer_to(50.0, 80.0);
    h.input.press(P);
    frame(&mut h, 50, &mut drag);
    h.input.pointer_to(300.0, 80.0);
    frame(&mut h, 60, &mut drag);
    assert!(drag.is_dragging());
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (f, _) = frame(&mut h, 70, &mut drag);
    assert!(f.cancelled);
    assert!(!drag.is_active());
}

/// A reorder gesture in screen terms: the first of three items pressed, dragged and released at the
/// same screen points, on a strip placed at screen x `ORIGIN` directly or through a parent scrolled
/// by (30, 100).
fn reorder_result(scrolled: bool) -> DropResult {
    let widths = [100.0, 100.0, 100.0];
    let mut drag = ReorderDrag::new();
    let mut h = UiHarness::new(WIN);
    let (cx, cy) = if scrolled { (30.0, 100.0) } else { (0.0, 0.0) };
    let area = Rect::new(0.0, 0.0, 590.0, 390.0);
    let opts = ScrollOptions {
        extent: Some((1000.0, 1000.0)),
        ..ScrollOptions::default()
    };
    let frame = |h: &mut UiHarness, ms: u64, drag: &mut ReorderDrag| {
        let mut result = DropResult::None;
        let mut items = |ui: &mut Ui<'_>| {
            let strip_at = Rect::new(ORIGIN as f32 + cx, 40.0 + cy, 320.0, ITEM_H);
            let origin = f64::from(ui.painter().transformed(strip_at).x);
            let m = Metrics {
                viewport: 400.0,
                offset: 0.0,
                extent: 310.0,
                ..metrics(&widths)
            };
            let r = ui.interact(
                item(1),
                Rect {
                    w: 100.0,
                    ..strip_at
                },
                Sense::DRAG,
            );
            let strip = Strip {
                index: 0,
                origin,
                metrics: m,
            };
            result = ui.drag_gesture(item(1), &r, drag, &strip).result;
        };
        h.at_ms(ms).frame(|ui| {
            if scrolled {
                ui.scroll_area("o", area, &opts, |ui, _| {
                    items(ui);
                    ((), Extent::default())
                });
            } else {
                items(ui);
            }
        });
        result
    };
    h.input.pointer_to(580.0, 380.0);
    frame(&mut h, 0, &mut drag);
    if scrolled {
        h.input.scroll_pixels(-cx, -cy);
        frame(&mut h, 5, &mut drag);
        frame(&mut h, 10, &mut drag);
    }
    h.input.pointer_to(ORIGIN as f32 + 50.0, 50.0);
    frame(&mut h, 20, &mut drag);
    h.input.press(P);
    frame(&mut h, 30, &mut drag);
    h.input.pointer_to(ORIGIN as f32 + 110.0, 50.0);
    frame(&mut h, 40, &mut drag);
    h.input.pointer_to(ORIGIN as f32 + 170.0, 50.0);
    frame(&mut h, 50, &mut drag);
    h.input.release(P);
    frame(&mut h, 60, &mut drag)
}

#[test]
fn a_reorder_gesture_in_a_scrolled_parent_reads_the_same_as_on_a_plain_page() {
    let plain = reorder_result(false);
    assert!(
        matches!(plain, DropResult::Commit { from: 0, .. }),
        "{plain:?}"
    );
    assert_eq!(reorder_result(true), plain);
}

/// [`reorder_result`] for a card dragged to the third column of a board.
fn free_result(scrolled: bool) -> FreeResult {
    let mut drag = FreeDrag::new();
    let mut h = UiHarness::new(WIN);
    let (cx, cy) = if scrolled { (30.0, 100.0) } else { (0.0, 0.0) };
    let area = Rect::new(0.0, 0.0, 590.0, 390.0);
    let opts = ScrollOptions {
        extent: Some((1000.0, 1000.0)),
        ..ScrollOptions::default()
    };
    let card = WidgetId::ROOT.with("card");
    let frame = |h: &mut UiHarness, ms: u64, drag: &mut FreeDrag| {
        let mut result = FreeResult::None;
        let mut items = |ui: &mut Ui<'_>| {
            let at = |ui: &mut Ui<'_>, x: f32, y: f32| {
                let r = ui
                    .painter()
                    .transformed(Rect::new(x + cx, y + cy, 0.0, 0.0));
                (f64::from(r.x), f64::from(r.y))
            };
            let mids = [at(ui, 0.0, 20.0).1, at(ui, 0.0, 60.0).1];
            let columns = [0.0, 110.0, 220.0].map(|x| Column {
                x: at(ui, x, 0.0).0,
                w: 100.0,
                card_mids: &mids,
            });
            let rect = Rect::new(10.0 + cx, 60.0 + cy, 80.0, 40.0);
            let r = ui.interact(card, rect, Sense::DRAG);
            result = ui.free_drag_gesture(card, &r, drag, 5, &columns).result;
        };
        h.at_ms(ms).frame(|ui| {
            if scrolled {
                ui.scroll_area("o", area, &opts, |ui, _| {
                    items(ui);
                    ((), Extent::default())
                });
            } else {
                items(ui);
            }
        });
        result
    };
    h.input.pointer_to(580.0, 380.0);
    frame(&mut h, 0, &mut drag);
    if scrolled {
        h.input.scroll_pixels(-cx, -cy);
        frame(&mut h, 5, &mut drag);
        frame(&mut h, 10, &mut drag);
    }
    h.input.pointer_to(50.0, 80.0);
    frame(&mut h, 20, &mut drag);
    h.input.press(P);
    frame(&mut h, 30, &mut drag);
    h.input.pointer_to(53.0, 83.0);
    frame(&mut h, 40, &mut drag);
    h.input.pointer_to(300.0, 40.0);
    frame(&mut h, 50, &mut drag);
    h.input.release(P);
    frame(&mut h, 60, &mut drag)
}

#[test]
fn a_free_gesture_in_a_scrolled_parent_reads_the_same_as_on_a_plain_page() {
    let plain = free_result(false);
    let want = FreeResult::Drop {
        source: 5,
        target: DropTarget {
            column: 2,
            index: 1,
        },
    };
    assert_eq!(plain, want);
    assert_eq!(free_result(true), plain);
}

// The edge zone, shared by the strip and the board.

#[test]
fn edge_step_is_the_strip_s_zone_on_any_viewport() {
    use lune_ui_core::drag::edge_step;
    // The acceptance table, on `0..500` and moved to `100..600`.
    for lo in [0.0, 100.0] {
        let hi = lo + 500.0;
        assert_eq!(edge_step(lo, lo, hi, true, true), -12.0);
        assert_eq!(edge_step(lo + 12.0, lo, hi, true, true), -6.0);
        assert_eq!(edge_step(lo + 24.0, lo, hi, true, true), 0.0);
        assert_eq!(edge_step(lo - 50.0, lo, hi, true, true), -12.0);
        assert_eq!(edge_step(hi, lo, hi, true, true), 12.0);
        assert_eq!(edge_step(hi - 12.0, lo, hi, true, true), 6.0);
        assert_eq!(edge_step(hi - 24.0, lo, hi, true, true), 0.0);
        assert_eq!(edge_step(hi + 50.0, lo, hi, true, true), 12.0);
        assert_eq!(edge_step(lo, lo, hi, false, true), 0.0);
        assert_eq!(edge_step(hi, lo, hi, true, false), 0.0);
    }
    assert_eq!(edge_step(f64::NAN, 0.0, 500.0, true, true), 0.0);
    assert_eq!(edge_step(0.0, f64::INFINITY, 500.0, true, true), 0.0);
}

#[test]
fn a_free_press_and_release_in_one_frame_is_a_click() {
    let b = board();
    let cols = columns(&b);
    let card = WidgetId::ROOT.with("card");
    let rect = Rect::new(10.0, 60.0, 80.0, 40.0);
    let mut drag = FreeDrag::new();
    let mut h = UiHarness::new(WIN);
    let frame = |h: &mut UiHarness, ms: u64, drag: &mut FreeDrag| {
        let mut out = None;
        h.at_ms(ms).frame(|ui: &mut Ui<'_>| {
            let r = ui.interact(card, rect, Sense::DRAG);
            out = Some(ui.free_drag_gesture(card, &r, drag, 5, &cols));
        });
        out.expect("a frame")
    };
    frame(&mut h, 0, &mut drag);
    h.input.pointer_to(50.0, 80.0);
    frame(&mut h, 10, &mut drag);
    h.input.press(P);
    let armed = frame(&mut h, 20, &mut drag);
    assert_eq!(armed.result, FreeResult::None);
    assert_eq!(drag.pressed(), Some(5));
    assert_eq!(drag.source(), None, "armed, no drag");
    h.input.release(P);
    assert_eq!(frame(&mut h, 30, &mut drag).result, FreeResult::Click);
    assert_eq!(drag.pressed(), None);
    h.input.click(P);
    assert_eq!(frame(&mut h, 40, &mut drag).result, FreeResult::Click);
    assert!(!drag.is_active());
}

// A free drag on a card in a dialog, with a second dialog over it on request: what each frame's
// gesture and the two dialogs report.
struct InDialog {
    gesture: FreeFrame,
    first: bool,
    second: bool,
}

fn free_in_dialog(
    h: &mut UiHarness,
    ms: u64,
    drag: &mut FreeDrag,
    cols: &[Column<'_>],
    open_second: bool,
) -> InDialog {
    let card = WidgetId::ROOT.with("card").with(5_u32);
    let rect = Rect::new(10.0, 60.0, 80.0, 40.0);
    let mut gesture = None;
    let mut second = false;
    let mut first = false;
    h.at_ms(ms).frame(|ui: &mut Ui<'_>| {
        first = ui
            .dialog("dlg", &DIALOG, &dialog_glass(), |ui, _, _, _| {
                let r = ui.interact(card, rect, Sense::DRAG);
                ui.node(Node::new(card, Role::ListItem, "card", rect));
                gesture = Some(ui.free_drag_gesture(card, &r, drag, 5, cols));
                if open_second {
                    second = ui
                        .dialog("over", &DIALOG, &dialog_glass(), |_, _, _, _| {})
                        .cancelled;
                }
            })
            .cancelled;
    });
    InDialog {
        gesture: gesture.expect("the frame ran"),
        first,
        second,
    }
}

/// Up to a live free drag in the dialog, with the pointer over the second column.
fn dragging_in_dialog(h: &mut UiHarness, drag: &mut FreeDrag, cols: &[Column<'_>]) {
    free_in_dialog(h, 0, drag, cols, false);
    free_in_dialog(h, 1000, drag, cols, false);
    h.input.pointer_to(50.0, 80.0);
    h.input.press(P);
    free_in_dialog(h, 1010, drag, cols, false);
    h.input.pointer_to(300.0, 80.0);
    free_in_dialog(h, 1020, drag, cols, false);
    assert!(drag.is_dragging());
}

#[test]
fn in_a_dialog_an_armed_free_drag_press_does_not_take_escape_from_the_dialog() {
    let b = board();
    let cols = columns(&b);
    let mut drag = FreeDrag::new();
    let mut h = UiHarness::new(WIN);
    free_in_dialog(&mut h, 0, &mut drag, &cols, false);
    free_in_dialog(&mut h, 1000, &mut drag, &cols, false);
    h.input.pointer_to(50.0, 80.0);
    h.input.press(P);
    free_in_dialog(&mut h, 1010, &mut drag, &cols, false);
    // The frame after the press too: the gesture has run armed to its end, where the note would be.
    free_in_dialog(&mut h, 1020, &mut drag, &cols, false);
    assert!(drag.is_active() && !drag.is_dragging(), "armed");
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let f = free_in_dialog(&mut h, 1030, &mut drag, &cols, false);
    assert!(f.first, "the flag means dragging: Escape is the dialog's");
}

#[test]
fn an_escape_after_the_release_in_one_frame_closes_the_dialog() {
    let b = board();
    let cols = columns(&b);
    let mut drag = FreeDrag::new();
    let mut h = UiHarness::new(WIN);
    dragging_in_dialog(&mut h, &mut drag, &cols);
    h.input.release(P);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let f = free_in_dialog(&mut h, 1030, &mut drag, &cols, false);
    assert!(f.first, "at the key no gesture held the capture");
}

#[test]
fn an_escape_in_the_frame_a_second_dialog_takes_the_capture_closes_that_dialog() {
    let b = board();
    let cols = columns(&b);
    let mut drag = FreeDrag::new();
    let mut h = UiHarness::new(WIN);
    dragging_in_dialog(&mut h, &mut drag, &cols);
    free_in_dialog(&mut h, 1030, &mut drag, &cols, true);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let f = free_in_dialog(&mut h, 1040, &mut drag, &cols, true);
    assert!(f.second && !f.first, "the top dialog, and only it");
}

/// The limit D176 states: at the key the press still holds, so the key queues, and at the gesture's
/// turn the press is gone. The drop commits and the one key is lost; the next Escape dismisses.
#[test]
fn an_escape_before_the_release_in_one_frame_is_lost_and_the_next_closes_the_dialog() {
    let b = board();
    let cols = columns(&b);
    let mut drag = FreeDrag::new();
    let mut h = UiHarness::new(WIN);
    dragging_in_dialog(&mut h, &mut drag, &cols);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    h.input.release(P);
    let f = free_in_dialog(&mut h, 1030, &mut drag, &cols, false);
    assert!(!f.first && !f.gesture.cancelled, "the key is lost");
    assert!(
        matches!(f.gesture.result, FreeResult::Drop { .. }),
        "and the drop commits"
    );
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let f = free_in_dialog(&mut h, 1040, &mut drag, &cols, false);
    assert!(f.first);
}

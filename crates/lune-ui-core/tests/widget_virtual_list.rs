//! The virtualized list (`layout.virtualized-list` acceptance): the height tree against a plain
//! model, the build bound and per-frame cost at a million rows, `f64` placement, the pointer, the
//! keys, focus, the look at scale 2, motion, measured rows, the node, retention and stale anchors.

use std::cell::Cell;
use std::collections::BTreeSet;

use lune_ui_core::access::{AccessAction, Action, Role, State};
use lune_ui_core::input::{FrameOutput, Key, Keymap, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Ui, WidgetId};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::virtual_list::{
    HOVER, Heights, ListOptions, ListResponse, ListState, MARGIN, RowCtx, RowHeights, RowKey,
    SELECTION, SelectionMode, row_fill,
};

const WIN: (f32, f32) = (400.0, 600.0);
const LIST: Rect = Rect::new(20.0, 20.0, 300.0, 400.0);
const ROW: f32 = 45.0;
const MILLION: usize = 1_000_000;
const P: PointerButton = PointerButton::Primary;
/// `ceil(400 / 45) + 2 * MARGIN`.
const BOUND: usize = 9 + 2 * MARGIN;

fn opts(count: usize) -> ListOptions<'static> {
    ListOptions {
        count,
        heights: Heights::Fixed(ROW),
        selection: SelectionMode::Multi,
        index_of: None,
        label: "rows",
    }
}

fn key(i: usize) -> RowKey {
    RowKey(i as u64)
}

fn list_id() -> WidgetId {
    WidgetId::ROOT.with("l")
}

fn row_id(k: u64) -> WidgetId {
    list_id().with(RowKey(k))
}

struct Run {
    r: ListResponse,
    rows: Vec<RowCtx>,
    out: FrameOutput,
    scene: Scene,
    state: ListState,
}

/// One frame of the list; `draw` runs for each built row.
fn run_with(
    h: &mut UiHarness,
    o: &ListOptions<'_>,
    keys: &dyn Fn(usize) -> RowKey,
    draw: &mut dyn FnMut(&mut Ui<'_>, &RowCtx) -> f32,
) -> Run {
    let mut rows = Vec::new();
    let mut r = None;
    let mut state = ListState::default();
    let (scene, out) = h.frame(|ui| {
        r = Some(ui.virtual_list("l", LIST, o, keys, |ui, c| {
            rows.push(*c);
            draw(ui, c)
        }));
        state = ui.list_state("l").clone();
    });
    Run {
        r: r.expect("the frame ran"),
        rows,
        out,
        scene,
        state,
    }
}

fn run(h: &mut UiHarness, o: &ListOptions<'_>) -> Run {
    run_with(h, o, &key, &mut |_, _| ROW)
}

fn harness() -> UiHarness {
    UiHarness::new(WIN)
}

/// A point in row `i` while the list is at offset 0.
fn over(i: usize) -> (f32, f32) {
    (100.0, LIST.y + (i as f32 + 0.5) * ROW)
}

fn click_row(h: &mut UiHarness, o: &ListOptions<'_>, i: usize, m: Modifiers) -> Run {
    let (x, y) = over(i);
    h.input.pointer_to(x, y).set_modifiers(m).click(P);
    let r = run(h, o);
    h.input.set_modifiers(Modifiers::default());
    r
}

fn press_key(h: &mut UiHarness, o: &ListOptions<'_>, k: NamedKey, m: Modifiers) -> Run {
    h.input.set_modifiers(m).key_down(Key::Named(k), None);
    let r = run(h, o);
    h.input.set_modifiers(Modifiers::default());
    r
}

const SHIFT: Modifiers = Modifiers {
    shift: true,
    control: false,
    alt: false,
    logo: false,
};
const META: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: true,
};
const NONE: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: false,
};

fn keys(s: &[u64]) -> BTreeSet<RowKey> {
    s.iter().map(|k| RowKey(*k)).collect()
}

/// Settles the frame's follow-ups.
fn settle(h: &mut UiHarness, o: &ListOptions<'_>, ms: &mut u64) -> Run {
    loop {
        *ms += 400;
        h.at_ms(*ms);
        let r = run(h, o);
        if r.out.repaint == Repaint::Idle {
            return r;
        }
    }
}

// The height tree.

/// A deterministic generator: the tests must not depend on the process.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % below
    }
}

/// The plain model: every row's height in a list, summed from the start.
fn model_top(rows: &[f64], i: usize) -> f64 {
    rows[..i].iter().sum()
}

fn assert_matches_model(t: &RowHeights, rows: &[f64]) {
    assert_eq!(t.count(), rows.len());
    assert_eq!(t.total(), model_top(rows, rows.len()));
    for i in 0..rows.len() {
        let top = model_top(rows, i);
        assert_eq!(t.top(i), top, "top of row {i}");
        assert_eq!(t.height(i), rows[i], "height of row {i}");
        assert_eq!(t.row_at(top), i, "row at the top of row {i}");
        assert_eq!(
            t.row_at(top + rows[i] - 0.5),
            i,
            "row near the end of row {i}"
        );
    }
}

#[test]
fn the_tree_agrees_with_a_plain_model_through_sets_appends_and_truncations() {
    let mut rng = Lcg(7);
    let mut t = RowHeights::default();
    let heights = Heights::Measured { estimate: 30.0 };
    let mut rows = vec![30.0; 37];
    t.sync(rows.len(), heights);
    assert_matches_model(&t, &rows);
    for round in 0..400 {
        match rng.next(10) {
            0 => {
                let n = rows.len() + rng.next(20);
                rows.resize(n, 30.0);
                t.sync(n, heights);
            }
            1 => {
                let n = rng.next(rows.len()) + 1;
                rows.truncate(n);
                t.sync(n, heights);
            }
            _ => {
                let i = rng.next(rows.len());
                let v = (rng.next(90) + 1) as f64;
                let change = t.set(i, v as f32);
                assert_eq!(change, v - rows[i], "round {round}");
                rows[i] = v;
            }
        }
        assert_matches_model(&t, &rows);
    }
}

#[test]
fn a_new_estimate_keeps_measured_rows_and_moves_the_others() {
    let mut t = RowHeights::default();
    t.sync(5, Heights::Measured { estimate: 30.0 });
    t.set(1, 70.0);
    t.sync(5, Heights::Measured { estimate: 40.0 });
    assert_matches_model(&t, &[40.0, 70.0, 40.0, 40.0, 40.0]);
}

#[test]
fn a_row_cannot_measure_to_nothing_and_a_nan_changes_nothing() {
    let mut t = RowHeights::default();
    t.sync(3, Heights::Measured { estimate: 30.0 });
    assert_eq!(t.set(0, 0.0), 1.0 - 30.0);
    assert_eq!(t.set(1, f32::NAN), 0.0);
    assert_eq!(t.set(9, 50.0), 0.0);
    assert_matches_model(&t, &[1.0, 30.0, 30.0]);
}

#[test]
fn fixed_rows_are_arithmetic_and_keep_exact_positions_at_a_million() {
    let mut t = RowHeights::default();
    t.sync(MILLION, Heights::Fixed(ROW));
    assert_eq!(t.total(), 45_000_000.0);
    assert_eq!(t.top(999_990), 44_999_550.0);
    assert_eq!(t.row_at(44_999_550.0), 999_990);
    assert_eq!(t.row_at(44_999_549.5), 999_989);
    assert_eq!(t.row_at(1e12), MILLION - 1);
    assert_eq!(t.row_at(-3.0), 0);
    assert_eq!(t.set(4, 99.0), 0.0);
}

#[test]
fn a_height_that_does_not_divide_evenly_still_finds_its_row() {
    let mut t = RowHeights::default();
    t.sync(MILLION, Heights::Fixed(33.3));
    for i in [1, 3, 7, 1000, 123_456, 999_999] {
        assert_eq!(t.row_at(t.top(i)), i);
        assert_eq!(t.row_at(t.top(i) - 0.01), i - 1);
    }
}

#[test]
fn a_bad_height_falls_back_to_one_pixel_rows() {
    for bad in [0.0, -4.0, f32::NAN, f32::INFINITY] {
        let mut t = RowHeights::default();
        t.sync(10, Heights::Fixed(bad));
        assert_eq!(t.total(), 10.0);
    }
}

#[test]
fn no_rows_is_an_empty_window() {
    let mut t = RowHeights::default();
    t.sync(0, Heights::Measured { estimate: 30.0 });
    assert_eq!(t.window(0.0, 400.0), 0..0);
    assert_eq!(t.total(), 0.0);
    assert_eq!(t.row_at(50.0), 0);
}

#[test]
fn the_window_covers_the_viewport_and_keeps_the_bound_at_every_offset() {
    let mut t = RowHeights::default();
    t.sync(1000, Heights::Fixed(ROW));
    let max = 1000.0 * 45.0 - 400.0;
    let mut offset = 0.0;
    while offset <= max {
        let w = t.window(offset, 400.0);
        assert!(w.len() <= BOUND, "{} rows at {offset}", w.len());
        assert!(t.top(w.start) <= offset);
        assert!(w.end == 1000 || t.top(w.end) >= offset + 400.0);
        offset += 7.25;
    }
}

// Virtualization at a million rows.

#[test]
fn a_million_rows_build_at_most_the_bound_each_frame_at_every_scroll_position() {
    let o = opts(MILLION);
    let mut h = harness();
    h.input.pointer_to(100.0, 100.0);
    run(&mut h, &o);
    for step in 1..200u64 {
        h.input.scroll_pixels(0.0, -(10_007.0 + step as f32));
        h.at_ms(step * 16);
        let r = run(&mut h, &o);
        assert!(r.rows.len() <= BOUND, "{} rows", r.rows.len());
        assert!(!r.rows.is_empty());
    }
    assert!(run(&mut h, &o).r.offset > 2_000_000.0);
}

/// `row_key` and `draw` calls over a scripted session.
fn calls_for(count: usize, heights: Heights) -> (usize, usize) {
    let (key_calls, draws) = (Cell::new(0), Cell::new(0));
    let mut o = opts(count);
    o.heights = heights;
    let counting = |i| {
        key_calls.set(key_calls.get() + 1);
        key(i)
    };
    let mut h = harness();
    h.input.pointer_to(100.0, 100.0);
    run_with(&mut h, &o, &counting, &mut |_, _| ROW);
    key_calls.set(0);
    draws.set(0);
    for step in 1..60u64 {
        h.input.scroll_pixels(0.0, -130.0);
        h.at_ms(step * 16);
        run_with(&mut h, &o, &counting, &mut |_, _| {
            draws.set(draws.get() + 1);
            ROW
        });
    }
    (key_calls.get(), draws.get())
}

#[test]
fn a_frames_work_does_not_depend_on_the_row_count() {
    for heights in [Heights::Fixed(ROW), Heights::Measured { estimate: ROW }] {
        assert_eq!(calls_for(10_000, heights), calls_for(MILLION, heights));
    }
}

#[test]
fn a_million_row_shift_click_builds_the_set_once_not_every_frame() {
    let o = opts(MILLION);
    let calls = Cell::new(0);
    let counting = |i| {
        calls.set(calls.get() + 1);
        key(i)
    };
    let mut h = harness();
    run_with(&mut h, &o, &counting, &mut |_, _| ROW);
    let (x, y) = over(0);
    h.input.pointer_to(x, y).click(P);
    h.at_ms(100);
    run_with(&mut h, &o, &counting, &mut |_, _| ROW);
    h.at_ms(600);
    run_with(&mut h, &o, &counting, &mut |_, _| ROW);
    h.input
        .set_modifiers(SHIFT)
        .key_down(Key::Named(NamedKey::End), None);
    h.at_ms(1500);
    let r = run_with(&mut h, &o, &counting, &mut |_, _| ROW);
    h.input.set_modifiers(NONE);
    assert_eq!(r.state.selection.len(), MILLION);
    calls.set(0);
    for ms in 0..20 {
        h.at_ms(2000 + ms * 16);
        run_with(&mut h, &o, &counting, &mut |_, _| ROW);
    }
    assert!(calls.get() <= 20 * BOUND);
}

#[test]
fn rows_near_the_end_of_a_million_are_placed_exact_to_a_thousandth() {
    let o = opts(MILLION);
    let mut h = harness();
    run(&mut h, &o);
    h.input.pointer_to(100.0, 100.0).click(P);
    h.at_ms(100);
    run(&mut h, &o);
    h.at_ms(200);
    run(&mut h, &o);
    press_key(&mut h, &o, NamedKey::End, NONE);
    h.at_ms(300);
    let r = run(&mut h, &o);
    assert_eq!(r.r.offset, 44_999_600.0);
    let row = r
        .rows
        .iter()
        .find(|c| c.index == 999_990)
        .expect("margin row");
    // top 44,999,550 at offset 44,999,600: 50 px above the viewport.
    assert!(
        (row.rect.y - (LIST.y - 50.0)).abs() <= 1e-3,
        "{}",
        row.rect.y
    );
    let last = r
        .rows
        .iter()
        .find(|c| c.index == 999_999)
        .expect("last row");
    assert!(
        (last.rect.y - (LIST.y + 355.0)).abs() <= 1e-3,
        "{}",
        last.rect.y
    );
}

// The pointer.

#[test]
fn a_click_selects_one_row_and_sets_the_anchor() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    let r = click_row(&mut h, &o, 3, NONE);
    assert_eq!(r.state.selection, keys(&[3]));
    assert_eq!(r.state.anchor, Some(RowKey(3)));
    assert_eq!(r.state.cursor, Some(RowKey(3)));
    assert!(r.r.response.changed);
    let r = click_row(&mut h, &o, 5, NONE);
    assert_eq!(r.state.selection, keys(&[5]));
    let row = |r: &Run, i: usize| r.rows[i].selected;
    assert!(
        row(&r, 5) && !row(&r, 3),
        "the same frame shows both rows' new state"
    );
}

#[test]
fn the_primary_modifier_toggles_and_keeps_the_rest() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    assert_eq!(
        click_row(&mut h, &o, 5, META).state.selection,
        keys(&[2, 5])
    );
    assert_eq!(click_row(&mut h, &o, 2, META).state.selection, keys(&[5]));
}

#[test]
fn the_primary_modifier_is_the_keymaps_not_a_fixed_key() {
    let o = opts(100);
    let mut h = harness();
    h.keymap = Keymap::PC;
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    let control = Modifiers {
        control: true,
        ..NONE
    };
    assert_eq!(
        click_row(&mut h, &o, 5, control).state.selection,
        keys(&[2, 5])
    );
    assert_eq!(click_row(&mut h, &o, 7, META).state.selection, keys(&[7]));
}

#[test]
fn shift_selects_anchor_to_row_replacing_the_set() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    click_row(&mut h, &o, 8, META);
    assert_eq!(
        click_row(&mut h, &o, 4, SHIFT).state.selection,
        keys(&[4, 5, 6, 7, 8]),
    );
    // the anchor stays, so the range turns round it
    let r = click_row(&mut h, &o, 8, NONE);
    assert_eq!(r.state.anchor, Some(RowKey(8)));
    assert_eq!(
        click_row(&mut h, &o, 6, SHIFT).state.selection,
        keys(&[6, 7, 8])
    );
    assert_eq!(
        click_row(&mut h, &o, 7, SHIFT).state.anchor,
        Some(RowKey(8))
    );
}

#[test]
fn press_then_drag_selects_from_the_anchor_to_the_row_under_the_pointer() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    let (x, y) = over(2);
    h.input.pointer_to(x, y).press(P);
    assert_eq!(run(&mut h, &o).state.selection, keys(&[2]));
    let (_, y) = over(5);
    h.input.pointer_to(x, y);
    assert_eq!(run(&mut h, &o).state.selection, keys(&[2, 3, 4, 5]));
    let (_, y) = over(1);
    h.input.pointer_to(x, y);
    let r = run(&mut h, &o);
    assert_eq!(r.state.selection, keys(&[1, 2]));
    assert_eq!(r.state.anchor, Some(RowKey(2)));
    // past the viewport's top the range stops at the first visible row
    h.input.pointer_to(x, -30.0);
    assert_eq!(run(&mut h, &o).state.selection, keys(&[0, 1, 2]));
    h.input.release(P);
    let r = run(&mut h, &o);
    assert_eq!(
        r.state.selection,
        keys(&[0, 1, 2]),
        "the release changes nothing"
    );
}

#[test]
fn a_drag_in_a_single_selection_list_selects_no_range() {
    let mut o = opts(100);
    o.selection = SelectionMode::Single;
    let mut h = harness();
    run(&mut h, &o);
    let (x, y) = over(2);
    h.input.pointer_to(x, y).press(P);
    run(&mut h, &o);
    let (_, y) = over(5);
    h.input.pointer_to(x, y);
    assert_eq!(run(&mut h, &o).state.selection, keys(&[2]));
}

#[test]
fn single_mode_ignores_the_modifiers_and_none_mode_selects_nothing() {
    let mut o = opts(100);
    o.selection = SelectionMode::Single;
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    assert_eq!(click_row(&mut h, &o, 5, META).state.selection, keys(&[5]));
    assert_eq!(click_row(&mut h, &o, 7, SHIFT).state.selection, keys(&[7]));

    o.selection = SelectionMode::None;
    let mut h = harness();
    run(&mut h, &o);
    let r = click_row(&mut h, &o, 2, NONE);
    assert!(r.state.selection.is_empty());
    assert!(!r.r.response.changed);
    assert_eq!(r.state.cursor, Some(RowKey(2)));
}

#[test]
fn a_press_on_a_child_widget_is_not_a_row_press() {
    let o = opts(100);
    let mut h = harness();
    let mut button = |ui: &mut Ui<'_>, c: &RowCtx| {
        let r = Rect::new(c.rect.x + 200.0, c.rect.y + 5.0, 60.0, 30.0);
        ui.button("b", r, "x", ButtonKind::Secondary);
        ROW
    };
    run_with(&mut h, &o, &key, &mut button);
    h.input.pointer_to(240.0, over(3).1).click(P);
    let r = run_with(&mut h, &o, &key, &mut button);
    assert!(r.state.selection.is_empty());
    assert_eq!(r.state.anchor, None);
    assert_eq!(r.out.focused, Some(row_id(3).with("b")));
    // the row around it still takes a press
    h.input.pointer_to(100.0, over(3).1).click(P);
    let r = run_with(&mut h, &o, &key, &mut button);
    assert_eq!(r.state.selection, keys(&[3]));
}

#[test]
fn a_secondary_click_reports_the_row_and_leaves_the_selection() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    let (x, y) = over(4);
    h.input.pointer_to(x, y).click(PointerButton::Secondary);
    let r = run(&mut h, &o);
    assert_eq!(r.r.context, Some(RowKey(4)));
    assert_eq!(r.state.selection, keys(&[2]));
    assert_eq!(run(&mut h, &o).r.context, None);
}

// The keys.

fn focused(h: &mut UiHarness, o: &ListOptions<'_>) {
    run(h, o);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    run(h, o);
}

#[test]
fn up_and_down_move_the_cursor_and_the_selection() {
    let o = opts(100);
    let mut h = harness();
    focused(&mut h, &o);
    // no cursor yet: the first press lands on the first visible row
    let r = press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    assert_eq!(r.state.selection, keys(&[0]));
    let r = press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    assert_eq!(r.state.selection, keys(&[1]));
    assert_eq!(r.state.cursor, Some(RowKey(1)));
    assert!(r.r.response.changed);
    let r = press_key(&mut h, &o, NamedKey::ArrowUp, NONE);
    assert_eq!(r.state.selection, keys(&[0]));
    let r = press_key(&mut h, &o, NamedKey::ArrowUp, NONE);
    assert_eq!(r.state.selection, keys(&[0]), "the first row is the end");
    assert!(!r.r.response.changed);
}

#[test]
fn shift_extends_from_the_anchor() {
    let o = opts(100);
    let mut h = harness();
    focused(&mut h, &o);
    click_row(&mut h, &o, 3, NONE);
    run(&mut h, &o);
    press_key(&mut h, &o, NamedKey::ArrowDown, SHIFT);
    let r = press_key(&mut h, &o, NamedKey::ArrowDown, SHIFT);
    assert_eq!(r.state.selection, keys(&[3, 4, 5]));
    assert_eq!(r.state.anchor, Some(RowKey(3)));
    assert_eq!(r.state.cursor, Some(RowKey(5)));
    press_key(&mut h, &o, NamedKey::ArrowUp, SHIFT);
    press_key(&mut h, &o, NamedKey::ArrowUp, SHIFT);
    let r = press_key(&mut h, &o, NamedKey::ArrowUp, SHIFT);
    assert_eq!(r.state.selection, keys(&[2, 3]));
}

#[test]
fn page_keys_move_by_a_viewport_and_home_and_end_go_to_the_ends() {
    let o = opts(100);
    let mut h = harness();
    focused(&mut h, &o);
    press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    let r = press_key(&mut h, &o, NamedKey::PageDown, NONE);
    assert_eq!(r.state.cursor, Some(RowKey(8)), "floor(400 / 45) rows");
    let r = press_key(&mut h, &o, NamedKey::PageUp, NONE);
    assert_eq!(r.state.cursor, Some(RowKey(0)));
    let r = press_key(&mut h, &o, NamedKey::End, NONE);
    assert_eq!(r.state.selection, keys(&[99]));
    h.at_ms(100);
    let r = run(&mut h, &o);
    assert_eq!(r.r.offset, 4500.0 - 400.0);
    let r = press_key(&mut h, &o, NamedKey::Home, SHIFT);
    assert_eq!(r.state.selection.len(), 100);
    h.at_ms(200);
    assert_eq!(run(&mut h, &o).r.offset, 0.0);
}

#[test]
fn a_row_taller_than_the_viewport_still_pages_on() {
    let mut o = opts(10);
    o.heights = Heights::Fixed(900.0);
    let mut h = harness();
    focused(&mut h, &o);
    press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    let r = press_key(&mut h, &o, NamedKey::PageDown, NONE);
    assert_eq!(r.state.cursor, Some(RowKey(1)));
    let r = press_key(&mut h, &o, NamedKey::PageUp, NONE);
    assert_eq!(r.state.cursor, Some(RowKey(0)));
}

#[test]
fn the_moved_row_is_revealed_with_the_least_movement() {
    let o = opts(100);
    let mut h = harness();
    focused(&mut h, &o);
    let mut ms = 0;
    press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    for _ in 0..8 {
        press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    }
    // row 8 ends at 405, 5 px past the viewport
    let r = settle(&mut h, &o, &mut ms);
    assert_eq!(r.state.cursor, Some(RowKey(8)));
    assert_eq!(r.r.offset, 5.0);
    // inside the viewport nothing moves
    press_key(&mut h, &o, NamedKey::ArrowUp, NONE);
    let r = settle(&mut h, &o, &mut ms);
    assert_eq!(r.r.offset, 5.0);
    // row 0 is above it
    for _ in 0..7 {
        press_key(&mut h, &o, NamedKey::ArrowUp, NONE);
    }
    let r = settle(&mut h, &o, &mut ms);
    assert_eq!(r.state.cursor, Some(RowKey(0)));
    assert_eq!(r.r.offset, 0.0);
}

#[test]
fn the_primary_modifier_with_c_reports_the_keys_in_key_order() {
    let o = opts(100);
    let mut h = harness();
    focused(&mut h, &o);
    click_row(&mut h, &o, 7, NONE);
    click_row(&mut h, &o, 2, META);
    click_row(&mut h, &o, 5, META);
    run(&mut h, &o);
    h.input
        .set_modifiers(META)
        .key_down(Key::Character("c".into()), Some("c"));
    let r = run(&mut h, &o);
    h.input.set_modifiers(NONE);
    assert_eq!(r.r.copy, Some(vec![RowKey(2), RowKey(5), RowKey(7)]));
    assert_eq!(run(&mut h, &o).r.copy, None);
    // a plain c is not a copy
    h.input.key_down(Key::Character("c".into()), Some("c"));
    assert_eq!(run(&mut h, &o).r.copy, None);
}

#[test]
fn copy_with_nothing_selected_reports_nothing() {
    let o = opts(100);
    let mut h = harness();
    focused(&mut h, &o);
    h.input
        .set_modifiers(META)
        .key_down(Key::Character("c".into()), None);
    assert_eq!(run(&mut h, &o).r.copy, None);
}

#[test]
fn keys_only_reach_the_focused_list_and_an_empty_list_declines_them() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    let r = press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    assert!(r.state.cursor.is_none(), "not focused");

    let o = opts(0);
    let mut h = harness();
    focused(&mut h, &o);
    let r = press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    assert!(r.state.cursor.is_none());
    assert!(r.rows.is_empty());
}

// Focus.

#[test]
fn the_list_is_one_tab_stop_however_many_rows_it_builds() {
    let o = opts(100);
    let mut h = harness();
    let frame = |h: &mut UiHarness| {
        let (_, out) = h.frame(|ui| {
            ui.button(
                "before",
                Rect::new(0.0, 500.0, 50.0, 30.0),
                "b",
                ButtonKind::Secondary,
            );
            ui.virtual_list("l", LIST, &o, key, |_, _| ROW);
            ui.button(
                "after",
                Rect::new(60.0, 500.0, 50.0, 30.0),
                "a",
                ButtonKind::Secondary,
            );
        });
        out.focused
    };
    frame(&mut h);
    let mut order = Vec::new();
    for _ in 0..4 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        order.push(frame(&mut h).expect("a stop"));
    }
    let (b, a) = (WidgetId::ROOT.with("before"), WidgetId::ROOT.with("after"));
    assert_eq!(order, [b, list_id(), a, b]);
}

#[test]
fn a_row_press_focuses_the_list_without_the_ring() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    let r = run(&mut h, &o);
    assert_eq!(r.out.focused, Some(list_id()));
    assert!(r.rows.iter().all(|c| !c.focus_visible));
    assert!(r.rows[2].cursor);
    // and then the keys work
    let r = press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    assert_eq!(r.state.selection, keys(&[3]));
}

#[test]
fn a_press_on_the_empty_part_focuses_the_list_and_keeps_the_selection() {
    let o = opts(3);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 1, NONE);
    h.input.pointer_to(100.0, 300.0).click(P);
    run(&mut h, &o);
    let r = run(&mut h, &o);
    assert_eq!(r.out.focused, Some(list_id()));
    assert_eq!(r.state.selection, keys(&[1]));
}

#[test]
fn row_ids_derive_from_the_key_not_the_index() {
    let o = opts(100);
    let reversed = |i: usize| RowKey(1000 - i as u64);
    let mut h = harness();
    run_with(&mut h, &o, &reversed, &mut |_, _| ROW);
    let slot = h.store.last_rect(row_id(1000)).expect("key 1000 at row 0");
    assert_eq!(slot.y, LIST.y);
    let slot = h.store.last_rect(row_id(997)).expect("key 997 at row 3");
    assert_eq!(slot.y, LIST.y + 3.0 * ROW);
    assert!(h.store.last_rect(row_id(3)).is_none());
}

#[test]
fn widgets_in_two_rows_never_share_an_id() {
    let o = opts(100);
    let mut h = harness();
    let mut ids = BTreeSet::new();
    run_with(&mut h, &o, &key, &mut |ui, _| {
        assert!(ids.insert(ui.id("same-key")));
        ROW
    });
    assert!(ids.len() > 5);
}

// The look.

fn under(s: &Scene) -> &[Shape] {
    &s.layers[0].geometry.under
}

fn premultiplied(c: [u8; 4]) -> [f32; 4] {
    let a = f32::from(c[3]) / 255.0;
    [
        f32::from(c[0]) / 255.0 * a,
        f32::from(c[1]) / 255.0 * a,
        f32::from(c[2]) / 255.0 * a,
        a,
    ]
}

#[test]
fn at_scale_2_the_slot_is_90_px_and_the_highlight_inset_8_and_2() {
    let o = opts(100);
    let mut h = harness();
    h.input.set_scale(2.0);
    run(&mut h, &o);
    h.input.set_scale(2.0);
    let r = click_row(&mut h, &o, 2, NONE);
    let mut ms = 0;
    settle(&mut h, &o, &mut ms);
    h.input.pointer_leave();
    let r2 = settle(&mut h, &o, &mut ms);
    assert_eq!(r.rows[1].rect.y + ROW, r.rows[2].rect.y);
    let sh = under(&r2.scene)
        .iter()
        .find(|s| s.rect[3] < 100.0 && s.params[0] == 0.0)
        .expect("the row's highlight");
    // slot: x 40, y (20 + 90) * 2 ... in device px, 600 wide and 90 tall
    let slot = [40.0, (LIST.y + 2.0 * ROW) * 2.0, 600.0, 90.0];
    assert_eq!(sh.rect[0], slot[0] + 8.0);
    assert_eq!(sh.rect[1], slot[1] + 2.0);
    assert_eq!(sh.rect[2], slot[2] - 16.0);
    assert_eq!(sh.rect[3], slot[3] - 4.0);
    assert_eq!(sh.radii, [DARK.item_radius * 2.0; 4]);
    for (got, want) in sh.color.iter().zip(premultiplied(DARK.surface_selected)) {
        assert!((got - want).abs() < 1e-6);
    }
}

#[test]
fn the_fill_follows_hover_and_selection_and_rest_is_transparent() {
    assert_eq!(row_fill(&DARK, false, false)[3], 0);
    assert_eq!(row_fill(&DARK, true, false), DARK.surface_hover);
    assert_eq!(row_fill(&DARK, false, true), DARK.surface_selected);
    assert_eq!(row_fill(&DARK, true, true), DARK.surface_active);

    let o = opts(100);
    let mut h = harness();
    let mut ms = 0;
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    let (x, y) = over(4);
    h.input.pointer_to(x, y);
    let r = settle(&mut h, &o, &mut ms);
    assert_eq!(r.rows[4].fill, DARK.surface_hover);
    assert!(r.rows[4].hovered);
    assert_eq!(r.rows[2].fill, DARK.surface_selected);
    let (x, y) = over(2);
    h.input.pointer_to(x, y);
    let r = settle(&mut h, &o, &mut ms);
    assert_eq!(r.rows[2].fill, DARK.surface_active);
    assert_eq!(r.rows[0].fill[3], 0);
    // a rest row emits no highlight at all
    assert_eq!(
        under(&r.scene)
            .iter()
            .filter(|s| s.rect[2] == 292.0)
            .count(),
        1,
    );
}

#[test]
fn the_outline_sits_on_the_cursor_row_after_keys_only() {
    let o = opts(100);
    let mut h = harness();
    focused(&mut h, &o);
    let mut ms = 0;
    for _ in 0..5 {
        press_key(&mut h, &o, NamedKey::ArrowDown, NONE);
    }
    let r = settle(&mut h, &o, &mut ms);
    assert!(r.rows[4].focus_visible && r.rows[4].cursor);
    assert_eq!(r.rows.iter().filter(|c| c.focus_visible).count(), 1);
    let rings = under(&r.scene)
        .iter()
        .filter(|s| s.params[0] == 1.0)
        .count();
    assert_eq!(rings, 1);
    let ring = under(&r.scene)
        .iter()
        .find(|s| s.params[0] == 1.0)
        .expect("ring");
    assert_eq!(ring.rect[1], LIST.y + 4.0 * ROW + 1.0);
}

#[test]
fn tab_with_no_cursor_shows_the_outline_on_the_first_visible_row() {
    let o = opts(100);
    let mut h = harness();
    focused(&mut h, &o);
    let r = run(&mut h, &o);
    assert!(r.rows[0].focus_visible);
    assert_eq!(r.state.cursor, None);
}

#[test]
fn a_disabled_list_is_inert_and_draws_at_0_4() {
    let o = opts(100);
    let mut h = harness();
    let frame = |h: &mut UiHarness| {
        let mut count = 0;
        let (s, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                ui.virtual_list("l", LIST, &o, key, |ui, c| {
                    count += 1;
                    ui.painter().fill(c.rect, 0.0, [255, 0, 0, 255]);
                    ROW
                });
            });
        });
        (s, out, count)
    };
    frame(&mut h);
    let (x, y) = over(2);
    h.input.pointer_to(x, y).click(P);
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    h.input.scroll_pixels(0.0, -100.0);
    let (s, out, count) = frame(&mut h);
    assert!(count > 5);
    let n = out.tree.get(list_id()).expect("list node");
    assert!(n.states.contains(State::Disabled));
    let item = out.tree.get(row_id(2)).expect("row");
    assert!(item.states.contains(State::Disabled));
    assert!(!item.states.contains(State::Selected));
    assert!(!item.actions.contains(Action::Default));
    let red = under(&s)
        .iter()
        .find(|sh| sh.color[0] > 0.0)
        .expect("a row");
    assert!((red.color[3] - 0.4).abs() < 1e-6);
    assert_eq!(red.rect[1], LIST.y, "the wheel moved nothing");
    h.frame(|ui| {
        assert!(ui.list_state("l").selection.is_empty());
    });
}

#[test]
fn an_empty_list_builds_nothing_and_settles() {
    let o = opts(0);
    let mut h = harness();
    let r = run(&mut h, &o);
    assert!(r.rows.is_empty());
    assert_eq!(r.r.extent, 0.0);
    let n = r.out.tree.get(list_id()).expect("node");
    assert!(n.children.is_empty());
    assert_eq!(r.out.repaint, Repaint::Idle);
}

// Motion.

#[test]
fn the_two_transitions_conform_at_every_tier() {
    let t = DARK.motion;
    for tier in [MotionTier::Full, MotionTier::Reduced] {
        for m in [HOVER, SELECTION] {
            let (from, to) = ([1.0, 1.0, 1.0, 0.0f32], [1.0, 1.0, 1.0, 0.25f32]);
            conformance(&m, &t, tier, from, to).expect("conforms");
        }
    }
}

#[test]
fn a_still_list_tweens_hover_over_instant_and_selection_over_fast() {
    let o = opts(100);
    let mut h = harness();
    let mut ms = 0;
    run(&mut h, &o);
    let r = settle(&mut h, &o, &mut ms);
    assert_eq!(r.rows[3].fill[3], 0);
    // hover: halfway through `instant` (120 ms)
    let (x, y) = over(3);
    h.input.pointer_to(x, y);
    ms += 400;
    h.at_ms(ms);
    run(&mut h, &o);
    h.at_ms(ms + 60);
    let r = run(&mut h, &o);
    let half = u32::from(DARK.surface_hover[3]) / 2;
    assert!(
        u32::from(r.rows[3].fill[3]).abs_diff(half) <= 1,
        "{:?}",
        r.rows[3].fill
    );
    assert_eq!(r.out.repaint, Repaint::Now);

    // selection: halfway through `fast` (180 ms), from the hover fill to the active fill
    let r = settle(&mut h, &o, &mut ms);
    assert_eq!(r.rows[3].fill, DARK.surface_hover);
    h.input.click(P);
    ms += 400;
    h.at_ms(ms);
    run(&mut h, &o);
    h.at_ms(ms + 90);
    let r = run(&mut h, &o);
    let mid = (u32::from(DARK.surface_hover[3]) + u32::from(DARK.surface_active[3])) / 2;
    assert!(
        u32::from(r.rows[3].fill[3]).abs_diff(mid) <= 1,
        "{:?}",
        r.rows[3].fill
    );
    let r = settle(&mut h, &o, &mut ms);
    assert_eq!(r.rows[3].fill, DARK.surface_active);
}

#[test]
fn the_reduced_tier_keeps_the_fades() {
    let o = opts(100);
    let mut h = harness();
    h.tier = MotionTier::Reduced;
    run(&mut h, &o);
    click_row(&mut h, &o, 1, NONE);
    h.at_ms(90);
    let r = run(&mut h, &o);
    assert_ne!(r.rows[1].fill, DARK.surface_active);
}

/// The animated values the overlay thumb of a pointed-at area keeps: its reveal and its colour.
const THUMB: usize = 2;

/// The store's animated values, in flight or not, from its debug form.
fn animations(h: &UiHarness) -> usize {
    let s = format!("{:?}", h.store);
    let (_, rest) = s.split_once("animations: ").expect("the field");
    rest.split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|n| n.parse().ok())
        .expect("a count")
}

#[test]
fn a_scripted_scroll_over_600_frames_starts_no_row_transition() {
    let o = opts(MILLION);
    let mut h = harness();
    let mut ms = 0;
    let (x, y) = over(2);
    h.input.pointer_to(x, y);
    run(&mut h, &o);
    click_row(&mut h, &o, 3, NONE);
    let settled = [
        row_fill(&DARK, false, false),
        row_fill(&DARK, true, false),
        row_fill(&DARK, false, true),
        row_fill(&DARK, true, true),
    ];
    settle(&mut h, &o, &mut ms);
    let (x, y) = over(2);
    h.input.pointer_to(x, y);
    settle(&mut h, &o, &mut ms);
    let mut hovers = BTreeSet::new();
    for frame in 0..600u64 {
        // wheel steps, then trackpad deltas, under a still pointer
        if frame % 2 == 0 {
            h.input.scroll_lines(0.0, -1.0);
        } else {
            h.input.scroll_pixels(0.0, -7.0);
        }
        ms += 16;
        h.at_ms(ms);
        let r = run(&mut h, &o);
        // The wheel's first frame has not moved the offset yet; rows keep their settled entries.
        if frame > 0 {
            assert_eq!(animations(&h), THUMB, "frame {frame}");
        }
        for c in &r.rows {
            assert!(settled.contains(&c.fill), "frame {frame}: {:?}", c.fill);
            if c.hovered {
                hovers.insert(c.key);
            }
        }
    }
    assert!(hovers.len() > 100, "rows did pass under the pointer");
    let r = settle(&mut h, &o, &mut ms);
    assert!(r.rows.iter().filter(|c| c.hovered).count() == 1);
}

#[test]
fn the_end_of_a_scroll_snaps_the_row_left_under_the_pointer() {
    let o = opts(100);
    let mut h = harness();
    let (x, y) = over(2);
    h.input.pointer_to(x, y);
    run(&mut h, &o);
    let mut ms = 0;
    settle(&mut h, &o, &mut ms);
    h.input.scroll_pixels(0.0, -90.0);
    ms += 16;
    for _ in 0..4 {
        ms += 16;
        h.at_ms(ms);
        let r = run(&mut h, &o);
        for c in &r.rows {
            let want = row_fill(&DARK, c.hovered, c.selected);
            assert_eq!(c.fill, want);
        }
    }
}

#[test]
fn a_fade_in_flight_snaps_when_the_list_starts_to_move() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    let mut ms = 0;
    settle(&mut h, &o, &mut ms);
    click_row(&mut h, &o, 3, NONE);
    h.at_ms(ms + 30);
    let r = run(&mut h, &o);
    assert_ne!(r.rows[3].fill, row_fill(&DARK, true, true));
    assert!(animations(&h) > THUMB);
    h.input.scroll_pixels(0.0, -20.0);
    h.at_ms(ms + 46);
    run(&mut h, &o);
    h.at_ms(ms + 62);
    let r = run(&mut h, &o);
    assert_eq!(animations(&h), THUMB);
    let row = r.rows.iter().find(|c| c.key == RowKey(3)).expect("row");
    assert_eq!(row.fill, row_fill(&DARK, row.hovered, true));
}

// Measured rows.

fn measured() -> ListOptions<'static> {
    let mut o = opts(1000);
    o.heights = Heights::Measured { estimate: 30.0 };
    o
}

#[test]
fn a_row_above_the_viewport_measuring_taller_leaves_the_first_visible_row_in_place() {
    let o = measured();
    let mut h = harness();
    let tall = Cell::new(30.0);
    let mut draw = |_: &mut Ui<'_>, c: &RowCtx| if c.index == 19 { tall.get() } else { 30.0 };
    run_with(&mut h, &o, &key, &mut draw);
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -610.0);
    h.at_ms(100);
    run_with(&mut h, &o, &key, &mut draw);
    h.at_ms(200);
    let before = run_with(&mut h, &o, &key, &mut draw);
    assert_eq!(before.r.offset, 610.0);
    let y_of = |r: &Run, i: usize| r.rows.iter().find(|c| c.index == i).expect("built").rect.y;
    assert!(before.rows.iter().any(|c| c.index == 19), "a margin row");
    let first = 20;
    let y = y_of(&before, first);
    assert_eq!(y, LIST.y - 10.0);

    tall.set(100.0);
    h.at_ms(300);
    let changed = run_with(&mut h, &o, &key, &mut draw);
    assert_eq!(changed.out.repaint, Repaint::Now, "one catch-up frame");
    h.at_ms(316);
    let next = run_with(&mut h, &o, &key, &mut draw);
    assert_eq!(next.r.offset, 680.0);
    assert_eq!(y_of(&next, first), y);
    assert_eq!(next.r.extent, 1000.0 * 30.0 + 70.0);
    assert_eq!(next.out.repaint, Repaint::Idle);
}

#[test]
fn a_row_below_the_first_visible_one_measuring_taller_moves_the_offset_not_at_all() {
    let o = measured();
    let mut h = harness();
    let tall = Cell::new(30.0);
    let mut draw = |_: &mut Ui<'_>, c: &RowCtx| if c.index == 25 { tall.get() } else { 30.0 };
    run_with(&mut h, &o, &key, &mut draw);
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -610.0);
    h.at_ms(100);
    run_with(&mut h, &o, &key, &mut draw);
    h.at_ms(200);
    run_with(&mut h, &o, &key, &mut draw);
    tall.set(100.0);
    h.at_ms(300);
    run_with(&mut h, &o, &key, &mut draw);
    h.at_ms(316);
    let r = run_with(&mut h, &o, &key, &mut draw);
    assert_eq!(r.r.offset, 610.0);
    let row = r.rows.iter().find(|c| c.index == 26).expect("built");
    assert_eq!(row.rect.y, LIST.y + 26.0 * 30.0 + 70.0 - 610.0);
    assert_eq!(r.out.repaint, Repaint::Idle);
}

#[test]
fn a_first_measure_settles_to_idle_and_rows_stack_on_their_measured_heights() {
    let o = measured();
    let mut h = harness();
    let height = |i: usize| 40.0 + (i % 3) as f32 * 10.0;
    let mut draw = |_: &mut Ui<'_>, c: &RowCtx| height(c.index);
    let mut frames = 0;
    let r = loop {
        frames += 1;
        h.at_ms(frames * 16);
        let r = run_with(&mut h, &o, &key, &mut draw);
        if r.out.repaint == Repaint::Idle {
            break r;
        }
        assert!(frames < 6, "the measuring settles");
    };
    let mut y = LIST.y;
    for c in &r.rows {
        assert_eq!(c.rect.y, y);
        assert_eq!(c.rect.h, height(c.index));
        y += c.rect.h;
    }
}

#[test]
fn scrolling_up_through_unmeasured_rows_keeps_the_view_anchored() {
    let o = measured();
    let mut h = harness();
    // rows are 50 px, 20 over the estimate; jump far down, then walk back up
    let mut draw = |_: &mut Ui<'_>, _: &RowCtx| 50.0;
    run_with(&mut h, &o, &key, &mut draw);
    h.input
        .pointer_to(100.0, 100.0)
        .scroll_pixels(0.0, -15_000.0);
    let mut ms = 16;
    h.at_ms(ms);
    run_with(&mut h, &o, &key, &mut draw);
    for _ in 0..4 {
        ms += 16;
        h.at_ms(ms);
        run_with(&mut h, &o, &key, &mut draw);
    }
    let y_of = |r: &Run, k: u64| r.rows.iter().find(|c| c.key == RowKey(k)).map(|c| c.rect.y);
    for _ in 0..40 {
        ms += 16;
        h.at_ms(ms);
        h.input.scroll_pixels(0.0, 11.0);
        let a = run_with(&mut h, &o, &key, &mut draw);
        let top_key = a.rows[MARGIN + 1].key;
        let y = y_of(&a, top_key.0).expect("built");
        ms += 16;
        h.at_ms(ms);
        let b = run_with(&mut h, &o, &key, &mut draw);
        assert_eq!(y_of(&b, top_key.0), Some(y), "the view does not jump");
    }
}

#[test]
fn an_animated_wheel_step_survives_a_shift() {
    let o = measured();
    let mut h = harness();
    let tall = Cell::new(30.0);
    let mut draw = |_: &mut Ui<'_>, c: &RowCtx| if c.index == 19 { tall.get() } else { 30.0 };
    run_with(&mut h, &o, &key, &mut draw);
    h.input.pointer_to(100.0, 100.0).scroll_pixels(0.0, -610.0);
    h.at_ms(100);
    run_with(&mut h, &o, &key, &mut draw);
    h.at_ms(200);
    run_with(&mut h, &o, &key, &mut draw);
    h.input.scroll_lines(0.0, -1.0);
    tall.set(100.0);
    h.at_ms(300);
    run_with(&mut h, &o, &key, &mut draw);
    let mut ms = 300;
    let mut prev = 0.0;
    for _ in 0..20 {
        ms += 16;
        h.at_ms(ms);
        let r = run_with(&mut h, &o, &key, &mut draw);
        assert!(r.r.offset >= prev);
        prev = r.r.offset;
    }
    assert_eq!(prev, 610.0 + 70.0 + 48.0);
}

#[test]
fn appended_rows_keep_the_measured_ones_and_the_offset() {
    let mut o = measured();
    let mut h = harness();
    let mut draw = |_: &mut Ui<'_>, _: &RowCtx| 50.0;
    run_with(&mut h, &o, &key, &mut draw);
    h.at_ms(16);
    let before = run_with(&mut h, &o, &key, &mut draw);
    o.count = 1500;
    h.at_ms(32);
    let after = run_with(&mut h, &o, &key, &mut draw);
    assert_eq!(after.r.extent - before.r.extent, 500.0 * 30.0);
    assert_eq!(after.r.offset, before.r.offset);
}

// The node.

#[test]
fn the_tree_has_the_list_a_list_item_per_built_row_and_selected_follows_the_set() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    let r = click_row(&mut h, &o, 2, NONE);
    let r2 = run(&mut h, &o);
    let list = r2.out.tree.get(list_id()).expect("list");
    assert_eq!(list.role, Role::List);
    assert_eq!(list.name, "rows");
    assert!(list.states.contains(State::Focusable));
    assert_eq!(list.children.len(), r2.rows.len());
    for c in &r2.rows {
        let n = r2.out.tree.get(row_id(c.key.0)).expect("item");
        assert_eq!(n.role, Role::ListItem);
        assert_eq!(n.parent, Some(list_id()));
        assert_eq!(n.states.contains(State::Selected), c.key == RowKey(2));
        assert_eq!(n.bounds, c.rect);
        for a in [
            Action::Default,
            Action::Focus,
            Action::ScrollIntoView,
            Action::ShowMenu,
        ] {
            assert!(n.actions.contains(a));
        }
    }
    assert_eq!(
        r.out
            .tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::ListItem)
            .count(),
        r.rows.len(),
    );
    let area = r2
        .out
        .tree
        .get(list.parent.expect("area"))
        .expect("area node");
    assert_eq!(area.role, Role::ScrollArea);
}

#[test]
fn a_rows_own_nodes_nest_under_its_item() {
    let o = opts(100);
    let mut h = harness();
    let mut draw = |ui: &mut Ui<'_>, c: &RowCtx| {
        let id = ui.id("text");
        ui.node(lune_ui_core::access::Node::new(
            id,
            Role::StaticText,
            "row text",
            c.rect,
        ));
        ROW
    };
    let r = run_with(&mut h, &o, &key, &mut draw);
    let item = r.out.tree.get(row_id(0)).expect("item");
    assert_eq!(item.children, [row_id(0).with("text")]);
}

fn access(h: &mut UiHarness, target: WidgetId, action: Action) {
    h.input.access(AccessAction {
        target,
        action,
        value: None,
    });
}

#[test]
fn an_access_default_action_selects_the_row() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    access(&mut h, row_id(4), Action::Default);
    let r = run(&mut h, &o);
    assert_eq!(r.state.selection, keys(&[4]));
    assert_eq!(r.state.anchor, Some(RowKey(4)));
    let n = run(&mut h, &o);
    assert!(
        n.out
            .tree
            .get(row_id(4))
            .expect("item")
            .states
            .contains(State::Selected)
    );
}

#[test]
fn a_real_click_is_not_applied_twice_over_a_modifier() {
    // the press toggles on one frame, the release on a later one must not reselect
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    let (x, y) = over(5);
    h.input.set_modifiers(META).pointer_to(x, y).press(P);
    run(&mut h, &o);
    h.input.release(P).set_modifiers(NONE);
    assert_eq!(run(&mut h, &o).state.selection, keys(&[2, 5]));
}

#[test]
fn access_focus_lands_on_the_list_with_the_cursor_on_the_row() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    access(&mut h, row_id(4), Action::Focus);
    run(&mut h, &o);
    let r = run(&mut h, &o);
    assert_eq!(r.out.focused, Some(list_id()));
    assert_eq!(r.state.cursor, Some(RowKey(4)));
}

#[test]
fn access_show_menu_reports_the_row_and_scroll_into_view_centres_it() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    access(&mut h, row_id(10), Action::ShowMenu);
    assert_eq!(run(&mut h, &o).r.context, Some(RowKey(10)));
    access(&mut h, row_id(9), Action::ScrollIntoView);
    run(&mut h, &o);
    let mut ms = 0;
    let r = settle(&mut h, &o, &mut ms);
    assert_eq!(r.r.offset, 9.0 * 45.0 - (400.0 - 45.0) / 2.0);
}

// Retention.

#[test]
fn a_selected_row_scrolled_out_and_back_stays_selected() {
    let o = opts(MILLION);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    h.input
        .pointer_to(100.0, 100.0)
        .scroll_pixels(0.0, -1_000_000.0);
    h.at_ms(100);
    run(&mut h, &o);
    h.at_ms(200);
    let away = run(&mut h, &o);
    assert!(away.rows.iter().all(|c| !c.selected && c.index > 1000));
    assert_eq!(away.state.selection, keys(&[2]));
    h.input.scroll_pixels(0.0, 1_000_000.0);
    h.at_ms(300);
    run(&mut h, &o);
    h.at_ms(400);
    let back = run(&mut h, &o);
    assert_eq!(back.r.offset, 0.0);
    assert!(back.rows[2].selected);
    assert_eq!(back.rows[2].fill[3], DARK.surface_selected[3]);
}

#[test]
fn six_hundred_undrawn_frames_under_keep_alive_keep_selection_and_offset() {
    let o = opts(1000);
    let mut h = harness();
    let frame = |h: &mut UiHarness| {
        let mut r = None;
        h.frame(|ui| {
            ui.keep_alive("tab", |ui| {
                r = Some(ui.virtual_list("l", LIST, &o, key, |_, _| ROW));
            });
        });
        r.expect("drew")
    };
    frame(&mut h);
    let (x, y) = over(3);
    h.input.pointer_to(x, y).click(P);
    frame(&mut h);
    h.input.scroll_pixels(0.0, -500.0);
    h.at_ms(100);
    frame(&mut h);
    h.at_ms(200);
    assert_eq!(frame(&mut h).offset, 500.0);
    for ms in 0..600 {
        h.at_ms(300 + ms);
        h.frame(|ui| ui.mark_alive("tab"));
    }
    h.at_ms(1000);
    assert_eq!(frame(&mut h).offset, 500.0);
    h.frame(|ui| {
        ui.keep_alive("tab", |ui| {
            assert_eq!(ui.list_state("l").selection, keys(&[3]));
            assert_eq!(ui.list_state("l").anchor, Some(RowKey(3)));
        });
    });
}

#[test]
fn without_keep_alive_the_state_goes_after_one_undrawn_frame() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 3, NONE);
    h.frame(|_| {});
    assert!(run(&mut h, &o).state.selection.is_empty());
}

#[test]
fn retain_prunes_dropped_keys_with_the_anchor_and_the_cursor() {
    let mut st = ListState::default();
    st.selection = keys(&[1, 2, 3]);
    st.anchor = Some(RowKey(2));
    st.cursor = Some(RowKey(3));
    st.retain(|k| k != RowKey(2));
    assert_eq!(st.selection, keys(&[1, 3]));
    assert_eq!(st.anchor, None);
    assert_eq!(st.cursor, Some(RowKey(3)));
}

#[test]
fn the_app_can_read_and_prune_the_state_from_inside_a_row() {
    let o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 1, NONE);
    let (_, _) = h.frame(|ui| {
        ui.virtual_list("l", LIST, &o, key, |ui, c| {
            let id = ui.scope_id();
            assert_eq!(id, row_id(c.key.0));
            ROW
        });
        ui.list_state("l").retain(|k| k != RowKey(1));
    });
    assert!(run(&mut h, &o).state.selection.is_empty());
}

// Stale anchors: the data changed under the key.

#[test]
fn a_stale_anchor_without_index_of_falls_back_to_a_single_row() {
    let o = opts(100);
    let shifted = |i: usize| RowKey(i as u64 + 1000);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    run_with(&mut h, &o, &shifted, &mut |_, _| ROW);
    let (x, y) = over(6);
    h.input.pointer_to(x, y).set_modifiers(SHIFT).click(P);
    let r = run_with(&mut h, &o, &shifted, &mut |_, _| ROW);
    h.input.set_modifiers(NONE);
    assert_eq!(r.state.selection, keys(&[1006]));
    assert_eq!(r.state.anchor, Some(RowKey(1006)));
}

#[test]
fn index_of_finds_a_moved_anchor() {
    let mut o = opts(100);
    let find = |k: RowKey| k.0.checked_sub(1000).map(|i| i as usize);
    let shifted = |i: usize| RowKey(i as u64 + 1001);
    let mut h = harness();
    let plain = |i: usize| RowKey(i as u64 + 1000);
    run_with(&mut h, &o, &plain, &mut |_, _| ROW);
    let (x, y) = over(2);
    h.input.pointer_to(x, y).click(P);
    run_with(&mut h, &o, &plain, &mut |_, _| ROW);
    // the anchor's key 1002 is at index 1 now
    let moved = |k: RowKey| find(k).map(|i| i - 1);
    o.index_of = Some(&moved);
    run_with(&mut h, &o, &shifted, &mut |_, _| ROW);
    let (x, y) = over(4);
    h.input.pointer_to(x, y).set_modifiers(SHIFT).click(P);
    let r = run_with(&mut h, &o, &shifted, &mut |_, _| ROW);
    h.input.set_modifiers(NONE);
    assert_eq!(r.state.selection, keys(&[1002, 1003, 1004, 1005]));
}

#[test]
fn a_stale_cursor_keeps_its_position_and_the_keys_go_on_from_there() {
    let o = opts(100);
    let shifted = |i: usize| RowKey(i as u64 + 1000);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 2, NONE);
    run(&mut h, &o);
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    let r = run_with(&mut h, &o, &shifted, &mut |_, _| ROW);
    assert_eq!(r.state.cursor, Some(RowKey(1003)));
    assert_eq!(r.state.selection, keys(&[1003]));
}

#[test]
fn rows_removed_under_the_cursor_clamp_it_to_the_last_row() {
    let mut o = opts(100);
    let mut h = harness();
    run(&mut h, &o);
    click_row(&mut h, &o, 8, NONE);
    run(&mut h, &o);
    o.count = 3;
    h.input.key_down(Key::Named(NamedKey::ArrowUp), None);
    let r = run(&mut h, &o);
    // row 8 is gone, so the cursor stands on row 2 and Up lands on row 1
    assert_eq!(r.state.cursor, Some(RowKey(1)));
}

#[test]
fn a_shrunk_list_clamps_its_offset_before_drawing() {
    let mut o = opts(1000);
    let mut h = harness();
    h.input.pointer_to(100.0, 100.0);
    run(&mut h, &o);
    h.input.scroll_pixels(0.0, -40_000.0);
    h.at_ms(100);
    run(&mut h, &o);
    o.count = 20;
    h.at_ms(200);
    let r = run(&mut h, &o);
    assert_eq!(r.r.offset, 20.0 * 45.0 - 400.0);
    assert!(r.rows.last().is_some_and(|c| c.index == 19));
}

#[test]
fn the_list_sits_on_a_scroll_region_so_the_wheel_works_over_a_row() {
    let o = opts(100);
    let mut h = harness();
    let (x, y) = over(1);
    h.input.pointer_to(x, y);
    run(&mut h, &o);
    h.input.scroll_pixels(0.0, -30.0);
    h.at_ms(16);
    assert_eq!(run(&mut h, &o).r.offset, 30.0);
}

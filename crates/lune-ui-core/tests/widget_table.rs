//! The data table (`data.table` acceptance, D162): the column layout, selection and keys through
//! the list frame, the header (sort requests, its roving stop, the keys), cells (tones, mono, num,
//! elision, wrap, controls), the look at scale 2, the tree, the access actions, the disabled and
//! chord rules, and motion.

use std::cell::Cell as Flag;

use lune_ui_core::access::{AccessAction, Action, Role, Sort, State};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::{Scene, Shape, TextItem};
use lune_ui_core::shortcuts::{Binding, Chord, CommandId, Scope};
use lune_ui_core::testing::{FakeText, ScriptedInput, UiHarness};
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{OverlayOptions, Sense, Ui, WidgetId};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::table::{
    ARROW, ARROW_GAP, Cell, CellCtx, CellTone, ColumnWidth, EDGE_PAD, HEADER, HEADER_FADE, PAD,
    ROW, SortKey, TableColumn, TableOptions, TableResponse, TableState, WRAP_PAD, columns,
};
use lune_ui_core::widgets::virtual_list::{Heights, ListOptions, ListState, RowKey, SelectionMode};

const WIN: (f32, f32) = (500.0, 400.0);
/// Header 20 to 48, the body from 48 to 300: nine rows in view.
const TABLE: Rect = Rect::new(20.0, 20.0, 400.0, 280.0);
const P: PointerButton = PointerButton::Primary;

const NAME: u64 = 1;
const STATUS: u64 = 2;
const AMOUNT: u64 = 3;
const NOTE: u64 = 4;

/// Name and Note share what Status (80) and Amount (100) leave of 400: 110 each. Name and Amount
/// sort.
fn cols(sortable: bool) -> [TableColumn<'static>; 4] {
    let col = |key, title, width, num, sort| TableColumn {
        key,
        title,
        width,
        num,
        sortable: sortable && sort,
    };
    [
        col(NAME, "Name", ColumnWidth::Fill(1.0), false, true),
        col(STATUS, "Status", ColumnWidth::Fixed(80.0), false, false),
        col(AMOUNT, "Amount", ColumnWidth::Fixed(100.0), true, true),
        col(NOTE, "Note", ColumnWidth::Fill(1.0), false, false),
    ]
}

/// The columns' places inside the table: x and width.
const PLACE: [(f32, f32); 4] = [(0.0, 110.0), (110.0, 80.0), (190.0, 100.0), (290.0, 110.0)];

#[derive(Clone, Copy)]
struct Setup {
    count: usize,
    wrap: bool,
    enabled: bool,
    sortable: bool,
    /// Row `i` is key `count - 1 - i`: the app sorted descending.
    reverse: bool,
    /// The app clears the sort before the table is drawn.
    clear_sort: bool,
}

fn setup(count: usize) -> Setup {
    Setup {
        count,
        wrap: false,
        enabled: true,
        sortable: true,
        reverse: false,
        clear_sort: false,
    }
}

struct Run {
    r: TableResponse,
    out: FrameOutput,
    scene: Scene,
    cells: Vec<CellCtx>,
    sel: ListState,
    state: TableState,
}

fn plain(_: &mut Ui<'_>, c: &CellCtx) -> Cell {
    match c.column {
        0 => Cell::text(format!("Row {}", c.key.0)),
        1 => Cell::text("ok"),
        2 => Cell::text(format!("{}", c.key.0 * 10)),
        _ => Cell::text(format!("n{}", c.key.0)),
    }
}

fn frame_with(
    h: &mut UiHarness,
    s: &Setup,
    cell: &mut dyn FnMut(&mut Ui<'_>, &CellCtx) -> Cell,
) -> Run {
    let columns = cols(s.sortable);
    let opts = TableOptions {
        columns: &columns,
        count: s.count,
        selection: SelectionMode::Multi,
        wrap: s.wrap,
        index_of: None,
        label: "Requests",
    };
    let (n, reverse) = (s.count, s.reverse);
    let row_key = move |i: usize| {
        RowKey(if reverse {
            (n - 1 - i) as u64
        } else {
            i as u64
        })
    };
    let mut cells = Vec::new();
    let mut r = None;
    let mut sel = ListState::default();
    let mut state = TableState::default();
    let (scene, out) = h.frame(|ui| {
        if s.clear_sort {
            ui.table_state("t").sort = None;
        }
        ui.with_enabled(s.enabled, |ui| {
            r = Some(ui.table("t", TABLE, &opts, row_key, |ui, c| {
                cells.push(*c);
                cell(ui, c)
            }));
        });
        sel = ui.table_selection("t").clone();
        state = *ui.table_state("t");
    });
    Run {
        r: r.expect("the frame ran"),
        out,
        scene,
        cells,
        sel,
        state,
    }
}

fn run(h: &mut UiHarness, s: &Setup) -> Run {
    frame_with(h, s, &mut plain)
}

fn run_off(h: &mut UiHarness, s: &Setup) -> Run {
    run(
        h,
        &Setup {
            enabled: false,
            ..*s
        },
    )
}

fn harness() -> UiHarness {
    UiHarness::new(WIN)
}

fn table_id() -> WidgetId {
    WidgetId::ROOT.with("t")
}

fn head_id() -> WidgetId {
    table_id().with("head")
}

fn header_id(col: u64) -> WidgetId {
    head_id().with(col)
}

fn body_id() -> WidgetId {
    table_id().with("body")
}

fn row_id(k: u64) -> WidgetId {
    body_id().with(RowKey(k))
}

fn cell_id(k: u64, col: u64) -> WidgetId {
    row_id(k).with(col)
}

/// The centre of header cell `i`.
fn over_header(i: usize) -> (f32, f32) {
    (
        TABLE.x + PLACE[i].0 + PLACE[i].1 / 2.0,
        TABLE.y + HEADER / 2.0,
    )
}

/// A point of row `i` (column Name) while the body is at offset 0.
fn over_row(i: usize) -> (f32, f32) {
    (TABLE.x + 40.0, TABLE.y + HEADER + (i as f32 + 0.5) * ROW)
}

fn click(h: &mut UiHarness, (x, y): (f32, f32)) {
    h.input.pointer_to(x, y).click(P);
}

fn press_key(h: &mut UiHarness, s: &Setup, k: NamedKey, m: Modifiers) -> Run {
    h.input.set_modifiers(m).key_down(Key::Named(k), None);
    let r = run(h, s);
    h.input.set_modifiers(Modifiers::default());
    r
}

fn tab(h: &mut UiHarness, s: &Setup) -> Run {
    press_key(h, s, NamedKey::Tab, NONE)
}

const NONE: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: false,
};
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

/// Focuses the header stop, then the body (two Tabs).
fn focus_header(h: &mut UiHarness, s: &Setup) {
    run(h, s);
    tab(h, s);
}

fn focus_body(h: &mut UiHarness, s: &Setup) {
    focus_header(h, s);
    if s.sortable {
        tab(h, s);
    }
}

/// Settles the frame's follow-ups.
fn settle(h: &mut UiHarness, s: &Setup, ms: &mut u64) -> Run {
    loop {
        *ms += 400;
        h.at_ms(*ms);
        let r = run(h, s);
        if r.out.repaint == Repaint::Idle {
            return r;
        }
    }
}

fn keys(s: &[u64]) -> Vec<RowKey> {
    s.iter().map(|k| RowKey(*k)).collect()
}

fn selected(r: &Run) -> Vec<RowKey> {
    r.sel.selection.iter().copied().collect()
}

// The column layout.

fn fixed(w: f32) -> TableColumn<'static> {
    TableColumn {
        key: 0,
        title: "",
        width: ColumnWidth::Fixed(w),
        num: false,
        sortable: false,
    }
}

fn fill(w: f32) -> TableColumn<'static> {
    TableColumn {
        width: ColumnWidth::Fill(w),
        ..fixed(0.0)
    }
}

#[test]
fn fixed_columns_take_their_width_one_after_the_other() {
    let got = columns(&[fixed(10.0), fixed(20.0), fixed(30.0)], 100.0);
    assert_eq!(got, [(0.0, 10.0), (10.0, 20.0), (30.0, 30.0)]);
}

#[test]
fn fill_columns_share_what_the_fixed_ones_leave_by_weight() {
    let got = columns(&[fixed(100.0), fill(1.0), fill(3.0)], 500.0);
    assert_eq!(got, [(0.0, 100.0), (100.0, 100.0), (200.0, 300.0)]);
    let got = columns(&[fill(1.0), fixed(100.0), fill(1.0)], 400.0);
    assert_eq!(got, [(0.0, 150.0), (150.0, 100.0), (250.0, 150.0)]);
}

#[test]
fn no_width_gives_every_column_none() {
    let got = columns(&[fixed(10.0), fill(1.0)], 0.0);
    assert_eq!(got, [(0.0, 0.0), (0.0, 0.0)]);
    assert!(columns(&[], 100.0).is_empty());
}

#[test]
fn a_fill_of_no_weight_gets_no_width() {
    let got = columns(&[fixed(50.0), fill(0.0), fill(0.0)], 100.0);
    assert_eq!(got, [(0.0, 50.0), (50.0, 0.0), (50.0, 0.0)]);
}

#[test]
fn an_overflow_shrinks_the_fill_then_clips_the_last_columns() {
    let got = columns(&[fixed(80.0), fixed(80.0), fill(1.0), fixed(50.0)], 120.0);
    assert_eq!(got, [(0.0, 80.0), (80.0, 40.0), (120.0, 0.0), (120.0, 0.0)]);
}

#[test]
fn bad_numbers_never_make_a_negative_or_nan_column() {
    let bad = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -5.0, 0.0, 30.0];
    for w in bad.iter().chain(&[200.0]) {
        for a in bad {
            for b in bad {
                let list = [fixed(a), fill(b), fill(1.0), fixed(b)];
                let total = if w.is_finite() { w.max(0.0) } else { 0.0 };
                for (x, width) in columns(&list, *w) {
                    assert!(x.is_finite() && x >= 0.0, "{a} {b} {w}: x {x}");
                    assert!(width.is_finite() && width >= 0.0, "{a} {b} {w}: {width}");
                    assert!(x + width <= total + 1e-3, "{a} {b} {w}: past the edge");
                }
            }
        }
    }
}

// Selection and the body keys.

#[test]
fn a_click_selects_one_row_and_a_second_replaces_it() {
    let s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_row(2));
    let r = run(&mut h, &s);
    assert_eq!(selected(&r), keys(&[2]));
    assert!(r.r.response.changed);
    click(&mut h, over_row(4));
    let r = run(&mut h, &s);
    assert_eq!(selected(&r), keys(&[4]));
    assert_eq!(r.out.focused, Some(body_id()), "the body holds the focus");
}

#[test]
fn the_keys_move_the_selection_through_the_list_frame() {
    let s = setup(100);
    let mut h = harness();
    focus_body(&mut h, &s);
    let r = press_key(&mut h, &s, NamedKey::ArrowDown, NONE);
    assert_eq!(selected(&r), keys(&[0]), "the first press lands on row 0");
    let r = press_key(&mut h, &s, NamedKey::ArrowDown, NONE);
    assert_eq!(selected(&r), keys(&[1]));
    let r = press_key(&mut h, &s, NamedKey::ArrowDown, SHIFT);
    assert_eq!(selected(&r), keys(&[1, 2]));
    let r = press_key(&mut h, &s, NamedKey::ArrowUp, NONE);
    assert_eq!(selected(&r), keys(&[1]));
    let r = press_key(&mut h, &s, NamedKey::End, NONE);
    assert_eq!(selected(&r), keys(&[99]));
    let r = press_key(&mut h, &s, NamedKey::Home, NONE);
    assert_eq!(selected(&r), keys(&[0]));
    let r = press_key(&mut h, &s, NamedKey::PageDown, NONE);
    assert_eq!(r.sel.cursor, Some(RowKey(9)), "floor(252 / 28) rows");
    assert_eq!(selected(&r), keys(&[9]));
}

#[test]
fn a_press_on_a_control_cell_reaches_the_control_and_not_the_selection() {
    let s = setup(100);
    let mut h = harness();
    let clicked = Flag::new(0);
    let mut cell = |ui: &mut Ui<'_>, c: &CellCtx| {
        if c.column == 3 {
            let r = Rect::new(c.rect.x, c.rect.y + 2.0, 50.0, 24.0);
            if ui.button("b", r, "go", ButtonKind::Secondary).clicked {
                clicked.set(clicked.get() + 1);
            }
            Cell::Control
        } else {
            plain(ui, c)
        }
    };
    frame_with(&mut h, &s, &mut cell);
    let (x, y) = (TABLE.x + 290.0 + PAD + 25.0, over_row(3).1);
    h.input.pointer_to(x, y).click(P);
    let r = frame_with(&mut h, &s, &mut cell);
    assert_eq!(clicked.get(), 1);
    assert!(r.sel.selection.is_empty());
    assert_eq!(r.sel.anchor, None);
    assert_eq!(
        r.out.focused,
        Some(cell_id(3, NOTE).with("b")),
        "the control is focused"
    );
    // The cell around the control still selects the row.
    click(&mut h, (TABLE.x + 40.0, y));
    let r = frame_with(&mut h, &s, &mut cell);
    assert_eq!(selected(&r), keys(&[3]));
}

#[test]
fn the_row_stays_lit_while_the_pointer_is_over_a_control_in_it() {
    let s = setup(100);
    let mut h = harness();
    let mut cell = |ui: &mut Ui<'_>, c: &CellCtx| {
        if c.column == 3 {
            let r = Rect::new(c.rect.x, c.rect.y + 2.0, 50.0, 24.0);
            ui.button("b", r, "go", ButtonKind::Secondary);
            Cell::Control
        } else {
            plain(ui, c)
        }
    };
    frame_with(&mut h, &s, &mut cell);
    h.input
        .pointer_to(TABLE.x + 290.0 + PAD + 25.0, over_row(3).1);
    frame_with(&mut h, &s, &mut cell);
    let r = frame_with(&mut h, &s, &mut cell);
    let lit: Vec<usize> = r
        .cells
        .iter()
        .filter(|c| c.hovered)
        .map(|c| c.row)
        .collect();
    assert_eq!(lit, [3, 3, 3, 3], "all four cells of row 3, no other row");
}

#[test]
fn a_dialog_over_the_table_lights_no_row() {
    let s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    h.input.pointer_to(over_row(2).0, over_row(2).1);
    run(&mut h, &s);
    assert!(run(&mut h, &s).cells.iter().any(|c| c.hovered), "lit alone");
    let modal = OverlayOptions {
        layer: LayerId::Modal,
        modal: true,
        dismiss_on_escape: true,
        dismiss_on_outside_press: false,
        dismiss_on_focus_loss: false,
        anchor: None,
        initial_focus: None,
        open: lune_ui_core::motion::Motion::of(lune_ui_core::motion::Kind::Appear),
        close: lune_ui_core::motion::Motion::of(lune_ui_core::motion::Kind::Dismiss),
    };
    let mut lit = Vec::new();
    for _ in 0..4 {
        let columns = cols(true);
        let opts = TableOptions {
            columns: &columns,
            count: s.count,
            selection: SelectionMode::Multi,
            wrap: false,
            index_of: None,
            label: "Requests",
        };
        lit.clear();
        h.frame(|ui| {
            ui.table(
                "t",
                TABLE,
                &opts,
                |i| RowKey(i as u64),
                |_, c| {
                    if c.hovered {
                        lit.push(c.row);
                    }
                    Cell::text("x")
                },
            );
            ui.overlay(
                "dlg",
                &modal,
                |_, _, _| {},
                |ui, _| {
                    let id = ui.id("card");
                    ui.interact(id, TABLE, Sense::HOVER);
                },
            );
        });
    }
    assert!(lit.is_empty(), "rows lit under a dialog: {lit:?}");
}

#[test]
fn selection_survives_a_re_sort_and_scrolling_out_of_view() {
    let mut s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_row(2));
    let before = run(&mut h, &s);
    let at = before.out.tree.get(row_id(2)).expect("row 2").bounds.y;
    // The app re-sorts: the same keys, the other way round.
    s.reverse = true;
    let after = run(&mut h, &s);
    assert_eq!(selected(&after), keys(&[2]));
    let n = after.out.tree.get(row_id(2));
    assert!(n.is_none(), "row 2 is now at the far end of the list");
    // Scroll to the end and the selected key is in view, selected.
    h.input.pointer_to(over_row(3).0, over_row(3).1);
    h.input.scroll_pixels(0.0, -100_000.0);
    run(&mut h, &s);
    let r = run(&mut h, &s);
    let n = r.out.tree.get(row_id(2)).expect("row 2 in view");
    assert!(n.states.contains(State::Selected));
    assert_ne!(n.bounds.y, at);
    // Away again: the key is still selected.
    h.input.scroll_pixels(0.0, 100_000.0);
    run(&mut h, &s);
    assert_eq!(selected(&run(&mut h, &s)), keys(&[2]));
}

#[test]
fn a_row_that_left_the_count_is_not_selected_and_retain_prunes_it() {
    let mut s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    h.frame(|ui| {
        ui.table_selection("t").selection.insert(RowKey(50));
        ui.table_selection("t").selection.insert(RowKey(3));
    });
    s.count = 10;
    let r = run(&mut h, &s);
    let marked: Vec<WidgetId> = r
        .out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Row && n.states.contains(State::Selected))
        .map(|n| n.id)
        .collect();
    assert_eq!(marked, [row_id(3)]);
    h.frame(|ui| ui.table_selection("t").retain(|k| k.0 < 10));
    assert_eq!(selected(&run(&mut h, &s)), keys(&[3]));
}

#[test]
fn a_hundred_thousand_rows_build_only_the_window() {
    let s = setup(100_000);
    let mut h = harness();
    run(&mut h, &s);
    let r = run(&mut h, &s);
    let rows = r
        .out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Row)
        .count();
    // 9 rows in view, MARGIN 2 each side, the header's row too.
    assert!(rows <= 1 + 9 + 4, "{rows} rows built");
    assert!(
        r.cells.len() <= 4 * (9 + 4),
        "{} cells asked",
        r.cells.len()
    );
    let cells = r
        .out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Cell)
        .count();
    assert_eq!(cells, r.cells.len());
    assert_eq!(r.r.extent, 100_000.0 * f64::from(ROW));
}

// The header.

#[test]
fn header_clicks_report_ascending_then_descending_and_another_column_ascending() {
    let s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_header(0));
    let r = run(&mut h, &s);
    let asc = SortKey {
        column: NAME,
        descending: false,
    };
    assert_eq!(r.r.sort, Some(asc));
    assert_eq!(r.state.sort, Some(asc));
    assert_eq!(run(&mut h, &s).r.sort, None, "only the frame it changed");
    click(&mut h, over_header(0));
    let r = run(&mut h, &s);
    let desc = SortKey {
        column: NAME,
        descending: true,
    };
    assert_eq!(r.r.sort, Some(desc));
    click(&mut h, over_header(0));
    assert_eq!(run(&mut h, &s).r.sort, Some(asc), "no unsorted third click");
    click(&mut h, over_header(2));
    let r = run(&mut h, &s);
    let amount = SortKey {
        column: AMOUNT,
        descending: false,
    };
    assert_eq!(r.r.sort, Some(amount));
    assert_eq!(r.state.sort, Some(amount));
    assert!(!r.r.response.changed, "a sort is not a selection");
}

#[test]
fn a_click_on_a_header_that_does_not_sort_does_nothing() {
    let s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_header(1));
    let r = run(&mut h, &s);
    assert_eq!(r.r.sort, None);
    assert_eq!(r.state.sort, None);
    assert_eq!(r.out.focused, None, "not a stop and takes no press");
    click(&mut h, over_header(3));
    assert_eq!(run(&mut h, &s).r.sort, None);
}

#[test]
fn the_app_clearing_the_sort_is_not_reported() {
    let s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_header(0));
    run(&mut h, &s);
    h.frame(|ui| ui.table_state("t").sort = None);
    let r = run(&mut h, &s);
    assert_eq!(r.state.sort, None);
    assert_eq!(r.r.sort, None);
}

#[test]
fn enter_and_space_on_a_focused_header_sort() {
    let s = setup(100);
    let mut h = harness();
    focus_header(&mut h, &s);
    assert_eq!(
        run(&mut h, &s).out.focused,
        Some(header_id(NAME)),
        "the first sortable header is the stop"
    );
    let r = press_key(&mut h, &s, NamedKey::Enter, NONE);
    assert_eq!(
        r.r.sort,
        Some(SortKey {
            column: NAME,
            descending: false
        })
    );
    let r = press_key(&mut h, &s, NamedKey::Space, NONE);
    assert_eq!(
        r.r.sort,
        Some(SortKey {
            column: NAME,
            descending: true
        })
    );
    assert_eq!(r.out.focused, Some(header_id(NAME)));
}

#[test]
fn left_right_home_and_end_rove_over_the_sortable_headers_only() {
    let s = setup(100);
    let mut h = harness();
    focus_header(&mut h, &s);
    // The roved focus lands on the next frame.
    let f = |h: &mut UiHarness, k| {
        press_key(h, &s, k, NONE);
        run(h, &s).out.focused
    };
    assert_eq!(f(&mut h, NamedKey::ArrowRight), Some(header_id(AMOUNT)));
    assert_eq!(
        f(&mut h, NamedKey::ArrowRight),
        Some(header_id(AMOUNT)),
        "no wrap"
    );
    assert_eq!(f(&mut h, NamedKey::ArrowLeft), Some(header_id(NAME)));
    assert_eq!(f(&mut h, NamedKey::ArrowLeft), Some(header_id(NAME)));
    assert_eq!(f(&mut h, NamedKey::End), Some(header_id(AMOUNT)));
    assert_eq!(f(&mut h, NamedKey::Home), Some(header_id(NAME)));
    // Roving moves focus and sorts nothing.
    assert_eq!(run(&mut h, &s).state.sort, None);
}

#[test]
fn the_sorted_header_is_the_stop_and_a_table_has_two_stops() {
    let s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_header(2));
    run(&mut h, &s);
    // Leave the header, come back with Tab: the stop is the sorted column.
    click(&mut h, over_row(1));
    run(&mut h, &s);
    assert_eq!(run(&mut h, &s).out.focused, Some(body_id()));
    let r = tab(&mut h, &s);
    assert_eq!(r.out.focused, Some(header_id(AMOUNT)));
    let r = tab(&mut h, &s);
    assert_eq!(r.out.focused, Some(body_id()));
    let r = tab(&mut h, &s);
    assert_eq!(
        r.out.focused,
        Some(header_id(AMOUNT)),
        "two stops in a ring"
    );
}

#[test]
fn with_nothing_sortable_the_table_has_one_stop() {
    let s = Setup {
        sortable: false,
        ..setup(100)
    };
    let mut h = harness();
    run(&mut h, &s);
    let r = tab(&mut h, &s);
    assert_eq!(r.out.focused, Some(body_id()));
    let r = tab(&mut h, &s);
    assert_eq!(r.out.focused, Some(body_id()));
    for c in cols(false) {
        let n = r.out.tree.get(header_id(c.key)).expect("header");
        assert_eq!(n.sort, None);
        assert!(!n.actions.contains(Action::Default));
    }
}

#[test]
fn an_access_default_action_sorts_on_a_header_and_selects_on_a_row() {
    let s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    h.input.access(AccessAction {
        target: header_id(AMOUNT),
        action: Action::Default,
        value: None,
    });
    let r = run(&mut h, &s);
    assert_eq!(
        r.r.sort,
        Some(SortKey {
            column: AMOUNT,
            descending: false
        })
    );
    h.input.access(AccessAction {
        target: row_id(4),
        action: Action::Default,
        value: None,
    });
    let r = run(&mut h, &s);
    assert_eq!(selected(&r), keys(&[4]));
}

// Cells.

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.texts.clone())
        .collect()
}

fn under(scene: &Scene) -> Vec<Shape> {
    scene.layers[0].geometry.under.clone()
}

fn body_style(color: [u8; 4]) -> Style {
    Style {
        line: 20.0,
        ..Style::ui(DARK.font_base, color)
    }
}

fn mono_style(color: [u8; 4]) -> Style {
    Style::mono(DARK.font_base, 20.0, color)
}

fn head_style(color: [u8; 4]) -> Style {
    Style {
        line: 16.0,
        ..Style::ui(DARK.font_s, color).medium()
    }
}

fn find(scene: &Scene, spec: &Spec, scale: f32) -> Option<TextItem> {
    let key = spec.cache_key(scale);
    texts(scene).into_iter().find(|t| t.key.0 == key)
}

fn scaled() -> UiHarness {
    let mut h = harness();
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    h
}

#[test]
fn each_tone_has_its_colour_and_a_selected_rows_text_is_bright_whatever_the_tone() {
    let tones = [
        (CellTone::Default, DARK.fg_primary),
        (CellTone::Muted, DARK.fg_secondary),
        (CellTone::Warn, DARK.warn),
        (CellTone::Err, DARK.err),
        (CellTone::Dim, DARK.fg_tertiary),
        (CellTone::Strong, DARK.fg_bright),
    ];
    let s = setup(6);
    let tone_of = |c: &CellCtx| tones[c.row].0;
    let mut h = scaled();
    let mut cell = |_: &mut Ui<'_>, c: &CellCtx| Cell::text(format!("t{}", c.row)).tone(tone_of(c));
    let r = frame_with(&mut h, &s, &mut cell);
    for (i, (tone, color)) in tones.iter().enumerate() {
        let spec = Spec::plain(format!("t{i}"), body_style(*color));
        let t = find(&r.scene, &spec, 2.0);
        assert!(t.is_some(), "{tone:?} row {i}");
        // Only the first column is asked for each row's text: it is one text a row and column.
    }
    // Select every row: all text is fg_bright.
    h.frame(|ui| {
        for i in 0..6 {
            ui.table_selection("t").selection.insert(RowKey(i));
        }
    });
    let r = frame_with(&mut h, &s, &mut cell);
    for (i, (_, tone_color)) in tones.iter().enumerate() {
        let spec = Spec::plain(format!("t{i}"), body_style(DARK.fg_bright));
        assert!(find(&r.scene, &spec, 2.0).is_some(), "row {i} bright");
        let old = Spec::plain(format!("t{i}"), body_style(*tone_color));
        if *tone_color != DARK.fg_bright {
            assert!(find(&r.scene, &old, 2.0).is_none(), "row {i} kept its tone");
        }
    }
}

#[test]
fn mono_and_num_cells_use_the_mono_face_and_num_ends_at_the_right_edge() {
    let s = setup(3);
    let mut h = scaled();
    let mut cell = |ui: &mut Ui<'_>, c: &CellCtx| match c.column {
        1 => Cell::text("mono").mono(),
        _ => plain(ui, c),
    };
    let r = frame_with(&mut h, &s, &mut cell);
    let mono = find(
        &r.scene,
        &Spec::plain("mono", mono_style(DARK.fg_primary)),
        2.0,
    );
    assert!(mono.is_some(), "a mono cell");
    let sans = Spec::plain("mono", body_style(DARK.fg_primary));
    assert!(find(&r.scene, &sans, 2.0).is_none());
    // Amount is num: mono, and its text ends 8 px inside the column's right edge.
    let amount = Spec::plain("10", mono_style(DARK.fg_primary));
    let t = find(&r.scene, &amount, 2.0).expect("a num cell");
    let width = 2.0 * 13.0 * 0.5;
    let edge = TABLE.x + PLACE[2].0 + PLACE[2].1 - PAD;
    assert_eq!(t.x, edge - width, "right edge");
    // The header of a num column is sans and right-aligned.
    let head = Spec::plain("Amount", head_style(DARK.fg_tertiary));
    let t = find(&r.scene, &head, 2.0).expect("the Amount header");
    let width = 6.0 * 11.0 * 0.5;
    assert_eq!(t.x, edge - width);
}

#[test]
fn a_long_cell_elides_and_keeps_its_full_text_as_the_node_name() {
    let long = "A long cell text that cannot fit";
    let s = setup(3);
    let mut h = harness();
    let mut cell = |ui: &mut Ui<'_>, c: &CellCtx| {
        if c.column == 0 {
            Cell::text(long)
        } else {
            plain(ui, c)
        }
    };
    let r = frame_with(&mut h, &s, &mut cell);
    let full = Spec::plain(long, body_style(DARK.fg_primary));
    assert!(find(&r.scene, &full, 1.0).is_none(), "the text was cut");
    let n = r.out.tree.get(cell_id(0, NAME)).expect("cell");
    assert_eq!(n.name, long);
    assert_eq!(n.role, Role::Cell);
}

#[test]
fn a_wrapping_row_is_as_tall_as_its_tallest_cell_and_never_under_28() {
    // The Note column is 86 px of text wide: 13 chars of 6.5 px.
    let text = "x".repeat(40);
    let lines = (40.0f32 * 6.5 / 86.0).ceil();
    let mut s = setup(4);
    s.wrap = true;
    let mut h = harness();
    let mut cell = |ui: &mut Ui<'_>, c: &CellCtx| {
        if c.column == 3 && c.row == 1 {
            Cell::text(text.clone()).wrapped()
        } else if c.column == 3 && c.row == 2 {
            Cell::text("short").wrapped()
        } else {
            plain(ui, c)
        }
    };
    frame_with(&mut h, &s, &mut cell);
    let r = frame_with(&mut h, &s, &mut cell);
    let r2 = frame_with(&mut h, &s, &mut cell);
    let h_of = |r: &Run, k: u64| r.out.tree.get(row_id(k)).expect("row").bounds.h;
    assert_eq!(h_of(&r2, 1), lines * 20.0 + 2.0 * WRAP_PAD);
    assert_eq!(h_of(&r2, 0), ROW);
    assert_eq!(h_of(&r2, 2), ROW, "one short line is under the least");
    assert_eq!(h_of(&r2, 3), ROW);
    let y = |r: &Run, k: u64| r.out.tree.get(row_id(k)).expect("row").bounds.y;
    assert_eq!(y(&r2, 2), y(&r2, 1) + h_of(&r2, 1), "rows stack on it");
    assert_eq!(r2.out.repaint, Repaint::Idle, "the measure settles");
    let _ = r;
}

/// A row's height follows the row, not its place: sorted, the rows already measured are placed as
/// the settled frame places them, on the first frame, and the frame asks for no catch-up.
#[test]
fn a_sort_places_measured_rows_at_their_own_heights_on_the_first_frame() {
    let text = "x".repeat(40);
    let lines = (40.0f32 * 6.5 / 86.0).ceil();
    let tall = lines * 20.0 + 2.0 * WRAP_PAD;
    let mut s = setup(4);
    s.wrap = true;
    let mut h = harness();
    let mut cell = |ui: &mut Ui<'_>, c: &CellCtx| {
        // Key 1 is the tall row, key 2 a short wrapped one, wherever they are sorted.
        if c.column == 3 && c.key.0 == 1 {
            Cell::text(text.clone()).wrapped()
        } else if c.column == 3 && c.key.0 == 2 {
            Cell::text("short").wrapped()
        } else {
            plain(ui, c)
        }
    };
    for _ in 0..3 {
        frame_with(&mut h, &s, &mut cell);
    }
    let h_of = |r: &Run, k: u64| r.out.tree.get(row_id(k)).expect("row").bounds.h;
    let y_of = |r: &Run, k: u64| r.out.tree.get(row_id(k)).expect("row").bounds.y;
    let settled = frame_with(&mut h, &s, &mut cell);
    assert_eq!(h_of(&settled, 1), tall, "instrument: the row is tall");
    assert_eq!(settled.out.repaint, Repaint::Idle);
    // The app sorts: the same rows the other way round.
    s.reverse = true;
    let first = frame_with(&mut h, &s, &mut cell);
    let after = frame_with(&mut h, &s, &mut cell);
    for k in 0..4 {
        assert_eq!(h_of(&first, k), h_of(&after, k), "row {k}'s height");
        assert_eq!(y_of(&first, k), y_of(&after, k), "row {k}'s place");
    }
    assert_eq!(h_of(&first, 1), tall);
    assert_eq!(
        first.out.repaint,
        Repaint::Idle,
        "no catch-up for rows already measured"
    );
}

/// A wrapping number cell right-aligns every visual line against the cell's right edge.
#[test]
fn a_wrapping_num_cell_right_aligns_each_visual_line() {
    let text = "aaa bbb cccc dd eeeee ff gggg h";
    let mut s = setup(2);
    s.wrap = true;
    let mut h = harness();
    let mut cell = |ui: &mut Ui<'_>, c: &CellCtx| {
        if c.column == 2 && c.key.0 == 1 {
            Cell::text(text).wrapped()
        } else {
            plain(ui, c)
        }
    };
    frame_with(&mut h, &s, &mut cell);
    let r = frame_with(&mut h, &s, &mut cell);
    let right = r
        .cells
        .iter()
        .find(|c| c.column == 2 && c.key.0 == 1)
        .expect("the cell")
        .rect;
    let style = mono_style(DARK.fg_primary);
    let spec = Spec::plain(text, style).wrapped(right.w);
    let lines = FakeText.lines(&spec, 1.0);
    assert!(lines.len() >= 3, "instrument: it wraps: {lines:?}");
    let mut ys = Vec::new();
    for l in &lines {
        let row = text[l.start..l.end].trim();
        let one = Spec::plain(row, style);
        let seen = find(&r.scene, &one, 1.0).unwrap_or_else(|| panic!("line {row:?} is drawn"));
        let w = FakeText.measure(&one, 1.0).0;
        assert_eq!(seen.x + w, right.right(), "{row:?} ends at the cell's edge");
        ys.push(seen.y);
    }
    assert!(
        ys.windows(2).all(|w| w[1] > w[0]),
        "one under the other: {ys:?}"
    );
}

#[test]
fn without_the_wrap_option_a_wrap_cell_stays_one_elided_line() {
    let s = setup(3);
    let mut h = harness();
    let mut cell = |ui: &mut Ui<'_>, c: &CellCtx| {
        if c.column == 3 {
            Cell::text("x".repeat(40)).wrapped()
        } else {
            plain(ui, c)
        }
    };
    frame_with(&mut h, &s, &mut cell);
    let r = frame_with(&mut h, &s, &mut cell);
    assert!(
        r.out
            .tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::Row && n.id != head_id())
            .all(|n| n.bounds.h == ROW)
    );
    // The text itself is one elided line: neither the whole text nor a wrapped layout of it is drawn
    // (the Note column's text is 86 px wide).
    let style = body_style(DARK.fg_primary);
    let long = "x".repeat(40);
    let whole = Spec::plain(long.clone(), style);
    assert!(find(&r.scene, &whole, 1.0).is_none(), "elided");
    assert!(
        find(&r.scene, &whole.clone().wrapped(86.0), 1.0).is_none(),
        "not wrapped, though the cell asks: the table does not"
    );
}

// The look at scale 2.

fn hairlines(scene: &Scene, scale: f32) -> Vec<[f32; 4]> {
    let c = premul(DARK.border_subtle);
    under(scene)
        .into_iter()
        .filter(|s| {
            s.rect[3] == scale
                && s.rect[2] == TABLE.w * scale
                && (0..4).all(|i| (s.color[i] - c[i]).abs() < 1e-6)
        })
        .map(|s| s.rect)
        .collect()
}

#[test]
fn at_scale_2_the_header_and_rows_are_56_the_hairlines_2_and_the_insets_32_and_16() {
    let s = setup(4);
    let mut h = scaled();
    let r = run(&mut h, &s);
    let lines = hairlines(&r.scene, 2.0);
    let ys: Vec<f32> = lines.iter().map(|r| r[1]).collect();
    let head_line = (TABLE.y + HEADER - 1.0) * 2.0;
    let rows: Vec<f32> = (0..3)
        .map(|i| (TABLE.y + HEADER + (i as f32 + 1.0) * ROW - 1.0) * 2.0)
        .collect();
    let mut want = vec![head_line];
    want.extend(rows);
    assert_eq!(ys, want, "header, then every row but the last");
    assert_eq!(
        ys[1] - ys[0],
        56.0,
        "the first row is 56 below the header's"
    );
    assert_eq!(ys[2] - ys[1], 56.0);
    assert_eq!(head_line - TABLE.y * 2.0 + 2.0, 56.0, "the header is 56");
    assert!(lines.iter().all(|r| r[0] == TABLE.x * 2.0 && r[3] == 2.0));

    let t = texts(&r.scene);
    let first = find(
        &r.scene,
        &Spec::plain("Row 0", body_style(DARK.fg_primary)),
        2.0,
    )
    .expect("a first cell");
    assert_eq!(
        (first.x - TABLE.x) * 2.0,
        32.0,
        "the first cell is inset 32"
    );
    let middle = find(
        &r.scene,
        &Spec::plain("ok", body_style(DARK.fg_primary)),
        2.0,
    )
    .expect("a middle cell");
    assert_eq!(
        (middle.x - TABLE.x - PLACE[1].0) * 2.0,
        16.0,
        "a middle cell is inset 16"
    );
    let _ = t;
    // The last cell's padded box ends 16 logical px (32 device) inside the table.
    let last = r.cells.iter().find(|c| c.column == 3).expect("a last cell");
    assert_eq!(last.rect.right(), TABLE.right() - EDGE_PAD);
    let first = r
        .cells
        .iter()
        .find(|c| c.column == 0)
        .expect("a first cell");
    assert_eq!(first.rect.x, TABLE.x + EDGE_PAD);
    assert_eq!(first.rect.h, ROW);
}

#[test]
fn the_last_row_has_no_hairline() {
    let s = setup(3);
    let mut h = harness();
    let r = run(&mut h, &s);
    let ys: Vec<f32> = hairlines(&r.scene, 1.0).iter().map(|r| r[1]).collect();
    assert_eq!(ys, [47.0, 75.0, 103.0], "header, row 0, row 1");
    let none = run(&mut h, &setup(0));
    assert_eq!(hairlines(&none.scene, 1.0).len(), 1, "the header's only");
}

#[test]
fn the_flat_fill_spans_the_slot_at_radius_0_and_a_plain_list_keeps_its_inset() {
    let s = setup(100);
    let mut h = scaled();
    run(&mut h, &s);
    h.frame(|ui| {
        ui.table_selection("t").selection.insert(RowKey(2));
    });
    let mut ms = 0;
    let r = settle(&mut h, &s, &mut ms);
    let slot_y = (TABLE.y + HEADER + 2.0 * ROW) * 2.0;
    let want = premul(DARK.surface_selected);
    let fill = under(&r.scene)
        .into_iter()
        .find(|sh| (0..4).all(|i| (sh.color[i] - want[i]).abs() < 1e-6) && sh.rect[3] > 4.0)
        .expect("the selected fill");
    assert_eq!(fill.rect, [TABLE.x * 2.0, slot_y, TABLE.w * 2.0, ROW * 2.0]);
    assert_eq!(fill.radii, [0.0; 4]);

    // The plain list: inset 4 and 1, the theme's item radius.
    let list = Rect::new(20.0, 20.0, 300.0, 200.0);
    let o = ListOptions {
        count: 20,
        heights: Heights::Fixed(ROW),
        selection: SelectionMode::Multi,
        index_of: None,
        label: "rows",
    };
    let mut h = scaled();
    h.frame(|ui| {
        ui.virtual_list("l", list, &o, |i| RowKey(i as u64), |_, _| ROW);
        ui.list_state("l").selection.insert(RowKey(2));
    });
    h.at_ms(1000);
    h.frame(|ui| {
        ui.virtual_list("l", list, &o, |i| RowKey(i as u64), |_, _| ROW);
    });
    h.at_ms(2000);
    let (scene, _) = h.frame(|ui| {
        ui.virtual_list("l", list, &o, |i| RowKey(i as u64), |_, _| ROW);
    });
    let fill = under(&scene)
        .into_iter()
        .find(|sh| (0..4).all(|i| (sh.color[i] - want[i]).abs() < 1e-6) && sh.rect[3] > 4.0)
        .expect("the list's fill");
    assert_eq!(
        fill.rect,
        [
            (list.x + 4.0) * 2.0,
            (list.y + 2.0 * ROW + 1.0) * 2.0,
            (list.w - 8.0) * 2.0,
            (ROW - 2.0) * 2.0
        ]
    );
    assert_eq!(fill.radii, [DARK.item_radius * 2.0; 4]);
}

#[test]
fn a_cursor_row_outline_sits_inside_the_slot() {
    let s = setup(100);
    let mut h = harness();
    focus_body(&mut h, &s);
    for _ in 0..3 {
        press_key(&mut h, &s, NamedKey::ArrowDown, NONE);
    }
    let r = run(&mut h, &s);
    let ring = under(&r.scene)
        .into_iter()
        .find(|sh| sh.params[0] == 1.0 && sh.rect[2] == TABLE.w)
        .expect("the ring");
    let slot_y = TABLE.y + HEADER + 2.0 * ROW;
    assert!(ring.rect[1] >= slot_y && ring.rect[1] + ring.rect[3] <= slot_y + ROW);
    assert!(ring.rect[0] >= TABLE.x && ring.rect[0] + ring.rect[2] <= TABLE.right());
}

// The header's look and motion.

/// The alpha of the header's focus ring in `r`, 0 when none is drawn.
fn header_ring_alpha(r: &Run) -> f32 {
    under(&r.scene)
        .into_iter()
        .filter(|sh| sh.params[0] == 1.0 && sh.rect[1] < TABLE.y + HEADER)
        .map(|sh| sh.color[3])
        .fold(0.0, f32::max)
}

/// A header's focus ring fades in like every other ring (`FOCUS`), by the cell's own id.
#[test]
fn a_header_ring_fades_with_the_focus_motion() {
    let s = setup(100);
    let mut h = harness();
    h.at_ms(1000);
    focus_header(&mut h, &s);
    h.at_ms(1060);
    let mid = header_ring_alpha(&run(&mut h, &s));
    h.at_ms(2000);
    let settled = header_ring_alpha(&run(&mut h, &s));
    assert!(settled > 0.0, "instrument: the ring settles drawn");
    assert!(
        mid > 0.0 && mid < settled,
        "between nothing and the settled {settled}: {mid}"
    );
    // And out again: focus leaves the header.
    click(&mut h, over_row(1));
    run(&mut h, &s);
    h.at_ms(2060);
    let out = header_ring_alpha(&run(&mut h, &s));
    assert!(out > 0.0 && out < settled, "fading out: {out}");
}

#[test]
fn the_sorted_header_is_secondary_with_an_arrow_four_px_after_the_text() {
    let s = setup(10);
    let mut h = harness();
    run(&mut h, &s);
    let before = run(&mut h, &s);
    let up = icon::glyph(icon::ARROW_UP, ARROW, DARK.fg_secondary);
    assert!(
        find(&before.scene, &up, 1.0).is_none(),
        "no arrow when unsorted"
    );
    assert!(
        find(
            &before.scene,
            &Spec::plain("Name", head_style(DARK.fg_tertiary)),
            1.0
        )
        .is_some()
    );
    click(&mut h, over_header(0));
    run(&mut h, &s);
    let mut ms = 0;
    let r = settle(&mut h, &s, &mut ms);
    let name = find(
        &r.scene,
        &Spec::plain("Name", head_style(DARK.fg_secondary)),
        1.0,
    )
    .expect("the sorted header");
    assert_eq!(name.x, TABLE.x + EDGE_PAD);
    let arrow = find(&r.scene, &up, 1.0).expect("the ascending arrow");
    // The glyph is centred in a 12 px box 4 px after the text (4 chars of 5.5 px).
    let box_x = name.x + 4.0 * 5.5 + ARROW_GAP;
    let w = 12.0 * 0.5;
    assert_eq!(arrow.x, box_x + (ARROW - w) / 2.0);
    // A second click flips the glyph.
    click(&mut h, over_header(0));
    run(&mut h, &s);
    let r = settle(&mut h, &s, &mut ms);
    let down = icon::glyph(icon::ARROW_DOWN, ARROW, DARK.fg_secondary);
    assert!(find(&r.scene, &down, 1.0).is_some());
    assert!(find(&r.scene, &up, 1.0).is_none());
}

#[test]
fn on_a_num_column_the_arrow_is_at_the_right_end_and_the_text_before_it() {
    let s = setup(10);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_header(2));
    run(&mut h, &s);
    let mut ms = 0;
    let r = settle(&mut h, &s, &mut ms);
    let edge = TABLE.x + PLACE[2].0 + PLACE[2].1 - PAD;
    let arrow = find(
        &r.scene,
        &icon::glyph(icon::ARROW_UP, ARROW, DARK.fg_secondary),
        1.0,
    )
    .expect("arrow");
    assert_eq!(arrow.x, edge - ARROW + (ARROW - 6.0) / 2.0);
    let text = find(
        &r.scene,
        &Spec::plain("Amount", head_style(DARK.fg_secondary)),
        1.0,
    )
    .expect("text");
    assert_eq!(text.x, edge - ARROW - ARROW_GAP - 33.0);
}

#[test]
fn the_header_fades_over_fast_and_the_reduced_and_off_tiers_agree_at_rest() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        let t = DARK.motion;
        let from = [1.0, 1.0, 1.0, 0.0f32];
        let to = [1.0, 1.0, 1.0, 0.5f32];
        conformance(&HEADER_FADE, &t, tier, from, to).expect("conforms");
    }
    let s = setup(10);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_header(0));
    h.at_ms(1000);
    run(&mut h, &s);
    h.at_ms(1060);
    let mid = run(&mut h, &s);
    assert_eq!(mid.out.repaint, Repaint::Now, "in flight");
    let tertiary = Spec::plain("Name", head_style(DARK.fg_tertiary));
    let secondary = Spec::plain("Name", head_style(DARK.fg_secondary));
    assert!(find(&mid.scene, &tertiary, 1.0).is_none());
    assert!(
        find(&mid.scene, &secondary, 1.0).is_none(),
        "a mixed colour"
    );
    let mut ms = 1000;
    let end = settle(&mut h, &s, &mut ms);
    assert!(find(&end.scene, &secondary, 1.0).is_some());

    // Off: the sorted look is there on the frame of the click.
    let mut h = harness();
    h.tier = MotionTier::Off;
    run(&mut h, &s);
    click(&mut h, over_header(0));
    let r = run(&mut h, &s);
    assert!(find(&r.scene, &secondary, 1.0).is_some());
    let up = icon::glyph(icon::ARROW_UP, ARROW, DARK.fg_secondary);
    assert!(find(&r.scene, &up, 1.0).is_some());
}

/// The brightness of the sort arrow `glyph` in `scene`, if it is drawn: its colour is `fg_secondary`
/// at the fade's alpha, found by the text key.
fn arrow_alpha(scene: &Scene, glyph: &str) -> Option<u8> {
    (1..=255u8).find(|&a| {
        let mut c = DARK.fg_secondary;
        c[3] = a;
        find(scene, &icon::glyph(glyph, ARROW, c), 1.0).is_some()
    })
}

/// A sorted header that the app unsorts fades its arrow out the way it came in: the same
/// direction, still drawn half way, gone at the end.
#[test]
fn an_unsorted_header_fades_its_arrow_out_in_the_direction_it_showed() {
    let s = setup(10);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_header(0));
    run(&mut h, &s);
    click(&mut h, over_header(0));
    run(&mut h, &s);
    let mut ms = 0;
    let r = settle(&mut h, &s, &mut ms);
    assert_eq!(r.state.sort.map(|k| k.descending), Some(true));
    let full = arrow_alpha(&r.scene, icon::ARROW_DOWN).expect("the descending arrow");
    assert_eq!(full, DARK.fg_secondary[3]);

    let clear = Setup {
        clear_sort: true,
        ..s
    };
    ms += 1000;
    h.at_ms(ms);
    run(&mut h, &clear);
    h.at_ms(ms + 60);
    let mid = run(&mut h, &clear);
    assert_eq!(mid.out.repaint, Repaint::Now, "in flight");
    let a = arrow_alpha(&mid.scene, icon::ARROW_DOWN).expect("still drawn half way, pointing down");
    assert!(a > 0 && a < full, "partly faded: {a}");
    assert!(
        arrow_alpha(&mid.scene, icon::ARROW_UP).is_none(),
        "not flipped by the fade"
    );
    let end = settle(&mut h, &clear, &mut ms);
    assert!(arrow_alpha(&end.scene, icon::ARROW_DOWN).is_none());
    assert!(arrow_alpha(&end.scene, icon::ARROW_UP).is_none());
}

#[test]
fn a_table_at_rest_is_idle_and_stays_idle_with_a_sort_and_a_selection() {
    let s = setup(100);
    let mut h = harness();
    let mut ms = 0;
    run(&mut h, &s);
    click(&mut h, over_row(3));
    run(&mut h, &s);
    click(&mut h, over_header(0));
    run(&mut h, &s);
    let r = settle(&mut h, &s, &mut ms);
    assert_eq!(r.out.repaint, Repaint::Idle);
    for _ in 0..3 {
        ms += 100;
        h.at_ms(ms);
        assert_eq!(run(&mut h, &s).out.repaint, Repaint::Idle);
    }
}

// The tree.

#[test]
fn the_tree_is_a_table_with_a_header_row_of_column_headers_and_a_group_of_rows_of_cells() {
    let s = setup(100);
    let mut h = harness();
    run(&mut h, &s);
    click(&mut h, over_header(2));
    run(&mut h, &s);
    click(&mut h, over_row(1));
    let r = run(&mut h, &s);
    let t = &r.out.tree;
    let table = t.get(table_id()).expect("table");
    assert_eq!(table.role, Role::Table);
    assert_eq!(table.name, "Requests");
    assert_eq!(table.children.first(), Some(&head_id()));
    let head = t.get(head_id()).expect("header row");
    assert_eq!(head.role, Role::Row);
    assert_eq!(head.parent, Some(table_id()));
    let kids: Vec<WidgetId> = [NAME, STATUS, AMOUNT, NOTE].map(header_id).to_vec();
    assert_eq!(head.children, kids);
    let sorts: Vec<Option<Sort>> = kids
        .iter()
        .map(|k| t.get(*k).expect("column header").sort)
        .collect();
    assert_eq!(
        sorts,
        [Some(Sort::None), None, Some(Sort::Ascending), None],
        "none on a sortable, the direction on the sorted, nothing on the rest"
    );
    for (k, title) in kids.iter().zip(["Name", "Status", "Amount", "Note"]) {
        let n = t.get(*k).expect("column header");
        assert_eq!((n.role, n.name.as_str()), (Role::ColumnHeader, title));
        assert_eq!(n.parent, Some(head_id()));
    }
    assert!(
        t.get(header_id(NAME))
            .expect("h")
            .actions
            .contains(Action::Default)
    );
    assert!(
        !t.get(header_id(STATUS))
            .expect("h")
            .actions
            .contains(Action::Default)
    );

    let body = t.get(body_id()).expect("body group");
    assert_eq!(body.role, Role::Group);
    assert_eq!(body.name, "Requests");
    assert!(body.states.contains(State::Focusable));
    let area = t.get(body.parent.expect("area")).expect("area");
    assert_eq!(area.role, Role::ScrollArea);
    assert_eq!(area.parent, Some(table_id()));
    let row = t.get(row_id(1)).expect("row 1");
    assert_eq!(row.role, Role::Row);
    assert_eq!(row.parent, Some(body_id()));
    assert!(row.states.contains(State::Selected));
    assert!(
        !t.get(row_id(2))
            .expect("row 2")
            .states
            .contains(State::Selected)
    );
    let cells: Vec<(Role, &str)> = row
        .children
        .iter()
        .map(|c| {
            let n = t.get(*c).expect("cell");
            (n.role, n.name.as_str())
        })
        .collect();
    assert_eq!(
        cells,
        [
            (Role::Cell, "Row 1"),
            (Role::Cell, "ok"),
            (Role::Cell, "10"),
            (Role::Cell, "n1")
        ]
    );
    // The same group of cells in the descending case.
    click(&mut h, over_header(2));
    let r = run(&mut h, &s);
    assert_eq!(
        r.out.tree.get(header_id(AMOUNT)).expect("h").sort,
        Some(Sort::Descending)
    );
}

#[test]
fn a_control_cells_nodes_nest_under_its_cell() {
    let s = setup(4);
    let mut h = harness();
    let mut cell = |ui: &mut Ui<'_>, c: &CellCtx| {
        if c.column == 3 {
            ui.button("b", c.rect, "go", ButtonKind::Secondary);
            Cell::Control
        } else {
            plain(ui, c)
        }
    };
    let r = frame_with(&mut h, &s, &mut cell);
    let cell_node = r.out.tree.get(cell_id(0, NOTE)).expect("cell");
    assert_eq!(cell_node.children, [cell_id(0, NOTE).with("b")]);
    assert_eq!(cell_node.name, "");
}

// Disabled and chords.

const HEADER_KEYS: [NamedKey; 6] = [
    NamedKey::ArrowLeft,
    NamedKey::ArrowRight,
    NamedKey::Home,
    NamedKey::End,
    NamedKey::Enter,
    NamedKey::Space,
];
const BODY_KEYS: [NamedKey; 6] = [
    NamedKey::ArrowUp,
    NamedKey::ArrowDown,
    NamedKey::Home,
    NamedKey::End,
    NamedKey::PageUp,
    NamedKey::PageDown,
];

fn bound(h: &mut UiHarness, k: NamedKey) {
    h.shortcuts
        .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::F5)),
        "f5",
    ));
}

fn fired(r: &Run) -> Vec<CommandId> {
    r.out.fired.clone()
}

#[test]
fn a_header_focused_then_disabled_takes_its_keys_and_changes_nothing() {
    let s = setup(100);
    for k in HEADER_KEYS {
        let mut h = harness();
        bound(&mut h, k);
        focus_header(&mut h, &s);
        let on = run(&mut h, &s);
        assert_eq!(on.out.focused, Some(header_id(NAME)));
        run_off(&mut h, &s);
        h.input.key_down(Key::Named(k), None);
        let r = run_off(&mut h, &s);
        assert!(fired(&r).is_empty(), "{k:?} fired a binding");
        assert_eq!(r.r.sort, None, "{k:?}");
        assert_eq!(r.state.sort, None, "{k:?}");
        assert_eq!(r.out.focused, on.out.focused, "{k:?}");
        h.input.key_down(Key::Named(NamedKey::F5), None);
        assert_eq!(fired(&run_off(&mut h, &s)).len(), 1, "F5 after {k:?}");
    }
}

#[test]
fn a_body_focused_then_disabled_takes_its_keys_and_changes_nothing() {
    let s = setup(100);
    for k in BODY_KEYS {
        let mut h = harness();
        bound(&mut h, k);
        focus_body(&mut h, &s);
        click(&mut h, over_row(3));
        run(&mut h, &s);
        let before = run(&mut h, &s).sel;
        run_off(&mut h, &s);
        h.input.key_down(Key::Named(k), None);
        let r = run_off(&mut h, &s);
        assert!(fired(&r).is_empty(), "{k:?} fired a binding");
        assert_eq!(r.sel.selection, before.selection, "{k:?}");
        assert_eq!(r.sel.cursor, before.cursor, "{k:?}");
        h.input.key_down(Key::Named(NamedKey::F5), None);
        assert_eq!(fired(&run_off(&mut h, &s)).len(), 1, "F5 after {k:?}");
    }
    // Command+C copies nothing and fires nothing.
    let mut h = harness();
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Character("c".into())).logo(),
        "copy",
    ));
    focus_body(&mut h, &s);
    click(&mut h, over_row(2));
    run(&mut h, &s);
    run_off(&mut h, &s);
    h.input
        .set_modifiers(META)
        .key_down(Key::Character("c".into()), Some("c"));
    let r = run_off(&mut h, &s);
    h.input.set_modifiers(NONE);
    assert!(fired(&r).is_empty());
    assert_eq!(r.r.copy, None);
}

type Held = (Modifiers, fn(Chord) -> Chord);

#[test]
fn a_chord_on_the_tables_keys_stays_a_shortcut() {
    let s = setup(100);
    let held = |control, alt, logo| Modifiers {
        shift: false,
        control,
        alt,
        logo,
    };
    let chords: [Held; 3] = [
        (held(true, false, false), Chord::ctrl),
        (held(false, true, false), Chord::alt),
        (held(false, false, true), Chord::logo),
    ];
    for (m, chord) in chords {
        for (target, list) in [(true, HEADER_KEYS), (false, BODY_KEYS)] {
            for k in list {
                for disabled in [false, true] {
                    let mut h = harness();
                    h.shortcuts.add(Binding::new(
                        Scope::Global,
                        chord(Chord::key(Key::Named(k))),
                        "chord",
                    ));
                    if target {
                        focus_header(&mut h, &s);
                    } else {
                        focus_body(&mut h, &s);
                        click(&mut h, over_row(3));
                    }
                    run(&mut h, &s);
                    let before = run(&mut h, &s);
                    let r = if disabled {
                        run_off(&mut h, &s);
                        h.input.set_modifiers(m).key_down(Key::Named(k), None);
                        run_off(&mut h, &s)
                    } else {
                        h.input.set_modifiers(m).key_down(Key::Named(k), None);
                        run(&mut h, &s)
                    };
                    h.input.set_modifiers(NONE);
                    let why = format!("{m:?} {k:?} header={target} disabled={disabled}");
                    assert_eq!(fired(&r).len(), 1, "{why}");
                    assert_eq!(r.r.sort, None, "{why}");
                    assert_eq!(r.sel.selection, before.sel.selection, "{why}");
                    assert_eq!(r.sel.cursor, before.sel.cursor, "{why}");
                    assert_eq!(r.out.focused, before.out.focused, "{why}");
                }
            }
        }
    }
}

#[test]
fn a_disabled_table_is_inert_and_its_nodes_are_disabled() {
    let s = setup(100);
    let mut h = harness();
    run_off(&mut h, &s);
    click(&mut h, over_header(0));
    click(&mut h, over_row(2));
    let r = run_off(&mut h, &s);
    assert_eq!(r.r.sort, None);
    assert!(r.sel.selection.is_empty());
    for id in [header_id(NAME), row_id(0), table_id()] {
        let n = r.out.tree.get(id).expect("node");
        assert!(n.states.contains(State::Disabled), "{id:?}");
        assert!(!n.actions.contains(Action::Default), "{id:?}");
    }
}

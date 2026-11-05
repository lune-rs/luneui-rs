//! The diff view (`text.diff-view` acceptance, D15 and D16): the pairing, the rows and their colours,
//! selection, keys and copy through the list frame in unified and in each split pane, the clear on a
//! press in the other pane, hunk actions, the linked scroll, wrap and the horizontal offset,
//! preservation by identity, marks, scale, the tree, the look at scale 2, and motion.

use lune_ui_core::access::{AccessAction, Action, Role, SemanticTree, State};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{
    ClipboardRequest, FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint,
};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::testing::{ScriptedInput, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::{HOVER, PRESS};
use lune_ui_core::widgets::diff_view::{
    DiffLine, DiffMark, DiffMode, DiffOptions, DiffResponse, HunkAction, LineKind, ROW, SplitRow,
    pair_rows, widest_chars,
};
use lune_ui_core::widgets::text_lines::HighlightSpan;
use lune_ui_core::widgets::virtual_list::RowKey;

const WIN: (f32, f32) = (700.0, 400.0);
/// Ten rows in view.
const DIFF: Rect = Rect::new(20.0, 20.0, 600.0, 200.0);
const P: PointerButton = PointerButton::Primary;
const NONE: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: false,
};
const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..NONE
};
const META: Modifiers = Modifiers { logo: true, ..NONE };

const ACTIONS: [HunkAction<'static>; 2] = [
    HunkAction {
        glyph: icon::ADD,
        label: "Stage hunk",
    },
    HunkAction {
        glyph: icon::REMOVE,
        label: "Unstage hunk",
    },
];

// The lines.

fn line(
    kind: LineKind,
    old: Option<u32>,
    new: Option<u32>,
    text: &'static str,
) -> DiffLine<'static> {
    DiffLine {
        old,
        new,
        ..DiffLine::new(kind, text)
    }
}

fn hunk(i: u32, text: &'static str) -> DiffLine<'static> {
    DiffLine {
        hunk: Some(i),
        ..DiffLine::new(LineKind::Hunk, text)
    }
}

/// Two hunks: lines 0 to 11.
fn doc() -> Vec<DiffLine<'static>> {
    use LineKind::*;
    vec![
        hunk(0, "@@ -1,4 +1,4 @@"),
        line(Context, Some(1), Some(1), "alpha"),
        line(Context, Some(2), Some(2), "beta"),
        line(Removed, Some(3), None, "gamma old"),
        line(Removed, Some(4), None, "delta old"),
        line(Added, None, Some(3), "gamma new"),
        line(Context, Some(5), Some(4), "epsilon"),
        hunk(1, "@@ -20,3 +20,3 @@"),
        line(Context, Some(20), Some(20), "zeta"),
        line(Removed, Some(21), None, "eta old"),
        line(Added, None, Some(21), "eta new"),
        line(Context, Some(22), Some(22), "theta"),
    ]
}

/// `n` context lines, generated.
fn big(n: usize) -> Vec<DiffLine<'static>> {
    (0..n)
        .map(|i| {
            let text: &'static str = Box::leak(format!("line {i}").into_boxed_str());
            line(
                LineKind::Context,
                Some(i as u32 + 1),
                Some(i as u32 + 1),
                text,
            )
        })
        .collect()
}

// The frame.

#[derive(Clone, Copy)]
struct Setup {
    split: bool,
    wrap: bool,
    actions: usize,
    identity: u64,
    enabled: bool,
}

fn setup() -> Setup {
    Setup {
        split: false,
        wrap: false,
        actions: 0,
        identity: 1,
        enabled: true,
    }
}

fn split() -> Setup {
    Setup {
        split: true,
        ..setup()
    }
}

thread_local! {
    /// What the last frame's queries saw: the selected keys of `u`, `l` and `r`, and the selected text.
    static SEEN: std::cell::RefCell<([Vec<u64>; 3], String)> = std::cell::RefCell::default();
}

struct Run {
    r: DiffResponse,
    out: FrameOutput,
    scene: Scene,
}

fn show(h: &mut UiHarness, lines: &[DiffLine<'_>], rows: &[SplitRow], s: &Setup) -> Run {
    let opts = DiffOptions {
        mode: if s.split {
            DiffMode::Split(rows)
        } else {
            DiffMode::Unified
        },
        wrap: s.wrap,
        label: "Changes",
        actions: &ACTIONS[..s.actions],
        identity: s.identity,
        widest_chars: widest_chars(lines),
    };
    let mut r = None;
    let (scene, out) = h.frame(|ui| {
        ui.with_enabled(s.enabled, |ui| {
            r = Some(ui.diff_view("d", DIFF, lines, &opts));
        });
        // The queries ride in the frame that drew: a frame without the widget would drop its hit
        // regions, and the next press would land on nothing.
        let keys = ["u", "l", "r"].map(|k| {
            ui.scope("d", |ui| {
                ui.list_state(k.to_owned())
                    .selection
                    .iter()
                    .map(|k| k.0)
                    .collect::<Vec<u64>>()
            })
        });
        let text = ui.diff_selected_text("d", lines, opts.mode);
        SEEN.with(|c| *c.borrow_mut() = (keys, text));
    });
    Run {
        r: r.expect("the frame ran"),
        out,
        scene,
    }
}

fn run(h: &mut UiHarness, lines: &[DiffLine<'_>], s: &Setup) -> Run {
    show(h, lines, &pair_rows(lines), s)
}

fn harness() -> UiHarness {
    UiHarness::new(WIN)
}

fn scaled() -> UiHarness {
    let mut h = harness();
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    h
}

/// Frames until the page settles.
fn settle(h: &mut UiHarness, lines: &[DiffLine<'_>], s: &Setup, ms: &mut u64) -> Run {
    for _ in 0..40 {
        *ms += 400;
        h.at_ms(*ms);
        let r = run(h, lines, s);
        if r.out.repaint == Repaint::Idle {
            return r;
        }
    }
    panic!("the page did not settle");
}

fn root() -> WidgetId {
    WidgetId::ROOT.with("d")
}

fn pane(key: &str) -> WidgetId {
    root().with(key.to_owned())
}

fn row_id(key: &str, k: u64) -> WidgetId {
    pane(key).with(RowKey(k))
}

/// The pointer on the text of unified row `i` while the offset is 0.
fn over_row(i: usize) -> (f32, f32) {
    (DIFF.x + 300.0, DIFF.y + (i as f32 + 0.5) * ROW)
}

/// The pointer on the text of row `i` of the left pane (0) or the right (1).
fn over_pane(side: usize, i: usize) -> (f32, f32) {
    let w = ((DIFF.w - 1.0) / 2.0).floor();
    let x = DIFF.x + side as f32 * (w + 1.0) + 150.0;
    (x, DIFF.y + (i as f32 + 0.5) * ROW)
}

fn click(h: &mut UiHarness, at: (f32, f32), m: Modifiers) {
    h.input.pointer_to(at.0, at.1).set_modifiers(m).click(P);
}

fn key(h: &mut UiHarness, k: NamedKey, m: Modifiers) {
    h.input.set_modifiers(m).key_down(Key::Named(k), None);
}

fn tree_nodes(t: &SemanticTree, role: Role) -> Vec<lune_ui_core::access::Node> {
    t.nodes.iter().filter(|n| n.role == role).cloned().collect()
}

/// The selected row keys of pane `key` as the last frame ended.
fn selected(_h: &mut UiHarness, key: &str) -> Vec<u64> {
    let at = ["u", "l", "r"]
        .iter()
        .position(|k| *k == key)
        .expect("a pane");
    SEEN.with(|c| c.borrow().0[at].clone())
}

/// [`Ui::diff_selected_text`] as the last frame ended.
fn selected_text() -> String {
    SEEN.with(|c| c.borrow().1.clone())
}

fn written(out: &FrameOutput) -> Vec<String> {
    out.clipboard
        .iter()
        .filter_map(|c| match c {
            ClipboardRequest::Write(t) => Some(t.clone()),
            ClipboardRequest::Read => None,
        })
        .collect()
}

fn copy(h: &mut UiHarness, lines: &[DiffLine<'_>], s: &Setup) -> Vec<String> {
    h.input
        .set_modifiers(META)
        .key_down(Key::Character("c".into()), Some("c"));
    let r = run(h, lines, s);
    h.input.set_modifiers(NONE);
    written(&r.out)
}

fn texts(scene: &Scene) -> Vec<lune_ui_core::scene::TextItem> {
    scene.layers.iter().flat_map(|l| l.texts.clone()).collect()
}

fn under(scene: &Scene) -> Vec<Shape> {
    scene
        .layers
        .iter()
        .flat_map(|l| l.geometry.under.clone())
        .collect()
}

fn same_color(a: [f32; 4], b: [f32; 4]) -> bool {
    (0..4).all(|i| (a[i] - b[i]).abs() < 1e-6)
}

fn fills_of(scene: &Scene, color: [u8; 4]) -> Vec<Shape> {
    let want = premul(color);
    under(scene)
        .into_iter()
        .filter(|s| same_color(s.color, want))
        .collect()
}

fn code_style(color: [u8; 4]) -> Style {
    Style::mono(DARK.font_base, 20.0, color)
}

fn find(scene: &Scene, spec: &Spec, scale: f32) -> Option<lune_ui_core::scene::TextItem> {
    let key = spec.cache_key(scale);
    texts(scene).into_iter().find(|t| t.key.0 == key)
}

// The pairing.

fn kinds(k: &[LineKind]) -> Vec<DiffLine<'static>> {
    k.iter().map(|k| DiffLine::new(*k, "x")).collect()
}

fn rows_of(pairs: &[(Option<usize>, Option<usize>)]) -> Vec<SplitRow> {
    pairs
        .iter()
        .map(|&(left, right)| SplitRow { left, right })
        .collect()
}

#[test]
fn a_removed_run_and_the_added_run_after_it_pair_in_order_the_rest_against_none() {
    use LineKind::*;
    let got = pair_rows(&kinds(&[Removed, Removed, Added, Context]));
    assert_eq!(
        got,
        rows_of(&[(Some(0), Some(2)), (Some(1), None), (Some(3), Some(3))])
    );
    // The added run is the longer.
    let got = pair_rows(&kinds(&[Removed, Added, Added]));
    assert_eq!(got, rows_of(&[(Some(0), Some(1)), (None, Some(2))]));
    // An added run with nothing removed before it; a removed run with nothing added after.
    let got = pair_rows(&kinds(&[Context, Added, Context, Removed, Context]));
    assert_eq!(
        got,
        rows_of(&[
            (Some(0), Some(0)),
            (None, Some(1)),
            (Some(2), Some(2)),
            (Some(3), None),
            (Some(4), Some(4)),
        ])
    );
}

#[test]
fn hunk_and_context_rows_pair_with_themselves_and_every_line_is_used_once_per_side() {
    let lines = doc();
    let rows = pair_rows(&lines);
    for (i, l) in lines.iter().enumerate() {
        if matches!(l.kind, LineKind::Hunk | LineKind::Context) {
            assert!(rows.contains(&SplitRow {
                left: Some(i),
                right: Some(i)
            }));
        }
    }
    let lefts: Vec<usize> = rows.iter().filter_map(|r| r.left).collect();
    let rights: Vec<usize> = rows.iter().filter_map(|r| r.right).collect();
    let want_left: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].kind != LineKind::Added)
        .collect();
    let want_right: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].kind != LineKind::Removed)
        .collect();
    assert_eq!(lefts, want_left);
    assert_eq!(rights, want_right);
    assert!(pair_rows(&[]).is_empty());
}

#[test]
fn widest_chars_counts_chars_not_bytes() {
    assert_eq!(widest_chars(&[]), 0);
    let lines = [
        DiffLine::new(LineKind::Context, "abc"),
        DiffLine::new(LineKind::Added, "\u{e9}\u{e9}\u{e9}\u{e9}"),
    ];
    assert_eq!(widest_chars(&lines), 4);
    assert_eq!(widest_chars(&doc()), "@@ -20,3 +20,3 @@".len());
}

#[test]
fn pairing_a_hundred_thousand_lines_is_one_pass() {
    let lines = big(100_000);
    let rows = pair_rows(&lines);
    assert_eq!(rows.len(), 100_000);
    assert_eq!(
        rows[99_999],
        SplitRow {
            left: Some(99_999),
            right: Some(99_999)
        }
    );
}

// Unified: selection, keys, copy.

#[test]
fn click_selects_one_row_meta_toggles_and_shift_selects_from_the_anchor() {
    let lines = doc();
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(1), NONE);
    let r = run(&mut h, &lines, &s);
    assert!(r.r.changed);
    assert_eq!(selected(&mut h, "u"), [1]);

    click(&mut h, over_row(3), META);
    run(&mut h, &lines, &s);
    assert_eq!(selected(&mut h, "u"), [1, 3]);
    click(&mut h, over_row(3), META);
    run(&mut h, &lines, &s);
    assert_eq!(selected(&mut h, "u"), [1], "toggled off");

    click(&mut h, over_row(2), META);
    run(&mut h, &lines, &s);
    click(&mut h, over_row(5), SHIFT);
    run(&mut h, &lines, &s);
    assert_eq!(
        selected(&mut h, "u"),
        [2, 3, 4, 5],
        "the range from the anchor"
    );
}

#[test]
fn a_drag_selects_the_range_under_the_pointer() {
    let lines = doc();
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    let (x, y) = over_row(2);
    h.input.pointer_to(x, y).press(P);
    run(&mut h, &lines, &s);
    h.input.pointer_to(x, over_row(4).1);
    run(&mut h, &lines, &s);
    assert_eq!(selected(&mut h, "u"), [2, 3, 4]);
}

#[test]
fn the_copy_shortcut_writes_the_selected_lines_in_row_order_in_one_request() {
    let lines = doc();
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    // Select out of order: 5, then 3 (meta), then 4 (meta).
    click(&mut h, over_row(5), NONE);
    run(&mut h, &lines, &s);
    for i in [3, 4] {
        click(&mut h, over_row(i), META);
        run(&mut h, &lines, &s);
    }
    assert_eq!(
        copy(&mut h, &lines, &s),
        ["gamma old\ndelta old\ngamma new".to_owned()]
    );
}

#[test]
fn copy_includes_hunk_text_and_trims_the_end_only() {
    let lines = [
        DiffLine::new(LineKind::Hunk, "@@ -1 +1 @@"),
        DiffLine::new(LineKind::Context, "  indented"),
        DiffLine::new(LineKind::Context, "tail   "),
        DiffLine::new(LineKind::Context, ""),
    ];
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(0), NONE);
    run(&mut h, &lines, &s);
    click(&mut h, over_row(3), SHIFT);
    run(&mut h, &lines, &s);
    assert_eq!(
        copy(&mut h, &lines, &s),
        ["@@ -1 +1 @@\n  indented\ntail".to_owned()],
        "the blank end and the trailing spaces go, the indent stays"
    );
}

#[test]
fn copy_with_nothing_selected_writes_nothing_and_the_menu_text_is_empty() {
    let lines = doc();
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(1), NONE);
    run(&mut h, &lines, &s);
    click(&mut h, over_row(1), META);
    run(&mut h, &lines, &s);
    assert!(copy(&mut h, &lines, &s).is_empty());
    assert_eq!(selected_text(), "");
}

#[test]
fn the_selected_text_query_is_what_the_shortcut_writes() {
    let lines = doc();
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(3), NONE);
    run(&mut h, &lines, &s);
    click(&mut h, over_row(6), SHIFT);
    run(&mut h, &lines, &s);
    let text = selected_text();
    assert_eq!(text, "gamma old\ndelta old\ngamma new\nepsilon");
    assert_eq!(copy(&mut h, &lines, &s), [text]);
}

#[test]
fn the_keys_move_the_cursor_and_extend_with_shift() {
    let lines = big(60);
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(2), NONE);
    run(&mut h, &lines, &s);
    key(&mut h, NamedKey::ArrowDown, NONE);
    run(&mut h, &lines, &s);
    assert_eq!(selected(&mut h, "u"), [3]);
    key(&mut h, NamedKey::ArrowDown, SHIFT);
    run(&mut h, &lines, &s);
    h.input.set_modifiers(NONE);
    assert_eq!(selected(&mut h, "u"), [3, 4]);
    key(&mut h, NamedKey::End, NONE);
    let r = run(&mut h, &lines, &s);
    assert_eq!(r.r.response.id, pane("u"));
    assert_eq!(selected(&mut h, "u"), [59]);
}

#[test]
fn a_secondary_click_reports_the_line_and_selects_nothing() {
    let lines = doc();
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    let (x, y) = over_row(4);
    h.input.pointer_to(x, y).click(PointerButton::Secondary);
    let r = run(&mut h, &lines, &s);
    assert_eq!(r.r.context, Some(4));
    assert!(!r.r.changed);
    assert_eq!(selected(&mut h, "u"), Vec::<u64>::new());
}

// Split: panes, the clear, copy.

#[test]
fn split_rows_show_the_halves_and_a_press_in_one_pane_clears_the_other() {
    let lines = doc();
    let rows = pair_rows(&lines);
    let s = split();
    let mut h = harness();
    show(&mut h, &lines, &rows, &s);
    click(&mut h, over_pane(0, 2), NONE);
    show(&mut h, &lines, &rows, &s);
    click(&mut h, over_pane(0, 3), SHIFT);
    show(&mut h, &lines, &rows, &s);
    assert_eq!(selected(&mut h, "l"), [2, 3]);

    click(&mut h, over_pane(1, 4), NONE);
    let r = show(&mut h, &lines, &rows, &s);
    assert!(r.r.changed);
    assert_eq!(selected(&mut h, "r"), [4]);
    assert_eq!(
        selected(&mut h, "l"),
        Vec::<u64>::new(),
        "the first pane cleared"
    );

    // And back: the right pane clears when the left is used.
    click(&mut h, over_pane(0, 1), NONE);
    show(&mut h, &lines, &rows, &s);
    assert_eq!(selected(&mut h, "l"), [1]);
    assert_eq!(selected(&mut h, "r"), Vec::<u64>::new());
}

#[test]
fn split_copy_writes_the_active_panes_lines_and_an_empty_half_adds_nothing() {
    use LineKind::*;
    let lines = vec![
        line(Removed, Some(1), None, "one"),
        line(Removed, Some(2), None, "two"),
        line(Added, None, Some(1), "uno"),
        line(Context, Some(3), Some(2), "tres"),
    ];
    let rows = pair_rows(&lines);
    let s = split();
    let mut h = harness();
    show(&mut h, &lines, &rows, &s);
    // The left pane, rows 0 to 2 (row 1's right half is empty, row 0 pairs one/uno).
    click(&mut h, over_pane(0, 0), NONE);
    show(&mut h, &lines, &rows, &s);
    click(&mut h, over_pane(0, 2), SHIFT);
    show(&mut h, &lines, &rows, &s);
    h.input
        .set_modifiers(META)
        .key_down(Key::Character("c".into()), Some("c"));
    let r = show(&mut h, &lines, &rows, &s);
    h.input.set_modifiers(NONE);
    assert_eq!(written(&r.out), ["one\ntwo\ntres".to_owned()]);

    // The right pane, the same rows: the empty half (row 1) adds nothing.
    click(&mut h, over_pane(1, 0), NONE);
    show(&mut h, &lines, &rows, &s);
    click(&mut h, over_pane(1, 2), SHIFT);
    show(&mut h, &lines, &rows, &s);
    h.input
        .set_modifiers(META)
        .key_down(Key::Character("c".into()), Some("c"));
    let r = show(&mut h, &lines, &rows, &s);
    h.input.set_modifiers(NONE);
    assert_eq!(written(&r.out), ["uno\ntres".to_owned()]);
    assert_eq!(selected_text(), "uno\ntres");
}

#[test]
fn a_secondary_click_in_split_names_the_pane_line_and_none_for_an_empty_half() {
    use LineKind::*;
    let lines = vec![
        line(Removed, Some(1), None, "one"),
        line(Removed, Some(2), None, "two"),
        line(Added, None, Some(1), "uno"),
    ];
    let rows = pair_rows(&lines);
    let s = split();
    let mut h = harness();
    show(&mut h, &lines, &rows, &s);
    for (side, row, want) in [(0, 1, Some(1)), (1, 0, Some(2)), (1, 1, None)] {
        let (x, y) = over_pane(side, row);
        h.input.pointer_to(x, y).click(PointerButton::Secondary);
        let r = show(&mut h, &lines, &rows, &s);
        assert_eq!(r.r.context, want, "pane {side} row {row}");
    }
}

#[test]
fn split_panes_are_equal_with_a_one_pixel_separator() {
    let lines = doc();
    let rows = pair_rows(&lines);
    let mut h = scaled();
    let r = show(&mut h, &lines, &rows, &split());
    let tables = tree_nodes(&r.out.tree, Role::Table);
    assert_eq!(tables.len(), 2);
    let w = ((DIFF.w - 1.0) / 2.0).floor();
    assert_eq!(tables[0].bounds, Rect::new(DIFF.x, DIFF.y, w, DIFF.h));
    assert_eq!(
        tables[1].bounds,
        Rect::new(DIFF.x + w + 1.0, DIFF.y, DIFF.w - w - 1.0, DIFF.h)
    );
    let sep = fills_of(&r.scene, DARK.surface_separator)
        .into_iter()
        .find(|s| s.rect[3] == DIFF.h * 2.0)
        .expect("the separator");
    assert_eq!(
        sep.rect,
        [(DIFF.x + w) * 2.0, DIFF.y * 2.0, 2.0, DIFF.h * 2.0]
    );
}

// Hunk actions.

fn buttons(r: &Run) -> Vec<lune_ui_core::access::Node> {
    tree_nodes(&r.out.tree, Role::Button)
}

fn centre(r: Rect) -> (f32, f32) {
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

#[test]
fn a_press_on_a_hunk_button_reports_the_hunk_and_action_and_selects_no_row() {
    let lines = doc();
    let s = Setup {
        actions: 2,
        ..setup()
    };
    let mut h = harness();
    let r = run(&mut h, &lines, &s);
    let nodes = buttons(&r);
    assert_eq!(
        nodes.len(),
        4,
        "two actions on each of two visible hunk rows"
    );
    // The second hunk (row 7), its second action.
    let target = nodes
        .iter()
        .filter(|n| n.name == "Unstage hunk")
        .nth(1)
        .expect("a button");
    assert_eq!(target.bounds.w, 18.0);
    assert_eq!(target.bounds.h, 18.0);
    let (x, y) = centre(target.bounds);
    h.input.pointer_to(x, y);
    run(&mut h, &lines, &s);
    h.input.click(P);
    let r = run(&mut h, &lines, &s);
    assert_eq!(r.r.hunk, Some((1, 1)));
    assert!(!r.r.changed, "no row selected");
    assert_eq!(selected(&mut h, "u"), Vec::<u64>::new());
    let r = run(&mut h, &lines, &s);
    assert_eq!(r.r.hunk, None, "reported on the frame it happened");
}

#[test]
fn the_default_action_of_a_hunk_button_invokes_it() {
    let lines = doc();
    let s = Setup {
        actions: 1,
        ..setup()
    };
    let mut h = harness();
    let r = run(&mut h, &lines, &s);
    let id = buttons(&r)[0].id;
    h.input.access(AccessAction {
        target: id,
        action: Action::Default,
        value: None,
    });
    let r = run(&mut h, &lines, &s);
    assert_eq!(r.r.hunk, Some((0, 0)));
}

#[test]
fn without_actions_there_are_no_buttons_and_no_extra_tab_stops() {
    let lines = doc();
    let mut h = harness();
    let r = run(&mut h, &lines, &setup());
    assert!(buttons(&r).is_empty());
    // The body is the only stop: Tab reaches it and the next Tab comes round to it again.
    for _ in 0..2 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        let r = run(&mut h, &lines, &setup());
        assert_eq!(r.out.focused, Some(pane("u")));
    }
}

#[test]
fn a_hunk_line_without_an_index_gets_no_buttons() {
    let mut lines = doc();
    lines[0].hunk = None;
    let s = Setup {
        actions: 1,
        ..setup()
    };
    let mut h = harness();
    let r = run(&mut h, &lines, &s);
    assert_eq!(buttons(&r).len(), 1, "only the second hunk's");
}

#[test]
fn tab_visits_the_body_then_each_visible_action_in_call_order() {
    let lines = doc();
    let s = Setup {
        actions: 2,
        ..setup()
    };
    let mut h = harness();
    let r = run(&mut h, &lines, &s);
    let order: Vec<WidgetId> = buttons(&r).iter().map(|n| n.id).collect();
    let mut seen = Vec::new();
    for _ in 0..5 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        let r = run(&mut h, &lines, &s);
        seen.push(r.out.focused);
    }
    assert_eq!(seen[0], Some(pane("u")), "the body first");
    let rest: Vec<WidgetId> = seen[1..=4].iter().map(|f| f.expect("a stop")).collect();
    assert_eq!(rest, order);
}

#[test]
fn split_has_two_body_stops_and_the_actions_of_both_panes() {
    let lines = doc();
    let rows = pair_rows(&lines);
    let s = Setup {
        actions: 1,
        ..split()
    };
    let mut h = harness();
    let r = show(&mut h, &lines, &rows, &s);
    assert_eq!(
        buttons(&r).len(),
        4,
        "a button on each pane's two hunk rows"
    );
    let mut seen = Vec::new();
    for _ in 0..3 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        seen.push(show(&mut h, &lines, &rows, &s).out.focused);
    }
    assert_eq!(seen[0], Some(pane("l")));
    assert!(seen[1].is_some_and(|id| id != pane("l")));
    let _ = seen[2];
    // The right pane's body is a stop of its own.
    let mut stops = vec![];
    for _ in 0..8 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        stops.push(show(&mut h, &lines, &rows, &s).out.focused);
    }
    assert!(stops.contains(&Some(pane("r"))));
}

#[test]
fn a_disabled_diff_selects_nothing_and_its_actions_report_nothing() {
    let lines = doc();
    let s = Setup {
        actions: 1,
        enabled: false,
        ..setup()
    };
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(2), NONE);
    let r = run(&mut h, &lines, &s);
    assert!(!r.r.changed);
    for n in buttons(&r) {
        let (x, y) = centre(n.bounds);
        click(&mut h, (x, y), NONE);
        let r = run(&mut h, &lines, &s);
        assert_eq!(r.r.hunk, None);
    }
    let on = Setup { enabled: true, ..s };
    assert_eq!(selected(&mut h, "u"), Vec::<u64>::new());
    run(&mut h, &lines, &on);
}

// Preservation.

#[test]
fn the_same_identity_keeps_the_offset_and_the_selection_a_new_one_starts_at_the_top() {
    let lines = big(200);
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(2), NONE);
    run(&mut h, &lines, &s);
    h.input.scroll_pixels(0.0, -400.0);
    let r = run(&mut h, &lines, &s);
    assert_eq!(r.out.repaint, Repaint::Now);
    let mut ms = 0;
    settle(&mut h, &lines, &s, &mut ms);
    // The app replaces the lines with shorter ones under the same key: the offset is clamped.
    let fewer = big(30);
    let r = run(&mut h, &fewer, &s);
    let tables = tree_nodes(&r.out.tree, Role::Table);
    assert_eq!(tables.len(), 1);
    assert_eq!(selected(&mut h, "u"), [2], "the selection is kept");
    let first_built = tree_nodes(&r.out.tree, Role::Row)
        .into_iter()
        .map(|n| n.bounds.y)
        .fold(f32::MAX, f32::min);
    assert!(first_built <= DIFF.y, "scrolled, not blank: {first_built}");
    let again = run(&mut h, &lines, &s);
    let _ = again;

    // A new document key starts at the top with none selected.
    let other = Setup { identity: 2, ..s };
    let r = run(&mut h, &lines, &other);
    let top = tree_nodes(&r.out.tree, Role::Row)
        .into_iter()
        .map(|n| n.bounds.y)
        .fold(f32::MAX, f32::min);
    assert_eq!(top, DIFF.y);
    assert_eq!(selected(&mut h, "u"), Vec::<u64>::new());
}

// Marks.

fn marked(
    text: &'static str,
    marks: &'static [DiffMark],
    kind: LineKind,
) -> Vec<DiffLine<'static>> {
    vec![DiffLine {
        marks,
        ..line(kind, Some(1), Some(1), text)
    }]
}

/// Mono 13 at half a char of size in the fake: 6.5 per char.
const CHAR_W: f32 = 6.5;

fn mark_boxes(scene: &Scene, kind: LineKind) -> Vec<Shape> {
    let tint = if kind == LineKind::Added {
        DARK.diff_added_mark
    } else {
        DARK.diff_removed_mark
    };
    fills_of(scene, tint)
}

#[test]
fn a_mark_past_the_line_end_is_clamped_and_empty_or_inverted_ones_add_nothing() {
    const MARKS: &[DiffMark] = &[DiffMark { start: 3, end: 99 }];
    let lines = marked("0123456789", MARKS, LineKind::Added);
    let mut h = scaled();
    let r = run(&mut h, &lines, &setup());
    let boxes = mark_boxes(&r.scene, LineKind::Added);
    assert_eq!(boxes.len(), 1, "{boxes:?}");
    let text_x = DIFF.x + 2.0 * 48.0 + 18.0;
    assert_eq!(boxes[0].rect[0], (text_x + 3.0 * CHAR_W) * 2.0);
    assert_eq!(boxes[0].rect[2], 7.0 * CHAR_W * 2.0, "3 to 10");

    for marks in [
        &[DiffMark { start: 5, end: 5 }][..],
        &[DiffMark { start: 7, end: 2 }],
        &[DiffMark { start: 20, end: 30 }],
        &[],
    ] {
        let lines = marked("0123456789", marks, LineKind::Added);
        let r = run(&mut harness(), &lines, &setup());
        assert!(
            mark_boxes(&r.scene, LineKind::Added).is_empty(),
            "{marks:?}"
        );
    }
}

#[test]
fn the_kind_picks_the_tint_and_context_and_hunk_lines_take_none() {
    const MARKS: &[DiffMark] = &[DiffMark { start: 1, end: 4 }];
    for (kind, added, removed) in [
        (LineKind::Added, 1, 0),
        (LineKind::Removed, 0, 1),
        (LineKind::Context, 0, 0),
        (LineKind::Hunk, 0, 0),
    ] {
        let r = run(&mut harness(), &marked("0123456789", MARKS, kind), &setup());
        assert_eq!(
            mark_boxes(&r.scene, LineKind::Added).len(),
            added,
            "{kind:?}"
        );
        assert_eq!(
            mark_boxes(&r.scene, LineKind::Removed).len(),
            removed,
            "{kind:?}"
        );
    }
}

#[test]
fn a_wrapped_mark_is_one_box_per_row_it_touches_and_marks_never_panic() {
    const MARKS: &[DiffMark] = &[DiffMark { start: 2, end: 60 }];
    let text: &'static str =
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
    let lines = marked(text, MARKS, LineKind::Removed);
    let s = Setup {
        wrap: true,
        ..setup()
    };
    let mut h = harness();
    run(&mut h, &lines, &s);
    let r = run(&mut h, &lines, &s);
    let boxes = mark_boxes(&r.scene, LineKind::Removed);
    // 78 chars at 6.5 is 507; the text column is 600 - 114 - 8 = 478: two rows, both touched.
    assert_eq!(boxes.len(), 2, "{boxes:?}");
    // Odd marks over odd text: nothing panics.
    const ODD: &[DiffMark] = &[
        DiffMark { start: 0, end: 0 },
        DiffMark {
            start: usize::MAX,
            end: usize::MAX,
        },
        DiffMark {
            start: 1,
            end: usize::MAX,
        },
        DiffMark {
            start: usize::MAX,
            end: 0,
        },
    ];
    for text in ["", "a", "\u{e9}\u{1f600}x"] {
        let lines = marked(text, ODD, LineKind::Added);
        for wrap in [false, true] {
            run(&mut harness(), &lines, &Setup { wrap, ..setup() });
        }
    }
}

#[test]
fn marks_draw_under_the_selection_and_the_text() {
    const MARKS: &[DiffMark] = &[DiffMark { start: 0, end: 4 }];
    let lines = marked("0123456789", MARKS, LineKind::Added);
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(0), NONE);
    run(&mut h, &lines, &s);
    let r = run(&mut h, &lines, &s);
    let order: Vec<[f32; 4]> = under(&r.scene).into_iter().map(|s| s.color).collect();
    let pos = |c: [u8; 4]| order.iter().position(|o| same_color(*o, premul(c)));
    let accent = [DARK.accent[0], DARK.accent[1], DARK.accent[2], 0x33];
    let (fill, mark, sel) = (pos(DARK.diff_added), pos(DARK.diff_added_mark), pos(accent));
    assert!(fill < mark, "fill then marks: {fill:?} {mark:?}");
    assert!(mark < sel, "marks then selection: {mark:?} {sel:?}");
    assert!(sel.is_some());
}

// The rows' look.

#[test]
fn rows_take_their_fills_text_colours_prefixes_and_numbers() {
    let lines = doc();
    let mut h = scaled();
    let r = run(&mut h, &lines, &setup());
    assert_eq!(fills_of(&r.scene, DARK.diff_added).len(), 2);
    assert_eq!(fills_of(&r.scene, DARK.diff_removed).len(), 3);
    for (text, color) in [
        ("alpha", DARK.fg_primary),
        ("gamma old", DARK.err),
        ("gamma new", DARK.ok),
    ] {
        let spec = Spec::plain(text, code_style(color));
        assert!(find(&r.scene, &spec, 2.0).is_some(), "{text}");
    }
    for (prefix, color) in [("+", DARK.ok), ("-", DARK.err), (" ", DARK.fg_primary)] {
        let spec = Spec::plain(prefix, code_style(color));
        assert!(find(&r.scene, &spec, 2.0).is_some(), "prefix {prefix:?}");
    }
    let faint = |n: &str| find(&r.scene, &Spec::plain(n, code_style(DARK.fg_faint)), 2.0);
    let old = faint("21").expect("an old number");
    let hunk = Style::mono(9.0, 12.0, DARK.link);
    assert!(
        find(&r.scene, &Spec::plain("@@ -1,4 +1,4 @@", hunk), 2.0).is_some(),
        "the hunk's text is the link colour"
    );
    // Numbers are right-aligned in a 48 column with 8 to the right: "21" is 13 wide.
    assert_eq!(old.x, DIFF.x + 48.0 - 8.0 - 13.0);
    let new = faint("3").expect("a new number");
    let _ = new;
}

#[test]
fn a_row_has_no_hover_and_the_selection_is_the_accent_at_0x33() {
    let lines = big(5);
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    h.input.pointer_to(over_row(1).0, over_row(1).1);
    let mut ms = 0;
    let r = settle(&mut h, &lines, &s, &mut ms);
    assert!(
        fills_of(&r.scene, DARK.surface_hover).is_empty(),
        "no hover fill"
    );
    click(&mut h, over_row(1), NONE);
    run(&mut h, &lines, &s);
    let r = settle(&mut h, &lines, &s, &mut ms);
    let accent = [DARK.accent[0], DARK.accent[1], DARK.accent[2], 0x33];
    let sel = fills_of(&r.scene, accent);
    assert_eq!(sel.len(), 1);
    assert_eq!(sel[0].rect, [DIFF.x, DIFF.y + ROW, DIFF.w, ROW]);
    assert!(fills_of(&r.scene, DARK.surface_selected).is_empty());
    assert!(fills_of(&r.scene, DARK.surface_active).is_empty());
}

// Wrap and the horizontal offset.

fn long_line() -> Vec<DiffLine<'static>> {
    use LineKind::*;
    let long: &'static str = Box::leak("x".repeat(300).into_boxed_str());
    vec![
        line(Context, Some(1), Some(1), "short"),
        line(Added, None, Some(2), long),
        line(Context, Some(2), Some(3), "after"),
    ]
}

#[test]
fn wrap_off_scrolls_sideways_with_the_numbers_and_prefix_staying_put() {
    let lines = long_line();
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    h.input.pointer_to(over_row(1).0, over_row(1).1);
    run(&mut h, &lines, &s);
    let before = run(&mut h, &lines, &s);
    h.input.scroll_pixels(-120.0, 0.0);
    run(&mut h, &lines, &s);
    let mut ms = 0;
    let after = settle(&mut h, &lines, &s, &mut ms);
    let num = Spec::plain("2", code_style(DARK.fg_faint));
    let pre = Spec::plain("+", code_style(DARK.ok));
    let text = Spec::plain("x".repeat(300), code_style(DARK.ok));
    let x_of = |r: &Run, spec: &Spec| find(&r.scene, spec, 1.0).map(|t| t.x);
    assert_eq!(x_of(&before, &num), x_of(&after, &num), "the number stays");
    assert_eq!(x_of(&before, &pre), x_of(&after, &pre), "the prefix stays");
    let (a, b) = (x_of(&before, &text), x_of(&after, &text));
    assert_eq!(a, Some(DIFF.x + 114.0));
    assert_eq!(b, Some(DIFF.x + 114.0 - 120.0), "the text moved");
    // The text is clipped to its column, left of which the numbers sit.
    let item = find(&after.scene, &text, 1.0).expect("the long line");
    assert!(item.clip.x >= DIFF.x + 114.0 - 1.0, "{:?}", item.clip);
}

#[test]
fn wrap_on_has_no_sideways_scroll_and_rows_grow() {
    let lines = long_line();
    let s = Setup {
        wrap: true,
        ..setup()
    };
    let mut h = harness();
    run(&mut h, &lines, &s);
    h.input.pointer_to(over_row(1).0, over_row(1).1);
    h.input.scroll_pixels(-120.0, 0.0);
    run(&mut h, &lines, &s);
    let mut ms = 0;
    let r = settle(&mut h, &lines, &s, &mut ms);
    let rows: Vec<_> = tree_nodes(&r.out.tree, Role::Row);
    assert_eq!(rows.len(), 3);
    assert!(rows[1].bounds.h > ROW, "300 chars wrap in a 478 column");
    assert_eq!(rows[0].bounds.h, ROW);
    assert_eq!(rows[2].bounds.y, rows[1].bounds.y + rows[1].bounds.h);
    let text = Spec::plain("x".repeat(300), code_style(DARK.ok)).wrapped(478.0);
    assert_eq!(
        find(&r.scene, &text, 1.0).map(|t| t.x),
        Some(DIFF.x + 114.0),
        "not moved"
    );
}

#[test]
fn the_horizontal_extent_is_the_widest_chars_by_the_advance() {
    // 300 chars at 6.5 plus the 114 gutter and 8 margin: 2072 wide, so the wheel can reach 1472.
    let lines = long_line();
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    h.input.pointer_to(over_row(1).0, over_row(1).1);
    h.input.scroll_pixels(-5000.0, 0.0);
    run(&mut h, &lines, &s);
    let mut ms = 0;
    let r = settle(&mut h, &lines, &s, &mut ms);
    let text = Spec::plain("x".repeat(300), code_style(DARK.ok));
    let x = find(&r.scene, &text, 1.0).expect("the long line").x;
    assert_eq!(x, DIFF.x + 114.0 - (300.0 * CHAR_W + 114.0 + 8.0 - DIFF.w));
}

// Wrapped split: the rows stay aligned.

#[test]
fn a_long_left_half_moves_the_next_right_row_down_equally() {
    use LineKind::*;
    let long: &'static str = Box::leak("w".repeat(200).into_boxed_str());
    let lines = vec![
        line(Context, Some(1), Some(1), "one"),
        line(Removed, Some(2), None, long),
        line(Added, None, Some(2), "short"),
        line(Context, Some(3), Some(3), "three"),
    ];
    let rows = pair_rows(&lines);
    let s = Setup {
        wrap: true,
        ..split()
    };
    let mut h = harness();
    show(&mut h, &lines, &rows, &s);
    let mut ms = 0;
    let mut r = show(&mut h, &lines, &rows, &s);
    for _ in 0..10 {
        ms += 400;
        h.at_ms(ms);
        r = show(&mut h, &lines, &rows, &s);
    }
    let node = |side: &str, k: u64| r.out.tree.get(row_id(side, k)).expect("a built row").bounds;
    for k in 0..3 {
        assert_eq!(node("l", k).y, node("r", k).y, "row {k} top");
        assert_eq!(node("l", k).h, node("r", k).h, "row {k} height");
    }
    assert!(
        node("l", 1).h > ROW,
        "the long half wraps: {}",
        node("l", 1).h
    );
    assert!(
        node("r", 2).y > node("r", 0).y + 2.0 * ROW,
        "row 2 moved down"
    );
    assert_eq!(r.out.repaint, Repaint::Idle);
}

// The linked scroll.

fn first_row_y(r: &Run, side: &str) -> f32 {
    let t = r.out.tree.get(pane(side)).expect("a pane");
    t.children
        .iter()
        .filter_map(|c| r.out.tree.get(*c))
        .map(|n| n.bounds.y)
        .fold(f32::MAX, f32::min)
}

#[test]
fn scrolling_the_left_pane_puts_the_right_at_the_same_offset_with_no_echo() {
    let lines = big(300);
    let rows = pair_rows(&lines);
    let s = split();
    let mut h = harness();
    show(&mut h, &lines, &rows, &s);
    let (x, y) = over_pane(0, 3);
    h.input.pointer_to(x, y);
    show(&mut h, &lines, &rows, &s);
    h.input.scroll_pixels(0.0, -120.0);
    let mut r = show(&mut h, &lines, &rows, &s);
    for ms in [400, 800, 1200, 1600] {
        h.at_ms(ms);
        r = show(&mut h, &lines, &rows, &s);
    }
    // Row 6 is the first fully visible at 120 px: both panes' row 6 sit at the same place.
    for side in ["l", "r"] {
        let n = r.out.tree.get(row_id(side, 6)).expect("row 6");
        assert_eq!(n.bounds.y, DIFF.y, "{side}");
    }
    assert_eq!(r.out.repaint, Repaint::Idle, "no echo, the page settles");
    let _ = first_row_y(&r, "l");
}

#[test]
fn scrolling_the_right_pane_moves_the_left_on_the_catch_up_frame() {
    let lines = big(300);
    let rows = pair_rows(&lines);
    let s = split();
    let mut h = harness();
    show(&mut h, &lines, &rows, &s);
    let (x, y) = over_pane(1, 3);
    h.input.pointer_to(x, y);
    show(&mut h, &lines, &rows, &s);
    h.input.scroll_pixels(0.0, -200.0);
    show(&mut h, &lines, &rows, &s);
    let mut r = show(&mut h, &lines, &rows, &s);
    for ms in [400, 800, 1200, 1600] {
        h.at_ms(ms);
        r = show(&mut h, &lines, &rows, &s);
    }
    for side in ["l", "r"] {
        let n = r.out.tree.get(row_id(side, 10)).expect("row 10");
        assert_eq!(n.bounds.y, DIFF.y, "{side}");
    }
    assert_eq!(r.out.repaint, Repaint::Idle);
}

#[test]
fn a_wrapped_split_scrolls_in_step_and_settles() {
    use LineKind::*;
    // Every third row is a long removed line paired with a short added one.
    let long: &'static str = Box::leak("w".repeat(200).into_boxed_str());
    let mut lines = Vec::new();
    for i in 0..60u32 {
        if i % 3 == 0 {
            lines.push(line(Removed, Some(i), None, long));
            lines.push(line(Added, None, Some(i), "short"));
        } else {
            lines.push(line(Context, Some(i), Some(i), "plain"));
        }
    }
    let rows = pair_rows(&lines);
    let s = Setup {
        wrap: true,
        ..split()
    };
    let mut h = harness();
    show(&mut h, &lines, &rows, &s);
    let (x, y) = over_pane(0, 2);
    h.input.pointer_to(x, y);
    show(&mut h, &lines, &rows, &s);
    let mut ms = 0;
    // Down in small steps, then jumps, then back up into rows nobody has measured: a height
    // learned above the first visible row shifts the offset in each pane.
    for (step, by) in [
        -150.0f32, -337.0, -3000.0, -1200.0, 900.0, 2500.0, -5000.0, 4200.0,
    ]
    .into_iter()
    .enumerate()
    {
        h.input.scroll_pixels(0.0, by);
        show(&mut h, &lines, &rows, &s);
        for _ in 0..6 {
            ms += 400;
            h.at_ms(ms);
            show(&mut h, &lines, &rows, &s);
        }
        ms += 400;
        h.at_ms(ms);
        let r = show(&mut h, &lines, &rows, &s);
        let ys = |side: &str| -> Vec<(u64, f32)> {
            (0..rows.len() as u64)
                .filter_map(|k| r.out.tree.get(row_id(side, k)).map(|n| (k, n.bounds.y)))
                .collect()
        };
        let (l, rr) = (ys("l"), ys("r"));
        let common: Vec<_> = l
            .iter()
            .filter_map(|(k, y)| {
                rr.iter()
                    .find(|(k2, _)| k2 == k)
                    .map(|(_, y2)| (*k, *y, *y2))
            })
            .collect();
        assert!(!common.is_empty());
        for (k, a, b) in common {
            assert_eq!(a, b, "step {step}: row {k} aligned");
        }
        assert_eq!(r.out.repaint, Repaint::Idle, "step {step} settles");
    }
}

// Scale.

fn built_rows(r: &Run) -> usize {
    tree_nodes(&r.out.tree, Role::Row).len()
}

#[test]
fn a_hundred_thousand_rows_build_only_the_window_in_both_modes() {
    let lines = big(100_000);
    let rows = pair_rows(&lines);
    for s in [
        setup(),
        split(),
        Setup {
            wrap: true,
            ..setup()
        },
    ] {
        let mut h = harness();
        show(&mut h, &lines, &rows, &s);
        h.input.pointer_to(over_row(3).0, over_row(3).1);
        h.input.scroll_pixels(0.0, -2_000_000.0);
        show(&mut h, &lines, &rows, &s);
        let mut ms = 0;
        let r = {
            ms += 400;
            h.at_ms(ms);
            show(&mut h, &lines, &rows, &s)
        };
        let panes = if s.split { 2 } else { 1 };
        let per_pane = built_rows(&r) / panes;
        assert!(per_pane <= 10 + 2 * 2 + 1, "built {per_pane} rows a pane");
        assert!(per_pane >= 10);
    }
}

// The tree.

#[test]
fn the_tree_is_a_table_of_rows_with_cells_descriptions_and_selected() {
    let lines = doc();
    let s = Setup {
        actions: 1,
        ..setup()
    };
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(3), NONE);
    run(&mut h, &lines, &s);
    let r = run(&mut h, &lines, &s);
    let tree = &r.out.tree;
    let table = tree.get(pane("u")).expect("the table");
    assert_eq!(table.role, Role::Table);
    assert_eq!(table.name, "Changes");
    assert_eq!(table.children.len(), lines.len());
    let row = |k: u64| tree.get(row_id("u", k)).expect("a row");
    assert_eq!(row(3).role, Role::Row);
    assert!(row(3).states.contains(State::Selected));
    assert!(!row(2).states.contains(State::Selected));
    assert_eq!(row(1).description.as_deref(), Some("Unchanged"));
    assert_eq!(row(3).description.as_deref(), Some("Removed"));
    assert_eq!(row(5).description.as_deref(), Some("Added"));
    assert_eq!(row(0).description, None, "a hunk row has none");
    let cells: Vec<_> = row(3)
        .children
        .iter()
        .filter_map(|c| tree.get(*c))
        .collect();
    assert_eq!(cells.len(), 2, "old number (no new one), text");
    assert!(cells.iter().all(|c| c.role == Role::Cell));
    assert_eq!(cells.last().map(|c| c.name.as_str()), Some("gamma old"));
    assert_eq!(cells[0].name, "3");
    let hunk_row = row(0);
    let hunk_kids: Vec<_> = hunk_row
        .children
        .iter()
        .filter_map(|c| tree.get(*c))
        .collect();
    assert!(
        hunk_kids
            .iter()
            .any(|c| c.role == Role::Cell && c.name == "@@ -1,4 +1,4 @@")
    );
    let button = hunk_kids
        .iter()
        .find(|c| c.role == Role::Button)
        .expect("the action");
    assert_eq!(button.name, "Stage hunk");
    assert!(button.actions.contains(Action::Default));
    assert!(button.states.contains(State::Focusable));
}

#[test]
fn a_disabled_diff_marks_its_whole_subtree() {
    let lines = doc();
    let s = Setup {
        actions: 2,
        enabled: false,
        ..setup()
    };
    let mut h = harness();
    let r = run(&mut h, &lines, &s);
    assert!(disabled_violations(&r.out.tree, pane("u")).is_empty());
    let rows = pair_rows(&lines);
    let s = Setup {
        actions: 2,
        enabled: false,
        ..split()
    };
    let mut h = harness();
    let r = show(&mut h, &lines, &rows, &s);
    for side in ["l", "r"] {
        assert!(disabled_violations(&r.out.tree, pane(side)).is_empty());
    }
    assert!(
        buttons(&r)
            .iter()
            .all(|b| b.states.contains(State::Disabled))
    );
}

// The look at scale 2.

#[test]
fn at_scale_2_the_row_is_40_the_number_column_96_the_prefix_20_and_the_fill_the_added_tint() {
    let lines = doc();
    let mut h = scaled();
    let r = run(&mut h, &lines, &setup());
    let rows = tree_nodes(&r.out.tree, Role::Row);
    assert!(rows.iter().all(|n| n.bounds.h * 2.0 >= 40.0));
    let added = fills_of(&r.scene, DARK.diff_added);
    assert_eq!(
        added[0].rect,
        [DIFF.x * 2.0, (DIFF.y + 5.0 * ROW) * 2.0, DIFF.w * 2.0, 40.0]
    );
    assert_eq!(added[0].color, premul(DARK.diff_added));

    // Number columns: 96 device px each pair; the second number's right edge is 16 from its column's.
    let cells: Vec<_> = r
        .out
        .tree
        .get(row_id("u", 1))
        .expect("a row")
        .children
        .iter()
        .filter_map(|c| r.out.tree.get(*c))
        .collect();
    assert_eq!(
        cells[0].bounds.w * 2.0,
        96.0,
        "a number column is 96 device px"
    );
    assert_eq!((cells[1].bounds.x - cells[0].bounds.x) * 2.0, 96.0);
    // The text starts after both numbers and the prefix with its margins: 2 * 96 + 8 + 20 + 8.
    assert_eq!((cells[2].bounds.x - DIFF.x) * 2.0, 228.0);
}

#[test]
fn at_scale_2_the_hunk_button_is_36_with_radius_10_and_the_pill_is_rounded_20() {
    let lines = doc();
    let s = Setup {
        actions: 1,
        ..setup()
    };
    let mut h = scaled();
    let r = run(&mut h, &lines, &s);
    let b = &buttons(&r)[0];
    assert_eq!((b.bounds.w * 2.0, b.bounds.h * 2.0), (36.0, 36.0));
    let overlay = fills_of(&r.scene, DARK.surface_overlay);
    let fill = overlay
        .iter()
        .find(|s| s.rect[2] == 36.0)
        .expect("the button's fill");
    assert_eq!(fill.radii, [10.0; 4]);
    let pill = fills_of(&r.scene, DARK.surface_selected);
    assert!(pill.iter().any(|s| s.radii[0] > 0.0));
}

// Motion.

#[test]
fn the_hover_and_press_transitions_conform_at_every_tier() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [(HOVER, "hover"), (PRESS, "press")] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
    }
}

#[test]
fn rows_never_animate_a_selection_change_is_there_in_its_frame() {
    let lines = doc();
    let s = setup();
    let mut h = harness();
    run(&mut h, &lines, &s);
    click(&mut h, over_row(2), NONE);
    let r = run(&mut h, &lines, &s);
    let accent = [DARK.accent[0], DARK.accent[1], DARK.accent[2], 0x33];
    let sel = fills_of(&r.scene, accent);
    assert_eq!(
        sel.len(),
        1,
        "the overlay at full alpha in the frame it appears"
    );
    assert_eq!(sel[0].color, premul(accent));
    let mut ms = 0;
    let settled = settle(&mut h, &lines, &s, &mut ms);
    assert_eq!(
        fills_of(&settled.scene, accent)[0].color,
        sel[0].color,
        "and it does not change after"
    );
    // A mode switch replaces the rows at once.
    let rows = pair_rows(&lines);
    let r = show(&mut h, &lines, &rows, &split());
    assert_eq!(tree_nodes(&r.out.tree, Role::Table).len(), 2);
    let r = run(&mut h, &lines, &setup());
    assert_eq!(tree_nodes(&r.out.tree, Role::Table).len(), 1);
}

#[test]
fn spans_colour_runs_of_a_line() {
    const SPANS: &[HighlightSpan] = &[HighlightSpan {
        start: 0,
        end: 3,
        color: [10, 20, 30, 255],
    }];
    let lines = [DiffLine {
        spans: SPANS,
        ..line(LineKind::Context, Some(1), Some(1), "abcdef")
    }];
    let mut h = harness();
    let r = run(&mut h, &lines, &setup());
    let want = Spec {
        runs: vec![
            ("abc".to_owned(), code_style([10, 20, 30, 255])),
            ("def".to_owned(), code_style(DARK.fg_primary)),
        ],
        wrap: None,
    };
    assert!(find(&r.scene, &want, 1.0).is_some());
}

//! The tree view (`layout.tree-view` acceptance): the nodes, the rows and their state across
//! rebuilds, the guide's vertices, the keys, the class of ways the cursor could point at a row
//! that is not shown, the expand motion in three tiers, only the rows in view being built, the
//! pointer, the disabled scope and the probes at scale 2.

use std::collections::BTreeSet;

use lune_ui_core::access::{AccessAction, Action, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::MotionTier;
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, StrokeVertex};
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{UiHarness, disabled_violations};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::tree_view::{
    CHEVRON, ChevronStyle, GuideStub, GuideStyle, INDENT, LABEL_GAP, MARK_DROP, PAD_LEFT, ROW,
    STUB_GAP, TreeOptions, TreeResponse, TreeRow, TreeState, column, guide_paths, straight_paths,
    stub_end,
};

const WIN: (f32, f32) = (500.0, 400.0);
/// Ten whole rows.
const TREE: Rect = Rect::new(10.0, 20.0, 300.0, 280.0);
const P: PointerButton = PointerButton::Primary;

const fn row(key: u64, depth: u16, text: &'static str, has_children: bool) -> TreeRow<'static> {
    TreeRow {
        key,
        depth,
        text,
        glyph: None,
        has_children,
    }
}

const A: u64 = 1;
const A1: u64 = 2;
const A1X: u64 = 3;
const A1Y: u64 = 4;
const A2: u64 = 5;
const B: u64 = 6;
const B1: u64 = 7;
const C: u64 = 8;

/// `A` holds `A1` (with `A1X`, `A1Y`) and `A2`; `B` holds `B1`; `C` is a leaf at the top.
const ROWS: [TreeRow<'static>; 8] = [
    row(A, 0, "a", true),
    row(A1, 1, "a1", true),
    row(A1X, 2, "a1x", false),
    row(A1Y, 2, "a1y", false),
    row(A2, 1, "a2", false),
    row(B, 0, "b", true),
    row(B1, 1, "b1", false),
    row(C, 0, "c", false),
];

fn opts<'a>(rows: &'a [TreeRow<'a>]) -> TreeOptions<'a> {
    TreeOptions {
        rows,
        label: "Files",
        guides: true,
        guide_style: GuideStyle::Elbow,
        chevron: ChevronStyle::Triangle,
        stub: GuideStub::Label,
        toggle_on_row: false,
    }
}

fn tid() -> WidgetId {
    WidgetId::ROOT.with("t")
}

fn rid(key: u64) -> WidgetId {
    tid().with(key)
}

/// One frame's results.
struct Run {
    scene: Scene,
    out: FrameOutput,
    r: TreeResponse,
    st: TreeState,
    /// The keys `cells` was called for.
    cells: Vec<u64>,
}

fn run_with(h: &mut UiHarness, o: &TreeOptions<'_>, before: impl FnOnce(&mut TreeState)) -> Run {
    let mut r = None;
    let mut st = None;
    let mut cells = Vec::new();
    let (scene, out) = h.frame(|ui| {
        before(ui.tree_state("t"));
        r = Some(ui.tree_view("t", TREE, o, |_, ctx| cells.push(ctx.key)));
        st = Some(ui.tree_state("t").clone());
    });
    Run {
        scene,
        out,
        r: r.expect("a frame ran"),
        st: st.expect("a frame ran"),
        cells,
    }
}

fn run(h: &mut UiHarness, rows: &[TreeRow<'_>]) -> Run {
    run_with(h, &opts(rows), |_| {})
}

/// A harness with `open` expanded and the tree focused by Tab, the cursor on the first row.
fn focused(open: &[u64]) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    let keys: BTreeSet<u64> = open.iter().copied().collect();
    run_with(&mut h, &opts(&ROWS), |st| st.expanded = keys);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.out.focused, Some(tid()));
    assert_eq!(f.st.cursor, Some(A));
    h.at_ms(20);
    run(&mut h, &ROWS);
    h
}

fn key(h: &mut UiHarness, rows: &[TreeRow<'_>], ms: u64, k: NamedKey) -> Run {
    h.input.key_down(Key::Named(k), None);
    h.at_ms(ms);
    run(h, rows)
}

/// The names of the built rows, in order.
fn built(out: &FrameOutput) -> Vec<&str> {
    out.tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::TreeItem)
        .map(|n| n.name.as_str())
        .collect()
}

fn top_of(out: &FrameOutput, key: u64) -> f32 {
    out.tree.get(rid(key)).expect("a built row").bounds.y
}

/// The centre of the chevron of the row at `y` of `depth`.
fn chevron_at(depth: u16, y: f32) -> (f32, f32) {
    (TREE.x + column(depth), y + ROW / 2.0)
}

fn click(h: &mut UiHarness, rows: &[TreeRow<'_>], ms: u64, at: (f32, f32)) -> Run {
    h.input.pointer_to(at.0, at.1);
    h.at_ms(ms);
    run(h, rows);
    h.input.click(P);
    h.at_ms(ms + 10);
    run(h, rows)
}

fn access(h: &mut UiHarness, target: WidgetId, action: Action) {
    h.input.access(AccessAction {
        target,
        action,
        value: None,
    });
}

/// The segments of the frame's strokes on the page layer, as (from, to) in device px.
fn segments(scene: &Scene) -> Vec<([f32; 2], [f32; 2])> {
    let mid = |a: &StrokeVertex, b: &StrokeVertex| {
        [(a.pos[0] + b.pos[0]) / 2.0, (a.pos[1] + b.pos[1]) / 2.0]
    };
    scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.geometry.strokes.chunks(6))
        .map(|c| (mid(&c[0], &c[1]), mid(&c[2], &c[4])))
        .collect()
}

/// Whether a vertical stroke at `x` runs from `y0` to `y1`.
fn has_line(scene: &Scene, x: f32, y0: f32, y1: f32) -> bool {
    segments(scene).iter().any(|(a, b)| {
        (a[0] - x).abs() < 0.01
            && (b[0] - x).abs() < 0.01
            && (a[1] - y0).abs() < 0.01
            && (b[1] - y1).abs() < 0.01
    })
}

// The nodes.

#[test]
fn the_tree_and_its_rows_are_nodes_with_level_state_and_actions() {
    let mut h = focused(&[A]);
    let f = key(&mut h, &ROWS, 100, NamedKey::Space);
    let tree = f.out.tree.get(tid()).expect("the tree");
    assert_eq!((tree.role, tree.name.as_str()), (Role::Tree, "Files"));
    assert!(tree.states.contains(State::Focusable));
    assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "c"]);
    assert!(
        tree.children
            .iter()
            .all(|c| { f.out.tree.get(*c).is_some_and(|n| n.role == Role::TreeItem) }),
        "rows hang off the tree"
    );

    let a = f.out.tree.get(rid(A)).expect("a");
    assert_eq!(a.level, Some(1));
    assert!(a.states.contains(State::Expanded) && a.states.contains(State::Selected));
    assert!(a.actions.contains(Action::Collapse) && !a.actions.contains(Action::Expand));
    assert!(a.actions.contains(Action::Default) && a.actions.contains(Action::ScrollIntoView));

    let a1 = f.out.tree.get(rid(A1)).expect("a1");
    assert_eq!(a1.level, Some(2));
    assert!(!a1.states.contains(State::Expanded) && !a1.states.contains(State::Selected));
    assert!(a1.actions.contains(Action::Expand) && !a1.actions.contains(Action::Collapse));

    let leaf = f.out.tree.get(rid(A2)).expect("a2");
    assert_eq!(leaf.level, Some(2));
    assert!(!leaf.actions.contains(Action::Expand) && !leaf.actions.contains(Action::Collapse));
    assert!(
        !leaf.states.contains(State::Focusable),
        "the tree is the stop"
    );
}

#[test]
fn a_row_that_rises_by_more_than_one_draws_at_its_depth_and_nothing_panics() {
    let rows = [
        row(1, 0, "top", false),
        row(2, 5, "deep", false),
        row(3, 1, "back", true),
        row(4, 60000, "far", false),
    ];
    let mut h = UiHarness::new(WIN);
    let f = run_with(&mut h, &opts(&rows), |st| {
        st.expanded.insert(3);
    });
    assert_eq!(built(&f.out), ["top", "deep", "back", "far"]);
    assert_eq!(f.out.tree.get(rid(2)).expect("deep").level, Some(6));
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    for k in [
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::End,
        NamedKey::ArrowLeft,
    ] {
        h.input.key_down(Key::Named(k), None);
    }
    h.at_ms(10);
    run(&mut h, &rows);
}

#[test]
fn no_rows_draw_nothing_and_take_the_keys() {
    let mut h = UiHarness::new(WIN);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::ArrowDown)),
        "down",
    ));
    run(&mut h, &[]);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    run(&mut h, &[]);
    let f = key(&mut h, &[], 20, NamedKey::ArrowDown);
    assert!(built(&f.out).is_empty());
    assert!(f.out.fired.is_empty());
    assert_eq!(f.st.cursor, None);
}

// State across rebuilds.

#[test]
fn expansion_and_selection_survive_a_rebuilt_row_list() {
    let mut h = focused(&[]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowRight);
    key(&mut h, &ROWS, 500, NamedKey::Space);
    let rebuilt: Vec<TreeRow<'static>> = ROWS.to_vec();
    h.at_ms(600);
    let f = run(&mut h, &rebuilt);
    assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "c"]);
    assert_eq!(f.st.expanded, BTreeSet::from([A]));
    assert_eq!(f.st.selected, BTreeSet::from([A]));
}

#[test]
fn state_lasts_600_undrawn_frames_under_keep_alive() {
    let mut h = UiHarness::new(WIN);
    h.frame(|ui| {
        ui.keep_alive("tab", |ui| {
            ui.tree_state("t").expanded.insert(A);
            ui.tree_view("t", TREE, &opts(&ROWS), |_, _| {});
        });
    });
    for i in 0..600u64 {
        h.at_ms(i);
        h.frame(|ui| ui.mark_alive("tab"));
    }
    h.at_ms(1000);
    let (_, out) = h.frame(|ui| {
        ui.keep_alive("tab", |ui| {
            assert_eq!(ui.tree_state("t").expanded, BTreeSet::from([A]));
            ui.tree_view("t", TREE, &opts(&ROWS), |_, _| {});
        });
    });
    assert_eq!(built(&out), ["a", "a1", "a2", "b", "c"]);
}

#[test]
fn a_selection_scrolled_out_of_view_is_kept() {
    let rows: Vec<TreeRow<'static>> = (0..100).map(|i| row(i, 0, "r", false)).collect();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &rows);
    h.input.pointer_to(50.0, 30.0);
    h.at_ms(10);
    run(&mut h, &rows);
    h.input.click(P);
    h.at_ms(20);
    let f = run(&mut h, &rows);
    assert_eq!(f.st.selected, BTreeSet::from([0]));
    h.input.scroll_pixels(0.0, -2000.0);
    h.at_ms(30);
    run(&mut h, &rows);
    h.at_ms(40);
    let f = run(&mut h, &rows);
    assert!(f.out.tree.get(rid(0)).is_none(), "scrolled away");
    assert_eq!(f.st.selected, BTreeSet::from([0]));
    h.input.scroll_pixels(0.0, 2000.0);
    h.at_ms(50);
    run(&mut h, &rows);
    h.at_ms(60);
    let f = run(&mut h, &rows);
    let n = f.out.tree.get(rid(0)).expect("back");
    assert!(n.states.contains(State::Selected));
}

// The guide's geometry, computed by hand from
// elbow x = floor((depth - 1) * 16 + 15) + 0.5 (under the parent's mark), stub = 8 + depth * 16 - 1,
// cy = h / 2, r = 3.5.

#[test]
fn the_guide_of_a_last_child_is_one_line_turning_into_the_stub() {
    for (depth, x, stub) in [(1, 15.5, 23.0), (2, 31.5, 39.0), (3, 47.5, 55.0)] {
        let paths = guide_paths(depth, true, &[], 28.0, stub);
        assert_eq!(paths.len(), 1, "depth {depth}");
        let p = &paths[0];
        assert_eq!(p[0], [x, 0.0]);
        assert_eq!(p[1], [x, 10.5], "down to cy - r");
        assert_eq!(
            p[p.len() - 2],
            [x + 3.5, 14.0],
            "the curve ends at x + r, cy"
        );
        assert_eq!(p[p.len() - 1], [stub, 14.0]);
        // Half way along the quadratic: x + r/4, cy - r/4.
        assert_eq!(p[4], [x + 0.875, 13.125]);
    }
}

#[test]
fn the_guide_of_a_child_with_a_next_sibling_is_a_trunk_and_a_branch() {
    for (depth, x, stub) in [(1, 15.5, 23.0), (2, 31.5, 39.0), (3, 47.5, 55.0)] {
        let paths = guide_paths(depth, false, &[], 28.0, stub);
        assert_eq!(paths.len(), 2, "depth {depth}");
        assert_eq!(paths[0], [[x, 0.0], [x, 28.0]], "the whole row");
        let b = &paths[1];
        assert_eq!(b[0], [x, 10.5]);
        assert_eq!(b[b.len() - 2], [x + 3.5, 14.0]);
        assert_eq!(b[b.len() - 1], [stub, 14.0]);
    }
}

#[test]
fn a_leaf_stub_reaches_its_label_by_default_and_stops_short_when_asked() {
    // A tree with marks: a leaf's stub crosses its empty slot to `LABEL_GAP` before the label,
    // or stops `STUB_GAP` before the slot; a row with a mark always stops before the slot.
    let label_x = |d: u16| PAD_LEFT + f32::from(d) * INDENT + CHEVRON + 4.0;
    for d in 1..4u16 {
        assert_eq!(stub_end(d, true, GuideStub::Label), label_x(d) - LABEL_GAP);
        assert_eq!(
            stub_end(d, true, GuideStub::Short),
            PAD_LEFT + f32::from(d) * INDENT - STUB_GAP
        );
        for reach in [GuideStub::Label, GuideStub::Short] {
            assert_eq!(
                stub_end(d, false, reach),
                PAD_LEFT + f32::from(d) * INDENT - STUB_GAP
            );
        }
    }

    // Drawn: a1x (a leaf at depth 2) ends at 54 by default and 39 with `Short`; a1 (with a mark,
    // depth 1) ends at 23 either way. The cap is a half disc at the stub's end.
    let caps = |stub: GuideStub| -> Vec<f32> {
        let mut h = UiHarness::new(WIN);
        let o = TreeOptions {
            stub,
            ..opts(&ROWS)
        };
        let f = run_with(&mut h, &o, |st| {
            st.expanded.extend([A, A1]);
        });
        let mut xs: Vec<f32> = f
            .scene
            .in_order()
            .into_iter()
            .flat_map(|l| {
                l.geometry
                    .under
                    .iter()
                    .filter(|s| s.radii == [0.0, 0.5, 0.5, 0.0])
                    .map(|s| s.rect[0] - TREE.x)
                    .collect::<Vec<_>>()
            })
            .collect();
        xs.sort_by(f32::total_cmp);
        xs.dedup();
        xs
    };
    assert_eq!(
        caps(GuideStub::Label),
        vec![23.0, 38.0, 54.0],
        "a1; a2; a1x and a1y"
    );
    assert_eq!(
        caps(GuideStub::Short),
        vec![23.0, 39.0],
        "every stub stops before the slot"
    );
}

#[test]
fn ancestor_lines_are_full_height_at_their_columns() {
    let paths = guide_paths(3, true, &[0, 1], 28.0, 55.0);
    assert_eq!(paths[0], [[15.5, 0.0], [15.5, 28.0]]);
    assert_eq!(paths[1], [[31.5, 0.0], [31.5, 28.0]]);
    assert_eq!(paths[2][0], [47.5, 0.0]);
    assert!(
        guide_paths(0, true, &[], 28.0, 7.0).is_empty(),
        "a top row has none"
    );
}

#[test]
fn the_guides_follow_siblings_across_a_collapsed_one_and_stop_at_the_last() {
    let mut h = UiHarness::new(WIN);
    // a open, a1 closed with hidden children, a2 after it.
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.insert(A);
    });
    let x = TREE.x + 15.5;
    let y = TREE.y;
    assert!(
        has_line(&f.scene, x, y + ROW, y + 2.0 * ROW),
        "a1 has a next sibling past its hidden children: a trunk through the row"
    );
    assert!(
        !has_line(&f.scene, x, y + 2.0 * ROW, y + 3.0 * ROW),
        "a2 is the last: no trunk through it"
    );

    // Everything open: a1x and a1y carry the a column, b1 does not.
    let mut h = UiHarness::new(WIN);
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.extend([A, A1, B]);
    });
    assert!(
        has_line(&f.scene, x, y + 3.0 * ROW, y + 4.0 * ROW),
        "a1y's ancestor a1 has a next sibling"
    );
    assert!(
        !has_line(&f.scene, x, y + 6.0 * ROW, y + 7.0 * ROW),
        "b1 is last"
    );
}

#[test]
fn guides_off_draw_no_stroke() {
    let mut h = UiHarness::new(WIN);
    let o = TreeOptions {
        guides: false,
        ..opts(&ROWS)
    };
    let f = run_with(&mut h, &o, |st| {
        st.expanded.insert(A);
    });
    assert!(segments(&f.scene).is_empty());
}

#[test]
fn a_branch_starts_at_the_trunks_edge_so_the_translucent_line_does_not_double() {
    let mut h = UiHarness::new(WIN);
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.insert(A);
    });
    let x = TREE.x + 15.5;
    let branch: Vec<_> = segments(&f.scene)
        .into_iter()
        .filter(|(a, b)| a[0] != b[0])
        .filter(|(a, _)| a[1] < TREE.y + 2.0 * ROW && a[1] >= TREE.y + ROW)
        .collect();
    assert!(!branch.is_empty());
    let left = branch
        .iter()
        .map(|(a, _)| a[0])
        .fold(f32::INFINITY, f32::min);
    assert!(
        (left - (x + 0.5)).abs() < 0.01,
        "at the trunk's edge, no gap, no overlap along it: was {left}"
    );
}

// Probes at scale 2.

#[test]
fn the_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.extend([A, A1]);
        st.selected.insert(A1);
    });
    let geometry = &f
        .scene
        .in_order()
        .into_iter()
        .find(|l| !l.geometry.strokes.is_empty());
    let geometry = &geometry.expect("guides").geometry;

    // The selected row's fill: 56 device px high, radius 12.
    let fill = geometry
        .under
        .iter()
        .find(|s| s.color == premul(DARK.surface_selected))
        .expect("the selected fill");
    assert_eq!(fill.rect[3], 56.0, "the row");
    assert_eq!(fill.radii, [12.0; 4]);

    // One depth moves the content 32 device px: the texts of a and a1 (depths 0, 1). The mark is
    // drawn, not a glyph, so every text is a row's.
    let texts: Vec<f32> = f
        .scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.texts.iter().map(|t| t.x))
        .collect();
    assert_eq!(
        (texts[1] - texts[0]) * 2.0,
        32.0,
        "text of depth 1 from depth 0"
    );

    // The guide is 2 device px wide: `style[1]` is the half width.
    assert_eq!(geometry.strokes[0].style[1], 1.0);

    // The elbow's curve spans 7 device px each way.
    let row = TREE.y + 3.0 * ROW;
    let (cx, cy) = (TREE.x + 31.5, row + 14.0);
    let curve: Vec<_> = segments(&f.scene)
        .into_iter()
        .map(|(a, _)| a)
        .filter(|a| {
            a[0] > (cx * 2.0) - 0.01
                && a[0] <= (cx + 3.5) * 2.0 + 0.01
                && a[1] >= (cy - 3.5) * 2.0 - 0.01
                && a[1] <= cy * 2.0 + 0.01
        })
        .collect();
    assert!(curve.len() >= 4);
    let top = curve.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
    assert_eq!(cy * 2.0 - top, 7.0, "the curve starts r above the stub");

    // The stub's free end has its round cap: a half disc, one device px past the end. a1y is a
    // leaf, so its stub crosses the empty mark slot to 4 px before its text: 8 + 32 + 14 + 4 - 4.
    let stub = (TREE.x + 54.0) * 2.0;
    let cap = geometry
        .under
        .iter()
        .find(|s| s.rect[0] == stub && s.rect[1] == (cy - 0.5) * 2.0)
        .expect("the cap");
    assert_eq!((cap.rect[2], cap.rect[3]), (1.0, 2.0));
    assert_eq!(cap.radii, [0.0, 1.0, 1.0, 0.0]);
    assert_eq!(cap.color, premul(DARK.tree_guide));
}

#[test]
fn the_chevron_slot_is_on_every_row_when_any_row_has_children() {
    let with = [row(1, 0, "x", false), row(2, 0, "y", true)];
    let without = [row(1, 0, "x", false), row(2, 0, "y", false)];
    let text_x = |rows: &[TreeRow<'_>]| {
        let mut h = UiHarness::new(WIN);
        let f = run(&mut h, rows);
        f.scene
            .in_order()
            .into_iter()
            .flat_map(|l| l.texts.iter().map(|t| t.x))
            .next()
            .expect("the first row's text")
    };
    assert_eq!(text_x(&without), TREE.x + PAD_LEFT);
    assert_eq!(text_x(&with), TREE.x + PAD_LEFT + CHEVRON + 4.0);
}

// Keys.

#[test]
fn up_down_home_and_end_move_the_cursor_over_visible_rows() {
    let mut h = focused(&[A]);
    let f = key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(A1));
    assert!(f.st.selected.is_empty(), "moving selects nothing");
    let f = key(&mut h, &ROWS, 120, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(A2), "a1's children are hidden");
    let f = key(&mut h, &ROWS, 140, NamedKey::End);
    assert_eq!(f.st.cursor, Some(C));
    let f = key(&mut h, &ROWS, 160, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(C), "no wrap");
    let f = key(&mut h, &ROWS, 180, NamedKey::Home);
    assert_eq!(f.st.cursor, Some(A));
    let f = key(&mut h, &ROWS, 200, NamedKey::ArrowUp);
    assert_eq!(f.st.cursor, Some(A), "no wrap");
}

#[test]
fn right_expands_then_moves_to_the_first_child() {
    let mut h = focused(&[]);
    let f = key(&mut h, &ROWS, 100, NamedKey::ArrowRight);
    assert_eq!(f.r.toggled, Some(A));
    assert_eq!(f.st.expanded, BTreeSet::from([A]));
    assert_eq!(f.st.cursor, Some(A));
    h.at_ms(500);
    run(&mut h, &ROWS);
    let f = key(&mut h, &ROWS, 520, NamedKey::ArrowRight);
    assert_eq!((f.r.toggled, f.st.cursor), (None, Some(A1)));
}

#[test]
fn right_on_a_leaf_does_nothing_and_is_consumed() {
    let mut h = focused(&[]);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::ArrowRight)),
        "right",
    ));
    key(&mut h, &ROWS, 100, NamedKey::End);
    let f = key(&mut h, &ROWS, 120, NamedKey::ArrowRight);
    assert_eq!((f.st.cursor, f.r.toggled), (Some(C), None));
    assert!(f.out.fired.is_empty());
}

#[test]
fn left_collapses_then_moves_to_the_parent() {
    let mut h = focused(&[A, A1]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    key(&mut h, &ROWS, 120, NamedKey::ArrowDown);
    let f = key(&mut h, &ROWS, 140, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(A1Y));
    let f = key(&mut h, &ROWS, 160, NamedKey::ArrowLeft);
    assert_eq!(
        (f.st.cursor, f.r.toggled),
        (Some(A1), None),
        "a leaf: the parent"
    );
    let f = key(&mut h, &ROWS, 180, NamedKey::ArrowLeft);
    assert_eq!(
        (f.st.cursor, f.r.toggled),
        (Some(A1), Some(A1)),
        "open: closes"
    );
    h.at_ms(600);
    run(&mut h, &ROWS);
    let f = key(&mut h, &ROWS, 620, NamedKey::ArrowLeft);
    assert_eq!(
        (f.st.cursor, f.r.toggled),
        (Some(A), None),
        "closed: the parent"
    );
}

#[test]
fn left_on_a_root_or_a_closed_leaf_at_the_top_does_nothing_and_is_consumed() {
    let mut h = focused(&[]);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::ArrowLeft)),
        "left",
    ));
    let f = key(&mut h, &ROWS, 100, NamedKey::ArrowLeft);
    assert_eq!((f.st.cursor, f.r.toggled), (Some(A), None), "a closed root");
    assert!(f.out.fired.is_empty());
    key(&mut h, &ROWS, 120, NamedKey::End);
    let f = key(&mut h, &ROWS, 140, NamedKey::ArrowLeft);
    assert_eq!(
        (f.st.cursor, f.r.toggled),
        (Some(C), None),
        "a leaf at the top"
    );
    assert!(f.out.fired.is_empty());
}

#[test]
fn enter_activates_and_space_selects() {
    let mut h = focused(&[]);
    let f = key(&mut h, &ROWS, 100, NamedKey::Enter);
    assert_eq!(f.r.activated, Some(A));
    assert!(f.st.selected.is_empty());
    key(&mut h, &ROWS, 120, NamedKey::ArrowDown);
    let f = key(&mut h, &ROWS, 140, NamedKey::Space);
    assert_eq!(f.st.selected, BTreeSet::from([B]));
    assert_eq!(f.r.activated, None);
    assert!(f.r.response.changed);
    key(&mut h, &ROWS, 160, NamedKey::ArrowDown);
    let f = key(&mut h, &ROWS, 180, NamedKey::Space);
    assert_eq!(f.st.selected, BTreeSet::from([C]), "single selection");
}

#[test]
fn page_keys_move_by_a_viewport_and_keep_the_cursor_in_view() {
    let rows: Vec<TreeRow<'static>> = (0..100).map(|i| row(i, 0, "r", false)).collect();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &rows);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    run(&mut h, &rows);
    let f = key(&mut h, &rows, 20, NamedKey::PageDown);
    assert_eq!(f.st.cursor, Some(10), "ten rows in the viewport");
    h.at_ms(500);
    let f = run(&mut h, &rows);
    let y = top_of(&f.out, 10);
    assert!(y >= TREE.y && y + ROW <= TREE.y + TREE.h, "in view at {y}");
    let f = key(&mut h, &rows, 520, NamedKey::PageDown);
    assert_eq!(f.st.cursor, Some(20));
    let f = key(&mut h, &rows, 540, NamedKey::PageUp);
    assert_eq!(f.st.cursor, Some(10));
    let f = key(&mut h, &rows, 560, NamedKey::PageUp);
    assert_eq!(f.st.cursor, Some(0));
    let f = key(&mut h, &rows, 580, NamedKey::PageUp);
    assert_eq!(f.st.cursor, Some(0));
    let f = key(&mut h, &rows, 600, NamedKey::End);
    assert_eq!(f.st.cursor, Some(99));
    h.at_ms(1100);
    run(&mut h, &rows);
    h.at_ms(1200);
    let f = run(&mut h, &rows);
    assert_eq!(top_of(&f.out, 99) + ROW, TREE.y + TREE.h, "the end shows");
}

#[test]
fn modified_arrows_are_left_for_shortcuts() {
    let mut h = focused(&[]);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::ArrowDown)).ctrl(),
        "ctrl-down",
    ));
    h.input.set_modifiers(lune_ui_core::input::Modifiers {
        control: true,
        ..Default::default()
    });
    let f = key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(A));
    assert_eq!(f.out.fired.len(), 1);
}

#[test]
fn modified_enter_and_space_are_left_for_shortcuts() {
    for k in [NamedKey::Enter, NamedKey::Space] {
        let mut h = focused(&[]);
        h.shortcuts.add(Binding::new(
            Scope::Global,
            Chord::key(Key::Named(k)).ctrl(),
            "chord",
        ));
        h.input.set_modifiers(lune_ui_core::input::Modifiers {
            control: true,
            ..Default::default()
        });
        let f = key(&mut h, &ROWS, 100, k);
        assert_eq!(f.r.activated, None, "{k:?}");
        assert_eq!(f.out.fired.len(), 1, "{k:?}");
    }
}

// The class: every way the cursor could point at a row that is not shown.

#[test]
fn a_chevron_press_on_an_ancestor_gives_the_cursor_to_it_at_the_toggle() {
    let mut h = focused(&[A, A1]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    let f = key(&mut h, &ROWS, 120, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(A1X));
    let f = click(&mut h, &ROWS, 200, chevron_at(0, TREE.y));
    assert_eq!(f.r.toggled, Some(A));
    assert_eq!(
        f.st.cursor,
        Some(A),
        "at the toggle, not when the motion ends"
    );
    let f = key(&mut h, &ROWS, 230, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(B), "the walk skips the closing block");
}

#[test]
fn a_collapse_two_levels_up_goes_to_the_nearest_visible_ancestor() {
    let mut h = focused(&[A, A1]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    key(&mut h, &ROWS, 120, NamedKey::ArrowDown);
    access(&mut h, rid(A), Action::Collapse);
    h.at_ms(200);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.r.toggled, Some(A));
    assert_eq!(f.st.expanded, BTreeSet::from([A1]));
    assert_eq!(f.st.cursor, Some(A), "a1 is hidden too");
}

#[test]
fn the_app_closing_a_row_moves_the_cursor_to_it() {
    let mut h = focused(&[A, A1]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    let f = key(&mut h, &ROWS, 120, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(A1X));
    h.at_ms(200);
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.remove(&A1);
    });
    assert_eq!(f.st.cursor, Some(A1));
    // Several keys at once snap: the cursor's rule is the same.
    let f = key(&mut h, &ROWS, 600, NamedKey::ArrowRight);
    assert_eq!(f.st.cursor, Some(A1));
    h.at_ms(900);
    run(&mut h, &ROWS);
    key(&mut h, &ROWS, 920, NamedKey::ArrowRight);
    h.at_ms(940);
    let f = run_with(&mut h, &opts(&ROWS), |st| st.expanded.clear());
    assert_eq!(f.st.cursor, Some(A));
    assert_eq!(built(&f.out), ["a", "b", "c"]);
}

#[test]
fn rows_the_app_removes_give_the_cursor_to_the_nearest_ancestor_left() {
    let mut h = focused(&[A, A1]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    key(&mut h, &ROWS, 120, NamedKey::ArrowDown);
    let f = key(&mut h, &ROWS, 140, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(A1Y));
    // a1y goes.
    let less: Vec<_> = ROWS.iter().copied().filter(|r| r.key != A1Y).collect();
    h.at_ms(200);
    let f = run(&mut h, &less);
    assert_eq!(f.st.cursor, Some(A1), "its parent");
    h.at_ms(220);
    run(&mut h, &ROWS);

    // a1 and its children go with the cursor deep inside: a is left.
    let mut h = focused(&[A, A1]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    key(&mut h, &ROWS, 120, NamedKey::ArrowDown);
    let less: Vec<_> = ROWS
        .iter()
        .copied()
        .filter(|r| ![A1, A1X, A1Y].contains(&r.key))
        .collect();
    h.at_ms(200);
    let f = run(&mut h, &less);
    assert_eq!(f.st.cursor, Some(A));
}

#[test]
fn with_no_ancestor_left_the_cursor_falls_to_its_last_index() {
    let mut h = focused(&[]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    let f = key(&mut h, &ROWS, 120, NamedKey::ArrowDown);
    assert_eq!(f.st.cursor, Some(C));
    let two = [row(10, 0, "x", false), row(11, 0, "y", false)];
    h.at_ms(200);
    let f = run(&mut h, &two);
    assert_eq!(
        f.st.cursor,
        Some(11),
        "index 2 is past the end: the last row"
    );
    h.at_ms(220);
    let f = run(&mut h, &[]);
    assert_eq!(f.st.cursor, Some(11), "no rows: nothing to stand on");
    h.at_ms(240);
    let f = run(&mut h, &two);
    assert_eq!(f.st.cursor, Some(11));
}

#[test]
fn a_selected_key_that_vanished_stays_until_retain_and_retain_prunes() {
    let mut h = focused(&[A]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    key(&mut h, &ROWS, 120, NamedKey::Space);
    let less: Vec<_> = ROWS
        .iter()
        .copied()
        .filter(|r| ![A1, A1X, A1Y].contains(&r.key))
        .collect();
    h.at_ms(200);
    let f = run(&mut h, &less);
    assert_eq!(f.st.selected, BTreeSet::from([A1]));
    assert_eq!(f.st.cursor, Some(A));
    h.at_ms(220);
    let f = run_with(&mut h, &opts(&less), |st| {
        st.retain(|k| k != A1 && k != A);
    });
    assert!(f.st.selected.is_empty());
    assert!(f.st.expanded.is_empty());
    assert_eq!(f.st.cursor, Some(A), "the row is still there");
}

#[test]
fn a_cursor_in_a_block_closed_mid_flight_follows_and_up_walks_the_new_list() {
    let mut h = focused(&[A]);
    key(&mut h, &ROWS, 100, NamedKey::ArrowDown);
    key(&mut h, &ROWS, 120, NamedKey::ArrowDown); // a2
    let f = key(&mut h, &ROWS, 140, NamedKey::ArrowLeft);
    assert_eq!(f.st.cursor, Some(A), "a2 is a leaf: its parent");
    let f = key(&mut h, &ROWS, 160, NamedKey::ArrowLeft);
    assert_eq!(f.r.toggled, Some(A));
    let f = key(&mut h, &ROWS, 180, NamedKey::ArrowDown);
    assert_eq!(
        f.st.cursor,
        Some(B),
        "the closing rows are out of reach already"
    );
    let f = key(&mut h, &ROWS, 200, NamedKey::ArrowUp);
    assert_eq!(f.st.cursor, Some(A));
}

// Expand motion.

/// The share of a block of `block_rows` that shows, read from the row below it.
fn share(f: &Run, below: u64, settled_top: f32, block_rows: f32) -> f32 {
    1.0 - (settled_top - top_of(&f.out, below)) / (block_rows * ROW)
}

#[test]
fn expand_reaches_full_height_at_base_on_a_sine_in_out_curve() {
    let mut h = focused(&[]);
    // a, then its two children, then b.
    let settled_b = TREE.y + 3.0 * ROW;
    let f = key(&mut h, &ROWS, 1000, NamedKey::ArrowRight);
    assert_eq!(f.out.repaint, Repaint::Now);
    let at0 = share(&f, B, settled_b, 2.0);
    assert_eq!(at0, 0.0, "the frame of the toggle shows no block");
    assert_eq!(built(&f.out), ["a", "b", "c"], "block rows have no nodes");

    h.at_ms(1125);
    let f = run(&mut h, &ROWS);
    let mid = share(&f, B, settled_b, 2.0);
    assert!((mid - 0.5).abs() < 1e-3, "half way, sine-in-out: {mid}");
    assert_eq!(f.out.repaint, Repaint::Now);
    assert_eq!(built(&f.out), ["a", "b", "c"]);
    assert_eq!(f.cells, [A, B, C], "cells are not called for the block");
    assert_eq!(
        top_of(&f.out, A),
        TREE.y,
        "the rows above do not move, none changes height"
    );
    assert_eq!(f.out.tree.get(rid(B)).expect("b").bounds.h, ROW);

    h.at_ms(1187);
    let f = run(&mut h, &ROWS);
    let quarter = share(&f, B, settled_b, 2.0);
    let want = (1.0 - (std::f32::consts::PI * 0.748).cos()) / 2.0;
    assert!((quarter - want).abs() < 2e-3, "{quarter} against {want}");

    h.at_ms(1250);
    let f = run(&mut h, &ROWS);
    assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "c"]);
    assert_eq!(top_of(&f.out, B), settled_b);
    assert_eq!(f.out.repaint, Repaint::Idle, "settled at base (250 ms)");
}

#[test]
fn block_rows_draw_under_the_clip_with_no_hits() {
    let mut h = focused(&[]);
    key(&mut h, &ROWS, 1000, NamedKey::ArrowRight);
    h.at_ms(1125);
    let f = run(&mut h, &ROWS);
    // The block's guide strokes carry its clip: one row high at half of two rows.
    let clipped = f
        .scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.geometry.strokes.iter())
        .find(|v| v.clip[3] < TREE.h - 1.0)
        .expect("a block stroke");
    assert_eq!(clipped.clip[1], TREE.y + ROW);
    assert!((clipped.clip[3] - ROW).abs() < 0.01);
    assert_eq!(h.store.last_rect(rid(A1)), None, "no hit region");
    assert_eq!(h.store.last_rect(rid(A2)), None);
}

#[test]
fn collapse_runs_the_same_curve_backwards_and_the_extent_follows() {
    // Enough rows that the area scrolls; the tree is scrolled to its end.
    let mut rows = vec![row(100, 0, "open", true)];
    rows.extend((0..6).map(|i| row(101 + i, 1, "kid", false)));
    rows.extend((0..12).map(|i| row(200 + i, 0, "top", false)));
    let mut h = UiHarness::new(WIN);
    run_with(&mut h, &opts(&rows), |st| {
        st.expanded.insert(100);
    });
    let max = |f: &Run| {
        let n = f.out.tree.nodes.iter().find(|n| n.role == Role::ScrollArea);
        match n.and_then(|n| n.value.as_ref()) {
            Some(Value::Number { max, .. }) => *max,
            other => panic!("no value: {other:?}"),
        }
    };
    h.at_ms(10);
    let f = run(&mut h, &rows);
    assert_eq!(max(&f), 19.0 * 28.0 - 280.0);

    h.at_ms(20);
    let f = run_with(&mut h, &opts(&rows), |st| {
        st.expanded.remove(&100);
    });
    assert_eq!(max(&f), 19.0 * 28.0 - 280.0, "the frame of the toggle");
    h.at_ms(145);
    let f = run(&mut h, &rows);
    assert!(
        (max(&f) - (16.0 * 28.0 - 280.0)).abs() < 0.1,
        "half of six rows"
    );
    h.at_ms(270);
    let f = run(&mut h, &rows);
    assert_eq!(max(&f), 13.0 * 28.0 - 280.0);
    assert_eq!(f.out.repaint, Repaint::Idle);
}

#[test]
fn a_settled_frame_is_the_frame_of_the_same_state_with_no_motion() {
    let mut moved = focused(&[]);
    key(&mut moved, &ROWS, 1000, NamedKey::ArrowRight);
    moved.at_ms(1125);
    run(&mut moved, &ROWS);
    moved.at_ms(1300);
    let a = run(&mut moved, &ROWS);

    let mut snapped = focused(&[A]);
    snapped.at_ms(1300);
    let b = run(&mut snapped, &ROWS);
    assert_eq!(a.scene, b.scene);
    assert_eq!(a.out.repaint, Repaint::Idle);
}

#[test]
fn reduced_and_off_snap_and_report_idle_the_frame_of_the_toggle() {
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        // The app's write has no input to catch up on: Idle on the frame itself.
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        run(&mut h, &ROWS);
        h.at_ms(100);
        let f = run_with(&mut h, &opts(&ROWS), |st| {
            st.expanded.insert(A);
        });
        assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "c"], "{tier:?}");
        assert_eq!(f.out.repaint, Repaint::Idle, "{tier:?}");

        // A click's frame asks the one catch-up every click asks (D81); the frame that shows the
        // toggle is Idle.
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        run(&mut h, &ROWS);
        let f = click(&mut h, &ROWS, 100, chevron_at(0, TREE.y));
        assert_eq!(f.r.toggled, Some(A));
        h.at_ms(120);
        let f = run(&mut h, &ROWS);
        assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "c"], "{tier:?}");
        assert_eq!(f.out.repaint, Repaint::Idle, "{tier:?}");

        // A key's frame shows the toggle at once, nothing in flight; only D81's catch-up follows.
        let mut h = focused(&[]);
        h.tier = tier;
        let f = key(&mut h, &ROWS, 100, NamedKey::ArrowRight);
        assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "c"], "{tier:?}");
        h.at_ms(101);
        let g = run(&mut h, &ROWS);
        assert_eq!(g.out.repaint, Repaint::Idle, "{tier:?}");
        assert_eq!(f.scene, g.scene, "{tier:?}: nothing moved");
    }
}

#[test]
fn a_toggle_mid_flight_retargets_from_the_current_height_with_no_jump() {
    let mut h = focused(&[]);
    let settled = TREE.y + 3.0 * ROW;
    key(&mut h, &ROWS, 1000, NamedKey::ArrowRight);
    h.at_ms(1125);
    let f = run(&mut h, &ROWS);
    let before = share(&f, B, settled, 2.0);

    let f = key(&mut h, &ROWS, 1126, NamedKey::ArrowRight);
    assert_eq!(f.r.toggled, None, "open: Right walks into the block");
    assert_eq!(f.st.cursor, Some(A1));
    let f = key(&mut h, &ROWS, 1127, NamedKey::ArrowLeft);
    assert_eq!(f.st.cursor, Some(A), "a1 is closed: its parent");
    let f = key(&mut h, &ROWS, 1128, NamedKey::ArrowLeft);
    assert_eq!(f.r.toggled, Some(A));
    let at_toggle = share(&f, B, settled, 2.0);
    assert!(
        (at_toggle - before).abs() < 0.04,
        "no jump: {before} to {at_toggle}"
    );
    h.at_ms(1140);
    let f = run(&mut h, &ROWS);
    let later = share(&f, B, settled, 2.0);
    assert!((later - at_toggle).abs() < 0.12, "{at_toggle} to {later}");
    h.at_ms(1500);
    let f = run(&mut h, &ROWS);
    assert_eq!(built(&f.out), ["a", "b", "c"]);
    assert_eq!(f.out.repaint, Repaint::Idle);
}

#[test]
fn a_toggle_of_another_row_snaps_the_first_block() {
    let mut h = focused(&[]);
    key(&mut h, &ROWS, 1000, NamedKey::ArrowRight);
    h.at_ms(1100);
    run(&mut h, &ROWS);
    // b is the second row while a's block is at a share: its chevron is where its row is.
    h.at_ms(1110);
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.insert(B);
    });
    assert_eq!(
        built(&f.out),
        ["a", "a1", "a2", "b", "c"],
        "a's block snapped open, b's opens: its child has no node"
    );
    assert_eq!(top_of(&f.out, B), TREE.y + 3.0 * ROW);
    assert_eq!(
        top_of(&f.out, C),
        TREE.y + 4.0 * ROW,
        "b1's block starts shut"
    );
    h.at_ms(1235);
    let f = run(&mut h, &ROWS);
    assert!((top_of(&f.out, C) - (TREE.y + 4.5 * ROW)).abs() < 0.1);
    h.at_ms(1400);
    let f = run(&mut h, &ROWS);
    assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "b1", "c"]);
}

#[test]
fn two_keys_changed_at_once_snap() {
    let mut h = UiHarness::new(WIN);
    run(&mut h, &ROWS);
    h.at_ms(100);
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.extend([A, B]);
    });
    assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "b1", "c"]);
    assert_eq!(f.out.repaint, Repaint::Idle);
}

#[test]
fn the_first_frame_and_a_leaf_never_move() {
    let mut h = UiHarness::new(WIN);
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.insert(A);
    });
    assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "c"]);
    h.at_ms(10);
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.insert(C);
    });
    assert_eq!(f.out.repaint, Repaint::Idle, "a key on a leaf has no block");
}

#[test]
fn a_block_whose_rows_change_mid_flight_ends_without_a_panic() {
    let mut h = focused(&[]);
    key(&mut h, &ROWS, 1000, NamedKey::ArrowRight);
    h.at_ms(1100);
    run(&mut h, &ROWS);
    h.at_ms(1110);
    let f = run(&mut h, &[row(C, 0, "c", false)]);
    assert_eq!(built(&f.out), ["c"]);
    assert_eq!(f.st.cursor, Some(C));
    h.at_ms(1120);
    let f = run(&mut h, &ROWS);
    assert_eq!(built(&f.out), ["a", "a1", "a2", "b", "c"]);
}

#[test]
fn a_block_taller_than_the_viewport_builds_only_what_the_clip_shows() {
    let mut rows = vec![row(1, 0, "open", true)];
    rows.extend((0..5000).map(|i| row(10 + i, 1, "kid", false)));
    let mut h = focused(&[]);
    h.at_ms(30);
    run(&mut h, &rows);
    key(&mut h, &rows, 1000, NamedKey::Home);
    h.at_ms(1010);
    run(&mut h, &rows);
    h.at_ms(1020);
    let f = run_with(&mut h, &opts(&rows), |st| {
        st.expanded.insert(1);
    });
    assert_eq!(built(&f.out), ["open"]);
    h.at_ms(1145);
    let f = run(&mut h, &rows);
    assert_eq!(built(&f.out), ["open"], "the viewport is all block");
    let strokes = f
        .scene
        .in_order()
        .into_iter()
        .map(|l| l.geometry.strokes.len())
        .sum::<usize>();
    // Nine rows of block at most reach into the viewport, each a bounded guide.
    assert!(strokes < 6 * 30 * 11, "{strokes} vertices");
}

#[test]
fn the_cursor_walked_into_an_opening_block_is_revealed_when_it_lands() {
    let mut rows = vec![row(1, 0, "open", true)];
    rows.extend((0..30).map(|i| row(10 + i, 1, "kid", false)));
    let mut h = UiHarness::new(WIN);
    run(&mut h, &rows);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    run(&mut h, &rows);
    key(&mut h, &rows, 1000, NamedKey::ArrowRight);
    let f = key(&mut h, &rows, 1010, NamedKey::End);
    assert_eq!(f.st.cursor, Some(39));
    h.at_ms(1300);
    run(&mut h, &rows);
    h.at_ms(1400);
    run(&mut h, &rows);
    h.at_ms(1800);
    let f = run(&mut h, &rows);
    assert_eq!(top_of(&f.out, 39) + ROW, TREE.y + TREE.h);
    assert_eq!(f.out.repaint, Repaint::Idle);
}

// Only the rows in view.

#[test]
fn a_10000_row_tree_builds_only_the_rows_in_view() {
    let rows: Vec<TreeRow<'static>> = (0..10_000).map(|i| row(i, 0, "r", false)).collect();
    let mut h = UiHarness::new(WIN);
    let f = run(&mut h, &rows);
    assert_eq!(f.cells.len(), 10, "ten whole rows");
    assert_eq!(built(&f.out).len(), 10);
    let hits = (0..10_000)
        .filter(|i| h.store.last_rect(rid(*i)).is_some())
        .count();
    assert_eq!(hits, 10);
    h.input.pointer_to(50.0, 30.0);
    h.input.scroll_pixels(0.0, -14.0 - 28.0 * 5000.0);
    h.at_ms(10);
    run(&mut h, &rows);
    h.at_ms(20);
    let f = run(&mut h, &rows);
    assert_eq!(f.cells.len(), 11, "a cut row at each end");
    assert_eq!(f.cells[0], 5000);
    assert_eq!(top_of(&f.out, 5000), TREE.y - 14.0);
}

// The pointer.

#[test]
fn a_press_on_a_row_selects_it_once_and_a_press_on_the_chevron_only_toggles() {
    let mut h = UiHarness::new(WIN);
    run(&mut h, &ROWS);
    // On the text of a.
    h.input.pointer_to(TREE.x + 100.0, TREE.y + 10.0);
    h.at_ms(10);
    run(&mut h, &ROWS);
    h.input.press(P);
    h.at_ms(20);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.st.selected, BTreeSet::from([A]));
    assert_eq!(f.st.cursor, Some(A));
    assert!(f.r.response.changed);
    assert_eq!(f.r.toggled, None);
    h.input.release(P);
    h.at_ms(30);
    let f = run(&mut h, &ROWS);
    assert!(!f.r.response.changed, "the release selects nothing again");

    let f = click(&mut h, &ROWS, 1000, chevron_at(0, TREE.y + ROW));
    assert_eq!(f.r.toggled, Some(B));
    assert_eq!(
        f.st.selected,
        BTreeSet::from([A]),
        "the chevron does not select"
    );
    assert_eq!(f.st.cursor, Some(A));
    assert_eq!(f.out.focused, Some(tid()));
}

#[test]
fn a_press_on_a_leaf_where_a_chevron_would_be_selects() {
    let mut h = UiHarness::new(WIN);
    run(&mut h, &ROWS);
    let f = click(&mut h, &ROWS, 100, chevron_at(0, TREE.y + 2.0 * ROW));
    assert_eq!(f.r.toggled, None);
    assert_eq!(f.st.selected, BTreeSet::from([C]));
}

#[test]
fn toggle_on_row_toggles_from_anywhere_on_a_row_with_children() {
    let o = TreeOptions {
        toggle_on_row: true,
        ..opts(&ROWS)
    };
    let mut h = UiHarness::new(WIN);
    run_with(&mut h, &o, |_| {});
    h.input.pointer_to(TREE.x + 150.0, TREE.y + 10.0);
    h.at_ms(10);
    run_with(&mut h, &o, |_| {});
    h.input.click(P);
    h.at_ms(20);
    let f = run_with(&mut h, &o, |_| {});
    assert_eq!(f.r.toggled, Some(A));
    assert_eq!(f.st.selected, BTreeSet::from([A]));
    assert_eq!(f.st.expanded, BTreeSet::from([A]));

    // A leaf only selects, once the block has settled.
    h.at_ms(30);
    run_with(&mut h, &o, |_| {});
    h.at_ms(1000);
    run_with(&mut h, &o, |_| {});
    h.input
        .pointer_to(TREE.x + 150.0, TREE.y + 4.0 * ROW + 10.0);
    h.at_ms(1010);
    run_with(&mut h, &o, |_| {});
    h.input.click(P);
    h.at_ms(1020);
    let f = run_with(&mut h, &o, |_| {});
    assert_eq!(f.r.toggled, None);
    assert_eq!(f.st.selected, BTreeSet::from([C]));
}

fn double(h: &mut UiHarness, o: &TreeOptions<'_>, at: (f32, f32), ms: u64) -> (Run, Run) {
    h.input.pointer_to(at.0, at.1);
    h.at_ms(ms);
    run_with(h, o, |_| {});
    h.input.click(P);
    h.at_ms(ms + 10);
    let first = run_with(h, o, |_| {});
    h.input.click(P);
    h.at_ms(ms + 100);
    let second = run_with(h, o, |_| {});
    (first, second)
}

#[test]
fn a_double_click_activates_and_does_not_also_toggle() {
    let at = (TREE.x + 150.0, TREE.y + 10.0);
    let o = opts(&ROWS);
    let mut h = UiHarness::new(WIN);
    run_with(&mut h, &o, |_| {});
    let (first, second) = double(&mut h, &o, at, 100);
    assert_eq!(first.r.activated, None);
    assert_eq!(second.r.activated, Some(A));
    assert!(second.st.expanded.is_empty(), "no toggle at all");

    // With toggle_on_row the first click has toggled; the second does not toggle back.
    let o = TreeOptions {
        toggle_on_row: true,
        ..opts(&ROWS)
    };
    let mut h = UiHarness::new(WIN);
    run_with(&mut h, &o, |_| {});
    let (first, second) = double(&mut h, &o, at, 100);
    assert_eq!(first.r.toggled, Some(A));
    assert_eq!(second.r.activated, Some(A));
    assert_eq!(second.r.toggled, None);
    assert_eq!(second.st.expanded, BTreeSet::from([A]));
}

#[test]
fn two_quick_presses_on_a_chevron_toggle_twice_and_activate_nothing() {
    let o = opts(&ROWS);
    let mut h = UiHarness::new(WIN);
    run_with(&mut h, &o, |_| {});
    let (first, second) = double(&mut h, &o, chevron_at(0, TREE.y), 100);
    assert_eq!(first.r.toggled, Some(A));
    assert_eq!(second.r.toggled, Some(A));
    assert_eq!(second.r.activated, None);
    assert!(second.st.expanded.is_empty());
}

#[test]
fn a_secondary_click_reports_the_row() {
    let mut h = UiHarness::new(WIN);
    run(&mut h, &ROWS);
    h.input.pointer_to(TREE.x + 100.0, TREE.y + ROW + 5.0);
    h.at_ms(10);
    run(&mut h, &ROWS);
    h.input.click(PointerButton::Secondary);
    h.at_ms(20);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.r.context, Some(B));
    assert!(f.st.selected.is_empty());
}

#[test]
fn a_hovered_row_fills_and_a_selected_one_keeps_its_own_hover() {
    let fills = |f: &Run| -> Vec<[f32; 4]> {
        f.scene
            .in_order()
            .into_iter()
            .flat_map(|l| l.geometry.under.iter())
            .map(|s| s.color)
            .collect()
    };
    let mut h = UiHarness::new(WIN);
    run(&mut h, &ROWS);
    h.input.pointer_to(TREE.x + 100.0, TREE.y + 10.0);
    h.at_ms(10);
    run(&mut h, &ROWS);
    h.at_ms(20);
    let f = run(&mut h, &ROWS);
    assert!(fills(&f).contains(&premul(DARK.surface_hover)));
    h.at_ms(30);
    run_with(&mut h, &opts(&ROWS), |st| {
        st.selected.insert(A);
    });
    h.at_ms(1000);
    let f = run(&mut h, &ROWS);
    assert!(
        fills(&f).contains(&premul(DARK.surface_active)),
        "selected, hovered"
    );
}

// Still-frame rule.

#[test]
fn row_fills_tween_while_still_and_snap_while_the_content_scrolls() {
    let rows: Vec<TreeRow<'static>> = (0..100).map(|i| row(i, 0, "r", false)).collect();
    let selected = premul(DARK.surface_selected);
    let has = |f: &Run| {
        f.scene
            .in_order()
            .into_iter()
            .flat_map(|l| l.geometry.under.iter())
            .any(|s| s.color == selected)
    };
    let setup = || {
        let mut h = UiHarness::new(WIN);
        run(&mut h, &rows);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        h.at_ms(10);
        run(&mut h, &rows);
        h.at_ms(20);
        run(&mut h, &rows);
        h.at_ms(30);
        run(&mut h, &rows);
        h
    };
    let mut h = setup();
    h.at_ms(100);
    run_with(&mut h, &opts(&rows), |st| {
        st.selected.insert(1);
    });
    h.at_ms(101);
    let f = run(&mut h, &rows);
    assert!(!has(&f), "still: the selection fill tweens");

    let mut h = setup();
    h.input.pointer_to(50.0, 30.0);
    h.input.scroll_pixels(0.0, -7.0);
    h.at_ms(100);
    let f = run_with(&mut h, &opts(&rows), |st| {
        st.selected.insert(1);
    });
    assert!(has(&f), "moving: the fill snaps");
}

// Accessibility actions.

#[test]
fn expand_and_collapse_actions_toggle_and_default_selects() {
    let mut h = UiHarness::new(WIN);
    run(&mut h, &ROWS);
    access(&mut h, rid(A), Action::Expand);
    h.at_ms(10);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.r.toggled, Some(A));
    assert!(f.st.selected.is_empty(), "Expand selects nothing");
    h.at_ms(500);
    run(&mut h, &ROWS);
    access(&mut h, rid(A), Action::Expand);
    h.at_ms(510);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.r.toggled, None, "already open");
    access(&mut h, rid(A), Action::Collapse);
    h.at_ms(520);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.r.toggled, Some(A));
    h.at_ms(1000);
    run(&mut h, &ROWS);
    access(&mut h, rid(B), Action::Default);
    h.at_ms(1010);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.r.toggled, None);
    assert_eq!(f.st.selected, BTreeSet::from([B]));
    assert_eq!(f.st.cursor, Some(B));
}

#[test]
fn scroll_into_view_brings_a_built_row_to_the_centre() {
    let rows: Vec<TreeRow<'static>> = (0..100).map(|i| row(i, 0, "r", false)).collect();
    let mut h = UiHarness::new(WIN);
    run(&mut h, &rows);
    access(&mut h, rid(9), Action::ScrollIntoView);
    h.at_ms(10);
    run(&mut h, &rows);
    h.at_ms(600);
    let f = run(&mut h, &rows);
    let y = top_of(&f.out, 9);
    assert_eq!(y, TREE.y + (TREE.h - ROW) / 2.0, "centred");
}

// Focus.

#[test]
fn a_tab_cycle_enters_once_and_leaves() {
    let before = Rect::new(320.0, 20.0, 80.0, 30.0);
    let after = Rect::new(320.0, 60.0, 80.0, 30.0);
    let mut h = UiHarness::new(WIN);
    let page = |h: &mut UiHarness| {
        let (_, out) = h.frame(|ui| {
            ui.button("before", before, "Before", ButtonKind::Secondary);
            ui.tree_state("t").expanded.insert(A);
            ui.tree_view("t", TREE, &opts(&ROWS), |_, _| {});
            ui.button("after", after, "After", ButtonKind::Secondary);
        });
        out.focused
    };
    page(&mut h);
    let order = [
        WidgetId::ROOT.with("before"),
        tid(),
        WidgetId::ROOT.with("after"),
        WidgetId::ROOT.with("before"),
    ];
    for (i, want) in order.into_iter().enumerate() {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        h.at_ms(100 + i as u64 * 20);
        assert_eq!(page(&mut h), Some(want), "stop {i}");
    }
}

#[test]
fn focus_rests_on_the_selected_row_when_it_shows_and_rings_only_the_cursor() {
    let mut h = UiHarness::new(WIN);
    run_with(&mut h, &opts(&ROWS), |st| {
        st.selected.insert(B);
    });
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(10);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.st.cursor, Some(B));
    let ring = f
        .scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.geometry.over.iter().chain(l.geometry.under.iter()))
        .filter(|s| s.params != [0.0; 4])
        .count();
    assert_eq!(ring, 1, "one ring");
}

#[test]
fn an_assistive_technology_can_give_the_tree_the_focus() {
    let mut h = UiHarness::new(WIN);
    run(&mut h, &ROWS);
    access(&mut h, tid(), Action::Focus);
    h.at_ms(10);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.out.focused, Some(tid()));
    assert_eq!(f.st.cursor, Some(A));
}

// The disabled scope.

#[test]
fn a_disabled_tree_toggles_and_selects_nothing_takes_its_keys_and_passes_the_walk() {
    let keys = [
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::Home,
        NamedKey::End,
        NamedKey::PageUp,
        NamedKey::PageDown,
        NamedKey::Enter,
        NamedKey::Space,
    ];
    let mut h = focused(&[]);
    for k in keys {
        h.shortcuts.add(Binding::new(
            Scope::Global,
            Chord::key(Key::Named(k)),
            format!("g-{k:?}"),
        ));
    }
    let disabled = |h: &mut UiHarness| {
        let mut r = None;
        let mut st = None;
        let (_, out) = h.frame(|ui| {
            r = Some(ui.with_enabled(false, |ui| ui.tree_view("t", TREE, &opts(&ROWS), |_, _| {})));
            st = Some(ui.tree_state("t").clone());
        });
        (r.expect("ran"), st.expect("ran"), out)
    };
    for (i, k) in keys.into_iter().enumerate() {
        h.input.key_down(Key::Named(k), None);
        h.at_ms(100 + i as u64 * 10);
        let (r, st, out) = disabled(&mut h);
        assert!(out.fired.is_empty(), "{k:?}: {:?}", out.fired);
        assert_eq!((r.toggled, r.activated), (None, None), "{k:?}");
        assert!(st.selected.is_empty() && st.expanded.is_empty(), "{k:?}");
        assert_eq!(st.cursor, Some(A), "{k:?}");
    }
    h.input.pointer_to(TREE.x + 100.0, TREE.y + 10.0);
    h.at_ms(300);
    disabled(&mut h);
    h.input.click(P);
    h.at_ms(310);
    let (r, st, out) = disabled(&mut h);
    assert_eq!(r.toggled, None);
    assert!(st.selected.is_empty());
    assert_eq!(disabled_violations(&out.tree, tid()), []);
    assert_eq!(
        built(&out),
        ["a", "b", "c"],
        "the rows are still in the walk"
    );
}

#[test]
fn an_enabled_tree_leaves_a_global_binding_alone_on_keys_it_does_not_use() {
    let mut h = focused(&[]);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Character("x".into())),
        "x",
    ));
    h.input.key_down(Key::Character("x".into()), Some("x"));
    h.at_ms(100);
    let f = run(&mut h, &ROWS);
    assert_eq!(f.out.fired.len(), 1, "no type-ahead");
    assert_eq!(f.st.cursor, Some(A));
}

// R15: the guide styles, the disclosure mark and its turn.

/// `p` holds one parent with two leaves: one mark on the page.
const PK: u64 = 1;
const P_ROWS: [TreeRow<'static>; 3] = [
    row(1, 0, "p", true),
    row(2, 1, "c1", false),
    row(3, 1, "c2", false),
];

/// Four deep: each row is the only child of the one above.
const CHAIN: [TreeRow<'static>; 4] = [
    row(1, 0, "d0", true),
    row(2, 1, "d1", true),
    row(3, 2, "d2", true),
    row(4, 3, "d3", false),
];

fn styled<'a>(
    rows: &'a [TreeRow<'a>],
    guide_style: GuideStyle,
    chevron: ChevronStyle,
) -> TreeOptions<'a> {
    TreeOptions {
        guide_style,
        chevron,
        ..opts(rows)
    }
}

/// Every stroke vertex of the frame.
fn strokes(scene: &Scene) -> Vec<StrokeVertex> {
    scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.geometry.strokes.iter().copied())
        .collect()
}

/// The largest alpha among the frame's strokes; 0 when there are none.
fn guide_alpha(scene: &Scene) -> f32 {
    strokes(scene)
        .iter()
        .map(|v| v.color[3])
        .fold(0.0, f32::max)
}

/// The three corners of the first filled triangle, in device px.
fn first_triangle(scene: &Scene) -> [[f32; 2]; 3] {
    let v: Vec<[f32; 2]> = scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.geometry.fills.iter().map(|v| v.pos))
        .take(3)
        .collect();
    assert_eq!(v.len(), 3, "a filled mark");
    [v[0], v[1], v[2]]
}

/// The bounding box of `pts`: (left, top, right, bottom).
fn bounds(pts: &[[f32; 2]]) -> (f32, f32, f32, f32) {
    let f = |g: fn(f32, f32) -> f32, k: usize, init: f32| pts.iter().fold(init, |a, p| g(a, p[k]));
    (
        f(f32::min, 0, f32::INFINITY),
        f(f32::min, 1, f32::INFINITY),
        f(f32::max, 0, f32::NEG_INFINITY),
        f(f32::max, 1, f32::NEG_INFINITY),
    )
}

/// The triangle's turn in degrees: from the middle of its longest edge (the base) to the opposite
/// corner (the apex), measured clockwise from pointing right.
fn turn_degrees(t: [[f32; 2]; 3]) -> f32 {
    let d = |a: [f32; 2], b: [f32; 2]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
    let edges = [(0, 1, 2), (1, 2, 0), (2, 0, 1)];
    let (a, b, apex) = *edges
        .iter()
        .max_by(|x, y| d(t[x.0], t[x.1]).total_cmp(&d(t[y.0], t[y.1])))
        .expect("edges");
    let mid = [(t[a][0] + t[b][0]) / 2.0, (t[a][1] + t[b][1]) / 2.0];
    (t[apex][1] - mid[1])
        .atan2(t[apex][0] - mid[0])
        .to_degrees()
}

#[test]
fn each_guide_style_draws_what_it_says_at_depth_3() {
    let rows = |style| {
        let mut h = UiHarness::new(WIN);
        let o = styled(&CHAIN, style, ChevronStyle::Triangle);
        run_with(&mut h, &o, |st| st.expanded.extend([1, 2, 3])).scene
    };
    let row3 = TREE.y + 3.0 * ROW;
    let verticals = |scene: &Scene| -> Vec<f32> {
        let mut xs: Vec<f32> = segments(scene)
            .into_iter()
            .filter(|(a, b)| a[0] == b[0] && a[1] == row3 && b[1] == row3 + ROW)
            .map(|(a, _)| a[0])
            .collect();
        xs.sort_by(f32::total_cmp);
        xs
    };

    // Straight: exactly one full height line per depth above the row, at `k x 16 + 15.5`.
    let straight = rows(GuideStyle::Straight);
    let want: Vec<f32> = (0..3).map(|k| TREE.x + k as f32 * INDENT + 15.5).collect();
    assert_eq!(verticals(&straight), want);
    assert!(
        segments(&straight)
            .iter()
            .all(|(a, b)| a[0] == b[0] && b[1] > a[1]),
        "no elbow, no stub"
    );
    assert_eq!(straight_paths(3, ROW).len(), 3);
    assert!(straight_paths(0, ROW).is_empty());

    // Elbow: the last child's line turns into the stub, so only the row's own column has a
    // full height line... none: it stops at the middle of the row, then curves.
    let elbow = rows(GuideStyle::Elbow);
    assert!(
        verticals(&elbow).is_empty(),
        "the last child's line stops mid row"
    );
    assert!(
        segments(&elbow).iter().any(|(a, b)| a[1] == b[1]),
        "the stub runs along the row"
    );

    // OnHover with the pointer away draws no guide; that it fades in is below.
    assert!(strokes(&rows(GuideStyle::OnHover)).is_empty());
}

#[test]
fn the_default_guide_colour_is_the_token_not_the_border() {
    assert_ne!(DARK.tree_guide, DARK.border);
    assert_eq!(DARK.tree_guide, [0xFF, 0xFF, 0xFF, 0x14]);
    let mut h = UiHarness::new(WIN);
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.insert(A);
    });
    let guide = strokes(&f.scene);
    assert!(!guide.is_empty());
    assert!(guide.iter().all(|v| v.color == premul(DARK.tree_guide)));
    // The stub's cap too.
    let cap = f
        .scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.geometry.under.iter())
        .any(|s| s.color == premul(DARK.tree_guide));
    assert!(cap, "the stub's cap");

    // A theme that moves the token moves the guide, and the border does not reach it.
    let mut h = UiHarness::new(WIN);
    h.theme.tree_guide = [0x11, 0x22, 0x33, 0xFF];
    h.theme.border = [0x44, 0x55, 0x66, 0xFF];
    let f = run_with(&mut h, &opts(&ROWS), |st| {
        st.expanded.insert(A);
    });
    assert!(
        strokes(&f.scene)
            .iter()
            .all(|v| v.color == premul(h.theme.tree_guide))
    );
}

#[test]
fn guides_off_draw_no_guide_under_any_style() {
    for style in [GuideStyle::Elbow, GuideStyle::Straight, GuideStyle::OnHover] {
        let mut h = UiHarness::new(WIN);
        let o = TreeOptions {
            guides: false,
            ..styled(&CHAIN, style, ChevronStyle::Triangle)
        };
        run_with(&mut h, &o, |st| st.expanded.extend([1, 2, 3]));
        h.input.pointer_to(TREE.x + 100.0, TREE.y + 40.0);
        h.at_ms(300);
        let f = run_with(&mut h, &o, |_| {});
        assert!(strokes(&f.scene).is_empty(), "{style:?}");
    }
}

#[test]
fn on_hover_fades_the_elbow_in_with_the_pointer_and_out_without() {
    let o = styled(&CHAIN, GuideStyle::OnHover, ChevronStyle::Triangle);
    let full = f32::from(DARK.tree_guide[3]) / 255.0;
    let mut h = UiHarness::new(WIN);
    let f = run_with(&mut h, &o, |st| st.expanded.extend([1, 2, 3]));
    assert_eq!(guide_alpha(&f.scene), 0.0, "pointer away: no guide");
    run_with(&mut h, &o, |_| {});

    h.input.pointer_to(TREE.x + 100.0, TREE.y + 40.0);
    h.at_ms(100);
    run_with(&mut h, &o, |_| {});
    h.at_ms(160);
    let mid = run_with(&mut h, &o, |_| {});
    let a = guide_alpha(&mid.scene);
    assert!(a > 0.0 && a < full - 0.005, "fading in: {a} of {full}");
    assert_eq!(mid.out.repaint, Repaint::Now);
    h.at_ms(400);
    let f = run_with(&mut h, &o, |_| {});
    assert!((guide_alpha(&f.scene) - full).abs() < 0.003, "fully in");
    assert!(
        strokes(&f.scene).iter().all(|v| v.color[3] <= full + 1e-4),
        "never brighter than the token"
    );

    h.input.pointer_to(480.0, 380.0);
    h.at_ms(500);
    run_with(&mut h, &o, |_| {});
    h.at_ms(700);
    let f = run_with(&mut h, &o, |_| {});
    assert_eq!(guide_alpha(&f.scene), 0.0, "pointer gone: faded out");
}

#[test]
fn on_hover_reduced_fades_and_off_snaps() {
    let o = styled(&CHAIN, GuideStyle::OnHover, ChevronStyle::Triangle);
    let full = f32::from(DARK.tree_guide[3]) / 255.0;
    for (tier, snaps) in [(MotionTier::Reduced, false), (MotionTier::Off, true)] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        run_with(&mut h, &o, |st| st.expanded.extend([1, 2, 3]));
        h.input.pointer_to(TREE.x + 100.0, TREE.y + 40.0);
        h.at_ms(100);
        run_with(&mut h, &o, |_| {});
        h.at_ms(160);
        let f = run_with(&mut h, &o, |_| {});
        let a = guide_alpha(&f.scene);
        if snaps {
            assert!((a - full).abs() < 0.003, "{tier:?}: snapped to {a}");
            assert_eq!(f.out.repaint, Repaint::Idle, "{tier:?}");
        } else {
            assert!(a > 0.0 && a < full - 0.005, "{tier:?}: fading {a}");
        }
    }
}

#[test]
fn the_triangle_records_a_fill_and_no_glyph_and_sits_on_the_label_line() {
    let mut h = UiHarness::new(WIN);
    let o = TreeOptions {
        guides: false,
        ..opts(&P_ROWS)
    };
    let f = run_with(&mut h, &o, |_| {});
    // One fill, no stroke, and a text for each row shown (the one row), none for the mark.
    assert!(strokes(&f.scene).is_empty());
    let texts: usize = f.scene.in_order().into_iter().map(|l| l.texts.len()).sum();
    assert_eq!(texts, 1, "the row's label only");
    let t = first_triangle(&f.scene);
    let (l, top, r, b) = bounds(&t);
    assert_eq!(
        (r - l, b - top),
        (5.5, 7.5),
        "closed: 5.5 wide, 7.5 tall, pointing right"
    );
    // On guide column 0, so a guide drops from the triangle's centre.
    assert_eq!((l + r) / 2.0, TREE.x + column(0));
    assert_eq!((top + b) / 2.0, TREE.y + ROW / 2.0 + MARK_DROP);
    assert_eq!(turn_degrees(t), 0.0);
    let colour = f.scene.in_order()[0].geometry.fills[0].color;
    assert_eq!(colour, premul(DARK.fg_tertiary));
}

#[test]
fn the_mark_turns_to_the_block_and_reaches_its_end_in_full_and_snaps_in_reduced_and_off() {
    let o = opts(&P_ROWS);
    let mut h = UiHarness::new(WIN);
    run_with(&mut h, &o, |_| {});
    h.at_ms(1000);
    let f = run_with(&mut h, &o, |st| {
        st.expanded.insert(PK);
    });
    assert_eq!(turn_degrees(first_triangle(&f.scene)), 0.0, "starts closed");
    h.at_ms(1125);
    let f = run_with(&mut h, &o, |_| {});
    let mid = turn_degrees(first_triangle(&f.scene));
    assert!((mid - 45.0).abs() < 1.0, "half way, sine-in-out: {mid}");
    h.at_ms(1250);
    let f = run_with(&mut h, &o, |_| {});
    let t = first_triangle(&f.scene);
    let (l, top, r, b) = bounds(&t);
    assert_eq!(turn_degrees(t), 90.0, "ends pointing down");
    assert_eq!((r - l, b - top), (7.5, 5.5));

    // Closing runs it back.
    h.at_ms(2000);
    let f = run_with(&mut h, &o, |st| {
        st.expanded.remove(&PK);
    });
    assert_eq!(turn_degrees(first_triangle(&f.scene)), 90.0);
    h.at_ms(2125);
    let f = run_with(&mut h, &o, |_| {});
    let mid = turn_degrees(first_triangle(&f.scene));
    assert!((mid - 45.0).abs() < 1.0, "back half way: {mid}");
    h.at_ms(2250);
    let f = run_with(&mut h, &o, |_| {});
    assert_eq!(turn_degrees(first_triangle(&f.scene)), 0.0);

    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        run_with(&mut h, &o, |_| {});
        h.at_ms(1000);
        let f = run_with(&mut h, &o, |st| {
            st.expanded.insert(PK);
        });
        // The frame of the change already shows the end, at exactly 90 deg: no transform.
        let t = first_triangle(&f.scene);
        assert_eq!(turn_degrees(t), 90.0, "{tier:?}");
        let (l, top, r, b) = bounds(&t);
        assert_eq!((r - l, b - top), (7.5, 5.5), "{tier:?}");
    }
}

#[test]
fn the_chevron_is_a_thin_stroke_that_brightens_on_row_hover() {
    let rows = [row(1, 0, "p", true), row(2, 0, "q", true)];
    let o = TreeOptions {
        guides: false,
        ..styled(&rows, GuideStyle::Elbow, ChevronStyle::Chevron)
    };
    let mut h = UiHarness::new(WIN);
    run_with(&mut h, &o, |_| {});
    // Hover the first row's text.
    h.input.pointer_to(TREE.x + 120.0, TREE.y + ROW / 2.0);
    h.at_ms(100);
    run_with(&mut h, &o, |_| {});
    h.at_ms(400);
    let f = run_with(&mut h, &o, |_| {});
    let all = strokes(&f.scene);
    assert!(!all.is_empty());
    assert!(
        f.scene
            .in_order()
            .iter()
            .all(|l| l.geometry.fills.is_empty()),
        "a stroke, not a fill"
    );
    let row_of = |v: &StrokeVertex| (v.pos[1] > TREE.y + ROW) as usize;
    let colour = |r: usize| {
        all.iter()
            .find(|v| row_of(v) == r)
            .expect("a chevron")
            .color
    };
    assert_eq!(colour(0), premul(DARK.fg_secondary), "the hovered row");
    assert_eq!(colour(1), premul(DARK.fg_tertiary), "a resting row");
    assert!(colour(0)[3] > colour(1)[3]);
    // 1.25 px wide: the half width of the stroke vertices is 0.625.
    assert!(
        (all[0].style[1] - 0.625).abs() < 1e-4,
        "{}",
        all[0].style[1]
    );
    // Its ink lies inside a 10 px box about the slot's centre; a vertex is one px past the ink
    // (the antialiasing fringe).
    let (cx, cy) = (
        TREE.x + PAD_LEFT + CHEVRON / 2.0,
        TREE.y + ROW / 2.0 + MARK_DROP,
    );
    for v in all.iter().filter(|v| row_of(v) == 0) {
        assert!(
            (v.pos[0] - cx).abs() <= 6.0 && (v.pos[1] - cy).abs() <= 6.0,
            "{:?} about {:?}",
            v.pos,
            (cx, cy)
        );
    }
}

#[test]
fn the_chevron_turns_with_the_block_too() {
    let rows = [row(1, 0, "p", true), row(2, 1, "c", false)];
    let o = TreeOptions {
        guides: false,
        ..styled(&rows, GuideStyle::Elbow, ChevronStyle::Chevron)
    };
    // The chevron's three points are the stroke's two segments' ends: the apex is the vertex with
    // the largest distance along the pointing direction; the arms' middle is the base mid.
    let tip = |scene: &Scene| -> f32 {
        let v = strokes(scene);
        let c = (
            TREE.x + PAD_LEFT + CHEVRON / 2.0,
            TREE.y + ROW / 2.0 + MARK_DROP,
        );
        let (l, t, r, b) = bounds(&v.iter().map(|q| q.pos).collect::<Vec<_>>());
        // Wider than tall pointing down (8 x 5 ink), taller than wide pointing right (5 x 8).
        let _ = c;
        (r - l) - (b - t)
    };
    let mut h = UiHarness::new(WIN);
    run_with(&mut h, &o, |_| {});
    h.at_ms(1000);
    let closed = run_with(&mut h, &o, |st| {
        st.expanded.insert(1);
    });
    assert!(tip(&closed.scene) < 0.0, "pointing right: taller than wide");
    h.at_ms(1250);
    let open = run_with(&mut h, &o, |_| {});
    assert!(tip(&open.scene) > 0.0, "pointing down: wider than tall");
}

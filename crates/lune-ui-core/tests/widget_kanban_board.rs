//! The kanban board (`data.kanban-board` acceptance): the anatomy and the tree, the pointer, every
//! way a drag can end and a card can jump, the keys, the ghost's layer, the auto-scroll, the
//! motion in three tiers and the probes at scale 2. Scripted input, injected time.

use std::collections::BTreeMap;

use lune_ui_core::access::{AccessAction, Action, Live, Role, State};
use lune_ui_core::color::{Rgba, premul};
use lune_ui_core::drag::{DIM, DIM_ALPHA, SETTLE, SLIDE};
use lune_ui_core::input::{FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{LayerId, Scene, Shape};
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Ui, WidgetId};
use lune_ui_core::widgets::dialog::{Cancel, Dialog, DialogGlass};
use lune_ui_core::widgets::kanban_board::{
    BoardError, BoardOptions, BoardResponse, CARD_GAP, CARD_INNER, CARD_LIST_MAX, CARD_W,
    COLUMN_GAP, COLUMN_W, CardMove, FOOTER, HEADER_GAP, KanbanCard, KanbanColumn, MARGIN,
    STACK_GAP, card_height, landed_index,
};
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};

const WIN: (f32, f32) = (900.0, 600.0);
const BOARD: Rect = Rect::new(10.0, 10.0, 880.0, 560.0);
const P: PointerButton = PointerButton::Primary;
/// The cards' first slot, and the pitch of cards with one title line and no footer.
const LIST_Y: f32 = 103.0;
const CARD_H: f32 = 45.0;
const PITCH: f32 = CARD_H + CARD_GAP;
const LANE_X: f32 = 22.0;
const LANE_PITCH: f32 = COLUMN_W + COLUMN_GAP;
const CARD_X: f32 = LANE_X + 11.0;
/// A column's height around its list: borders, padding, header and its gap.
const CHROME: f32 = 54.0;

#[derive(Debug, Clone)]
struct Model {
    cols: Vec<(u64, Vec<u64>)>,
    error: bool,
    enabled: bool,
    long: Option<u64>,
    in_dialog: Option<bool>,
    /// A modal opened over the board: its regions fall out of reach and the capture ends.
    modal: bool,
    /// The board is drawn under a translation of this many px (a parent that moves its content).
    shift: Option<(f32, f32)>,
}

/// Three columns and an empty fourth.
fn model() -> Model {
    Model {
        cols: vec![
            (1, vec![11, 12, 13]),
            (2, vec![21, 22]),
            (3, vec![31]),
            (4, vec![]),
        ],
        error: false,
        enabled: true,
        long: None,
        in_dialog: None,
        modal: false,
        shift: None,
    }
}

fn tall() -> Model {
    Model {
        cols: vec![(1, (100..130).collect()), (2, vec![21])],
        ..model()
    }
}

fn wide() -> Model {
    Model {
        cols: (1..=6).map(|c| (c, vec![c * 10 + 1])).collect(),
        ..model()
    }
}

fn apply(m: &mut Model, mv: CardMove) {
    let (from, at) = m
        .cols
        .iter()
        .enumerate()
        .find_map(|(c, col)| Some((c, col.1.iter().position(|k| *k == mv.card)?)))
        .expect("the moved card");
    let to = m
        .cols
        .iter()
        .position(|c| c.0 == mv.column)
        .expect("the target column");
    let index = landed_index((from == to).then_some(at), mv.index);
    let card = m.cols[from].1.remove(at);
    m.cols[to].1.insert(index, card);
}

fn title(k: u64) -> String {
    format!("card {k}")
}

const ERROR: BoardError<'static> = BoardError {
    title: "Roadmap",
    message: "Cannot read it",
    path: "/tmp/board.json",
};

fn glass() -> DialogGlass {
    DialogGlass {
        fill: [0.0; 4],
        noise: false,
    }
}

fn dialog() -> Dialog<'static> {
    Dialog {
        title: "t",
        message: "m",
        extra_h: 300.0,
        confirm: "OK",
        cancel: Cancel::Default,
    }
}

fn show(ui: &mut Ui<'_>, m: &Model, rect: Rect) -> BoardResponse {
    let titles: BTreeMap<u64, String> = m
        .cols
        .iter()
        .flat_map(|c| c.1.iter())
        .map(|k| {
            let t = if m.long == Some(*k) {
                "a long title ".repeat(12)
            } else {
                title(*k)
            };
            (*k, t)
        })
        .collect();
    let cards: Vec<Vec<KanbanCard<'_>>> = m
        .cols
        .iter()
        .map(|c| {
            c.1.iter()
                .map(|k| KanbanCard {
                    key: *k,
                    title: &titles[k],
                    tags: &[],
                    assignee: None,
                    note: false,
                })
                .collect()
        })
        .collect();
    let names: Vec<String> = m.cols.iter().map(|c| format!("col {}", c.0)).collect();
    let columns: Vec<KanbanColumn<'_>> = m
        .cols
        .iter()
        .zip(&cards)
        .zip(&names)
        .map(|((c, cards), title)| KanbanColumn {
            key: c.0,
            title,
            cards,
        })
        .collect();
    let opts = BoardOptions {
        title: "Roadmap",
        columns: &columns,
        error: m.error.then_some(&ERROR),
        glass: lune_ui_core::GlassLayer {
            tint: [0, 0, 0, 255],
            tint_opacity: 0.5,
            material_opacity: 0.5,
            curve: lune_ui_core::LuminosityCurve::AVALONIA,
        },
        under: [0.0; 4],
        name: "",
    };
    ui.with_enabled(m.enabled, |ui| ui.kanban_board("b", rect, &opts))
}

/// One frame's results.
struct Run {
    scene: Scene,
    out: FrameOutput,
    r: BoardResponse,
    /// The dialog around the board (if any) reported a cancel this frame.
    dialog_cancelled: bool,
}

fn run(h: &mut UiHarness, m: &Model, ms: u64) -> Run {
    h.at_ms(ms);
    let mut r = None;
    let mut dialog_cancelled = false;
    let (scene, out) = h.frame(|ui| {
        ui.painter().on(LayerId::Page);
        match m.in_dialog {
            None => {
                if let Some((dx, dy)) = m.shift {
                    ui.painter().push_translate(dx, dy);
                }
                r = Some(show(ui, m, BOARD));
                if m.shift.is_some() {
                    ui.painter().pop_translate();
                }
                if m.modal {
                    ui.dialog("over", &dialog(), &glass(), |_, _, _, _| {});
                }
            }
            Some(open) => {
                if open {
                    dialog_cancelled = ui
                        .dialog("d", &dialog(), &glass(), |ui, x, y, _| {
                            r = Some(show(ui, m, Rect::new(x, y, 400.0, 300.0)));
                        })
                        .cancelled;
                }
            }
        }
    });
    let r = r.unwrap_or_else(|| {
        // The dialog is closed: a response nothing reads.
        let mut h2 = UiHarness::new(WIN);
        let mut r = None;
        h2.frame(|ui| r = Some(show(ui, &model(), BOARD)));
        r.expect("a response")
    });
    Run {
        scene,
        out,
        r,
        dialog_cancelled,
    }
}

fn bid() -> WidgetId {
    WidgetId::ROOT.with("b")
}

fn cid(key: u64) -> WidgetId {
    bid().with(("card", key))
}

fn col_id(key: u64) -> WidgetId {
    bid().with(("column", key))
}

fn bounds(out: &FrameOutput, key: u64) -> Rect {
    out.tree.get(cid(key)).expect("a card").bounds
}

fn center(out: &FrameOutput, key: u64) -> (f32, f32) {
    let b = bounds(out, key);
    (b.x + b.w / 2.0, b.y + b.h / 2.0)
}

/// Every drawing of `key`'s title in `scene`: layer, position and alpha.
fn drawn(scene: &Scene, key: u64, scale: f32) -> Vec<(usize, f32, f32, u8)> {
    let mut keys = BTreeMap::new();
    for a in 0..=255u8 {
        let mut style = Style::ui(DARK.font_base, DARK.fg_primary);
        style.color[3] = a;
        let spec = Spec::plain(title(key), style).wrapped(CARD_INNER);
        keys.insert(spec.cache_key(scale), a);
    }
    let mut found = Vec::new();
    for (l, layer) in scene.in_order().into_iter().enumerate() {
        for t in &layer.texts {
            if let Some(a) = keys.get(&t.key.0) {
                found.push((l, t.x, t.y, *a));
            }
        }
    }
    found
}

/// The under shapes of `layer` with the colour `c`, as drawn.
fn fills(scene: &Scene, layer: LayerId, c: Rgba) -> Vec<Shape> {
    scene
        .merged(layer)
        .geometry
        .under
        .iter()
        .filter(|s| s.color == premul(c))
        .copied()
        .collect()
}

/// The ghost's frost on `layer`: the `menu_glass` one there (D168: the ghost stands on glass).
fn ghost_on(scene: &Scene, layer: LayerId) -> Option<Shape> {
    scene
        .merged(layer)
        .geometry
        .under
        .iter()
        .find(|s| s.is_frost() && s.color == premul(DARK.menu_glass))
        .copied()
}

fn ghost(scene: &Scene) -> Option<Shape> {
    ghost_on(scene, LayerId::Popup)
}

/// Presses on `key` and drags 10 px down: frames at `ms`, +10, +20; the frame before sets the
/// regions.
fn grab(h: &mut UiHarness, m: &Model, key: u64, ms: u64) -> Run {
    let first = run(h, m, ms);
    let (x, y) = center(&first.out, key);
    h.input.pointer_to(x, y);
    run(h, m, ms + 10);
    h.input.press(P);
    run(h, m, ms + 20);
    h.input.pointer_to(x, y + 10.0);
    run(h, m, ms + 30)
}

fn drag_to(h: &mut UiHarness, m: &Model, ms: u64, at: (f32, f32)) -> Run {
    h.input.pointer_to(at.0, at.1);
    run(h, m, ms)
}

fn release(h: &mut UiHarness, m: &Model, ms: u64) -> Run {
    h.input.release(P);
    run(h, m, ms)
}

fn press_key(h: &mut UiHarness, k: NamedKey) {
    h.input.key_down(Key::Named(k), None);
}

fn shift_key(h: &mut UiHarness, k: NamedKey) {
    h.input.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    press_key(h, k);
    h.input.set_modifiers(Modifiers::default());
}

fn access(h: &mut UiHarness, target: WidgetId, action: Action) {
    h.input.access(AccessAction {
        target,
        action,
        value: None,
    });
}

fn focus(h: &mut UiHarness, m: &Model, key: u64, ms: u64) {
    run(h, m, ms);
    access(h, cid(key), Action::Focus);
    run(h, m, ms + 1);
    run(h, m, ms + 2);
}

/// Runs frames until a frame reports `Idle`, at most `limit`, from `ms`; returns the last.
fn settle(h: &mut UiHarness, m: &Model, ms: u64, limit: u64) -> Run {
    let mut r = run(h, m, ms);
    for k in 1..limit {
        if r.out.repaint == Repaint::Idle {
            return r;
        }
        r = run(h, m, ms + k * 17);
    }
    r
}

// The model's index.

#[test]
fn the_landed_index_pins_both_directions() {
    // Down: into index 3 of [a b c d] from 1 counts the list as it stands, and lands at 2.
    assert_eq!(landed_index(Some(1), 3), 2);
    assert_eq!(landed_index(Some(1), 4), 3);
    // Up, and at its own index, nothing changes.
    assert_eq!(landed_index(Some(3), 1), 1);
    assert_eq!(landed_index(Some(2), 2), 2);
    // Another column.
    assert_eq!(landed_index(None, 3), 3);
    assert_eq!(landed_index(None, 0), 0);
}

// The anatomy.

#[test]
fn a_card_and_a_column_sit_at_the_anatomy_s_numbers() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    let r = run(&mut h, &m, 0);
    assert_eq!(MARGIN + 28.0 + HEADER_GAP, 50.0);
    let col = r.out.tree.get(col_id(2)).expect("a column").bounds;
    assert_eq!((col.x, col.y, col.w), (LANE_X + LANE_PITCH, 60.0, COLUMN_W));
    assert_eq!(CARD_W, 250.0);
    assert_eq!(CARD_INNER, 228.0);
    let b = bounds(&r.out, 12);
    assert_eq!((b.x, b.y, b.w), (CARD_X, LIST_Y + PITCH, CARD_W));
    assert_eq!(b.h, CARD_H);
}

#[test]
fn a_title_wraps_at_the_card_s_inner_width_and_grows_the_card() {
    let mut h = UiHarness::new(WIN);
    let m = Model {
        long: Some(11),
        ..model()
    };
    let r = run(&mut h, &m, 0);
    assert!(bounds(&r.out, 11).h > CARD_H + 16.0);
    assert_eq!(bounds(&r.out, 11).w, CARD_W);
    assert_eq!(
        bounds(&r.out, 12).y,
        LIST_Y + bounds(&r.out, 11).h + CARD_GAP
    );
}

#[test]
fn tags_flow_and_the_footer_has_its_row() {
    let plain = KanbanCard {
        key: 1,
        title: "card",
        tags: &[],
        assignee: None,
        note: false,
    };
    // 9 px text at 4.5 a glyph: "tag" chips are 25.5 wide with 4 between, eight to a row of 228.
    const ONE_ROW: [&str; 7] = ["tag"; 7];
    const TWO_ROWS: [&str; 9] = ["tag"; 9];
    let mut h = UiHarness::new(WIN);
    let mut sizes = Vec::new();
    h.frame(|ui| {
        let p = ui.painter();
        for card in [
            plain,
            KanbanCard {
                assignee: Some("Jo"),
                ..plain
            },
            KanbanCard {
                note: true,
                ..plain
            },
            KanbanCard {
                tags: &ONE_ROW,
                ..plain
            },
            KanbanCard {
                tags: &TWO_ROWS,
                ..plain
            },
        ] {
            sizes.push(card_height(p, &card, &DARK));
        }
    });
    let chip = 12.0 + 2.0 + 2.0;
    assert_eq!(
        sizes,
        [
            CARD_H,
            CARD_H + FOOTER,
            CARD_H + FOOTER,
            CARD_H + STACK_GAP + chip,
            CARD_H + STACK_GAP + 2.0 * chip
        ]
    );
}

#[test]
fn a_column_hugs_its_cards_then_its_list_scrolls() {
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, &model(), 0);
    let hug = r.out.tree.get(col_id(3)).expect("a column").bounds.h;
    assert_eq!(hug, CHROME + PITCH);
    let m = tall();
    let r = run(&mut h, &m, 10);
    let full = r.out.tree.get(col_id(1)).expect("a column").bounds.h;
    // The room left in the board, under the 640 cap.
    assert_eq!(full, 498.0);
    let mut h = UiHarness::new((900.0, 1200.0));
    h.at_ms(0);
    let mut seen = None;
    let (_, out) = h.frame(|ui| {
        seen = Some(show(ui, &m, Rect::new(10.0, 10.0, 880.0, 1150.0)));
    });
    assert_eq!(
        out.tree.get(col_id(1)).expect("a column").bounds.h,
        CHROME + CARD_LIST_MAX
    );
}

#[test]
fn an_empty_column_shows_the_hint() {
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, &model(), 0);
    let hint = r
        .out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::StaticText && n.name == "No cards yet.")
        .expect("the hint");
    assert_eq!(hint.bounds.x, CARD_X + 3.0 * LANE_PITCH + 2.0);
    assert_eq!(hint.bounds.y, LIST_Y + 6.0);
}

#[test]
fn the_error_state_is_all_there_is_and_it_is_assertive() {
    let mut h = UiHarness::new(WIN);
    let m = Model {
        error: true,
        ..model()
    };
    let r = run(&mut h, &m, 0);
    let alert = r.out.tree.get(bid()).expect("the alert");
    assert_eq!(alert.role, Role::Alert);
    assert_eq!(alert.live, Some(Live::Assertive));
    assert_eq!(alert.name, "Roadmap");
    assert_eq!(alert.description.as_deref(), Some("Cannot read it"));
    assert!(alert.bounds.w <= 420.0);
    let roles = |role| r.out.tree.nodes.iter().filter(|n| n.role == role).count();
    assert_eq!(
        roles(Role::ListItem) + roles(Role::Group) + roles(Role::Button),
        0
    );
    assert!(
        r.out
            .tree
            .nodes
            .iter()
            .any(|n| n.role == Role::StaticText && n.name == "/tmp/board.json")
    );
}

// The tree.

#[test]
fn the_tree_is_a_list_of_groups_of_items() {
    let mut h = UiHarness::new(WIN);
    let r = run(&mut h, &model(), 0);
    let list = r.out.tree.get(bid()).expect("the list");
    assert_eq!((list.role, list.name.as_str()), (Role::List, "Roadmap"));
    let col = r.out.tree.get(col_id(1)).expect("a column");
    assert_eq!((col.role, col.name.as_str()), (Role::Group, "col 1"));
    assert_eq!(col.description.as_deref(), Some("3"));
    let card = r.out.tree.get(cid(11)).expect("a card");
    assert_eq!((card.role, card.name.as_str()), (Role::ListItem, "card 11"));
    assert!(card.states.contains(State::Focusable));
    for a in [
        Action::Default,
        Action::Focus,
        Action::ScrollIntoView,
        Action::ShowMenu,
    ] {
        assert!(card.actions.contains(a), "{a:?}");
    }
    let buttons: Vec<&str> = r
        .out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Button)
        .map(|n| n.name.as_str())
        .collect();
    assert_eq!(buttons[..3], ["Add column", "Add card", "Column actions"]);
    assert_eq!(buttons.len(), 1 + 2 * 4);
}

#[test]
fn a_name_given_wins_over_the_title() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    let (_, out) = h.frame(|ui| {
        let opts = BoardOptions {
            title: "Roadmap",
            columns: &[],
            error: None,
            glass: lune_ui_core::GlassLayer {
                tint: [0, 0, 0, 255],
                tint_opacity: 0.5,
                material_opacity: 0.5,
                curve: lune_ui_core::LuminosityCurve::AVALONIA,
            },
            under: [0.0; 4],
            name: "Plan",
        };
        ui.kanban_board("b", BOARD, &opts);
    });
    assert_eq!(out.tree.get(bid()).expect("the list").name, "Plan");
}

#[test]
fn the_disabled_walk_is_clean_and_nothing_activates() {
    let mut h = UiHarness::new(WIN);
    let m = Model {
        enabled: false,
        ..model()
    };
    let r = run(&mut h, &m, 0);
    assert_eq!(disabled_violations(&r.out.tree, bid()), []);
    let at = center(&r.out, 11);
    h.input.pointer_to(at.0, at.1);
    run(&mut h, &m, 10);
    h.input.click(P);
    let r = run(&mut h, &m, 20);
    assert_eq!(r.r.opened, None);
    access(&mut h, cid(11), Action::Default);
    let r = run(&mut h, &m, 30);
    assert_eq!(r.r.opened, None);
}

// The pointer.

#[test]
fn a_press_under_the_threshold_is_a_click_and_opens() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    let r = run(&mut h, &m, 0);
    let (x, y) = center(&r.out, 12);
    h.input.pointer_to(x, y);
    run(&mut h, &m, 10);
    h.input.press(P);
    run(&mut h, &m, 20);
    h.input.pointer_to(x + 2.0, y + 2.0);
    let r = run(&mut h, &m, 30);
    assert!(drawn(&r.scene, 12, 1.0).len() == 1, "no drag yet");
    let r = release(&mut h, &m, 40);
    assert_eq!(r.r.opened, Some(12));
    assert_eq!(r.r.moved, None);
    // A press and its release inside one frame.
    h.input.click(P);
    let r = run(&mut h, &m, 600);
    assert_eq!(r.r.opened, Some(12));
    let r = run(&mut h, &m, 610);
    assert_eq!(r.r.opened, None);
}

#[test]
fn the_default_action_opens_and_show_menu_reports_context() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    run(&mut h, &m, 0);
    access(&mut h, cid(21), Action::Default);
    let r = run(&mut h, &m, 10);
    assert_eq!(r.r.opened, Some(21));
    access(&mut h, cid(21), Action::ShowMenu);
    let r = run(&mut h, &m, 20);
    let b = bounds(&r.out, 21);
    assert_eq!(r.r.context, Some((21, b)));
}

#[test]
fn a_secondary_click_reports_context() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    let r = run(&mut h, &m, 0);
    let (x, y) = center(&r.out, 13);
    h.input.pointer_to(x, y);
    run(&mut h, &m, 10);
    h.input.click(PointerButton::Secondary);
    let r = run(&mut h, &m, 20);
    assert_eq!(r.r.context.map(|c| c.0), Some(13));
    assert_eq!(r.r.opened, None);
}

#[test]
fn the_buttons_report_their_presses() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    let r = run(&mut h, &m, 0);
    let named = |name: &str, nth: usize| {
        r.out
            .tree
            .nodes
            .iter()
            .filter(|n| n.name == name)
            .nth(nth)
            .expect("a button")
            .id
    };
    let (add_col, add, menu) = (
        named("Add column", 0),
        named("Add card", 1),
        named("Column actions", 2),
    );
    access(&mut h, add_col, Action::Default);
    assert!(run(&mut h, &m, 10).r.add_column);
    access(&mut h, add, Action::Default);
    assert_eq!(run(&mut h, &m, 20).r.add_card, Some(2));
    access(&mut h, menu, Action::Default);
    let r = run(&mut h, &m, 30);
    assert_eq!(r.r.column_menu.map(|c| c.0), Some(3));
}

#[test]
fn a_move_across_columns_lands_in_the_model_where_the_line_said() {
    let mut h = UiHarness::new(WIN);
    let mut m = model();
    grab(&mut h, &m, 11, 0);
    // Between card 21 and card 22.
    let at = (CARD_X + LANE_PITCH + 50.0, LIST_Y + PITCH - 3.0);
    let r = drag_to(&mut h, &m, 50, at);
    assert!(
        r.out
            .tree
            .get(cid(11))
            .is_some_and(|n| n.states.contains(State::Dragging))
    );
    let r = release(&mut h, &m, 60);
    let mv = r.r.moved.expect("a move");
    assert!(r.r.response.changed);
    assert_eq!(
        mv,
        CardMove {
            card: 11,
            column: 2,
            index: 1
        }
    );
    apply(&mut m, mv);
    assert_eq!(m.cols[1].1, [21, 11, 22]);
    let r = run(&mut h, &m, 70);
    assert!(
        !r.out
            .tree
            .get(cid(11))
            .expect("a card")
            .states
            .contains(State::Dragging)
    );
}

#[test]
fn moves_down_and_up_its_own_column_match_the_model() {
    let mut h = UiHarness::new(WIN);
    let mut m = model();
    grab(&mut h, &m, 11, 0);
    // Under card 13: index 3 as the list stands, landing last.
    let r = drag_to(&mut h, &m, 50, (CARD_X + 50.0, LIST_Y + 2.0 * PITCH + 40.0));
    assert!(r.r.moved.is_none());
    let mv = release(&mut h, &m, 60).r.moved.expect("a move");
    assert_eq!(mv.index, 3);
    apply(&mut m, mv);
    assert_eq!(m.cols[0].1, [12, 13, 11]);
    settle(&mut h, &m, 70, 40);

    grab(&mut h, &m, 11, 2000);
    let r = drag_to(&mut h, &m, 2050, (CARD_X + 50.0, LIST_Y + 2.0));
    assert!(r.r.moved.is_none());
    let mv = release(&mut h, &m, 2060).r.moved.expect("a move");
    assert_eq!(mv.index, 0);
    apply(&mut m, mv);
    assert_eq!(m.cols[0].1, [11, 12, 13]);
}

#[test]
fn a_drop_at_its_own_place_or_outside_every_column_reports_nothing() {
    let m = model();
    let spots = [
        // Its own index, and the next.
        (CARD_X + 50.0, LIST_Y + PITCH + 2.0),
        (CARD_X + 50.0, LIST_Y + PITCH + CARD_H - 2.0),
        // The gap between two columns.
        (LANE_X + COLUMN_W + 5.0, LIST_Y + 10.0),
        // Above the board's viewport, over the first column's x.
        (CARD_X + 50.0, 30.0),
        // Past the last column.
        (LANE_X + 4.0 * LANE_PITCH + 2.0, LIST_Y),
    ];
    for at in spots {
        let mut h = UiHarness::new(WIN);
        grab(&mut h, &m, 12, 0);
        drag_to(&mut h, &m, 50, at);
        let r = release(&mut h, &m, 60);
        assert_eq!(r.r.moved, None, "{at:?}");
        assert_eq!(r.r.opened, None, "{at:?}");
    }
}

#[test]
fn the_drop_line_sits_in_the_gap_of_the_slot_and_the_target_fills() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    grab(&mut h, &m, 11, 0);
    let at = (CARD_X + LANE_PITCH + 50.0, LIST_Y + PITCH - 3.0);
    drag_to(&mut h, &m, 50, at);
    let r = settle(&mut h, &m, 60, 40);
    let line = fills(&r.scene, LayerId::Page, DARK.accent);
    let want = [
        CARD_X + LANE_PITCH,
        LIST_Y + PITCH - CARD_GAP + 2.0,
        CARD_W,
        2.0,
    ];
    assert_eq!(line.len(), 1);
    assert_eq!(line[0].rect, want);
    let target = fills(&r.scene, LayerId::Page, DARK.surface_hover);
    let frame = r.out.tree.get(col_id(2)).expect("a column").bounds;
    assert!(
        target
            .iter()
            .any(|s| s.rect == [frame.x, frame.y, frame.w, frame.h])
    );

    // Past the last card: the gap under it; the same column, so the line slides there.
    let at = (CARD_X + LANE_PITCH + 50.0, LIST_Y + 2.0 * PITCH);
    let mid = drag_to(&mut h, &m, 1000, at);
    let start = fills(&mid.scene, LayerId::Page, DARK.accent)[0].rect[1];
    assert_eq!(start, want[1], "a slide starts where it was");
    assert_eq!(mid.out.repaint, Repaint::Now);
    let r = settle(&mut h, &m, 1010, 40);
    let line = fills(&r.scene, LayerId::Page, DARK.accent);
    assert_eq!(line[0].rect[1], LIST_Y + 2.0 * PITCH - CARD_GAP + 2.0 + 0.0);

    // Across columns it snaps; no column, no line.
    let at = (CARD_X + 2.0 * LANE_PITCH + 50.0, LIST_Y + 2.0);
    let r = drag_to(&mut h, &m, 2000, at);
    let line = fills(&r.scene, LayerId::Page, DARK.accent);
    assert_eq!(line[0].rect[1], LIST_Y - CARD_GAP + 2.0);
    assert_eq!(line[0].rect[0], CARD_X + 2.0 * LANE_PITCH);
    let r = drag_to(&mut h, &m, 2010, (LANE_X + COLUMN_W + 5.0, LIST_Y));
    assert!(fills(&r.scene, LayerId::Page, DARK.accent).is_empty());
}

#[test]
fn the_ghost_stands_on_glass_so_what_is_under_it_does_not_read_through() {
    // D168's reason: a card dragged over text must not show that text through its own 4% fill.
    // The ghost asks for a blur of what is under it and frosts `menu_glass` over that.
    let mut h = UiHarness::new(WIN);
    let m = model();
    let r = grab(&mut h, &m, 12, 0);
    let g = ghost(&r.scene).expect("a ghost");
    let blurs: Vec<_> = r
        .scene
        .backdrops()
        .filter(|b| {
            b.rect[0] <= g.rect[0] + 0.01
                && b.rect[1] <= g.rect[1] + 0.01
                && b.rect[0] + b.rect[2] >= g.rect[0] + g.rect[2] - 0.01
                && b.rect[1] + b.rect[3] >= g.rect[1] + g.rect[3] - 0.01
        })
        .collect();
    assert!(!blurs.is_empty(), "a request covers the ghost's frost");
    let layer = r.scene.merged(LayerId::Popup);
    assert!(
        layer
            .geometry
            .under
            .iter()
            .all(|s| !(s.rect == g.rect && s.color[3] >= 1.0 - 1e-6)),
        "no opaque fill under the card"
    );
}

#[test]
fn the_source_stays_in_place_dimmed_while_the_ghost_follows_one_to_one() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    let r = grab(&mut h, &m, 12, 0);
    let g = ghost(&r.scene).expect("a ghost");
    assert_eq!(g.rect[0], CARD_X);
    assert_eq!(g.rect[1], LIST_Y + PITCH + 10.0);
    let c = center(&r.out, 12);
    let r = drag_to(&mut h, &m, 300, (c.0 + 200.0, c.1 + 70.0));
    let g = ghost(&r.scene).expect("a ghost");
    assert_eq!(
        (g.rect[0], g.rect[1]),
        (CARD_X + 200.0, LIST_Y + PITCH + 70.0)
    );
    let seen = drawn(&r.scene, 12, 1.0);
    assert_eq!(seen.len(), 2, "the source and the ghost: {seen:?}");
    let source = seen
        .iter()
        .find(|s| s.0 == LayerId::Page as usize)
        .expect("the source");
    assert_eq!(
        source.3,
        (f32::from(DARK.fg_primary[3]) * DIM_ALPHA).round() as u8
    );
    assert_eq!(source.2, LIST_Y + PITCH + 11.0);
}

// Every way a drag ends early.

/// Each way a drag can outlive its inputs: what it does to the frame, and the model it leaves.
enum Ending {
    Escape,
    CaptureLoss,
    ColumnAdded,
    SourceDeleted,
    WindowLost,
    Disabled,
    Error,
}

#[test]
fn nothing_outlives_what_a_drag_stood_on() {
    for ending in [
        Ending::Escape,
        Ending::CaptureLoss,
        Ending::ColumnAdded,
        Ending::SourceDeleted,
        Ending::WindowLost,
        Ending::Disabled,
        Ending::Error,
    ] {
        let mut h = UiHarness::new(WIN);
        let mut m = model();
        grab(&mut h, &m, 11, 0);
        drag_to(&mut h, &m, 50, (CARD_X + LANE_PITCH + 50.0, LIST_Y + 10.0));
        match ending {
            Ending::Escape => press_key(&mut h, NamedKey::Escape),
            Ending::CaptureLoss => {
                m.modal = true;
                run(&mut h, &m, 60);
            }
            Ending::ColumnAdded => m.cols.push((9, vec![])),
            Ending::SourceDeleted => m.cols[0].1.retain(|k| *k != 11),
            Ending::WindowLost => {
                h.input.focus(false);
            }
            Ending::Disabled => m.enabled = false,
            Ending::Error => m.error = true,
        }
        let r = run(&mut h, &m, 70);
        let dragging = |r: &Run| {
            r.out
                .tree
                .nodes
                .iter()
                .any(|n| n.states.contains(State::Dragging))
        };
        assert!(!dragging(&r));
        assert!(fills(&r.scene, LayerId::Page, DARK.accent).is_empty());
        let r = settle(&mut h, &m, 80, 40);
        assert_eq!(r.out.repaint, Repaint::Idle);
        assert!(ghost(&r.scene).is_none());
        assert!(!dragging(&r));
        // The release that follows is nothing.
        let r = release(&mut h, &m, 2000);
        assert_eq!(r.r.moved, None);
        assert_eq!(r.r.opened, None);
    }
}

// Every way a card can jump. A landing is sampled at 60 Hz by the card's title.

fn frame_ms(k: u64) -> u64 {
    k * 1000 / 60
}

/// Runs 60 Hz frames from frame `k0` until `Idle`; every frame draws the card exactly once, and no
/// step passes the conformance bound of the distance from `from` to where it ends.
fn assert_lands(h: &mut UiHarness, m: &Model, key: u64, k0: u64, from: (f32, f32)) -> (f32, f32) {
    let mut path = vec![from];
    let mut idle = false;
    for k in k0..k0 + 40 {
        let r = run(h, m, frame_ms(k));
        let seen = drawn(&r.scene, key, 1.0);
        assert_eq!(seen.len(), 1, "drawn once at frame {k}: {seen:?}");
        path.push((seen[0].1, seen[0].2));
        if r.out.repaint == Repaint::Idle {
            idle = true;
            break;
        }
    }
    assert!(idle, "settles: {path:?}");
    let end = *path.last().expect("a path");
    let dist = ((end.0 - from.0).powi(2) + (end.1 - from.1).powi(2)).sqrt();
    for w in path.windows(2) {
        let step = ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt();
        assert!(
            step <= 0.35 * dist + 0.01,
            "step {step} of {dist}: {path:?}"
        );
    }
    end
}

/// The title's offset from its card's top left.
const TEXT: f32 = 11.0;

/// The ghost's title position after [`grab`] and a drag to `at`: drawn by the frame of the drag.
fn dragged_to(h: &mut UiHarness, m: &Model, key: u64, at: (f32, f32)) -> (f32, f32) {
    grab(h, m, key, 0);
    let r = drag_to(h, m, 100, at);
    let g = ghost(&r.scene).expect("a ghost");
    (g.rect[0] + TEXT, g.rect[1] + TEXT)
}

#[test]
fn an_accepted_drop_lands_in_the_new_slot_without_a_jump() {
    let mut h = UiHarness::new(WIN);
    let mut m = model();
    let at = (CARD_X + LANE_PITCH + 50.0, LIST_Y + 5.0);
    let from = dragged_to(&mut h, &m, 11, at);
    h.input.release(P);
    let r = run(&mut h, &m, frame_ms(10));
    assert_eq!(drawn(&r.scene, 11, 1.0).len(), 1, "the ghost only");
    apply(&mut m, r.r.moved.expect("a move"));
    let end = assert_lands(&mut h, &m, 11, 11, from);
    assert_eq!(end, (CARD_X + LANE_PITCH + TEXT, LIST_Y + TEXT));
}

#[test]
fn an_accepted_drop_down_its_own_column_or_to_another_row_lands_without_a_jump() {
    // Under card 13 in its own column, then the second row of the next column.
    let spots = [
        (
            (CARD_X + 50.0, LIST_Y + 2.0 * PITCH + 40.0),
            (0, 2.0 * PITCH),
        ),
        ((CARD_X + LANE_PITCH + 50.0, LIST_Y + 40.0), (1, PITCH)),
    ];
    for (at, (lane, y)) in spots {
        let mut h = UiHarness::new(WIN);
        let mut m = model();
        let from = dragged_to(&mut h, &m, 11, at);
        h.input.release(P);
        let r = run(&mut h, &m, frame_ms(10));
        apply(&mut m, r.r.moved.expect("a move"));
        let end = assert_lands(&mut h, &m, 11, 11, from);
        let x = CARD_X + lane as f32 * LANE_PITCH + TEXT;
        assert_eq!(end, (x, LIST_Y + y + TEXT));
    }
}

#[test]
fn the_body_takes_over_exactly_on_the_slot_at_any_frame_rate() {
    let mut h = UiHarness::new(WIN);
    let mut m = model();
    let at = (CARD_X + 50.0, LIST_Y + 2.0 * PITCH + 40.0);
    dragged_to(&mut h, &m, 11, at);
    h.input.release(P);
    let r = run(&mut h, &m, 200);
    apply(&mut m, r.r.moved.expect("a move"));
    let slot = (CARD_X + TEXT, LIST_Y + 2.0 * PITCH + TEXT);
    let mut bodies = 0;
    // A long frame first, then quick ones: a slide started late would still be on its way.
    for ms in (230..500).step_by(4) {
        let r = run(&mut h, &m, ms);
        if ghost(&r.scene).is_none() {
            let seen = drawn(&r.scene, 11, 1.0);
            assert_eq!((seen[0].1, seen[0].2), slot, "at {ms}");
            bodies += 1;
        }
    }
    assert!(bodies > 0);
}

#[test]
fn a_refused_drop_lands_back_without_a_jump() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    let at = (CARD_X + LANE_PITCH + 50.0, LIST_Y + 5.0);
    let from = dragged_to(&mut h, &m, 11, at);
    h.input.release(P);
    run(&mut h, &m, frame_ms(10));
    let end = assert_lands(&mut h, &m, 11, 11, from);
    assert_eq!(end, (CARD_X + TEXT, LIST_Y + TEXT));
}

#[test]
fn a_drop_outside_escape_and_capture_loss_land_back_without_a_jump() {
    for how in 0..4 {
        let mut h = UiHarness::new(WIN);
        let mut m = model();
        let from = dragged_to(&mut h, &m, 11, (LANE_X + COLUMN_W + 5.0, LIST_Y + 60.0));
        match how {
            0 => {
                h.input.release(P);
                run(&mut h, &m, frame_ms(10));
            }
            1 => {
                press_key(&mut h, NamedKey::Escape);
                run(&mut h, &m, frame_ms(10));
            }
            2 => {
                // A column appears mid-drag.
                m.cols.push((9, vec![]));
                run(&mut h, &m, frame_ms(10));
            }
            _ => {
                // A modal takes the board's regions out of reach: the first frame still drags.
                m.modal = true;
                run(&mut h, &m, frame_ms(9));
                let r = run(&mut h, &m, frame_ms(10));
                assert_eq!(drawn(&r.scene, 11, 1.0).len(), 1, "the capture ended");
            }
        }
        let end = assert_lands(&mut h, &m, 11, 11, from);
        assert_eq!(end, (CARD_X + TEXT, LIST_Y + TEXT), "{how}");
    }
}

#[test]
fn a_keyboard_move_in_a_column_slides_and_across_columns_lands() {
    let mut h = UiHarness::new(WIN);
    let mut m = model();
    focus(&mut h, &m, 11, 0);
    shift_key(&mut h, NamedKey::ArrowDown);
    let r = run(&mut h, &m, frame_ms(1));
    apply(&mut m, r.r.moved.expect("a move"));
    let from = (CARD_X + TEXT, LIST_Y + TEXT);
    let end = assert_lands(&mut h, &m, 11, 2, from);
    assert_eq!(end, (CARD_X + TEXT, LIST_Y + PITCH + TEXT));

    shift_key(&mut h, NamedKey::ArrowRight);
    let r = run(&mut h, &m, frame_ms(60));
    apply(&mut m, r.r.moved.expect("a move"));
    let end = assert_lands(&mut h, &m, 11, 61, (CARD_X + TEXT, LIST_Y + PITCH + TEXT));
    assert_eq!(end, (CARD_X + LANE_PITCH + TEXT, LIST_Y + PITCH + TEXT));
}

#[test]
fn deleting_a_landing_card_or_its_column_ends_the_landing() {
    for column in [false, true] {
        let mut h = UiHarness::new(WIN);
        let mut m = model();
        dragged_to(&mut h, &m, 11, (LANE_X + COLUMN_W + 5.0, LIST_Y));
        h.input.release(P);
        run(&mut h, &m, frame_ms(10));
        run(&mut h, &m, frame_ms(11));
        if column {
            m.cols.remove(0);
        } else {
            m.cols[0].1.remove(0);
        }
        let r = run(&mut h, &m, frame_ms(12));
        assert!(drawn(&r.scene, 11, 1.0).is_empty());
        assert!(ghost(&r.scene).is_none());
        assert_eq!(
            settle(&mut h, &m, frame_ms(13), 40).out.repaint,
            Repaint::Idle
        );
    }
}

#[test]
fn a_press_on_a_landing_card_holds_it_and_it_goes_on_or_drags_from_there() {
    for drags in [false, true] {
        let mut h = UiHarness::new(WIN);
        let m = model();
        let at = (CARD_X + LANE_PITCH + 50.0, LIST_Y + 5.0);
        dragged_to(&mut h, &m, 11, at);
        h.input.release(P);
        run(&mut h, &m, frame_ms(10));
        let mut r = run(&mut h, &m, frame_ms(11));
        for k in 12..15 {
            r = run(&mut h, &m, frame_ms(k));
        }
        let held = drawn(&r.scene, 11, 1.0)[0];
        // The slot's hit region, while the ghost is still on its way there.
        h.input.pointer_to(CARD_X + 20.0, LIST_Y + 20.0);
        run(&mut h, &m, frame_ms(15));
        let before = ghost(&run(&mut h, &m, frame_ms(16)).scene).expect("still landing");
        h.input.press(P);
        let r = run(&mut h, &m, frame_ms(17));
        let g = ghost(&r.scene).expect("held");
        assert_eq!(drawn(&r.scene, 11, 1.0).len(), 1);
        let step = (g.rect[0] - before.rect[0]).abs();
        assert!(step < 0.35 * LANE_PITCH, "{step}, from {held:?}");
        let r = run(&mut h, &m, frame_ms(18));
        assert_eq!(ghost(&r.scene).expect("held").rect[..2], g.rect[..2]);
        if drags {
            h.input.pointer_to(CARD_X + 20.0, LIST_Y + 25.0);
            let r = run(&mut h, &m, frame_ms(19));
            let d = ghost(&r.scene).expect("the drag");
            assert_eq!(d.rect[0], g.rect[0], "no jump into the drag");
            assert_eq!(d.rect[1], g.rect[1] + 5.0);
            assert_eq!(
                drawn(&r.scene, 11, 1.0).len(),
                2,
                "the source and the ghost"
            );
        } else {
            h.input.release(P);
            let r = run(&mut h, &m, frame_ms(19));
            assert_eq!(r.r.opened, Some(11));
            let from = (g.rect[0] + TEXT, g.rect[1] + TEXT);
            let end = assert_lands(&mut h, &m, 11, 20, from);
            assert_eq!(end, (CARD_X + TEXT, LIST_Y + TEXT));
        }
    }
}

#[test]
fn a_landing_follows_its_slot_while_the_list_scrolls() {
    let mut h = UiHarness::new(WIN);
    let m = tall();
    let at = (CARD_X + LANE_PITCH + 50.0, LIST_Y + 5.0);
    dragged_to(&mut h, &m, 101, at);
    h.input.release(P);
    run(&mut h, &m, frame_ms(10));
    run(&mut h, &m, frame_ms(11));
    // A trackpad moves the list 1:1 under the landing, and the ghost lands in the slot.
    h.input.pointer_to(CARD_X + 20.0, LIST_Y + 100.0);
    h.input.scroll_pixels(0.0, -6.0);
    run(&mut h, &m, frame_ms(12));
    let r = settle(&mut h, &m, frame_ms(13), 40);
    let seen = drawn(&r.scene, 101, 1.0);
    assert_eq!(seen.len(), 1);
    let b = bounds(&r.out, 101);
    assert_eq!((seen[0].1, seen[0].2), (b.x + TEXT, b.y + TEXT));
    assert_eq!(b.y, LIST_Y + PITCH - 6.0);
}

#[test]
fn a_tier_or_scale_change_mid_landing_ends_on_the_slot() {
    for tier_change in [true, false] {
        let mut h = UiHarness::new(WIN);
        let m = model();
        dragged_to(&mut h, &m, 11, (LANE_X + COLUMN_W + 5.0, LIST_Y));
        h.input.release(P);
        run(&mut h, &m, frame_ms(10));
        run(&mut h, &m, frame_ms(11));
        if tier_change {
            h.tier = MotionTier::Off;
        } else {
            h.input.set_scale(2.0);
        }
        let scale = if tier_change { 1.0 } else { 2.0 };
        let r = settle(&mut h, &m, frame_ms(12), 40);
        assert_eq!(r.out.repaint, Repaint::Idle);
        let seen = drawn(&r.scene, 11, scale);
        assert_eq!(seen.len(), 1, "{tier_change}");
        assert_eq!(seen[0].0, LayerId::Page as usize);
        assert_eq!((seen[0].1, seen[0].2), (CARD_X + TEXT, LIST_Y + TEXT));
    }
}

// The motion.

#[test]
fn the_rows_conform_in_three_tiers() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for motion in [SLIDE, SETTLE] {
            conformance(&motion, &DARK.motion, tier, 0.0f32, 100.0f32).expect("conforms");
        }
        conformance(&DIM, &DARK.motion, tier, 1.0f32, DIM_ALPHA).expect("conforms");
    }
}

#[test]
fn reduced_and_off_have_no_landing_and_reduced_keeps_the_dim_fade() {
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        let mut m = model();
        let at = (CARD_X + LANE_PITCH + 50.0, LIST_Y + 5.0);
        dragged_to(&mut h, &m, 11, at);
        h.input.release(P);
        let r = run(&mut h, &m, frame_ms(10));
        assert!(ghost(&r.scene).is_none(), "{tier:?}");
        assert_eq!(drawn(&r.scene, 11, 1.0).len(), 1);
        apply(&mut m, r.r.moved.expect("a move"));
        let r = run(&mut h, &m, frame_ms(11));
        let seen = drawn(&r.scene, 11, 1.0);
        assert_eq!(
            (seen[0].1, seen[0].2),
            (CARD_X + LANE_PITCH + TEXT, LIST_Y + TEXT)
        );
        // The neighbour snaps with it.
        assert_eq!(drawn(&r.scene, 12, 1.0)[0].2, LIST_Y + TEXT);
    }
}

#[test]
fn neighbours_slide_to_their_slots_in_the_full_tier() {
    let mut h = UiHarness::new(WIN);
    let mut m = model();
    run(&mut h, &m, 0);
    m.cols[0].1.remove(0);
    let r = run(&mut h, &m, 100);
    assert_eq!(
        drawn(&r.scene, 12, 1.0)[0].2,
        LIST_Y + PITCH + TEXT,
        "starts"
    );
    assert_eq!(bounds(&r.out, 12).y, LIST_Y, "the hit is the slot");
    let r = run(&mut h, &m, 190);
    let y = drawn(&r.scene, 12, 1.0)[0].2;
    assert!(y > LIST_Y + TEXT && y < LIST_Y + PITCH + TEXT, "{y}");
    let r = run(&mut h, &m, 300);
    assert_eq!(drawn(&r.scene, 12, 1.0)[0].2, LIST_Y + TEXT);
}

#[test]
fn a_resting_board_is_idle() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    run(&mut h, &m, 0);
    assert_eq!(run(&mut h, &m, 16).out.repaint, Repaint::Idle);
}

// The keys.

/// The focused card's key.
fn focused(out: &FrameOutput) -> Option<u64> {
    (0..200).find(|k| out.focused == Some(cid(*k)))
}

#[test]
fn arrows_home_and_end_walk_the_board() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    focus(&mut h, &m, 12, 0);
    let mut ms = 10;
    let mut step = |h: &mut UiHarness, k: NamedKey| {
        press_key(h, k);
        ms += 20;
        run(h, &m, ms);
        focused(&run(h, &m, ms + 1).out)
    };
    assert_eq!(step(&mut h, NamedKey::ArrowDown), Some(13));
    assert_eq!(step(&mut h, NamedKey::ArrowDown), Some(13), "the end");
    assert_eq!(step(&mut h, NamedKey::Home), Some(11));
    assert_eq!(step(&mut h, NamedKey::ArrowUp), Some(11), "the top");
    assert_eq!(step(&mut h, NamedKey::End), Some(13));
    // The nearest index: the next column has two cards, the one after one.
    assert_eq!(step(&mut h, NamedKey::ArrowRight), Some(22));
    assert_eq!(step(&mut h, NamedKey::ArrowRight), Some(31));
    // The fourth column is empty: skipped, and there is no fifth.
    assert_eq!(step(&mut h, NamedKey::ArrowRight), Some(31));
    assert_eq!(step(&mut h, NamedKey::ArrowLeft), Some(21));
    assert_eq!(step(&mut h, NamedKey::ArrowLeft), Some(11));
    assert_eq!(
        step(&mut h, NamedKey::ArrowLeft),
        Some(11),
        "the first column"
    );
}

#[test]
fn left_and_right_skip_an_empty_column() {
    let mut h = UiHarness::new(WIN);
    let m = Model {
        cols: vec![(1, vec![11]), (2, vec![]), (3, vec![31])],
        ..model()
    };
    focus(&mut h, &m, 11, 0);
    press_key(&mut h, NamedKey::ArrowRight);
    run(&mut h, &m, 10);
    assert_eq!(focused(&run(&mut h, &m, 11).out), Some(31));
}

#[test]
fn enter_and_space_open_and_shift_keeps_them() {
    for k in [NamedKey::Enter, NamedKey::Space] {
        let mut h = UiHarness::new(WIN);
        let m = model();
        focus(&mut h, &m, 21, 0);
        press_key(&mut h, k);
        assert_eq!(run(&mut h, &m, 10).r.opened, Some(21));
        shift_key(&mut h, k);
        assert_eq!(run(&mut h, &m, 20).r.opened, Some(21));
    }
}

#[test]
fn shift_with_an_arrow_reports_the_same_move_as_a_drop() {
    let cases = [
        (12, NamedKey::ArrowUp, Some((1, 0))),
        (12, NamedKey::ArrowDown, Some((1, 3))),
        (11, NamedKey::ArrowUp, None),
        (13, NamedKey::ArrowDown, None),
        (11, NamedKey::ArrowLeft, None),
        (12, NamedKey::ArrowRight, Some((2, 1))),
        (13, NamedKey::ArrowRight, Some((2, 2))),
        (22, NamedKey::ArrowLeft, Some((1, 1))),
        (22, NamedKey::ArrowRight, Some((3, 1))),
        // An empty column is a target, at index 0.
        (31, NamedKey::ArrowRight, Some((4, 0))),
    ];
    for (card, k, want) in cases {
        let mut h = UiHarness::new(WIN);
        let m = model();
        focus(&mut h, &m, card, 0);
        shift_key(&mut h, k);
        let got = run(&mut h, &m, 10).r.moved;
        let want = want.map(|(column, index)| CardMove {
            card,
            column,
            index,
        });
        assert_eq!(got, want, "{card} {k:?}");
    }
}

#[test]
fn focus_stays_on_a_moved_card_across_columns_and_it_is_revealed() {
    let mut h = UiHarness::new(WIN);
    let mut m = Model {
        cols: (1..=6).map(|c| (c, vec![c * 10 + 1])).collect(),
        ..model()
    };
    focus(&mut h, &m, 31, 0);
    for ms in [100, 500, 900] {
        shift_key(&mut h, NamedKey::ArrowRight);
        let r = run(&mut h, &m, ms);
        apply(&mut m, r.r.moved.expect("a move"));
        run(&mut h, &m, ms + 10);
        settle(&mut h, &m, ms + 20, 30);
    }
    let r = settle(&mut h, &m, 2000, 40);
    assert_eq!(focused(&r.out), Some(31));
    let b = bounds(&r.out, 31);
    assert!(b.right() <= 878.0 && b.x >= 22.0, "{b:?}");
}

#[test]
fn a_walk_to_a_column_out_of_view_reveals_it() {
    let mut h = UiHarness::new(WIN);
    let m = wide();
    focus(&mut h, &m, 11, 0);
    for ms in [100, 500, 900, 1300, 1700] {
        press_key(&mut h, NamedKey::ArrowRight);
        run(&mut h, &m, ms);
        settle(&mut h, &m, ms + 10, 30);
    }
    let r = settle(&mut h, &m, 2100, 40);
    assert_eq!(focused(&r.out), Some(61));
    let b = bounds(&r.out, 61);
    assert!(b.right() <= 878.0 && b.x >= 22.0, "{b:?}");
}

#[test]
fn a_card_moved_down_a_tall_column_is_revealed() {
    let mut h = UiHarness::new(WIN);
    let mut m = tall();
    let last = *m.cols[0].1.last().expect("a card");
    focus(&mut h, &m, last - 1, 0);
    let r = settle(&mut h, &m, 10, 30);
    assert!(bounds(&r.out, last - 1).y > 500.0, "not in view yet");
    press_key(&mut h, NamedKey::End);
    run(&mut h, &m, 600);
    let r = settle(&mut h, &m, 620, 40);
    let b = bounds(&r.out, last);
    assert!(b.bottom() <= 558.0, "{b:?}");
    shift_key(&mut h, NamedKey::ArrowUp);
    let r = run(&mut h, &m, 1500);
    apply(&mut m, r.r.moved.expect("a move"));
    settle(&mut h, &m, 1510, 40);
}

/// A held modifier and the same modifier on a binding's chord.
type Held = (Modifiers, fn(Chord) -> Chord);

fn chords() -> [Held; 3] {
    let held = |control, alt, logo| Modifiers {
        control,
        alt,
        logo,
        ..Modifiers::default()
    };
    [
        (held(true, false, false), Chord::ctrl),
        (held(false, true, false), Chord::alt),
        (held(false, false, true), Chord::logo),
    ]
}

const KEYS: [NamedKey; 8] = [
    NamedKey::Enter,
    NamedKey::Space,
    NamedKey::ArrowUp,
    NamedKey::ArrowDown,
    NamedKey::ArrowLeft,
    NamedKey::ArrowRight,
    NamedKey::Home,
    NamedKey::End,
];

#[test]
fn a_chord_on_a_card_stays_a_shortcut_enabled_and_disabled() {
    for enabled in [true, false] {
        for (held, chord) in chords() {
            for k in KEYS {
                let mut h = UiHarness::new(WIN);
                h.shortcuts.add(Binding::new(
                    Scope::Global,
                    chord(Chord::key(Key::Named(k))),
                    "chord",
                ));
                let mut m = model();
                focus(&mut h, &m, 12, 0);
                m.enabled = enabled;
                run(&mut h, &m, 10);
                h.input.set_modifiers(held);
                press_key(&mut h, k);
                h.input.set_modifiers(Modifiers::default());
                let r = run(&mut h, &m, 20);
                assert_eq!(r.out.fired.len(), 1, "{held:?} {k:?} {enabled}");
                assert_eq!(r.r.opened, None);
                assert_eq!(r.r.moved, None);
                assert_eq!(focused(&run(&mut h, &m, 21).out), Some(12), "{k:?}");
            }
        }
    }
}

#[test]
fn a_focused_card_turned_disabled_keeps_its_keys_and_acts_on_none() {
    for shift in [false, true] {
        for k in KEYS {
            let mut h = UiHarness::new(WIN);
            let chord = Chord::key(Key::Named(k));
            let chord = if shift { chord.shift() } else { chord };
            h.shortcuts.add(Binding::new(Scope::Global, chord, "bare"));
            h.shortcuts.add(Binding::new(
                Scope::Global,
                Chord::key(Key::Named(NamedKey::F5)),
                "f5",
            ));
            let mut m = model();
            focus(&mut h, &m, 12, 0);
            m.enabled = false;
            run(&mut h, &m, 10);
            if shift {
                shift_key(&mut h, k);
            } else {
                press_key(&mut h, k);
            }
            let r = run(&mut h, &m, 20);
            assert_eq!(r.out.fired.len(), 0, "{k:?}: taken");
            assert_eq!((r.r.opened, r.r.moved), (None, None));
            assert_eq!(focused(&run(&mut h, &m, 21).out), Some(12));
            press_key(&mut h, NamedKey::F5);
            assert_eq!(
                run(&mut h, &m, 30).out.fired.len(),
                1,
                "unhandled keys pass"
            );
        }
    }
}

#[test]
fn all_the_cards_are_one_tab_stop_and_it_remembers_the_cursor() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    run(&mut h, &m, 0);
    let mut cards = Vec::new();
    let mut seen = Vec::new();
    for ms in 1..=14 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        let r = run(&mut h, &m, ms * 10);
        if let Some(k) = focused(&r.out) {
            cards.push(k);
        }
        seen.push(r.out.focused);
    }
    // Add column, two buttons a column and the cards' stop after the first column's: ten stops.
    assert_eq!(cards, [11, 11], "one stop, the 4th and the 14th: {seen:?}");

    focus(&mut h, &m, 12, 500);
    press_key(&mut h, NamedKey::ArrowRight);
    run(&mut h, &m, 510);
    assert_eq!(focused(&run(&mut h, &m, 511).out), Some(22));
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let r = run(&mut h, &m, 520);
    assert_eq!(focused(&r.out), None);
    let mods = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    h.input.set_modifiers(mods);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.input.set_modifiers(Modifiers::default());
    assert_eq!(focused(&run(&mut h, &m, 530).out), Some(22));
}

#[test]
fn a_tab_right_after_an_arrow_counts_the_cards_as_one_stop() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    focus(&mut h, &m, 11, 0);
    press_key(&mut h, NamedKey::ArrowDown);
    run(&mut h, &m, 10);
    // Focus reaches card 12 on this frame's begin, and the regions it reads still have 11 as the
    // stop: backwards it must leave the composite, not walk into it.
    h.input.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.input.set_modifiers(Modifiers::default());
    let r = run(&mut h, &m, 20);
    let to = r
        .out
        .focused
        .and_then(|f| r.out.tree.get(f))
        .expect("a stop");
    assert_eq!(to.name, "Column actions");
}

#[test]
fn page_keys_scroll_a_column_and_die_disabled() {
    let mut h = UiHarness::new(WIN);
    let mut m = tall();
    focus(&mut h, &m, 100, 0);
    press_key(&mut h, NamedKey::PageDown);
    run(&mut h, &m, 10);
    let r = settle(&mut h, &m, 20, 30);
    let moved = bounds(&r.out, 100).y;
    assert!(moved < LIST_Y - 100.0, "{moved}");
    assert_eq!(focused(&r.out), Some(100));

    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::PageDown)),
        "page",
    ));
    m.enabled = false;
    run(&mut h, &m, 1000);
    press_key(&mut h, NamedKey::PageDown);
    let r = run(&mut h, &m, 1010);
    assert_eq!(r.out.fired.len(), 0);
    let r = settle(&mut h, &m, 1020, 30);
    assert_eq!(bounds(&r.out, 100).y, moved);
}

#[test]
fn escape_with_no_drag_stays_a_shortcut() {
    let mut h = UiHarness::new(WIN);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Named(NamedKey::Escape)),
        "escape",
    ));
    let m = model();
    focus(&mut h, &m, 12, 0);
    press_key(&mut h, NamedKey::Escape);
    assert_eq!(run(&mut h, &m, 10).out.fired.len(), 1);
}

#[test]
fn keys_do_nothing_during_a_pointer_drag() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    grab(&mut h, &m, 12, 0);
    shift_key(&mut h, NamedKey::ArrowDown);
    press_key(&mut h, NamedKey::Enter);
    let r = run(&mut h, &m, 50);
    assert_eq!((r.r.moved, r.r.opened), (None, None));
    assert_eq!(drawn(&r.scene, 12, 1.0).len(), 2, "still dragging");
}

// The ghost.

#[test]
fn the_ghost_is_above_the_board_cut_by_no_clip_and_hits_nothing() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    grab(&mut h, &m, 11, 0);
    let r = drag_to(&mut h, &m, 100, (5.0, 590.0));
    let g = ghost(&r.scene).expect("a ghost");
    assert_eq!(g.clip[2], 0.0, "no clip");
    assert!(ghost_on(&r.scene, LayerId::Page).is_none());
    let layer = r.scene.merged(LayerId::Popup);
    let text = layer.texts.first().expect("the ghost's title");
    assert_eq!((text.clip.x, text.clip.y), (0.0, 0.0));
    assert_eq!((text.clip.w, text.clip.h), WIN);

    // A landing over card 21: a press there reaches the card under it.
    let mut h = UiHarness::new(WIN);
    let m = model();
    let at = center(&run(&mut h, &m, 0).out, 21);
    grab(&mut h, &m, 11, 10);
    drag_to(&mut h, &m, 100, at);
    h.input.release(P);
    run(&mut h, &m, 110);
    h.input.click(P);
    let r = run(&mut h, &m, 120);
    assert!(ghost(&r.scene).is_some(), "still landing");
    assert_eq!(r.r.opened, Some(21));
}

#[test]
fn inside_a_dialog_the_ghost_is_raised_and_a_close_replays_none() {
    let mut h = UiHarness::new(WIN);
    let mut m = Model {
        in_dialog: Some(true),
        cols: vec![(1, vec![11, 12])],
        ..model()
    };
    run(&mut h, &m, 0);
    settle(&mut h, &m, 10, 40);
    let r = run(&mut h, &m, 1000);
    let b = r
        .out
        .tree
        .nodes
        .iter()
        .find(|n| n.name == "card 11")
        .expect("a card")
        .bounds;
    h.input.pointer_to(b.x + 20.0, b.y + 10.0);
    run(&mut h, &m, 1010);
    h.input.press(P);
    run(&mut h, &m, 1020);
    h.input.pointer_to(b.x + 60.0, b.y + 30.0);
    let r = run(&mut h, &m, 1030);
    assert!(
        ghost_on(&r.scene, LayerId::Raised).is_some(),
        "above the modal"
    );
    assert!(ghost_on(&r.scene, LayerId::Modal).is_none());
    assert!(ghost_on(&r.scene, LayerId::Popup).is_none());
    assert_eq!(
        r.scene.merged(LayerId::Raised).geometry.under[0].rect[0],
        b.x + 40.0
    );

    m.in_dialog = Some(false);
    let mut last = run(&mut h, &m, 1040);
    for ms in (1050..1400).step_by(17) {
        for l in [LayerId::Modal, LayerId::Raised, LayerId::Popup] {
            assert!(ghost_on(&last.scene, l).is_none(), "{l:?} at {ms}");
        }
        last = run(&mut h, &m, ms);
    }
    assert_eq!(last.out.repaint, Repaint::Idle);
}

#[test]
fn in_a_dialog_the_first_escape_cancels_the_drag_and_only_the_second_closes_the_dialog() {
    let mut h = UiHarness::new(WIN);
    let m = Model {
        in_dialog: Some(true),
        cols: vec![(1, vec![11, 12])],
        ..model()
    };
    run(&mut h, &m, 0);
    settle(&mut h, &m, 10, 40);
    let r = run(&mut h, &m, 1000);
    let b = bounds(&r.out, 11);
    h.input.pointer_to(b.x + 20.0, b.y + 10.0);
    run(&mut h, &m, 1010);
    h.input.press(P);
    run(&mut h, &m, 1020);
    h.input.pointer_to(b.x + 60.0, b.y + 30.0);
    let r = run(&mut h, &m, 1030);
    assert!(ghost_on(&r.scene, LayerId::Raised).is_some(), "dragging");
    press_key(&mut h, NamedKey::Escape);
    let r = run(&mut h, &m, 1040);
    assert!(!r.dialog_cancelled, "the first Escape is the drag's");
    assert!(
        !r.out
            .tree
            .get(cid(11))
            .is_some_and(|n| n.states.contains(State::Dragging)),
        "not dragging"
    );
    // The ghost lands in its slot (the cancel is the landing's path) and the frames go idle.
    let r = settle(&mut h, &m, 1045, 40);
    assert!(
        ghost_on(&r.scene, LayerId::Raised).is_none() && r.out.repaint == Repaint::Idle,
        "no ghost past its landing"
    );
    press_key(&mut h, NamedKey::Escape);
    let r = run(&mut h, &m, 2000);
    assert!(r.dialog_cancelled, "the second Escape is the dialog's");
}

// The auto-scroll.

#[test]
fn a_pointer_at_the_board_s_edge_scrolls_by_dt_to_the_end_and_idles() {
    let mut h = UiHarness::new(WIN);
    let m = wide();
    grab(&mut h, &m, 11, 0);
    // 16 px into the 24 px zone of the right edge at 878.
    let r = drag_to(&mut h, &m, 100, (870.0, LIST_Y + 10.0));
    assert_eq!(r.out.repaint, Repaint::Now, "asks a frame");
    let x0 = run(&mut h, &m, 116)
        .out
        .tree
        .get(col_id(1))
        .expect("a column")
        .bounds
        .x;
    let x1 = run(&mut h, &m, 132)
        .out
        .tree
        .get(col_id(1))
        .expect("a column")
        .bounds
        .x;
    let x2 = run(&mut h, &m, 164)
        .out
        .tree
        .get(col_id(1))
        .expect("a column")
        .bounds
        .x;
    assert_eq!(x0 - x1, 8.0, "8 px per 16 ms at two thirds of the ramp");
    assert_eq!(x1 - x2, 16.0, "by dt");
    let mut ms = 180;
    let mut r = run(&mut h, &m, ms);
    for _ in 0..400 {
        if r.out.repaint != Repaint::Now {
            break;
        }
        ms += 16;
        r = run(&mut h, &m, ms);
    }
    assert_eq!(r.out.repaint, Repaint::Idle, "the end, the pointer still");
    let end = 6.0 * LANE_PITCH - 856.0;
    let col = r.out.tree.get(col_id(1)).expect("a column").bounds.x;
    assert_eq!(col, LANE_X - end);
    // Back: the left edge at full ramp, past the zone.
    let r = drag_to(&mut h, &m, ms + 16, (10.0, LIST_Y + 10.0));
    assert_eq!(r.out.repaint, Repaint::Now);
    run(&mut h, &m, ms + 32);
    let x = run(&mut h, &m, ms + 48)
        .out
        .tree
        .get(col_id(1))
        .expect("a column")
        .bounds
        .x;
    assert_eq!(
        x - col,
        24.0,
        "two steps: the first frame of a zone has no dt"
    );
}

#[test]
fn idle_time_before_the_zone_is_no_step() {
    let mut h = UiHarness::new(WIN);
    let m = wide();
    grab(&mut h, &m, 11, 0);
    drag_to(&mut h, &m, 100, (400.0, LIST_Y + 10.0));
    let r = drag_to(&mut h, &m, 5000, (870.0, LIST_Y + 10.0));
    assert_eq!(
        r.out.tree.get(col_id(1)).expect("a column").bounds.x,
        LANE_X
    );
}

#[test]
fn a_pointer_at_the_target_column_s_edge_scrolls_its_list_and_stops_at_the_end() {
    let mut h = UiHarness::new(WIN);
    let m = tall();
    grab(&mut h, &m, 21, 0);
    let bottom = 60.0 + 498.0 - 11.0;
    drag_to(&mut h, &m, 100, (CARD_X + 50.0, bottom - 1.0));
    let y0 = bounds(&run(&mut h, &m, 116).out, 100).y;
    let y1 = bounds(&run(&mut h, &m, 132).out, 100).y;
    assert!(y0 - y1 > 10.0 && y0 - y1 <= 12.0, "{}", y0 - y1);
    let mut ms = 148;
    let mut r = run(&mut h, &m, ms);
    for _ in 0..400 {
        if r.out.repaint != Repaint::Now {
            break;
        }
        ms += 16;
        r = run(&mut h, &m, ms);
    }
    assert_eq!(r.out.repaint, Repaint::Idle);
    let b = bounds(&r.out, 129);
    assert_eq!(b.bottom() + CARD_GAP, bottom);
    // A drop here counts the whole list, scrolled or not.
    let mv = release(&mut h, &m, ms + 16).r.moved.expect("a move");
    assert_eq!((mv.column, mv.index), (1, 30));
}

#[test]
fn a_drop_on_a_scrolled_board_targets_what_is_under_the_pointer() {
    let mut h = UiHarness::new(WIN);
    let m = wide();
    let first = run(&mut h, &m, 0);
    let bar = first.out.tree.get(col_id(1)).expect("a column").bounds;
    h.input.pointer_to(bar.x + 100.0, bar.y + 5.0);
    h.input.scroll_pixels(-LANE_PITCH, 0.0);
    run(&mut h, &m, 10);
    let r = settle(&mut h, &m, 20, 30);
    assert_eq!(
        r.out.tree.get(col_id(2)).expect("a column").bounds.x,
        LANE_X
    );
    // Column 1 is cut away: at its old place the pointer is over column 2.
    grab(&mut h, &m, 21, 500);
    let at = (CARD_X + LANE_PITCH + 50.0, LIST_Y + 2.0);
    drag_to(&mut h, &m, 600, at);
    let mv = release(&mut h, &m, 610).r.moved.expect("a move");
    assert_eq!((mv.column, mv.index), (3, 0));
}

/// A column whose cards fit leaves a wheel over them to the board; one that overflows keeps it.
#[test]
fn a_wheel_over_the_cards_of_a_column_that_fits_scrolls_the_board() {
    let mut h = UiHarness::new(WIN);
    let m = wide();
    let r = run(&mut h, &m, 0);
    let x0 = r.out.tree.get(col_id(2)).expect("a column").bounds.x;
    let (cx, cy) = center(&r.out, 11);
    h.input.pointer_to(cx, cy);
    run(&mut h, &m, 10);
    h.input.scroll_pixels(0.0, -60.0);
    run(&mut h, &m, 20);
    let r = settle(&mut h, &m, 30, 30);
    let x1 = r.out.tree.get(col_id(2)).expect("a column").bounds.x;
    assert_eq!(x0 - x1, 60.0, "over a card the delta reaches the board");
}

/// One frame of the board in a scrolling page; the page's offset.
fn in_page(h: &mut UiHarness, m: &Model, ms: u64) -> (f64, FrameOutput) {
    h.at_ms(ms);
    let mut off = 0.0;
    let (_, out) = h.frame(|ui| {
        let o = ScrollOptions {
            extent: Some((880.0, 2000.0)),
            ..ScrollOptions::default()
        };
        let page = ui.scroll_area("page", Rect::new(0.0, 0.0, 900.0, 600.0), &o, |ui, _| {
            show(ui, m, BOARD);
            ((), Extent::new(880.0, 2000.0))
        });
        off = page.offset.1;
    });
    (off, out)
}

/// A 60 px wheel step over `key`'s card of a board in a page: where the page and the second column
/// went.
fn wheel_in_page(h: &mut UiHarness, m: &Model, key: u64, ms: u64) -> (f64, f32) {
    let (_, out) = in_page(h, m, ms);
    let x0 = out.tree.get(col_id(2)).expect("a column").bounds.x;
    let (cx, cy) = center(&out, key);
    h.input.pointer_to(cx, cy);
    in_page(h, m, ms + 10);
    h.input.scroll_pixels(0.0, -60.0);
    in_page(h, m, ms + 20);
    let (off, out) = in_page(h, m, ms + 400);
    let x1 = out.tree.get(col_id(2)).expect("a column").bounds.x;
    (off, x0 - x1)
}

#[test]
fn a_board_whose_columns_fit_leaves_the_wheel_to_the_page_around_it() {
    let m = Model {
        cols: vec![(1, vec![11, 12]), (2, vec![21])],
        ..model()
    };
    let mut h = UiHarness::new(WIN);
    assert_eq!(wheel_in_page(&mut h, &m, 11, 0), (60.0, 0.0));
}

#[test]
fn a_board_that_overflows_keeps_the_wheel_from_the_page_around_it() {
    let mut h = UiHarness::new(WIN);
    assert_eq!(wheel_in_page(&mut h, &wide(), 11, 0), (0.0, 60.0));
}

/// The disabled axis still clamps: a board scrolled sideways that comes to fit returns to 0, and
/// from then on the wheel is the page's.
#[test]
fn a_scrolled_board_that_shrinks_to_fit_returns_to_zero_and_leaves_the_wheel_to_the_page() {
    let mut h = UiHarness::new(WIN);
    let mut m = wide();
    let (_, out) = in_page(&mut h, &m, 0);
    let x0 = out.tree.get(col_id(2)).expect("a column").bounds.x;
    assert_eq!(wheel_in_page(&mut h, &m, 11, 10), (0.0, 60.0));
    m.cols.truncate(2);
    let (_, out) = in_page(&mut h, &m, 1000);
    let x1 = out.tree.get(col_id(2)).expect("a column").bounds.x;
    assert_eq!(x1, x0, "on the frame it fits");
    let (off, out) = in_page(&mut h, &m, 1010);
    assert_eq!(off, 0.0);
    assert_eq!(out.repaint, Repaint::Idle);
    assert_eq!(wheel_in_page(&mut h, &m, 11, 1020), (60.0, 0.0));
    // The offset itself went: a board that overflows again starts at 0, not where it was.
    let again = wide();
    let (_, out) = in_page(&mut h, &again, 2000);
    let x2 = out.tree.get(col_id(2)).expect("a column").bounds.x;
    assert_eq!(x2, x0);
}

/// The same with a wheel step in flight when the board comes to fit: no frame shows the old offset.
#[test]
fn a_board_that_shrinks_to_fit_during_a_wheel_step_returns_to_zero() {
    let mut h = UiHarness::new(WIN);
    let mut m = wide();
    let (_, out) = in_page(&mut h, &m, 0);
    let x0 = out.tree.get(col_id(2)).expect("a column").bounds.x;
    let (cx, cy) = center(&out, 11);
    h.input.pointer_to(cx, cy);
    in_page(&mut h, &m, 10);
    h.input.scroll_lines(0.0, -4.0);
    in_page(&mut h, &m, 20);
    let (_, out) = in_page(&mut h, &m, 40);
    assert!(
        out.tree.get(col_id(2)).expect("a column").bounds.x < x0,
        "under way"
    );
    m.cols.truncate(2);
    let (_, out) = in_page(&mut h, &m, 60);
    assert_eq!(out.tree.get(col_id(2)).expect("a column").bounds.x, x0);
    // The card's hover fade from the pointer's arrival is what may still run; the board holds still.
    for ms in (70..600).step_by(10) {
        let (_, out) = in_page(&mut h, &m, ms);
        assert_eq!(out.tree.get(col_id(2)).expect("a column").bounds.x, x0);
    }
    assert_eq!(in_page(&mut h, &m, 600).1.repaint, Repaint::Idle);
}

#[test]
fn a_wheel_over_the_cards_of_a_column_that_overflows_scrolls_the_column_and_not_the_board() {
    let mut h = UiHarness::new(WIN);
    let m = Model {
        cols: (1..=6)
            .map(|c| {
                (
                    c,
                    if c == 1 {
                        (100..130).collect()
                    } else {
                        vec![c * 10 + 1]
                    },
                )
            })
            .collect(),
        ..model()
    };
    let r = run(&mut h, &m, 0);
    let x0 = r.out.tree.get(col_id(2)).expect("a column").bounds.x;
    let y0 = bounds(&r.out, 101).y;
    h.input.pointer_to(CARD_X + 20.0, LIST_Y + 100.0);
    run(&mut h, &m, 10);
    h.input.scroll_pixels(0.0, -60.0);
    run(&mut h, &m, 20);
    let r = settle(&mut h, &m, 30, 30);
    assert_eq!(bounds(&r.out, 101).y, y0 - 60.0, "the column scrolled");
    assert_eq!(
        r.out.tree.get(col_id(2)).expect("a column").bounds.x,
        x0,
        "the board did not"
    );
    // A sideways swipe over an overflowing column goes to the board: the column is live only on y,
    // and a delta routes per axis (D178, which retires the M8 limit).
    h.input.scroll_pixels(-60.0, 0.0);
    run(&mut h, &m, 600);
    let r = settle(&mut h, &m, 610, 30);
    assert_eq!(
        r.out.tree.get(col_id(2)).expect("a column").bounds.x,
        x0 - 60.0
    );
    assert_eq!(bounds(&r.out, 101).y, y0 - 60.0, "the column stayed");
}

// Contract lines a mutant of the code left standing (fix round 1, F3): each test below names the
// line it holds.

/// The accent line of the page layer, if one is drawn.
fn line_of(r: &Run) -> Option<Shape> {
    fills(&r.scene, LayerId::Page, DARK.accent).first().copied()
}

/// A card of column 1 dragged over a list scrolled down: the line sits in the gap above the card
/// the pointer is on, wherever the list has scrolled to.
#[test]
fn the_drop_line_follows_a_column_list_scrolled_down() {
    let mut h = UiHarness::new(WIN);
    let m = tall();
    let first = run(&mut h, &m, 0);
    h.input.pointer_to(CARD_X + 20.0, LIST_Y + 100.0);
    run(&mut h, &m, 10);
    h.input.scroll_pixels(0.0, -300.0);
    run(&mut h, &m, 20);
    let r = settle(&mut h, &m, 30, 30);
    let target = bounds(&r.out, 108);
    assert!(
        target.y > LIST_Y + 30.0 && target.bottom() < LIST_Y + 400.0,
        "in view"
    );
    let (cx, cy) = center(&first.out, 21);
    h.input.pointer_to(cx, cy);
    run(&mut h, &m, 500);
    h.input.press(P);
    run(&mut h, &m, 510);
    h.input.pointer_to(cx, cy + 10.0);
    run(&mut h, &m, 520);
    // Above the middle of card 108: the gap above it.
    h.input.pointer_to(CARD_X + 20.0, target.y + 2.0);
    run(&mut h, &m, 530);
    let r = settle(&mut h, &m, 540, 40);
    let line = line_of(&r).expect("a line");
    assert_eq!(
        line.rect[1],
        target.y - CARD_GAP + 2.0,
        "in the gap above card 108"
    );
    assert_eq!(line.rect[0], target.x);
    let mv = release(&mut h, &m, 1000).r.moved.expect("a move");
    assert_eq!((mv.column, mv.index), (1, 8));
}

/// The board scrolled sideways: the line is in the column's place now, and cut to the viewport.
#[test]
fn the_drop_line_follows_a_scrolled_board_and_is_cut_to_its_viewport() {
    let mut h = UiHarness::new(WIN);
    let m = wide();
    let first = run(&mut h, &m, 0);
    let bar = first.out.tree.get(col_id(1)).expect("a column").bounds;
    h.input.pointer_to(bar.x + 100.0, bar.y + 5.0);
    h.input.scroll_pixels(-100.0, 0.0);
    run(&mut h, &m, 10);
    let r = settle(&mut h, &m, 20, 30);
    let cut = r.out.tree.get(col_id(1)).expect("a column").bounds;
    assert_eq!(cut.x, bar.x - 100.0);
    // Card 21 onto the cut column, at a point still inside the viewport.
    let (cx, cy) = center(&r.out, 21);
    h.input.pointer_to(cx, cy);
    run(&mut h, &m, 500);
    h.input.press(P);
    run(&mut h, &m, 510);
    h.input.pointer_to(cx, cy + 10.0);
    run(&mut h, &m, 520);
    h.input.pointer_to(120.0, LIST_Y + 2.0);
    run(&mut h, &m, 530);
    let r = settle(&mut h, &m, 540, 40);
    let line = line_of(&r).expect("a line");
    let card = bounds(&r.out, 11);
    assert!(card.x < bar.x, "the column is scrolled: {card:?}");
    assert_eq!(line.rect[0], card.x, "the line is at the scrolled column");
    // Cut to the viewport: from the board's inner left edge to the column's right edge.
    assert_eq!(line.clip[0], bar.x, "the clip starts at the viewport");
    assert_eq!(line.clip[0] + line.clip[2], cut.x + cut.w);
}

/// A column the viewport cuts is no target where it is cut away.
#[test]
fn a_column_is_a_drop_target_only_where_the_viewport_shows_it() {
    let mut h = UiHarness::new(WIN);
    let m = wide();
    let first = run(&mut h, &m, 0);
    let bar = first.out.tree.get(col_id(1)).expect("a column").bounds;
    h.input.pointer_to(bar.x + 100.0, bar.y + 5.0);
    h.input.scroll_pixels(-100.0, 0.0);
    run(&mut h, &m, 10);
    let r = settle(&mut h, &m, 20, 30);
    let (cx, cy) = center(&r.out, 21);
    h.input.pointer_to(cx, cy);
    run(&mut h, &m, 500);
    h.input.press(P);
    run(&mut h, &m, 510);
    h.input.pointer_to(cx, cy + 10.0);
    run(&mut h, &m, 520);
    // Left of the viewport, over where the cut column's rect would still reach.
    let r = drag_to(&mut h, &m, 530, (bar.x - 6.0, LIST_Y + 2.0));
    assert!(line_of(&r).is_none(), "the cut-away part is no column");
}

/// The board under a parent's translation: pointer, columns and ghost all agree.
#[test]
fn a_board_under_a_translation_drops_where_the_pointer_is() {
    let at_origin = {
        let mut h = UiHarness::new(WIN);
        let m = model();
        grab(&mut h, &m, 11, 0);
        drag_to(
            &mut h,
            &m,
            100,
            (CARD_X + LANE_PITCH + 50.0, LIST_Y + PITCH - 3.0),
        );
        release(&mut h, &m, 110).r.moved.expect("a move")
    };
    let mut h = UiHarness::new(WIN);
    let m = Model {
        shift: Some((40.0, 60.0)),
        ..model()
    };
    grab(&mut h, &m, 11, 0);
    let r = drag_to(
        &mut h,
        &m,
        100,
        (
            CARD_X + LANE_PITCH + 50.0 + 40.0,
            LIST_Y + PITCH - 3.0 + 60.0,
        ),
    );
    let g = ghost(&r.scene).expect("a ghost");
    let (cx, cy) = center(&r.out, 11);
    assert!(
        cx > CARD_X + 39.0 && cy > LIST_Y + 59.0,
        "drawn at the shift"
    );
    assert!(g.rect[0] > 0.0);
    let moved = release(&mut h, &m, 110).r.moved.expect("a move");
    assert_eq!(moved, at_origin, "the same drop as on an untranslated page");
}

/// Two Shift moves in one frame report the first only: the second would count on a list the app
/// has not changed (D173).
#[test]
fn one_shift_move_a_frame_reports_the_first_key_only() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    focus(&mut h, &m, 12, 0);
    shift_key(&mut h, NamedKey::ArrowDown);
    shift_key(&mut h, NamedKey::ArrowRight);
    let r = run(&mut h, &m, 10);
    let mv = r.r.moved.expect("a move");
    assert_eq!(
        (mv.card, mv.column, mv.index),
        (12, 1, 3),
        "Down: past card 13"
    );
}

/// A card landing when the window loses its focus lands on its slot from where the ghost was.
#[test]
fn a_window_lost_mid_drag_lands_the_card_from_the_ghost() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    let from = dragged_to(&mut h, &m, 11, (CARD_X + LANE_PITCH + 50.0, LIST_Y + 120.0));
    h.input.focus(false);
    let end = assert_lands(&mut h, &m, 11, 10, from);
    assert_eq!(end, (CARD_X + TEXT, LIST_Y + TEXT), "back in its slot");
}

/// A landing in flight ends at once when its scope turns disabled: no ghost is drawn.
#[test]
fn a_landing_ends_at_once_when_the_scope_turns_disabled() {
    let mut h = UiHarness::new(WIN);
    let mut m = model();
    dragged_to(&mut h, &m, 11, (CARD_X + LANE_PITCH + 50.0, LIST_Y + 120.0));
    h.input.release(P);
    run(&mut h, &m, frame_ms(10));
    let r = run(&mut h, &m, frame_ms(11));
    assert!(ghost(&r.scene).is_some(), "landing");
    m.enabled = false;
    let r = run(&mut h, &m, frame_ms(12));
    assert!(ghost(&r.scene).is_none(), "no ghost in a disabled scope");
    assert_eq!(
        drawn(&r.scene, 11, 1.0).len(),
        1,
        "the card draws in its slot"
    );
    assert_eq!(
        settle(&mut h, &m, frame_ms(13), 30).out.repaint,
        Repaint::Idle
    );
}

/// A drag held through the error state is gone when the board is back.
#[test]
fn a_drag_does_not_survive_the_error_state_with_the_pointer_held() {
    let mut h = UiHarness::new(WIN);
    let mut m = model();
    grab(&mut h, &m, 11, 0);
    drag_to(&mut h, &m, 100, (CARD_X + LANE_PITCH + 50.0, LIST_Y + 10.0));
    m.error = true;
    run(&mut h, &m, 110);
    m.error = false;
    let r = run(&mut h, &m, 120);
    assert!(
        !r.out
            .tree
            .nodes
            .iter()
            .any(|n| n.states.contains(State::Dragging)),
        "no card is dragging"
    );
    assert!(ghost(&r.scene).is_none() && line_of(&r).is_none());
    let r = release(&mut h, &m, 130);
    assert_eq!((r.r.moved, r.r.opened), (None, None));
}

/// A press the window's loss cancelled opens nothing when it is released.
#[test]
fn a_press_cancelled_by_a_lost_window_opens_nothing_on_its_release() {
    let mut h = UiHarness::new(WIN);
    let m = model();
    let first = run(&mut h, &m, 0);
    let (x, y) = center(&first.out, 12);
    h.input.pointer_to(x, y);
    run(&mut h, &m, 10);
    h.input.press(P);
    run(&mut h, &m, 20);
    h.input.focus(false);
    run(&mut h, &m, 30);
    h.input.focus(true);
    h.input.release(P);
    let r = run(&mut h, &m, 40);
    assert_eq!((r.r.opened, r.r.moved), (None, None));
}

/// A card carried through a keyboard move keeps its focus ring on the ghost while it lands: the
/// ring, which the key just made visible, goes on fading in on the ghost.
#[test]
fn the_ghost_of_a_focused_card_carries_the_focus_ring() {
    let mut h = UiHarness::new(WIN);
    let mut m = model();
    focus(&mut h, &m, 11, 0);
    run(&mut h, &m, 1000);
    shift_key(&mut h, NamedKey::ArrowRight);
    let r = run(&mut h, &m, 1016);
    apply(&mut m, r.r.moved.expect("a move"));
    // The card's border is one ring; the focus ring is a second, white at a rising alpha.
    let mut alphas = Vec::new();
    for ms in [1032, 1048, 1064, 1080] {
        let r = run(&mut h, &m, ms);
        assert!(ghost(&r.scene).is_some(), "landing at {ms}");
        let rings: Vec<f32> = r
            .scene
            .merged(LayerId::Popup)
            .geometry
            .under
            .iter()
            .filter(|s| s.params[0] == 1.0)
            .map(|s| s.color[3])
            .collect();
        assert_eq!(rings.len(), 2, "the border and the focus ring at {ms}");
        alphas.push(rings[1]);
    }
    assert!(
        alphas.windows(2).all(|w| w[1] > w[0]),
        "the focus ring fades in on the ghost: {alphas:?}"
    );
    // At the ghost's own pace (the ring's level is the ghost's to set: 0.235 here, and 0.11 when
    // the ghost asks for no ring).
    assert!(alphas[3] > 0.2, "{alphas:?}");
}

// The probes at scale 2.

#[test]
fn at_scale_two_a_column_is_544_a_card_s_radius_12_and_the_threshold_8() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let m = model();
    let r = run(&mut h, &m, 0);
    let page = &r.scene.layers[LayerId::Page as usize].geometry.under;
    assert!(page.iter().any(|s| s.rect[2] == 544.0), "a column");
    let card = page
        .iter()
        .find(|s| s.color == premul(DARK.surface_input))
        .expect("a card");
    assert_eq!(card.rect[2], 500.0);
    assert_eq!(card.radii, [12.0; 4]);

    // 7 device px is 3.5 logical: still a click; 8 is the drag.
    let (x, y) = center(&r.out, 11);
    h.input.pointer_to(x, y);
    run(&mut h, &m, 10);
    h.input.press(P);
    run(&mut h, &m, 20);
    h.input.pointer_to(x + 3.5, y);
    let r = run(&mut h, &m, 30);
    assert!(ghost(&r.scene).is_none());
    h.input.pointer_to(x + 4.0, y);
    let r = run(&mut h, &m, 40);
    assert!(ghost(&r.scene).is_some());
}

//! The search picker (`widget.search-picker` acceptance, D124): every path that opens or closes
//! the popup, each by name, with its invariants: `open` follows the overlay, a press toggles once,
//! a close gives focus back to the trigger, an open starts with an empty search and focus in the
//! box, a pick is an index into `items`, and an index out of range is none.

use lune_ui_core::access::{AccessAction, Action, Node, Role, SemanticTree, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::MotionTier;
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Sense, Ui, WidgetId};
use lune_ui_core::widgets::label;
use lune_ui_core::widgets::search_picker::{
    BAR_GAP, BAR_HEIGHT, MAX_HEIGHT, MIN_WIDTH, POPUP_PADDING, PickerOptions, PickerResponse,
    ROW_HEIGHT, ROW_RADIUS, Trigger, matches,
};

const WIN: (f32, f32) = (800.0, 600.0);
const RECT: Rect = Rect::new(100.0, 100.0, 300.0, 36.0);
const NARROW: Rect = Rect::new(100.0, 100.0, 120.0, 36.0);
/// The popup's rect's top: the trigger's bottom and the popover's gap of 4.
const POPUP_Y: f32 = 140.0;
/// The first row's top.
const ROWS_Y: f32 = POPUP_Y + POPUP_PADDING + BAR_HEIGHT + BAR_GAP;
const ON_TRIGGER: (f32, f32) = (150.0, 118.0);
const OTHER: Rect = Rect::new(600.0, 40.0, 80.0, 30.0);
const P: PointerButton = PointerButton::Primary;
const FRUIT: [&str; 5] = ["Apple", "Banana", "Cherry", "Fig", "Éclair"];

fn id() -> WidgetId {
    WidgetId::ROOT.with("p")
}

fn box_id() -> WidgetId {
    id().with("search")
}

fn other_id() -> WidgetId {
    WidgetId::ROOT.with("other")
}

fn row_id(item: usize) -> WidgetId {
    id().with("list").with(item)
}

fn row_at(n: usize) -> (f32, f32) {
    (150.0, ROWS_Y + ROW_HEIGHT * n as f32 + ROW_HEIGHT / 2.0)
}

/// The picker's world: what the app holds and how it draws.
struct App {
    items: Vec<String>,
    selected: Option<usize>,
    enabled: bool,
    drawn: bool,
    rect: Rect,
    trigger: Trigger,
}

impl App {
    fn new() -> Self {
        Self {
            items: FRUIT.iter().map(|s| (*s).to_owned()).collect(),
            selected: None,
            enabled: true,
            drawn: true,
            rect: RECT,
            trigger: Trigger::Field,
        }
    }

    fn draw(&mut self, ui: &mut Ui<'_>) -> Option<PickerResponse> {
        ui.interact(other_id(), OTHER, Sense::CLICK_FOCUS);
        let mut other = Node::new(other_id(), Role::Button, "Other", OTHER);
        other.states.insert(State::Focusable);
        other.actions.insert(Action::Focus);
        ui.node(other);
        let opts = PickerOptions {
            trigger: self.trigger,
            icon: None,
            placeholder: "Find",
            name: "Fruit",
        };
        if !self.drawn {
            return None;
        }
        Some(ui.with_enabled(self.enabled, |ui| {
            ui.search_picker("p", self.rect, &self.items, &mut self.selected, &opts)
        }))
    }
}

struct Seen {
    scene: Scene,
    resp: Option<PickerResponse>,
    tree: SemanticTree,
    focused: Option<WidgetId>,
    repaint: Repaint,
}

impl Seen {
    fn picked(&self) -> Option<usize> {
        self.resp.as_ref().and_then(|r| r.picked)
    }

    /// The trigger says it is expanded.
    fn expanded(&self) -> bool {
        let node = self.tree.get(id()).expect("a trigger");
        node.states.contains(State::Expanded)
    }

    /// The overlay was called this frame: its box is in the tree.
    fn called(&self) -> bool {
        self.tree.get(box_id()).is_some()
    }

    fn rows(&self) -> Vec<String> {
        self.tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::ListItem)
            .map(|n| n.name.clone())
            .collect()
    }

    fn search(&self) -> Option<Value> {
        self.tree.get(box_id()).and_then(|n| n.value.clone())
    }
}

fn frame(h: &mut UiHarness, app: &mut App) -> Seen {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| resp = app.draw(ui));
    Seen {
        scene,
        resp,
        tree: out.tree,
        focused: out.focused,
        repaint: out.repaint,
    }
}

/// Moves time past any motion and runs two frames, and returns the last: the invariant holds on it.
fn settle(h: &mut UiHarness, app: &mut App, ms: &mut u64) -> Seen {
    frame(h, app);
    *ms += 1000;
    h.at_ms(*ms);
    frame(h, app);
    let s = frame(h, app);
    assert_eq!(s.expanded(), s.called(), "open follows the overlay");
    s
}

fn click(h: &mut UiHarness, at: (f32, f32)) {
    h.input.pointer_to(at.0, at.1);
    h.input.click(P);
}

fn key(h: &mut UiHarness, k: NamedKey) {
    h.input.key_down(Key::Named(k), None);
    h.input.key_up(Key::Named(k));
}

fn type_text(h: &mut UiHarness, s: &str) {
    for c in s.chars() {
        let c = c.to_string();
        h.input
            .key_down(Key::Character(c.as_str().into()), Some(&c));
    }
}

fn access(h: &mut UiHarness, target: WidgetId, action: Action) {
    h.input.access(AccessAction {
        target,
        action,
        value: None,
    });
}

/// A harness with the picker drawn once, and the clock.
fn start(app: &mut App) -> (UiHarness, u64) {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, app);
    (h, 0)
}

/// As [`start`], with the popup open by a trigger click and settled.
fn opened(app: &mut App) -> (UiHarness, u64) {
    let (mut h, mut ms) = start(app);
    click(&mut h, ON_TRIGGER);
    let s = settle(&mut h, app, &mut ms);
    assert!(s.expanded());
    assert_eq!(s.focused, Some(box_id()));
    (h, ms)
}

fn assert_closed_on_the_trigger(s: &Seen) {
    assert!(!s.expanded());
    assert!(!s.called());
    assert_eq!(s.focused, Some(id()), "focus back on the trigger");
}

// Opening.

#[test]
fn a_trigger_click_opens_once_with_an_empty_search_and_focus_in_the_box() {
    let mut app = App::new();
    let (mut h, mut ms) = start(&mut app);
    click(&mut h, ON_TRIGGER);
    let s = frame(&mut h, &mut app);
    assert!(s.expanded());
    assert!(
        s.called(),
        "the overlay is called on the frame that opens it"
    );
    assert_eq!(
        s.focused,
        Some(box_id()),
        "the frame ends with focus in the box"
    );
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(s.expanded(), "one toggle");
    assert_eq!(s.search(), Some(Value::Text(String::new())));
    assert_eq!(s.rows(), FRUIT);
}

#[test]
fn enter_and_space_on_the_trigger_open_and_typing_reaches_the_box() {
    for k in [NamedKey::Enter, NamedKey::Space] {
        let mut app = App::new();
        let (mut h, mut ms) = start(&mut app);
        key(&mut h, NamedKey::Tab);
        frame(&mut h, &mut app);
        key(&mut h, NamedKey::Tab);
        assert_eq!(frame(&mut h, &mut app).focused, Some(id()));
        key(&mut h, k);
        let s = frame(&mut h, &mut app);
        assert!(s.expanded(), "{k:?}");
        assert_eq!(s.focused, Some(box_id()));
        // The frame after the open: the first keys land.
        type_text(&mut h, "fi");
        let s = frame(&mut h, &mut app);
        assert_eq!(s.search(), Some(Value::Text("fi".to_owned())));
        assert_eq!(s.rows(), ["Fig"], "the rows follow on the same frame");
        settle(&mut h, &mut app, &mut ms);
    }
}

#[test]
fn the_expand_and_default_actions_open() {
    for action in [Action::Expand, Action::Default] {
        let mut app = App::new();
        let (mut h, mut ms) = start(&mut app);
        access(&mut h, id(), action);
        let s = settle(&mut h, &mut app, &mut ms);
        assert!(s.expanded(), "{action:?}");
        assert_eq!(s.focused, Some(box_id()));
    }
}

#[test]
fn the_expand_action_on_an_open_picker_changes_nothing() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    access(&mut h, id(), Action::Expand);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(s.expanded());
    assert_eq!(s.focused, Some(box_id()));
}

// Closing: each path ends closed with focus on the trigger.

#[test]
fn escape_closes() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    key(&mut h, NamedKey::Escape);
    let s = frame(&mut h, &mut app);
    assert!(!s.expanded(), "the frame that dismisses says so");
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
    assert_eq!(app.selected, None);
}

#[test]
fn an_outside_press_closes_and_reaches_nothing() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    click(&mut h, (OTHER.x + 10.0, OTHER.y + 10.0));
    let s = settle(&mut h, &mut app, &mut ms);
    assert_closed_on_the_trigger(&s);
}

#[test]
fn a_press_on_the_open_trigger_closes_once_and_does_not_reopen() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    h.input.pointer_to(ON_TRIGGER.0, ON_TRIGGER.1);
    h.input.press(P);
    let s = frame(&mut h, &mut app);
    assert!(!s.expanded(), "closed by the press");
    frame(&mut h, &mut app);
    h.input.release(P);
    let s = frame(&mut h, &mut app);
    assert!(!s.expanded(), "the release reopens nothing");
    assert!(!s.called());
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));

    // Press and release in one frame.
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    click(&mut h, ON_TRIGGER);
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
    // The next click opens again.
    click(&mut h, ON_TRIGGER);
    assert!(settle(&mut h, &mut app, &mut ms).expanded());
}

#[test]
fn window_focus_loss_closes() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    h.input.focus(false);
    frame(&mut h, &mut app);
    h.input.focus(true);
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
}

#[test]
fn focus_moved_to_another_widget_closes_and_stays_there() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    access(&mut h, other_id(), Action::Focus);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(!s.expanded());
    assert_eq!(s.focused, Some(other_id()), "the move is what closed it");
}

#[test]
fn a_row_click_picks_the_row_and_closes() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    click(&mut h, row_at(2));
    let s = frame(&mut h, &mut app);
    assert_eq!(s.picked(), Some(2));
    assert!(s.resp.as_ref().expect("drawn").response.changed);
    assert!(!s.expanded());
    assert_eq!(app.selected, Some(2));
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
}

#[test]
fn the_default_action_on_a_row_picks_it() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    access(&mut h, row_id(3), Action::Default);
    let s = frame(&mut h, &mut app);
    assert_eq!(s.picked(), Some(3));
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
}

#[test]
fn a_pick_of_the_selected_row_reports_it_and_changes_nothing() {
    let mut app = App::new();
    app.selected = Some(1);
    let (mut h, mut ms) = opened(&mut app);
    click(&mut h, row_at(1));
    let s = frame(&mut h, &mut app);
    assert_eq!(s.picked(), Some(1));
    assert!(!s.resp.as_ref().expect("drawn").response.changed);
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
}

#[test]
fn enter_in_the_box_picks_the_first_match_as_an_index_into_items_and_closes() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    type_text(&mut h, "an");
    frame(&mut h, &mut app);
    key(&mut h, NamedKey::Enter);
    let s = frame(&mut h, &mut app);
    assert_eq!(s.picked(), Some(1), "Banana is item 1, row 0");
    assert_eq!(app.selected, Some(1));
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
}

#[test]
fn enter_with_an_empty_search_picks_the_first_item() {
    let mut app = App::new();
    let (mut h, _) = opened(&mut app);
    key(&mut h, NamedKey::Enter);
    assert_eq!(frame(&mut h, &mut app).picked(), Some(0));
}

#[test]
fn enter_typed_on_the_same_frame_uses_the_new_filter() {
    let mut app = App::new();
    let (mut h, _) = opened(&mut app);
    type_text(&mut h, "fig");
    key(&mut h, NamedKey::Enter);
    assert_eq!(frame(&mut h, &mut app).picked(), Some(3));
}

#[test]
fn enter_with_no_match_stays_open_and_picks_nothing() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    type_text(&mut h, "zzz");
    frame(&mut h, &mut app);
    key(&mut h, NamedKey::Enter);
    let s = frame(&mut h, &mut app);
    assert_eq!(s.picked(), None);
    assert_eq!(app.selected, None);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(s.expanded());
    assert_eq!(s.rows(), Vec::<String>::new());
    assert_eq!(s.focused, Some(box_id()));
}

#[test]
fn the_collapse_and_default_actions_close() {
    for action in [Action::Collapse, Action::Default] {
        let mut app = App::new();
        let (mut h, mut ms) = opened(&mut app);
        access(&mut h, id(), action);
        let s = frame(&mut h, &mut app);
        assert!(!s.expanded(), "{action:?}");
        assert!(
            !s.called(),
            "a close on the trigger never calls the overlay"
        );
        assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
    }
}

#[test]
fn the_dismiss_action_closes() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    access(&mut h, box_id(), Action::Dismiss);
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
}

#[test]
fn disabling_while_open_closes_and_enabling_does_not_reopen() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    app.enabled = false;
    let s = frame(&mut h, &mut app);
    assert!(!s.expanded());
    assert!(!s.called());
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
    app.enabled = true;
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(!s.expanded(), "it does not pop back");
    assert_eq!(s.focused, Some(id()));
}

#[test]
fn a_disabled_picker_does_not_open() {
    let mut app = App::new();
    app.enabled = false;
    let (mut h, mut ms) = start(&mut app);
    click(&mut h, ON_TRIGGER);
    access(&mut h, id(), Action::Expand);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(!s.expanded());
    let node = s.tree.get(id()).expect("a node");
    assert!(node.states.contains(State::Disabled));
    assert!(!node.actions.contains(Action::Expand));
}

#[test]
fn items_replaced_while_open_stay_open_and_the_rows_follow() {
    let mut app = App::new();
    app.selected = Some(4);
    let (mut h, mut ms) = opened(&mut app);
    app.items = vec!["Kiwi".to_owned(), "Lime".to_owned()];
    let s = frame(&mut h, &mut app);
    assert!(s.expanded());
    assert_eq!(s.rows(), ["Kiwi", "Lime"]);
    assert_eq!(s.tree.get(id()).expect("a node").value, None);
    assert_eq!(app.selected, Some(4), "left as it is");
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(s.focused, Some(box_id()));
    click(&mut h, row_at(1));
    assert_eq!(frame(&mut h, &mut app).picked(), Some(1));
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
}

#[test]
fn items_emptied_while_open_do_not_panic_and_enter_picks_nothing() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    app.items.clear();
    let s = frame(&mut h, &mut app);
    assert_eq!(s.rows(), Vec::<String>::new());
    key(&mut h, NamedKey::Enter);
    assert_eq!(frame(&mut h, &mut app).picked(), None);
    assert!(settle(&mut h, &mut app, &mut ms).expanded());
}

#[test]
fn a_selected_index_out_of_range_reads_as_none() {
    let mut app = App::new();
    app.selected = Some(99);
    let (mut h, mut ms) = opened(&mut app);
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(s.tree.get(id()).expect("a node").value, None);
    assert!(
        !s.tree
            .nodes
            .iter()
            .any(|n| n.states.contains(State::Selected))
    );
    assert_eq!(app.selected, Some(99));
}

#[test]
fn a_keep_alive_gap_closes_the_popup_and_it_does_not_come_back() {
    let mut app = App::new();
    let mut h = UiHarness::new(WIN);
    let seen = |h: &mut UiHarness, app: &mut App| {
        let (_, out) = h.frame(|ui| {
            if app.drawn {
                ui.keep_alive("k", |ui| {
                    let mut selected = None;
                    ui.search_picker(
                        "p",
                        RECT,
                        &app.items,
                        &mut selected,
                        &PickerOptions::default(),
                    );
                });
            } else {
                ui.keep_alive("k", |_| {});
            }
        });
        out
    };
    let scoped = WidgetId::ROOT.with("k").with("p");
    seen(&mut h, &mut app);
    click(&mut h, ON_TRIGGER);
    seen(&mut h, &mut app);
    h.at_ms(1000);
    let out = seen(&mut h, &mut app);
    assert!(
        out.tree
            .get(scoped)
            .expect("a node")
            .states
            .contains(State::Expanded)
    );
    app.drawn = false;
    seen(&mut h, &mut app);
    app.drawn = true;
    let out = seen(&mut h, &mut app);
    assert!(
        !out.tree
            .get(scoped)
            .expect("a node")
            .states
            .contains(State::Expanded)
    );
    assert!(out.tree.get(scoped.with("search")).is_none());
}

// Reopening.

#[test]
fn a_reopen_after_a_pick_shows_an_empty_search_and_all_rows() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    type_text(&mut h, "ch");
    frame(&mut h, &mut app);
    click(&mut h, row_at(0));
    assert_eq!(frame(&mut h, &mut app).picked(), Some(2));
    settle(&mut h, &mut app, &mut ms);
    click(&mut h, ON_TRIGGER);
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(s.search(), Some(Value::Text(String::new())));
    assert_eq!(s.rows(), FRUIT);
    assert_eq!(s.focused, Some(box_id()));
    type_text(&mut h, "z");
    let s = frame(&mut h, &mut app);
    assert_eq!(s.search(), Some(Value::Text("z".to_owned())));
}

#[test]
fn a_reopen_after_escape_with_text_starts_empty() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    type_text(&mut h, "ch");
    frame(&mut h, &mut app);
    key(&mut h, NamedKey::Escape);
    settle(&mut h, &mut app, &mut ms);
    key(&mut h, NamedKey::Enter);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(s.expanded());
    assert_eq!(s.search(), Some(Value::Text(String::new())));
}

#[test]
fn a_reopen_during_the_close_motion_focuses_the_box() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    key(&mut h, NamedKey::Escape);
    frame(&mut h, &mut app);
    frame(&mut h, &mut app);
    let s = frame(&mut h, &mut app);
    assert_eq!(s.focused, Some(id()));
    key(&mut h, NamedKey::Enter);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(s.expanded());
    assert_eq!(s.focused, Some(box_id()));
}

#[test]
fn an_open_on_the_frame_right_after_a_close_is_a_fresh_open() {
    // The overlay of the close has not left the stack yet on that frame.
    for action in [Action::Expand, Action::Default] {
        let mut app = App::new();
        let (mut h, mut ms) = opened(&mut app);
        type_text(&mut h, "ch");
        frame(&mut h, &mut app);
        key(&mut h, NamedKey::Escape);
        assert!(!frame(&mut h, &mut app).expanded());
        access(&mut h, id(), action);
        let s = frame(&mut h, &mut app);
        assert!(s.expanded(), "{action:?}");
        assert_eq!(s.focused, Some(box_id()), "{action:?}");
        let s = settle(&mut h, &mut app, &mut ms);
        assert!(s.expanded(), "{action:?}: no old dismissal closes it");
        assert_eq!(s.focused, Some(box_id()), "{action:?}");
        assert_eq!(s.search(), Some(Value::Text(String::new())));
    }
}

#[test]
fn opened_by_an_action_with_focus_elsewhere_a_close_still_lands_on_the_trigger() {
    let mut app = App::new();
    let (mut h, mut ms) = start(&mut app);
    click(&mut h, (OTHER.x + 10.0, OTHER.y + 10.0));
    assert_eq!(frame(&mut h, &mut app).focused, Some(other_id()));
    access(&mut h, id(), Action::Expand);
    settle(&mut h, &mut app, &mut ms);
    key(&mut h, NamedKey::Escape);
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
}

#[test]
fn a_keyboard_close_keeps_the_ring_and_a_pointer_close_has_none() {
    let mut app = App::new();
    let (mut h, mut ms) = start(&mut app);
    key(&mut h, NamedKey::Tab);
    frame(&mut h, &mut app);
    key(&mut h, NamedKey::Tab);
    frame(&mut h, &mut app);
    key(&mut h, NamedKey::Enter);
    settle(&mut h, &mut app, &mut ms);
    key(&mut h, NamedKey::Escape);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(s.resp.as_ref().expect("drawn").response.focus_visible);

    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    click(&mut h, row_at(0));
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(!s.resp.as_ref().expect("drawn").response.focus_visible);
}

#[test]
fn tab_in_the_popup_stays_inside_it() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    key(&mut h, NamedKey::Tab);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(s.expanded());
    assert_eq!(s.focused, Some(box_id()), "no row is a stop");
    type_text(&mut h, "a");
    frame(&mut h, &mut app);
    key(&mut h, NamedKey::Tab);
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(
        s.focused,
        Some(box_id().with("clear")),
        "the clear button is"
    );
    key(&mut h, NamedKey::Escape);
    assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
}

#[test]
fn a_row_press_moves_no_focus_and_a_press_in_the_popup_dismisses_nothing() {
    let mut app = App::new();
    let (mut h, _) = opened(&mut app);
    let at = row_at(1);
    h.input.pointer_to(at.0, at.1);
    h.input.press(P);
    let s = frame(&mut h, &mut app);
    assert!(s.expanded());
    assert_eq!(s.focused, Some(box_id()));
    let s = frame(&mut h, &mut app);
    assert!(s.expanded());
    assert_eq!(s.focused, Some(box_id()));
}

#[test]
fn the_clear_button_in_the_box_empties_the_search_and_keeps_the_popup() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    type_text(&mut h, "fig");
    frame(&mut h, &mut app);
    frame(&mut h, &mut app);
    let bar_right = 100.0 + 300.0 - POPUP_PADDING;
    click(
        &mut h,
        (bar_right - 17.0, POPUP_Y + POPUP_PADDING + BAR_HEIGHT / 2.0),
    );
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(s.expanded());
    assert_eq!(s.rows(), FRUIT);
    assert_eq!(s.focused, Some(box_id()));
}

// The filter.

#[test]
fn the_filter_trims_ignores_case_and_shows_all_for_spaces() {
    assert_eq!(matches(&FRUIT, "AN"), [1]);
    assert_eq!(matches(&FRUIT, "  an  "), [1]);
    assert_eq!(matches(&FRUIT, "éCL"), [4], "Unicode case");
    assert_eq!(matches(&FRUIT, ""), [0, 1, 2, 3, 4]);
    assert_eq!(matches(&FRUIT, "   "), [0, 1, 2, 3, 4]);
    assert_eq!(matches(&FRUIT, "zzz"), Vec::<usize>::new());
    assert_eq!(matches(&["a b"], "a b"), [0], "the inside is not trimmed");
}

#[test]
fn a_search_of_spaces_shows_all_rows() {
    let mut app = App::new();
    let (mut h, _) = opened(&mut app);
    type_text(&mut h, "  ");
    assert_eq!(frame(&mut h, &mut app).rows(), FRUIT);
}

// The list.

fn long(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("Item {i}")).collect()
}

#[test]
fn ten_thousand_items_build_only_the_visible_rows() {
    let mut app = App::new();
    app.items = long(10_000);
    let (mut h, mut ms) = opened(&mut app);
    let s = settle(&mut h, &mut app, &mut ms);
    let height = MAX_HEIGHT - 2.0 * POPUP_PADDING - BAR_HEIGHT - BAR_GAP;
    let visible = (height / ROW_HEIGHT).ceil() as usize;
    assert_eq!(s.rows().len(), visible, "{height}");
    let hits = |n: &&lune_ui_core::access::Node| n.role == Role::ListItem;
    assert!(s.tree.nodes.iter().filter(hits).count() <= visible + 1);
    assert!(s.tree.nodes.len() < 30, "{} nodes", s.tree.nodes.len());
}

#[test]
fn the_wheel_scrolls_and_a_click_picks_the_item_under_it() {
    let mut app = App::new();
    app.items = long(100);
    let (mut h, mut ms) = opened(&mut app);
    let at = row_at(1);
    h.input.pointer_to(at.0, at.1);
    h.input.scroll_pixels(0.0, -(ROW_HEIGHT * 10.0));
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(s.rows()[0], "Item 10");
    click(&mut h, row_at(1));
    assert_eq!(frame(&mut h, &mut app).picked(), Some(11));
    settle(&mut h, &mut app, &mut ms);
}

#[test]
fn a_new_filter_and_a_reopen_start_the_list_at_its_top() {
    let mut app = App::new();
    app.items = long(100);
    let (mut h, mut ms) = opened(&mut app);
    let at = row_at(1);
    h.input.pointer_to(at.0, at.1);
    h.input.scroll_pixels(0.0, -(ROW_HEIGHT * 20.0));
    settle(&mut h, &mut app, &mut ms);
    type_text(&mut h, "Item");
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(s.rows()[0], "Item 0", "a new filter");

    h.input.scroll_pixels(0.0, -(ROW_HEIGHT * 20.0));
    settle(&mut h, &mut app, &mut ms);
    key(&mut h, NamedKey::Escape);
    settle(&mut h, &mut app, &mut ms);
    key(&mut h, NamedKey::Enter);
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(s.rows()[0], "Item 0", "a reopen");
}

#[test]
fn rows_show_selection_and_hover_at_radius_4() {
    let mut app = App::new();
    app.selected = Some(1);
    let (mut h, mut ms) = opened(&mut app);
    let at = row_at(2);
    h.input.pointer_to(at.0, at.1);
    let s = settle(&mut h, &mut app, &mut ms);
    let layer = s.scene.merged(LayerId::Popup);
    let g = &layer.geometry;
    let fill = |color| {
        *g.under
            .iter()
            .find(|x| x.color == premul(color) && x.rect[3] == ROW_HEIGHT)
            .expect("a row fill")
    };
    let sel = fill(DARK.surface_selected);
    assert_eq!(sel.rect[1], ROWS_Y + ROW_HEIGHT);
    assert_eq!(sel.radii, [ROW_RADIUS; 4]);
    assert_eq!(fill(DARK.surface_hover).rect[1], ROWS_Y + ROW_HEIGHT * 2.0);
    let node = s.tree.get(row_id(1)).expect("a row");
    assert!(node.states.contains(State::Selected));
    assert!(node.actions.contains(Action::Default));
}

// Size and place.

fn popup_panel(s: &Scene) -> [f32; 4] {
    s.merged(LayerId::Popup)
        .geometry
        .under
        .iter()
        .find(|x| x.color == premul(DARK.dropdown_bg))
        .expect("the panel")
        .rect
}

#[test]
fn the_popup_is_as_wide_as_a_wider_field_and_at_least_220() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(popup_panel(&s.scene)[2], 300.0);

    let mut app = App::new();
    app.rect = NARROW;
    let (mut h, mut ms) = opened(&mut app);
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(popup_panel(&s.scene)[2], MIN_WIDTH);

    let mut app = App::new();
    app.trigger = Trigger::Link;
    let (mut h, mut ms) = opened(&mut app);
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(popup_panel(&s.scene)[2], MIN_WIDTH, "a link is no measure");
}

#[test]
fn the_popup_height_follows_the_filtered_count_up_to_320() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    let s = settle(&mut h, &mut app, &mut ms);
    let chrome = 2.0 * POPUP_PADDING + BAR_HEIGHT;
    assert_eq!(
        popup_panel(&s.scene)[3],
        chrome + BAR_GAP + 5.0 * ROW_HEIGHT
    );
    type_text(&mut h, "fig");
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(popup_panel(&s.scene)[3], chrome + BAR_GAP + ROW_HEIGHT);
    type_text(&mut h, "zzz");
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(popup_panel(&s.scene)[3], chrome, "no list");

    let mut app = App::new();
    app.items = long(100);
    let (mut h, mut ms) = opened(&mut app);
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(popup_panel(&s.scene)[3], MAX_HEIGHT);
}

#[test]
fn a_popup_flipped_at_open_stays_above_the_trigger_while_it_narrows() {
    let mut app = App::new();
    app.rect = Rect::new(100.0, 500.0, 300.0, 36.0);
    app.items = long(100);
    let (mut h, mut ms) = start(&mut app);
    click(&mut h, (150.0, 518.0));
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(popup_panel(&s.scene)[1], 500.0 - 4.0 - MAX_HEIGHT);
    type_text(&mut h, "Item 99");
    let s = settle(&mut h, &mut app, &mut ms);
    let panel = popup_panel(&s.scene);
    assert_eq!(panel[1] + panel[3], 496.0, "it stays above");
}

#[test]
fn a_window_shorter_than_the_popup_keeps_the_list_inside_it() {
    let mut app = App::new();
    app.rect = Rect::new(100.0, 10.0, 300.0, 36.0);
    app.items = long(100);
    let mut h = UiHarness::new((800.0, 200.0));
    frame(&mut h, &mut app);
    let mut ms = 0;
    click(&mut h, (150.0, 28.0));
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(popup_panel(&s.scene)[3], 184.0);
}

#[test]
fn a_trigger_in_a_scrolled_parent_anchors_where_it_is_drawn() {
    let mut h = UiHarness::new(WIN);
    let items = FRUIT;
    let mut selected = None;
    let opts = PickerOptions::default();
    let mut run = |h: &mut UiHarness| {
        h.frame(|ui| {
            ui.painter().push_translate(0.0, 50.0);
            ui.search_picker("p", RECT, &items, &mut selected, &opts);
            ui.painter().pop_translate();
        })
    };
    run(&mut h);
    h.input.pointer_to(ON_TRIGGER.0, ON_TRIGGER.1 + 50.0);
    h.input.click(P);
    run(&mut h);
    h.at_ms(1000);
    run(&mut h);
    let (scene, _) = run(&mut h);
    assert_eq!(popup_panel(&scene)[1], POPUP_Y + 50.0);
}

// The triggers.

#[test]
fn the_field_trigger_shows_the_value_on_the_surface_with_an_ellipsis() {
    let mut app = App::new();
    app.selected = Some(0);
    app.items[0] = "x".repeat(200);
    let (mut h, _) = start(&mut app);
    let s = frame(&mut h, &mut app);
    let g = &s.scene.layers[0].geometry;
    let surface = g
        .under
        .iter()
        .find(|x| x.color == premul(DARK.surface_input))
        .expect("the surface");
    assert_eq!(surface.radii, [DARK.item_radius; 4]);
    let whole = label(&DARK, &app.items[0], DARK.fg_primary).cache_key(1.0);
    assert!(
        s.scene.layers[0].texts.iter().all(|t| t.key.0 != whole),
        "cut"
    );
}

#[test]
fn the_link_trigger_is_secondary_primary_on_hover_with_a_dashed_underline_as_wide_as_the_text() {
    let mut app = App::new();
    app.trigger = Trigger::Link;
    app.selected = Some(1);
    let (mut h, mut ms) = start(&mut app);
    let s = frame(&mut h, &mut app);
    let item = |s: &Seen, color| {
        let key = label(&DARK, "Banana", color).cache_key(1.0);
        s.scene.layers[0].texts.iter().any(|t| t.key.0 == key)
    };
    assert!(item(&s, DARK.fg_secondary));
    let strokes = &s.scene.layers[0].geometry.strokes;
    assert!(!strokes.is_empty(), "the dashes");
    let xs: Vec<f32> = strokes.iter().map(|v| v.pos[0]).collect();
    let (x0, x1) = (
        xs.iter().copied().fold(f32::MAX, f32::min),
        xs.iter().copied().fold(f32::MIN, f32::max),
    );
    let mut text = lune_ui_core::testing::FakeText;
    let (w, _) = lune_ui_core::text::TextSystem::measure(
        &mut text,
        &label(&DARK, "Banana", DARK.fg_secondary),
        1.0,
    );
    assert!((x1 - x0 - w).abs() <= 2.0, "{x0}..{x1} for {w}");

    h.input.pointer_to(ON_TRIGGER.0, ON_TRIGGER.1);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(item(&s, DARK.fg_primary));
}

#[test]
fn the_rest_trigger_settles_to_idle_and_the_ring_shows_on_the_keyboard() {
    for trigger in [Trigger::Field, Trigger::Link] {
        let mut app = App::new();
        app.trigger = trigger;
        let (mut h, mut ms) = start(&mut app);
        assert_eq!(settle(&mut h, &mut app, &mut ms).repaint, Repaint::Idle);
        let rest = frame(&mut h, &mut app);
        key(&mut h, NamedKey::Tab);
        frame(&mut h, &mut app);
        key(&mut h, NamedKey::Tab);
        let s = settle(&mut h, &mut app, &mut ms);
        assert_eq!(s.focused, Some(id()));
        assert_ne!(s.scene, rest.scene, "{trigger:?}: a ring");
    }
}

#[test]
fn reduced_motion_opens_and_closes_all_the_same() {
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let mut app = App::new();
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        frame(&mut h, &mut app);
        let mut ms = 0;
        click(&mut h, ON_TRIGGER);
        let s = settle(&mut h, &mut app, &mut ms);
        assert_eq!(s.focused, Some(box_id()));
        key(&mut h, NamedKey::Escape);
        assert_closed_on_the_trigger(&settle(&mut h, &mut app, &mut ms));
    }
}

// The nodes.

#[test]
fn the_nodes_are_a_combo_box_a_text_input_and_a_list_of_items() {
    let mut app = App::new();
    app.selected = Some(2);
    let (mut h, mut ms) = opened(&mut app);
    let s = settle(&mut h, &mut app, &mut ms);
    let trigger = s.tree.get(id()).expect("a trigger");
    assert_eq!(trigger.role, Role::ComboBox);
    assert_eq!(trigger.name, "Fruit");
    assert_eq!(trigger.value, Some(Value::Text("Cherry".to_owned())));
    assert!(trigger.states.contains(State::Expanded));
    assert!(trigger.states.contains(State::Focusable));
    for a in [Action::Default, Action::Expand, Action::Collapse] {
        assert!(trigger.actions.contains(a), "{a:?}");
    }
    let bar = s.tree.get(box_id()).expect("a box");
    assert_eq!(bar.role, Role::TextInput);
    assert_eq!(bar.name, "Find");
    let list = s.tree.get(id().with("list")).expect("a list");
    assert_eq!(list.role, Role::List);
    assert_eq!(list.name, "Fruit");
    assert_eq!(list.children.len(), 5);
    let row = s.tree.get(list.children[2]).expect("a row");
    assert_eq!(row.role, Role::ListItem);
    assert_eq!(row.name, "Cherry");
    assert!(row.states.contains(State::Selected));
}

#[test]
fn a_box_with_no_placeholder_is_named_by_the_picker() {
    let mut h = UiHarness::new(WIN);
    let mut selected = None;
    let opts = PickerOptions {
        name: "Fruit",
        ..PickerOptions::default()
    };
    let mut run = |h: &mut UiHarness| {
        let (_, out) = h.frame(|ui| {
            ui.search_picker("p", RECT, &FRUIT, &mut selected, &opts);
        });
        out
    };
    run(&mut h);
    h.input.pointer_to(ON_TRIGGER.0, ON_TRIGGER.1);
    h.input.click(P);
    let out = run(&mut h);
    assert_eq!(out.tree.get(box_id()).expect("a box").name, "Fruit");
}

#[test]
fn a_closed_picker_with_no_value_has_no_value_node_and_settles() {
    let mut app = App::new();
    let (mut h, mut ms) = opened(&mut app);
    key(&mut h, NamedKey::Escape);
    settle(&mut h, &mut app, &mut ms);
    let s = settle(&mut h, &mut app, &mut ms);
    assert_eq!(s.tree.get(id()).expect("a node").value, None);
    assert_eq!(s.repaint, Repaint::Idle);
}

#[test]
fn at_scale_2_the_popup_radius_is_16_the_search_bar_8_and_the_dash_is_1_px_in_a_2_4_pattern() {
    use lune_ui_core::testing::ScriptedInput;
    let mut app = App::new();
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    let mut ms = 0;
    frame(&mut h, &mut app);
    click(&mut h, ON_TRIGGER);
    let s = settle(&mut h, &mut app, &mut ms);
    assert!(s.expanded());
    let radii: Vec<f32> = s
        .scene
        .merged(LayerId::Popup)
        .geometry
        .under
        .iter()
        .map(|x| x.radii[0])
        .collect();
    assert!(radii.contains(&16.0), "the popup panel: {radii:?}");
    assert!(
        radii.contains(&8.0),
        "the search bar and the rows: {radii:?}"
    );
    // The link trigger's underline: 1 logical px wide (half width 1 in device px), dashed 1 on,
    // 2 off in logical px, which is 2 and 4 device px (the pattern itself: `paint_vector.rs`).
    assert_eq!(lune_ui_core::widgets::search_picker::DASH, (1.0, 2.0));
    let mut link = App::new();
    link.trigger = Trigger::Link;
    link.selected = Some(1);
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    let s = frame(&mut h, &mut link);
    let strokes = &s.scene.layers[0].geometry.strokes;
    assert!(!strokes.is_empty());
    assert!(
        strokes.iter().all(|v| v.style[1] == 1.0),
        "2 device px wide"
    );
}

//! The tab strip (`widget.tab-strip` acceptance): press routing, the gesture and its frozen widths
//! (D123), identity, overflow, keys (D122), the outline and its notch, the busy cue per tier, the
//! menu, the nodes and the probes at scale 2.

use std::time::Duration;

use lune_ui_core::access::{AccessAction, Action, Node, Role, SemanticTree, State, Value};
use lune_ui_core::chrome;
use lune_ui_core::color::premul;
use lune_ui_core::drag::{SETTLE, SLIDE};
use lune_ui_core::input::{FrameOutput, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, Token, conformance};
use lune_ui_core::paint::{Rect, polyline_length};
use lune_ui_core::scene::{Scene, StrokeVertex};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Ui, WidgetId};
use lune_ui_core::widgets::progress::breath_opacity;
use lune_ui_core::widgets::tab_strip::{
    BRUSH, BusyCue, CONTENT_SWAP, FOCUS, GAP, HOVER, MAX_WIDTH, MIN_WIDTH, Tab, TabEvent, TabKey,
    TabStripOptions,
};
use lune_ui_core::widgets::tone::Tone;

const WIN: (f32, f32) = (1000.0, 300.0);
const CARD: Rect = Rect::new(20.0, 10.0, 600.0, 200.0);
/// As wide as the card; the viewport ends 26 px earlier, at the new-tab button.
const STRIP: Rect = Rect::new(20.0, 10.0, 600.0, 32.0);
const VIEW_RIGHT: f32 = 594.0;
const Y: f32 = 26.0;
const PRIMARY: PointerButton = PointerButton::Primary;

/// A title 165 px wide in the fake text: over the minimum with or without the close button.
fn long(c: char) -> String {
    std::iter::repeat_n(c, 30).collect()
}

/// The app side of the strip: the list, and applying events the way an app would.
#[derive(Clone)]
struct App {
    tabs: Vec<(u64, String, bool, Option<Tone>)>,
    active: Option<u64>,
    panel_focused: bool,
    busy: Option<BusyCue>,
    tone: Tone,
    extra: Vec<&'static str>,
    apply: bool,
    /// Translates the whole strip, card and all.
    parent: (f32, f32),
    tint: bool,
}

impl App {
    fn new(titles: &[&str], active: Option<u64>) -> Self {
        App {
            tabs: titles
                .iter()
                .enumerate()
                .map(|(i, t)| (i as u64, (*t).to_owned(), true, None))
                .collect(),
            active,
            panel_focused: true,
            busy: None,
            tone: Tone::Neutral,
            extra: Vec::new(),
            apply: true,
            parent: (0.0, 0.0),
            tint: false,
        }
    }

    fn of(n: u64, active: Option<u64>) -> Self {
        let titles: Vec<String> = (0..n).map(|i| format!("t{i}")).collect();
        let refs: Vec<&str> = titles.iter().map(String::as_str).collect();
        Self::new(&refs, active)
    }

    fn take(&mut self, events: &[TabEvent]) {
        if !self.apply {
            return;
        }
        for e in events {
            match *e {
                TabEvent::Activate(k) => self.active = Some(k.0),
                TabEvent::Reorder { from, to } => {
                    let t = self.tabs.remove(from);
                    self.tabs.insert(to, t);
                }
                _ => {}
            }
        }
    }
}

struct Run {
    scene: Scene,
    out: FrameOutput,
    events: Vec<TabEvent>,
}

fn draw(ui: &mut Ui<'_>, app: &App) -> Vec<TabEvent> {
    let tabs: Vec<Tab<'_>> = app
        .tabs
        .iter()
        .map(|t| Tab {
            key: TabKey(t.0),
            title: &t.1,
            closable: t.2,
            progress: t.3,
        })
        .collect();
    let extra: Vec<&str> = app.extra.clone();
    let opts = TabStripOptions {
        active: app.active.map(TabKey),
        panel_focused: app.panel_focused,
        busy: app.busy,
        busy_tone: app.tone,
        card: CARD,
        menu_extra: &extra,
        name: "Sessions",
        header_tint: app.tint.then_some([0, 0, 0, 0x40]),
    };
    let translated = app.parent != (0.0, 0.0);
    if translated {
        ui.painter().push_translate(app.parent.0, app.parent.1);
    }
    let events = ui.tab_strip("s", STRIP, &tabs, &opts).events;
    if translated {
        ui.painter().pop_translate();
    }
    events
}

fn run(h: &mut UiHarness, app: &mut App) -> Run {
    let mut events = Vec::new();
    let (scene, out) = h.frame(|ui| events = draw(ui, app));
    app.take(&events);
    Run { scene, out, events }
}

fn bench(app: &mut App) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    run(&mut h, app);
    h
}

fn tick(h: &mut UiHarness, ms: &mut u64) {
    *ms += 16;
    h.at_ms(*ms);
}

fn nodes(tree: &SemanticTree, role: Role) -> Vec<&Node> {
    tree.nodes.iter().filter(|n| n.role == role).collect()
}

fn tab_node<'t>(tree: &'t SemanticTree, name: &str) -> &'t Node {
    nodes(tree, Role::Tab)
        .into_iter()
        .find(|n| n.name == name)
        .unwrap_or_else(|| panic!("no tab {name}"))
}

/// Every tab's slot as the frame laid it out: name, left, width.
fn slots(tree: &SemanticTree) -> Vec<(String, f32, f32)> {
    nodes(tree, Role::Tab)
        .iter()
        .map(|n| (n.name.clone(), n.bounds.x, n.bounds.w))
        .collect()
}

fn mid(tree: &SemanticTree, name: &str) -> (f32, f32) {
    let b = tab_node(tree, name).bounds;
    (b.x + b.w / 2.0, b.y + b.h / 2.0)
}

/// The left edge a tab's title was drawn at, less its padding: where the tab is drawn.
fn drawn_x(scene: &Scene, title: &str) -> f32 {
    let keys = [DARK.fg_primary, DARK.fg_muted]
        .map(|c| Spec::plain(title, Style::ui(DARK.font_s, c)).cache_key(1.0));
    let item = scene.layers[0]
        .texts
        .iter()
        .find(|t| keys.contains(&t.key.0))
        .unwrap_or_else(|| panic!("no text {title}"));
    item.x - 10.0
}

/// The loop's points (logical) and colour of the stroke pass `pass`. A closed polyline of `n`
/// points is `6 n` vertices.
fn pass(scene: &Scene, from: usize, n: usize) -> &[StrokeVertex] {
    &scene.layers[0].geometry.strokes[from..from + 6 * n]
}

fn stroke_points(v: &[StrokeVertex]) -> Vec<[f32; 2]> {
    v.chunks(6)
        .map(|c| {
            [
                f32::midpoint(c[0].pos[0], c[1].pos[0]),
                f32::midpoint(c[0].pos[1], c[1].pos[1]),
            ]
        })
        .collect()
}

fn expected_loop(span: Option<(f32, f32)>) -> Vec<[f32; 2]> {
    let span = span.map(|(left, right)| chrome::TabSpan { left, right });
    let pts = chrome::outline(CARD.w, CARD.h, 32.0, span);
    let mut out: Vec<[f32; 2]> = Vec::new();
    for q in pts {
        let q = [q[0] + CARD.x, q[1] + CARD.y];
        if out
            .last()
            .is_none_or(|l| (l[0] - q[0]).abs() + (l[1] - q[1]).abs() > 1e-3)
        {
            out.push(q);
        }
    }
    if out.len() > 2 && out[0] == out[out.len() - 1] {
        out.pop();
    }
    out
}

fn assert_loop(scene: &Scene, span: Option<(f32, f32)>) {
    let want = expected_loop(span);
    let got = stroke_points(pass(scene, 0, want.len()));
    for (a, b) in got.iter().zip(&want) {
        assert!(
            (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01,
            "{a:?} != {b:?}"
        );
    }
}

fn press_at(h: &mut UiHarness, at: (f32, f32)) {
    h.input.pointer_to(at.0, at.1);
    h.input.press(PRIMARY);
}

fn key(h: &mut UiHarness, k: NamedKey) {
    h.input.key_down(Key::Named(k), None);
}

fn has(events: &[TabEvent], f: impl Fn(&TabEvent) -> bool) -> usize {
    events.iter().filter(|e| f(e)).count()
}

fn activations(events: &[TabEvent]) -> usize {
    has(events, |e| matches!(e, TabEvent::Activate(_)))
}

fn reorders(events: &[TabEvent]) -> Vec<(usize, usize)> {
    events
        .iter()
        .filter_map(|e| match e {
            TabEvent::Reorder { from, to } => Some((*from, *to)),
            _ => None,
        })
        .collect()
}

// Layout.

#[test]
fn widths_follow_the_title_within_the_limits_and_the_gap_is_between_tabs_only() {
    let wide = "x".repeat(100);
    let mut app = App::new(&["a", &long('b'), &wide], None);
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let s = slots(&r.out.tree);
    assert_eq!(s[0].2, MIN_WIDTH);
    assert_eq!(s[1].2, 18.0 + 165.0);
    assert_eq!(s[2].2, MAX_WIDTH);
    assert_eq!(s[0].1, STRIP.x, "the first tab is flush left");
    assert_eq!(s[1].1, s[0].1 + s[0].2 + GAP);
    assert_eq!(s[2].1, s[1].1 + s[1].2 + GAP);
}

#[test]
fn a_long_title_is_elided_and_a_title_that_fits_is_not() {
    let wide = "x".repeat(100);
    let mut app = App::new(&[&wide, &long('b')], None);
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let spec = Spec::plain(wide.as_str(), Style::ui(DARK.font_s, DARK.fg_muted));
    let texts = &r.scene.layers[0].texts;
    assert!(texts.iter().all(|t| t.key.0 != spec.cache_key(1.0)));
    drawn_x(&r.scene, &long('b'));
}

#[test]
fn the_close_button_shows_on_the_active_and_the_hovered_tab_only() {
    let mut app = App::of(3, Some(1));
    let mut h = bench(&mut app);
    let closes = |tree: &SemanticTree| {
        nodes(tree, Role::Button)
            .iter()
            .filter(|n| n.name == "Close tab")
            .count()
    };
    let r = run(&mut h, &mut app);
    assert_eq!(closes(&r.out.tree), 1);
    let at = mid(&r.out.tree, "t2");
    h.input.pointer_to(at.0, at.1);
    let r = run(&mut h, &mut app);
    assert_eq!(closes(&r.out.tree), 2);
    app.tabs[2].2 = false;
    let r = run(&mut h, &mut app);
    assert_eq!(closes(&r.out.tree), 1, "not closable");
}

#[test]
fn an_empty_strip_has_only_the_new_tab_button_and_no_notch() {
    let mut app = App::of(0, None);
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    assert!(nodes(&r.out.tree, Role::Tab).is_empty());
    let buttons = nodes(&r.out.tree, Role::Button);
    assert_eq!(buttons.len(), 1);
    assert_eq!(buttons[0].name, "New tab");
    assert_loop(&r.scene, None);
    assert_eq!(r.out.repaint, Repaint::Idle);
}

// Press routing.

#[test]
fn a_press_on_a_tab_activates_it_once() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    press_at(&mut h, mid(&tree, "t2"));
    let r = run(&mut h, &mut app);
    assert_eq!(r.events, vec![TabEvent::Activate(TabKey(2))]);
    assert!(run(&mut h, &mut app).events.is_empty(), "held");
    h.input.release(PRIMARY);
    assert!(run(&mut h, &mut app).events.is_empty(), "released");
}

#[test]
fn a_click_within_one_frame_activates_once() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t1");
    h.input.pointer_to(at.0, at.1);
    h.input.click(PRIMARY);
    assert_eq!(activations(&run(&mut h, &mut app).events), 1);
    assert_eq!(activations(&run(&mut h, &mut app).events), 0);
}

#[test]
fn a_press_on_the_close_button_closes_and_neither_activates_nor_arms_a_drag() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let close = nodes(&r.out.tree, Role::Button)
        .iter()
        .find(|n| n.name == "Close tab")
        .map(|n| n.bounds)
        .expect("close");
    let at = (close.x + 10.0, close.y + 10.0);
    press_at(&mut h, at);
    let r = run(&mut h, &mut app);
    assert!(r.events.is_empty());
    h.input.pointer_to(at.0 + 150.0, at.1);
    let r = run(&mut h, &mut app);
    assert!(r.events.is_empty());
    h.input.pointer_to(at.0, at.1);
    run(&mut h, &mut app);
    h.input.release(PRIMARY);
    let r = run(&mut h, &mut app);
    assert_eq!(r.events, vec![TabEvent::Close(TabKey(0))]);
}

#[test]
fn the_new_tab_button_reports_new() {
    let mut app = App::of(1, Some(0));
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let new = nodes(&r.out.tree, Role::Button)
        .iter()
        .find(|n| n.name == "New tab")
        .map(|n| n.bounds)
        .expect("new");
    assert_eq!((new.w, new.h), (20.0, 20.0));
    assert_eq!(new.right(), STRIP.right() - 6.0);
    h.input.pointer_to(new.x + 10.0, new.y + 10.0);
    run(&mut h, &mut app);
    h.input.click(PRIMARY);
    assert_eq!(run(&mut h, &mut app).events, vec![TabEvent::New]);
}

// The gesture.

/// Drags the tab under `at` by `dx` in two moves, without releasing.
fn drag_by(h: &mut UiHarness, app: &mut App, at: (f32, f32), dx: f32) -> Vec<TabEvent> {
    let mut events = Vec::new();
    press_at(h, at);
    events.extend(run(h, app).events);
    h.input.pointer_to(at.0 + 4.0, at.1);
    events.extend(run(h, app).events);
    h.input.pointer_to(at.0 + dx, at.1);
    events.extend(run(h, app).events);
    events
}

#[test]
fn press_move_4_and_release_reorders() {
    let mut app = App::of(4, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t0");
    let during = drag_by(&mut h, &mut app, at, 240.0);
    assert!(reorders(&during).is_empty());
    h.input.release(PRIMARY);
    let r = run(&mut h, &mut app);
    assert_eq!(reorders(&r.events), vec![(0, 2)]);
    assert_eq!(app.tabs[2].0, 0);
}

#[test]
fn a_press_and_release_without_travel_reports_no_reorder() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    press_at(&mut h, mid(&tree, "t1"));
    run(&mut h, &mut app);
    h.input.release(PRIMARY);
    assert!(reorders(&run(&mut h, &mut app).events).is_empty());
}

#[test]
fn escape_cancels_with_no_reorder() {
    let mut app = App::of(4, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t0");
    drag_by(&mut h, &mut app, at, 240.0);
    key(&mut h, NamedKey::Escape);
    let r = run(&mut h, &mut app);
    assert!(
        (drawn_x(&r.scene, "t0") - STRIP.x).abs() < 0.01,
        "snaps home"
    );
    h.input.release(PRIMARY);
    let r = run(&mut h, &mut app);
    assert!(reorders(&r.events).is_empty());
    assert_eq!(
        activations(&r.events),
        0,
        "the release is no second activation"
    );
}

/// The frames of a press on `pressed` while the app applies the activation: every slot of every
/// frame must be the press frame's.
fn assert_frozen_through_a_press(titles: &[&str], active: u64, pressed: &str) {
    let mut app = App::new(titles, Some(active));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, pressed);
    h.input.pointer_to(at.0, at.1);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, pressed);
    press_at(&mut h, at);
    let press_frame = run(&mut h, &mut app);
    assert_eq!(activations(&press_frame.events), 1);
    let frozen = slots(&press_frame.out.tree);
    for dx in [0.0, 2.0, 1.0] {
        h.input.pointer_to(at.0 + dx, at.1);
        assert_eq!(slots(&run(&mut h, &mut app).out.tree), frozen);
    }
    h.input.release(PRIMARY);
    run(&mut h, &mut app);
    let after = run(&mut h, &mut app);
    assert_ne!(
        slots(&after.out.tree),
        frozen,
        "live again after the gesture"
    );
}

#[test]
fn a_press_left_of_the_old_active_shifts_nothing_while_it_is_held() {
    let (a, b, c, d) = (long('a'), long('b'), long('c'), long('d'));
    assert_frozen_through_a_press(&[&a, &b, &c, &d], 1, &a);
}

#[test]
fn a_press_right_of_the_old_active_shifts_nothing_while_it_is_held() {
    let (a, b, c) = (long('a'), long('b'), long('c'));
    assert_frozen_through_a_press(&[&a, &b, &c], 0, &c);
}

#[test]
fn a_title_change_and_a_hover_change_move_no_slot_during_a_gesture() {
    let (a, b, c) = (long('a'), long('b'), long('c'));
    let mut app = App::new(&[&a, &b, &c], Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, &b);
    let frozen = {
        press_at(&mut h, at);
        slots(&run(&mut h, &mut app).out.tree)
    };
    app.tabs[0].1 = "short".to_owned();
    let r = run(&mut h, &mut app);
    let widths: Vec<f32> = slots(&r.out.tree).iter().map(|s| s.2).collect();
    assert_eq!(widths, frozen.iter().map(|s| s.2).collect::<Vec<_>>());
}

#[test]
fn a_tab_closed_mid_drag_cancels() {
    let mut app = App::of(4, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t0");
    drag_by(&mut h, &mut app, at, 240.0);
    app.tabs.remove(3);
    let r = run(&mut h, &mut app);
    assert_eq!(slots(&r.out.tree).len(), 3);
    assert!((drawn_x(&r.scene, "t0") - STRIP.x).abs() < 0.01);
    h.input.release(PRIMARY);
    assert!(reorders(&run(&mut h, &mut app).events).is_empty());
}

#[test]
fn the_dragged_tab_itself_closed_cancels() {
    let mut app = App::of(4, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t1");
    drag_by(&mut h, &mut app, at, 240.0);
    app.tabs.remove(1);
    let r = run(&mut h, &mut app);
    assert_eq!(r.out.repaint, Repaint::Now, "the frame after a change");
    h.input.release(PRIMARY);
    let r = run(&mut h, &mut app);
    assert!(reorders(&r.events).is_empty());
    // The strip works again: a fresh gesture reorders.
    let tree = r.out.tree;
    let at = mid(&tree, "t0");
    drag_by(&mut h, &mut app, at, 240.0);
    h.input.release(PRIMARY);
    assert_eq!(reorders(&run(&mut h, &mut app).events), vec![(0, 2)]);
}

#[test]
fn a_tab_swapped_for_another_at_the_same_count_cancels() {
    let mut app = App::of(4, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t0");
    drag_by(&mut h, &mut app, at, 240.0);
    app.tabs[3] = (9, "t9".to_owned(), true, None);
    run(&mut h, &mut app);
    h.input.release(PRIMARY);
    assert!(reorders(&run(&mut h, &mut app).events).is_empty());
}

#[test]
fn a_gesture_works_in_a_translated_parent() {
    let mut app = App::of(4, Some(0));
    app.parent = (37.0, 11.0);
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t1");
    let before = tab_node(&tree, "t1").bounds.x;
    assert_eq!(before, STRIP.x + 37.0 + MIN_WIDTH + GAP);
    press_at(&mut h, at);
    run(&mut h, &mut app);
    h.input.pointer_to(at.0 + 4.0, at.1);
    run(&mut h, &mut app);
    h.input.pointer_to(at.0 + 30.0, at.1);
    let r = run(&mut h, &mut app);
    let moved = drawn_x(&r.scene, "t1");
    // The grab is re-taken on the move that crosses the threshold: 26 px of the 30 show.
    assert!((moved - (before + 26.0)).abs() < 0.01, "{moved}");
    h.input.pointer_to(at.0 + 120.0, at.1);
    run(&mut h, &mut app);
    h.input.release(PRIMARY);
    assert_eq!(reorders(&run(&mut h, &mut app).events), vec![(1, 2)]);
}

#[test]
fn the_dragged_tab_paints_last() {
    let mut app = App::of(4, Some(3));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t0");
    drag_by(&mut h, &mut app, at, 100.0);
    let r = run(&mut h, &mut app);
    let texts = &r.scene.layers[0].texts;
    let at = |title: &str| {
        let spec = |c| Spec::plain(title, Style::ui(DARK.font_s, c)).cache_key(1.0);
        let keys = [spec(DARK.fg_primary), spec(DARK.fg_muted)];
        texts.iter().position(|t| keys.contains(&t.key.0))
    };
    for other in ["t1", "t2", "t3"] {
        assert!(at("t0") > at(other), "{other}");
    }
    let tab = r
        .out
        .tree
        .nodes
        .iter()
        .find(|n| n.name == "t0")
        .expect("t0");
    assert!(tab.states.contains(State::Dragging));
}

#[test]
fn a_drop_keeps_every_tab_where_it_was_drawn_and_hover_with_its_tab() {
    let mut app = App::of(4, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t0");
    let mut ms = 0;
    press_at(&mut h, at);
    tick(&mut h, &mut ms);
    run(&mut h, &mut app);
    h.input.pointer_to(at.0 + 4.0, at.1);
    tick(&mut h, &mut ms);
    run(&mut h, &mut app);
    h.input.pointer_to(at.0 + 240.0, at.1);
    tick(&mut h, &mut ms);
    run(&mut h, &mut app);
    // Let the neighbours finish sliding.
    for _ in 0..20 {
        tick(&mut h, &mut ms);
        run(&mut h, &mut app);
    }
    let names = ["t0", "t1", "t2", "t3"];
    let mut last = {
        tick(&mut h, &mut ms);
        let r = run(&mut h, &mut app);
        names.map(|n| drawn_x(&r.scene, n))
    };
    h.input.release(PRIMARY);
    for frame in 0..20 {
        tick(&mut h, &mut ms);
        let r = run(&mut h, &mut app);
        let now = names.map(|n| drawn_x(&r.scene, n));
        for (i, n) in names.iter().enumerate() {
            let step = (now[i] - last[i]).abs();
            assert!(step < 40.0, "{n} jumped {step} on frame {frame}");
        }
        last = now;
    }
    // Settled in the applied order, and t0's hover fill is at t0, where the pointer is.
    assert_eq!(
        app.tabs.iter().map(|t| t.0).collect::<Vec<_>>(),
        [1, 2, 0, 3]
    );
    let r = run(&mut h, &mut app);
    let x0 = tab_node(&r.out.tree, "t0").bounds.x;
    assert!((drawn_x(&r.scene, "t0") - x0).abs() < 0.01);
    let fills: Vec<f32> = r.scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.radii == [8.0, 8.0, 0.0, 0.0] && s.color[3] > 0.0)
        .map(|s| s.rect[0])
        .collect();
    assert_eq!(fills, vec![x0]);
}

#[test]
fn removing_a_tab_moves_no_state_to_its_neighbour() {
    let mut app = App::of(4, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t1");
    h.input.pointer_to(at.0, at.1);
    let r = run(&mut h, &mut app);
    let id1 = tab_node(&r.out.tree, "t1").id;
    let id2 = tab_node(&r.out.tree, "t2").id;
    app.tabs.remove(0);
    let r = run(&mut h, &mut app);
    assert_eq!(tab_node(&r.out.tree, "t1").id, id1);
    assert_eq!(tab_node(&r.out.tree, "t2").id, id2);
}

// Scrolling.

fn offset(tree: &SemanticTree) -> f64 {
    let area = nodes(tree, Role::ScrollArea)[0];
    match area.value {
        Some(Value::Number { value, .. }) => value,
        _ => panic!("no value"),
    }
}

#[test]
fn a_plain_wheel_scrolls_the_strip() {
    let mut app = App::of(12, Some(0));
    let mut h = bench(&mut app);
    h.input.pointer_to(100.0, Y);
    run(&mut h, &mut app);
    h.input.scroll_pixels(0.0, -70.0);
    let r = run(&mut h, &mut app);
    assert_eq!(offset(&r.out.tree), 70.0);
    h.at_ms(10);
    h.input.scroll_lines(0.0, -1.0);
    run(&mut h, &mut app);
    h.at_ms(1000);
    let r = run(&mut h, &mut app);
    assert_eq!(offset(&r.out.tree), 118.0);
}

#[test]
fn the_active_tab_is_revealed_only_when_it_changes() {
    let mut app = App::of(12, Some(0));
    let mut h = bench(&mut app);
    h.input.pointer_to(100.0, Y);
    run(&mut h, &mut app);
    h.input.scroll_pixels(0.0, -300.0);
    run(&mut h, &mut app);
    h.at_ms(500);
    run(&mut h, &mut app);
    h.at_ms(700);
    let r = run(&mut h, &mut app);
    assert_eq!(offset(&r.out.tree), 300.0, "the user's scroll stays");
    assert_ne!(
        r.out.repaint,
        Repaint::Now,
        "settled, whatever the tooltip waits for"
    );
    app.active = Some(11);
    run(&mut h, &mut app);
    h.at_ms(1000);
    let r = run(&mut h, &mut app);
    let tab = tab_node(&r.out.tree, "t11").bounds;
    assert!((tab.right() - VIEW_RIGHT).abs() < 0.01, "{}", tab.right());
    h.input.pointer_leave();
    h.at_ms(2000);
    run(&mut h, &mut app);
    h.at_ms(3000);
    assert_eq!(run(&mut h, &mut app).out.repaint, Repaint::Idle);
}

#[test]
fn a_press_on_a_cut_tab_waits_for_the_release_to_reveal_it() {
    let mut app = App::of(12, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let cut = tab_node(&tree, "t5").bounds;
    assert!(cut.right() >= VIEW_RIGHT - 1.0);
    let at = (VIEW_RIGHT - 3.0, Y);
    assert!(cut.x < at.0);
    press_at(&mut h, at);
    run(&mut h, &mut app);
    let mut ms = 0;
    for _ in 0..30 {
        tick(&mut h, &mut ms);
        let r = run(&mut h, &mut app);
        assert_eq!(offset(&r.out.tree), 0.0, "no scroll under a held pointer");
    }
    h.input.release(PRIMARY);
    tick(&mut h, &mut ms);
    let r = run(&mut h, &mut app);
    assert!(reorders(&r.events).is_empty(), "a still press is no drag");
    tick(&mut h, &mut ms);
    run(&mut h, &mut app);
    h.at_ms(ms + 1000);
    let r = run(&mut h, &mut app);
    let tab = tab_node(&r.out.tree, "t5").bounds;
    assert!(offset(&r.out.tree) > 0.0 && tab.right() <= VIEW_RIGHT + 0.01);
}

#[test]
fn the_edge_auto_scrolls_during_a_drag() {
    let mut app = App::of(12, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t1");
    let mut ms = 0;
    press_at(&mut h, at);
    tick(&mut h, &mut ms);
    run(&mut h, &mut app);
    h.input.pointer_to(at.0 + 4.0, at.1);
    tick(&mut h, &mut ms);
    run(&mut h, &mut app);
    h.input.pointer_to(VIEW_RIGHT - 4.0, at.1);
    tick(&mut h, &mut ms);
    assert_eq!(run(&mut h, &mut app).out.repaint, Repaint::Now);
    let widths = slots(&run(&mut h, &mut app).out.tree);
    let mut last = 0.0;
    for _ in 0..30 {
        tick(&mut h, &mut ms);
        let r = run(&mut h, &mut app);
        let now = offset(&r.out.tree);
        assert!(now >= last);
        last = now;
    }
    assert!(last > 50.0, "scrolled {last}");
    let after = slots(&run(&mut h, &mut app).out.tree);
    let w = |s: &[(String, f32, f32)]| s.iter().map(|t| t.2).collect::<Vec<_>>();
    assert_eq!(w(&after), w(&widths));
    h.input.release(PRIMARY);
    tick(&mut h, &mut ms);
    let r = run(&mut h, &mut app);
    let moved = reorders(&r.events);
    assert_eq!(moved.len(), 1);
    assert!(moved[0].1 > 5, "landed past the first screen: {moved:?}");
}

// Keys.

fn focused(tree: &SemanticTree) -> Option<String> {
    tree.nodes
        .iter()
        .find(|n| n.states.contains(State::Focused))
        .map(|n| n.name.clone())
}

fn tab_in(h: &mut UiHarness, app: &mut App) {
    key(h, NamedKey::Tab);
    run(h, app);
}

#[test]
fn the_strip_is_one_stop_on_the_active_tab_then_the_new_tab_button() {
    let mut app = App::of(3, Some(1));
    let mut h = bench(&mut app);
    tab_in(&mut h, &mut app);
    assert_eq!(
        focused(&run(&mut h, &mut app).out.tree).as_deref(),
        Some("t1")
    );
    tab_in(&mut h, &mut app);
    let f = focused(&run(&mut h, &mut app).out.tree);
    assert_eq!(
        f.as_deref(),
        Some("New tab"),
        "no close button in the order"
    );
    tab_in(&mut h, &mut app);
    assert_eq!(
        focused(&run(&mut h, &mut app).out.tree).as_deref(),
        Some("t1")
    );
}

#[test]
fn the_arrows_rove_and_wrap_home_and_end_go_to_the_ends_and_focus_does_not_activate() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    tab_in(&mut h, &mut app);
    run(&mut h, &mut app);
    for (k, want) in [
        (NamedKey::ArrowLeft, "t2"),
        (NamedKey::ArrowRight, "t0"),
        (NamedKey::ArrowRight, "t1"),
        (NamedKey::End, "t2"),
        (NamedKey::Home, "t0"),
    ] {
        key(&mut h, k);
        let r = run(&mut h, &mut app);
        assert!(r.events.is_empty(), "{k:?}");
        assert_eq!(
            focused(&run(&mut h, &mut app).out.tree).as_deref(),
            Some(want)
        );
    }
}

#[test]
fn enter_and_space_activate_the_focused_tab() {
    let mut app = App::of(3, Some(0));
    app.apply = false;
    let mut h = bench(&mut app);
    tab_in(&mut h, &mut app);
    run(&mut h, &mut app);
    key(&mut h, NamedKey::ArrowRight);
    run(&mut h, &mut app);
    run(&mut h, &mut app);
    for k in [NamedKey::Enter, NamedKey::Space] {
        key(&mut h, k);
        assert_eq!(
            run(&mut h, &mut app).events,
            vec![TabEvent::Activate(TabKey(1))]
        );
    }
}

#[test]
fn focus_moving_to_a_cut_tab_reveals_it() {
    let mut app = App::of(12, Some(0));
    let mut h = bench(&mut app);
    tab_in(&mut h, &mut app);
    run(&mut h, &mut app);
    key(&mut h, NamedKey::ArrowLeft);
    run(&mut h, &mut app);
    run(&mut h, &mut app);
    h.at_ms(1000);
    run(&mut h, &mut app);
    h.at_ms(2000);
    let r = run(&mut h, &mut app);
    let tab = tab_node(&r.out.tree, "t11").bounds;
    assert!(tab.right() <= VIEW_RIGHT + 0.01 && tab.x >= STRIP.x);
}

// The outline.

#[test]
fn the_notch_wraps_the_active_tab_and_a_flush_tab_keeps_the_card_corner() {
    let mut app = App::of(3, Some(1));
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let b = tab_node(&r.out.tree, "t1").bounds;
    let (l, rt) = (b.x - CARD.x, b.right() - CARD.x);
    assert_loop(&r.scene, Some((l, rt)));
    app.active = Some(0);
    let r = run(&mut h, &mut app);
    assert_loop(&r.scene, Some((0.0, MIN_WIDTH)));
}

#[test]
fn the_notch_follows_the_dragged_active_tab() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t0");
    press_at(&mut h, at);
    run(&mut h, &mut app);
    h.input.pointer_to(at.0 + 4.0, at.1);
    run(&mut h, &mut app);
    h.input.pointer_to(at.0 + 50.0, at.1);
    let r = run(&mut h, &mut app);
    let x = drawn_x(&r.scene, "t0");
    assert!((x - (CARD.x + 46.0)).abs() < 0.01, "{x}");
    assert_loop(&r.scene, Some((46.0, 46.0 + MIN_WIDTH)));
}

#[test]
fn the_active_tab_cut_under_12_px_or_scrolled_away_has_no_notch() {
    let mut app = App::of(12, Some(0));
    let mut h = bench(&mut app);
    h.input.pointer_to(300.0, Y);
    run(&mut h, &mut app);
    // 100 px scrolled: 8 px of the 108 show.
    h.input.scroll_pixels(0.0, -100.0);
    let r = run(&mut h, &mut app);
    assert_loop(&r.scene, None);
    // 90 px: 18 px show, and the notch is back.
    h.input.scroll_pixels(0.0, 10.0);
    let r = run(&mut h, &mut app);
    assert_loop(&r.scene, Some((0.0, 18.0)));
    h.input.scroll_pixels(0.0, -400.0);
    let r = run(&mut h, &mut app);
    assert_loop(&r.scene, None);
}

#[test]
fn the_active_tab_past_the_right_edge_clips_its_span_to_the_viewport() {
    let mut app = App::of(12, Some(5));
    let mut h = bench(&mut app);
    h.input.pointer_to(300.0, Y);
    run(&mut h, &mut app);
    h.at_ms(1000);
    run(&mut h, &mut app);
    let r = run(&mut h, &mut app);
    let b = tab_node(&r.out.tree, "t5").bounds;
    assert!(b.right() <= VIEW_RIGHT + 0.01);
}

#[test]
fn the_brush_is_border_when_the_panel_is_focused_else_border_subtle() {
    let mut app = App::of(2, Some(0));
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let first = r.scene.layers[0].geometry.strokes[0];
    assert_eq!(first.color, premul(DARK.border));
    assert_eq!(first.style[1], 0.5, "1 px");
    app.panel_focused = false;
    h.at_ms(60);
    let r = run(&mut h, &mut app);
    assert_eq!(r.out.repaint, Repaint::Now, "the brush tweens at instant");
    h.at_ms(500);
    let r = run(&mut h, &mut app);
    let first = r.scene.layers[0].geometry.strokes[0];
    assert_eq!(first.color, premul(DARK.border_subtle));
}

#[test]
fn the_header_tint_leaves_the_active_tab_out() {
    let mut app = App::of(3, Some(1));
    app.tint = true;
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let b = tab_node(&r.out.tree, "t1").bounds;
    let fills = &r.scene.layers[0].geometry.fills;
    assert!(!fills.is_empty());
    let inner = (b.x + 9.0, b.right() - 9.0);
    let inside: Vec<_> = fills
        .iter()
        .filter(|v| v.pos[1] < CARD.y + 30.0 && v.pos[0] > inner.0 && v.pos[0] < inner.1)
        .collect();
    assert!(inside.is_empty(), "{}", inside.len());
    let (lo, hi) = fills.iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| {
        (lo.min(v.pos[0]), hi.max(v.pos[0]))
    });
    assert!(
        lo < CARD.x + 1.5 && hi > CARD.right() - 1.5,
        "spans the card"
    );
    // No notch: one band across the card, and a flush tab has no left piece.
    app.active = None;
    assert!(
        !run(&mut h, &mut app).scene.layers[0]
            .geometry
            .fills
            .is_empty()
    );
    app.active = Some(0);
    let r = run(&mut h, &mut app);
    let left = r.scene.layers[0].geometry.fills.iter();
    assert!(left.fold(f32::MAX, |m, v| m.min(v.pos[0])) > CARD.x + 90.0);
}

// The busy cue.

fn busy_run(cue: BusyCue, tier: MotionTier, ms: u64) -> (Scene, FrameOutput, usize) {
    let mut app = App::of(3, Some(1));
    app.busy = Some(cue);
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    h.at_ms(ms);
    let r = run(&mut h, &mut app);
    let b = tab_node(&r.out.tree, "t1").bounds;
    let n = expected_loop(Some((b.x - CARD.x, b.right() - CARD.x))).len();
    (r.scene, r.out, n)
}

#[test]
fn the_sweep_is_two_passes_and_its_phase_is_time_over_2400_ms() {
    for (ms, phase) in [(0, 0.0), (600, 0.25), (1800, 0.75), (3000, 0.25)] {
        let (scene, out, n) = busy_run(BusyCue::Sweep, MotionTier::Full, ms);
        assert_eq!(out.repaint, Repaint::Now);
        let s = &scene.layers[0].geometry.strokes;
        assert_eq!(s.len(), 3 * 6 * n, "the line and two passes");
        let (bloom, line) = (s[6 * n], s[12 * n]);
        assert_eq!(bloom.style[0], 1.0);
        assert!((bloom.style[2] - phase).abs() < 1e-5, "{ms}");
        assert_eq!(bloom.style[1], 1.5, "3 px");
        assert_eq!(line.style[1], 0.5, "1 px");
        assert_eq!(bloom.extra[..2], [CARD.x + 300.0, CARD.y + 100.0]);
        let accent = premul(DARK.accent);
        assert_eq!(line.color, accent);
        assert!((bloom.color[3] - 0.25).abs() < 0.01);
    }
}

#[test]
fn the_comet_is_0_14_of_the_contour_and_travels_it_in_2400_ms() {
    let (scene, _, n) = busy_run(BusyCue::Comet, MotionTier::Full, 600);
    let s = &scene.layers[0].geometry.strokes;
    assert_eq!(s.len(), 2 * 6 * n);
    let v = s[6 * n];
    let contour = polyline_length(&stroke_points(pass(&scene, 0, n)));
    assert_eq!(v.style[0], 2.0);
    assert!((v.extra[2] - contour).abs() < 0.1);
    assert!((v.style[2] - 0.14 * contour).abs() < 0.1);
    assert!((v.style[3] - 0.25 * contour).abs() < 0.1);
}

#[test]
fn the_breath_runs_between_0_38_and_1_over_1800_ms() {
    let alpha = |ms| {
        let (scene, _, n) = busy_run(BusyCue::Breath, MotionTier::Full, ms);
        let s = &scene.layers[0].geometry.strokes;
        assert_eq!(s.len(), 2 * 6 * n);
        s[6 * n].color[3]
    };
    assert!((alpha(0) - 0.38).abs() < 0.01);
    assert!((alpha(900) - 1.0).abs() < 0.01);
    assert!((alpha(1800) - 0.38).abs() < 0.01);
    let mid = alpha(450);
    assert!((mid - breath_opacity(Duration::from_millis(450))).abs() < 0.01);
}

#[test]
fn reduced_forces_the_breath_and_off_is_static_and_idle() {
    for cue in [BusyCue::Sweep, BusyCue::Comet, BusyCue::Breath] {
        let (scene, out, n) = busy_run(cue, MotionTier::Reduced, 900);
        let s = &scene.layers[0].geometry.strokes;
        assert_eq!(s.len(), 2 * 6 * n, "{cue:?}");
        assert_eq!(s[6 * n].style[0], 0.0, "{cue:?} is solid");
        assert_eq!(out.repaint, Repaint::Now);
        let (scene, out, n) = busy_run(cue, MotionTier::Off, 900);
        assert_eq!(scene.layers[0].geometry.strokes.len(), 6 * n);
        assert_eq!(out.repaint, Repaint::Idle);
    }
}

#[test]
fn no_cue_is_idle_and_a_finished_cue_stops() {
    let mut app = App::of(3, Some(1));
    let mut h = bench(&mut app);
    assert_eq!(run(&mut h, &mut app).out.repaint, Repaint::Idle);
    app.busy = Some(BusyCue::Sweep);
    assert_eq!(run(&mut h, &mut app).out.repaint, Repaint::Now);
    app.busy = None;
    assert_eq!(run(&mut h, &mut app).out.repaint, Repaint::Idle);
}

#[test]
fn the_cue_takes_the_tone() {
    let mut app = App::of(3, Some(1));
    app.busy = Some(BusyCue::Comet);
    app.tone = Tone::Err;
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let last = r.scene.layers[0].geometry.strokes.last().expect("stroke");
    assert_eq!(last.color, premul(DARK.err));
}

// Progress.

#[test]
fn the_spinner_turns_under_full_only_and_takes_the_tone() {
    let spinner = |tier, tone: Tone, ms| {
        let mut app = App::of(2, Some(0));
        app.tabs[1].3 = Some(tone);
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        h.at_ms(ms);
        let r = run(&mut h, &mut app);
        let image = r.scene.layers[0].images.first().copied().expect("spinner");
        (image.rotation, r.out.repaint)
    };
    let (rot, repaint) = spinner(MotionTier::Full, Tone::Err, 300);
    assert!((rot - std::f32::consts::FRAC_PI_2).abs() < 1e-3, "{rot}");
    assert_eq!(repaint, Repaint::Now);
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let (rot, repaint) = spinner(tier, Tone::Err, 300);
        assert_eq!(rot, 0.0);
        assert_eq!(repaint, Repaint::Idle);
    }
}

#[test]
fn a_spinning_tab_is_a_little_wider_and_marked_busy() {
    let mut app = App::new(&[&long('a'), &long('b')], Some(0));
    app.tabs[1].3 = Some(Tone::Neutral);
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let s = slots(&r.out.tree);
    assert_eq!(s[1].2, 18.0 + 14.0 + 165.0);
    assert!(
        tab_node(&r.out.tree, &long('b'))
            .states
            .contains(State::Busy)
    );
    assert!(
        !tab_node(&r.out.tree, &long('a'))
            .states
            .contains(State::Busy)
    );
}

// The menu.

fn open_menu(h: &mut UiHarness, app: &mut App, tab: &str) -> Vec<Node> {
    let tree = run(h, app).out.tree;
    let at = mid(&tree, tab);
    h.input.pointer_to(at.0, at.1);
    run(h, app);
    h.input.click(PointerButton::Secondary);
    let r = run(h, app);
    assert_eq!(
        activations(&r.events),
        0,
        "a secondary click does not activate"
    );
    let r = run(h, app);
    nodes(&r.out.tree, Role::MenuItem)
        .into_iter()
        .cloned()
        .collect()
}

#[test]
fn a_secondary_click_opens_the_menu_with_the_apps_rows_after_the_strips() {
    let mut app = App::of(3, Some(0));
    app.extra = vec!["Duplicate Tab"];
    let mut h = bench(&mut app);
    let rows = open_menu(&mut h, &mut app, "t2");
    let names: Vec<&str> = rows.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Close",
            "Close Others",
            "Close All",
            "Rename\u{2026}",
            "Duplicate Tab"
        ]
    );
}

#[test]
fn each_row_reports_its_event_for_the_tab_the_menu_is_for() {
    let want = [
        TabEvent::Close(TabKey(2)),
        TabEvent::CloseOthers(TabKey(2)),
        TabEvent::CloseAll,
        TabEvent::Rename(TabKey(2)),
        TabEvent::MenuExtra(TabKey(2), 0),
    ];
    for (i, event) in want.into_iter().enumerate() {
        let mut app = App::of(3, Some(0));
        app.extra = vec!["Duplicate Tab"];
        let mut h = bench(&mut app);
        let rows = open_menu(&mut h, &mut app, "t2");
        let b = rows[i].bounds;
        h.at_ms(1000);
        h.input.pointer_to(b.x + 5.0, b.y + 5.0);
        run(&mut h, &mut app);
        h.input.click(PRIMARY);
        assert_eq!(run(&mut h, &mut app).events, vec![event], "row {i}");
        h.at_ms(3000);
        let r = run(&mut h, &mut app);
        assert!(nodes(&r.out.tree, Role::MenuItem).is_empty(), "closes");
    }
}

#[test]
fn a_tab_that_is_not_closable_has_no_close_row() {
    let mut app = App::of(3, Some(0));
    app.tabs[2].2 = false;
    let mut h = bench(&mut app);
    let rows = open_menu(&mut h, &mut app, "t2");
    assert_eq!(rows[0].name, "Close Others");
}

#[test]
fn removing_the_tab_closes_its_menu_and_escape_dismisses_it() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    assert_eq!(open_menu(&mut h, &mut app, "t2").len(), 4);
    app.tabs.remove(2);
    h.at_ms(1000);
    let r = run(&mut h, &mut app);
    assert!(nodes(&r.out.tree, Role::MenuItem).is_empty());

    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    open_menu(&mut h, &mut app, "t2");
    key(&mut h, NamedKey::Escape);
    run(&mut h, &mut app);
    h.at_ms(1000);
    let r = run(&mut h, &mut app);
    assert!(nodes(&r.out.tree, Role::MenuItem).is_empty());
}

#[test]
fn a_reorder_leaves_the_menu_on_its_tab() {
    let mut app = App::of(3, Some(0));
    app.extra = vec!["Extra"];
    let mut h = bench(&mut app);
    let rows = open_menu(&mut h, &mut app, "t2");
    let t = app.tabs.remove(2);
    app.tabs.insert(0, t);
    run(&mut h, &mut app);
    let b = rows[3].bounds;
    h.at_ms(1000);
    h.input.pointer_to(b.x + 5.0, b.y + 5.0);
    run(&mut h, &mut app);
    h.input.click(PRIMARY);
    assert_eq!(
        run(&mut h, &mut app).events,
        vec![TabEvent::Rename(TabKey(2))]
    );
}

#[test]
fn the_tooltip_is_the_full_title_after_the_delay() {
    let wide = "x".repeat(100);
    let mut app = App::new(&[&wide, "b"], None);
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, &wide);
    h.input.pointer_to(at.0, at.1);
    run(&mut h, &mut app);
    h.at_ms(300);
    let r = run(&mut h, &mut app);
    assert!(matches!(r.out.repaint, Repaint::At(_)));
    h.at_ms(801);
    let r = run(&mut h, &mut app);
    let tip = nodes(&r.out.tree, Role::Tooltip);
    assert_eq!(tip.len(), 1);
    assert_eq!(tip[0].name, wide);
    assert_eq!(
        tab_node(&r.out.tree, &wide).description.as_deref(),
        Some(wide.as_str())
    );
}

#[test]
fn no_tooltip_waits_or_shows_during_a_drag() {
    let mut app = App::of(4, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t0");
    drag_by(&mut h, &mut app, at, 240.0);
    h.at_ms(300);
    assert_eq!(run(&mut h, &mut app).out.repaint, Repaint::Idle);
    h.at_ms(2000);
    let r = run(&mut h, &mut app);
    assert!(nodes(&r.out.tree, Role::Tooltip).is_empty());
}

// Motion rows.

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [
            (HOVER, "hover"),
            (FOCUS, "focus"),
            (BRUSH, "brush"),
            (SLIDE, "slide"),
            (SETTLE, "settle"),
            (CONTENT_SWAP, "content swap"),
        ] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
    }
}

#[test]
fn the_content_swap_is_base_full_fast_reduced_and_nothing_off() {
    let m = DARK.motion;
    let at = |tier| m.resolve(&CONTENT_SWAP, tier).map(|r| r.duration);
    assert_eq!(at(MotionTier::Full), Some(m.duration(Token::Base)));
    assert_eq!(at(MotionTier::Reduced), Some(m.duration(Token::Fast)));
    assert_eq!(at(MotionTier::Off), None);
    assert_eq!(BRUSH.token, Token::Instant);
}

// Nodes.

#[test]
fn one_tab_list_a_tab_per_tab_and_the_active_one_selected() {
    let mut app = App::of(3, Some(1));
    app.busy = Some(BusyCue::Sweep);
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let lists = nodes(&r.out.tree, Role::TabList);
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].name, "Sessions");
    let tabs = nodes(&r.out.tree, Role::Tab);
    assert_eq!(tabs.len(), 3);
    for t in &tabs {
        let active = t.name == "t1";
        assert_eq!(t.states.contains(State::Selected), active, "{}", t.name);
        assert_eq!(t.states.contains(State::Busy), active, "{}", t.name);
        assert!(t.actions.contains(Action::Default));
        assert!(t.actions.contains(Action::ShowMenu));
    }
    let buttons = nodes(&r.out.tree, Role::Button);
    let names: Vec<&str> = buttons.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Close tab", "New tab"]);
}

fn act(h: &mut UiHarness, target: WidgetId, action: Action) {
    h.input.access(AccessAction {
        target,
        action,
        value: None,
    });
}

#[test]
fn an_access_default_activates_and_a_close_default_closes() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    act(&mut h, tab_node(&tree, "t2").id, Action::Default);
    let r = run(&mut h, &mut app);
    assert_eq!(r.events, vec![TabEvent::Activate(TabKey(2))]);
    let tree = run(&mut h, &mut app).out.tree;
    let close = nodes(&tree, Role::Button)[0].id;
    act(&mut h, close, Action::Default);
    let events = run(&mut h, &mut app).events;
    assert_eq!(has(&events, |e| matches!(e, TabEvent::Close(_))), 1);
    assert_eq!(activations(&events), 0);
}

#[test]
fn an_access_show_menu_opens_the_menu_under_the_tab() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let tab = tab_node(&tree, "t1").clone();
    act(&mut h, tab.id, Action::ShowMenu);
    let r = run(&mut h, &mut app);
    assert_eq!(activations(&r.events), 0);
    let menu = nodes(&r.out.tree, Role::Menu);
    assert_eq!(menu.len(), 1);
    assert_eq!(menu[0].bounds.x, tab.bounds.x);
    assert_eq!(menu[0].bounds.y, tab.bounds.bottom());
}

// Probes at scale 2.

#[test]
fn at_scale_2_the_strip_keeps_its_measures_in_device_px() {
    let wide = "x".repeat(100);
    let mut app = App::new(&["a", &wide, "b"], Some(0));
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    h.at_ms(0);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "b");
    h.input.pointer_to(at.0, at.1);
    run(&mut h, &mut app);
    h.at_ms(500);
    let r = run(&mut h, &mut app);
    let g = &r.scene.layers[0].geometry;
    let hover = g
        .under
        .iter()
        .find(|s| s.radii == [16.0, 16.0, 0.0, 0.0])
        .expect("hover fill");
    assert_eq!(hover.rect[3], 64.0, "strip height");
    let s = slots(&r.out.tree);
    assert_eq!(s[0].2 * 2.0, 216.0 + 56.0 - 56.0, "minimum width");
    assert_eq!(s[1].2 * 2.0, 540.0, "maximum width");
    assert_eq!((s[1].1 - s[0].1 - s[0].2) * 2.0, 10.0, "gap");
    let close = nodes(&r.out.tree, Role::Button)
        .iter()
        .find(|n| n.name == "Close tab")
        .map(|n| n.bounds)
        .expect("close");
    assert_eq!((close.w * 2.0, close.h * 2.0), (40.0, 40.0));
    assert_eq!(
        g.strokes[0].style[1], 1.0,
        "the half width of a 2 px stroke"
    );
}

//! The tab strip (`widget.tab-strip` acceptance): press routing, the gesture and its frozen widths
//! (D123), identity, overflow, keys (D122), the outline and its notch, the busy cue per tier, the
//! context report, the nodes and the probes at scale 2.

use std::time::Duration;

use lune_ui_core::access::{AccessAction, Action, Node, Role, SemanticTree, State, Value};
use lune_ui_core::chrome;
use lune_ui_core::color::premul;
use lune_ui_core::drag::{SETTLE, SLIDE};

use lune_ui_core::input::{FrameOutput, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, Token, conformance};
use lune_ui_core::paint::{Rect, polyline_length};
use lune_ui_core::scene::{Scene, StrokeVertex};
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Sense, Ui, WidgetId};
use lune_ui_core::widgets::progress::breath_opacity;
use lune_ui_core::widgets::shine;
use lune_ui_core::widgets::tab_strip::{
    BRUSH, BusyCue, CONTENT_SWAP, FOCUS, GAP, HOVER, MAX_WIDTH, MIN_WIDTH, SLIDE as NOTCH_SLIDE,
    Tab, TabEvent, TabKey, TabStripOptions, TabStripResponse,
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
    apply: bool,
    /// Translates the whole strip, card and all.
    parent: (f32, f32),
    tint: bool,
    /// The whole strip under `Ui::with_enabled`.
    enabled: bool,
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
            apply: true,
            parent: (0.0, 0.0),
            tint: false,
            enabled: true,
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
    context: Option<(TabKey, Rect)>,
}

fn draw(ui: &mut Ui<'_>, app: &App) -> Vec<TabEvent> {
    draw_response(ui, app).events
}

fn draw_response(ui: &mut Ui<'_>, app: &App) -> TabStripResponse {
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
    let opts = TabStripOptions {
        active: app.active.map(TabKey),
        panel_focused: app.panel_focused,
        busy: app.busy,
        busy_tone: app.tone,
        card: CARD,
        name: "Sessions",
        header_tint: app.tint.then_some([0, 0, 0, 0x40]),
    };
    let translated = app.parent != (0.0, 0.0);
    if translated {
        ui.painter().push_translate(app.parent.0, app.parent.1);
    }
    let response = ui.with_enabled(app.enabled, |ui| ui.tab_strip("s", STRIP, &tabs, &opts));
    if translated {
        ui.painter().pop_translate();
    }
    response
}

fn run(h: &mut UiHarness, app: &mut App) -> Run {
    let mut response = TabStripResponse::default();
    let (scene, out) = h.frame(|ui| response = draw_response(ui, app));
    app.take(&response.events);
    Run {
        scene,
        out,
        events: response.events,
        context: response.context,
    }
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
    assert_eq!(
        s[1].2,
        46.0 + 165.0,
        "the close column is part of a closable tab"
    );
    assert_eq!(s[2].2, MAX_WIDTH);
    assert_eq!(s[0].1, STRIP.x, "the first tab is flush left");
    assert_eq!(s[1].1, s[0].1 + s[0].2 + GAP);
    assert_eq!(s[2].1, s[1].1 + s[1].2 + GAP);
}

/// A title `n` characters wide (5.5 px each in the fake text).
fn chars(n: usize) -> String {
    "m".repeat(n)
}

#[test]
fn hover_and_activation_move_no_slot_for_titles_from_90_to_224_px() {
    // 17 to 41 characters: 93.5 to 225.5 px of title, from under the minimum to over the maximum.
    let titles: Vec<String> = (17..=41).step_by(4).map(chars).collect();
    let refs: Vec<&str> = titles.iter().map(String::as_str).collect();
    let mut app = App::new(&refs, Some(0));
    let mut h = bench(&mut app);
    let rest = slots(&run(&mut h, &mut app).out.tree);
    for name in &refs {
        let at = mid(&run(&mut h, &mut app).out.tree, name);
        h.input.pointer_to(at.0, at.1);
        run(&mut h, &mut app);
        assert_eq!(slots(&run(&mut h, &mut app).out.tree), rest, "hover {name}");
    }
    h.input.pointer_to(WIN.0 - 1.0, WIN.1 - 1.0);
    for i in 1..refs.len() as u64 {
        app.active = Some(i);
        run(&mut h, &mut app);
        assert_eq!(slots(&run(&mut h, &mut app).out.tree), rest, "active {i}");
    }
}

#[test]
fn the_elided_title_is_the_same_hovered_or_not() {
    let wide = "x".repeat(100);
    let mut app = App::new(&[&wide, &long('b')], Some(1));
    let mut h = bench(&mut app);
    let titles = |r: &Run| -> Vec<(u64, u32)> {
        r.scene.layers[0]
            .texts
            .iter()
            .map(|t| (t.key.0, t.x.to_bits()))
            .collect()
    };
    let rest = titles(&run(&mut h, &mut app));
    let at = mid(&run(&mut h, &mut app).out.tree, &wide);
    h.input.pointer_to(at.0, at.1);
    run(&mut h, &mut app);
    assert_eq!(titles(&run(&mut h, &mut app)), rest);
}

#[test]
fn a_tab_that_is_not_closable_reserves_no_close_column() {
    let mut app = App::new(&["a", &long('b')], None);
    app.tabs[1].2 = false;
    let mut h = bench(&mut app);
    let s = slots(&run(&mut h, &mut app).out.tree);
    assert_eq!(s[1].2, 18.0 + 165.0);
    let at = mid(&run(&mut h, &mut app).out.tree, &long('b'));
    h.input.pointer_to(at.0, at.1);
    run(&mut h, &mut app);
    assert_eq!(slots(&run(&mut h, &mut app).out.tree), s);
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

/// The new-tab button's bounds in a frame's tree.
fn new_bounds(tree: &SemanticTree) -> Rect {
    nodes(tree, Role::Button)
        .iter()
        .find(|n| n.name == "New tab")
        .map(|n| n.bounds)
        .expect("new")
}

#[test]
fn the_new_tab_button_reports_new() {
    let mut app = App::of(1, Some(0));
    let mut h = bench(&mut app);
    let r = run(&mut h, &mut app);
    let new = new_bounds(&r.out.tree);
    assert_eq!((new.w, new.h), (20.0, 20.0));
    assert_eq!(new.x, STRIP.x + MIN_WIDTH + 6.0, "after the last tab");
    h.input.pointer_to(new.x + 10.0, new.y + 10.0);
    run(&mut h, &mut app);
    h.input.click(PRIMARY);
    assert_eq!(run(&mut h, &mut app).events, vec![TabEvent::New]);
}

#[test]
fn the_new_tab_button_follows_the_last_tab_by_6_px() {
    let mut app = App::new(&[&long('a'), "b", &long('c')], Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let last = slots(&tree).pop().expect("a tab");
    let new = new_bounds(&tree);
    assert_eq!(new.x, last.1 + last.2 + 6.0);
    assert_eq!(new.y, STRIP.y + (STRIP.h - 20.0) / 2.0);
    // A tab closed or added moves it with the strip's end.
    app.tabs.pop();
    let tree = run(&mut h, &mut app).out.tree;
    let last = slots(&tree).pop().expect("a tab");
    assert_eq!(new_bounds(&tree).x, last.1 + last.2 + 6.0);
}

#[test]
fn with_no_tabs_the_new_tab_button_sits_6_px_in_from_the_strip() {
    let mut app = App::of(0, None);
    let mut h = bench(&mut app);
    let new = new_bounds(&run(&mut h, &mut app).out.tree);
    assert_eq!(new.x, STRIP.x + 6.0);
}

#[test]
fn on_overflow_the_new_tab_button_is_pinned_at_the_right_end() {
    let mut app = App::of(12, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let new = new_bounds(&tree);
    assert_eq!(new.right(), STRIP.right() - 6.0);
    // The viewport is what it was: it ends at the button's left edge.
    let area = nodes(&tree, Role::ScrollArea)[0];
    assert_eq!(area.bounds.right(), VIEW_RIGHT);
    h.input.pointer_to(new.x + 10.0, new.y + 10.0);
    run(&mut h, &mut app);
    h.input.click(PRIMARY);
    assert_eq!(run(&mut h, &mut app).events, vec![TabEvent::New]);
}

/// A frame with an app region over the whole strip, registered first so the strip's own regions
/// win where they exist. Returns what the region saw and whether it was the hovered one.
fn under_frame(h: &mut UiHarness, app: &mut App) -> (bool, bool, Vec<TabEvent>) {
    let mut seen = (false, false);
    let mut events = Vec::new();
    h.frame(|ui| {
        let id = ui.id("under");
        let hovered = ui.painter().hover == Some(id);
        let resp = ui.interact(id, STRIP, Sense::CLICK);
        seen = (resp.press_started, hovered);
        events = draw(ui, app);
    });
    app.take(&events);
    (seen.0, seen.1, events)
}

#[test]
fn a_press_in_the_empty_strip_space_reaches_the_region_under_it() {
    let mut app = App::of(2, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let last = slots(&tree).pop().expect("a tab");
    let new = new_bounds(&tree);
    let end = last.1 + last.2;
    // Between the last tab and the button, and between the button and the strip's right end.
    for x in [end + 3.0, new.right() + 10.0, STRIP.right() - 2.0] {
        let mut h = bench(&mut app);
        h.input.pointer_to(x, Y);
        under_frame(&mut h, &mut app);
        let (_, hovered, _) = under_frame(&mut h, &mut app);
        assert!(hovered, "the app region is the hovered one at {x}");
        h.input.press(PRIMARY);
        let (pressed, _, events) = under_frame(&mut h, &mut app);
        assert!(pressed, "the app region gets the press at {x}");
        assert!(events.is_empty(), "the strip reports nothing at {x}");
    }
}

#[test]
fn nothing_of_the_strip_is_hovered_in_its_empty_space() {
    let mut app = App::of(2, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let last = slots(&tree).pop().expect("a tab");
    h.input.pointer_to(last.1 + last.2 + 3.0, Y);
    run(&mut h, &mut app);
    let mut hover = None;
    h.frame(|ui| {
        hover = Some(ui.painter().hover);
        draw(ui, &app);
    });
    assert_eq!(hover, Some(None));
}

#[test]
fn on_overflow_the_wheel_still_scrolls_the_strip() {
    let mut app = App::of(12, Some(0));
    let mut h = bench(&mut app);
    h.input.pointer_to(100.0, Y);
    run(&mut h, &mut app);
    h.input.scroll_pixels(0.0, -40.0);
    assert_eq!(offset(&run(&mut h, &mut app).out.tree), 40.0);
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
    // An activation moves no width, so the layout is live again when a title change moves one.
    app.tabs[0].1 = "short".to_owned();
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
    run(&mut h, &mut app);
    // The switch slides; once it is over the notch is on the flush tab.
    h.at_ms(1000);
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

// The notch's slide.

/// A tab's span in the card's space, from the tree.
fn span_of(tree: &SemanticTree, name: &str) -> (f32, f32) {
    let b = tab_node(tree, name).bounds;
    (b.x - CARD.x, b.right() - CARD.x)
}

fn mix(a: (f32, f32), b: (f32, f32), p: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * p, a.1 + (b.1 - a.1) * p)
}

/// A strip of `n` tabs with `from` active, settled at 0 ms, and its tree.
fn slide_bench(tier: MotionTier, n: u64, from: u64) -> (UiHarness, App, SemanticTree) {
    let mut app = App::of(n, Some(from));
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    h.at_ms(0);
    let tree = run(&mut h, &mut app).out.tree;
    h.input.pointer_to(WIN.0 - 1.0, WIN.1 - 1.0);
    for ms in [30, 60, 90] {
        h.at_ms(ms);
        run(&mut h, &mut app);
    }
    (h, app, tree)
}

/// The frames after the switch was applied: the first is the key's change (the notch is where it
/// was), the middle is half way, and the last is on the new tab with nothing left to draw.
fn assert_slides(h: &mut UiHarness, app: &mut App, from: (f32, f32), to: (f32, f32)) {
    h.at_ms(200);
    let r = run(h, app);
    assert_loop(&r.scene, Some(from));
    assert_eq!(r.out.repaint, Repaint::Now);
    h.at_ms(290);
    let r = run(h, app);
    assert_loop(&r.scene, Some(mix(from, to, 0.5)));
    assert_eq!(r.out.repaint, Repaint::Now);
    h.at_ms(700);
    let r = run(h, app);
    assert_loop(&r.scene, Some(to));
    h.at_ms(716);
    let r = run(h, app);
    assert_loop(&r.scene, Some(to));
    assert_eq!(r.out.repaint, Repaint::Idle, "nothing left to draw");
}

#[test]
fn a_click_slides_the_notch_to_the_new_tab() {
    let (mut h, mut app, tree) = slide_bench(MotionTier::Full, 4, 1);
    let (from, to) = (span_of(&tree, "t1"), span_of(&tree, "t3"));
    h.at_ms(100);
    press_at(&mut h, mid(&tree, "t3"));
    assert_eq!(activations(&run(&mut h, &mut app).events), 1);
    h.input.release(PRIMARY);
    h.input.pointer_to(WIN.0 - 1.0, WIN.1 - 1.0);
    assert_slides(&mut h, &mut app, from, to);
}

#[test]
fn enter_on_a_roving_tab_slides_the_notch() {
    let (mut h, mut app, tree) = slide_bench(MotionTier::Full, 4, 0);
    let (from, to) = (span_of(&tree, "t0"), span_of(&tree, "t1"));
    h.at_ms(60);
    tab_in(&mut h, &mut app);
    key(&mut h, NamedKey::ArrowRight);
    run(&mut h, &mut app);
    h.at_ms(100);
    key(&mut h, NamedKey::Enter);
    let r = run(&mut h, &mut app);
    assert_eq!(activations(&r.events), 1);
    assert_eq!(app.active, Some(1));
    // The ring and the slide: only the notch is asserted here, so skip the idle check.
    h.at_ms(200);
    assert_loop(&run(&mut h, &mut app).scene, Some(from));
    h.at_ms(290);
    assert_loop(&run(&mut h, &mut app).scene, Some(mix(from, to, 0.5)));
    h.at_ms(700);
    assert_loop(&run(&mut h, &mut app).scene, Some(to));
}

#[test]
fn closing_the_active_tab_slides_the_notch_to_the_one_that_takes_over() {
    let (mut h, mut app, tree) = slide_bench(MotionTier::Full, 4, 1);
    let (from, to) = (span_of(&tree, "t1"), span_of(&tree, "t0"));
    app.tabs.remove(1);
    app.active = Some(0);
    assert_slides(&mut h, &mut app, from, to);
}

#[test]
fn closing_the_last_active_tab_keeps_the_notch_for_the_whole_slide() {
    let (mut h, mut app, tree) = slide_bench(MotionTier::Full, 4, 3);
    let (from, to) = (span_of(&tree, "t3"), span_of(&tree, "t2"));
    app.tabs.pop();
    app.active = Some(2);
    // The old span lies past the tabs that are left, where the viewport no longer reaches.
    assert_slides(&mut h, &mut app, from, to);
}

#[test]
fn a_second_switch_mid_slide_continues_from_where_the_notch_is() {
    let (mut h, mut app, tree) = slide_bench(MotionTier::Full, 4, 0);
    let (a, b) = (span_of(&tree, "t0"), span_of(&tree, "t3"));
    let c = span_of(&tree, "t1");
    app.active = Some(3);
    h.at_ms(200);
    run(&mut h, &mut app);
    h.at_ms(290);
    assert_loop(&run(&mut h, &mut app).scene, Some(mix(a, b, 0.5)));
    app.active = Some(1);
    assert_loop(&run(&mut h, &mut app).scene, Some(mix(a, b, 0.5)));
    h.at_ms(380);
    assert_loop(
        &run(&mut h, &mut app).scene,
        Some(mix(mix(a, b, 0.5), c, 0.5)),
    );
    h.at_ms(900);
    assert_loop(&run(&mut h, &mut app).scene, Some(c));
}

#[test]
fn reduced_and_off_switch_in_one_frame() {
    for tier in [MotionTier::Reduced, MotionTier::Off] {
        let (mut h, mut app, tree) = slide_bench(tier, 4, 1);
        app.active = Some(3);
        h.at_ms(200);
        let r = run(&mut h, &mut app);
        assert_loop(&r.scene, Some(span_of(&tree, "t3")));
        // The old tab's close button may still fade; the notch does not.
        h.at_ms(1000);
        assert_eq!(run(&mut h, &mut app).out.repaint, Repaint::Idle, "{tier:?}");
    }
}

#[test]
fn the_first_frame_and_an_active_tab_that_comes_and_goes_do_not_slide() {
    let mut app = App::of(4, Some(2));
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    let r = run(&mut h, &mut app);
    let (t2, t1) = (span_of(&r.out.tree, "t2"), span_of(&r.out.tree, "t1"));
    assert_loop(&r.scene, Some(t2));
    h.at_ms(16);
    assert_eq!(run(&mut h, &mut app).out.repaint, Repaint::Idle);
    app.active = None;
    h.at_ms(100);
    assert_loop(&run(&mut h, &mut app).scene, None);
    app.active = Some(1);
    h.at_ms(116);
    let r = run(&mut h, &mut app);
    assert_loop(&r.scene, Some(t1));
}

#[test]
fn a_reorder_drag_and_its_drop_keep_the_notch_on_the_active_tab_each_frame() {
    let mut app = App::of(4, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t0");
    let mut ms = 0;
    let on_tab = |h: &mut UiHarness, app: &mut App, ms: &mut u64| {
        tick(h, ms);
        let r = run(h, app);
        let x = drawn_x(&r.scene, "t0") - CARD.x;
        assert_loop(&r.scene, Some((x, x + MIN_WIDTH)));
    };
    press_at(&mut h, at);
    on_tab(&mut h, &mut app, &mut ms);
    h.input.pointer_to(at.0 + 4.0, at.1);
    on_tab(&mut h, &mut app, &mut ms);
    h.input.pointer_to(at.0 + 240.0, at.1);
    for _ in 0..25 {
        on_tab(&mut h, &mut app, &mut ms);
    }
    h.input.release(PRIMARY);
    for _ in 0..30 {
        on_tab(&mut h, &mut app, &mut ms);
    }
    assert_eq!(app.tabs[2].0, 0, "the drop was applied");
}

#[test]
fn a_tab_opened_or_closed_left_of_the_active_one_and_a_title_change_do_not_slide() {
    let (mut h, mut app, _) = slide_bench(MotionTier::Full, 4, 2);
    let mut ms = 100;
    let mut exact = |h: &mut UiHarness, app: &mut App, name: &str| {
        ms += 16;
        h.at_ms(ms);
        let r = run(h, app);
        assert_loop(&r.scene, Some(span_of(&r.out.tree, name)));
    };
    app.tabs.insert(0, (99, "new".to_owned(), true, None));
    exact(&mut h, &mut app, "t2");
    app.tabs.remove(0);
    exact(&mut h, &mut app, "t2");
    app.tabs[2].1 = "a longer title".to_owned();
    exact(&mut h, &mut app, "a longer title");
    app.tabs[2].1 = "x".to_owned();
    exact(&mut h, &mut app, "x");
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
    assert_eq!(last.color, premul(DARK.status_error_light));
}

#[test]
fn the_cue_tints_are_the_light_status_brushes() {
    for (tone, want) in [
        (Tone::Warn, DARK.status_warning_light),
        (Tone::Err, DARK.status_error_light),
        (Tone::Neutral, DARK.accent),
    ] {
        let mut app = App::of(3, Some(1));
        app.busy = Some(BusyCue::Comet);
        app.tone = tone;
        let mut h = bench(&mut app);
        let r = run(&mut h, &mut app);
        let last = r.scene.layers[0].geometry.strokes.last().expect("stroke");
        assert_eq!(last.color, premul(want), "{tone:?}");
    }
}

// Progress: the title shines.

/// One tab `long('a')` (165 px in the fake text), active, shining in `tone`, at `ms` under `tier`.
fn shining(tier: MotionTier, tone: Tone, ms: u64) -> Run {
    let mut app = App::new(&[&long('a')], Some(0));
    app.tabs[0].3 = Some(tone);
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    h.at_ms(ms);
    run(&mut h, &mut app)
}

/// The title's text items: the ones drawn where the first text is, left to right by clip.
fn title_texts(scene: &Scene) -> Vec<&lune_ui_core::scene::TextItem> {
    let at = scene.layers[0].texts.first().expect("a title");
    let mut items: Vec<_> = scene.layers[0]
        .texts
        .iter()
        .filter(|t| t.x == at.x && t.y == at.y)
        .collect();
    items.sort_by(|a, b| a.clip.x.total_cmp(&b.clip.x));
    items
}

/// The light a tone gives a shining title.
fn light(tone: Tone) -> [u8; 4] {
    match tone {
        Tone::Neutral => DARK.fg_bright,
        Tone::Warn => DARK.status_warning_light,
        _ => DARK.status_error_light,
    }
}

fn title_key(color: [u8; 4]) -> u64 {
    Spec::plain(long('a'), Style::ui(DARK.font_s, color)).cache_key(1.0)
}

#[test]
fn the_band_sits_where_the_formula_says() {
    // Off both ends the title is drawn whole, as it is.
    assert_eq!(
        title_texts(&shining(MotionTier::Full, Tone::Neutral, 0).scene).len(),
        1
    );
    // Fully on the title: the pieces other than the outer two are the strips and span the band.
    for ms in [450u64, 900, 1350] {
        let r = shining(MotionTier::Full, Tone::Neutral, ms);
        let items = title_texts(&r.scene);
        let x = items[0].x;
        let centre = shine::band_center(Duration::from_millis(ms), x, 165.0);
        let inner = &items[1..items.len() - 1];
        let left = inner.first().expect("strips").clip.x;
        let right = inner.last().map(|t| t.clip.x + t.clip.w).expect("strips");
        assert!(
            (left - (centre - shine::BAND_WIDTH / 2.0)).abs() <= 0.5
                && (right - (centre + shine::BAND_WIDTH / 2.0)).abs() <= 0.5,
            "{ms} ms: {left}..{right} around {centre}"
        );
    }
}

#[test]
fn the_band_starts_off_the_left_end_and_leaves_off_the_right() {
    let x = 30.0;
    let at = |ms| shine::band_center(Duration::from_millis(ms), x, 165.0);
    assert_eq!(at(0) + shine::BAND_WIDTH / 2.0, x);
    assert!((at(900) - (x + 82.5)).abs() < 1e-3);
    assert!(at(1799) - shine::BAND_WIDTH / 2.0 < x + 165.0);
    assert!(at(1799) - shine::BAND_WIDTH / 2.0 > x + 160.0);
    assert_eq!(at(1800), at(0));
}

#[test]
fn the_strips_partition_the_band_with_no_overlap_or_gap() {
    for scale in [1.0_f32, 1.5, 2.0, 3.0] {
        let band = shine::Band {
            left: 31.3,
            width: shine::BAND_WIDTH,
            light: DARK.fg_bright,
        };
        let strips = shine::strips(&band, scale);
        assert!(strips.len() >= 2, "scale {scale}");
        for pair in strips.windows(2) {
            assert_eq!(pair[0].1, pair[1].0, "scale {scale}: a seam");
        }
        for (a, b, _) in &strips {
            assert!(b > a);
            // On a whole device pixel.
            assert!(
                (a * scale - (a * scale).round()).abs() < 1e-3,
                "{a} at {scale}"
            );
        }
        let (first, last) = (strips[0].0, strips[strips.len() - 1].1);
        assert!((first - band.left).abs() <= 0.5 / scale + 1e-3);
        assert!((last - (band.left + band.width)).abs() <= 0.5 / scale + 1e-3);
    }
}

#[test]
fn the_bell_is_soft_at_both_edges_and_full_at_the_centre() {
    assert_eq!(shine::bell(0.0), 0.0);
    assert_eq!(shine::bell(1.0), 0.0);
    assert!((shine::bell(0.5) - 1.0).abs() < 1e-6);
    for f in [0.1_f32, 0.3, 0.45] {
        assert!((shine::bell(f) - shine::bell(1.0 - f)).abs() < 1e-5);
        assert!(shine::bell(f) < shine::bell(f + 0.05));
    }
    // Zero slope at the edge: the first step is far under the average one.
    assert!(shine::bell(0.01) < 0.002);
}

#[test]
fn no_strip_is_a_step_from_its_neighbour_for_any_tone() {
    for tone in [Tone::Neutral, Tone::Warn, Tone::Err] {
        for base in [DARK.fg_primary, DARK.fg_muted] {
            let band = shine::Band {
                left: 0.0,
                width: shine::BAND_WIDTH,
                light: light(tone),
            };
            let colours: Vec<[u8; 4]> = shine::strips(&band, 1.0)
                .iter()
                .map(|s| shine::mix(base, band.light, s.2))
                .chain([base])
                .collect();
            let first = [base, colours[0]];
            for pair in colours.windows(2).chain([&first[..]]) {
                let step = (0..4)
                    .map(|i| pair[0][i].abs_diff(pair[1][i]))
                    .max()
                    .unwrap_or(0);
                assert!(step <= 12, "{tone:?}: a step of {step}");
            }
        }
    }
}

#[test]
fn the_strips_are_the_title_cut_not_a_second_title() {
    // Every piece is the one title at the one place; only the colour and the clip differ.
    let r = shining(MotionTier::Full, Tone::Err, 900);
    let items = title_texts(&r.scene);
    assert!(items.len() > shine::STRIPS / 2, "{}", items.len());
    for pair in items.windows(2) {
        assert_eq!(
            pair[0].clip.x + pair[0].clip.w,
            pair[1].clip.x,
            "overlap or gap"
        );
    }
    // Both outer pieces are the title's own colour.
    let own = title_key(DARK.fg_primary);
    assert_eq!(items[0].key.0, own);
    assert_eq!(items[items.len() - 1].key.0, own);
}

#[test]
fn the_tone_picks_the_light() {
    for tone in [Tone::Neutral, Tone::Warn, Tone::Err] {
        // The centre strip of the band at the title's middle.
        let r = shining(MotionTier::Full, tone, 900);
        let band = shine::Band {
            left: shine::band_center(Duration::from_millis(900), 30.0, 165.0)
                - shine::BAND_WIDTH / 2.0,
            width: shine::BAND_WIDTH,
            light: light(tone),
        };
        let top = shine::strips(&band, 1.0)
            .into_iter()
            .max_by(|a, b| a.2.total_cmp(&b.2))
            .expect("strips");
        let want = title_key(shine::mix(DARK.fg_primary, light(tone), top.2));
        assert!(
            title_texts(&r.scene).iter().any(|t| t.key.0 == want),
            "{tone:?}"
        );
        // Off is the fixed mix of the same light.
        let r = shining(MotionTier::Off, tone, 300);
        let want = title_key(shine::mix(DARK.fg_primary, light(tone), shine::STATIC_MIX));
        assert_eq!(title_texts(&r.scene)[0].key.0, want, "{tone:?} under Off");
    }
}

#[test]
fn reduced_breathes_in_one_piece_and_off_is_static_and_idle() {
    let at = |tier, ms| shining(tier, Tone::Err, ms);
    // Reduced: one text, one clip, the colour moving with the breath.
    let (a, b) = (at(MotionTier::Reduced, 0), at(MotionTier::Reduced, 900));
    assert_eq!(title_texts(&a.scene).len(), 1);
    assert_eq!(title_texts(&b.scene).len(), 1);
    assert_eq!(title_texts(&a.scene)[0].clip, title_texts(&b.scene)[0].clip);
    assert_ne!(title_texts(&a.scene)[0].key, title_texts(&b.scene)[0].key);
    let want = |ms| {
        title_key(shine::mix(
            DARK.fg_primary,
            DARK.status_error_light,
            shine::breath_mix(Duration::from_millis(ms)),
        ))
    };
    assert_eq!(title_texts(&a.scene)[0].key.0, want(0));
    assert_eq!(title_texts(&b.scene)[0].key.0, want(900));
    assert_eq!(a.out.repaint, Repaint::Now);
    // Off: the same at any time, and nothing to repaint.
    let (a, b) = (at(MotionTier::Off, 0), at(MotionTier::Off, 900));
    assert_eq!(title_texts(&a.scene).len(), 1);
    assert_eq!(title_texts(&a.scene)[0], title_texts(&b.scene)[0]);
    assert_eq!(a.out.repaint, Repaint::Idle);
    assert_eq!(b.out.repaint, Repaint::Idle);
}

#[test]
fn only_a_shining_tab_asks_for_frames() {
    assert_eq!(
        shining(MotionTier::Full, Tone::Neutral, 300).out.repaint,
        Repaint::Now
    );
    let mut app = App::of(2, Some(0));
    let mut h = UiHarness::new(WIN);
    h.at_ms(300);
    assert_eq!(run(&mut h, &mut app).out.repaint, Repaint::Idle);
}

#[test]
fn a_tab_in_progress_is_as_wide_as_one_not_and_marked_busy() {
    let widths = |progress: Option<Tone>| {
        let mut app = App::new(&[&long('a'), &long('b')], Some(0));
        app.tabs[1].3 = progress;
        let mut h = bench(&mut app);
        let r = run(&mut h, &mut app);
        (slots(&r.out.tree), r.out.tree)
    };
    let (still, _) = widths(None);
    let (shone, tree) = widths(Some(Tone::Neutral));
    assert_eq!(still, shone);
    assert!(tab_node(&tree, &long('b')).states.contains(State::Busy));
    assert!(!tab_node(&tree, &long('a')).states.contains(State::Busy));
}

#[test]
fn the_title_starts_where_a_still_one_starts() {
    let still = shining(MotionTier::Off, Tone::Neutral, 0);
    let mut app = App::new(&[&long('a')], Some(0));
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    let plain = run(&mut h, &mut app);
    assert_eq!(
        title_texts(&still.scene)[0].x,
        title_texts(&plain.scene)[0].x
    );
}

// The context report: the strip opens no menu, it names the tab and its rect.

/// A secondary click on the middle of tab `tab`; the frame it lands in.
fn secondary_click(h: &mut UiHarness, app: &mut App, tab: &str) -> Run {
    let tree = run(h, app).out.tree;
    let at = mid(&tree, tab);
    h.input.pointer_to(at.0, at.1);
    run(h, app);
    h.input.click(PointerButton::Secondary);
    run(h, app)
}

fn no_menu(r: &Run) {
    assert!(nodes(&r.out.tree, Role::Menu).is_empty(), "no menu node");
    assert!(
        nodes(&r.out.tree, Role::MenuItem).is_empty(),
        "no menu item"
    );
}

#[test]
fn a_secondary_click_reports_the_tab_and_its_rect_and_opens_no_menu() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let r = secondary_click(&mut h, &mut app, "t2");
    assert_eq!(
        activations(&r.events),
        0,
        "a secondary click does not activate"
    );
    let tree = run(&mut h, &mut app).out.tree;
    let want = tab_node(&tree, "t2").bounds;
    assert_eq!(r.context, Some((TabKey(2), want)));
    no_menu(&r);
    no_menu(&run(&mut h, &mut app));
}

#[test]
fn the_close_buttons_secondary_click_reports_its_tab() {
    let mut app = App::of(3, Some(1));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let t = tab_node(&tree, "t1").bounds;
    // The active tab shows its close button.
    let close = nodes(&tree, Role::Button)
        .into_iter()
        .find(|b| b.name == "Close tab")
        .expect("the active tab's close button")
        .bounds;
    h.input
        .pointer_to(close.x + close.w / 2.0, close.y + close.h / 2.0);
    run(&mut h, &mut app);
    h.input.click(PointerButton::Secondary);
    let r = run(&mut h, &mut app);
    assert_eq!(r.context, Some((TabKey(1), t)));
    assert!(
        !r.events.iter().any(|e| matches!(e, TabEvent::Close(_))),
        "a secondary click does not close"
    );
}

#[test]
fn a_tab_that_is_not_closable_is_still_reported() {
    let mut app = App::of(3, Some(0));
    app.tabs[2].2 = false;
    let mut h = bench(&mut app);
    let r = secondary_click(&mut h, &mut app, "t2");
    assert_eq!(r.context.map(|c| c.0), Some(TabKey(2)));
}

#[test]
fn no_secondary_click_reports_no_context() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t2");
    h.input.pointer_to(at.0, at.1);
    assert_eq!(run(&mut h, &mut app).context, None);
    h.input.click(PRIMARY);
    let r = run(&mut h, &mut app);
    assert_eq!(activations(&r.events), 1);
    assert_eq!(r.context, None);
    assert_eq!(run(&mut h, &mut app).context, None, "one frame only");
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

#[test]
fn a_press_hides_the_tooltip_until_the_pointer_leaves() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    press_at(&mut h, mid(&tree, "t1"));
    run(&mut h, &mut app);
    h.at_ms(100);
    run(&mut h, &mut app);
    h.input.release(PRIMARY);
    h.at_ms(200);
    run(&mut h, &mut app);
    h.at_ms(300);
    run(&mut h, &mut app);
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
            (NOTCH_SLIDE, "notch slide"),
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
fn an_access_show_menu_reports_the_tab_and_opens_no_menu() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let tab = tab_node(&tree, "t1").clone();
    act(&mut h, tab.id, Action::ShowMenu);
    let r = run(&mut h, &mut app);
    assert_eq!(activations(&r.events), 0);
    assert_eq!(r.context, Some((TabKey(1), tab.bounds)));
    no_menu(&r);
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

// A disabled strip.

fn disabled_after_focus() -> (UiHarness, App) {
    let mut app = App::of(3, Some(1));
    let mut h = bench(&mut app);
    tab_in(&mut h, &mut app);
    assert_eq!(
        focused(&run(&mut h, &mut app).out.tree).as_deref(),
        Some("t1")
    );
    app.enabled = false;
    run(&mut h, &mut app);
    run(&mut h, &mut app);
    (h, app)
}

#[test]
fn a_disabled_strip_activates_nothing_from_enter_or_space_and_still_does_after_enabling() {
    for k in [NamedKey::Enter, NamedKey::Space] {
        let (mut h, mut app) = disabled_after_focus();
        // An unconsumed key bubbles to the shortcuts, so this fires if the tab lets the key by.
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        key(&mut h, k);
        let r = run(&mut h, &mut app);
        assert!(
            r.events.is_empty(),
            "{k:?} on a disabled strip: {:?}",
            r.events
        );
        assert!(
            r.out.fired.is_empty(),
            "{k:?} fell through a disabled strip: {:?}",
            r.out.fired
        );
        app.enabled = true;
        key(&mut h, k);
        let r = run(&mut h, &mut app);
        assert_eq!(activations(&r.events), 1, "{k:?} works again once enabled");
    }
}

#[test]
fn a_disabled_strip_keeps_the_arrows_home_and_end_from_the_shortcuts() {
    use NamedKey::{ArrowLeft, ArrowRight, End, Home};
    for k in [ArrowLeft, ArrowRight, Home, End] {
        let (mut h, mut app) = disabled_after_focus();
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        key(&mut h, k);
        let r = run(&mut h, &mut app);
        assert!(r.events.is_empty(), "{k:?}: {:?}", r.events);
        assert!(
            r.out.fired.is_empty(),
            "{k:?} fell through: {:?}",
            r.out.fired
        );
    }
}

#[test]
fn a_disabled_strip_nodes_are_disabled_and_offer_no_focus_or_action() {
    let mut app = App::of(3, Some(1));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    for n in nodes(&tree, Role::Tab) {
        assert!(n.states.contains(State::Focusable) && !n.states.contains(State::Disabled));
        assert!(n.actions.contains(Action::Default));
    }
    assert!(
        !nodes(&tree, Role::TabList)[0]
            .states
            .contains(State::Disabled)
    );
    app.enabled = false;
    let tree = run(&mut h, &mut app).out.tree;
    let tabs = nodes(&tree, Role::Tab);
    assert_eq!(tabs.len(), 3);
    let buttons = nodes(&tree, Role::Button);
    assert!(
        buttons.iter().any(|n| n.name == "Close tab"),
        "the active tab shows its close button"
    );
    assert!(buttons.iter().any(|n| n.name == "New tab"));
    for n in tabs.into_iter().chain(buttons) {
        assert!(!n.states.contains(State::Focusable), "{} focusable", n.name);
        assert_eq!(n.actions, Default::default(), "{} has actions", n.name);
    }
    // Every node the strip builds, by walking the tree: a list of roles is how the group was missed.
    assert_eq!(nodes(&tree, Role::TabList).len(), 1);
    for n in &tree.nodes {
        assert!(
            n.states.contains(State::Disabled),
            "{:?} {} disabled",
            n.role,
            n.name
        );
    }
    assert!(
        tree.nodes
            .iter()
            .find(|n| n.role == Role::Tab && n.name == "t1")
            .is_some_and(|n| n.states.contains(State::Selected)),
        "the selection is still told"
    );
}

#[test]
fn a_disabled_strip_pointer_input_reports_nothing() {
    let mut app = App::of(3, Some(0));
    app.enabled = false;
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    let at = mid(&tree, "t2");
    h.input.pointer_to(at.0, at.1);
    run(&mut h, &mut app);
    h.input.click(PRIMARY);
    assert!(run(&mut h, &mut app).events.is_empty());
    h.input.click(PointerButton::Secondary);
    let r = run(&mut h, &mut app);
    assert_eq!(r.context, None);
    no_menu(&r);
    no_menu(&run(&mut h, &mut app));
}

/// A press on a tab activates it; the strip is disabled while the button is held; the release after
/// it does not activate the tab again (the capture outlives the frame in which the response lies).
#[test]
fn a_press_held_across_a_disabled_frame_then_released_activates_once() {
    let mut app = App::of(3, Some(0));
    let mut h = bench(&mut app);
    let tree = run(&mut h, &mut app).out.tree;
    press_at(&mut h, mid(&tree, "t2"));
    let r = run(&mut h, &mut app);
    assert_eq!(
        activations(&r.events),
        1,
        "instrument: the press activates it"
    );
    app.enabled = false;
    assert!(run(&mut h, &mut app).events.is_empty(), "held, disabled");
    app.enabled = true;
    h.input.release(PRIMARY);
    let r = run(&mut h, &mut app);
    assert_eq!(
        activations(&r.events),
        0,
        "the release is the end of the press, not a click: {:?}",
        r.events
    );
}

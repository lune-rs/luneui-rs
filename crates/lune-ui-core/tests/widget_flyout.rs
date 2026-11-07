//! The tray flyout (`overlay.flyout` acceptance): its size equals the drawn card, dismissal,
//! picking, the menu's keys, the pixel probes at scale 2, the menu's motion under every tier, the
//! semantic nodes and the disabled rule.

use lune_ui_core::access::{Action, Live, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{FrameOutput, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::flyout::{Flyout, FlyoutResponse, flyout_size};
use lune_ui_core::widgets::menu::{CLOSE, MenuItem, MenuRow, OPEN};
use lune_ui_core::widgets::popover::{PlaceAlign, Side};
use lune_ui_core::widgets::stat::StatOptions;
use lune_ui_core::widgets::tone::Tone;

const WIN: (f32, f32) = (800.0, 600.0);
/// The tray icon's stand-in, at the bottom right.
const ANCHOR: Rect = Rect::new(760.0, 560.0, 24.0, 24.0);
const P: PointerButton = PointerButton::Primary;

const STATS: [StatOptions<'static>; 3] = [
    StatOptions {
        label: "Today",
        value: "RM 48.20",
        number: None,
        live: false,
        sub: None,
        sub_tone: Tone::Neutral,
        gold: false,
        compact: false,
        text: false,
        meter: None,
        no_data: false,
    },
    StatOptions {
        label: "Saved",
        value: "14.6%",
        number: None,
        live: false,
        sub: None,
        sub_tone: Tone::Neutral,
        gold: true,
        compact: false,
        text: false,
        meter: None,
        no_data: false,
    },
    StatOptions {
        label: "Sessions",
        value: "3 live",
        number: None,
        live: false,
        sub: None,
        sub_tone: Tone::Neutral,
        gold: false,
        compact: false,
        text: false,
        meter: None,
        no_data: false,
    },
];

const ROUTES: [(&str, &str); 3] = [
    ("Opus", "corp-claude-opus"),
    ("Sonnet", "corp-gpt-5"),
    ("Haiku", "qwen3-coder-30b-a3b"),
];

/// Row 1 is disabled; the picks are 0, 2, 3.
const ROWS: [MenuRow<'static>; 4] = [
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::ADD),
        text: "Open app",
        shortcut: "",
        disabled: false,
        danger: false,
        chevron: false,
    }),
    MenuRow::Entry(MenuItem {
        glyph: None,
        text: "Pause compression",
        shortcut: "",
        disabled: true,
        danger: false,
        chevron: false,
    }),
    MenuRow::Entry(MenuItem {
        glyph: None,
        text: "Profile",
        shortcut: "",
        disabled: false,
        danger: false,
        chevron: true,
    }),
    MenuRow::Entry(MenuItem {
        glyph: Some(icon::CLOSE),
        text: "Quit app",
        shortcut: "",
        disabled: false,
        danger: true,
        chevron: false,
    }),
];

fn flyout<'a>(state: &'a str, tone: Tone) -> Flyout<'a> {
    Flyout {
        anchor: ANCHOR,
        anchor_id: None,
        side: Side::Top,
        align: PlaceAlign::End,
        mark: None,
        wordmark: "Atlas",
        state,
        tone,
        stats: &STATS,
        routes: &ROUTES,
        rows: &ROWS,
        name: "App tray",
    }
}

fn serving() -> Flyout<'static> {
    flyout("Serving", Tone::Ok)
}

fn win() -> Rect {
    Rect::new(0.0, 0.0, WIN.0, WIN.1)
}

fn size_of(f: &Flyout<'_>) -> (f32, f32) {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    flyout_size(&mut p, f)
}

fn root() -> WidgetId {
    WidgetId::ROOT.with("fly")
}

fn rows_id() -> WidgetId {
    root().with("rows")
}

fn draw(
    h: &mut UiHarness,
    ms: u64,
    f: Option<&Flyout<'_>>,
) -> (Scene, FlyoutResponse, FrameOutput) {
    let mut resp = FlyoutResponse::default();
    let (scene, out) = h.at_ms(ms).frame(|ui| {
        if let Some(f) = f {
            resp = ui.flyout("fly", f);
        }
    });
    (scene, resp, out)
}

/// Opens, and settles past the 180 ms open.
fn opened(f: &Flyout<'_>) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    draw(&mut h, 0, Some(f));
    draw(&mut h, 300, Some(f));
    h
}

/// The card: the first shape on the popup layer.
fn card(scene: &Scene) -> [f32; 4] {
    scene.merged(LayerId::Popup).geometry.under[0].rect
}

fn key(h: &mut UiHarness, k: NamedKey) {
    h.input.key_down(Key::Named(k), None);
}

/// The centre of the `n`th menu item node, in window coordinates.
fn row_centre(out: &FrameOutput, n: usize) -> (f32, f32) {
    let r = out
        .tree
        .nodes
        .iter()
        .filter(|node| node.role == Role::MenuItem)
        .nth(n)
        .expect("row node")
        .bounds;
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

#[test]
fn the_size_is_the_drawn_card_at_the_three_states() {
    for (state, tone) in [
        ("Serving", Tone::Ok),
        ("degraded", Tone::Warn),
        ("paused", Tone::Paused),
    ] {
        let f = flyout(state, tone);
        let (w, h) = size_of(&f);
        let mut ui = opened(&f);
        let (scene, _, _) = draw(&mut ui, 600, Some(&f));
        let c = card(&scene);
        assert_eq!((c[2], c[3]), (w, h), "{state}");
        // The card sits above its anchor, its right edge on the anchor's, 12 px between.
        assert_eq!(c[1] + c[3] + 12.0, ANCHOR.y, "{state}");
        assert_eq!(c[0] + c[2], ANCHOR.right(), "{state}");
    }
}

#[test]
fn empty_stats_and_routes_take_no_height_and_neighbours_share_a_margin() {
    // 6 + 6 of padding, a 32 px header; the strip is 6 + 16 + 20 + 6 = 48 in a 1 px
    // outline = 50; three routes are 72;
    // the separator 11 and the rows 28 each.
    let rows = 11.0 + 4.0 * 28.0;
    let full = serving();
    let none = Flyout {
        stats: &[],
        routes: &[],
        ..full
    };
    let stats = Flyout {
        routes: &[],
        ..full
    };
    let routes = Flyout { stats: &[], ..full };
    let bare = Flyout { rows: &[], ..none };
    assert_eq!(size_of(&bare), (340.0, 12.0 + 32.0));
    assert_eq!(size_of(&none), (340.0, 12.0 + 32.0 + rows));
    assert_eq!(size_of(&stats).1, 12.0 + 32.0 + 8.0 + 50.0 + 8.0 + rows);
    assert_eq!(size_of(&routes).1, 12.0 + 32.0 + 8.0 + 72.0 + 8.0 + rows);
    assert_eq!(
        size_of(&full).1,
        12.0 + 32.0 + 8.0 + 50.0 + 8.0 + 72.0 + 8.0 + rows
    );
    // Drawn, the card is the size, and the empty parts leave no node.
    let mut h = opened(&none);
    let (scene, _, out) = draw(&mut h, 600, Some(&none));
    let c = card(&scene);
    assert_eq!((c[2], c[3]), size_of(&none));
    assert!(out.tree.get(root().with("stats-group")).is_none());
    assert!(out.tree.get(root().with("routes-group")).is_none());
    assert!(out.tree.get(rows_id()).is_some());
    let mut h = opened(&bare);
    let (_, _, out) = draw(&mut h, 600, Some(&bare));
    assert!(out.tree.get(rows_id()).is_none(), "no rows, no menu node");
}

#[test]
fn escape_an_outside_press_and_focus_loss_dismiss() {
    let f = serving();
    let mut h = opened(&f);
    key(&mut h, NamedKey::Escape);
    assert!(draw(&mut h, 310, Some(&f)).1.dismissed);

    let mut h = opened(&f);
    h.input.pointer_to(50.0, 50.0).click(P);
    assert!(draw(&mut h, 310, Some(&f)).1.dismissed);

    let mut h = opened(&f);
    h.input.focus(false);
    assert!(draw(&mut h, 310, Some(&f)).1.dismissed);

    // A press on the card's header, a margin or a stat is not outside.
    let mut h = opened(&f);
    let c = card(&draw(&mut h, 320, Some(&f)).0);
    for (dx, dy) in [(60.0, 22.0), (10.0, 80.0), (30.0, 60.0)] {
        h.input.pointer_to(c[0] + dx, c[1] + dy).click(P);
        let (_, r, _) = draw(&mut h, 330, Some(&f));
        assert!(!r.dismissed && r.picked.is_none(), "({dx}, {dy})");
    }
}

#[test]
fn a_click_picks_once_and_a_disabled_row_picks_nothing() {
    let f = serving();
    let mut h = opened(&f);
    let (_, _, out) = draw(&mut h, 310, Some(&f));
    let (x, y) = row_centre(&out, 2);
    h.input.pointer_to(x, y);
    draw(&mut h, 320, Some(&f));
    h.input.click(P);
    let (_, r, _) = draw(&mut h, 330, Some(&f));
    assert_eq!(r.picked, Some(2));
    assert_eq!(draw(&mut h, 340, Some(&f)).1.picked, None, "once");
    let (x, y) = row_centre(&out, 1);
    h.input.pointer_to(x, y).click(P);
    let (_, r, _) = draw(&mut h, 350, Some(&f));
    assert!(r.picked.is_none() && !r.dismissed);
}

#[test]
fn focus_lands_on_the_first_row_and_enter_and_space_pick_once() {
    let f = serving();
    for k in [NamedKey::Enter, NamedKey::Space] {
        let mut h = opened(&f);
        let (_, _, out) = draw(&mut h, 310, Some(&f));
        assert_eq!(out.focused, Some(rows_id().with(0_usize)), "{k:?}");
        key(&mut h, k);
        assert_eq!(draw(&mut h, 320, Some(&f)).1.picked, Some(0), "{k:?}");
        assert_eq!(draw(&mut h, 330, Some(&f)).1.picked, None, "{k:?}: once");
    }
}

#[test]
fn arrows_wrap_and_skip_a_disabled_row() {
    let f = serving();
    let mut h = opened(&f);
    draw(&mut h, 310, Some(&f));
    // Opened with the pointer, nothing is lit: the first Down lights the first enabled row, the
    // next skips the disabled 1, and the last wraps.
    for (ms, k, row) in [
        (320, NamedKey::ArrowDown, 0),
        (330, NamedKey::ArrowDown, 2),
        (340, NamedKey::ArrowDown, 3),
        (350, NamedKey::ArrowDown, 0),
        (360, NamedKey::ArrowUp, 3),
        (370, NamedKey::Home, 0),
        (380, NamedKey::End, 3),
    ] {
        key(&mut h, k);
        draw(&mut h, ms, Some(&f));
        key(&mut h, NamedKey::Enter);
        let (_, r, _) = draw(&mut h, ms + 5, Some(&f));
        assert_eq!(r.picked, Some(row), "{k:?}");
    }
}

#[test]
fn pixel_probes_at_scale_2() {
    let f = Flyout {
        mark: None,
        ..serving()
    };
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    draw(&mut h, 0, Some(&f));
    let (scene, _, _) = draw(&mut h, 400, Some(&f));
    let fill = scene.merged(LayerId::Popup).geometry.under[0];
    assert_eq!(fill.rect[2], 680.0, "340 logical px wide");
    assert_eq!(fill.radii, [18.0; 4], "radius 9");
    assert_eq!(fill.color, premul(DARK.menu_glass));
    // The header: 6 px of padding inside the 6 px inset, 8 px at the sides.
    let style = Style {
        line: 20.0,
        ..Style::ui(DARK.font_m, DARK.gold).bold()
    };
    let key = FakeText.prepare(&Spec::plain("Atlas", style), 2.0).0;
    let layer = scene.merged(LayerId::Popup);
    let name = layer
        .texts
        .iter()
        .find(|t| t.key == key)
        .expect("the wordmark");
    let (inset, pad) = (12.0, (12.0, 16.0));
    // (the inset is the 5 px padding and the 1 px ring, doubled)
    // Text positions are logical.
    assert_eq!(
        name.x * 2.0 - fill.rect[0],
        inset + pad.1,
        "16 device px at the side"
    );
    assert_eq!(
        name.y * 2.0 - fill.rect[1],
        inset + pad.0,
        "12 device px above"
    );
    // The backdrop sits under the popup layer, over the card.
    let b = scene.merged(LayerId::Popup).backdrops[0];
    assert_eq!(b.rect, fill.rect);
    assert_eq!(b.radius, 18.0);
}

#[test]
fn no_text_is_closer_than_9_px_to_the_card() {
    let f = Flyout {
        mark: Some(icon::INFO),
        ..serving()
    };
    let mut h = opened(&f);
    let (scene, ..) = draw(&mut h, 600, Some(&f));
    let c = card(&scene);
    for t in &scene.merged(LayerId::Popup).texts {
        assert!(t.x >= c[0] + 9.0, "{t:?} against {c:?}");
    }
    // The state ends 14 px from the right edge: 6 inset and 8 padding.
    let style = Style {
        line: 16.0,
        ..Style::ui(DARK.font_s, DARK.fg_secondary)
    };
    let spec = Spec::plain("Serving", style);
    let (key, (w, _)) = FakeText.prepare(&spec, 1.0);
    let layer = scene.merged(LayerId::Popup);
    let state = layer
        .texts
        .iter()
        .find(|t| t.key == key)
        .expect("the state");
    assert_eq!(state.x + w, c[0] + c[2] - 14.0);
}

/// Opens, settles, stops calling at 1000 ms and steps 10 ms frames: ms until idle and the closing
/// frames' fill alphas.
fn close_run(tier: MotionTier) -> (u64, Vec<f32>) {
    let f = serving();
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    draw(&mut h, 0, Some(&f));
    draw(&mut h, 300, Some(&f));
    let mut alphas = Vec::new();
    let mut t = 1000;
    loop {
        let (scene, _, out) = draw(&mut h, t, None);
        if let Some(s) = scene.merged(LayerId::Popup).geometry.under.first() {
            alphas.push(s.color[3]);
        }
        if out.repaint == Repaint::Idle {
            return (t - 1000, alphas);
        }
        t += 10;
        assert!(t < 2000, "never settled");
    }
}

#[test]
fn the_curves_are_the_menus_under_every_tier() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, a, b) in [(OPEN, 0.0_f32, 1.0), (CLOSE, 1.0, 0.0)] {
            conformance(&row, &DARK.motion, tier, a, b)
                .unwrap_or_else(|e| panic!("{tier:?} {row:?}: {e}"));
        }
    }
    for (tier, ms) in [
        (MotionTier::Full, 120),
        (MotionTier::Reduced, 120),
        (MotionTier::Off, 0),
    ] {
        let (idle_after, alphas) = close_run(tier);
        assert_eq!(idle_after, if ms == 0 { 10 } else { ms }, "{tier:?}");
        assert_eq!(alphas.len() as u64, ms / 10, "{tier:?}");
        assert!(alphas.windows(2).all(|w| w[1] <= w[0]), "{tier:?}");
    }
    // Opening: opacity only grows, and the card is settled by 180 ms.
    let f = serving();
    let mut h = UiHarness::new(WIN);
    draw(&mut h, 0, Some(&f));
    let rest = {
        let mut r = UiHarness::new(WIN);
        draw(&mut r, 0, Some(&f));
        let (scene, ..) = draw(&mut r, 400, Some(&f));
        scene.merged(LayerId::Popup).geometry.under[0].color[3]
    };
    let mut last = 0.0;
    for k in 1..=15_u64 {
        let dt = (k * 1000 + 30) / 60;
        let (scene, _, out) = draw(&mut h, dt, Some(&f));
        let a = scene.merged(LayerId::Popup).geometry.under[0].color[3];
        assert!(a >= last && a.is_finite(), "{dt}: {a} after {last}");
        last = a;
        if dt >= 180 + 17 {
            assert_eq!(a, rest, "settled at {dt} ms");
            assert_eq!(out.repaint, Repaint::Idle);
        }
    }
}

#[test]
fn a_closing_flyout_replays_its_blur_with_the_fade() {
    let f = serving();
    let mut h = opened(&f);
    let (scene, ..) = draw(&mut h, 600, Some(&f));
    let open = scene.backdrops().next().copied().expect("blur while open");
    let (scene, _, out) = draw(&mut h, 1000, None);
    let closing = scene
        .backdrops()
        .next()
        .copied()
        .expect("blur while closing");
    assert_eq!(closing.rect[2], open.rect[2]);
    assert!(closing.alpha <= open.alpha);
    assert_eq!(out.repaint, Repaint::Now);
    let (scene, ..) = draw(&mut h, 1200, None);
    assert!(
        scene.backdrops().next().is_none(),
        "gone once the close is over"
    );
}

#[test]
fn the_state_node_is_polite_and_follows_the_state_text() {
    let mut seen = Vec::new();
    for (state, tone) in [
        ("Serving", Tone::Ok),
        ("degraded", Tone::Warn),
        ("paused", Tone::Paused),
    ] {
        let f = flyout(state, tone);
        let mut h = opened(&f);
        let (_, _, out) = draw(&mut h, 600, Some(&f));
        let node = out.tree.get(root().with("state")).expect("state node");
        assert_eq!(node.role, Role::Status);
        assert_eq!(node.live, Some(Live::Polite));
        assert_eq!(node.name, state);
        assert!(node.actions == Default::default());
        seen.push(node.name.clone());
    }
    assert_eq!(seen, ["Serving", "degraded", "paused"]);
    // The same flyout, the state changing under it.
    let mut h = opened(&serving());
    let paused = flyout("paused", Tone::Paused);
    let (_, _, out) = draw(&mut h, 600, Some(&paused));
    assert_eq!(
        out.tree.get(root().with("state")).expect("node").name,
        "paused"
    );
}

#[test]
fn the_nodes_are_a_menu_a_status_and_two_groups() {
    let f = serving();
    let mut h = opened(&f);
    let (_, _, out) = draw(&mut h, 600, Some(&f));
    let menu = out.tree.get(rows_id()).expect("menu node");
    assert_eq!(menu.role, Role::Menu);
    assert_eq!(menu.name, "App tray");
    assert_eq!(menu.children.len(), 4, "a node per row");
    for id in ["stats-group", "routes-group"] {
        let g = out.tree.get(root().with(id)).expect(id);
        assert_eq!(g.role, Role::Group);
        assert!(g.actions == Default::default(), "{id}: no actions");
    }
    assert_eq!(
        out.tree.get(root().with("stats-group")).expect("g").name,
        "Today, Saved, Sessions"
    );
    // No name given: the toolkit's.
    let unnamed = Flyout { name: "", ..f };
    let mut h = opened(&unnamed);
    let (_, _, out) = draw(&mut h, 600, Some(&unnamed));
    assert_eq!(out.tree.get(rows_id()).expect("menu").name, "Menu");
    // The disabled row is a disabled node.
    let row = out.tree.get(rows_id().with(1_usize)).expect("row 1");
    assert!(row.states.contains(State::Disabled));
    assert!(!row.actions.contains(Action::Default));
}

#[test]
fn a_disabled_flyout_closes_on_escape_picks_nothing_and_marks_every_node() {
    let keys = [
        NamedKey::Enter,
        NamedKey::Space,
        NamedKey::ArrowDown,
        NamedKey::ArrowUp,
        NamedKey::Home,
        NamedKey::End,
    ];
    let f = serving();
    let mut h = opened(&f);
    draw(&mut h, 310, Some(&f));
    for k in keys {
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
    }
    let frame = |h: &mut UiHarness, ms: u64| {
        let mut resp = FlyoutResponse::default();
        let (_, out) = h.at_ms(ms).frame(|ui| {
            ui.with_enabled(false, |ui| resp = ui.flyout("fly", &f));
        });
        (out, resp)
    };
    frame(&mut h, 320);
    for (i, k) in keys.iter().enumerate() {
        key(&mut h, *k);
        let (out, resp) = frame(&mut h, 330 + 10 * i as u64);
        assert!(out.fired.is_empty(), "{k:?}");
        assert_eq!(resp.picked, None, "{k:?}");
    }
    let (out, _) = frame(&mut h, 400);
    for id in [
        rows_id(),
        root().with("state"),
        root().with("stats-group"),
        root().with("routes-group"),
    ] {
        assert!(out.tree.get(id).is_some(), "{id:?}");
        assert_eq!(disabled_violations(&out.tree, id), [], "{id:?}");
    }
    key(&mut h, NamedKey::Escape);
    assert!(frame(&mut h, 410).1.dismissed, "Escape still closes it");
}

/// The routes' keys are set at `font_s` (the app design's key column), and the stats strip has its outline:
/// a 1 px ring at radius 8 on the strip's 50 px rect, with the cells 1 px inside it.
#[test]
fn the_routes_keys_are_small_and_the_strip_has_its_outline() {
    let f = serving();
    let mut h = opened(&f);
    let (scene, _, out) = draw(&mut h, 600, Some(&f));
    let key_style = Style {
        line: 20.0,
        ..Style::ui(DARK.font_s, DARK.fg_tertiary)
    };
    let layer = &scene.merged(LayerId::Popup);
    let want = lune_ui_core::text::TextKey(Spec::plain("Sonnet", key_style).cache_key(1.0));
    assert!(layer.texts.iter().any(|t| t.key == want), "key at font_s");
    let strip = out
        .tree
        .get(root().with("stats-group"))
        .expect("stats")
        .bounds;
    assert_eq!(strip.h, 50.0);
    let ring = layer
        .geometry
        .under
        .iter()
        .find(|s| s.params[0] == 1.0 && s.radii == [8.0; 4])
        .expect("the strip's ring");
    assert_eq!(ring.rect, [strip.x, strip.y, strip.w, strip.h]);
    assert_eq!(ring.color, premul(DARK.border_subtle));
}

// The flyout in a window of its own (`Ui::flyout_window`, D17): the window is the card's size and
// the card fills it. The scripts are the anchored flyout's, run on a harness the card's size.

fn win_draw(
    h: &mut UiHarness,
    ms: u64,
    f: Option<&Flyout<'_>>,
) -> (Scene, FlyoutResponse, FrameOutput) {
    let mut resp = FlyoutResponse::default();
    let (scene, out) = h.at_ms(ms).frame(|ui| {
        if let Some(f) = f {
            resp = ui.flyout_window("fly", f);
        }
    });
    (scene, resp, out)
}

/// A harness the card's size, opened and settled past the 180 ms open.
fn win_opened(f: &Flyout<'_>) -> UiHarness {
    let mut h = UiHarness::new(size_of(f));
    win_draw(&mut h, 0, Some(f));
    win_draw(&mut h, 300, Some(f));
    h
}

#[test]
fn a_window_flyout_card_fills_the_window_at_the_three_states() {
    for (state, tone) in [
        ("Serving", Tone::Ok),
        ("degraded", Tone::Warn),
        ("paused", Tone::Paused),
    ] {
        let f = flyout(state, tone);
        let (w, h) = size_of(&f);
        let mut ui = win_opened(&f);
        let (scene, ..) = win_draw(&mut ui, 600, Some(&f));
        // The card is the window: its size, at the origin, whatever the anchor says.
        assert_eq!(card(&scene), [0.0, 0.0, w, h], "{state}");
        let b = scene
            .backdrops()
            .next()
            .copied()
            .expect("a glass card asks for the blur");
        assert_eq!(b.rect, [0.0, 0.0, w, h], "{state}");
    }
}

#[test]
fn a_window_flyout_dismisses_on_escape_and_focus_loss_and_not_on_its_own_card() {
    let f = serving();
    let mut h = win_opened(&f);
    key(&mut h, NamedKey::Escape);
    assert!(win_draw(&mut h, 310, Some(&f)).1.dismissed);

    let mut h = win_opened(&f);
    h.input.focus(false);
    assert!(win_draw(&mut h, 310, Some(&f)).1.dismissed);

    // A press on the card's header, a margin or a stat is not outside.
    let mut h = win_opened(&f);
    for (dx, dy) in [(60.0, 22.0), (10.0, 80.0), (30.0, 60.0)] {
        h.input.pointer_to(dx, dy).click(P);
        let (_, r, _) = win_draw(&mut h, 330, Some(&f));
        assert!(!r.dismissed && r.picked.is_none(), "({dx}, {dy})");
    }
}

#[test]
fn a_focus_loss_before_the_window_flyout_first_opened_does_nothing() {
    let f = serving();
    let mut h = UiHarness::new(size_of(&f));
    h.input.focus(false);
    let (_, r, _) = win_draw(&mut h, 0, None);
    assert!(!r.dismissed);
    let (_, r, _) = win_draw(&mut h, 10, Some(&f));
    assert!(!r.dismissed && r.picked.is_none());
    let (_, r, _) = win_draw(&mut h, 400, Some(&f));
    assert!(!r.dismissed);
}

#[test]
fn a_window_flyout_click_picks_once_and_a_disabled_row_picks_nothing() {
    let f = serving();
    let mut h = win_opened(&f);
    let (_, _, out) = win_draw(&mut h, 310, Some(&f));
    let (x, y) = row_centre(&out, 2);
    h.input.pointer_to(x, y);
    win_draw(&mut h, 320, Some(&f));
    h.input.click(P);
    let (_, r, _) = win_draw(&mut h, 330, Some(&f));
    assert_eq!(r.picked, Some(2));
    assert_eq!(win_draw(&mut h, 340, Some(&f)).1.picked, None, "once");
    let (x, y) = row_centre(&out, 1);
    h.input.pointer_to(x, y).click(P);
    let (_, r, _) = win_draw(&mut h, 350, Some(&f));
    assert!(r.picked.is_none() && !r.dismissed);
}

#[test]
fn a_window_flyout_takes_the_menus_keys() {
    let f = serving();
    for k in [NamedKey::Enter, NamedKey::Space] {
        let mut h = win_opened(&f);
        let (_, _, out) = win_draw(&mut h, 310, Some(&f));
        assert_eq!(out.focused, Some(rows_id().with(0_usize)), "{k:?}");
        key(&mut h, k);
        assert_eq!(win_draw(&mut h, 320, Some(&f)).1.picked, Some(0), "{k:?}");
        assert_eq!(
            win_draw(&mut h, 330, Some(&f)).1.picked,
            None,
            "{k:?}: once"
        );
    }
    let mut h = win_opened(&f);
    win_draw(&mut h, 310, Some(&f));
    for (ms, k, row) in [
        (320, NamedKey::ArrowDown, 0),
        (330, NamedKey::ArrowDown, 2),
        (340, NamedKey::ArrowDown, 3),
        (350, NamedKey::ArrowDown, 0),
        (360, NamedKey::ArrowUp, 3),
        (370, NamedKey::Home, 0),
        (380, NamedKey::End, 3),
    ] {
        key(&mut h, k);
        win_draw(&mut h, ms, Some(&f));
        key(&mut h, NamedKey::Enter);
        let (_, r, _) = win_draw(&mut h, ms + 5, Some(&f));
        assert_eq!(r.picked, Some(row), "{k:?}");
    }
}

/// Opens, settles, stops calling at 1000 ms and steps 10 ms frames: ms until idle and the closing
/// frames' fill alphas, in the window.
fn win_close_run(tier: MotionTier) -> (u64, Vec<f32>, bool) {
    let f = serving();
    let mut h = UiHarness::new(size_of(&f));
    h.tier = tier;
    win_draw(&mut h, 0, Some(&f));
    win_draw(&mut h, 300, Some(&f));
    let (mut alphas, mut blurred) = (Vec::new(), true);
    let mut t = 1000;
    loop {
        let (scene, _, out) = win_draw(&mut h, t, None);
        if let Some(s) = scene.merged(LayerId::Popup).geometry.under.first() {
            alphas.push(s.color[3]);
            blurred &= scene.backdrops().next().is_some();
        }
        if out.repaint == Repaint::Idle {
            return (t - 1000, alphas, blurred);
        }
        t += 10;
        assert!(t < 2000, "never settled");
    }
}

#[test]
fn a_window_flyout_opens_and_closes_with_the_menus_curves_and_goes_idle() {
    for (tier, ms) in [
        (MotionTier::Full, 120),
        (MotionTier::Reduced, 120),
        (MotionTier::Off, 0),
    ] {
        let (idle_after, alphas, blurred) = win_close_run(tier);
        assert_eq!(idle_after, if ms == 0 { 10 } else { ms }, "{tier:?}");
        assert_eq!(alphas.len() as u64, ms / 10, "{tier:?}");
        assert!(alphas.windows(2).all(|w| w[1] <= w[0]), "{tier:?}");
        assert!(blurred, "{tier:?}: the exit replays the blur");
    }
    let f = serving();
    let mut h = UiHarness::new(size_of(&f));
    win_draw(&mut h, 0, Some(&f));
    let rest = {
        let mut r = UiHarness::new(size_of(&f));
        win_draw(&mut r, 0, Some(&f));
        let (scene, ..) = win_draw(&mut r, 400, Some(&f));
        scene.merged(LayerId::Popup).geometry.under[0].color[3]
    };
    let mut last = 0.0;
    for k in 1..=15_u64 {
        let dt = (k * 1000 + 30) / 60;
        let (scene, _, out) = win_draw(&mut h, dt, Some(&f));
        let a = scene.merged(LayerId::Popup).geometry.under[0].color[3];
        assert!(a >= last && a.is_finite(), "{dt}: {a} after {last}");
        last = a;
        if dt >= 180 + 17 {
            assert_eq!(a, rest, "settled at {dt} ms");
            assert_eq!(out.repaint, Repaint::Idle);
        }
    }
    // Stopped calling, it fades out; once the close is over the blur is gone.
    let (scene, ..) = win_draw(&mut h, 1200, None);
    assert!(scene.backdrops().next().is_some());
    let (scene, ..) = win_draw(&mut h, 1400, None);
    assert!(scene.backdrops().next().is_none());
}

#[test]
fn a_window_flyouts_nodes_are_the_anchored_ones_and_a_disabled_one_picks_nothing() {
    let f = serving();
    let mut h = win_opened(&f);
    let (_, _, out) = win_draw(&mut h, 600, Some(&f));
    let menu = out.tree.get(rows_id()).expect("menu node");
    assert_eq!((menu.role, menu.name.as_str()), (Role::Menu, "App tray"));
    let state = out.tree.get(root().with("state")).expect("state node");
    assert_eq!((state.role, state.live), (Role::Status, Some(Live::Polite)));
    for id in ["stats-group", "routes-group"] {
        assert_eq!(out.tree.get(root().with(id)).expect(id).role, Role::Group);
    }

    let keys = [
        NamedKey::Enter,
        NamedKey::Space,
        NamedKey::ArrowDown,
        NamedKey::ArrowUp,
        NamedKey::Home,
        NamedKey::End,
    ];
    for k in keys {
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
    }
    let frame = |h: &mut UiHarness, ms: u64| {
        let mut resp = FlyoutResponse::default();
        let (_, out) = h.at_ms(ms).frame(|ui| {
            ui.with_enabled(false, |ui| resp = ui.flyout_window("fly", &f));
        });
        (out, resp)
    };
    frame(&mut h, 620);
    for (i, k) in keys.iter().enumerate() {
        key(&mut h, *k);
        let (out, resp) = frame(&mut h, 630 + 10 * i as u64);
        assert!(out.fired.is_empty(), "{k:?}");
        assert_eq!(resp.picked, None, "{k:?}");
    }
    let (out, _) = frame(&mut h, 700);
    for id in [rows_id(), root().with("state"), root().with("stats-group")] {
        assert_eq!(disabled_violations(&out.tree, id), [], "{id:?}");
    }
    key(&mut h, NamedKey::Escape);
    assert!(frame(&mut h, 710).1.dismissed, "Escape still closes it");
}

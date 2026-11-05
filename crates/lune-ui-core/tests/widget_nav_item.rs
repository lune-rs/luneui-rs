//! The navigation list (`widget.nav-item` acceptance): one roving stop, Up and Down without wrap,
//! Enter, Space and click navigate once, focus alone navigates nothing, the active fill sampled at
//! 60 Hz, probes at scale 2 and the nodes.

use lune_ui_core::access::{AccessAction, Action, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::nav_item::{
    FOCUS, HOVER, NavEntry, NavLook, NavResponse, NavState, SELECTION,
};

const WIN: (f32, f32) = (400.0, 300.0);
/// The list at (20, 40), 200 wide: rows 32 high at y = 40, 74, 108 (2 apart).
const LIST: Rect = Rect::new(20.0, 40.0, 200.0, 0.0);
const KEYS: [u64; 3] = [1, 2, 3];

fn row_centre(i: usize) -> (f32, f32) {
    (120.0, 40.0 + 34.0 * i as f32 + 16.0)
}

fn kid(key: u64) -> WidgetId {
    WidgetId::ROOT.with("nav").with(key)
}

#[derive(Clone)]
struct Case {
    selected: u64,
    enabled: [bool; 3],
    ask: Option<WidgetId>,
}

fn case(selected: u64) -> Case {
    Case {
        selected,
        enabled: [true; 3],
        ask: None,
    }
}

struct Seen {
    scene: Scene,
    nav: NavResponse,
    focused: Option<WidgetId>,
    repaint: Repaint,
    tree: lune_ui_core::access::SemanticTree,
}

fn frame(h: &mut UiHarness, c: &mut Case) -> Seen {
    let mut nav = None;
    let (scene, out) = h.frame(|ui| {
        ui.button(
            "before",
            Rect::new(20.0, 4.0, 60.0, 24.0),
            "Before",
            ButtonKind::Secondary,
        );
        let entries: Vec<NavEntry<'_>> = KEYS
            .iter()
            .zip(["Overview", "Sessions", "Settings"])
            .zip(c.enabled)
            .map(|((key, label), enabled)| NavEntry {
                key: *key,
                label,
                icon: Some(lune_ui_core::icon::GEAR),
                enabled,
            })
            .collect();
        nav = Some(ui.nav_list("nav", LIST, &entries, &mut c.selected, "Screens"));
        ui.button(
            "after",
            Rect::new(20.0, 200.0, 60.0, 24.0),
            "After",
            ButtonKind::Secondary,
        );
        if let Some(id) = c.ask {
            ui.request_focus(id);
        }
    });
    Seen {
        scene,
        nav: nav.expect("frame ran"),
        focused: out.focused,
        repaint: out.repaint,
        tree: out.tree,
    }
}

fn press(h: &mut UiHarness, key: NamedKey) {
    h.input.key_down(Key::Named(key), None);
}

/// Focus on the list's stop (the active row), by two Tabs.
fn enter(h: &mut UiHarness, c: &mut Case) {
    frame(h, c);
    press(h, NamedKey::Tab);
    frame(h, c);
    press(h, NamedKey::Tab);
    frame(h, c);
}

#[test]
fn tab_enters_once_on_the_active_row_and_leaves_once() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(2);
    frame(&mut h, &mut c);
    let mut stops = Vec::new();
    for _ in 0..4 {
        press(&mut h, NamedKey::Tab);
        stops.push(frame(&mut h, &mut c).focused);
    }
    assert_eq!(
        stops,
        [
            Some(WidgetId::ROOT.with("before")),
            Some(kid(2)),
            Some(WidgetId::ROOT.with("after")),
            Some(WidgetId::ROOT.with("before")),
        ]
    );
}

#[test]
fn down_and_up_move_focus_without_navigating_and_do_not_wrap() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(1);
    enter(&mut h, &mut c);
    let mut seen_focus = Vec::new();
    for key in [
        NamedKey::ArrowDown,
        NamedKey::ArrowDown,
        NamedKey::ArrowDown,
        NamedKey::ArrowUp,
        NamedKey::ArrowUp,
        NamedKey::ArrowUp,
    ] {
        press(&mut h, key);
        frame(&mut h, &mut c);
        let seen = frame(&mut h, &mut c);
        assert!(seen.focused.is_some());
        assert!(seen.nav.activated.is_none() && !seen.nav.response.changed);
        seen_focus.push(seen.focused);
    }
    assert_eq!(
        seen_focus,
        [
            Some(kid(2)),
            Some(kid(3)),
            Some(kid(3)),
            Some(kid(2)),
            Some(kid(1)),
            Some(kid(1)),
        ],
        "stops at both ends"
    );
    assert_eq!(c.selected, 1, "focus alone navigates nothing");
}

#[test]
fn a_disabled_entry_is_skipped() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(1);
    c.enabled = [true, false, true];
    enter(&mut h, &mut c);
    press(&mut h, NamedKey::ArrowDown);
    frame(&mut h, &mut c);
    assert_eq!(frame(&mut h, &mut c).focused, Some(kid(3)));
}

#[test]
fn enter_space_and_click_each_navigate_once() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(1);
    enter(&mut h, &mut c);
    press(&mut h, NamedKey::ArrowDown);
    frame(&mut h, &mut c);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Enter);
    let seen = frame(&mut h, &mut c);
    assert_eq!(seen.nav.activated, Some(2));
    assert!(seen.nav.response.changed);
    assert_eq!(c.selected, 2);
    let seen = frame(&mut h, &mut c);
    assert_eq!(seen.nav.activated, None);
    assert!(!seen.nav.response.changed);

    press(&mut h, NamedKey::ArrowDown);
    frame(&mut h, &mut c);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Space);
    let seen = frame(&mut h, &mut c);
    assert_eq!((seen.nav.activated, c.selected), (Some(3), 3));
    assert_eq!(frame(&mut h, &mut c).nav.activated, None);

    let (x, y) = row_centre(0);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    let seen = frame(&mut h, &mut c);
    assert_eq!((seen.nav.activated, c.selected), (Some(1), 1));
    assert!(seen.nav.response.changed);
    assert_eq!(frame(&mut h, &mut c).nav.activated, None);
    // A click on the current screen still navigates, but changes no value.
    h.input.click(PointerButton::Primary);
    let seen = frame(&mut h, &mut c);
    assert_eq!(seen.nav.activated, Some(1));
    assert!(!seen.nav.response.changed);
}

#[test]
fn a_disabled_entry_takes_no_click() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(1);
    c.enabled = [true, false, true];
    frame(&mut h, &mut c);
    let (x, y) = row_centre(1);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    let seen = frame(&mut h, &mut c);
    assert_eq!((seen.nav.activated, c.selected), (None, 1));
}

#[test]
fn the_default_action_navigates() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(1);
    frame(&mut h, &mut c);
    h.input.access(AccessAction {
        target: kid(3),
        action: Action::Default,
        value: None,
    });
    let seen = frame(&mut h, &mut c);
    assert_eq!((seen.nav.activated, c.selected), (Some(3), 3));
}

#[test]
fn the_nodes_are_a_list_of_list_items_and_the_active_one_is_selected_and_current() {
    let mut h = UiHarness::new(WIN);
    for selected in KEYS {
        let mut c = case(selected);
        c.enabled = [true, true, false];
        let seen = frame(&mut h, &mut c);
        let list = seen.tree.get(WidgetId::ROOT.with("nav")).expect("list");
        assert_eq!((list.role, list.name.as_str()), (Role::List, "Screens"));
        assert_eq!(
            list.children,
            KEYS.iter().map(|k| kid(*k)).collect::<Vec<_>>()
        );
        for (i, key) in KEYS.iter().enumerate() {
            let node = seen.tree.get(kid(*key)).expect("item");
            assert_eq!(node.role, Role::ListItem);
            assert_eq!(node.name, ["Overview", "Sessions", "Settings"][i]);
            let on = *key == selected;
            assert_eq!(node.states.contains(State::Selected), on);
            assert_eq!(node.states.contains(State::Current), on);
            assert_eq!(node.states.contains(State::Disabled), i == 2);
            assert_eq!(node.actions.contains(Action::Default), i != 2);
        }
    }
}

#[test]
fn the_settled_looks_are_the_catalogue_states() {
    let t = DARK;
    let look = |active, hovered, pressed| {
        NavLook::of(
            &t,
            NavState {
                active,
                hovered,
                pressed,
                focus_visible: false,
            },
        )
    };
    let rest = look(false, false, false);
    assert_eq!(
        (rest.fill[3], rest.label, rest.medium),
        (0, t.fg_secondary, false)
    );
    let hover = look(false, true, false);
    assert_eq!((hover.fill, hover.label), (t.surface_hover, t.fg_primary));
    let active = look(true, false, false);
    assert_eq!(
        (active.fill, active.label),
        (t.surface_selected, t.fg_bright)
    );
    assert!(active.medium);
    assert_eq!(look(true, true, false).fill, t.surface_active);
    assert_eq!(look(false, false, true).fill, t.surface_active);
    assert_eq!(look(true, false, true).fill, t.surface_active);
}

/// Row 1 as a shape: (20, 74), 200 by 32, radius 6; the alpha of its fill, 0 when not drawn.
fn fill_alpha(scene: &Scene) -> f32 {
    scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|s| s.rect == [20.0, 74.0, 200.0, 32.0] && s.radii == [6.0; 4])
        .map_or(0.0, |s| s.color[3])
}

/// Row 1 becomes the current screen at 500 ms (the app sets it), then its fill alpha at 60 Hz.
fn fill_run(tier: MotionTier) -> (Vec<(u64, f32)>, Repaint, Scene) {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let mut c = case(1);
    frame(&mut h, &mut c);
    h.at_ms(500);
    c.selected = 2;
    let first = frame(&mut h, &mut c);
    let mut xs = vec![(0, fill_alpha(&first.scene))];
    let mut last = first;
    for k in 1..=15_u64 {
        let dt = (k * 1000 + 30) / 60;
        h.at_ms(500 + dt);
        last = frame(&mut h, &mut c);
        xs.push((dt, fill_alpha(&last.scene)));
    }
    let repaint = last.repaint;
    (xs, repaint, last.scene)
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [(SELECTION, "selection"), (HOVER, "hover"), (FOCUS, "focus")] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
    }
}

#[test]
fn the_active_fill_rises_monotonically_ends_exact_and_settles_within_180_ms_and_a_frame() {
    let (xs, repaint, last) = fill_run(MotionTier::Full);
    assert!(xs.windows(2).all(|w| w[1].1 >= w[0].1), "monotonic: {xs:?}");
    assert!(
        xs.iter().any(|&(_, a)| a > 0.0 && a < xs[15].1),
        "moves: {xs:?}"
    );
    let end = premul(DARK.surface_selected)[3];
    for &(dt, a) in &xs {
        if dt >= 180 + 17 {
            assert_eq!(a, end, "settled at {dt} ms");
        }
    }
    assert_eq!(repaint, Repaint::Idle);
    let settled = {
        let mut h = UiHarness::new(WIN);
        let mut c = case(2);
        frame(&mut h, &mut c);
        h.at_ms(1000);
        frame(&mut h, &mut c).scene
    };
    assert_eq!(last, settled);
}

#[test]
fn reduced_keeps_the_fill_change_and_off_snaps_it() {
    let end = premul(DARK.surface_selected)[3];
    let (xs, _, _) = fill_run(MotionTier::Reduced);
    assert!(xs.iter().any(|&(_, a)| a > 0.0 && a < end), "{xs:?}");
    let (xs, _, _) = fill_run(MotionTier::Off);
    assert!(xs[1..].iter().all(|&(_, a)| a == end), "{xs:?}");
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let mut c = case(2);
    frame(&mut h, &mut c);
    h.at_ms(1000);
    let s = frame(&mut h, &mut c).scene;
    let under = &s.layers[0].geometry.under;
    // The active row (row 1: 34 px below row 0, so 68 device px): 64 px high, radius 12 px.
    let row = under
        .iter()
        .find(|sh| sh.rect == [40.0, 148.0, 400.0, 64.0])
        .expect("active row fill");
    assert_eq!(row.radii, [12.0; 4]);
    // The icon is a 16 px glyph at 28; the label follows 8 px after it (20 + 8 + 16 + 8 = 52).
    let texts = &s.layers[0].texts;
    assert!(texts.iter().any(|t| t.x == 52.0), "label at 52: {texts:?}");
}

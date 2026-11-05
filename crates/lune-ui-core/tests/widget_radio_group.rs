//! The radio group (`widget.radio-group` acceptance): one roving stop, the arrows per D120 (wrap
//! both ways, disabled options skipped, selection and focus move together), Space and click,
//! a disabled group, the selection fill sampled at 60 Hz, probes at scale 2 and the nodes.

use lune_ui_core::access::{AccessAction, Action, Role, State};
use lune_ui_core::color::{Rgba, premul};
use lune_ui_core::input::{Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::radio_group::{FOCUS, HOVER, RadioOption, SELECTION};

const WIN: (f32, f32) = (400.0, 300.0);
/// The group at (20, 40), 200 wide: rows 28 high at y = 40, 68, 96.
const GROUP: Rect = Rect::new(20.0, 40.0, 200.0, 0.0);
const KEYS: [u64; 3] = [10, 20, 30];

fn row_centre(i: usize) -> (f32, f32) {
    (120.0, 40.0 + 28.0 * i as f32 + 14.0)
}

fn kid(key: u64) -> WidgetId {
    WidgetId::ROOT.with("g").with(key)
}

#[derive(Clone)]
struct Case {
    selected: u64,
    enabled: [bool; 3],
    group_on: bool,
    ask: Option<WidgetId>,
}

fn case(selected: u64) -> Case {
    Case {
        selected,
        enabled: [true; 3],
        group_on: true,
        ask: None,
    }
}

struct Seen {
    scene: Scene,
    group: Response,
    focused: Option<WidgetId>,
    repaint: Repaint,
    tree: lune_ui_core::access::SemanticTree,
}

fn frame(h: &mut UiHarness, c: &mut Case) -> Seen {
    let mut group = None;
    let (scene, out) = h.frame(|ui| {
        ui.button(
            "before",
            Rect::new(20.0, 4.0, 60.0, 24.0),
            "Before",
            ButtonKind::Secondary,
        );
        let options: Vec<RadioOption<'_>> = KEYS
            .iter()
            .zip(["Alpha", "Beta", "Gamma"])
            .zip(c.enabled)
            .map(|((key, label), enabled)| RadioOption {
                key: *key,
                label,
                enabled,
            })
            .collect();
        let selected = &mut c.selected;
        group = Some(ui.with_enabled(c.group_on, |ui| {
            ui.radio_group("g", GROUP, &options, selected, "Mode")
        }));
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
        group: group.expect("frame ran"),
        focused: out.focused,
        repaint: out.repaint,
        tree: out.tree,
    }
}

fn press(h: &mut UiHarness, key: NamedKey) {
    h.input.key_down(Key::Named(key), None);
}

#[test]
fn tab_enters_once_on_the_selected_option_and_leaves_once() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(20);
    frame(&mut h, &mut c);
    let mut stops = Vec::new();
    for _ in 0..4 {
        press(&mut h, NamedKey::Tab);
        stops.push(frame(&mut h, &mut c).focused);
    }
    let before = WidgetId::ROOT.with("before");
    let after = WidgetId::ROOT.with("after");
    assert_eq!(
        stops,
        [Some(before), Some(kid(20)), Some(after), Some(before),],
        "one stop for the whole group"
    );
}

#[test]
fn tab_from_a_child_the_arrows_reached_leaves_the_group() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(10);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    assert_eq!(frame(&mut h, &mut c).focused, Some(kid(10)));
    press(&mut h, NamedKey::ArrowDown);
    frame(&mut h, &mut c);
    assert_eq!(frame(&mut h, &mut c).focused, Some(kid(20)));
    press(&mut h, NamedKey::Tab);
    assert_eq!(
        frame(&mut h, &mut c).focused,
        Some(WidgetId::ROOT.with("after"))
    );
    // Shift+Tab comes back to the one stop: the selected option.
    h.input.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    press(&mut h, NamedKey::Tab);
    h.input.set_modifiers(Modifiers::default());
    assert_eq!(frame(&mut h, &mut c).focused, Some(kid(20)));
}

/// Enters the group on option `from` and presses `key`; returns the selection, `changed` of that
/// frame, and the focus and ring two frames later.
fn arrow(from: u64, enabled: [bool; 3], key: NamedKey) -> (u64, bool, Option<WidgetId>, bool) {
    let mut h = UiHarness::new(WIN);
    let mut c = case(from);
    c.enabled = enabled;
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    assert_eq!(frame(&mut h, &mut c).focused, Some(kid(from)));
    press(&mut h, key);
    let moved = frame(&mut h, &mut c);
    let (selected, changed) = (c.selected, moved.group.changed);
    let after = frame(&mut h, &mut c);
    assert!(!after.group.changed, "changed once");
    (selected, changed, after.focused, after.group.focus_visible)
}

#[test]
fn an_arrow_moves_focus_and_selection_in_the_same_frame_and_marks_changed_once() {
    use NamedKey::*;
    for (from, key, to) in [
        (10, ArrowDown, 20),
        (10, ArrowRight, 20),
        (20, ArrowDown, 30),
        (30, ArrowUp, 20),
        (30, ArrowLeft, 20),
        (20, ArrowUp, 10),
    ] {
        let (selected, changed, focused, ring) = arrow(from, [true; 3], key);
        assert_eq!(selected, to, "{key:?} from {from}");
        assert!(changed, "{key:?} from {from}");
        assert_eq!(focused, Some(kid(to)), "{key:?} from {from}");
        assert!(ring, "the ring follows a key move");
    }
}

#[test]
fn the_arrows_wrap_both_ways() {
    let (selected, changed, focused, _) = arrow(30, [true; 3], NamedKey::ArrowDown);
    assert_eq!((selected, changed, focused), (10, true, Some(kid(10))));
    let (selected, changed, focused, _) = arrow(10, [true; 3], NamedKey::ArrowUp);
    assert_eq!((selected, changed, focused), (30, true, Some(kid(30))));
    let (selected, _, _, _) = arrow(10, [true; 3], NamedKey::ArrowLeft);
    assert_eq!(selected, 30);
    let (selected, _, _, _) = arrow(30, [true; 3], NamedKey::ArrowRight);
    assert_eq!(selected, 10);
}

#[test]
fn a_disabled_option_is_skipped_both_ways_and_never_selected() {
    let enabled = [true, false, true];
    let (selected, changed, focused, _) = arrow(10, enabled, NamedKey::ArrowDown);
    assert_eq!((selected, changed, focused), (30, true, Some(kid(30))));
    let (selected, _, focused, _) = arrow(30, enabled, NamedKey::ArrowUp);
    assert_eq!((selected, focused), (10, Some(kid(10))));
    // With only one enabled option the arrow finds no move, and reports nothing.
    let (selected, changed, focused, _) = arrow(10, [true, false, false], NamedKey::ArrowDown);
    assert_eq!((selected, changed, focused), (10, false, Some(kid(10))));
}

#[test]
fn space_selects_the_focused_option_once() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(10);
    frame(&mut h, &mut c);
    c.ask = Some(kid(30));
    frame(&mut h, &mut c);
    c.ask = None;
    let seen = frame(&mut h, &mut c);
    assert_eq!(seen.focused, Some(kid(30)));
    assert_eq!(c.selected, 10, "focus alone selects nothing");
    press(&mut h, NamedKey::Space);
    let seen = frame(&mut h, &mut c);
    assert!(seen.group.changed);
    assert_eq!(c.selected, 30);
    let seen = frame(&mut h, &mut c);
    assert!(!seen.group.changed);
    press(&mut h, NamedKey::Space);
    assert!(!frame(&mut h, &mut c).group.changed, "already selected");
}

#[test]
fn a_click_selects_once_and_a_click_on_the_selected_option_changes_nothing() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(10);
    frame(&mut h, &mut c);
    let (x, y) = row_centre(2);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    let seen = frame(&mut h, &mut c);
    assert!(seen.group.changed && seen.group.clicked);
    assert_eq!(c.selected, 30);
    assert_eq!(seen.focused, Some(kid(30)), "a press focuses the row");
    let seen = frame(&mut h, &mut c);
    assert!(!seen.group.changed);
    assert!(!seen.group.focus_visible, "a pointer press shows no ring");
    h.input.click(PointerButton::Primary);
    assert!(!frame(&mut h, &mut c).group.changed);
}

#[test]
fn a_disabled_option_takes_no_click() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(10);
    c.enabled = [true, false, true];
    frame(&mut h, &mut c);
    let (x, y) = row_centre(1);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    let seen = frame(&mut h, &mut c);
    assert_eq!(c.selected, 10);
    assert!(!seen.group.changed);
}

#[test]
fn a_disabled_group_changes_nothing_and_reports_nothing() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(10);
    c.group_on = false;
    frame(&mut h, &mut c);
    let (x, y) = row_centre(2);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    press(&mut h, NamedKey::Space);
    press(&mut h, NamedKey::ArrowDown);
    let seen = frame(&mut h, &mut c);
    assert_eq!(c.selected, 10);
    let g = seen.group;
    assert!(!g.enabled && !g.clicked && !g.changed && !g.hovered && !g.focused);
    // No stop: Tab goes from the first button to the second.
    press(&mut h, NamedKey::Tab);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    let focused = frame(&mut h, &mut c).focused;
    assert_ne!(
        focused.map(|f| KEYS.iter().any(|k| kid(*k) == f)),
        Some(true)
    );
}

#[test]
fn the_accessibility_default_action_selects_and_focus_reaches_a_child() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(10);
    frame(&mut h, &mut c);
    h.input.access(AccessAction {
        target: kid(20),
        action: Action::Default,
        value: None,
    });
    assert!(frame(&mut h, &mut c).group.changed);
    assert_eq!(c.selected, 20);
    h.input.access(AccessAction {
        target: kid(30),
        action: Action::Focus,
        value: None,
    });
    frame(&mut h, &mut c);
    let seen = frame(&mut h, &mut c);
    assert_eq!(seen.focused, Some(kid(30)));
    assert!(!seen.group.focus_visible, "no ring after an access focus");
}

#[test]
fn the_nodes_are_a_radio_group_over_radios_and_checked_follows_the_selection() {
    let mut h = UiHarness::new(WIN);
    for selected in KEYS {
        let mut c = case(selected);
        c.enabled = [true, true, false];
        let seen = frame(&mut h, &mut c);
        let group = seen.tree.get(WidgetId::ROOT.with("g")).expect("group node");
        assert_eq!(
            (group.role, group.name.as_str()),
            (Role::RadioGroup, "Mode")
        );
        assert_eq!(
            group.children,
            KEYS.iter().map(|k| kid(*k)).collect::<Vec<_>>()
        );
        for (i, key) in KEYS.iter().enumerate() {
            let node = seen.tree.get(kid(*key)).expect("radio node");
            assert_eq!(node.role, Role::Radio);
            assert_eq!(node.name, ["Alpha", "Beta", "Gamma"][i]);
            assert_eq!(node.states.contains(State::Checked), *key == selected);
            assert_eq!(node.parent, Some(group.id));
            assert_eq!(node.states.contains(State::Focusable), i != 2);
            assert_eq!(node.states.contains(State::Disabled), i == 2);
            assert_eq!(node.actions.contains(Action::Default), i != 2);
        }
    }
}

/// The circle of row 1: (28, 74), 16 by 16.
const CIRCLE_1: [f32; 4] = [28.0, 74.0, 16.0, 16.0];

/// The alpha of the accent fill of the circle in row 1, 0 when it is not drawn.
fn fill_alpha(scene: &Scene) -> f32 {
    let acc = premul(DARK.accent);
    scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.rect == CIRCLE_1)
        .find(|s| {
            let a = s.color[3];
            a > 0.0
                && (s.color[0] - acc[0] * a).abs() < 1e-4
                && (s.color[1] - acc[1] * a).abs() < 1e-4
        })
        .map_or(0.0, |s| s.color[3])
}

/// Row 1 becomes the selection at 500 ms (the app sets it), then the fill alpha on a 60 Hz grid.
fn fill_run(tier: MotionTier) -> (Vec<(u64, f32)>, Repaint, Scene) {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let mut c = case(10);
    frame(&mut h, &mut c);
    h.at_ms(500);
    c.selected = 20;
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
    let clear: Rgba = [theme.accent[0], theme.accent[1], theme.accent[2], 0];
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [(SELECTION, "selection"), (HOVER, "hover"), (FOCUS, "focus")] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
        conformance(
            &SELECTION,
            &theme.motion,
            tier,
            clear.map(f32::from),
            theme.accent.map(f32::from),
        )
        .unwrap_or_else(|e| panic!("fill {tier:?}: {e}"));
    }
}

#[test]
fn the_fill_rises_monotonically_and_settles_within_180_ms_and_a_frame() {
    let (xs, repaint, last) = fill_run(MotionTier::Full);
    assert!(xs.windows(2).all(|w| w[1].1 >= w[0].1), "monotonic: {xs:?}");
    assert!(xs.iter().any(|&(_, a)| a > 0.0 && a < 1.0), "moves: {xs:?}");
    for &(dt, a) in &xs {
        if dt >= 180 + 17 {
            assert_eq!(a, 1.0, "settled at {dt} ms");
        }
    }
    assert_eq!(repaint, Repaint::Idle);
    // The end is exactly the settled look of the selection on row 1.
    let settled = {
        let mut h = UiHarness::new(WIN);
        let mut c = case(20);
        frame(&mut h, &mut c);
        h.at_ms(1000);
        frame(&mut h, &mut c).scene
    };
    assert_eq!(last, settled);
}

#[test]
fn reduced_keeps_the_fill_change_and_off_snaps_it() {
    let (xs, _, _) = fill_run(MotionTier::Reduced);
    assert!(xs.iter().any(|&(_, a)| a > 0.0 && a < 1.0), "{xs:?}");
    let (xs, _, _) = fill_run(MotionTier::Off);
    assert!(xs[1..].iter().all(|&(_, a)| a == 1.0), "{xs:?}");
}

#[test]
fn pixel_probes_at_scale_two() {
    let probe = |selected: u64, hover: Option<usize>| {
        let mut h = UiHarness::new(WIN);
        h.input.set_scale(2.0);
        let mut c = case(selected);
        frame(&mut h, &mut c);
        if let Some(i) = hover {
            let (x, y) = row_centre(i);
            h.input.pointer_to(x, y);
        }
        h.at_ms(1000);
        frame(&mut h, &mut c);
        h.at_ms(2000);
        frame(&mut h, &mut c).scene
    };
    // Row 1 selected: the circle is 32 px, centred in a 56 px row (y 136 to 192).
    let s = probe(20, None);
    let shapes = &s.layers[0].geometry.under;
    let circle = shapes
        .iter()
        .find(|sh| sh.rect == [56.0, 148.0, 32.0, 32.0] && sh.color == premul(DARK.accent))
        .expect("accent circle");
    assert_eq!(circle.radii, [16.0; 4]);
    let dot = shapes
        .iter()
        .find(|sh| sh.color == premul(DARK.on_accent))
        .expect("dot");
    // 12 px, at the centre of the circle: 56 + 10 = 66, 148 + 10 = 158.
    assert_eq!(dot.rect, [66.0, 158.0, 12.0, 12.0]);
    // Hover on row 0: the row fill is 56 px high and the full width.
    let s = probe(20, Some(0));
    let row = s.layers[0]
        .geometry
        .under
        .iter()
        .find(|sh| sh.rect[3] == 56.0 && sh.rect[2] == 400.0)
        .expect("row fill");
    assert_eq!(row.rect, [40.0, 80.0, 400.0, 56.0]);
    // The label starts 8 px after the circle (its right edge is 44).
    assert!(s.layers[0].texts.iter().any(|t| t.x == 44.0 + 8.0));
}

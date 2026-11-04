//! The star rating (`widget.star-rating` acceptance): click N sets N and a second click clears,
//! `max` changes the count, one roving stop whose arrows move focus only, Enter and Space,
//! the hover fill sampled at 60 Hz, probes at scale 2 and the nodes.

use lune_ui_core::access::{AccessAction, Action, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, WidgetId};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::star_rating::{FOCUS, HOVER, MAX, StarLook, StarState};

const WIN: (f32, f32) = (400.0, 300.0);
/// The first star's top left; buttons are 24 by 24, 2 apart (pitch 26).
const AT: (f32, f32) = (20.0, 40.0);

fn centre(n: u32) -> (f32, f32) {
    (AT.0 + 26.0 * (n - 1) as f32 + 12.0, AT.1 + 12.0)
}

fn kid(n: u32) -> WidgetId {
    WidgetId::ROOT.with("stars").with(n)
}

struct Case {
    value: u32,
    max: u32,
    enabled: bool,
}

fn case(value: u32) -> Case {
    Case {
        value,
        max: MAX,
        enabled: true,
    }
}

struct Seen {
    scene: Scene,
    resp: Response,
    focused: Option<WidgetId>,
    repaint: Repaint,
    tree: lune_ui_core::access::SemanticTree,
}

fn frame(h: &mut UiHarness, c: &mut Case) -> Seen {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| {
        ui.button(
            "before",
            Rect::new(20.0, 4.0, 60.0, 24.0),
            "Before",
            ButtonKind::Secondary,
        );
        let (value, max) = (&mut c.value, c.max);
        resp = Some(ui.with_enabled(c.enabled, |ui| {
            ui.star_rating("stars", AT, value, max, "Rating")
        }));
        ui.button(
            "after",
            Rect::new(20.0, 200.0, 60.0, 24.0),
            "After",
            ButtonKind::Secondary,
        );
    });
    Seen {
        scene,
        resp: resp.expect("frame ran"),
        focused: out.focused,
        repaint: out.repaint,
        tree: out.tree,
    }
}

fn press(h: &mut UiHarness, key: NamedKey) {
    h.input.key_down(Key::Named(key), None);
}

fn click(h: &mut UiHarness, c: &mut Case, n: u32) -> Seen {
    let (x, y) = centre(n);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    frame(h, c)
}

#[test]
fn click_n_sets_n_and_a_second_click_on_n_sets_zero() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(0);
    frame(&mut h, &mut c);
    let seen = click(&mut h, &mut c, 3);
    assert_eq!(c.value, 3);
    assert!(seen.resp.changed);
    assert!(!frame(&mut h, &mut c).resp.changed, "once per click");
    click(&mut h, &mut c, 5);
    assert_eq!(c.value, 5);
    click(&mut h, &mut c, 2);
    assert_eq!(c.value, 2);
    let seen = click(&mut h, &mut c, 2);
    assert_eq!(c.value, 0);
    assert!(seen.resp.changed);
}

#[test]
fn max_changes_the_count_and_the_names() {
    let mut h = UiHarness::new(WIN);
    for max in [3_u32, 5, 7] {
        let mut c = case(0);
        c.max = max;
        let seen = frame(&mut h, &mut c);
        let group = seen.tree.get(WidgetId::ROOT.with("stars")).expect("group");
        assert_eq!(group.children.len(), max as usize);
        for n in 1..=max {
            let node = seen.tree.get(kid(n)).expect("star");
            assert_eq!(node.name, format!("{n} of {max}"));
        }
        assert!(seen.tree.get(kid(max + 1)).is_none());
        assert_eq!(seen.resp.rect.w, 24.0 * max as f32 + 2.0 * (max - 1) as f32);
    }
    // Clicking the last star of a 3-star row sets 3.
    let mut c = case(0);
    c.max = 3;
    frame(&mut h, &mut c);
    click(&mut h, &mut c, 3);
    assert_eq!(c.value, 3);
}

#[test]
fn a_value_above_max_draws_as_max_and_is_left_alone() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(9);
    c.max = 3;
    let seen = frame(&mut h, &mut c);
    assert_eq!(c.value, 9);
    assert!(!seen.resp.changed);
    let node = seen.tree.get(kid(3)).expect("star");
    assert!(node.states.contains(State::Checked));
    // The star drawn as the value is star 3: clicking it clears.
    click(&mut h, &mut c, 3);
    assert_eq!(c.value, 0);
}

#[test]
fn tab_enters_once_on_the_value_star_and_arrows_move_focus_only() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(3);
    frame(&mut h, &mut c);
    let mut stops = Vec::new();
    for _ in 0..3 {
        press(&mut h, NamedKey::Tab);
        stops.push(frame(&mut h, &mut c).focused);
    }
    assert_eq!(
        stops,
        [
            Some(WidgetId::ROOT.with("before")),
            Some(kid(3)),
            Some(WidgetId::ROOT.with("after")),
        ]
    );
    // Back in: Shift+Tab lands on the stop, then the arrows move focus, not the value, and stop
    // at the ends.
    let mut h = UiHarness::new(WIN);
    let mut c = case(3);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    frame(&mut h, &mut c);
    let mut focus = Vec::new();
    for key in [
        NamedKey::ArrowRight,
        NamedKey::ArrowRight,
        NamedKey::ArrowRight,
        NamedKey::ArrowLeft,
        NamedKey::Home,
        NamedKey::ArrowLeft,
        NamedKey::End,
    ] {
        press(&mut h, key);
        let moved = frame(&mut h, &mut c);
        assert!(!moved.resp.changed);
        focus.push(frame(&mut h, &mut c).focused);
    }
    assert_eq!(
        focus,
        [
            Some(kid(4)),
            Some(kid(5)),
            Some(kid(5)),
            Some(kid(4)),
            Some(kid(1)),
            Some(kid(1)),
            Some(kid(5)),
        ]
    );
    assert_eq!(c.value, 3, "arrows never set the value");
    // Tab from a star an arrow reached leaves the row.
    press(&mut h, NamedKey::Tab);
    assert_eq!(
        frame(&mut h, &mut c).focused,
        Some(WidgetId::ROOT.with("after"))
    );
}

#[test]
fn enter_and_space_set_the_focused_star_and_again_clear_it() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(0);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    assert_eq!(
        frame(&mut h, &mut c).focused,
        Some(kid(1)),
        "no value: the first star"
    );
    press(&mut h, NamedKey::ArrowRight);
    frame(&mut h, &mut c);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Enter);
    let seen = frame(&mut h, &mut c);
    assert_eq!((c.value, seen.resp.changed), (2, true));
    assert!(!frame(&mut h, &mut c).resp.changed);
    press(&mut h, NamedKey::Space);
    let seen = frame(&mut h, &mut c);
    assert_eq!(
        (c.value, seen.resp.changed),
        (0, true),
        "the current value clears"
    );
    press(&mut h, NamedKey::Space);
    assert!(frame(&mut h, &mut c).resp.changed);
    assert_eq!(c.value, 2);
}

#[test]
fn the_default_action_sets_the_value_and_clears_it_on_the_current_one() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(0);
    frame(&mut h, &mut c);
    let act = |h: &mut UiHarness, n| {
        h.input.access(AccessAction {
            target: kid(n),
            action: Action::Default,
            value: None,
        });
    };
    act(&mut h, 4);
    assert!(frame(&mut h, &mut c).resp.changed);
    assert_eq!(c.value, 4);
    act(&mut h, 4);
    frame(&mut h, &mut c);
    assert_eq!(c.value, 0);
}

#[test]
fn a_disabled_row_changes_nothing_and_is_no_stop() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(2);
    c.enabled = false;
    frame(&mut h, &mut c);
    click(&mut h, &mut c, 4);
    press(&mut h, NamedKey::Enter);
    let seen = frame(&mut h, &mut c);
    assert_eq!(c.value, 2);
    assert!(!seen.resp.enabled && !seen.resp.changed && !seen.resp.clicked);
    press(&mut h, NamedKey::Tab);
    frame(&mut h, &mut c);
    press(&mut h, NamedKey::Tab);
    let focused = frame(&mut h, &mut c).focused;
    assert!((1..=5).all(|n| focused != Some(kid(n))));
    let node = frame(&mut h, &mut c).tree;
    let node = node.get(kid(2)).expect("star");
    assert!(node.states.contains(State::Disabled));
    assert!(!node.states.contains(State::Focusable));
}

#[test]
fn the_nodes_are_a_radio_group_of_radios_and_checked_follows_the_value() {
    let mut h = UiHarness::new(WIN);
    for value in 0..=5 {
        let mut c = case(value);
        let seen = frame(&mut h, &mut c);
        let group = seen.tree.get(WidgetId::ROOT.with("stars")).expect("group");
        assert_eq!(
            (group.role, group.name.as_str()),
            (Role::RadioGroup, "Rating")
        );
        for n in 1..=5 {
            let node = seen.tree.get(kid(n)).expect("star");
            assert_eq!(node.role, Role::Radio);
            assert_eq!(
                node.states.contains(State::Checked),
                n == value,
                "{n} of {value}"
            );
            assert!(node.states.contains(State::Focusable));
            assert!(node.actions.contains(Action::Default));
            assert_eq!(node.parent, Some(group.id));
        }
    }
}

#[test]
fn filled_stars_are_the_warning_tone_and_empty_ones_the_tertiary_tone() {
    let t = DARK;
    let full = StarLook::of(
        &t,
        StarState {
            filled: true,
            ..StarState::default()
        },
    );
    assert_eq!((full.glyph, full.filled), (t.status_warning, true));
    let empty = StarLook::of(&t, StarState::default());
    assert_eq!((empty.glyph, empty.filled), (t.fg_tertiary, false));
    assert_eq!(empty.fill[3], 0);
    let hover = StarLook::of(
        &t,
        StarState {
            hovered: true,
            ..StarState::default()
        },
    );
    assert_eq!(hover.fill, t.surface_hover);
    let pressed = StarLook::of(
        &t,
        StarState {
            pressed: true,
            ..StarState::default()
        },
    );
    assert_eq!(pressed.fill, t.surface_active);
    // Drawn: value 3 of 5 puts the full glyph on three stars and the empty one on two.
    let mut h = UiHarness::new(WIN);
    let mut c = case(3);
    frame(&mut h, &mut c);
    h.at_ms(1000);
    let s = frame(&mut h, &mut c).scene;
    let glyphs = |g: &str, color| {
        let key = icon::glyph(g, 20.0, color).cache_key(1.0);
        s.layers[0]
            .texts
            .iter()
            .filter(|item| item.key.0 == key)
            .count()
    };
    assert_eq!(
        (
            glyphs(icon::STAR_FULL, t.status_warning),
            glyphs(icon::STAR_EMPTY, t.fg_tertiary)
        ),
        (3, 2)
    );
}

/// The hover fill of star 4: (98, 40), 24 by 24; its alpha, 0 when not drawn.
fn fill_alpha(scene: &Scene) -> f32 {
    scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|s| s.rect == [98.0, 40.0, 24.0, 24.0])
        .map_or(0.0, |s| s.color[3])
}

fn hover_run(tier: MotionTier) -> (Vec<(u64, f32)>, Repaint) {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let mut c = case(0);
    frame(&mut h, &mut c);
    h.at_ms(500);
    let (x, y) = centre(4);
    h.input.pointer_to(x, y);
    let first = frame(&mut h, &mut c);
    let mut xs = vec![(0, fill_alpha(&first.scene))];
    let mut repaint = first.repaint;
    for k in 1..=10_u64 {
        let dt = (k * 1000 + 30) / 60;
        h.at_ms(500 + dt);
        let seen = frame(&mut h, &mut c);
        xs.push((dt, fill_alpha(&seen.scene)));
        repaint = seen.repaint;
    }
    (xs, repaint)
}

#[test]
fn the_hover_fill_rises_monotonically_and_settles_within_120_ms_and_a_frame() {
    let (xs, repaint) = hover_run(MotionTier::Full);
    assert!(xs.windows(2).all(|w| w[1].1 >= w[0].1), "monotonic: {xs:?}");
    assert!(
        xs.iter().any(|&(_, a)| a > 0.0 && a < xs[10].1),
        "moves: {xs:?}"
    );
    let end = premul(DARK.surface_hover)[3];
    for &(dt, a) in &xs {
        if dt >= 120 + 17 {
            assert_eq!(a, end, "settled at {dt} ms");
        }
    }
    assert_eq!(repaint, Repaint::Idle);
    let (xs, _) = hover_run(MotionTier::Off);
    assert!(xs.iter().all(|&(_, a)| a == end), "Off snaps: {xs:?}");
}

#[test]
fn every_motion_row_conforms_under_every_tier() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [(HOVER, "hover"), (FOCUS, "focus")] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
    }
}

#[test]
fn a_value_change_snaps() {
    let mut h = UiHarness::new(WIN);
    let mut c = case(1);
    frame(&mut h, &mut c);
    h.at_ms(500);
    c.value = 4;
    let seen = frame(&mut h, &mut c);
    assert_eq!(
        seen.repaint,
        Repaint::Idle,
        "nothing in flight after a change"
    );
    let settled = {
        let mut h = UiHarness::new(WIN);
        let mut c = case(4);
        frame(&mut h, &mut c);
        h.at_ms(1000);
        frame(&mut h, &mut c).scene
    };
    assert_eq!(seen.scene, settled);
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let mut c = case(0);
    frame(&mut h, &mut c);
    let (x, y) = centre(4);
    h.input.pointer_to(x, y);
    h.at_ms(1000);
    frame(&mut h, &mut c);
    h.at_ms(2000);
    let s = frame(&mut h, &mut c).scene;
    // Star 4's hover fill: a 48 px square with radius 8 px, at 2 * (20 + 3 * 26) = 196.
    let fill = s.layers[0]
        .geometry
        .under
        .iter()
        .find(|sh| sh.rect == [196.0, 80.0, 48.0, 48.0])
        .expect("hover fill");
    assert_eq!(fill.radii, [8.0; 4]);
}

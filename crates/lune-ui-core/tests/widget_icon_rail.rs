//! The icon rail (`layout.icon-rail` acceptance): the padded stacks and the docked bottom group at
//! scale 1 and 2, one tab stop with roving arrows that wrap across the top and bottom groups and
//! skip a disabled item, activation by click, Enter and Space, the active look, the dot and the
//! badge (never animated, scaled with the press), press and hover in three tiers, the nodes, the
//! tooltip and the disabled walk.

use lune_ui_core::access::{Action, Role, SemanticTree, State};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{FrameOutput, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style, TextKey, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::{ButtonKind, ButtonOptions};
use lune_ui_core::widgets::button::{FOCUS, HOVER, PRESS};
use lune_ui_core::widgets::icon_rail::{
    BADGE_H, BADGE_MIN_W, BOTTOM_PITCH, DOT, PITCH, RailItem, badge_text, bottom_height,
    group_height,
};
use lune_ui_core::widgets::toolbar::{ACTION_GLYPH, ACTION_SIZE};

const WIN: (f32, f32) = (400.0, 600.0);
/// The rail at (100, 20), 48 wide (4 + 40 + 4), 500 high.
const RAIL: Rect = Rect::new(100.0, 20.0, 48.0, 500.0);
const TOP: [(u64, &str); 5] = [
    (1, "One"),
    (2, "Two"),
    (3, "Three"),
    (4, "Four"),
    (5, "Five"),
];
const BOTTOM: [(u64, &str); 5] = [
    (6, "Six"),
    (7, "Seven"),
    (8, "Eight"),
    (9, "Nine"),
    (10, "Ten"),
];
const GLYPHS: [&str; 10] = [
    icon::ADD,
    icon::GEAR,
    icon::EDIT,
    icon::COPY,
    icon::INFO,
    icon::SEARCH,
    icon::SYNC,
    icon::CHECK,
    icon::CLOSE,
    icon::CALENDAR,
];

/// The buttons around the rail: ghost, so they fill nothing at rest.
const GHOST: ButtonOptions<'static> = ButtonOptions {
    kind: ButtonKind::Ghost,
    small: false,
    icon: None,
};

fn rail_id() -> WidgetId {
    WidgetId::ROOT.with("rail")
}

fn item_id(key: u64) -> WidgetId {
    rail_id().with(key)
}

/// The rect of the button with `key`.
fn button_rect(key: u64) -> Rect {
    let i = key as usize - 1;
    let y = if i < 5 {
        RAIL.y + 10.0 + PITCH * i as f32
    } else {
        RAIL.bottom() - 10.0 - bottom_height(5) + BOTTOM_PITCH * (i - 5) as f32
    };
    Rect::new(RAIL.x + 4.0, y, ACTION_SIZE, ACTION_SIZE)
}

fn centre(key: u64) -> (f32, f32) {
    let r = button_rect(key);
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

#[derive(Clone)]
struct Cfg {
    active: Option<u64>,
    disabled: Vec<u64>,
    dot: Option<u64>,
    badge: Option<(u64, u32)>,
    note: Option<(u64, &'static str)>,
    enabled: bool,
}

fn cfg() -> Cfg {
    Cfg {
        active: None,
        disabled: Vec::new(),
        dot: None,
        badge: None,
        note: None,
        enabled: true,
    }
}

struct Out {
    scene: Scene,
    out: FrameOutput,
    activated: Option<u64>,
}

fn frame(h: &mut UiHarness, c: &Cfg) -> Out {
    let mut activated = None;
    let (scene, out) = h.frame(|ui| {
        ui.button_with("before", Rect::new(20.0, 4.0, 60.0, 24.0), "B", &GHOST);
        let make = |(key, name): (u64, &'static str)| RailItem {
            key,
            glyph: GLYPHS[key as usize - 1],
            name,
            active: c.active == Some(key),
            enabled: !c.disabled.contains(&key),
            dot: c.dot == Some(key),
            badge: c.badge.filter(|b| b.0 == key).map(|b| b.1),
            note: c.note.filter(|n| n.0 == key).map(|n| n.1),
        };
        let top: Vec<RailItem<'_>> = TOP.iter().map(|i| make(*i)).collect();
        let bottom: Vec<RailItem<'_>> = BOTTOM.iter().map(|i| make(*i)).collect();
        activated = ui.with_enabled(c.enabled, |ui| {
            ui.icon_rail("rail", RAIL, &top, &bottom, "Panels")
        });
        ui.button_with("after", Rect::new(220.0, 4.0, 60.0, 24.0), "A", &GHOST);
    });
    Out {
        scene,
        out,
        activated,
    }
}

/// The clock of a test: frames 16 ms apart.
struct Clock(u64);

impl Clock {
    fn next(&mut self, h: &mut UiHarness) {
        self.0 += 16;
        h.at_ms(self.0);
    }

    fn frame(&mut self, h: &mut UiHarness, c: &Cfg) -> Out {
        self.next(h);
        frame(h, c)
    }

    /// Frames until everything has settled.
    fn settle(&mut self, h: &mut UiHarness, c: &Cfg) -> Out {
        self.frame(h, c);
        self.frame(h, c);
        self.0 += 5000;
        h.at_ms(self.0);
        frame(h, c)
    }

    /// `key` pressed, then two frames; returns the second.
    fn key(&mut self, h: &mut UiHarness, c: &Cfg, key: NamedKey) -> Out {
        h.input.key_down(Key::Named(key), None);
        self.frame(h, c);
        self.frame(h, c)
    }
}

fn start() -> (UiHarness, Clock) {
    (UiHarness::new(WIN), Clock(0))
}

fn fills(scene: &Scene) -> &[Shape] {
    &scene.layers[0].geometry.under
}

fn glyph_key(key: u64, color: [u8; 4], scale: f32) -> TextKey {
    TextKey(icon::glyph(GLYPHS[key as usize - 1], ACTION_GLYPH, color).cache_key(scale))
}

fn drawn(scene: &Scene, key: TextKey) -> bool {
    scene.layers[0].texts.iter().any(|t| t.key == key)
}

fn focused(out: &Out) -> Option<WidgetId> {
    out.out.focused
}

// ---- geometry ----

#[test]
fn the_buttons_are_40_with_gap_4_from_the_padding_and_the_bottom_group_is_docked() {
    let (mut h, mut clock) = start();
    let out = clock.settle(&mut h, &cfg());
    assert_eq!(ACTION_SIZE, 40.0);
    assert_eq!(PITCH, 44.0);
    assert_eq!(group_height(5), 5.0 * 40.0 + 4.0 * 4.0);
    for key in 1..=10u64 {
        let node = out.out.tree.get(item_id(key)).expect("button");
        assert_eq!(node.bounds, button_rect(key), "button {key}");
        assert_eq!(node.role, Role::Button);
    }
    // 4 from the left and right of the 48 wide rail; 10 above the first and below the last.
    assert_eq!(button_rect(1).x - RAIL.x, 4.0);
    assert_eq!(RAIL.right() - button_rect(1).right(), 4.0);
    assert_eq!(button_rect(1).y - RAIL.y, 10.0);
    assert_eq!(RAIL.bottom() - button_rect(10).bottom(), 10.0);
    // The top group is 4 apart inside.
    assert_eq!(button_rect(2).y - button_rect(1).bottom(), 4.0);
    // The bottom group has no spacing (`IconRailView.axaml:22-24`): its buttons touch.
    assert_eq!(button_rect(7).y - button_rect(6).bottom(), 0.0);
    assert_eq!(bottom_height(5), 5.0 * 40.0);
    // The rail itself fills nothing at rest.
    assert!(fills(&out.scene).is_empty());
    assert_eq!(out.out.repaint, Repaint::Idle);
}

#[test]
fn pixel_probes_at_scale_two() {
    let (mut h, mut clock) = start();
    h.input.set_scale(2.0);
    let c = Cfg {
        active: Some(2),
        dot: Some(3),
        badge: Some((8, 3)),
        ..cfg()
    };
    let out = clock.settle(&mut h, &c);
    // The active button: 80 by 80 px, radius 12 px, at the 8 px side padding and 20 px top
    // padding of the rail.
    let sel = fills(&out.scene)
        .iter()
        .find(|s| s.color == premul(DARK.surface_selected))
        .expect("the active fill");
    let r = button_rect(2);
    assert_eq!(sel.rect, [r.x * 2.0, r.y * 2.0, 80.0, 80.0]);
    assert_eq!(sel.radii, [12.0; 4]);
    assert_eq!(sel.rect[0] - RAIL.x * 2.0, 8.0);
    assert_eq!(button_rect(1).y * 2.0 - RAIL.y * 2.0, 20.0);
    assert_eq!(
        button_rect(2).y * 2.0 - button_rect(1).y * 2.0,
        88.0,
        "40 px + the 8 px gap"
    );
    // The icon is 40 px: the 20 px glyph at scale 2.
    assert!(drawn(&out.scene, glyph_key(2, DARK.fg_primary, 2.0)));
    assert!(drawn(&out.scene, glyph_key(1, DARK.fg_muted, 2.0)));
    // The dot is 16 px, radius 8, the badge 32 px high with radius 16.
    let b3 = button_rect(3);
    let dot = fills(&out.scene)
        .iter()
        .find(|s| s.color == premul(DARK.dirty_indicator))
        .expect("dot");
    assert_eq!(
        dot.rect,
        [(b3.x + 27.0) * 2.0, (b3.y + 7.0) * 2.0, 16.0, 16.0]
    );
    assert_eq!(dot.radii, [8.0; 4]);
    let badge = fills(&out.scene)
        .iter()
        .find(|s| s.color == premul(DARK.count_badge))
        .expect("badge");
    assert_eq!(badge.rect[3], 32.0);
    assert_eq!(badge.radii, [16.0; 4]);
}

// ---- the look ----

#[test]
fn rest_is_ghost_hover_and_press_use_the_toolbars_fills() {
    let (mut h, mut clock) = start();
    let out = clock.settle(&mut h, &cfg());
    for key in 1..=10 {
        assert!(
            drawn(&out.scene, glyph_key(key, DARK.fg_muted, 1.0)),
            "{key}"
        );
    }
    assert!(fills(&out.scene).is_empty(), "no fill at rest");
    let (x, y) = centre(4);
    h.input.pointer_to(x, y);
    clock.frame(&mut h, &cfg());
    clock.next(&mut h);
    clock.0 += 300;
    h.at_ms(clock.0);
    let hov = frame(&mut h, &cfg());
    let f = fills(&hov.scene);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].color, premul(DARK.surface_hover));
    assert_eq!(f[0].radii, [DARK.item_radius; 4]);
}

#[test]
fn an_active_item_takes_the_selected_fill_the_bright_glyph_and_reads_pressed() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        active: Some(3),
        ..cfg()
    };
    let first = clock.frame(&mut h, &c);
    // The first frame does not animate: the look is there at once.
    assert_eq!(first.out.repaint, Repaint::Idle);
    let f = fills(&first.scene);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].color, premul(DARK.surface_selected));
    let r = button_rect(3);
    assert_eq!(f[0].rect, [r.x, r.y, 40.0, 40.0]);
    assert!(drawn(&first.scene, glyph_key(3, DARK.fg_primary, 1.0)));
    assert!(drawn(&first.scene, glyph_key(1, DARK.fg_muted, 1.0)));
    let node = first.out.tree.get(item_id(3)).expect("node");
    assert!(node.states.contains(State::Pressed));
    assert!(
        !first
            .out
            .tree
            .get(item_id(1))
            .expect("present")
            .states
            .contains(State::Pressed)
    );
    // Moving the mark fades the fill over `fast` (selection) and settles on the new item.
    let moved = Cfg {
        active: Some(4),
        ..cfg()
    };
    let mid = {
        clock.next(&mut h);
        frame(&mut h, &moved)
    };
    assert_eq!(mid.out.repaint, Repaint::Now);
    clock.0 += 400;
    h.at_ms(clock.0);
    let done = frame(&mut h, &moved);
    assert_eq!(done.out.repaint, Repaint::Idle);
    let f = fills(&done.scene);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].rect[1], button_rect(4).y);
}

#[test]
fn the_dot_and_the_badge_appear_at_once_and_at_their_offsets() {
    let (mut h, mut clock) = start();
    clock.settle(&mut h, &cfg());
    let c = Cfg {
        dot: Some(3),
        badge: Some((8, 3)),
        ..cfg()
    };
    let out = clock.frame(&mut h, &c);
    // Never animated: full colour on the first frame that has them, nothing in flight.
    assert_eq!(out.out.repaint, Repaint::Idle);
    let b3 = button_rect(3);
    let dot = fills(&out.scene)
        .iter()
        .find(|s| s.color == premul(DARK.dirty_indicator))
        .expect("dot");
    assert_eq!(dot.rect, [b3.x + 27.0, b3.y + 7.0, DOT, DOT]);
    assert_eq!(dot.radii, [4.0; 4]);
    let b8 = button_rect(8);
    let badge = fills(&out.scene)
        .iter()
        .find(|s| s.color == premul(DARK.count_badge))
        .expect("badge");
    assert_eq!((badge.rect[3], badge.radii), (BADGE_H, [8.0; 4]));
    assert_eq!(badge.rect[0] + badge.rect[2], b8.x + 34.0, "the right edge");
    assert_eq!(badge.rect[1], b8.y + 10.0);
    assert!(badge.rect[2] >= BADGE_MIN_W);
    // The count's text is 9 px, bold, on_accent.
    let spec = Spec::plain("3", Style::ui(9.0, DARK.on_accent).bold());
    assert!(drawn(&out.scene, TextKey(spec.cache_key(1.0))));
    // Removing them takes them away at once.
    let gone = clock.frame(&mut h, &cfg());
    assert!(fills(&gone.scene).is_empty());
}

#[test]
fn a_count_above_99_reads_99_plus_and_zero_shows_no_badge() {
    assert_eq!(badge_text(0), "0");
    assert_eq!(badge_text(7), "7");
    assert_eq!(badge_text(99), "99");
    assert_eq!(badge_text(100), "99+");
    assert_eq!(badge_text(u32::MAX), "99+");
    let (mut h, mut clock) = start();
    let c = Cfg {
        badge: Some((8, 250)),
        ..cfg()
    };
    let out = clock.settle(&mut h, &c);
    let spec = Spec::plain("99+", Style::ui(9.0, DARK.on_accent).bold());
    assert!(drawn(&out.scene, TextKey(spec.cache_key(1.0))));
    // Wider than the minimum when the text needs it.
    let w = FakeText.prepare(&spec, 1.0).1.0 + 4.0;
    let badge = fills(&out.scene)
        .iter()
        .find(|s| s.color == premul(DARK.count_badge))
        .expect("badge");
    assert_eq!(badge.rect[2], w.max(BADGE_MIN_W));
    let none = Cfg {
        badge: Some((8, 0)),
        ..cfg()
    };
    let out = clock.settle(&mut h, &none);
    assert!(fills(&out.scene).is_empty(), "zero shows none");
}

#[test]
fn the_dot_and_the_badge_scale_with_the_press() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        dot: Some(3),
        badge: Some((3, 5)),
        ..cfg()
    };
    clock.settle(&mut h, &c);
    let (x, y) = centre(3);
    h.input.pointer_to(x, y);
    clock.frame(&mut h, &c);
    h.input.press(PointerButton::Primary);
    clock.frame(&mut h, &c);
    clock.0 += 400;
    h.at_ms(clock.0);
    let out = frame(&mut h, &c);
    assert_eq!(out.out.repaint, Repaint::Idle);
    let r = button_rect(3);
    let (cx, cy) = (r.x + 20.0, r.y + 20.0);
    let s = 0.98_f32;
    let dot = fills(&out.scene)
        .iter()
        .find(|s| s.color == premul(DARK.dirty_indicator))
        .expect("dot");
    let want = [
        cx + (r.x + 27.0 - cx) * s,
        cy + (r.y + 7.0 - cy) * s,
        DOT * s,
        DOT * s,
    ];
    for (g, w) in dot.rect.iter().zip(want) {
        assert!((g - w).abs() < 1e-3, "{:?} vs {want:?}", dot.rect);
    }
    let badge = fills(&out.scene)
        .iter()
        .find(|s| s.color == premul(DARK.count_badge))
        .expect("badge");
    assert!((badge.rect[3] - BADGE_H * s).abs() < 1e-3);
}

#[test]
fn press_hover_and_focus_conform_in_three_tiers() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [(PRESS, "press"), (HOVER, "hover"), (FOCUS, "focus")] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
    }
    assert!(theme.motion.resolve(&PRESS, MotionTier::Reduced).is_none());
    // Press: half way through `fast` the scale is between 1 and 0.98.
    let scale_at = |tier: MotionTier, ms: u64| {
        let (mut h, mut clock) = start();
        h.tier = tier;
        let c = Cfg {
            active: Some(1),
            ..cfg()
        };
        clock.settle(&mut h, &c);
        let (x, y) = centre(2);
        h.input.pointer_to(x, y);
        clock.frame(&mut h, &c);
        h.input.press(PointerButton::Primary);
        clock.frame(&mut h, &c);
        let t = clock.0;
        h.at_ms(t + ms);
        let out = frame(&mut h, &c);
        // The pressed button's fill (the active one's is the first button's).
        let f = fills(&out.scene)
            .iter()
            .find(|s| (s.rect[1] - button_rect(2).y).abs() < 2.0)
            .unwrap_or_else(|| panic!("the pressed fill {tier:?} {ms}: {:?}", fills(&out.scene)));
        (f.rect[2] / ACTION_SIZE, out.out.repaint)
    };
    let (mid, repaint) = scale_at(MotionTier::Full, 90);
    assert!(mid < 1.0 && mid > 0.98, "{mid}");
    assert_eq!(repaint, Repaint::Now);
    let (done, repaint) = scale_at(MotionTier::Full, 250);
    assert!((done - 0.98).abs() < 1e-4, "{done}");
    assert_eq!(repaint, Repaint::Idle);
    let (snap, _) = scale_at(MotionTier::Reduced, 0);
    assert!((snap - 0.98).abs() < 1e-4, "{snap}");
}

// ---- input ----

#[test]
fn the_rail_is_one_tab_stop_entered_once_on_the_active_item() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        active: Some(4),
        ..cfg()
    };
    clock.settle(&mut h, &c);
    let mut stops = Vec::new();
    for _ in 0..4 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        stops.push(focused(&clock.frame(&mut h, &c)));
    }
    assert_eq!(
        stops,
        [
            Some(WidgetId::ROOT.with("before")),
            Some(item_id(4)),
            Some(WidgetId::ROOT.with("after")),
            Some(WidgetId::ROOT.with("before")),
        ],
        "before, the rail's one stop on the active item, after, around"
    );
    // With no active item the first enabled one is the stop.
    let (mut h, mut clock) = start();
    let c = Cfg {
        disabled: vec![1],
        ..cfg()
    };
    clock.settle(&mut h, &c);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    clock.frame(&mut h, &c);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    assert_eq!(focused(&clock.frame(&mut h, &c)), Some(item_id(2)));
}

fn at_stop(h: &mut UiHarness, clock: &mut Clock, c: &Cfg) {
    clock.settle(h, c);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    clock.frame(h, c);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    clock.frame(h, c);
}

#[test]
fn the_arrows_move_through_both_groups_and_wrap_at_the_ends() {
    let (mut h, mut clock) = start();
    let c = cfg();
    at_stop(&mut h, &mut clock, &c);
    assert_eq!(focused(&clock.frame(&mut h, &c)), Some(item_id(1)));
    // Up from the first wraps to the last of the bottom group.
    let out = clock.key(&mut h, &c, NamedKey::ArrowUp);
    assert_eq!(focused(&out), Some(item_id(10)));
    // Down from the last wraps to the first.
    let out = clock.key(&mut h, &c, NamedKey::ArrowDown);
    assert_eq!(focused(&out), Some(item_id(1)));
    // Down walks the top group and crosses into the bottom one.
    let mut seen = Vec::new();
    for _ in 0..5 {
        seen.push(focused(&clock.key(&mut h, &c, NamedKey::ArrowDown)));
    }
    assert_eq!(
        seen,
        [2, 3, 4, 5, 6].map(|k| Some(item_id(k))),
        "across the groups"
    );
    // Up goes back across.
    assert_eq!(
        focused(&clock.key(&mut h, &c, NamedKey::ArrowUp)),
        Some(item_id(5))
    );
    // Home and End.
    assert_eq!(
        focused(&clock.key(&mut h, &c, NamedKey::End)),
        Some(item_id(10))
    );
    assert_eq!(
        focused(&clock.key(&mut h, &c, NamedKey::Home)),
        Some(item_id(1))
    );
    // The ring follows the keyboard.
    let out = clock.frame(&mut h, &c);
    assert!(
        out.out
            .tree
            .get(item_id(1))
            .expect("present")
            .states
            .contains(State::Focused)
    );
}

#[test]
fn a_disabled_item_is_skipped_inert_and_drawn_at_0_4() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        disabled: vec![3, 6],
        ..cfg()
    };
    at_stop(&mut h, &mut clock, &c);
    assert_eq!(focused(&clock.frame(&mut h, &c)), Some(item_id(1)));
    // 1 -> 2 -> 4 (3 skipped) -> 5 -> 7 (6 skipped).
    let order: Vec<_> = {
        let (mut h, mut clock) = start();
        at_stop(&mut h, &mut clock, &c);
        (0..4)
            .map(|_| focused(&clock.key(&mut h, &c, NamedKey::ArrowDown)))
            .collect()
    };
    assert_eq!(order, [2, 4, 5, 7].map(|k| Some(item_id(k))));
    // A click on a disabled item activates nothing.
    let (x, y) = centre(3);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    let out = clock.frame(&mut h, &c);
    assert_eq!(out.activated, None);
    let node = out.out.tree.get(item_id(3)).expect("node");
    assert!(node.states.contains(State::Disabled));
    assert!(!node.states.contains(State::Focusable));
    assert!(!node.actions.contains(Action::Default));
    // Its glyph is drawn at 0.4 of the muted alpha; an enabled one at full.
    let alpha = |scene: &Scene, key: u64| {
        (0..=255u8).find(|a| {
            let [r, g, b, _] = DARK.fg_muted;
            drawn(scene, glyph_key(key, [r, g, b, *a], 1.0))
        })
    };
    let base = f32::from(DARK.fg_muted[3]);
    assert_eq!(alpha(&out.scene, 1), Some(DARK.fg_muted[3]));
    let dim = f32::from(alpha(&out.scene, 3).expect("drawn"));
    assert!((dim - base * 0.4).abs() <= 1.0, "{dim} vs {}", base * 0.4);
}

#[test]
fn click_enter_and_space_each_activate_once_and_report_the_key() {
    let (mut h, mut clock) = start();
    let c = cfg();
    clock.settle(&mut h, &c);
    let (x, y) = centre(7);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    assert_eq!(clock.frame(&mut h, &c).activated, Some(7));
    assert_eq!(clock.frame(&mut h, &c).activated, None, "once");
    // Enter and Space on the focused item.
    for key in [NamedKey::Enter, NamedKey::Space] {
        let (mut h, mut clock) = start();
        at_stop(&mut h, &mut clock, &c);
        clock.key(&mut h, &c, NamedKey::ArrowDown);
        h.input.key_down(Key::Named(key), None);
        let out = clock.frame(&mut h, &c);
        assert_eq!(out.activated, Some(2), "{key:?}");
        assert_eq!(clock.frame(&mut h, &c).activated, None);
    }
    // The default action of the node.
    let (mut h, mut clock) = start();
    clock.settle(&mut h, &c);
    h.input.access(lune_ui_core::access::AccessAction {
        target: item_id(9),
        action: Action::Default,
        value: None,
    });
    assert_eq!(clock.frame(&mut h, &c).activated, Some(9));
}

// ---- nodes ----

#[test]
fn the_nodes_are_a_toolbar_of_named_buttons_pressed_when_active_and_described_by_the_note() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        active: Some(2),
        dot: Some(3),
        badge: Some((8, 3)),
        note: Some((8, "3 unread")),
        ..cfg()
    };
    let out = clock.settle(&mut h, &c);
    let tree: &SemanticTree = &out.out.tree;
    let bar = tree.get(rail_id()).expect("toolbar");
    assert_eq!((bar.role, bar.name.as_str()), (Role::Toolbar, "Panels"));
    assert_eq!(bar.bounds, RAIL);
    assert_eq!(bar.children.len(), 10);
    for (key, name) in TOP.iter().chain(&BOTTOM) {
        let n = tree.get(item_id(*key)).expect("button");
        assert_eq!((n.role, n.name.as_str()), (Role::Button, *name));
        assert_eq!(n.parent, Some(rail_id()));
        assert_eq!(n.states.contains(State::Pressed), *key == 2);
        assert!(n.actions.contains(Action::Default));
    }
    assert_eq!(
        tree.get(item_id(8))
            .expect("present")
            .description
            .as_deref(),
        Some("3 unread")
    );
    assert_eq!(tree.get(item_id(3)).expect("present").description, None);
    assert_eq!(tree.get(item_id(1)).expect("present").description, None);
}

#[test]
fn a_tooltip_names_the_button_after_the_delay_and_the_description_stays_the_note() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        note: Some((3, "Uncommitted changes")),
        ..cfg()
    };
    clock.settle(&mut h, &c);
    let (x, y) = centre(3);
    h.input.pointer_to(x, y);
    clock.frame(&mut h, &c);
    clock.0 += 1200;
    h.at_ms(clock.0);
    let out = frame(&mut h, &c);
    let shown = !out.scene.layers[LayerId::Popup as usize]
        .geometry
        .under
        .is_empty();
    assert!(shown, "the tooltip is up");
    assert_eq!(
        out.out
            .tree
            .get(item_id(3))
            .expect("present")
            .description
            .as_deref(),
        Some("Uncommitted changes")
    );
    // Without a note the description is empty even while the tooltip shows.
    let (x, y) = centre(4);
    h.input.pointer_to(x, y);
    clock.frame(&mut h, &c);
    clock.0 += 1200;
    h.at_ms(clock.0);
    let out = frame(&mut h, &c);
    assert!(
        !out.scene.layers[LayerId::Popup as usize]
            .geometry
            .under
            .is_empty()
    );
    assert_eq!(
        out.out.tree.get(item_id(4)).expect("present").description,
        None
    );
}

// ---- disabled ----

#[test]
fn a_disabled_rail_is_disabled_all_the_way_down_and_consumes_no_key() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        enabled: false,
        dot: Some(3),
        badge: Some((8, 3)),
        ..cfg()
    };
    let out = clock.settle(&mut h, &c);
    let bad = disabled_violations(&out.out.tree, rail_id());
    assert!(bad.is_empty(), "{bad:?}");
    // The only focusable things are the two plain buttons around it.
    for _ in 0..3 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        let f = focused(&clock.frame(&mut h, &c));
        assert!(
            f != Some(item_id(1))
                && f.is_none_or(
                    |id| id == WidgetId::ROOT.with("before") || id == WidgetId::ROOT.with("after")
                ),
            "{f:?}"
        );
    }
    // An enabled rail is not marked disabled.
    let (mut h, mut clock) = start();
    let out = clock.settle(&mut h, &cfg());
    assert!(
        !out.out
            .tree
            .get(rail_id())
            .expect("present")
            .states
            .contains(State::Disabled)
    );
    // A global binding on Enter, Space and the arrows still fires with the rail disabled.
    for k in [
        NamedKey::Enter,
        NamedKey::Space,
        NamedKey::ArrowDown,
        NamedKey::ArrowUp,
    ] {
        let (mut h, mut clock) = start();
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        clock.settle(&mut h, &c);
        h.input.key_down(Key::Named(k), None);
        let out = clock.frame(&mut h, &c);
        assert_eq!(out.out.fired.len(), 1, "{k:?}");
    }
}

#[test]
fn the_rail_draws_only_on_the_layer_it_is_called_on() {
    let (mut h, mut clock) = start();
    let c = Cfg {
        active: Some(1),
        dot: Some(3),
        badge: Some((8, 3)),
        ..cfg()
    };
    let out = clock.settle(&mut h, &c);
    for (i, l) in out.scene.layers.iter().enumerate() {
        if i != LayerId::Base as usize {
            assert!(
                l.texts.is_empty() && l.geometry.under.is_empty(),
                "layer {i}"
            );
        }
    }
}

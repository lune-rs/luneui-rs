//! The toolbar (`widget.toolbar` acceptance): glass while active and the solid fallback when not,
//! the columns, each action running once and a disabled one not, the tooltip after the delay,
//! pixel probes at scale 2, press and hover motion per tier, the labelled button's elision, and
//! the semantic nodes.

use std::time::Duration;

use lune_ui_core::GlassLayer;
use lune_ui_core::access::{Action, Role, SemanticTree, State};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Spec, Style, TextKey, elide};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::{FOCUS, HOVER, PRESS};
use lune_ui_core::widgets::toolbar::{
    ACTION_GLYPH, ACTION_SIZE, LABEL_GAP, LABEL_GLYPH, LABEL_MAX_WIDTH, LABEL_PAD_X, ToolbarCols,
    ToolbarOptions, label_width,
};

const WIN: (f32, f32) = (900.0, 200.0);
const BAR: Rect = Rect::new(0.0, 0.0, 900.0, 52.0);
/// Any layer and under colour will do: the widget draws what it is given.
const GLASS: GlassLayer = GlassLayer {
    tint: [10, 20, 30, 255],
    tint_opacity: 0.8,
    material_opacity: 0.7,
};
const UNDER: [f32; 4] = [0.05, 0.06, 0.07, 1.0];
const NAMES: [&str; 4] = ["Fetch", "Pull", "Push", "Branch"];

fn bar_id() -> WidgetId {
    WidgetId::ROOT.with("bar")
}

fn id(key: &str) -> WidgetId {
    bar_id().with(key)
}

fn action_rect(i: usize) -> Rect {
    Rect::new(
        8.0 + i as f32 * (ACTION_SIZE + 8.0),
        6.0,
        ACTION_SIZE,
        ACTION_SIZE,
    )
}

fn centre(r: Rect) -> (f32, f32) {
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

fn opts(active: bool) -> ToolbarOptions<'static> {
    ToolbarOptions {
        active,
        glass: GLASS,
        under: UNDER,
        name: "Main toolbar",
    }
}

/// What one frame of the bar reports.
struct Out {
    scene: Scene,
    repaint: Repaint,
    tree: SemanticTree,
    clicked: Vec<usize>,
}

/// The bar with four actions (the third disabled when `third_enabled` is false).
fn frame(h: &mut UiHarness, active: bool, third_enabled: bool) -> Out {
    let mut clicked = Vec::new();
    let (scene, out) = h.frame(|ui| {
        ui.toolbar("bar", BAR, &opts(active), |ui, _cols| {
            for (i, name) in NAMES.iter().enumerate() {
                let key = name.to_lowercase();
                let enabled = i != 2 || third_enabled;
                let r = ui.toolbar_action(key, action_rect(i), icon::SYNC, name, enabled);
                if r.clicked {
                    clicked.push(i);
                }
            }
        });
    });
    Out {
        scene,
        repaint: out.repaint,
        tree: out.tree,
        clicked,
    }
}

fn settled(h: &mut UiHarness, active: bool) -> Out {
    frame(h, active, true);
    h.at_ms(10);
    frame(h, active, true);
    h.at_ms(2000);
    frame(h, active, true)
}

#[test]
fn active_draws_the_glass_layer_and_no_fallback_fill() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, true);
    let g = &out.scene.layers[0].geometry;
    assert_eq!(g.acrylic.len(), 1);
    let a = g.acrylic[0].shape;
    assert_eq!(a.rect, [0.0, 0.0, 900.0, 52.0]);
    assert_eq!(a.color, GLASS.premultiplied());
    assert_eq!(a.under, UNDER);
    assert!(g.under.is_empty(), "nothing else is filled at rest");
}

#[test]
fn inactive_draws_the_window_fallback_fill_and_no_acrylic() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, false);
    let g = &out.scene.layers[0].geometry;
    assert!(g.acrylic.is_empty());
    assert_eq!(g.under.len(), 1);
    assert_eq!(g.under[0].color, premul(DARK.window_fallback));
    assert_eq!(g.under[0].rect, [0.0, 0.0, 900.0, 52.0]);
    assert_eq!(DARK.window_fallback, [0x33, 0x33, 0x33, 0xFF]);
    assert_eq!(out.repaint, Repaint::Idle);
}

#[test]
fn the_columns_are_the_padded_grid_with_a_zero_wide_middle() {
    let mut cols = None;
    let mut h = UiHarness::new(WIN);
    h.frame(|ui| {
        cols = Some(ui.toolbar("bar", BAR, &opts(true), |_, c| c));
    });
    let c = cols.expect("ran");
    // Padding 0 left, 6 top and bottom, 8 right: the inner box is 892 by 40 at (0, 6).
    assert_eq!(c.left, Rect::new(0.0, 6.0, 446.0, 40.0));
    assert_eq!(c.center, Rect::new(446.0, 6.0, 0.0, 40.0));
    assert_eq!(c.right, Rect::new(446.0, 6.0, 446.0, 40.0));
    assert_eq!(ToolbarCols::of(BAR), c);
}

#[test]
fn each_action_runs_once_by_click_and_by_key() {
    for i in 0..NAMES.len() {
        let mut h = UiHarness::new(WIN);
        frame(&mut h, true, true);
        let (x, y) = centre(action_rect(i));
        h.input.pointer_to(x, y).click(PointerButton::Primary);
        h.at_ms(10);
        assert_eq!(frame(&mut h, true, true).clicked, vec![i], "click {i}");
        h.at_ms(20);
        assert!(frame(&mut h, true, true).clicked.is_empty(), "once");
    }
    // Tab to the first action, Enter then Space.
    for key in [NamedKey::Enter, NamedKey::Space] {
        let mut h = UiHarness::new(WIN);
        frame(&mut h, true, true);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        h.at_ms(10);
        frame(&mut h, true, true);
        h.input.key_down(Key::Named(key), None);
        h.at_ms(20);
        assert_eq!(frame(&mut h, true, true).clicked, vec![0], "{key:?}");
    }
}

#[test]
fn a_disabled_action_does_not_run_and_is_marked_disabled() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, true, false);
    let (x, y) = centre(action_rect(2));
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    h.at_ms(10);
    let out = frame(&mut h, true, false);
    assert!(out.clicked.is_empty());
    let node = out.tree.get(id("push")).expect("node");
    assert!(node.states.contains(State::Disabled));
    assert!(!node.states.contains(State::Focusable));
    assert!(!node.actions.contains(Action::Default));
    // Tab skips it and the others still work.
    let mut h = UiHarness::new(WIN);
    frame(&mut h, true, false);
    for _ in 0..3 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        h.at_ms(10);
        frame(&mut h, true, false);
    }
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    h.at_ms(20);
    assert_eq!(frame(&mut h, true, false).clicked, vec![3], "Push skipped");
}

fn shown(scene: &Scene) -> bool {
    !scene.layers[LayerId::Popup as usize]
        .geometry
        .under
        .is_empty()
}

#[test]
fn a_tooltip_shows_after_the_delay_and_names_the_action() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, true, true);
    let (x, y) = centre(action_rect(1));
    h.input.pointer_to(x, y);
    h.at_ms(10);
    frame(&mut h, true, true);
    // The hover tween has settled by 300 ms: only the dwell is pending.
    h.at_ms(300);
    let pending = frame(&mut h, true, true);
    assert!(!shown(&pending.scene));
    assert_eq!(
        pending.repaint,
        Repaint::At(Duration::from_millis(810)),
        "waiting for the dwell, not moving"
    );
    h.at_ms(809);
    assert!(!shown(&frame(&mut h, true, true).scene), "809 ms");
    h.at_ms(810);
    let at = frame(&mut h, true, true);
    assert!(shown(&at.scene), "after the delay");
    let node = at.tree.get(id("pull")).expect("node");
    assert_eq!(node.name, "Pull");
    assert_eq!(node.description.as_deref(), Some("Pull"));
    // Leaving hides it.
    h.input.pointer_to(700.0, 150.0);
    h.at_ms(1000);
    frame(&mut h, true, true);
    h.at_ms(1200);
    assert!(!shown(&frame(&mut h, true, true).scene));
}

#[test]
fn a_disabled_action_still_shows_its_tooltip() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, true, false);
    let (x, y) = centre(action_rect(2));
    h.input.pointer_to(x, y);
    h.at_ms(10);
    frame(&mut h, true, false);
    h.at_ms(900);
    assert!(shown(&frame(&mut h, true, false).scene));
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    frame(&mut h, true, true);
    let (x, y) = centre(action_rect(1));
    h.input.pointer_to(x, y);
    h.at_ms(10);
    frame(&mut h, true, true);
    h.at_ms(2000);
    let out = frame(&mut h, true, true);
    let under = &out.scene.layers[0].geometry.under;
    assert_eq!(under.len(), 1, "the hover fill");
    // 80 by 80 px, radius 12 px.
    let r = action_rect(1);
    assert_eq!(under[0].rect, [r.x * 2.0, r.y * 2.0, 80.0, 80.0]);
    assert_eq!(under[0].radii, [12.0; 4]);
    assert_eq!(under[0].color, premul(DARK.surface_hover));
    // The icon is 20 logical px (40 px here) in fg_muted: the first action's text is the shaped
    // glyph of that size and colour.
    let spec = icon::glyph(icon::SYNC, ACTION_GLYPH, DARK.fg_muted);
    let want = TextKey(spec.cache_key(2.0));
    assert!(
        out.scene.layers[0].texts.iter().all(|t| t.key == want),
        "every action glyph is the 20 px muted glyph"
    );
    assert_eq!(out.scene.layers[0].texts.len(), 4);
}

#[test]
fn press_and_hover_conform_and_reduced_snaps_only_the_scale() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, name) in [(PRESS, "press"), (HOVER, "hover"), (FOCUS, "focus")] {
            conformance(&row, &theme.motion, tier, 0.0_f32, 1.0)
                .unwrap_or_else(|e| panic!("{name} {tier:?}: {e}"));
        }
    }
    assert!(theme.motion.resolve(&PRESS, MotionTier::Reduced).is_none());
    assert!(theme.motion.resolve(&HOVER, MotionTier::Reduced).is_some());

    // Press at 10 ms; the next frames show the scale.
    let scale_at = |tier: MotionTier, ms: u64| {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        frame(&mut h, true, true);
        let (x, y) = centre(action_rect(0));
        h.input.pointer_to(x, y);
        h.at_ms(10);
        frame(&mut h, true, true);
        h.input.press(PointerButton::Primary);
        h.at_ms(20);
        frame(&mut h, true, true);
        h.at_ms(20 + ms);
        let out = frame(&mut h, true, true);
        let under = &out.scene.layers[0].geometry.under;
        (under[0].rect[2] / ACTION_SIZE, out.repaint)
    };
    // Full: half way through `fast` (180 ms) the scale is between 1 and 0.98, and still moving.
    let (mid, repaint) = scale_at(MotionTier::Full, 90);
    assert!(mid < 1.0 && mid > 0.98, "{mid}");
    assert_eq!(repaint, Repaint::Now);
    // Settled by 180 ms + 1 frame.
    let (done, repaint) = scale_at(MotionTier::Full, 200);
    assert!((done - 0.98).abs() < 1e-4, "{done}");
    assert_eq!(repaint, Repaint::Idle);
    // Reduced: the scale is the final one at once.
    let (snap, _) = scale_at(MotionTier::Reduced, 0);
    assert!((snap - 0.98).abs() < 1e-4, "{snap}");
}

// ---- The labelled button ----

const LABEL_RECT: Rect = Rect::new(300.0, 6.0, 200.0, 40.0);

fn label_frame(h: &mut UiHarness, rect: Rect, text: &str) -> (Scene, bool, SemanticTree) {
    let mut clicked = false;
    let (scene, out) = h.frame(|ui| {
        ui.toolbar("bar", BAR, &opts(true), |ui, _| {
            clicked = ui
                .toolbar_label("branch", rect, icon::SYNC, text, "Branch")
                .clicked;
        });
    });
    (scene, clicked, out.tree)
}

#[test]
fn a_short_label_is_drawn_whole_and_a_long_one_is_elided() {
    let theme = DARK;
    let glyph = icon::glyph(icon::SYNC, LABEL_GLYPH, theme.fg_subtle);
    let mut tmp = FakeText;
    // A rect much wider than the 420 cap.
    let wide = Rect::new(10.0, 6.0, 800.0, 40.0);
    let long = "feature/".to_owned() + &"x".repeat(200);
    let mut h = UiHarness::new(WIN);
    let (scene, _, _) = label_frame(&mut h, wide, &long);
    let texts = &scene.layers[0].texts;
    assert_eq!(texts.len(), 2, "glyph and text");
    assert_eq!(
        texts[0].key,
        TextKey(glyph.cache_key(1.0)),
        "the glyph is 14 px"
    );
    let full = label_spec(&theme, &long, theme.fg_subtle);
    let want = elide(&mut FakeText, &full, 1.0, LABEL_MAX_WIDTH);
    assert_ne!(want, full, "the long text really is cut");
    assert_eq!(texts[1].key, TextKey(want.cache_key(1.0)));
    assert!(
        tmp_width(&mut tmp, &want) <= LABEL_MAX_WIDTH,
        "elided to at most 420"
    );

    // Short text is untouched.
    let mut h = UiHarness::new(WIN);
    let (scene, _, _) = label_frame(&mut h, LABEL_RECT, "main");
    let short = label_spec(&theme, "main", theme.fg_subtle);
    assert_eq!(
        scene.layers[0].texts[1].key,
        TextKey(short.cache_key(1.0)),
        "fits: unchanged"
    );
}

fn tmp_width(text: &mut FakeText, spec: &lune_ui_core::text::Spec) -> f32 {
    use lune_ui_core::text::TextSystem;
    text.measure(spec, 1.0).0
}

#[test]
fn a_narrow_rect_cuts_the_text_to_what_is_left() {
    let theme = DARK;
    let narrow = Rect::new(10.0, 6.0, 100.0, 40.0);
    let mut h = UiHarness::new(WIN);
    let (scene, _, _) = label_frame(&mut h, narrow, "a-rather-long-branch-name");
    let gw = tmp_width(
        &mut FakeText,
        &icon::glyph(icon::SYNC, LABEL_GLYPH, theme.fg_subtle),
    );
    let room = 100.0 - 2.0 * LABEL_PAD_X - gw - LABEL_GAP;
    let full = label_spec(&theme, "a-rather-long-branch-name", theme.fg_subtle);
    let want = elide(&mut FakeText, &full, 1.0, room);
    assert_eq!(scene.layers[0].texts[1].key, TextKey(want.cache_key(1.0)));
}

#[test]
fn the_label_button_runs_once_and_has_a_node_named_by_the_owner() {
    let mut h = UiHarness::new(WIN);
    label_frame(&mut h, LABEL_RECT, "main");
    let (x, y) = centre(LABEL_RECT);
    h.input.pointer_to(x, y).click(PointerButton::Primary);
    h.at_ms(10);
    let (_, clicked, tree) = label_frame(&mut h, LABEL_RECT, "main");
    assert!(clicked);
    let node = tree.get(id("branch")).expect("node");
    assert_eq!((node.role, node.name.as_str()), (Role::Button, "Branch"));
    assert!(node.actions.contains(Action::Default));
    h.at_ms(20);
    assert!(!label_frame(&mut h, LABEL_RECT, "main").1);
}

#[test]
fn label_width_fits_the_content_and_caps_the_text() {
    let mut text = FakeText;
    let win = Rect::new(0.0, 0.0, 900.0, 200.0);
    let mut p = lune_ui_core::paint::Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    let short = label_width(&mut p, icon::SYNC, "main");
    let long = label_width(&mut p, icon::SYNC, &"x".repeat(500));
    let cap = 2.0 * LABEL_PAD_X + 7.0 + LABEL_GAP + LABEL_MAX_WIDTH;
    assert!(short < 100.0);
    assert!((long - cap).abs() < 1e-3, "{long} vs {cap}");
}

// ---- Nodes ----

#[test]
fn the_bar_is_a_toolbar_node_holding_the_named_actions() {
    let mut h = UiHarness::new(WIN);
    let out = frame(&mut h, true, true);
    let bar = out.tree.get(bar_id()).expect("toolbar node");
    assert_eq!(
        (bar.role, bar.name.as_str()),
        (Role::Toolbar, "Main toolbar")
    );
    assert_eq!(bar.bounds, BAR);
    assert_eq!(bar.children.len(), 4);
    for (i, name) in NAMES.iter().enumerate() {
        let node = out.tree.get(id(&name.to_lowercase())).expect("action");
        assert_eq!((node.role, node.name.as_str()), (Role::Button, *name));
        assert_eq!(node.parent, Some(bar_id()));
        assert_eq!(node.bounds, action_rect(i));
        assert!(node.states.contains(State::Focusable));
    }
}

/// The label's text: UI face at `font_m`.
fn label_spec(
    theme: &lune_ui_core::theme::Theme,
    text: &str,
    color: lune_ui_core::color::Rgba,
) -> Spec {
    Spec::plain(text, Style::ui(theme.font_m, color))
}

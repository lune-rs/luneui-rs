//! The caption buttons through the `Ui` (`platform.window`, D240): the Painter form's look at rest,
//! hover and the close fill, the click, the restore glyph, the 92 device px at scale 2, the hover
//! motion under every tier, the three nodes with their names, the default action, and that they
//! are no tab stops.

use lune_ui_core::access::{AccessAction, Action, Role};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::strings::default_text;
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::HOVER;
use lune_ui_core::widgets::caption::{CaptionButton, CaptionOptions};
use lune_ui_core::window;

const WIN: (f32, f32) = (400.0, 300.0);
const WINDOW: Rect = Rect::new(0.0, 0.0, WIN.0, WIN.1);
const HEIGHT: f32 = 32.0;
/// Pointer positions over the three buttons (46 wide, right-aligned in 400: 262, 308, 354).
const OVER: [(f32, f32); 3] = [(285.0, 16.0), (331.0, 16.0), (377.0, 16.0)];

fn opts(maximized: bool, active: bool) -> CaptionOptions {
    CaptionOptions {
        height: HEIGHT,
        maximized,
        active,
    }
}

fn id(name: &str) -> WidgetId {
    WidgetId::ROOT.with("cap").with(name)
}

fn frame(
    h: &mut UiHarness,
    o: CaptionOptions,
) -> (
    Scene,
    Option<CaptionButton>,
    Repaint,
    lune_ui_core::input::FrameOutput,
) {
    let mut hit = None;
    let (scene, out) = h.frame(|ui| hit = ui.caption_buttons("cap", WINDOW, o));
    let repaint = out.repaint;
    (scene, hit, repaint, out)
}

/// What the Painter form draws, with button `hovered` (0, 1, 2) under the pointer, if any.
fn painter_scene(maximized: bool, hovered: Option<u8>) -> Scene {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, WINDOW, &mut text, hovered, DARK);
    window::caption_buttons(&mut p, WINDOW, HEIGHT, maximized, [0, 1, 2]);
    p.scene
}

/// The Ui form's scene once settled, the pointer over button `over` if any.
fn settled(maximized: bool, over: Option<usize>) -> Scene {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, opts(maximized, true));
    if let Some(k) = over {
        h.input.pointer_to(OVER[k].0, OVER[k].1);
        h.at_ms(10);
        frame(&mut h, opts(maximized, true));
    }
    h.at_ms(1000);
    let (scene, _, repaint, _) = frame(&mut h, opts(maximized, true));
    assert_eq!(repaint, Repaint::Idle);
    scene
}

#[test]
fn at_rest_and_active_the_scene_is_the_painter_forms() {
    for maximized in [false, true] {
        assert_eq!(
            settled(maximized, None),
            painter_scene(maximized, None),
            "maximized={maximized}"
        );
    }
}

#[test]
fn a_settled_hover_is_the_painter_forms_hover() {
    for k in 0..3u8 {
        assert_eq!(
            settled(false, Some(usize::from(k))),
            painter_scene(false, Some(k)),
            "button {k}"
        );
    }
}

#[test]
fn a_pointer_over_close_fills_caption_close_hover() {
    let scene = settled(false, Some(2));
    let fills = &scene.layers[0].geometry.under;
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].color, premul(DARK.caption_close_hover));
    assert_eq!(fills[0].rect, [354.0, 0.0, 46.0, 32.0]);
    // Minimize and maximize fill `surface_hover` instead.
    for k in 0..2 {
        let fills = settled(false, Some(k)).layers[0].geometry.under.clone();
        assert_eq!(fills.len(), 1, "button {k}");
        assert_eq!(fills[0].color, premul(DARK.surface_hover), "button {k}");
    }
}

#[test]
fn a_click_returns_the_button_once() {
    for (k, want) in [
        CaptionButton::Minimize,
        CaptionButton::Maximize,
        CaptionButton::Close,
    ]
    .into_iter()
    .enumerate()
    {
        let mut h = UiHarness::new(WIN);
        frame(&mut h, opts(false, true));
        h.input.pointer_to(OVER[k].0, OVER[k].1);
        h.input.click(PointerButton::Primary);
        h.at_ms(10);
        let (_, hit, _, _) = frame(&mut h, opts(false, true));
        assert_eq!(hit, Some(want));
        h.at_ms(20);
        let (_, hit, _, _) = frame(&mut h, opts(false, true));
        assert_eq!(hit, None, "{want:?} fires once");
    }
}

#[test]
fn a_press_takes_no_focus_and_tab_finds_no_stop() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, opts(false, true));
    h.input.pointer_to(OVER[2].0, OVER[2].1);
    h.input.click(PointerButton::Primary);
    h.at_ms(10);
    let (_, _, _, out) = frame(&mut h, opts(false, true));
    assert_eq!(out.focused, None);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(20);
    let (_, _, _, out) = frame(&mut h, opts(false, true));
    assert_eq!(out.focused, None, "the caption buttons are no tab stops");
    for name in ["minimize", "maximize", "close"] {
        let node = out.tree.get(id(name)).expect("node");
        assert!(!node.states.contains(lune_ui_core::access::State::Focusable));
    }
}

#[test]
fn maximized_draws_the_restore_glyph() {
    let max = settled(true, None);
    let normal = settled(false, None);
    assert_ne!(max, normal);
    assert_eq!(max, painter_scene(true, None));
    // The glyph is the only difference: the middle text item.
    let (a, b) = (&max.layers[0].texts, &normal.layers[0].texts);
    assert_eq!(a.len(), 3);
    assert_eq!((a[0], a[2]), (b[0], b[2]));
    assert_ne!(a[1], b[1]);
    // And that glyph is the restore codicon at 10 px.
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, WINDOW, &mut text, None, DARK);
    p.text_centered(
        &icon::glyph(icon::CHROME_RESTORE, 10.0, DARK.fg_primary),
        Rect::new(308.0, 0.0, 46.0, 32.0),
    );
    assert_eq!(a[1], p.scene.layers[0].texts[0]);
}

#[test]
fn each_button_is_92_device_px_wide_at_scale_2() {
    for (k, over) in OVER.into_iter().enumerate() {
        let mut h = UiHarness::new(WIN);
        h.input.set_scale(2.0);
        frame(&mut h, opts(false, true));
        h.input.pointer_to(over.0, over.1);
        h.at_ms(10);
        frame(&mut h, opts(false, true));
        h.at_ms(1000);
        let (scene, _, _, _) = frame(&mut h, opts(false, true));
        let fill = scene.layers[0].geometry.under[0];
        assert_eq!(fill.rect[2], 92.0, "button {k}");
        assert_eq!(fill.rect[3], 64.0, "button {k} height");
        assert_eq!(fill.rect[0], (262.0 + 46.0 * k as f32) * 2.0, "button {k}");
    }
}

#[test]
fn the_hover_fill_passes_motion_conformance_in_every_tier() {
    let rest = |c: [u8; 4]| [c[0], c[1], c[2], 0].map(f32::from);
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (name, to) in [
            ("surface", DARK.surface_hover),
            ("close", DARK.caption_close_hover),
        ] {
            conformance(&HOVER, &DARK.motion, tier, rest(to), to.map(f32::from))
                .unwrap_or_else(|e| panic!("{name} fill {tier:?}: {e}"));
        }
        conformance(
            &HOVER,
            &DARK.motion,
            tier,
            DARK.fg_primary.map(f32::from),
            DARK.caption_close_fg.map(f32::from),
        )
        .unwrap_or_else(|e| panic!("close glyph {tier:?}: {e}"));
    }
    // Reduced keeps the tween; Off snaps.
    assert!(DARK.motion.resolve(&HOVER, MotionTier::Reduced).is_some());
    assert!(DARK.motion.resolve(&HOVER, MotionTier::Off).is_none());
}

#[test]
fn the_fill_fades_in_over_instant_and_off_snaps() {
    let fill_alpha = |tier: MotionTier, ms: u64| {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        frame(&mut h, opts(false, true));
        h.input.pointer_to(OVER[2].0, OVER[2].1);
        h.at_ms(0);
        frame(&mut h, opts(false, true));
        h.at_ms(ms);
        let (scene, _, repaint, _) = frame(&mut h, opts(false, true));
        let alpha = scene.layers[0]
            .geometry
            .under
            .first()
            .map_or(0.0, |s| s.color[3]);
        (alpha, repaint)
    };
    let full = premul(DARK.caption_close_hover)[3];
    // Linear over 120 ms: half way at 60 ms.
    let (mid, repaint) = fill_alpha(MotionTier::Full, 60);
    assert!((mid - full / 2.0).abs() < 0.02, "mid {mid} of {full}");
    assert_eq!(repaint, Repaint::Now);
    let (done, repaint) = fill_alpha(MotionTier::Full, 200);
    assert_eq!(done, full);
    assert_eq!(repaint, Repaint::Idle);
    // Reduced keeps the tween.
    let (mid, _) = fill_alpha(MotionTier::Reduced, 60);
    assert!(mid > 0.0 && mid < full);
    // Off snaps at the first frame.
    let (snap, repaint) = fill_alpha(MotionTier::Off, 0);
    assert_eq!(snap, full);
    assert_eq!(repaint, Repaint::Idle);
}

#[test]
fn an_inactive_window_draws_its_glyphs_in_fg_quaternary() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, opts(false, false));
    h.at_ms(1000);
    let (scene, _, _, _) = frame(&mut h, opts(false, false));
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, WINDOW, &mut text, None, DARK);
    for (k, g) in [
        icon::CHROME_MINIMIZE,
        icon::CHROME_MAXIMIZE,
        icon::CHROME_CLOSE,
    ]
    .into_iter()
    .enumerate()
    {
        p.text_centered(
            &icon::glyph(g, 10.0, DARK.fg_quaternary),
            Rect::new(262.0 + 46.0 * k as f32, 0.0, 46.0, 32.0),
        );
    }
    assert_eq!(scene, p.scene);
    assert_ne!(scene, painter_scene(false, None), "not fg_primary");
}

#[test]
fn flipping_active_cuts_the_glyphs_to_the_new_colour_on_that_frame() {
    // The window fill hard-cuts on a focus change (`platform/window.md`: "stays a snap"), so the
    // glyphs do too: the flip frame is the new scene and asks for no repaint.
    let inactive = {
        let mut h = UiHarness::new(WIN);
        frame(&mut h, opts(false, false));
        h.at_ms(1000);
        frame(&mut h, opts(false, false)).0
    };
    let mut h = UiHarness::new(WIN);
    frame(&mut h, opts(false, true));
    h.at_ms(1000);
    let (rest, _, repaint, _) = frame(&mut h, opts(false, true));
    assert_eq!(repaint, Repaint::Idle);
    assert_eq!(rest, painter_scene(false, None));
    h.at_ms(2000);
    let (flipped, _, repaint, _) = frame(&mut h, opts(false, false));
    assert_eq!(flipped, inactive, "the flip frame holds fq glyphs");
    assert_eq!(repaint, Repaint::Idle);
    // And back.
    h.at_ms(3000);
    let (back, _, repaint, _) = frame(&mut h, opts(false, true));
    assert_eq!(back, rest);
    assert_eq!(repaint, Repaint::Idle);
}

#[test]
fn the_nodes_are_buttons_with_their_names_and_the_default_action_clicks() {
    for maximized in [false, true] {
        let mut h = UiHarness::new(WIN);
        frame(&mut h, opts(maximized, true));
        let (_, _, _, out) = frame(&mut h, opts(maximized, true));
        let names = [
            ("minimize", "Minimize"),
            ("maximize", if maximized { "Restore" } else { "Maximize" }),
            ("close", "Close"),
        ];
        for (k, (key, name)) in names.into_iter().enumerate() {
            let node = out.tree.get(id(key)).unwrap_or_else(|| panic!("{key}"));
            assert_eq!(node.role, Role::Button);
            assert_eq!(node.name, name);
            assert!(node.actions.contains(Action::Default));
            assert_eq!(
                node.bounds,
                Rect::new(262.0 + 46.0 * k as f32, 0.0, 46.0, 32.0)
            );
        }
    }
    // The default action runs the button, once.
    for (key, want) in [
        ("minimize", CaptionButton::Minimize),
        ("maximize", CaptionButton::Maximize),
        ("close", CaptionButton::Close),
    ] {
        let mut h = UiHarness::new(WIN);
        frame(&mut h, opts(false, true));
        h.input.access(AccessAction {
            target: id(key),
            action: Action::Default,
            value: None,
        });
        h.at_ms(10);
        let (_, hit, _, _) = frame(&mut h, opts(false, true));
        assert_eq!(hit, Some(want), "{key}");
        h.at_ms(20);
        let (_, hit, _, _) = frame(&mut h, opts(false, true));
        assert_eq!(hit, None, "{key} once");
    }
}

#[test]
fn the_names_come_from_the_string_table_and_default_to_english() {
    for (key, text) in [
        ("luneui.window.minimize", "Minimize"),
        ("luneui.window.maximize", "Maximize"),
        ("luneui.window.restore", "Restore"),
        ("luneui.window.close", "Close"),
    ] {
        assert_eq!(default_text(key), Some(text), "{key}");
    }
    let mut h = UiHarness::new(WIN);
    h.strings
        .insert("luneui.window.close", "Schlie\u{df}en")
        .expect("key");
    h.strings
        .insert("luneui.window.restore", "Wiederherstellen")
        .expect("key");
    let (_, _, _, out) = frame(&mut h, opts(true, true));
    assert_eq!(
        out.tree.get(id("close")).expect("node").name,
        "Schlie\u{df}en"
    );
    assert_eq!(
        out.tree.get(id("maximize")).expect("node").name,
        "Wiederherstellen"
    );
}

#[test]
fn a_disabled_scope_leaves_no_violation_and_no_click() {
    let mut h = UiHarness::new(WIN);
    let run = |h: &mut UiHarness| {
        let mut hit = None;
        let (_, out) = h.frame(|ui| {
            ui.with_enabled(false, |ui| {
                hit = ui.caption_buttons("cap", WINDOW, opts(false, true));
            });
        });
        (hit, out)
    };
    run(&mut h);
    h.input.pointer_to(OVER[2].0, OVER[2].1);
    h.input.click(PointerButton::Primary);
    h.at_ms(10);
    let (hit, out) = run(&mut h);
    assert_eq!(hit, None);
    for name in ["minimize", "maximize", "close"] {
        assert_eq!(disabled_violations(&out.tree, id(name)), [], "{name}");
    }
}

#[test]
fn the_painter_form_keeps_its_geometry() {
    // The shared constants: 46 wide, a 10 px glyph.
    assert_eq!(window::CAPTION_BUTTON_WIDTH, 46.0);
    assert_eq!(window::CAPTION_GLYPH_SIZE, 10.0);
}

#[test]
fn the_buttons_are_placed_from_the_windows_own_origin() {
    use lune_ui_core::testing::hit_regions;
    let win = Rect::new(100.0, 40.0, 800.0, 600.0);
    // The Ui form: the three hit regions, right-aligned along the top of `win`.
    let mut h = UiHarness::new((1000.0, 700.0));
    h.frame(|ui| {
        ui.caption_buttons("cap", win, opts(false, true));
    });
    let regions = hit_regions(&h.store);
    assert_eq!(regions.len(), 3);
    let close = regions[2].rect;
    assert_eq!((close.right(), close.y), (900.0, 40.0));
    assert_eq!(regions[0].rect.x, 900.0 - 3.0 * 46.0);
    // A press on the close button at its place is a click on it.
    let mut hit = None;
    h.input
        .pointer_to(880.0, 56.0)
        .press(PointerButton::Primary);
    h.at_ms(10)
        .frame(|ui| hit = ui.caption_buttons("cap", win, opts(false, true)));
    h.input.release(PointerButton::Primary);
    h.at_ms(20)
        .frame(|ui| hit = ui.caption_buttons("cap", win, opts(false, true)));
    assert_eq!(hit, Some(CaptionButton::Close));

    // The Painter form shares the placement.
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(
        1.0,
        Rect::new(0.0, 0.0, 1000.0, 700.0),
        &mut text,
        None,
        DARK,
    );
    window::caption_buttons(&mut p, win, HEIGHT, false, [0, 1, 2]);
    let rects: Vec<Rect> = p.hits.iter().map(|(r, _)| *r).collect();
    assert_eq!(rects, regions.iter().map(|r| r.rect).collect::<Vec<_>>());
    // At the origin the known answers stand (the 262, 308, 354 of `OVER`).
    let mut h = UiHarness::new(WIN);
    h.frame(|ui| {
        ui.caption_buttons("cap", WINDOW, opts(false, true));
    });
    let xs: Vec<f32> = hit_regions(&h.store).iter().map(|r| r.rect.x).collect();
    assert_eq!(xs, [262.0, 308.0, 354.0]);
}

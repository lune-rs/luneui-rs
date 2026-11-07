//! The side panel host (`layout.side-panel` acceptance): the glass card and the solid inactive
//! fill, the radius and padding probes at scale 2, exactly one panel's nodes, hidden panels that
//! keep their scroll offset and text edit across 600 undrawn frames, the show motion in three
//! tiers (the first frame does not animate, a show in flight restarts from its look), a hide that
//! snaps, no focus moved by a show, and the disabled walk.

use lune_ui_core::GlassLayer;
use lune_ui_core::access::{Role, SemanticTree, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{Easing, MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style, TextKey};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::field::FieldOptions;
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};
use lune_ui_core::widgets::side_panel::{
    PAD, RADIUS, RING, SHOW_OPACITY, SHOW_SLIDE, SHOW_SLIDE_FROM, SidePanelOptions, content_rect,
};

const WIN: (f32, f32) = (600.0, 500.0);
const HOST: Rect = Rect::new(20.0, 20.0, 300.0, 400.0);
const KEYS: [u64; 3] = [1, 2, 3];
const MODERATE: u64 = 350;

thread_local! {
    /// The field's text, the app's: it lives outside the host.
    static TEXT: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}
const GLASS: GlassLayer = GlassLayer {
    tint: [10, 20, 30, 255],
    tint_opacity: 0.8,
    material_opacity: 0.7,
};
const UNDER: [f32; 4] = [0.05, 0.06, 0.07, 1.0];

fn host_id() -> WidgetId {
    WidgetId::ROOT.with("host")
}

/// The id of `key` inside panel `panel`: under the host's keep-alive scope.
fn in_panel(panel: u64, key: &str) -> WidgetId {
    WidgetId::ROOT.with(("host", panel)).with(key)
}

#[derive(Clone, Copy)]
struct Cfg {
    panel: u64,
    active: bool,
    enabled: bool,
    keys: &'static [u64],
}

impl Cfg {
    fn new(panel: u64) -> Cfg {
        Cfg {
            panel,
            active: true,
            enabled: true,
            keys: &KEYS,
        }
    }
}

struct Out {
    scene: Scene,
    out: FrameOutput,
    inner: Option<Rect>,
    offset: Option<f64>,
    drawn: Vec<u64>,
}

fn line_style(alpha: u8) -> Style {
    Style::ui(13.0, [255, 255, 255, alpha])
}

fn line(panel: u64) -> &'static str {
    ["", "Panel one", "Panel two", "Panel three"][panel as usize]
}

fn frame(h: &mut UiHarness, c: &Cfg) -> Out {
    let mut inner = None;
    let mut offset = None;
    let mut drawn = Vec::new();
    let (scene, out) = h.frame(|ui| {
        let opts = SidePanelOptions {
            panel: c.panel,
            keys: c.keys,
            active: c.active,
            glass: GLASS,
            under: UNDER,
            name: line(c.panel),
        };
        ui.with_enabled(c.enabled, |ui| {
            ui.side_panel("host", HOST, &opts, |ui, rect| {
                drawn.push(c.panel);
                inner = Some(rect);
                ui.painter()
                    .text(&Spec::plain(line(c.panel), line_style(255)), rect.x, rect.y);
                match c.panel {
                    1 => {
                        TEXT.with(|t| {
                            ui.text_field_with(
                                "f",
                                Rect::new(rect.x, rect.y + 20.0, rect.w, 32.0),
                                &mut t.borrow_mut(),
                                &FieldOptions::default(),
                            );
                        });
                        let r = ui.scroll_area(
                            "list",
                            Rect::new(rect.x, rect.y + 60.0, rect.w, 200.0),
                            &ScrollOptions::default(),
                            |_, _| ((), Extent::new(f64::from(rect.w), 1000.0)),
                        );
                        offset = Some(r.offset.1);
                    }
                    2 => {
                        ui.button(
                            "b2",
                            Rect::new(rect.x, rect.y + 20.0, 80.0, 32.0),
                            "Two",
                            ButtonKind::Secondary,
                        );
                    }
                    _ => {}
                }
            });
        });
    });
    Out {
        scene,
        out,
        inner,
        offset,
        drawn,
    }
}

fn settled(h: &mut UiHarness, c: &Cfg) -> Out {
    frame(h, c);
    h.at_ms(10);
    frame(h, c);
    h.at_ms(5000);
    frame(h, c)
}

fn texts(scene: &Scene) -> Vec<lune_ui_core::scene::TextItem> {
    scene.layers[0].texts.clone()
}

fn key_of(spec: &Spec, scale: f32) -> TextKey {
    TextKey(spec.cache_key(scale))
}

/// The opacity (0 to 255) panel `panel`'s line is drawn at, found by trying each.
fn alpha_of(scene: &Scene, panel: u64) -> Option<u8> {
    let keys: Vec<TextKey> = texts(scene).iter().map(|t| t.key).collect();
    (0..=255u8).find(|a| keys.contains(&key_of(&Spec::plain(line(panel), line_style(*a)), 1.0)))
}

fn x_of(scene: &Scene, panel: u64) -> f32 {
    let a = alpha_of(scene, panel).expect("the line is drawn");
    let want = key_of(&Spec::plain(line(panel), line_style(a)), 1.0);
    texts(scene)
        .iter()
        .find(|t| t.key == want)
        .map(|t| t.x)
        .expect("text")
}

#[test]
fn the_card_is_the_acrylic_at_radius_8_with_the_given_under_and_content_inside_ring_and_10() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new(1));
    let g = &out.scene.layers[0].geometry;
    assert_eq!(g.acrylic.len(), 1);
    let a = g.acrylic[0].shape;
    assert_eq!(a.rect, [20.0, 20.0, 300.0, 400.0]);
    assert_eq!(a.radii, [RADIUS; 4]);
    assert_eq!(a.color, GLASS.premultiplied());
    assert_eq!(a.under, UNDER);
    assert_eq!(RADIUS, 8.0);
    assert_eq!((PAD, RING), (10.0, 1.0));
    assert_eq!(out.inner, Some(Rect::new(31.0, 31.0, 278.0, 378.0)));
    assert_eq!(content_rect(HOST), Rect::new(31.0, 31.0, 278.0, 378.0));
    assert_eq!(x_of(&out.scene, 1), 31.0);
    assert_eq!(out.out.repaint, Repaint::Idle);
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let out = settled(&mut h, &Cfg::new(1));
    let a = out.scene.layers[0].geometry.acrylic[0].shape;
    // Radius 16 px; the content starts 22 px in (the 1 px ring and the padding of 10, doubled),
    // and the ring is `border`, 2 px.
    assert_eq!(a.radii, [16.0; 4]);
    let t = texts(&out.scene)
        .into_iter()
        .find(|t| t.key == key_of(&Spec::plain(line(1), line_style(255)), 2.0))
        .expect("line");
    assert_eq!(
        (t.x - 20.0, t.y - 20.0),
        (11.0, 11.0),
        "logical 11 px inside"
    );
    let ring = out.scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|s| s.params[0] == 1.0)
        .expect("the ring");
    assert_eq!(ring.params[1], 2.0, "1 logical px is 2 physical");
    assert_eq!(ring.rect, a.rect);
    assert_eq!(ring.color, premul(DARK.border));
}

#[test]
fn inactive_draws_the_fallback_fill_and_no_acrylic() {
    let mut h = UiHarness::new(WIN);
    let out = settled(
        &mut h,
        &Cfg {
            active: false,
            ..Cfg::new(1)
        },
    );
    let g = &out.scene.layers[0].geometry;
    assert!(g.acrylic.is_empty());
    let fill = g
        .under
        .iter()
        .find(|s| s.rect == [20.0, 20.0, 300.0, 400.0])
        .expect("the card fill");
    assert_eq!(fill.color, premul(DARK.window_fallback));
    assert_eq!(fill.radii, [RADIUS; 4]);
}

#[test]
fn exactly_one_panels_nodes_exist_and_hidden_panels_emit_none() {
    let mut h = UiHarness::new(WIN);
    let one = settled(&mut h, &Cfg::new(1));
    assert_eq!(one.drawn, [1]);
    let group = one.out.tree.get(host_id()).expect("group");
    assert_eq!(
        (group.role, group.name.as_str()),
        (Role::Group, "Panel one")
    );
    assert!(one.out.tree.get(in_panel(1, "f")).is_some());
    assert!(one.out.tree.get(in_panel(2, "b2")).is_none());
    // Every node of the frame is the host or inside it.
    for n in &one.out.tree.nodes {
        assert!(n.id == host_id() || n.parent.is_some(), "{:?}", n.role);
    }
    h.at_ms(5100);
    let two = frame(&mut h, &Cfg::new(2));
    h.at_ms(5200);
    let two = {
        let _ = two;
        frame(&mut h, &Cfg::new(2))
    };
    assert_eq!(two.drawn, [2]);
    assert!(two.out.tree.get(in_panel(2, "b2")).is_some());
    assert!(two.out.tree.get(in_panel(1, "f")).is_none());
    assert!(two.out.tree.get(in_panel(1, "list")).is_none());
    assert_eq!(
        two.out.tree.get(host_id()).expect("group").name,
        "Panel two"
    );
    // A panel nobody lists is still drawn when it is shown.
    let three = frame(
        &mut h,
        &Cfg {
            keys: &[],
            ..Cfg::new(3)
        },
    );
    assert_eq!(three.drawn, [3]);
}

fn typed(h: &mut UiHarness, s: &str) {
    for c in s.chars() {
        let t = c.to_string();
        h.input.key_down(Key::Character(t.clone()), Some(&t));
    }
}

#[test]
fn a_hidden_panels_scroll_offset_and_text_edit_survive_600_undrawn_frames() {
    let mut h = UiHarness::new(WIN);
    let one = Cfg::new(1);
    frame(&mut h, &one);
    // Focus the field by a click, type and select the last character.
    let f = in_panel(1, "f");
    let _ = f;
    h.input
        .pointer_to(100.0, 62.0)
        .click(PointerButton::Primary);
    h.at_ms(10);
    frame(&mut h, &one);
    typed(&mut h, "abc");
    h.at_ms(20);
    frame(&mut h, &one);
    h.input.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    h.input.key_down(Key::Named(NamedKey::ArrowLeft), None);
    h.input.set_modifiers(Modifiers::default());
    h.at_ms(30);
    frame(&mut h, &one);
    // Scroll the list by the wheel (the pointer over it) and let the tween settle.
    h.input.pointer_to(100.0, 150.0).scroll_lines(0.0, -3.0);
    h.at_ms(40);
    frame(&mut h, &one);
    h.at_ms(3000);
    let before = frame(&mut h, &one);
    let offset = before.offset.expect("list");
    assert!(offset > 0.0, "scrolled: {offset}");
    let selection = |o: &Out| {
        o.out
            .tree
            .nodes
            .iter()
            .find_map(|n| n.text.as_ref())
            .expect("the field's text")
            .selection
    };
    assert_eq!(selection(&before), (2, 3));

    // Panel 2 for 600 frames: panel 1 is not drawn.
    for i in 0..600u64 {
        h.at_ms(3100 + i * 16);
        let out = frame(&mut h, &Cfg::new(2));
        assert_eq!(out.drawn, [2]);
    }
    h.at_ms(20_000);
    frame(&mut h, &one);
    h.at_ms(21_000);
    let back = frame(&mut h, &one);
    assert_eq!(back.offset, Some(offset), "the scroll offset");
    assert_eq!(selection(&back), (2, 3), "the caret and selection");
}

// ---- the show ----

/// A show of panel 2 at 6000 ms, sampled `after` ms later.
fn shown(tier: MotionTier, after: u64) -> Out {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    settled(&mut h, &Cfg::new(1));
    h.at_ms(6000);
    frame(&mut h, &Cfg::new(2));
    h.at_ms(6000 + after);
    frame(&mut h, &Cfg::new(2))
}

#[test]
fn the_first_frame_of_a_host_does_not_animate() {
    let mut h = UiHarness::new(WIN);
    let out = frame(&mut h, &Cfg::new(2));
    assert_eq!(alpha_of(&out.scene, 2), Some(255));
    assert_eq!(x_of(&out.scene, 2), 31.0);
    assert_eq!(out.out.repaint, Repaint::Idle);
    // And a host that starts on another panel animates nothing either.
    let mut h = UiHarness::new(WIN);
    let out = frame(&mut h, &Cfg::new(3));
    assert_eq!(out.out.repaint, Repaint::Idle);
}

#[test]
fn a_show_fades_the_panel_in_and_slides_it_from_the_left_and_leaves_the_card() {
    let mut h = UiHarness::new(WIN);
    let rest = settled(&mut h, &Cfg::new(1));
    h.at_ms(6000);
    let first = frame(&mut h, &Cfg::new(2));
    assert_eq!(first.out.repaint, Repaint::Now);
    assert_eq!(alpha_of(&first.scene, 2), Some(0));
    assert_eq!(x_of(&first.scene, 2), 31.0 + SHOW_SLIDE_FROM);
    // The card does not move.
    let card = |o: &Out| o.scene.layers[0].geometry.acrylic[0].shape.rect;
    assert_eq!(card(&first), card(&rest));
    // Half way: quad-out opacity, sine-in-out slide.
    h.at_ms(6175);
    let mid = frame(&mut h, &Cfg::new(2));
    let p = 175.0 / MODERATE as f32;
    let want_a = (Easing::QuadOut.eval(p) * 255.0).round();
    let got_a = f32::from(alpha_of(&mid.scene, 2).expect("faded"));
    assert!((got_a - want_a).abs() <= 1.0, "{got_a} vs {want_a}");
    let want_x = 31.0 + SHOW_SLIDE_FROM * (1.0 - Easing::SineInOut.eval(p));
    assert!((x_of(&mid.scene, 2) - want_x).abs() < 0.01);
    assert_eq!(mid.out.repaint, Repaint::Now);
    // Settled by 350 ms.
    h.at_ms(6000 + MODERATE);
    let done = frame(&mut h, &Cfg::new(2));
    assert_eq!(alpha_of(&done.scene, 2), Some(255));
    assert_eq!(x_of(&done.scene, 2), 31.0);
    assert_eq!(done.out.repaint, Repaint::Idle);
}

#[test]
fn a_hide_snaps_the_old_panel_is_gone_on_the_first_frame_of_the_change() {
    let mut h = UiHarness::new(WIN);
    settled(&mut h, &Cfg::new(1));
    h.at_ms(6000);
    let first = frame(&mut h, &Cfg::new(2));
    assert_eq!(alpha_of(&first.scene, 1), None, "panel one is not drawn");
    assert_eq!(first.drawn, [2]);
}

#[test]
fn show_conforms_in_three_tiers() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&SHOW_OPACITY, &theme.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("opacity {tier:?}: {e}"));
        conformance(&SHOW_SLIDE, &theme.motion, tier, SHOW_SLIDE_FROM, 0.0_f32)
            .unwrap_or_else(|e| panic!("slide {tier:?}: {e}"));
    }
    let full = theme
        .motion
        .resolve(&SHOW_OPACITY, MotionTier::Full)
        .expect("full");
    assert_eq!(full.duration.as_millis(), 350, "moderate");
    assert_eq!(full.easing, Easing::QuadOut);
    let slide = theme
        .motion
        .resolve(&SHOW_SLIDE, MotionTier::Full)
        .expect("full");
    assert_eq!(slide.duration.as_millis(), 350);
    assert_eq!(slide.easing, Easing::SineInOut);
    assert!(
        theme
            .motion
            .resolve(&SHOW_OPACITY, MotionTier::Reduced)
            .is_some()
    );
    assert!(
        theme
            .motion
            .resolve(&SHOW_SLIDE, MotionTier::Reduced)
            .is_none()
    );
    assert!(
        theme
            .motion
            .resolve(&SHOW_OPACITY, MotionTier::Off)
            .is_none()
    );
    assert!(theme.motion.resolve(&SHOW_SLIDE, MotionTier::Off).is_none());
}

#[test]
fn reduced_fades_the_opacity_only_and_off_snaps() {
    let r = shown(MotionTier::Reduced, 0);
    assert_eq!(x_of(&r.scene, 2), 31.0, "no slide");
    assert_eq!(alpha_of(&r.scene, 2), Some(0));
    assert_eq!(r.out.repaint, Repaint::Now);
    let r = shown(MotionTier::Reduced, 60);
    let a = alpha_of(&r.scene, 2).expect("faded");
    assert!(a > 0 && a < 255, "{a}");
    assert_eq!(x_of(&r.scene, 2), 31.0);
    let r = shown(MotionTier::Reduced, 400);
    assert_eq!(alpha_of(&r.scene, 2), Some(255));
    assert_eq!(r.out.repaint, Repaint::Idle);
    let o = shown(MotionTier::Off, 0);
    assert_eq!(alpha_of(&o.scene, 2), Some(255));
    assert_eq!(x_of(&o.scene, 2), 31.0);
    assert_eq!(o.out.repaint, Repaint::Idle);
}

#[test]
fn a_show_in_flight_restarts_from_the_look_it_has() {
    let mut h = UiHarness::new(WIN);
    settled(&mut h, &Cfg::new(1));
    h.at_ms(6000);
    frame(&mut h, &Cfg::new(2));
    h.at_ms(6150);
    let mid = frame(&mut h, &Cfg::new(2));
    let (a_mid, x_mid) = (alpha_of(&mid.scene, 2).expect("faded"), x_of(&mid.scene, 2));
    assert!(a_mid > 0 && a_mid < 255);
    // Panel 3 one frame later starts where panel 2 was, not at 0.
    h.at_ms(6150);
    let again = frame(&mut h, &Cfg::new(3));
    assert_eq!(alpha_of(&again.scene, 3), Some(a_mid));
    assert!((x_of(&again.scene, 3) - x_mid).abs() < 0.01);
    h.at_ms(6150 + MODERATE);
    let done = frame(&mut h, &Cfg::new(3));
    assert_eq!(alpha_of(&done.scene, 3), Some(255));
    assert_eq!(done.out.repaint, Repaint::Idle);
}

#[test]
fn a_show_moves_no_focus() {
    let mut h = UiHarness::new(WIN);
    settled(&mut h, &Cfg::new(2));
    h.input.pointer_to(50.0, 60.0).click(PointerButton::Primary);
    h.at_ms(5100);
    let before = frame(&mut h, &Cfg::new(2));
    assert_eq!(before.out.focused, Some(in_panel(2, "b2")));
    h.at_ms(5200);
    let after = frame(&mut h, &Cfg::new(1));
    assert_ne!(
        after.out.focused,
        Some(in_panel(1, "f")),
        "the field takes no focus"
    );
    h.at_ms(5300);
    let later = frame(&mut h, &Cfg::new(1));
    assert_ne!(later.out.focused, Some(in_panel(1, "f")));
}

#[test]
fn a_disabled_host_is_disabled_all_the_way_down_and_consumes_no_key() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        enabled: false,
        ..Cfg::new(2)
    };
    let out = settled(&mut h, &c);
    let bad = disabled_violations(&out.out.tree, host_id());
    assert!(bad.is_empty(), "{bad:?}");
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(5100);
    assert_eq!(frame(&mut h, &c).out.focused, None);
    // An enabled host is not marked.
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new(2));
    assert!(
        !out.out
            .tree
            .get(host_id())
            .expect("group")
            .states
            .contains(State::Disabled)
    );
    for k in [NamedKey::Enter, NamedKey::Space, NamedKey::ArrowDown] {
        let mut h = UiHarness::new(WIN);
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        settled(&mut h, &c);
        h.input.key_down(Key::Named(k), None);
        h.at_ms(5100);
        assert_eq!(frame(&mut h, &c).out.fired.len(), 1, "{k:?}");
    }
}

#[test]
fn the_host_draws_only_on_the_layer_it_is_called_on() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new(2));
    for (i, l) in out.scene.in_order().into_iter().enumerate() {
        if i != LayerId::Base as usize {
            assert!(
                l.texts.is_empty() && l.geometry.under.is_empty(),
                "layer {i}"
            );
        }
    }
}

#[test]
fn the_tree_of_a_frame_has_the_host_group_first() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new(2));
    let tree: &SemanticTree = &out.out.tree;
    assert_eq!(tree.nodes[0].id, host_id());
}

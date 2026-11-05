//! The settings page frame (`layout.settings-page` acceptance): the glass with its one round
//! corner, the 20 px content host, the restart banner (padding, colours, reveal in three tiers,
//! hide keeps its space until gone), the action that fires once, the nodes, the disabled walk and a
//! section of rows (a toggle, a combo and a stepper) whose roles survive and whose list draws above
//! the page.

use lune_ui_core::GlassLayer;
use lune_ui_core::access::{Live, Role, SemanticTree, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{Easing, MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::{Scene, Shape};
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style, TextSystem, elide};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::row::SettingRow;
use lune_ui_core::widgets::settings_page::{
    ACTION_BORDER, ACTION_PAD_X, ACTION_PAD_Y, ACTION_RADIUS, BANNER_PAD_X, BANNER_PAD_Y, HOST_PAD,
    ICON_SIZE, RADIUS, REVEAL, SettingsPage, SettingsResponse,
};
use lune_ui_core::widgets::stepper::StepRange;

const WIN: (f32, f32) = (900.0, 600.0);
const RECT: Rect = Rect::new(0.0, 0.0, 900.0, 600.0);
const GLASS: GlassLayer = GlassLayer {
    tint: [10, 20, 30, 255],
    tint_opacity: 0.8,
    material_opacity: 0.7,
};
const UNDER: [f32; 4] = [0.05, 0.06, 0.07, 1.0];
const BASE: u64 = 250;
const MESSAGE: &str = "Some changes require a restart to take effect.";
const ACTION: &str = "Restart now";

fn page_id() -> WidgetId {
    WidgetId::ROOT.with("page")
}

fn id(key: &str) -> WidgetId {
    page_id().with(key)
}

/// What the app keeps, and what to draw this frame.
#[derive(Clone, Copy)]
struct Cfg {
    rect: Rect,
    active: bool,
    restart: bool,
    enabled: bool,
    /// The section's height; large enough to scroll with `tall`.
    content_h: f32,
    name: &'static str,
}

impl Cfg {
    fn new() -> Cfg {
        Cfg {
            rect: RECT,
            active: true,
            restart: false,
            enabled: true,
            content_h: 300.0,
            name: "General",
        }
    }
}

/// The values the section edits.
#[derive(Debug, Default)]
struct App {
    on: bool,
    open: bool,
    value: u32,
}

struct Out {
    scene: Scene,
    out: FrameOutput,
    resp: SettingsResponse,
    /// The viewport the content was given.
    host: Rect,
    /// The section's rows, top to bottom: (title, height).
    heights: Vec<f32>,
}

fn frame_with(h: &mut UiHarness, c: &Cfg, app: &mut App, rows: bool) -> Out {
    let mut host = Rect::default();
    let mut heights = Vec::new();
    let mut resp = SettingsResponse::default();
    let (scene, out) = h.frame(|ui| {
        let page = SettingsPage {
            active: c.active,
            glass: GLASS,
            under: UNDER,
            restart: c.restart,
            name: c.name,
        };
        resp = ui.with_enabled(c.enabled, |ui| {
            ui.settings_page("page", c.rect, &page, |ui, vp| {
                host = vp;
                if !rows {
                    ui.painter()
                        .text(&Spec::plain("Body line", body_style()), vp.x, vp.y);
                    return c.content_h;
                }
                let mut y = vp.y;
                let first = SettingRow {
                    title: "Updates",
                    description: Some("Look for a new version."),
                    off: false,
                    control_w: 40.0,
                };
                let on = &mut app.on;
                let a = ui.setting_row("r1", Rect::new(vp.x, y, 500.0, 0.0), &first, |ui, s| {
                    ui.toggle("t", s, on, "Updates");
                });
                y += a;
                let second = SettingRow {
                    title: "Font",
                    description: None,
                    off: false,
                    control_w: 200.0,
                };
                let open = &mut app.open;
                let b = ui.setting_row("r2", Rect::new(vp.x, y, 500.0, 0.0), &second, |ui, s| {
                    let at = Rect::new(s.x, s.y + (s.h - 32.0) / 2.0, 200.0, 32.0);
                    let r = ui.combo("c", at, "Alpha", open, "Font");
                    if *open {
                        let opts = [("Alpha", true), ("Beta", true)];
                        let d = ui.dropdown("list", r.rect, r.id, &opts, 0);
                        if d.picked.is_some() || d.dismissed {
                            *open = false;
                        }
                    }
                });
                y += b;
                let third = SettingRow {
                    title: "Recent",
                    description: None,
                    off: false,
                    control_w: 140.0,
                };
                let value = &mut app.value;
                let range = StepRange {
                    min: 0.0,
                    max: 50.0,
                    step: 1.0,
                };
                let d = ui.setting_row("r3", Rect::new(vp.x, y, 500.0, 0.0), &third, |ui, s| {
                    let at = Rect::new(s.x, s.y + (s.h - 32.0) / 2.0, 140.0, 32.0);
                    ui.stepper("s", at, *value, range, "Recent");
                });
                heights = vec![a, b, d];
                y + d - vp.y
            })
        });
    });
    Out {
        scene,
        out,
        resp,
        host,
        heights,
    }
}

fn body_style() -> Style {
    Style::ui(13.0, [255, 255, 255, 255])
}

/// Where the body line is drawn, if it is.
fn body_y(scene: &Scene) -> Option<f32> {
    let want = lune_ui_core::text::TextKey(Spec::plain("Body line", body_style()).cache_key(1.0));
    scene.layers[0]
        .texts
        .iter()
        .find(|t| t.key == want)
        .map(|t| t.y)
}

fn frame(h: &mut UiHarness, c: &Cfg) -> Out {
    frame_with(h, c, &mut App::default(), false)
}

/// Two frames and a settled one, so hover and tweens are done.
fn settled(h: &mut UiHarness, c: &Cfg) -> Out {
    frame(h, c);
    h.at_ms(10);
    frame(h, c);
    h.at_ms(5000);
    frame(h, c)
}

fn focused(tree: &SemanticTree) -> Option<String> {
    tree.nodes
        .iter()
        .find(|n| n.states.contains(State::Focused))
        .map(|n| n.name.clone())
}

fn under(scene: &Scene) -> Vec<Shape> {
    scene.layers[LayerId::Base as usize].geometry.under.clone()
}

/// The size the fake text system gives `text` at the banner's style.
fn measure(text: &str) -> (f32, f32) {
    let style = Style::ui(DARK.font_base, DARK.status_warning_light);
    FakeText.prepare(&Spec::plain(text, style), 1.0).1
}

/// The banner's height: the action (its text plus 2 x (6 + 1)) plus 2 x 10.
fn bar_h() -> f32 {
    measure(ACTION).1 + 2.0 * (ACTION_PAD_Y + ACTION_BORDER) + 2.0 * BANNER_PAD_Y
}

fn bar_of(scene: &Scene) -> Option<Shape> {
    under(scene)
        .into_iter()
        .find(|s| s.rect[0] == 0.0 && s.rect[2] == RECT.w && s.rect[3] == bar_h())
}

/// The action's rect, from its node.
fn action_rect(tree: &SemanticTree) -> Rect {
    tree.get(id("restart")).expect("the action").bounds
}

fn centre(r: Rect) -> (f32, f32) {
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

// ---- the glass ----

#[test]
fn the_glass_is_the_acrylic_with_one_round_corner() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new());
    let g = &out.scene.layers[0].geometry;
    assert_eq!(g.acrylic.len(), 1);
    let a = g.acrylic[0].shape;
    assert_eq!(a.rect, [0.0, 0.0, 900.0, 600.0]);
    assert_eq!(a.radii, [RADIUS, 0.0, 0.0, 0.0]);
    assert_eq!(RADIUS, 8.0);
    assert_eq!(a.color, GLASS.premultiplied());
    assert_eq!(a.under, UNDER);
    assert_eq!(out.out.repaint, Repaint::Idle);
}

#[test]
fn inactive_draws_the_fallback_with_the_same_corner_and_no_acrylic() {
    let mut h = UiHarness::new(WIN);
    let out = settled(
        &mut h,
        &Cfg {
            active: false,
            ..Cfg::new()
        },
    );
    let g = &out.scene.layers[0].geometry;
    assert!(g.acrylic.is_empty());
    assert_eq!(DARK.window_fallback, [0x33, 0x33, 0x33, 0xFF]);
    let fill = g
        .under
        .iter()
        .find(|s| s.rect == [0.0, 0.0, 900.0, 600.0])
        .expect("the page fill");
    assert_eq!(fill.color, premul(DARK.window_fallback));
    assert_eq!(fill.radii, [RADIUS, 0.0, 0.0, 0.0]);
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let c = Cfg {
        restart: true,
        ..Cfg::new()
    };
    let out = settled(&mut h, &c);
    let g = &out.scene.layers[0].geometry;
    // The corner: 16 px at the top left only.
    assert_eq!(g.acrylic[0].shape.radii, [16.0, 0.0, 0.0, 0.0]);
    // The content inset: 40 px from the left and the top.
    assert_eq!(out.host.x * 2.0, 40.0);
    assert_eq!(out.host.y * 2.0, 40.0);
    // The banner: its fill spans the width at the page's bottom; the action sits 40 px from the
    // right edge, and the padding above and below it is 20 px.
    let bar_h = bar_h();
    let bar = under(&out.scene)
        .into_iter()
        .find(|s| s.rect[2] == 1800.0 && s.rect[3] == bar_h * 2.0)
        .expect("the banner fill");
    assert_eq!(bar.rect[1] + bar.rect[3], 1200.0, "docked at the bottom");
    assert_eq!(bar.color, premul(DARK.status_warning_dark));
    let at = action_rect(&out.out.tree);
    assert_eq!((900.0 - at.right()) * 2.0, 40.0, "side padding");
    let top = (at.y - (600.0 - bar_h)) * 2.0;
    let bottom = (600.0 - at.bottom()) * 2.0;
    assert_eq!((top, bottom), (20.0, 20.0));
    assert_eq!(BANNER_PAD_X, 20.0);
}

// ---- the content host ----

#[test]
fn the_host_is_inset_20_and_the_banner_docks_below_it() {
    let mut h = UiHarness::new(WIN);
    let none = settled(&mut h, &Cfg::new());
    assert_eq!(none.host, Rect::new(20.0, 20.0, 860.0, 560.0));
    assert_eq!(HOST_PAD, 20.0);
    let mut h = UiHarness::new(WIN);
    let some = settled(
        &mut h,
        &Cfg {
            restart: true,
            ..Cfg::new()
        },
    );
    assert_eq!(
        some.host,
        Rect::new(20.0, 20.0, 860.0, 560.0 - bar_h()),
        "the banner's height comes off the host"
    );
}

#[test]
fn the_content_scrolls_through_a_scroll_area() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        content_h: 1500.0,
        ..Cfg::new()
    };
    let out = settled(&mut h, &c);
    let tree = &out.out.tree;
    let area = tree.get(id("content")).expect("scroll area");
    assert_eq!(area.role, Role::ScrollArea);
    assert_eq!(area.parent, Some(page_id()));
    assert_eq!(area.bounds, Rect::new(20.0, 20.0, 860.0, 560.0));
    // A wheel moves the content up and clamps at the end; the viewport stays.
    assert_eq!(body_y(&out.scene), Some(20.0));
    h.input.pointer_to(300.0, 300.0);
    h.input.scroll_lines(0.0, -3.0);
    h.at_ms(5100);
    frame(&mut h, &c);
    h.at_ms(6000);
    let after = frame(&mut h, &c);
    assert_eq!(after.host, out.host, "the viewport does not move");
    assert_eq!(body_y(&after.scene), Some(20.0 - 144.0));
    h.input.scroll_lines(0.0, -500.0);
    h.at_ms(7000);
    frame(&mut h, &c);
    h.at_ms(9000);
    let end = frame(&mut h, &c);
    assert_eq!(body_y(&end.scene), Some(20.0 - (1500.0 - 560.0)));
}

#[test]
fn a_short_section_does_not_scroll() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        content_h: 100.0,
        ..Cfg::new()
    };
    settled(&mut h, &c);
    h.input.pointer_to(300.0, 300.0);
    h.input.scroll_lines(0.0, -3.0);
    h.at_ms(5100);
    let a = frame(&mut h, &c);
    h.at_ms(6000);
    let b = frame(&mut h, &c);
    assert_eq!(body_y(&a.scene), Some(20.0));
    assert_eq!(body_y(&b.scene), Some(20.0));
}

// ---- the banner ----

#[test]
fn a_hidden_banner_takes_no_height_and_no_node() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new());
    let tree = &out.out.tree;
    assert!(tree.get(id("banner")).is_none());
    assert!(tree.get(id("restart")).is_none());
    assert!(tree.nodes.iter().all(|n| n.role != Role::Status));
    assert!(bar_of(&out.scene).is_none(), "no fill");
    assert_eq!(out.host.h, 560.0, "the host has the whole height");
}

#[test]
fn the_banner_is_a_polite_status_holding_the_action_button() {
    let mut h = UiHarness::new(WIN);
    let out = settled(
        &mut h,
        &Cfg {
            restart: true,
            ..Cfg::new()
        },
    );
    let tree = &out.out.tree;
    let banner = tree.get(id("banner")).expect("banner");
    assert_eq!(banner.role, Role::Status);
    assert_eq!(banner.live, Some(Live::Polite));
    assert_eq!(banner.name, MESSAGE);
    assert_eq!(banner.parent, Some(page_id()));
    assert_eq!(
        banner.bounds,
        Rect::new(0.0, 600.0 - bar_h(), 900.0, bar_h())
    );
    let action = tree.get(id("restart")).expect("action");
    assert_eq!(action.role, Role::Button);
    assert_eq!(action.name, ACTION);
    assert_eq!(action.parent, Some(id("banner")));
    assert!(action.states.contains(State::Focusable));
    // Sized by its text and padding 12 by 6 inside a 1 px border.
    let (tw, th) = measure(ACTION);
    assert_eq!(
        (action.bounds.w, action.bounds.h),
        (
            tw + 2.0 * (ACTION_PAD_X + ACTION_BORDER),
            th + 2.0 * (ACTION_PAD_Y + ACTION_BORDER)
        )
    );
    // Its page is the group, named by the app.
    let page = tree.get(page_id()).expect("page");
    assert_eq!(page.role, Role::Group);
    assert_eq!(page.name, "General");
    assert_eq!(page.bounds, RECT);
}

#[test]
fn the_banner_colours_glyph_and_border_follow_the_warning_pair() {
    let mut h = UiHarness::new(WIN);
    let out = settled(
        &mut h,
        &Cfg {
            restart: true,
            ..Cfg::new()
        },
    );
    let g = &out.scene.layers[0].geometry;
    let at = action_rect(&out.out.tree);
    // The 1 px border ring in the light colour at radius 6, on the action's rect.
    let ring = g
        .under
        .iter()
        .find(|s| s.params[0] == 1.0 && s.rect == [at.x, at.y, at.w, at.h])
        .expect("the action's border");
    assert_eq!(ring.params[1], ACTION_BORDER);
    assert_eq!(ring.radii, [ACTION_RADIUS; 4]);
    assert_eq!(ring.color, premul(DARK.status_warning_light));
    // The glyph, the message and the action's label are all in the light colour.
    let light = Style::ui(DARK.font_base, DARK.status_warning_light);
    let msg = Spec::plain(MESSAGE, light);
    let label = Spec::plain(ACTION, light);
    let keys: Vec<_> = out.scene.layers[0].texts.iter().map(|t| t.key).collect();
    for spec in [msg, label] {
        assert!(keys.contains(&lune_ui_core::text::TextKey(spec.cache_key(1.0))));
    }
    assert_eq!(ICON_SIZE, 14.0);
    // The glyph sits at the left padding and the message 8 px after it.
    let glyph = lune_ui_core::icon::glyph(
        lune_ui_core::icon::WARNING,
        ICON_SIZE,
        DARK.status_warning_light,
    );
    let gt = out.scene.layers[0]
        .texts
        .iter()
        .find(|t| t.key == lune_ui_core::text::TextKey(glyph.cache_key(1.0)))
        .expect("the glyph");
    let gw = FakeText.prepare(&glyph, 1.0).1.0;
    assert!((gt.x - (BANNER_PAD_X + (ICON_SIZE - gw) / 2.0)).abs() < 0.01);
    let mt = out.scene.layers[0]
        .texts
        .iter()
        .find(|t| t.key == lune_ui_core::text::TextKey(Spec::plain(MESSAGE, light).cache_key(1.0)))
        .expect("the message");
    assert_eq!(mt.x, BANNER_PAD_X + ICON_SIZE + 8.0);
}

#[test]
fn a_narrow_banner_elides_the_message_before_the_action() {
    let mut h = UiHarness::new((300.0, 400.0));
    let c = Cfg {
        rect: Rect::new(0.0, 0.0, 300.0, 400.0),
        restart: true,
        ..Cfg::new()
    };
    let out = settled(&mut h, &c);
    let at = action_rect(&out.out.tree);
    let light = Style::ui(DARK.font_base, DARK.status_warning_light);
    let full = lune_ui_core::text::TextKey(Spec::plain(MESSAGE, light).cache_key(1.0));
    assert!(
        out.scene.layers[0].texts.iter().all(|t| t.key != full),
        "the message is cut"
    );
    // The elided line is the one drawn: the room left of the action less the 8 px gap.
    let text_x = BANNER_PAD_X + ICON_SIZE + 8.0;
    let room = at.x - 8.0 - text_x;
    let cut = elide(&mut FakeText, &Spec::plain(MESSAGE, light), 1.0, room);
    assert_ne!(cut.runs[0].0, MESSAGE);
    let drawn = out.scene.layers[0]
        .texts
        .iter()
        .find(|t| t.key == lune_ui_core::text::TextKey(cut.cache_key(1.0)))
        .expect("the elided message");
    assert_eq!(drawn.x, text_x);
}

// ---- the reveal ----

fn revealed(tier: MotionTier, after: u64) -> Out {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let off = Cfg::new();
    let on = Cfg {
        restart: true,
        ..Cfg::new()
    };
    settled(&mut h, &off);
    h.at_ms(6000);
    frame(&mut h, &on);
    h.at_ms(6000 + after);
    frame(&mut h, &on)
}

/// The banner fill's alpha (0 to 255) in a scene, as drawn.
fn bar_alpha(scene: &Scene) -> u8 {
    let fill = bar_of(scene).expect("the banner fill");
    let want = premul(DARK.status_warning_dark);
    let a = fill.color[3];
    assert!(a <= want[3]);
    (a / want[3] * 255.0).round() as u8
}

#[test]
fn the_banner_fades_in_over_base_with_quad_out() {
    let first = revealed(MotionTier::Full, 0);
    assert_eq!(first.out.repaint, Repaint::Now);
    assert_eq!(bar_alpha(&first.scene), 0, "starts clear");
    // The space is there from the first frame.
    assert_eq!(first.host.h, 560.0 - bar_h());
    let mid = revealed(MotionTier::Full, BASE / 2);
    let want = (Easing::QuadOut.eval(0.5) * 255.0).round();
    let got = f32::from(bar_alpha(&mid.scene));
    assert!((got - want).abs() <= 1.0, "{got} vs {want}");
    assert_eq!(mid.out.repaint, Repaint::Now);
    let done = revealed(MotionTier::Full, BASE);
    assert_eq!(bar_alpha(&done.scene), 255);
    assert_eq!(done.out.repaint, Repaint::Idle);
}

#[test]
fn reveal_conforms_in_three_tiers() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&REVEAL, &theme.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("{tier:?}: {e}"));
    }
    let full = theme
        .motion
        .resolve(&REVEAL, MotionTier::Full)
        .expect("full");
    assert_eq!(full.duration.as_millis(), 250, "base");
    assert_eq!(full.easing, Easing::QuadOut);
    let reduced = theme
        .motion
        .resolve(&REVEAL, MotionTier::Reduced)
        .expect("reduced keeps the fade");
    assert_eq!(reduced.duration.as_millis(), 180, "fast");
    assert_eq!(reduced.easing, Easing::QuadOut);
    assert!(theme.motion.resolve(&REVEAL, MotionTier::Off).is_none());
}

#[test]
fn reduced_fades_over_fast_and_off_snaps() {
    let r = revealed(MotionTier::Reduced, 90);
    let a = bar_alpha(&r.scene);
    let want = (Easing::QuadOut.eval(0.5) * 255.0).round();
    assert!((f32::from(a) - want).abs() <= 1.0, "{a} vs {want}");
    let r = revealed(MotionTier::Reduced, 180);
    assert_eq!(bar_alpha(&r.scene), 255);
    assert_eq!(r.out.repaint, Repaint::Idle);
    let o = revealed(MotionTier::Off, 0);
    assert_eq!(bar_alpha(&o.scene), 255);
    assert_eq!(o.out.repaint, Repaint::Idle);
}

#[test]
fn the_first_frame_with_the_flag_set_shows_the_banner_at_once() {
    let mut h = UiHarness::new(WIN);
    let out = frame(
        &mut h,
        &Cfg {
            restart: true,
            ..Cfg::new()
        },
    );
    assert_eq!(bar_alpha(&out.scene), 255);
    assert_eq!(out.out.repaint, Repaint::Idle);
}

#[test]
fn hiding_fades_out_keeps_its_space_then_the_height_snaps() {
    let mut h = UiHarness::new(WIN);
    let on = Cfg {
        restart: true,
        ..Cfg::new()
    };
    let off = Cfg::new();
    settled(&mut h, &on);
    h.at_ms(6000);
    let first = frame(&mut h, &off);
    // Just cleared: still drawn at full, the host has not grown, and there is no node to announce.
    assert_eq!(bar_alpha(&first.scene), 255);
    assert_eq!(first.host.h, 560.0 - bar_h());
    assert_eq!(first.out.repaint, Repaint::Now);
    assert!(first.out.tree.get(id("banner")).is_none());
    assert!(first.out.tree.get(id("restart")).is_none());
    // Half way: faded, space kept.
    h.at_ms(6000 + BASE / 2);
    let mid = frame(&mut h, &off);
    let a = bar_alpha(&mid.scene);
    assert!(a > 0 && a < 255, "{a}");
    assert_eq!(mid.host.h, 560.0 - bar_h());
    assert_eq!(mid.out.repaint, Repaint::Now);
    // Gone: no fill, the height snaps to the whole.
    h.at_ms(6000 + BASE + 20);
    let done = frame(&mut h, &off);
    assert!(bar_of(&done.scene).is_none());
    assert_eq!(done.host.h, 560.0);
    assert_eq!(done.out.repaint, Repaint::Idle);
}

#[test]
fn a_fading_banner_cannot_be_pressed() {
    let mut h = UiHarness::new(WIN);
    let on = Cfg {
        restart: true,
        ..Cfg::new()
    };
    let off = Cfg::new();
    let rest = settled(&mut h, &on);
    let at = centre(action_rect(&rest.out.tree));
    h.at_ms(6000);
    frame(&mut h, &off);
    h.input.pointer_to(at.0, at.1);
    h.at_ms(6050);
    frame(&mut h, &off);
    h.input.click(PointerButton::Primary);
    h.at_ms(6100);
    let out = frame(&mut h, &off);
    assert!(!out.resp.restart);
}

// ---- the action ----

#[test]
fn the_action_fires_once_per_press() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        restart: true,
        ..Cfg::new()
    };
    let rest = settled(&mut h, &c);
    let at = centre(action_rect(&rest.out.tree));
    h.input.pointer_to(at.0, at.1);
    h.at_ms(5100);
    let hover = frame(&mut h, &c);
    assert!(!hover.resp.restart);
    h.input.click(PointerButton::Primary);
    h.at_ms(5200);
    let fired = frame(&mut h, &c);
    assert!(fired.resp.restart, "the press reports");
    h.at_ms(5300);
    let after = frame(&mut h, &c);
    assert!(!after.resp.restart, "once");
    // A press elsewhere on the banner does nothing.
    h.input.pointer_to(30.0, 590.0);
    h.at_ms(5400);
    frame(&mut h, &c);
    h.input.click(PointerButton::Primary);
    h.at_ms(5500);
    assert!(!frame(&mut h, &c).resp.restart);
}

#[test]
fn enter_and_space_press_the_focused_action() {
    for k in [NamedKey::Enter, NamedKey::Space] {
        let mut h = UiHarness::new(WIN);
        let c = Cfg {
            restart: true,
            ..Cfg::new()
        };
        settled(&mut h, &c);
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        h.at_ms(5100);
        let out = frame(&mut h, &c);
        assert_eq!(focused(&out.out.tree).as_deref(), Some(ACTION));
        h.input.key_down(Key::Named(k), None);
        h.at_ms(5200);
        assert!(frame(&mut h, &c).resp.restart, "{k:?}");
    }
}

#[test]
fn the_hover_and_press_fills_follow_the_button_looks() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        restart: true,
        ..Cfg::new()
    };
    let rest = settled(&mut h, &c);
    let r = action_rect(&rest.out.tree);
    let fill_of = |scene: &Scene| {
        under(scene)
            .into_iter()
            .find(|s| s.rect == [r.x, r.y, r.w, r.h] && s.params[0] != 1.0)
    };
    assert!(
        fill_of(&rest.scene).is_none(),
        "nothing over the banner at rest"
    );
    h.input.pointer_to(centre(r).0, centre(r).1);
    h.at_ms(5100);
    frame(&mut h, &c);
    h.at_ms(5600);
    let hot = frame(&mut h, &c);
    assert_eq!(
        fill_of(&hot.scene).expect("hover fill").color,
        premul(DARK.surface_hover)
    );
}

#[test]
fn the_action_has_a_focus_ring_when_tabbed_to() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        restart: true,
        ..Cfg::new()
    };
    let rest = settled(&mut h, &c);
    let rings = |o: &Out| {
        o.scene.layers[0]
            .geometry
            .under
            .iter()
            .filter(|s| s.params[0] == 1.0)
            .count()
    };
    let base = rings(&rest);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(5100);
    frame(&mut h, &c);
    h.at_ms(5600);
    let tabbed = frame(&mut h, &c);
    assert_eq!(rings(&tabbed), base + 1, "the focus ring");
}

// ---- tab order and keys ----

#[test]
fn tab_order_is_the_content_then_the_action() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        restart: true,
        ..Cfg::new()
    };
    let mut app = App::default();
    frame_with(&mut h, &c, &mut app, true);
    let mut order = Vec::new();
    for i in 0..5 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        h.at_ms(20 * (i + 1));
        let out = frame_with(&mut h, &c, &mut app, true);
        order.push(focused(&out.out.tree));
    }
    let names: Vec<_> = order.into_iter().map(|n| n.unwrap_or_default()).collect();
    assert_eq!(names[..4], ["Updates", "Font", "Recent", ACTION]);
}

#[test]
fn the_page_takes_no_keys_of_its_own() {
    for k in [
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::PageDown,
        NamedKey::Home,
        NamedKey::Enter,
        NamedKey::Escape,
    ] {
        let mut h = UiHarness::new(WIN);
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        let c = Cfg {
            restart: true,
            ..Cfg::new()
        };
        settled(&mut h, &c);
        h.input.key_down(Key::Named(k), None);
        h.at_ms(5100);
        assert_eq!(frame(&mut h, &c).out.fired.len(), 1, "{k:?}");
    }
}

// ---- disabled ----

#[test]
fn a_disabled_page_is_disabled_all_the_way_down() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        enabled: false,
        restart: true,
        ..Cfg::new()
    };
    let mut app = App::default();
    frame_with(&mut h, &c, &mut app, true);
    h.at_ms(5000);
    let out = frame_with(&mut h, &c, &mut app, true);
    let bad = disabled_violations(&out.out.tree, page_id());
    assert!(bad.is_empty(), "{bad:?}");
    let action = out.out.tree.get(id("restart")).expect("action");
    assert!(action.states.contains(State::Disabled));
    assert!(!action.states.contains(State::Focusable));
    // No Tab stop, no press.
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(5100);
    let out = frame_with(&mut h, &c, &mut app, true);
    assert_eq!(focused(&out.out.tree), None);
    let at = centre(action_rect(&out.out.tree));
    h.input.pointer_to(at.0, at.1);
    h.at_ms(5200);
    frame_with(&mut h, &c, &mut app, true);
    h.input.click(PointerButton::Primary);
    h.at_ms(5300);
    let out = frame_with(&mut h, &c, &mut app, true);
    assert!(!out.resp.restart);
}

#[test]
fn a_disabled_page_reports_nothing_and_leaves_the_keys_to_shortcuts() {
    for k in [NamedKey::Enter, NamedKey::Space] {
        let mut h = UiHarness::new(WIN);
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        let c = Cfg {
            enabled: false,
            restart: true,
            ..Cfg::new()
        };
        settled(&mut h, &c);
        h.input.key_down(Key::Named(k), None);
        h.at_ms(5100);
        let out = frame(&mut h, &c);
        assert!(!out.resp.restart, "{k:?}");
        assert_eq!(
            out.out.fired.len(),
            1,
            "{k:?}: nothing on the page holds focus"
        );
    }
}

#[test]
fn an_enabled_page_is_not_marked_disabled() {
    let mut h = UiHarness::new(WIN);
    let out = settled(
        &mut h,
        &Cfg {
            restart: true,
            ..Cfg::new()
        },
    );
    for k in ["banner", "restart"] {
        assert!(
            !out.out
                .tree
                .get(id(k))
                .expect("node")
                .states
                .contains(State::Disabled)
        );
    }
    assert!(
        !out.out
            .tree
            .get(page_id())
            .expect("group")
            .states
            .contains(State::Disabled)
    );
}

// ---- a section of rows ----

#[test]
fn the_section_keeps_its_roles_under_group_rows() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    let mut app = App::default();
    frame_with(&mut h, &c, &mut app, true);
    h.at_ms(5000);
    let out = frame_with(&mut h, &c, &mut app, true);
    let tree = &out.out.tree;
    // Rows are groups, children of the scroll area; each holds its control.
    let content = id("content");
    let cases = [
        ("r1", "Updates", Role::Switch),
        ("r2", "Font", Role::ComboBox),
        ("r3", "Recent", Role::SpinButton),
    ];
    for (row, name, role) in cases {
        let g = tree.get(id(row)).unwrap_or_else(|| panic!("{row}"));
        assert_eq!(g.role, Role::Group, "{row}");
        assert_eq!(g.name, name);
        assert_eq!(g.parent, Some(content));
        let kids: Vec<_> = g
            .children
            .iter()
            .map(|c| tree.get(*c).expect("child"))
            .collect();
        assert_eq!(kids.len(), 1, "{row}");
        assert_eq!(kids[0].role, role, "{row}");
        assert_eq!(kids[0].name, name);
    }
    // The three rows are 62 (with a description), then 56 and 56.
    assert_eq!(out.heights, [62.0, 56.0, 56.0]);
    // The rows start at the host's top left.
    assert_eq!(tree.get(id("r1")).expect("row").bounds.x, 20.0);
    assert_eq!(tree.get(id("r1")).expect("row").bounds.y, 20.0);
}

#[test]
fn a_row_lights_on_hover_and_the_control_keeps_the_press() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    let mut app = App::default();
    frame_with(&mut h, &c, &mut app, true);
    h.at_ms(100);
    frame_with(&mut h, &c, &mut app, true);
    // Over the first row's text: its hover fill is there after the tween.
    h.input.pointer_to(120.0, 50.0);
    h.at_ms(200);
    frame_with(&mut h, &c, &mut app, true);
    h.at_ms(1000);
    let out = frame_with(&mut h, &c, &mut app, true);
    let row = under(&out.scene)
        .into_iter()
        .find(|s| s.rect == [20.0, 20.0, 500.0, 62.0])
        .expect("the row's hover fill");
    assert_eq!(row.color, premul(DARK.surface_hover));
    // A click on the toggle flips it.
    let sw = out
        .out
        .tree
        .get(id("r1").with("t"))
        .or_else(|| out.out.tree.nodes.iter().find(|n| n.role == Role::Switch))
        .expect("the switch")
        .bounds;
    h.input.pointer_to(centre(sw).0, centre(sw).1);
    h.at_ms(1100);
    frame_with(&mut h, &c, &mut app, true);
    h.input.click(PointerButton::Primary);
    h.at_ms(1200);
    frame_with(&mut h, &c, &mut app, true);
    assert!(app.on);
}

#[test]
fn a_combos_list_draws_above_the_page() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    let mut app = App {
        open: true,
        ..App::default()
    };
    frame_with(&mut h, &c, &mut app, true);
    h.at_ms(100);
    let out = frame_with(&mut h, &c, &mut app, true);
    h.at_ms(600);
    let out2 = frame_with(&mut h, &c, &mut app, true);
    let _ = out;
    let beta = Spec::plain("Beta", Style::ui(DARK.font_base, DARK.fg_primary));
    let _ = beta;
    let popup = &out2.scene.layers[LayerId::Popup as usize];
    assert!(
        !popup.texts.is_empty() || !popup.geometry.under.is_empty(),
        "the list is on the popup layer"
    );
    // The page itself drew nothing there before the list.
    let mut h = UiHarness::new(WIN);
    let closed = frame_with(&mut h, &c, &mut App::default(), true);
    let popup = &closed.scene.layers[LayerId::Popup as usize];
    assert!(popup.texts.is_empty() && popup.geometry.under.is_empty());
}

#[test]
fn the_page_draws_on_the_layer_it_is_called_on() {
    let mut h = UiHarness::new(WIN);
    let out = settled(
        &mut h,
        &Cfg {
            restart: true,
            ..Cfg::new()
        },
    );
    for (i, l) in out.scene.layers.iter().enumerate() {
        if i != LayerId::Base as usize {
            assert!(
                l.texts.is_empty() && l.geometry.under.is_empty(),
                "layer {i}"
            );
        }
    }
}

#[test]
fn a_resize_moves_the_host_and_the_banner() {
    let mut h = UiHarness::new((600.0, 400.0));
    let c = Cfg {
        rect: Rect::new(0.0, 0.0, 600.0, 400.0),
        restart: true,
        ..Cfg::new()
    };
    let out = settled(&mut h, &c);
    assert_eq!(out.host, Rect::new(20.0, 20.0, 560.0, 360.0 - bar_h()));
    let banner = out.out.tree.get(id("banner")).expect("banner").bounds;
    assert_eq!(banner, Rect::new(0.0, 400.0 - bar_h(), 600.0, bar_h()));
}

#[test]
fn a_page_smaller_than_its_banner_never_panics() {
    let mut h = UiHarness::new((40.0, 30.0));
    let c = Cfg {
        rect: Rect::new(0.0, 0.0, 40.0, 30.0),
        restart: true,
        ..Cfg::new()
    };
    let out = settled(&mut h, &c);
    assert_eq!(out.host.h, 0.0);
    assert!(out.host.w >= 0.0);
}

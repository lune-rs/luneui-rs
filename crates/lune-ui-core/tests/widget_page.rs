//! The page card (`layout.page` acceptance): the glass card and the solid inactive fill, the
//! margin and radius probes at scale 2, the header, the scrolling body that clamps, the pinned
//! footer, tab order, the keyed swap in three tiers (restarting from the current look), hidden
//! slots, a resize, keep-alive, the text limit near the rounded corners, the semantic nodes and the
//! disabled walk.

use lune_ui_core::GlassLayer;
use lune_ui_core::access::{Role, SemanticTree, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, NamedKey, Repaint};
use lune_ui_core::motion::{Easing, MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::{Scene, TextItem};
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations};
use lune_ui_core::text::{Spec, Style, TextKey, TextSystem, elide};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::page::{
    FOOTER_H, MARGIN, PAD, PageOptions, PageSlot, RADIUS, SWAP_DY, SWAP_OPACITY, SWAP_SLIDE,
    card_rect, header_height,
};
use lune_ui_core::widgets::scroll_area::ScrollResponse;

const WIN: (f32, f32) = (900.0, 600.0);
const RECT: Rect = Rect::new(0.0, 0.0, 900.0, 600.0);
const GLASS: GlassLayer = GlassLayer {
    tint: [10, 20, 30, 255],
    tint_opacity: 0.8,
    material_opacity: 0.7,
};
const UNDER: [f32; 4] = [0.05, 0.06, 0.07, 1.0];
const MODERATE: u64 = 350;

fn page_id() -> WidgetId {
    WidgetId::ROOT.with("page")
}

fn id(key: &str) -> WidgetId {
    page_id().with(key)
}

#[derive(Clone, Copy)]
struct Cfg {
    rect: Rect,
    key: u64,
    active: bool,
    footer: bool,
    actions_w: f32,
    body_h: f32,
    subtitle: bool,
    enabled: bool,
    title: &'static str,
}

impl Cfg {
    fn new() -> Cfg {
        Cfg {
            rect: RECT,
            key: 1,
            active: true,
            footer: true,
            actions_w: 80.0,
            body_h: 1200.0,
            subtitle: false,
            enabled: true,
            title: "Overview",
        }
    }
}

struct Out {
    scene: Scene,
    out: FrameOutput,
    slots: Vec<&'static str>,
    resp: ScrollResponse<()>,
    inner: Rect,
}

fn body_style(alpha: u8) -> Style {
    Style::ui(13.0, [255, 255, 255, alpha])
}

fn frame(h: &mut UiHarness, c: &Cfg) -> Out {
    let mut slots = Vec::new();
    let mut inner = Rect::default();
    let mut resp = None;
    let (scene, out) = h.frame(|ui| {
        let opts = PageOptions {
            title: c.title,
            subtitle: c.subtitle.then_some("Last 30 days"),
            active: c.active,
            glass: GLASS,
            under: UNDER,
            actions_w: c.actions_w,
            footer: c.footer,
            swap_key: c.key,
            name: "",
        };
        resp = Some(ui.with_enabled(c.enabled, |ui| {
            ui.page("page", c.rect, &opts, |ui, slot| match slot {
                PageSlot::Actions(at) => {
                    slots.push("actions");
                    ui.button(
                        "act",
                        Rect::new(at.x, at.y, at.w, 24.0),
                        "Act",
                        ButtonKind::Secondary,
                    );
                    0.0
                }
                PageSlot::Body(view, rect) => {
                    slots.push("body");
                    assert_eq!(view.viewport.x, card_rect(c.rect).x);
                    inner = rect;
                    let p = ui.painter();
                    p.text(&Spec::plain("Body line", body_style(255)), rect.x, rect.y);
                    ui.button(
                        "body-btn",
                        Rect::new(rect.x, rect.y + 40.0, 100.0, 32.0),
                        "Go",
                        ButtonKind::Secondary,
                    );
                    c.body_h
                }
                PageSlot::Footer(row) => {
                    slots.push("footer");
                    ui.button(
                        "foot-btn",
                        Rect::new(row.x, row.y, 100.0, row.h),
                        "Save",
                        ButtonKind::Primary,
                    );
                    0.0
                }
            })
        }));
    });
    Out {
        scene,
        out,
        slots,
        resp: resp.expect("ran"),
        inner,
    }
}

/// Two frames and a settled one, so hover and scroll tweens are done.
fn settled(h: &mut UiHarness, c: &Cfg) -> Out {
    frame(h, c);
    h.at_ms(10);
    frame(h, c);
    h.at_ms(5000);
    frame(h, c)
}

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers[0].texts.clone()
}

fn key_of(spec: &Spec, scale: f32) -> TextKey {
    TextKey(spec.cache_key(scale))
}

/// The opacity (0 to 255) the body line is drawn at, found by trying each.
fn body_alpha(scene: &Scene) -> Option<u8> {
    let keys: Vec<TextKey> = texts(scene).iter().map(|t| t.key).collect();
    (0..=255u8).find(|a| keys.contains(&key_of(&Spec::plain("Body line", body_style(*a)), 1.0)))
}

fn body_y(scene: &Scene) -> f32 {
    let alpha = body_alpha(scene).expect("the body line is drawn");
    let want = key_of(&Spec::plain("Body line", body_style(alpha)), 1.0);
    texts(scene)
        .iter()
        .find(|t| t.key == want)
        .map(|t| t.y)
        .expect("text")
}

fn focused(tree: &SemanticTree) -> Option<String> {
    tree.nodes
        .iter()
        .find(|n| n.states.contains(State::Focused))
        .map(|n| n.name.clone())
}

fn tab(h: &mut UiHarness, c: &Cfg) -> Out {
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.at_ms(20);
    frame(h, c)
}

#[test]
fn the_card_is_the_acrylic_with_the_margin_and_radius_and_a_ring_last() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new());
    let card = card_rect(RECT);
    assert_eq!(card, Rect::new(6.0, 0.0, 884.0, 590.0));
    assert_eq!(MARGIN, (0.0, 10.0, 10.0, 6.0));
    let g = &out.scene.layers[0].geometry;
    assert_eq!(g.acrylic.len(), 1);
    let a = g.acrylic[0].shape;
    assert_eq!(a.rect, [6.0, 0.0, 884.0, 590.0]);
    assert_eq!(a.radii, [RADIUS; 4]);
    assert_eq!(a.color, GLASS.premultiplied());
    assert_eq!(a.under, UNDER);
    // The ring is the last shape over the text: 1 px, border_subtle.
    let ring = g.over.last().expect("ring");
    assert_eq!(ring.rect, [6.0, 0.0, 884.0, 590.0]);
    assert_eq!(ring.params[1], 1.0);
    assert_eq!(ring.color, premul(DARK.border_subtle));
    assert_eq!(out.out.repaint, Repaint::Idle);
}

#[test]
fn pixel_probes_at_scale_two() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let out = settled(&mut h, &Cfg::new());
    let g = &out.scene.layers[0].geometry;
    let a = g.acrylic[0].shape;
    // Margin: 12 px left, 20 px right, 20 px bottom; radius 16 px.
    assert_eq!(a.rect[0], 12.0);
    assert_eq!(a.rect[1], 0.0);
    assert_eq!(900.0 * 2.0 - (a.rect[0] + a.rect[2]), 20.0);
    assert_eq!(600.0 * 2.0 - (a.rect[1] + a.rect[3]), 20.0);
    assert_eq!(a.radii, [16.0; 4]);
    let ring = g.over.last().expect("ring");
    assert_eq!(ring.params[1], 2.0, "a 1 px ring is 2 px at scale 2");
    assert_eq!(ring.rect, a.rect);
    // The title sits at the header padding of 32 px.
    let title = Spec::plain(
        "Overview",
        Style {
            line: 24.0,
            ..Style::ui(DARK.font_title, DARK.fg_bright).medium()
        },
    );
    let t = out.scene.layers[0]
        .texts
        .iter()
        .find(|t| t.key == key_of(&title, 2.0))
        .expect("title");
    // Text positions are logical: 16 px from the card's left and top is 32 px at this scale.
    assert_eq!((t.x - 6.0, t.y), (16.0, 16.0));
}

#[test]
fn inactive_draws_the_fallback_fill_and_no_acrylic() {
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
        .find(|s| s.rect == [6.0, 0.0, 884.0, 590.0])
        .expect("the card fill");
    assert_eq!(fill.color, premul([0x33, 0x33, 0x33, 0xFF]));
    assert_eq!(fill.radii, [RADIUS; 4]);
}

#[test]
fn the_header_is_padded_16_and_the_title_is_elided_left_of_the_actions() {
    let mut h = UiHarness::new(WIN);
    let long = "A very long page title that cannot fit the room beside the actions at all, whatever it says";
    let c = Cfg {
        title: long,
        subtitle: true,
        actions_w: 300.0,
        ..Cfg::new()
    };
    let out = settled(&mut h, &c);
    let card = card_rect(RECT);
    let style = Style {
        line: 24.0,
        ..Style::ui(DARK.font_title, DARK.fg_bright).medium()
    };
    // Room: card less 2 x 16, the actions and the 12 px gap.
    let room = card.w - 2.0 * PAD - 300.0 - 12.0;
    let elided = elide(&mut FakeText, &Spec::plain(long, style), 1.0, room);
    assert_ne!(elided.runs[0].0, long, "the title is cut");
    let t = texts(&out.scene)
        .into_iter()
        .find(|t| t.key == key_of(&elided, 1.0))
        .expect("the elided title");
    assert_eq!((t.x, t.y), (card.x + PAD, card.y + PAD));
    // The subtitle sits one title line down.
    let sub = Spec::plain(
        "Last 30 days",
        Style {
            line: 20.0,
            ..Style::ui(DARK.font_base, DARK.fg_tertiary)
        },
    );
    let s = texts(&out.scene)
        .into_iter()
        .find(|t| t.key == key_of(&sub, 1.0))
        .expect("the subtitle");
    assert_eq!((s.x, s.y), (card.x + PAD, card.y + PAD + 24.0));
    // The body starts under the 76 px header.
    assert_eq!(header_height(true), 76.0);
    assert_eq!(out.inner.y, card.y + 76.0);
    assert_eq!(header_height(false), 56.0);
}

#[test]
fn the_body_box_is_padded_0_16_16_and_the_footer_is_pinned() {
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new());
    let card = card_rect(RECT);
    let body = Rect::new(card.x, card.y + 56.0, card.w, card.h - 56.0 - FOOTER_H);
    assert_eq!(
        out.inner,
        Rect::new(body.x + 16.0, body.y, body.w - 32.0, body.h - 16.0)
    );
    assert_eq!(out.resp.viewport, body);
    assert_eq!(
        out.resp.extent.1,
        1200.0 + 16.0,
        "the bottom padding is part of the extent"
    );
    // The footer hairline sits at the top of the 56 px footer, full width inside the border.
    let g = &out.scene.layers[0].geometry;
    let hair = g
        .under
        .iter()
        .find(|s| s.rect[3] == 1.0 && s.rect[2] == card.w - 2.0)
        .expect("hairline");
    assert_eq!(hair.rect[1], card.bottom() - FOOTER_H);
    assert_eq!(hair.color, premul(DARK.border_subtle));
}

#[test]
fn a_wheel_scroll_moves_the_body_clamps_at_the_end_and_the_footer_stays() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    let rest = settled(&mut h, &c);
    assert_eq!(rest.resp.offset.1, 0.0);
    let hairline_y = |o: &Out| {
        o.scene.layers[0]
            .geometry
            .under
            .iter()
            .find(|s| s.rect[3] == 1.0)
            .map(|s| s.rect[1])
    };
    let before = hairline_y(&rest);
    h.input.pointer_to(400.0, 200.0);
    h.input.scroll_lines(0.0, -3.0);
    h.at_ms(5100);
    frame(&mut h, &c);
    h.at_ms(6000);
    let one = frame(&mut h, &c);
    assert_eq!(one.resp.offset.1, 144.0, "three lines of 48");
    assert_eq!(hairline_y(&one), before, "the footer stays");
    // The footer button stays put while the body moves.
    // A big wheel clamps at extent - viewport.
    h.input.scroll_lines(0.0, -1000.0);
    h.at_ms(6100);
    frame(&mut h, &c);
    h.at_ms(9000);
    let end = frame(&mut h, &c);
    let max = end.resp.extent.1 - f64::from(end.resp.viewport.h);
    assert_eq!(end.resp.offset.1, max);
    h.input.scroll_lines(0.0, -10.0);
    h.at_ms(9100);
    frame(&mut h, &c);
    h.at_ms(12000);
    assert_eq!(frame(&mut h, &c).resp.offset.1, max, "no further");
    // The other way, back to the top.
    h.input.scroll_lines(0.0, 1000.0);
    h.at_ms(12100);
    frame(&mut h, &c);
    h.at_ms(15000);
    assert_eq!(frame(&mut h, &c).resp.offset.1, 0.0);
}

#[test]
fn the_body_scrolls_vertically_only() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    settled(&mut h, &c);
    h.input.pointer_to(400.0, 200.0);
    h.input.scroll_lines(-5.0, 0.0);
    h.at_ms(5100);
    frame(&mut h, &c);
    h.at_ms(7000);
    let out = frame(&mut h, &c);
    assert_eq!(out.resp.offset.0, 0.0);
}

#[test]
fn tab_order_is_header_body_footer() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    frame(&mut h, &c);
    assert_eq!(focused(&tab(&mut h, &c).out.tree).as_deref(), Some("Act"));
    assert_eq!(focused(&tab(&mut h, &c).out.tree).as_deref(), Some("Go"));
    assert_eq!(focused(&tab(&mut h, &c).out.tree).as_deref(), Some("Save"));
}

#[test]
fn the_slots_are_called_in_order_and_hidden_ones_are_not() {
    let mut h = UiHarness::new(WIN);
    let out = frame(&mut h, &Cfg::new());
    assert_eq!(out.slots, ["actions", "body", "footer"]);
    h.at_ms(10);
    let out = frame(
        &mut h,
        &Cfg {
            actions_w: 0.0,
            footer: false,
            ..Cfg::new()
        },
    );
    assert_eq!(out.slots, ["body"], "no actions and no footer");
    // And the body fills to the bottom of the card.
    let card = card_rect(RECT);
    assert_eq!(out.resp.viewport.bottom(), card.bottom());
}

#[test]
fn the_nodes_are_a_group_a_scroll_area_and_a_footer_group() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        subtitle: true,
        ..Cfg::new()
    };
    let out = settled(&mut h, &c);
    let tree = &out.out.tree;
    let g = tree.get(page_id()).expect("page group");
    assert_eq!(g.role, Role::Group);
    assert_eq!(g.name, "Overview");
    assert_eq!(g.description.as_deref(), Some("Last 30 days"));
    assert_eq!(g.bounds, card_rect(RECT));
    let kids: Vec<Role> = g
        .children
        .iter()
        .map(|c| tree.get(*c).expect("child").role)
        .collect();
    assert_eq!(kids, [Role::Button, Role::ScrollArea, Role::Group]);
    let body = tree.get(id("body")).expect("body");
    assert_eq!(body.role, Role::ScrollArea);
    assert_eq!(body.parent, Some(page_id()));
    let foot = tree.get(id("footer")).expect("footer");
    assert_eq!(foot.role, Role::Group);
    assert!(
        foot.children.contains(&id("foot-btn")),
        "the footer buttons are its children"
    );
    // An explicit name wins over the title.
    let mut h = UiHarness::new(WIN);
    let (_, o) = h.frame(|ui| {
        let opts = PageOptions {
            title: "Overview",
            subtitle: None,
            active: true,
            glass: GLASS,
            under: UNDER,
            actions_w: 0.0,
            footer: false,
            swap_key: 0,
            name: "Main page",
        };
        ui.page("page", RECT, &opts, |_, _| 0.0);
    });
    assert_eq!(o.tree.get(page_id()).expect("page").name, "Main page");
}

#[test]
fn a_resize_keeps_the_offset_and_clamps_it_to_the_new_room() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        body_h: 800.0,
        ..Cfg::new()
    };
    settled(&mut h, &c);
    h.input.pointer_to(400.0, 200.0);
    h.input.scroll_lines(0.0, -5.0);
    h.at_ms(5100);
    frame(&mut h, &c);
    h.at_ms(8000);
    let before = frame(&mut h, &c);
    assert_eq!(before.resp.offset.1, 240.0);
    // A little taller: the offset stays.
    let taller = Cfg {
        rect: Rect::new(0.0, 0.0, 900.0, 650.0),
        ..c
    };
    h.at_ms(8100);
    assert_eq!(frame(&mut h, &taller).resp.offset.1, 240.0);
    // Much taller: the room exceeds the content, and the offset clamps to 0.
    let huge = Cfg {
        rect: Rect::new(0.0, 0.0, 900.0, 1200.0),
        ..c
    };
    h.at_ms(8200);
    frame(&mut h, &huge);
    h.at_ms(8300);
    assert_eq!(frame(&mut h, &huge).resp.offset.1, 0.0);
}

#[test]
fn the_scroll_offset_survives_keep_alive() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    let draw = |h: &mut UiHarness, ms: u64, shown: bool| {
        h.at_ms(ms);
        let mut resp = None;
        h.frame(|ui| {
            ui.keep_alive("tab", |ui| {
                if shown {
                    let opts = PageOptions {
                        title: "T",
                        subtitle: None,
                        active: true,
                        glass: GLASS,
                        under: UNDER,
                        actions_w: 0.0,
                        footer: false,
                        swap_key: c.key,
                        name: "",
                    };
                    resp = Some(ui.page("page", RECT, &opts, |_, slot| {
                        if let PageSlot::Body(..) = slot {
                            1200.0
                        } else {
                            0.0
                        }
                    }));
                }
            });
        });
        resp
    };
    draw(&mut h, 0, true);
    h.input.pointer_to(400.0, 200.0);
    h.input.scroll_lines(0.0, -4.0);
    draw(&mut h, 10, true);
    let at = draw(&mut h, 3000, true).expect("shown").offset.1;
    assert_eq!(at, 192.0);
    // Hidden for a while, then shown again.
    for ms in [3100, 3200, 3300] {
        draw(&mut h, ms, false);
    }
    assert_eq!(draw(&mut h, 3400, true).expect("shown").offset.1, at);
}

// ---- the swap ----

/// A swap to `key` at `ms`, sampled at `ms + after`.
fn swapped(tier: MotionTier, after: u64) -> Out {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let c = Cfg::new();
    settled(&mut h, &c);
    h.at_ms(6000);
    frame(&mut h, &Cfg { key: 2, ..c });
    h.at_ms(6000 + after);
    frame(&mut h, &Cfg { key: 2, ..c })
}

#[test]
fn the_first_frame_of_a_page_does_not_animate() {
    let mut h = UiHarness::new(WIN);
    let out = frame(&mut h, &Cfg::new());
    assert_eq!(body_alpha(&out.scene), Some(255));
    assert_eq!(out.out.repaint, Repaint::Idle);
    assert_eq!(body_y(&out.scene), 56.0);
}

#[test]
fn a_swap_fades_the_body_in_and_slides_it_up_and_leaves_the_rest_alone() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    let rest = settled(&mut h, &c);
    let rest_y = body_y(&rest.scene);
    h.at_ms(6000);
    let first = frame(&mut h, &Cfg { key: 2, ..c });
    assert_eq!(first.out.repaint, Repaint::Now);
    // At the instant of the swap: invisible, SWAP_DY below.
    assert_eq!(body_alpha(&first.scene), Some(0));
    assert_eq!(body_y(&first.scene), rest_y + SWAP_DY);
    // The card, the header title and the footer do not move or fade.
    let card = |o: &Out| o.scene.layers[0].geometry.acrylic[0].shape.rect;
    assert_eq!(card(&first), card(&rest));
    let title = Spec::plain(
        "Overview",
        Style {
            line: 24.0,
            ..Style::ui(DARK.font_title, DARK.fg_bright).medium()
        },
    );
    for o in [&first, &rest] {
        let t = texts(&o.scene)
            .into_iter()
            .find(|t| t.key == key_of(&title, 1.0))
            .expect("the title is drawn unfaded");
        assert_eq!((t.x, t.y), (22.0, 16.0));
    }
    // Half way (175 ms): quad-out opacity and sine-in-out slide.
    h.at_ms(6175);
    let mid = frame(&mut h, &Cfg { key: 2, ..c });
    let p = 175.0 / MODERATE as f32;
    let want_a = (Easing::QuadOut.eval(p) * 255.0).round();
    let got_a = f32::from(body_alpha(&mid.scene).expect("faded"));
    assert!((got_a - want_a).abs() <= 1.0, "{got_a} vs {want_a}");
    let want_dy = SWAP_DY * (1.0 - Easing::SineInOut.eval(p));
    assert!((body_y(&mid.scene) - rest_y - want_dy).abs() < 0.01);
    assert_eq!(mid.out.repaint, Repaint::Now);
    // Settled by 350 ms: opacity 1, offset 0, and the scene is the rest scene.
    h.at_ms(6000 + MODERATE);
    let done = frame(&mut h, &Cfg { key: 2, ..c });
    assert_eq!(body_alpha(&done.scene), Some(255));
    assert_eq!(body_y(&done.scene), rest_y);
    assert_eq!(done.out.repaint, Repaint::Idle);
    assert_eq!(done.scene, rest.scene);
}

#[test]
fn a_new_key_scrolls_the_body_back_to_the_top() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    settled(&mut h, &c);
    h.input.pointer_to(400.0, 200.0);
    h.input.scroll_lines(0.0, -4.0);
    h.at_ms(5100);
    frame(&mut h, &c);
    h.at_ms(6000);
    assert_eq!(frame(&mut h, &c).resp.offset.1, 192.0);
    h.at_ms(6100);
    assert_eq!(frame(&mut h, &Cfg { key: 2, ..c }).resp.offset.1, 0.0);
}

#[test]
fn swap_conforms_in_three_tiers() {
    let theme = DARK;
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&SWAP_OPACITY, &theme.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("opacity {tier:?}: {e}"));
        conformance(&SWAP_SLIDE, &theme.motion, tier, SWAP_DY, 0.0_f32)
            .unwrap_or_else(|e| panic!("slide {tier:?}: {e}"));
    }
    let full = theme
        .motion
        .resolve(&SWAP_OPACITY, MotionTier::Full)
        .expect("full");
    assert_eq!(full.duration.as_millis(), 350, "moderate");
    assert_eq!(full.easing, Easing::QuadOut);
    let slide = theme
        .motion
        .resolve(&SWAP_SLIDE, MotionTier::Full)
        .expect("full");
    assert_eq!(slide.duration.as_millis(), 350);
    assert_eq!(slide.easing, Easing::SineInOut);
    // Reduced keeps the opacity and drops the slide; Off keeps neither.
    assert!(
        theme
            .motion
            .resolve(&SWAP_OPACITY, MotionTier::Reduced)
            .is_some()
    );
    assert!(
        theme
            .motion
            .resolve(&SWAP_SLIDE, MotionTier::Reduced)
            .is_none()
    );
    assert!(
        theme
            .motion
            .resolve(&SWAP_OPACITY, MotionTier::Off)
            .is_none()
    );
    assert!(theme.motion.resolve(&SWAP_SLIDE, MotionTier::Off).is_none());
}

#[test]
fn reduced_fades_the_opacity_only_and_off_snaps() {
    let rest = body_y(&swapped(MotionTier::Full, 5000).scene);
    // Reduced: no slide, an opacity in flight.
    let r = swapped(MotionTier::Reduced, 0);
    assert_eq!(body_y(&r.scene), rest, "no slide");
    assert_eq!(body_alpha(&r.scene), Some(0));
    assert_eq!(r.out.repaint, Repaint::Now);
    let r = swapped(MotionTier::Reduced, 60);
    let a = body_alpha(&r.scene).expect("faded");
    assert!(a > 0 && a < 255, "{a}");
    assert_eq!(body_y(&r.scene), rest);
    let r = swapped(MotionTier::Reduced, 400);
    assert_eq!(body_alpha(&r.scene), Some(255));
    assert_eq!(r.out.repaint, Repaint::Idle);
    // Off: the swap lands at once.
    let o = swapped(MotionTier::Off, 0);
    assert_eq!(body_alpha(&o.scene), Some(255));
    assert_eq!(body_y(&o.scene), rest);
    assert_eq!(o.out.repaint, Repaint::Idle);
}

#[test]
fn a_swap_in_flight_restarts_from_the_look_it_has() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg::new();
    settled(&mut h, &c);
    h.at_ms(6000);
    frame(&mut h, &Cfg { key: 2, ..c });
    h.at_ms(6150);
    let mid = frame(&mut h, &Cfg { key: 2, ..c });
    let (a_mid, y_mid) = (body_alpha(&mid.scene).expect("faded"), body_y(&mid.scene));
    assert!(a_mid > 0 && a_mid < 255);
    // Another swap one frame later starts where the first was, not at 0.
    h.at_ms(6150);
    let again = frame(&mut h, &Cfg { key: 3, ..c });
    assert_eq!(body_alpha(&again.scene), Some(a_mid));
    assert!((body_y(&again.scene) - y_mid).abs() < 0.01);
    // And takes a whole `moderate` to settle.
    h.at_ms(6150 + MODERATE / 2);
    assert!(body_alpha(&frame(&mut h, &Cfg { key: 3, ..c }).scene) < Some(255));
    h.at_ms(6150 + MODERATE);
    let done = frame(&mut h, &Cfg { key: 3, ..c });
    assert_eq!(body_alpha(&done.scene), Some(255));
    assert_eq!(done.out.repaint, Repaint::Idle);
}

// ---- the limit: text and the rounded corners ----

/// The width the fake text system gives a shaped text.
fn width_of(spec: &Spec) -> f32 {
    FakeText.prepare(spec, 1.0).1.0
}

#[test]
fn the_text_the_page_draws_keeps_its_edges_8_from_the_card() {
    let mut h = UiHarness::new(WIN);
    for subtitle in [false, true] {
        let c = Cfg {
            subtitle,
            ..Cfg::new()
        };
        let out = settled(&mut h, &c);
        let card = card_rect(RECT);
        // Every text the page draws is one of these; the fake system reports the widths.
        let long = Spec::plain(
            c.title,
            Style {
                line: 24.0,
                ..Style::ui(DARK.font_title, DARK.fg_bright).medium()
            },
        );
        let sub = Spec::plain(
            "Last 30 days",
            Style {
                line: 20.0,
                ..Style::ui(DARK.font_base, DARK.fg_tertiary)
            },
        );
        let body = Spec::plain("Body line", body_style(255));
        let known = [long, sub, body];
        for t in texts(&out.scene) {
            let Some(spec) = known.iter().find(|s| key_of(s, 1.0) == t.key) else {
                continue; // a button label: the caller's
            };
            let w = width_of(spec);
            assert!(t.x - card.x >= RADIUS, "left edge {}", t.x);
            assert!(card.right() - (t.x + w) >= RADIUS, "right edge {}", t.x + w);
        }
    }
}

#[test]
fn the_body_is_cut_by_the_rounded_clip_at_the_cards_rect() {
    let mut h = UiHarness::new(WIN);
    // No footer: the body reaches the card's bottom corners, where a fill is cut.
    let c = Cfg {
        footer: false,
        ..Cfg::new()
    };
    let mut ran = false;
    h.frame(|_| {});
    h.at_ms(10);
    let (scene, _) = h.frame(|ui| {
        let opts = PageOptions {
            title: "T",
            subtitle: None,
            active: true,
            glass: GLASS,
            under: UNDER,
            actions_w: 0.0,
            footer: false,
            swap_key: 1,
            name: "",
        };
        ui.page("page", c.rect, &opts, |ui, slot| {
            if let PageSlot::Body(view, _) = slot {
                ran = true;
                let r = view.viewport;
                ui.painter().fill(
                    Rect::new(r.x, r.bottom() - 40.0, r.w, 40.0),
                    0.0,
                    [200, 0, 0, 255],
                );
            }
            100.0
        });
    });
    assert!(ran);
    let fills: Vec<_> = scene.layers[0]
        .geometry
        .under
        .iter()
        .filter(|s| s.color == premul([200, 0, 0, 255]))
        .collect();
    assert_eq!(fills.len(), 1);
    let f = fills[0];
    let card = card_rect(c.rect);
    assert_eq!(
        f.round,
        [card.x, card.y, card.w, card.h],
        "the clip is the card's rect"
    );
    assert_eq!(f.round_radius, RADIUS);
}

// ---- the disabled walk and keys ----

#[test]
fn a_disabled_page_is_disabled_all_the_way_down() {
    let mut h = UiHarness::new(WIN);
    let c = Cfg {
        enabled: false,
        ..Cfg::new()
    };
    let out = settled(&mut h, &c);
    let bad = disabled_violations(&out.out.tree, page_id());
    assert!(bad.is_empty(), "{bad:?}");
    // Nothing takes focus.
    let out = tab(&mut h, &c);
    assert_eq!(focused(&out.out.tree), None);
    // And an enabled one is not marked disabled.
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new());
    assert!(
        !out.out
            .tree
            .get(page_id())
            .expect("group")
            .states
            .contains(State::Disabled)
    );
}

#[test]
fn the_page_takes_no_keys_of_its_own() {
    // A global binding on each arrow and page key fires with focus outside the page.
    for k in [
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::PageDown,
        NamedKey::Home,
    ] {
        let mut h = UiHarness::new(WIN);
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        let c = Cfg::new();
        settled(&mut h, &c);
        h.input.key_down(Key::Named(k), None);
        h.at_ms(5100);
        let out = frame(&mut h, &c);
        assert_eq!(out.out.fired.len(), 1, "{k:?}");
    }
    // A disabled page consumes none either.
    for k in [
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::Enter,
        NamedKey::Space,
    ] {
        let mut h = UiHarness::new(WIN);
        h.shortcuts
            .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        let c = Cfg {
            enabled: false,
            ..Cfg::new()
        };
        settled(&mut h, &c);
        h.input.key_down(Key::Named(k), None);
        h.at_ms(5100);
        let out = frame(&mut h, &c);
        assert_eq!(out.out.fired.len(), 1, "{k:?} on a disabled page");
    }
}

#[test]
fn the_layer_is_the_one_the_app_draws_on() {
    // The page draws nowhere but the layer it is called on.
    let mut h = UiHarness::new(WIN);
    let out = settled(&mut h, &Cfg::new());
    for (i, l) in out.scene.in_order().into_iter().enumerate() {
        if i != LayerId::Base as usize {
            assert!(
                l.texts.is_empty() && l.geometry.under.is_empty(),
                "layer {i}"
            );
        }
    }
}

//! The popover (`overlay.popover` acceptance): placement known answers, dismissal by press,
//! Escape and focus loss, focus restore and trap, exit retention, motion per tier, pixel probes
//! at scale 2 and the semantic tree.

use lune_ui_core::access::{AccessAction, Action, Node, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Dismissal, Sense, Ui, WidgetId};
use lune_ui_core::widgets::dropdown::panel_rect;
use lune_ui_core::widgets::menu::{CLOSE, OPEN};
use lune_ui_core::widgets::popover::{
    PlaceAlign, PlaceRequest, Placement, PopoverOptions, Side, Surface, place,
};
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};

const WIN: (f32, f32) = (800.0, 600.0);
const ANCHOR: Rect = Rect::new(100.0, 100.0, 200.0, 32.0);
const SIZE: (f32, f32) = (220.0, 120.0);
const P: PointerButton = PointerButton::Primary;
const PAGE_BTN: Rect = Rect::new(500.0, 300.0, 80.0, 30.0);

fn win() -> Rect {
    Rect::new(0.0, 0.0, WIN.0, WIN.1)
}

fn req(anchor: Rect, size: (f32, f32)) -> PlaceRequest {
    PlaceRequest {
        anchor,
        size,
        side: Side::Bottom,
        align: PlaceAlign::Start,
        gap: 4.0,
        margin: 8.0,
        shift: true,
        window: win(),
    }
}

fn anchor_id() -> WidgetId {
    WidgetId::ROOT.with("anchor")
}

fn pop_id(key: &str) -> WidgetId {
    WidgetId::ROOT.with(key)
}

/// The popover with two buttons inside, a list node, and what the frame reported.
#[derive(Debug, Default)]
struct Out {
    scene: Scene,
    repaint: Option<Repaint>,
    dismissed: Option<Dismissal>,
    placement: Option<Placement>,
    page_clicked: bool,
    anchor_focused: bool,
    focused: Option<WidgetId>,
    inner_clicked: bool,
    tree: Option<lune_ui_core::access::SemanticTree>,
}

fn inner(key: &str, n: u8) -> WidgetId {
    WidgetId::ROOT.with(key).with(("inner", n))
}

fn node(ui: &mut Ui<'_>, id: WidgetId, r: Rect) {
    let mut n = Node::new(id, Role::Button, "b", r);
    n.states.insert(State::Focusable);
    ui.node(n);
}

fn content(ui: &mut Ui<'_>, key: &str, rect: Rect) -> bool {
    let button = |ui: &mut Ui<'_>, n: u8, y: f32| {
        let r = Rect::new(rect.x + 10.0, rect.y + y, 80.0, 30.0);
        let resp = ui.interact(inner(key, n), r, Sense::CLICK_FOCUS);
        node(ui, inner(key, n), r);
        resp
    };
    let a = button(ui, 0, 10.0);
    let b = button(ui, 1, 50.0);
    ui.node(Node::new(
        pop_id(key).with("list"),
        Role::List,
        "items",
        rect,
    ));
    a.clicked || b.clicked
}

/// One frame: the page (anchor button, a page button) and the popovers in `open`, anchored at
/// `anchors[i]`.
fn run(
    h: &mut UiHarness,
    ms: u64,
    open: &[(&str, Rect)],
    opts: impl Fn(&mut PopoverOptions),
) -> Out {
    let mut out = Out::default();
    let (scene, fo) = h.at_ms(ms).frame(|ui| {
        ui.painter().on(LayerId::Page);
        let a = ui.interact(anchor_id(), ANCHOR, Sense::CLICK_FOCUS);
        node(ui, anchor_id(), ANCHOR);
        let b = ui.interact(WidgetId::ROOT.with("page"), PAGE_BTN, Sense::CLICK_FOCUS);
        out.page_clicked = b.clicked;
        out.anchor_focused = a.focused;
        for (key, anchor) in open {
            let mut o = PopoverOptions::new(*anchor, SIZE);
            o.anchor_id = Some(anchor_id());
            opts(&mut o);
            let (frame, placement, clicked) =
                ui.popover(*key, &o, |ui, rect| content(ui, key, rect));
            out.dismissed = out.dismissed.or(frame.dismissed);
            out.placement = Some(placement);
            out.inner_clicked |= clicked;
        }
    });
    out.scene = scene;
    out.repaint = Some(fo.repaint);
    out.focused = fo.focused;
    out.tree = Some(fo.tree);
    out
}

fn open_one(h: &mut UiHarness, ms: u64) -> Out {
    run(h, ms, &[("pop", ANCHOR)], |_| {})
}

fn popup_fill_alpha(scene: &Scene) -> Option<f32> {
    scene
        .merged(LayerId::Popup)
        .geometry
        .under
        .first()
        .map(|s| s.color[3])
}

#[test]
fn placement_known_answers() {
    let below = place(&req(ANCHOR, SIZE));
    assert_eq!(
        (below.rect.y, below.flipped, below.shifted),
        (136.0, false, false)
    );
    let flipped = place(&req(Rect::new(100.0, 500.0, 200.0, 32.0), SIZE));
    assert_eq!((flipped.rect.y, flipped.flipped), (376.0, true));
    let clamped = place(&req(Rect::new(100.0, 20.0, 200.0, 32.0), (220.0, 700.0)));
    assert_eq!((clamped.rect.y, clamped.overflow), (8.0, true));
    let shifted = place(&req(Rect::new(790.0, 100.0, 200.0, 32.0), SIZE));
    assert_eq!((shifted.rect.x, shifted.shifted), (572.0, true));
    // A popover that fits is never flagged as overflowing.
    assert!(!below.overflow && !flipped.overflow && !shifted.overflow);
}

#[test]
fn the_dropdown_panel_is_the_place_answer() {
    let mut text = FakeText;
    let p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    for anchor in [ANCHOR, Rect::new(34.0, 560.0, 210.0, 32.0)] {
        let panel = panel_rect(&p, anchor, 4);
        let placed = place(&PlaceRequest {
            shift: false,
            ..req(anchor, (anchor.w, 4.0 * 32.0 + 8.0))
        });
        assert_eq!(panel, placed.rect);
    }
}

#[test]
fn the_popover_sits_where_place_says_and_reports_it() {
    let mut h = UiHarness::new(WIN);
    open_one(&mut h, 0);
    let out = open_one(&mut h, 400);
    let pl = out.placement.expect("placed");
    assert_eq!(pl, place(&req(ANCHOR, SIZE)));
    let panel = &out.scene.merged(LayerId::Popup).geometry.under[0];
    assert_eq!(panel.rect[..2], [pl.rect.x, pl.rect.y]);
}

#[test]
fn an_outside_press_dismisses_once_and_reaches_nothing() {
    let mut h = UiHarness::new(WIN);
    open_one(&mut h, 0);
    open_one(&mut h, 300);
    h.input.pointer_to(540.0, 315.0);
    open_one(&mut h, 310);
    h.input.click(P);
    let out = open_one(&mut h, 320);
    assert_eq!(out.dismissed, Some(Dismissal::OutsidePress));
    assert!(!out.page_clicked, "the dismissing press is consumed");
    // The app stops calling it; nothing dismisses again.
    let out = run(&mut h, 330, &[], |_| {});
    assert_eq!(out.dismissed, None);
}

#[test]
fn a_press_inside_the_popover_does_not_dismiss() {
    let mut h = UiHarness::new(WIN);
    let pl = open_one(&mut h, 0).placement.expect("placed");
    open_one(&mut h, 300);
    h.input.pointer_to(pl.rect.x + 40.0, pl.rect.y + 20.0);
    open_one(&mut h, 310);
    h.input.click(P);
    let out = open_one(&mut h, 320);
    assert_eq!(out.dismissed, None);
    assert!(out.inner_clicked);
}

#[test]
fn escape_closes_only_the_topmost_of_two() {
    let mut h = UiHarness::new(WIN);
    let both = [("a", ANCHOR), ("b", Rect::new(100.0, 300.0, 200.0, 32.0))];
    run(&mut h, 0, &both, |_| {});
    run(&mut h, 300, &both, |_| {});
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let mut dismissed = Vec::new();
    h.at_ms(310).frame(|ui| {
        for (key, anchor) in &both {
            let (frame, _, _) = ui.popover(*key, &PopoverOptions::new(*anchor, SIZE), |_, _| {});
            dismissed.push((*key, frame.dismissed));
        }
    });
    assert_eq!(dismissed, vec![("a", None), ("b", Some(Dismissal::Escape))]);
}

#[test]
fn focus_loss_dismisses() {
    let mut h = UiHarness::new(WIN);
    open_one(&mut h, 0);
    open_one(&mut h, 300);
    h.input.focus(false);
    let out = open_one(&mut h, 310);
    assert_eq!(out.dismissed, Some(Dismissal::FocusLoss));
}

#[test]
fn closing_restores_focus_to_the_anchor_and_tab_stays_inside() {
    let mut h = UiHarness::new(WIN);
    run(&mut h, 0, &[], |_| {});
    h.input.pointer_to(150.0, 116.0).click(P);
    let out = run(&mut h, 10, &[], |_| {});
    assert_eq!(out.focused, Some(anchor_id()));
    open_one(&mut h, 20);
    let out = open_one(&mut h, 30);
    assert_eq!(out.focused, Some(inner("pop", 0)), "focus moves in");
    let mut seen = Vec::new();
    for i in 0..4 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        seen.push(open_one(&mut h, 40 + i * 10).focused);
    }
    assert_eq!(
        seen,
        vec![
            Some(inner("pop", 1)),
            Some(inner("pop", 0)),
            Some(inner("pop", 1)),
            Some(inner("pop", 0)),
        ],
        "Tab cycles inside the popover"
    );
    let out = run(&mut h, 100, &[], |_| {});
    assert_eq!(
        out.focused,
        Some(anchor_id()),
        "closing restores the anchor"
    );
}

/// Opens, settles, stops calling at 1000 ms and steps 10 ms frames: ms until idle and the closing
/// frames' fill alphas.
fn close_run(tier: MotionTier) -> (u64, Vec<f32>, Scene) {
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    open_one(&mut h, 0);
    open_one(&mut h, 300);
    let mut alphas = Vec::new();
    let mut t = 1000;
    loop {
        let out = run(&mut h, t, &[], |_| {});
        if let Some(a) = popup_fill_alpha(&out.scene) {
            alphas.push(a);
        }
        if out.repaint == Some(Repaint::Idle) {
            return (t - 1000, alphas, out.scene);
        }
        t += 10;
        assert!(t < 2000, "never settled");
    }
}

#[test]
fn a_popover_the_app_stops_calling_finishes_its_dismiss_then_idles() {
    for (tier, ms) in [
        (MotionTier::Full, 120),
        (MotionTier::Reduced, 120),
        (MotionTier::Off, 0),
    ] {
        let (idle_after, alphas, last) = close_run(tier);
        // Off removes it at once, but focus returns after the page drew, so one more frame asks.
        assert_eq!(idle_after, if ms == 0 { 10 } else { ms }, "{tier:?}");
        assert_eq!(alphas.len() as u64, ms / 10, "{tier:?}");
        assert!(
            alphas.windows(2).all(|w| w[1] <= w[0]),
            "{tier:?}: {alphas:?}"
        );
        assert!(
            last.merged(LayerId::Popup).geometry.under.is_empty(),
            "{tier:?}: nothing drawn once idle"
        );
    }
}

#[test]
fn the_exit_replays_the_content_and_hits_nothing() {
    let mut h = UiHarness::new(WIN);
    open_one(&mut h, 0);
    let open = open_one(&mut h, 300);
    let closing = run(&mut h, 1000, &[], |_| {});
    let (o, c) = (
        &open.scene.merged(LayerId::Popup).geometry.under,
        &closing.scene.merged(LayerId::Popup).geometry.under,
    );
    assert_eq!(o.len(), c.len());
    assert_eq!(o[0].radii, c[0].radii);
    // A press on the closing popover's inner button reaches the page under it, not the popover.
    h.input
        .pointer_to(PAGE_BTN.x + 5.0, PAGE_BTN.y + 5.0)
        .click(P);
    let out = run(&mut h, 1010, &[], |_| {});
    assert!(out.page_clicked);
}

#[test]
fn the_curves_conform_under_every_tier_and_opacity_is_monotone_through_the_ui() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for (row, a, b) in [(OPEN, 0.0_f32, 1.0), (CLOSE, 1.0, 0.0)] {
            conformance(&row, &DARK.motion, tier, a, b)
                .unwrap_or_else(|e| panic!("{tier:?} {row:?}: {e}"));
        }
    }
    let mut h = UiHarness::new(WIN);
    open_one(&mut h, 0);
    let mut last = 0.0;
    let mut settled = None;
    for k in 1..=15_u64 {
        let dt = (k * 1000 + 30) / 60;
        let out = open_one(&mut h, dt);
        let a = popup_fill_alpha(&out.scene).expect("drawn");
        assert!(a >= last && a.is_finite(), "{dt}: {a} after {last}");
        last = a;
        if dt >= 180 + 17 {
            settled.get_or_insert(a);
            assert_eq!(Some(a), settled);
            assert_eq!(out.repaint, Some(Repaint::Idle), "{dt}");
        }
    }
}

#[test]
fn reduced_fades_opacity_only_and_off_snaps() {
    let mut h = UiHarness::new(WIN);
    h.tier = MotionTier::Reduced;
    open_one(&mut h, 0);
    let out = open_one(&mut h, 40);
    let pl = out.placement.expect("placed");
    let fill = out.scene.merged(LayerId::Popup).geometry.under[0];
    assert_eq!(fill.rect, [pl.rect.x, pl.rect.y, pl.rect.w, pl.rect.h]);
    assert!(fill.color[3] < 1.0);

    let mut h = UiHarness::new(WIN);
    h.tier = MotionTier::Off;
    open_one(&mut h, 0);
    let out = open_one(&mut h, 10);
    assert_eq!(out.repaint, Some(Repaint::Idle));
}

#[test]
fn full_tier_opens_from_4_px_lower_at_97_percent() {
    let mut h = UiHarness::new(WIN);
    let first = open_one(&mut h, 0);
    let pl = first.placement.expect("placed");
    let fill = first.scene.merged(LayerId::Popup).geometry.under[0];
    assert!(
        (fill.rect[1] - (pl.rect.y + 4.0)).abs() < 1e-3,
        "{:?}",
        fill.rect
    );
    assert!((fill.rect[2] - pl.rect.w * 0.97).abs() < 1e-3);
}

#[test]
fn pixel_probes_at_scale_2() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    open_one(&mut h, 0);
    let out = open_one(&mut h, 400);
    let fill = out.scene.merged(LayerId::Popup).geometry.under[0];
    // Gap: 4 logical px = 8 device px under the anchor.
    assert_eq!(fill.rect[1] - ANCHOR.bottom() * 2.0, 8.0);
    assert_eq!(fill.radii, [16.0; 4], "surface radius 16 device px");
    // Margin: 8 logical px = 16 device px from the window edge when shifted.
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let edge = Rect::new(790.0, 100.0, 200.0, 32.0);
    run(&mut h, 0, &[("pop", edge)], |_| {});
    let out = run(&mut h, 400, &[("pop", edge)], |_| {});
    let fill = out.scene.merged(LayerId::Popup).geometry.under[0];
    assert_eq!(WIN.0 * 2.0 - (fill.rect[0] + fill.rect[2]), 16.0);
    // And the flipped-clamped one sits on the top margin.
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let tall = |o: &mut PopoverOptions| o.size = (220.0, 700.0);
    let near_top = Rect::new(100.0, 20.0, 200.0, 32.0);
    run(&mut h, 0, &[("pop", near_top)], tall);
    let out = run(&mut h, 400, &[("pop", near_top)], tall);
    let fill = out.scene.merged(LayerId::Popup).geometry.under[0];
    assert_eq!(fill.rect[1], 16.0);
}

#[test]
fn surface_none_draws_nothing_of_its_own() {
    let mut h = UiHarness::new(WIN);
    let none = |o: &mut PopoverOptions| o.surface = Surface::None;
    run(&mut h, 0, &[("pop", ANCHOR)], none);
    let out = run(&mut h, 400, &[("pop", ANCHOR)], none);
    assert!(
        out.scene.merged(LayerId::Popup).geometry.under.is_empty(),
        "the content here draws no shape"
    );
}

#[test]
fn the_content_node_appears_while_open_and_an_access_dismiss_closes_it() {
    let mut h = UiHarness::new(WIN);
    let list = pop_id("pop").with("list");
    let closed = run(&mut h, 0, &[], |_| {});
    assert!(closed.tree.expect("tree").get(list).is_none());
    open_one(&mut h, 10);
    let out = open_one(&mut h, 20);
    let tree = out.tree.expect("tree");
    let node = tree.get(list).expect("content node while open");
    assert_eq!(node.role, Role::List);
    h.input.access(AccessAction {
        target: list,
        action: Action::Dismiss,
        value: None,
    });
    let out = open_one(&mut h, 30);
    assert_eq!(out.dismissed, Some(Dismissal::Access));
    let out = run(&mut h, 40, &[], |_| {});
    assert!(
        out.tree.expect("tree").get(list).is_none(),
        "hidden once closed"
    );
}

#[test]
fn a_popover_anchored_on_a_response_in_a_scrolled_area_opens_at_it_on_screen() {
    let opts = ScrollOptions {
        extent: Some((400.0, 1000.0)),
        ..ScrollOptions::default()
    };
    let frame = |h: &mut UiHarness, ms: u64| {
        let mut placed = None;
        h.at_ms(ms).frame(|ui| {
            ui.scroll_area("s", Rect::new(0.0, 0.0, 400.0, 200.0), &opts, |ui, _| {
                let rect = Rect::new(10.0, 150.0, 100.0, 32.0);
                let r = ui.interact(anchor_id(), rect, Sense::CLICK_FOCUS);
                let o = PopoverOptions::new(r.rect, (120.0, 60.0));
                placed = Some(ui.popover("p", &o, |_, _| ()).1);
                ((), Extent::default())
            });
        });
        placed.expect("ran")
    };
    let mut h = UiHarness::new(WIN);
    h.input.pointer_to(390.0, 190.0);
    frame(&mut h, 0);
    h.input.scroll_pixels(0.0, -100.0);
    frame(&mut h, 5);
    let placed = frame(&mut h, 10);
    // Content y 150 at offset 100 is screen y 50; 32 high, gap 4.
    assert_eq!((placed.rect.x, placed.rect.y), (10.0, 86.0));
}

#[test]
fn a_glass_popover_blurs_what_is_under_it_and_replays_the_blur_while_it_closes() {
    let glass = |o: &mut PopoverOptions| o.surface = Surface::Glass;
    let popup = LayerId::Popup;
    let mut h = UiHarness::new(WIN);
    run(&mut h, 0, &[("pop", ANCHOR)], glass);
    let out = run(&mut h, 400, &[("pop", ANCHOR)], glass);
    let pl = out.placement.expect("placed");
    let under = &out.scene.merged(popup).geometry.under;
    assert_eq!(under[0].color, premul(DARK.menu_glass));
    assert_eq!(under[0].radii, [9.0; 4]);
    assert_eq!(under[1].color, premul(DARK.menu_popup_border), "the ring");
    let open = out.scene.backdrop.expect("the blur request");
    assert_eq!(open.layer, popup as usize);
    assert_eq!(open.rect, [pl.rect.x, pl.rect.y, pl.rect.w, pl.rect.h]);
    assert_eq!(open.radius, 9.0);
    // The app stops calling it: the blur is replayed with the fade, then gone.
    run(&mut h, 1000, &[], |_| {});
    let closing = run(&mut h, 1060, &[], |_| {});
    let blur = closing.scene.backdrop.expect("blur while it fades");
    assert!(
        blur.alpha < open.alpha,
        "{} against {}",
        blur.alpha,
        open.alpha
    );
    assert!(
        blur.rect[2] < open.rect[2] && blur.rect[2] > 0.97 * open.rect[2],
        "scaled with the close"
    );
    assert_eq!(closing.repaint, Some(Repaint::Now));
    let idle = run(&mut h, 1300, &[], |_| {});
    assert!(idle.scene.backdrop.is_none());
    // The default surface asks for no blur.
    let mut h = UiHarness::new(WIN);
    open_one(&mut h, 0);
    assert!(open_one(&mut h, 400).scene.backdrop.is_none());
}

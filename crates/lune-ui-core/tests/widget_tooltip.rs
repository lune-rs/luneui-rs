//! The tooltip and the hover dwell (`overlay.tooltip` acceptance): the 800 ms delay, leaving and
//! moving between anchors, disabled and keyboard-focused anchors, press and Escape hiding it,
//! placement, motion per tier, no hit region and no focus, and the semantic nodes.

use std::time::Duration;

use lune_ui_core::access::{Node, Role, SemanticTree, State};
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::MotionTier;
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Sense, Ui, WidgetId};
use lune_ui_core::widgets::popover::PopoverOptions;
use lune_ui_core::widgets::scroll_area::{Extent, ScrollOptions};
use lune_ui_core::widgets::tooltip::LAZY_HOVER_DWELL;

const WIN: (f32, f32) = (800.0, 600.0);
const A: Rect = Rect::new(100.0, 100.0, 32.0, 32.0);
const B: Rect = Rect::new(300.0, 100.0, 32.0, 32.0);
const P: PointerButton = PointerButton::Primary;
const TEXT: &str = "Rename";

fn id(key: &str) -> WidgetId {
    WidgetId::ROOT.with(key)
}

#[derive(Clone, Copy)]
struct Anchor {
    key: &'static str,
    rect: Rect,
    enabled: bool,
    /// The node's name; empty for an icon-only button.
    name: &'static str,
    /// Whether the app calls it this frame.
    called: bool,
}

fn anchor(key: &'static str, rect: Rect) -> Anchor {
    Anchor {
        key,
        rect,
        enabled: true,
        name: "",
        called: true,
    }
}

struct Out {
    scene: Scene,
    repaint: Repaint,
    tree: SemanticTree,
    focused: Option<WidgetId>,
    under_clicked: bool,
}

impl Out {
    fn shown(&self) -> bool {
        !self.scene.merged(LayerId::Popup).geometry.under.is_empty()
    }
}

/// A page with a click region under everything, then `anchors`, each with its own tooltip text
/// (`TEXT` for "a", "other" for the rest).
fn frame(h: &mut UiHarness, ms: u64, anchors: &[Anchor]) -> Out {
    let mut under_clicked = false;
    let (scene, fo) = h.at_ms(ms).frame(|ui| {
        ui.painter().on(LayerId::Page);
        let under = ui.interact(
            id("under"),
            Rect::new(0.0, 136.0, 800.0, 300.0),
            Sense::CLICK,
        );
        under_clicked = under.clicked;
        for a in anchors.iter().filter(|a| a.called) {
            let resp = ui.with_enabled(a.enabled, |ui| {
                let r = ui.interact(id(a.key), a.rect, Sense::CLICK_FOCUS);
                let mut n = Node::new(id(a.key), Role::Button, a.name, a.rect);
                n.states.insert(State::Focusable);
                ui.node(n);
                r
            });
            ui.tooltip(&resp, if a.key == "a" { TEXT } else { "other" });
        }
    });
    Out {
        scene,
        repaint: fo.repaint,
        tree: fo.tree,
        focused: fo.focused,
        under_clicked,
    }
}

fn centre(r: Rect) -> (f32, f32) {
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

fn hover(h: &mut UiHarness, r: Rect) {
    let (x, y) = centre(r);
    h.input.pointer_to(x, y);
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// Hovers `A` from 0 and returns the harness at 10 ms, pending.
fn hovering() -> UiHarness {
    let mut h = UiHarness::new(WIN);
    hover(&mut h, A);
    frame(&mut h, 0, &[anchor("a", A)]);
    frame(&mut h, 10, &[anchor("a", A)]);
    h
}

#[test]
fn the_delay_is_800_ms_of_hover() {
    let mut h = hovering();
    let pending = frame(&mut h, 20, &[anchor("a", A)]);
    assert!(!pending.shown());
    assert_eq!(
        pending.repaint,
        Repaint::At(ms(800)),
        "pending asks for the due time, not Now"
    );
    assert!(!frame(&mut h, 799, &[anchor("a", A)]).shown(), "799 ms");
    let at = frame(&mut h, 800, &[anchor("a", A)]);
    assert!(at.shown(), "800 ms");
    assert_eq!(at.repaint, Repaint::Now, "appearing");
    let settled = frame(&mut h, 1200, &[anchor("a", A)]);
    assert!(settled.shown());
    assert_eq!(settled.repaint, Repaint::Idle);
}

#[test]
fn leaving_starts_the_dismiss_at_once_and_it_is_hidden_after_120_ms() {
    let mut h = hovering();
    frame(&mut h, 800, &[anchor("a", A)]);
    frame(&mut h, 1200, &[anchor("a", A)]);
    h.input.pointer_to(600.0, 500.0);
    let leave = frame(&mut h, 2000, &[anchor("a", A)]);
    assert!(leave.shown(), "the dismiss fade starts, it is not cut");
    assert_eq!(leave.repaint, Repaint::Now);
    assert!(frame(&mut h, 2119, &[anchor("a", A)]).shown());
    let gone = frame(&mut h, 2120, &[anchor("a", A)]);
    assert!(!gone.shown(), "hidden after 120 ms");
    let idle = frame(&mut h, 2130, &[anchor("a", A)]);
    assert_eq!(idle.repaint, Repaint::Idle);
}

#[test]
fn moving_to_another_anchor_restarts_the_timer() {
    let mut h = hovering();
    frame(&mut h, 500, &[anchor("a", A)]);
    hover(&mut h, B);
    let both = [anchor("a", A), anchor("b", B)];
    frame(&mut h, 510, &both);
    assert!(
        !frame(&mut h, 800, &both).shown(),
        "A's 800 ms do not count for B"
    );
    assert!(!frame(&mut h, 1309, &both).shown());
    assert!(
        frame(&mut h, 1310, &both).shown(),
        "800 ms after arriving on B"
    );
}

#[test]
fn an_anchor_the_app_stops_calling_never_shows_and_calling_again_restarts() {
    let mut h = hovering();
    frame(&mut h, 500, &[anchor("a", A)]);
    let gone = Anchor {
        called: false,
        ..anchor("a", A)
    };
    let out = frame(&mut h, 700, &[gone]);
    assert!(!out.shown());
    frame(&mut h, 900, &[anchor("a", A)]);
    assert!(
        !frame(&mut h, 1000, &[anchor("a", A)]).shown(),
        "the old 800 ms is gone"
    );
    assert!(!frame(&mut h, 1699, &[anchor("a", A)]).shown());
    assert!(frame(&mut h, 1700, &[anchor("a", A)]).shown());
}

#[test]
fn a_press_hides_it_until_the_pointer_leaves() {
    let mut h = hovering();
    assert!(frame(&mut h, 900, &[anchor("a", A)]).shown());
    h.input.click(P);
    frame(&mut h, 1000, &[anchor("a", A)]);
    assert!(!frame(&mut h, 1200, &[anchor("a", A)]).shown());
    assert!(
        !frame(&mut h, 5000, &[anchor("a", A)]).shown(),
        "still over the anchor"
    );
    h.input.pointer_to(600.0, 500.0);
    frame(&mut h, 5010, &[anchor("a", A)]);
    hover(&mut h, A);
    frame(&mut h, 5020, &[anchor("a", A)]);
    assert!(!frame(&mut h, 5819, &[anchor("a", A)]).shown());
    assert!(
        frame(&mut h, 5820, &[anchor("a", A)]).shown(),
        "the delay runs again"
    );
}

#[test]
fn escape_hides_it_the_same_way() {
    let mut h = hovering();
    assert!(frame(&mut h, 900, &[anchor("a", A)]).shown());
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    // The Ui reports the dismissal during the frame; the next frame starts the fade.
    let esc = frame(&mut h, 1000, &[anchor("a", A)]);
    assert_eq!(esc.repaint, Repaint::Now);
    frame(&mut h, 1010, &[anchor("a", A)]);
    assert!(!frame(&mut h, 1130, &[anchor("a", A)]).shown());
    assert!(!frame(&mut h, 9000, &[anchor("a", A)]).shown());
}

#[test]
fn a_disabled_anchor_shows_it() {
    let mut h = UiHarness::new(WIN);
    let a = Anchor {
        enabled: false,
        ..anchor("a", A)
    };
    hover(&mut h, A);
    frame(&mut h, 0, &[a]);
    frame(&mut h, 10, &[a]);
    assert!(!frame(&mut h, 799, &[a]).shown());
    assert!(frame(&mut h, 800, &[a]).shown());
}

#[test]
fn a_focus_visible_anchor_shows_it_after_the_same_delay() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, &[anchor("a", A)]);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let out = frame(&mut h, 10, &[anchor("a", A)]);
    assert_eq!(out.focused, Some(id("a")));
    assert!(!frame(&mut h, 809, &[anchor("a", A)]).shown());
    assert!(frame(&mut h, 810, &[anchor("a", A)]).shown());
}

#[test]
fn it_has_no_hit_region_and_takes_no_focus() {
    let mut h = hovering();
    let shown = frame(&mut h, 900, &[anchor("a", A)]);
    assert!(shown.shown());
    // The pointer moves onto the tooltip's own panel: the page under it gets the hover and click.
    let panel = shown.scene.merged(LayerId::Popup).geometry.under[0].rect;
    h.input
        .pointer_to(panel[0] + panel[2] / 2.0, panel[1] + panel[3] / 2.0);
    frame(&mut h, 910, &[anchor("a", A)]);
    h.input.click(P);
    let out = frame(&mut h, 920, &[anchor("a", A)]);
    assert!(out.under_clicked, "a tooltip never covers what is under it");
    assert_eq!(out.focused, shown.focused, "it never takes focus");
}

#[test]
fn it_stays_inside_the_window_flipped_and_shifted() {
    let size = |text: &str| {
        let mut t = FakeText;
        let s = t.measure(
            &Spec::plain(text, Style::ui(DARK.font_s, DARK.fg_primary)),
            1.0,
        );
        (s.0 + 20.0, s.1 + 16.0)
    };
    // Flipped above an anchor near the bottom edge.
    let low = Rect::new(100.0, 560.0, 32.0, 32.0);
    let mut h = UiHarness::new(WIN);
    hover(&mut h, low);
    for t in [0, 10, 800, 1200] {
        frame(&mut h, t, &[anchor("a", low)]);
    }
    let out = frame(&mut h, 1300, &[anchor("a", low)]);
    let panel = out.scene.merged(LayerId::Popup).geometry.under[0].rect;
    assert!(panel[1] + panel[3] <= low.y, "above the anchor: {panel:?}");
    // Shifted inside at the right edge.
    let edge = Rect::new(780.0, 100.0, 16.0, 32.0);
    let mut h = UiHarness::new(WIN);
    hover(&mut h, edge);
    for t in [0, 10, 800, 1200] {
        frame(&mut h, t, &[anchor("a", edge)]);
    }
    let out = frame(&mut h, 1300, &[anchor("a", edge)]);
    let panel = out.scene.merged(LayerId::Popup).geometry.under[0].rect;
    assert_eq!(panel[0] + panel[2], WIN.0 - 8.0, "8 px from the right edge");
    assert_eq!(panel[2], size(TEXT).0);
}

#[test]
fn pixel_probes_at_scale_2() {
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    hover(&mut h, A);
    for t in [0, 10, 800, 1200] {
        frame(&mut h, t, &[anchor("a", A)]);
    }
    let out = frame(&mut h, 1300, &[anchor("a", A)]);
    let layer = &out.scene.merged(LayerId::Popup);
    let (fill, ring) = (layer.geometry.under[0], layer.geometry.under[1]);
    assert_eq!(fill.radii, [16.0; 4], "radius 16 device px");
    assert_eq!(ring.params[1], 2.0, "2 px border");
    let mut t = FakeText;
    let text = t.measure(
        &Spec::plain(TEXT, Style::ui(DARK.font_s, DARK.fg_primary)),
        2.0,
    );
    assert_eq!(
        fill.rect[2],
        (text.0 + 20.0) * 2.0,
        "20 device px each side"
    );
    assert_eq!(fill.rect[3], (text.1 + 16.0) * 2.0);
    let item = layer.texts[0];
    // Text items are in logical px: 10 across and 8 down from the panel, which is 20 and 16
    // device px at scale 2.
    assert_eq!(item.x - fill.rect[0] / 2.0, 10.0);
    assert_eq!(item.y - fill.rect[1] / 2.0, 8.0);
}

#[test]
fn long_text_wraps_at_320() {
    let long = "word ".repeat(40);
    let mut h = UiHarness::new(WIN);
    hover(&mut h, A);
    let run = |h: &mut UiHarness, t: u64| {
        let (scene, _) = h.at_ms(t).frame(|ui| {
            ui.painter().on(LayerId::Page);
            let r = ui.interact(id("a"), A, Sense::CLICK_FOCUS);
            ui.tooltip(&r, &long);
        });
        scene
    };
    for t in [0, 10, 800, 1200] {
        run(&mut h, t);
    }
    let scene = run(&mut h, 1300);
    let fill = scene.merged(LayerId::Popup).geometry.under[0];
    assert_eq!(fill.rect[2], 320.0);
}

#[test]
fn the_fade_is_opacity_only_reduced_keeps_it_and_off_snaps() {
    for (tier, expect_fade) in [
        (MotionTier::Full, true),
        (MotionTier::Reduced, true),
        (MotionTier::Off, false),
    ] {
        let mut h = UiHarness::new(WIN);
        h.tier = tier;
        hover(&mut h, A);
        frame(&mut h, 0, &[anchor("a", A)]);
        frame(&mut h, 10, &[anchor("a", A)]);
        frame(&mut h, 790, &[anchor("a", A)]);
        let mid = frame(&mut h, 830, &[anchor("a", A)]);
        let fill = mid.scene.merged(LayerId::Popup).geometry.under[0];
        assert_eq!(
            fill.color[3] < DARK.dropdown_bg[3] as f32 / 255.0 - 1e-6,
            expect_fade,
            "{tier:?}"
        );
        // It never moves or scales.
        let settled = frame(&mut h, 1300, &[anchor("a", A)]);
        let rest = settled.scene.merged(LayerId::Popup).geometry.under[0];
        assert_eq!(fill.rect, rest.rect, "{tier:?}: opacity only");
        assert_eq!(settled.repaint, Repaint::Idle);
    }
}

#[test]
fn leaving_off_tier_hides_at_once() {
    let mut h = UiHarness::new(WIN);
    h.tier = MotionTier::Off;
    hover(&mut h, A);
    for t in [0, 10, 800, 900] {
        frame(&mut h, t, &[anchor("a", A)]);
    }
    h.input.pointer_to(600.0, 500.0);
    assert!(!frame(&mut h, 1000, &[anchor("a", A)]).shown());
}

#[test]
fn a_popover_over_the_anchor_blocks_the_hover() {
    let mut h = UiHarness::new(WIN);
    let run = |h: &mut UiHarness, t: u64| {
        let mut shown = false;
        let (scene, _) = h.at_ms(t).frame(|ui| {
            ui.painter().on(LayerId::Page);
            let r = ui.interact(id("a"), A, Sense::CLICK_FOCUS);
            ui.tooltip(&r, TEXT);
            let o = PopoverOptions::new(Rect::new(90.0, 60.0, 100.0, 20.0), (200.0, 120.0));
            ui.popover("pop", &o, |ui, rect| {
                ui.interact(id("pop-body"), rect, Sense::HOVER);
            });
        });
        shown |= scene.merged(LayerId::Popup).geometry.under.len() > 2;
        shown
    };
    hover(&mut h, A);
    for t in [0, 10, 800, 1200, 2000] {
        assert!(!run(&mut h, t), "the popover covers the anchor at {t}");
    }
}

#[test]
fn a_list_scrolling_under_a_parked_pointer_shows_none() {
    let mut h = UiHarness::new(WIN);
    h.input.pointer_to(50.0, 150.0);
    for k in 0..30u64 {
        let t = k * 100;
        let offset = (k * 20) as f32;
        let mut shown = false;
        let (scene, _) = h.at_ms(t).frame(|ui| {
            ui.painter().on(LayerId::Page);
            for row in 0..40u32 {
                let y = row as f32 * 20.0 - offset + 100.0;
                if !(0.0..600.0).contains(&y) {
                    continue;
                }
                let r = ui.interact(
                    id("row").with(row),
                    Rect::new(0.0, y, 200.0, 20.0),
                    Sense::CLICK,
                );
                ui.tooltip(&r, "row");
            }
        });
        shown |= !scene.merged(LayerId::Popup).geometry.under.is_empty();
        assert!(!shown, "frame {k}");
    }
}

#[test]
fn the_semantic_nodes_follow_the_tooltip() {
    let mut h = hovering();
    let hidden = frame(&mut h, 100, &[anchor("a", A)]);
    let tip = id("a").with("tooltip");
    // The overlay's id is under the page scope.
    let tip_id = WidgetId::ROOT.with(tip);
    assert!(hidden.tree.get(tip_id).is_none(), "no node while hidden");
    let a = hidden.tree.get(id("a")).expect("anchor");
    assert_eq!(
        a.description.as_deref(),
        Some(TEXT),
        "the description is the text"
    );
    assert_eq!(a.name, TEXT, "an icon-only anchor takes it as its name");
    frame(&mut h, 800, &[anchor("a", A)]);
    let shown = frame(&mut h, 900, &[anchor("a", A)]);
    let node = shown.tree.get(tip_id).expect("tooltip node while shown");
    assert_eq!((node.role, node.name.as_str()), (Role::Tooltip, TEXT));
    // A named anchor keeps its name.
    let named = Anchor {
        name: "Edit",
        ..anchor("a", A)
    };
    let out = frame(&mut h, 1000, &[named]);
    let a = out.tree.get(id("a")).expect("anchor");
    assert_eq!(
        (a.name.as_str(), a.description.as_deref()),
        ("Edit", Some(TEXT))
    );
}

#[test]
fn hover_dwell_counts_from_the_frame_it_turned_on_and_resets_when_off() {
    let mut h = UiHarness::new(WIN);
    let probe = |h: &mut UiHarness, t: u64, active: bool| {
        let mut got = (false, Repaint::Idle);
        let (_, fo) = h.at_ms(t).frame(|ui: &mut Ui<'_>| {
            got.0 = ui.hover_dwell(id("d"), active, LAZY_HOVER_DWELL);
        });
        got.1 = fo.repaint;
        got
    };
    assert_eq!(probe(&mut h, 0, true), (false, Repaint::At(ms(150))));
    assert!(!probe(&mut h, 149, true).0);
    assert_eq!(probe(&mut h, 150, true), (true, Repaint::Idle));
    assert!(!probe(&mut h, 200, false).0);
    assert_eq!(probe(&mut h, 210, true), (false, Repaint::At(ms(360))));
    assert!(!probe(&mut h, 359, true).0);
    assert!(probe(&mut h, 360, true).0);
    assert_eq!(LAZY_HOVER_DWELL, ms(150));
    assert_eq!(DARK.motion.tooltip_delay, ms(800));
}

/// One frame of a page scrolled by `offset`, with an anchor at content `y` in a 200 px area.
fn scrolled_tip(h: &mut UiHarness, ms: u64, y: f32) -> Out {
    let area = Rect::new(0.0, 0.0, 400.0, 200.0);
    let opts = ScrollOptions {
        extent: Some((400.0, 1000.0)),
        ..ScrollOptions::default()
    };
    let (scene, fo) = h.at_ms(ms).frame(|ui| {
        ui.painter().on(LayerId::Page);
        ui.scroll_area("s", area, &opts, |ui, _| {
            let rect = Rect::new(10.0, y, 100.0, 32.0);
            let r = ui.interact(id("a"), rect, Sense::CLICK_FOCUS);
            ui.node(Node::new(id("a"), Role::Button, "", rect));
            ui.tooltip(&r, TEXT);
            ((), Extent::default())
        });
    });
    Out {
        scene,
        repaint: fo.repaint,
        tree: fo.tree,
        focused: fo.focused,
        under_clicked: false,
    }
}

/// A harness whose scroll area sits at offset 100, the pointer at `at`, past the anchor's delay.
fn scrolled_and_waited(y: f32, at: (f32, f32)) -> Out {
    let mut h = UiHarness::new(WIN);
    h.input.pointer_to(390.0, 190.0);
    scrolled_tip(&mut h, 0, y);
    h.input.scroll_pixels(0.0, -100.0);
    scrolled_tip(&mut h, 5, y);
    scrolled_tip(&mut h, 10, y);
    h.input.pointer_to(at.0, at.1);
    scrolled_tip(&mut h, 20, y);
    scrolled_tip(&mut h, 30, y);
    scrolled_tip(&mut h, 2000, y)
}

#[test]
fn a_tooltip_in_a_scrolled_area_shows_for_the_anchor_on_screen() {
    // Content y 150 at offset 100 is screen y 50.
    let out = scrolled_and_waited(150.0, (50.0, 66.0));
    assert!(out.shown());
    let tip = out.tree.nodes.iter().find(|n| n.role == Role::Tooltip);
    assert_eq!(tip.expect("the tooltip's node").bounds.y, 86.0);
}

#[test]
fn a_tooltip_in_a_scrolled_area_ignores_a_point_one_offset_away_from_its_anchor() {
    // Content y 250 at offset 100 is screen y 150; screen y 66 is what a second translate gives.
    assert!(!scrolled_and_waited(250.0, (50.0, 66.0)).shown());
}

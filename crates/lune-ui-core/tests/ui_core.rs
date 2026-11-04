//! The `Ui` frame, part A (C4 and core.ui acceptance): ids and scopes, the retained store and
//! keep-alive, layered hit testing with capture, `Response`, animated values, `Repaint` and the
//! clipboard requests. Driven through `UiHarness` with test widgets.

use std::collections::BTreeSet;
use std::time::Duration;

use lune_ui_core::input::{ClipboardRequest, PointerButton, Repaint};
use lune_ui_core::motion::{Kind, Motion, MotionTier};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::{Response, Sense, Ui, WidgetId};
use lune_ui_core::widgets;

const P: PointerButton = PointerButton::Primary;

fn harness() -> UiHarness {
    UiHarness::new((400.0, 300.0))
}

/// A test widget: one hit region under `key`.
fn w(ui: &mut Ui<'_>, key: &str, rect: Rect, sense: Sense) -> Response {
    let id = ui.id(key);
    ui.interact(id, rect, sense)
}

const A: Rect = Rect::new(10.0, 10.0, 100.0, 40.0);
const B: Rect = Rect::new(200.0, 10.0, 100.0, 40.0);

/// Runs a frame drawing `A` (and `B`) with `sense`; returns their responses.
fn ab(h: &mut UiHarness, sense: Sense) -> (Response, Response) {
    let mut out = None;
    h.frame(|ui| out = Some((w(ui, "a", A, sense), w(ui, "b", B, sense))));
    out.expect("frame ran")
}

#[test]
fn a_click_needs_press_and_release_on_the_same_id() {
    let mut h = harness();
    ab(&mut h, Sense::CLICK);
    h.input.pointer_to(20.0, 20.0).press(P);
    let (a, _) = ab(&mut h, Sense::CLICK);
    assert!(a.pressed && !a.clicked);
    h.input.release(P);
    let (a, b) = ab(&mut h, Sense::CLICK);
    assert!(a.clicked && !a.pressed && !b.clicked);
    // Press and release in one frame is a click too.
    h.input.click(P);
    let (a, _) = ab(&mut h, Sense::CLICK);
    assert!(a.clicked);
}

#[test]
fn a_release_outside_the_rect_is_no_click() {
    let mut h = harness();
    ab(&mut h, Sense::CLICK);
    h.input
        .pointer_to(20.0, 20.0)
        .press(P)
        .pointer_to(150.0, 20.0)
        .release(P);
    let (a, b) = ab(&mut h, Sense::CLICK);
    assert!(!a.clicked && !b.clicked);
}

#[test]
fn press_on_a_release_on_b_is_no_click() {
    let mut h = harness();
    ab(&mut h, Sense::CLICK);
    h.input.pointer_to(20.0, 20.0).press(P);
    ab(&mut h, Sense::CLICK);
    h.input.pointer_to(220.0, 20.0).release(P);
    let (a, b) = ab(&mut h, Sense::CLICK);
    assert!(!a.clicked && !b.clicked);
}

#[test]
fn double_click_is_strictly_under_400_ms() {
    for (gap, double) in [(399, true), (400, false)] {
        let mut h = harness();
        ab(&mut h, Sense::CLICK);
        h.at_ms(1000).input.pointer_to(20.0, 20.0).click(P);
        let (a, _) = ab(&mut h, Sense::CLICK);
        assert!(a.clicked && !a.double_clicked);
        h.at_ms(1000 + gap).input.click(P);
        let (a, _) = ab(&mut h, Sense::CLICK);
        assert!(a.clicked, "{gap} ms");
        assert_eq!(a.double_clicked, double, "{gap} ms");
    }
}

#[test]
fn a_drag_starts_at_4_px_with_the_full_travel() {
    let mut h = harness();
    ab(&mut h, Sense::DRAG);
    h.input.pointer_to(20.0, 20.0).press(P);
    ab(&mut h, Sense::DRAG);
    h.input.pointer_to(23.0, 20.0);
    let (a, _) = ab(&mut h, Sense::DRAG);
    assert!(a.pressed && !a.drag_started && !a.dragged);
    assert_eq!(a.drag_delta, (0.0, 0.0));
    h.input.pointer_to(24.5, 20.0);
    let (a, _) = ab(&mut h, Sense::DRAG);
    assert!(a.drag_started && a.dragged);
    assert_eq!(a.drag_delta, (4.5, 0.0));
    h.input.pointer_to(26.5, 21.0);
    let (a, _) = ab(&mut h, Sense::DRAG);
    assert!(!a.drag_started && a.dragged);
    assert_eq!(a.drag_delta, (2.0, 1.0));
    h.input.release(P);
    let (a, _) = ab(&mut h, Sense::DRAG);
    assert!(
        a.drag_stopped && !a.dragged && !a.clicked,
        "a drag is no click"
    );
}

#[test]
fn a_capture_outside_the_rect_keeps_pressed_and_blocks_other_hover() {
    let mut h = harness();
    ab(&mut h, Sense::CLICK);
    h.input
        .pointer_to(20.0, 20.0)
        .press(P)
        .pointer_to(220.0, 20.0);
    let (a, b) = ab(&mut h, Sense::CLICK);
    assert!(a.pressed && !a.hovered);
    assert!(
        !b.hovered && !b.pressed,
        "no other id is hovered while captured"
    );
    h.input.pointer_to(20.0, 20.0);
    let (a, _) = ab(&mut h, Sense::CLICK);
    assert!(a.pressed && a.hovered);
}

fn overlap(h: &mut UiHarness, first: LayerId, second: LayerId) -> (Response, Response) {
    let mut out = None;
    h.frame(|ui| {
        ui.painter().on(first);
        let x = w(ui, "x", A, Sense::CLICK);
        ui.painter().on(second);
        let y = w(ui, "y", A, Sense::CLICK);
        out = Some((x, y));
    });
    out.expect("frame ran")
}

#[test]
fn a_later_hit_in_the_same_layer_wins() {
    let mut h = harness();
    overlap(&mut h, LayerId::Page, LayerId::Page);
    h.input.pointer_to(20.0, 20.0).press(P);
    let (x, y) = overlap(&mut h, LayerId::Page, LayerId::Page);
    assert!(y.hovered && y.pressed);
    assert!(!x.hovered && !x.pressed);
}

#[test]
fn popup_over_page_wins_at_the_same_point() {
    let mut h = harness();
    overlap(&mut h, LayerId::Popup, LayerId::Page);
    h.input.pointer_to(20.0, 20.0);
    let (popup, page) = overlap(&mut h, LayerId::Popup, LayerId::Page);
    assert!(
        popup.hovered && !page.hovered,
        "the popup was drawn first and still wins"
    );
}

#[test]
fn all_five_layers_are_ordered_pairwise() {
    let layers = [
        LayerId::Base,
        LayerId::Page,
        LayerId::Popup,
        LayerId::Modal,
        LayerId::Toast,
    ];
    for (i, lo) in layers.iter().enumerate() {
        for hi in &layers[i + 1..] {
            // Higher first in call order, so only the layer can make it win.
            let mut h = harness();
            overlap(&mut h, *hi, *lo);
            h.input.pointer_to(20.0, 20.0).press(P);
            let (top, low) = overlap(&mut h, *hi, *lo);
            assert!(top.hovered && top.pressed, "{hi:?} over {lo:?}");
            assert!(!low.hovered && !low.pressed, "{hi:?} over {lo:?}");
        }
    }
}

#[test]
fn a_hover_only_hit_blocks_presses_below_it() {
    let mut h = harness();
    let frame = |h: &mut UiHarness| {
        let mut out = None;
        h.frame(|ui| {
            let below = w(ui, "below", A, Sense::CLICK);
            w(ui, "card", A, Sense::HOVER);
            out = Some(below);
        });
        out.expect("frame ran")
    };
    frame(&mut h);
    h.input.pointer_to(20.0, 20.0).click(P);
    let below = frame(&mut h);
    assert!(!below.hovered && !below.pressed && !below.clicked);
}

#[test]
fn an_old_api_widget_sees_the_hover_and_blocks_a_lower_hit() {
    let mut h = harness();
    let frame = |h: &mut UiHarness| {
        let mut out = None;
        h.frame(|ui| {
            let lower = w(ui, "lower", A, Sense::CLICK);
            let old = ui.id("old");
            widgets::button(ui.painter(), A, "Old", true, old);
            let hover = ui.painter().hover;
            out = Some((lower, old, hover));
        });
        out.expect("frame ran")
    };
    frame(&mut h);
    h.input.pointer_to(20.0, 20.0).click(P);
    let (lower, old, hover) = frame(&mut h);
    assert_eq!(hover, Some(old));
    assert!(!lower.hovered && !lower.clicked);
}

#[test]
fn an_unused_transient_id_is_gone_after_one_frame() {
    let mut h = harness();
    let id = WidgetId::ROOT.with("t");
    h.frame(|ui| *ui.transient::<u32>(id) = 7);
    h.frame(|ui| assert_eq!(*ui.transient::<u32>(id), 7));
    h.frame(|_| {});
    h.frame(|ui| assert_eq!(*ui.transient::<u32>(id), 0));
}

#[derive(Default)]
struct Undo(Vec<String>);

#[test]
fn a_hidden_tab_under_keep_alive_keeps_its_state_until_the_scope_ends() {
    let mut h = harness();
    h.frame(|ui| {
        ui.keep_alive("tab", |ui| {
            let id = ui.id("list");
            *ui.persistent::<f32>(id) = 120.5;
            ui.persistent::<Undo>(id).0.push("typed".to_owned());
        });
    });
    for _ in 0..600 {
        h.frame(|_| {});
    }
    h.frame(|ui| {
        ui.keep_alive("tab", |ui| {
            let id = ui.id("list");
            assert_eq!(*ui.persistent::<f32>(id), 120.5);
            assert_eq!(ui.persistent::<Undo>(id).0, ["typed"]);
        });
    });
    h.frame(|ui| ui.end_keep_alive("tab"));
    h.frame(|ui| {
        ui.keep_alive("tab", |ui| {
            let id = ui.id("list");
            assert_eq!(*ui.persistent::<f32>(id), 0.0);
            assert!(ui.persistent::<Undo>(id).0.is_empty());
        });
    });
}

#[test]
fn persistent_state_outside_any_scope_lives_one_frame() {
    let mut h = harness();
    let id = WidgetId::ROOT.with("p");
    h.frame(|ui| *ui.persistent::<u8>(id) = 3);
    h.frame(|_| {});
    h.frame(|ui| assert_eq!(*ui.persistent::<u8>(id), 0));
}

#[test]
fn mark_alive_opens_a_scope_without_drawing() {
    let mut h = harness();
    h.frame(|ui| {
        ui.mark_alive("tab");
        ui.keep_alive("tab", |ui| {
            let id = ui.id("x");
            *ui.persistent::<u8>(id) = 9;
        });
    });
    h.frame(|ui| ui.mark_alive("tab"));
    h.frame(|ui| {
        ui.keep_alive("tab", |ui| {
            let id = ui.id("x");
            assert_eq!(*ui.persistent::<u8>(id), 9);
        });
    });
}

#[test]
fn a_selection_survives_frames_where_the_selected_row_is_not_drawn() {
    let mut h = harness();
    let rows = |h: &mut UiHarness, first: u64| {
        let mut selected = BTreeSet::new();
        h.frame(|ui| {
            let list = ui.id("list");
            let mut clicked = None;
            ui.scope("list", |ui| {
                for (i, key) in (first..first + 5).enumerate() {
                    let r = Rect::new(0.0, i as f32 * 20.0, 100.0, 20.0);
                    let id = ui.id(key);
                    if ui.interact(id, r, Sense::CLICK).clicked {
                        clicked = Some(key);
                    }
                }
            });
            let sel = ui.persistent::<BTreeSet<u64>>(list);
            if let Some(k) = clicked {
                sel.insert(k);
            }
            selected = sel.clone();
        });
        selected
    };
    rows(&mut h, 0);
    h.input.pointer_to(10.0, 25.0).click(P); // row 1
    assert_eq!(rows(&mut h, 0), BTreeSet::from([1]));
    for first in 10..40 {
        assert_eq!(
            rows(&mut h, first),
            BTreeSet::from([1]),
            "scrolled to {first}"
        );
    }
}

#[test]
fn repaint_follows_the_animation() {
    let mut h = harness();
    let id = WidgetId::ROOT.with("fade");
    let motion = Motion::of(Kind::Appear); // base: 250 ms
    let (_, out) = h
        .at_ms(0)
        .frame(|ui| assert_eq!(ui.animate_f32(id, 0.0, &motion), 0.0));
    assert_eq!(
        out.repaint,
        Repaint::Idle,
        "first call: target, not in flight"
    );
    let (_, out) = h
        .at_ms(100)
        .frame(|ui| assert_eq!(ui.animate_f32(id, 1.0, &motion), 0.0));
    assert_eq!(out.repaint, Repaint::Now);
    let (_, out) = h.at_ms(200).frame(|ui| {
        let v = ui.animate_f32(id, 1.0, &motion);
        assert!(v > 0.0 && v < 1.0, "{v}");
    });
    assert_eq!(out.repaint, Repaint::Now);
    let (_, out) = h
        .at_ms(350)
        .frame(|ui| assert_eq!(ui.animate_f32(id, 1.0, &motion), 1.0));
    assert_eq!(
        out.repaint,
        Repaint::Idle,
        "the first frame at the end is settled"
    );
    let (_, out) = h.at_ms(400).frame(|_| {});
    assert_eq!(out.repaint, Repaint::Idle);
}

#[test]
fn repaint_at_the_earliest_future_request() {
    let mut h = harness();
    let (_, out) = h.at_ms(1000).frame(|ui| {
        ui.request_repaint_at(Duration::from_secs(9));
        ui.request_repaint_at(Duration::from_secs(5));
    });
    assert_eq!(out.repaint, Repaint::At(Duration::from_secs(5)));
    let (_, out) = h
        .at_ms(5000)
        .frame(|ui| ui.request_repaint_at(Duration::from_secs(5)));
    assert_eq!(
        out.repaint,
        Repaint::Idle,
        "a request not after now is due already"
    );
    // A due request never hides a later one (review F2).
    let (_, out) = h.at_ms(1000).frame(|ui| {
        let now = ui.time();
        ui.request_repaint_at(now + Duration::from_secs(5));
        ui.request_repaint_at(now);
        ui.request_repaint_at(now - Duration::from_millis(10));
    });
    assert_eq!(out.repaint, Repaint::At(Duration::from_secs(6)));
}

#[test]
fn a_colour_animates_and_settles_exactly() {
    let mut h = harness();
    let id = WidgetId::ROOT.with("c");
    let m = Motion::of(Kind::Hover);
    h.at_ms(0)
        .frame(|ui| assert_eq!(ui.animate_color(id, [0, 0, 0, 255], &m), [0, 0, 0, 255]));
    h.at_ms(10).frame(|ui| {
        ui.animate_color(id, [200, 100, 50, 255], &m);
    });
    let (_, out) = h.at_ms(60).frame(|ui| {
        let c = ui.animate_color(id, [200, 100, 50, 255], &m);
        assert!(c[0] > 0 && c[0] < 200, "{c:?}");
    });
    assert_eq!(out.repaint, Repaint::Now);
    h.at_ms(130).frame(|ui| {
        assert_eq!(
            ui.animate_color(id, [200, 100, 50, 255], &m),
            [200, 100, 50, 255]
        );
    });
}

#[test]
fn off_tier_snaps_and_a_tier_change_snaps_in_flight_values() {
    let mut h = harness();
    let id = WidgetId::ROOT.with("v");
    let m = Motion::of(Kind::Appear);
    h.tier = MotionTier::Off;
    h.at_ms(0).frame(|ui| {
        ui.animate_f32(id, 0.0, &m);
    });
    let (_, out) = h
        .at_ms(10)
        .frame(|ui| assert_eq!(ui.animate_f32(id, 1.0, &m), 1.0));
    assert_eq!(out.repaint, Repaint::Idle);
    h.tier = MotionTier::Full;
    let (_, out) = h
        .at_ms(20)
        .frame(|ui| assert_eq!(ui.animate_f32(id, 0.0, &m), 1.0));
    assert_eq!(out.repaint, Repaint::Now, "in flight under Full");
    h.tier = MotionTier::Reduced;
    let (_, out) = h
        .at_ms(30)
        .frame(|ui| assert_eq!(ui.animate_f32(id, 0.0, &m), 0.0));
    assert_eq!(out.repaint, Repaint::Idle, "the tier change snapped it");
}

/// A 30-frame script: moves, presses, drags, scrolls, an animation and a keep-alive.
fn script(
    h: &mut UiHarness,
) -> Vec<(lune_ui_core::scene::Scene, lune_ui_core::input::FrameOutput)> {
    let mut frames = Vec::new();
    for i in 0..30u64 {
        h.at_ms(i * 16);
        match i % 6 {
            1 => {
                h.input.pointer_to(20.0 + i as f32, 20.0);
            }
            2 => {
                h.input.press(P);
            }
            3 => {
                h.input.pointer_to(40.0 + i as f32, 25.0);
            }
            4 => {
                h.input.release(P).scroll_lines(0.0, 1.0);
            }
            _ => {}
        }
        frames.push(h.frame(|ui| {
            let a = w(ui, "a", A, Sense::DRAG);
            let t = ui.animate_f32(
                a.id,
                if a.hovered { 1.0 } else { 0.0 },
                &Motion::of(Kind::Hover),
            );
            let c = ui.theme().accent;
            ui.painter()
                .fill(A, 4.0, lune_ui_core::color::with_alpha(c, t));
            let b = ui.id("b");
            widgets::button(ui.painter(), B, "Go", a.pressed, b);
            ui.keep_alive("k", |ui| {
                let id = ui.id("s");
                *ui.persistent::<f32>(id) += a.drag_delta.0;
            });
        }));
    }
    frames
}

#[test]
fn a_script_run_in_two_fresh_stores_gives_identical_frames() {
    let one = script(&mut harness());
    let two = script(&mut harness());
    assert_eq!(one.len(), 30);
    assert!(
        one.iter().any(|(_, o)| o.repaint == Repaint::Now),
        "the script animates"
    );
    for (i, (a, b)) in one.iter().zip(&two).enumerate() {
        assert_eq!(a, b, "frame {i}");
    }
}

#[test]
fn the_same_keys_in_two_stores_keep_independent_state() {
    let (mut one, mut two) = (harness(), harness());
    let id = WidgetId::ROOT.with("n");
    one.frame(|ui| *ui.transient::<u32>(id) = 1);
    two.frame(|ui| *ui.transient::<u32>(id) = 2);
    one.frame(|ui| assert_eq!(*ui.transient::<u32>(id), 1));
    two.frame(|ui| assert_eq!(*ui.transient::<u32>(id), 2));
    ab(&mut one, Sense::CLICK);
    ab(&mut two, Sense::CLICK);
    one.input.pointer_to(20.0, 20.0).press(P);
    let (a1, _) = ab(&mut one, Sense::CLICK);
    let (a2, _) = ab(&mut two, Sense::CLICK);
    assert!(a1.pressed && !a2.pressed && !a2.hovered);
}

#[test]
fn disabled_responses_are_inert_and_still_block() {
    let mut h = harness();
    let frame = |h: &mut UiHarness| {
        let mut out = None;
        let (scene, _) = h.frame(|ui| {
            let lower = w(ui, "lower", A, Sense::CLICK);
            let d = ui.with_enabled(false, |ui| {
                ui.painter().fill(A, 0.0, [255, 255, 255, 255]);
                w(ui, "d", A, Sense::DRAG)
            });
            out = Some((lower, d));
        });
        let (lower, d) = out.expect("frame ran");
        (lower, d, scene)
    };
    frame(&mut h);
    h.input
        .pointer_to(20.0, 20.0)
        .press(P)
        .pointer_to(30.0, 20.0)
        .release(P);
    let (lower, d, scene) = frame(&mut h);
    assert!(!d.enabled);
    assert!(!d.hovered && !d.pressed && !d.clicked && !d.drag_started && !d.drag_stopped);
    assert!(!lower.hovered && !lower.clicked, "the disabled hit blocks");
    let fill = scene.layers[0].geometry.under[0];
    assert_eq!(fill.color, [0.4, 0.4, 0.4, 0.4], "drawn at opacity 0.4");
    assert_eq!(
        fill.rect,
        [10.0, 10.0, 100.0, 40.0],
        "alpha only: no movement"
    );
}

#[test]
fn clipboard_keeps_the_last_write_and_one_read() {
    let mut h = harness();
    let (_, out) = h.frame(|ui| {
        ui.clipboard_write("one".to_owned());
        ui.clipboard_read();
        ui.clipboard_write("two".to_owned());
        ui.clipboard_read();
    });
    assert_eq!(
        out.clipboard,
        [
            ClipboardRequest::Write("two".to_owned()),
            ClipboardRequest::Read
        ]
    );
    let (_, out) = h.frame(|_| {});
    assert!(out.clipboard.is_empty());
}

#[test]
fn scroll_goes_to_the_topmost_scroll_area_not_below_the_topmost_hit() {
    let mut h = harness();
    // A list (scroll) with a row button on top, same layer: the list scrolls.
    let frame = |h: &mut UiHarness, popup: bool| {
        let mut out = None;
        h.frame(|ui| {
            ui.painter().on(LayerId::Page);
            let list = w(ui, "list", Rect::new(0.0, 0.0, 300.0, 200.0), Sense::SCROLL);
            let row = w(ui, "row", A, Sense::CLICK);
            if popup {
                ui.painter().on(LayerId::Popup);
                w(ui, "menu", A, Sense::HOVER);
            }
            out = Some((list, row));
        });
        out.expect("frame ran")
    };
    frame(&mut h, false);
    h.input
        .pointer_to(20.0, 20.0)
        .scroll_lines(0.0, -2.0)
        .scroll_pixels(1.0, 3.0);
    let (list, row) = frame(&mut h, true);
    assert_eq!(list.scroll_lines, (0.0, -2.0));
    assert_eq!(list.scroll_pixels, (1.0, 3.0));
    assert_eq!(row.scroll_lines, (0.0, 0.0));
    // Now a popup covers the row: the list is on a lower layer and gets nothing.
    h.input.scroll_lines(0.0, 1.0);
    let (list, _) = frame(&mut h, true);
    assert_eq!(list.scroll_lines, (0.0, 0.0), "blocked by the popup");
}

#[test]
fn last_rect_is_the_previous_frames_registered_rect() {
    let mut h = harness();
    let id = WidgetId::ROOT.with("a");
    assert_eq!(h.store.last_rect(id), None);
    h.frame(|ui| {
        ui.painter()
            .set_clip(Some(Rect::new(0.0, 0.0, 50.0, 300.0)));
        w(ui, "a", A, Sense::CLICK);
    });
    assert_eq!(
        h.store.last_rect(id),
        Some(Rect::new(10.0, 10.0, 40.0, 40.0))
    );
}

#[test]
fn ids_nest_under_scopes() {
    let mut h = harness();
    h.frame(|ui| {
        let top = ui.id("x");
        assert_eq!(top, WidgetId::ROOT.with("x"));
        ui.scope("panel", |ui| {
            assert_eq!(ui.id("x"), WidgetId::ROOT.with("panel").with("x"));
        });
        assert_eq!(ui.id("x"), top);
    });
}

#[test]
fn a_focus_request_repeated_every_frame_settles_to_idle() {
    // Review F10: "keep the field focused" each frame must not keep the window redrawing once
    // the field is focused and shows no ring.
    let mut h = harness();
    h.tier = MotionTier::Off;
    let mut seq = Vec::new();
    for i in 0..6u64 {
        let mut focused = false;
        let (_, out) = h.at_ms(i * 10).frame(|ui| {
            let id = ui.id("field");
            focused = ui.interact(id, A, Sense::CLICK_FOCUS).focused;
            ui.request_focus(id);
        });
        seq.push((out.repaint, focused));
    }
    let now = seq.iter().filter(|(r, _)| *r == Repaint::Now).count();
    assert!(now <= 1, "at most one Now: {seq:?}");
    assert!(seq[2..].iter().all(|(r, f)| *r == Repaint::Idle && *f));
}

#[test]
fn a_region_that_moves_with_its_own_hover_does_not_loop_the_catch_up() {
    // Review F11, D81: region A sits under the still pointer; while A is hovered, region B is
    // drawn over it, so the hover never settles. The move frame asks for one catch-up frame, and
    // an event-less frame never asks for another one right after: no two Nows in a row.
    let mut h = harness();
    h.tier = MotionTier::Off;
    let run = |h: &mut UiHarness, ms: u64| {
        let mut a_hovered = false;
        let (_, out) = h.at_ms(ms).frame(|ui| {
            a_hovered = w(ui, "a", A, Sense::CLICK).hovered;
            if a_hovered {
                w(ui, "b", A, Sense::CLICK);
            }
        });
        (a_hovered, out.repaint)
    };
    assert_eq!(run(&mut h, 0), (false, Repaint::Idle));
    h.input.pointer_to(20.0, 20.0);
    let (hovered, repaint) = run(&mut h, 10);
    assert!(hovered);
    assert_eq!(repaint, Repaint::Now, "the move frame asks for a catch-up");
    let mut seq = vec![repaint];
    for i in 2..10u64 {
        seq.push(run(&mut h, i * 10).1);
    }
    assert_eq!(seq[1], Repaint::Idle, "the event-less frame after the move");
    assert!(
        seq.windows(2).all(|w| w != [Repaint::Now, Repaint::Now]),
        "{seq:?}"
    );
}

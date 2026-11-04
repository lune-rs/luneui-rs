//! The overlay stack (C4): modal capture, Escape and outside-press dismissal on the top overlay,
//! phases, exit retention per tier through the exit painter, reopening a closing overlay, and
//! replaying recorded content.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{Easing, Kind, Motion, MotionTier, Reduce, Token};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::{
    Dismissal, OverlayFrame, OverlayOptions, OverlayPhase, Response, Sense, Ui,
};

const P: PointerButton = PointerButton::Primary;
const PAGE_BTN: Rect = Rect::new(10.0, 10.0, 80.0, 30.0);
const CARD: Rect = Rect::new(150.0, 100.0, 200.0, 120.0);
const MENU: Rect = Rect::new(150.0, 10.0, 100.0, 60.0);

/// The dialog's close motion: dismiss, fast, quad-out, fade.
const DIALOG_CLOSE: Motion = Motion {
    kind: Kind::Dismiss,
    token: Token::Fast,
    easing: Easing::QuadOut,
    reduced: Reduce::Fade,
    reduced_token: None,
};

fn modal() -> OverlayOptions {
    OverlayOptions {
        layer: LayerId::Modal,
        modal: true,
        dismiss_on_escape: true,
        dismiss_on_outside_press: false,
        dismiss_on_focus_loss: false,
        anchor: None,
        initial_focus: None,
        open: Motion::of(Kind::Appear),
        close: DIALOG_CLOSE,
    }
}

fn popup() -> OverlayOptions {
    OverlayOptions {
        layer: LayerId::Popup,
        modal: false,
        dismiss_on_outside_press: true,
        ..modal()
    }
}

fn harness() -> UiHarness {
    UiHarness::new((400.0, 300.0))
}

fn page_button(ui: &mut Ui<'_>) -> Response {
    ui.painter().on(LayerId::Page);
    let id = ui.id("page-button");
    ui.interact(id, PAGE_BTN, Sense::CLICK_FOCUS)
}

/// A dialog card with one button; `fill` is drawn on the card.
fn dialog(ui: &mut Ui<'_>, key: &str, opts: &OverlayOptions) -> (OverlayFrame, Response) {
    ui.overlay(
        key,
        opts,
        |_, _, _| {},
        |ui, _| {
            let card = ui.id((key, "card"));
            ui.interact(card, CARD, Sense::HOVER);
            let ok = ui.id((key, "ok"));
            ui.interact(ok, Rect::new(160.0, 180.0, 60.0, 24.0), Sense::CLICK_FOCUS)
        },
    )
}

#[test]
fn a_click_on_a_page_button_under_an_open_modal_does_nothing() {
    let mut h = harness();
    let run = |h: &mut UiHarness| {
        let mut out = None;
        h.frame(|ui| {
            let b = page_button(ui);
            dialog(ui, "dialog", &modal());
            out = Some(b);
        });
        out.expect("frame ran")
    };
    run(&mut h);
    h.input.pointer_to(20.0, 20.0);
    let b = run(&mut h);
    assert!(!b.hovered, "no hover under the modal");
    h.input.press(P);
    let b = run(&mut h);
    assert!(!b.pressed && !b.focused);
    h.input.release(P);
    let b = run(&mut h);
    assert!(!b.clicked && !b.focused);
}

#[test]
fn a_toast_above_a_modal_stays_live() {
    let mut h = harness();
    let run = |h: &mut UiHarness| {
        let mut out = None;
        h.frame(|ui| {
            dialog(ui, "dialog", &modal());
            ui.painter().on(LayerId::Toast);
            let id = ui.id("toast");
            out = Some(ui.interact(id, PAGE_BTN, Sense::CLICK));
        });
        out.expect("frame ran")
    };
    run(&mut h);
    h.input.pointer_to(20.0, 20.0).click(P);
    let t = run(&mut h);
    assert!(t.hovered && t.clicked);
}

#[derive(Default)]
struct Two {
    lower: bool,
    upper: bool,
    lower_dismissed: Option<Dismissal>,
    upper_dismissed: Option<Dismissal>,
}

fn two_overlays(h: &mut UiHarness, s: &mut Two) {
    h.frame(|ui| {
        if s.lower {
            let (f, _) = dialog(ui, "lower", &modal());
            s.lower_dismissed = f.dismissed;
        }
        if s.upper {
            let (f, _) = dialog(ui, "upper", &modal());
            s.upper_dismissed = f.dismissed;
        }
    });
}

#[test]
fn escape_closes_only_the_top_overlay() {
    let mut h = harness();
    let mut s = Two {
        lower: true,
        upper: true,
        ..Two::default()
    };
    two_overlays(&mut h, &mut s);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    two_overlays(&mut h, &mut s);
    assert_eq!(s.upper_dismissed, Some(Dismissal::Escape));
    assert_eq!(s.lower_dismissed, None);
    s.upper = false; // the app closes it
    two_overlays(&mut h, &mut s);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    two_overlays(&mut h, &mut s);
    assert_eq!(s.lower_dismissed, Some(Dismissal::Escape));
}

#[test]
fn escape_is_consumed_by_the_overlay_and_never_reaches_the_focused_widget() {
    let mut h = harness();
    let mut got = false;
    let run = |h: &mut UiHarness, got: &mut bool| {
        h.frame(|ui| {
            let (_, ok) = dialog(ui, "dialog", &modal());
            *got |= ui.consume_key(ok.id, NamedKey::Escape);
        });
    };
    run(&mut h, &mut got);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    run(&mut h, &mut got);
    assert!(!got);
}

#[test]
fn an_outside_press_dismisses_a_light_dismiss_overlay_and_presses_nothing() {
    let mut h = harness();
    let run = |h: &mut UiHarness| {
        let mut out = None;
        h.frame(|ui| {
            let b = page_button(ui);
            let (f, _) = ui.overlay(
                "menu",
                &popup(),
                |_, _, _| {},
                |ui, _| {
                    let id = ui.id("menu-card");
                    ui.interact(id, MENU, Sense::HOVER);
                },
            );
            out = Some((b, f));
        });
        out.expect("frame ran")
    };
    run(&mut h);
    h.input.pointer_to(20.0, 20.0);
    let (b, _) = run(&mut h);
    assert!(b.hovered, "a non-modal overlay leaves hover alone");
    h.input.press(P);
    let (b, f) = run(&mut h);
    assert_eq!(f.dismissed, Some(Dismissal::OutsidePress));
    assert!(!b.pressed && !b.focused);
    h.input.release(P);
    let (b, f) = run(&mut h);
    assert!(!b.clicked && f.dismissed.is_none());
    // A press inside the overlay does not dismiss it.
    h.input.pointer_to(160.0, 20.0).click(P);
    let (_, f) = run(&mut h);
    assert_eq!(f.dismissed, None);
}

#[test]
fn opening_progress_is_linear_and_reports_now_until_open() {
    let mut h = harness();
    let mut phases = Vec::new();
    let mut outs = Vec::new();
    for ms in [0u64, 100, 249, 250, 300] {
        let (_, out) = h
            .at_ms(1000 + ms)
            .frame(|ui| phases.push(dialog(ui, "d", &modal()).0.phase));
        outs.push(out.repaint);
    }
    assert_eq!(phases[0], OverlayPhase::Opening(0.0));
    assert_eq!(
        phases[1],
        OverlayPhase::Opening(Duration::from_millis(100).as_secs_f32() / 0.25)
    );
    assert!(matches!(phases[2], OverlayPhase::Opening(p) if p < 1.0));
    assert_eq!(phases[3], OverlayPhase::Open);
    assert_eq!(phases[4], OverlayPhase::Open);
    assert_eq!(
        outs,
        [
            Repaint::Now,
            Repaint::Now,
            Repaint::Now,
            Repaint::Idle,
            Repaint::Idle
        ]
    );
}

#[test]
fn off_tier_opens_at_once() {
    let mut h = harness();
    h.tier = MotionTier::Off;
    let mut phase = None;
    let (_, out) = h.frame(|ui| phase = Some(dialog(ui, "d", &modal()).0.phase));
    assert_eq!(phase, Some(OverlayPhase::Open));
    // Focus moved into it after the page drew: one more frame (D81), then still.
    assert_eq!(out.repaint, Repaint::Now);
    let (_, out) = h
        .at_ms(16)
        .frame(|ui| phase = Some(dialog(ui, "d", &modal()).0.phase));
    assert_eq!(phase, Some(OverlayPhase::Open));
    assert_eq!(out.repaint, Repaint::Idle);
}

/// Opens a modal whose exit painter fills `CARD` at `1 − p` alpha and logs `p`, lets it settle
/// open, stops calling it at `t0`, and steps 10 ms frames until idle. Returns the logged exit
/// progress, the repaint of each closing frame and the time of the first idle frame.
fn close_run(tier: MotionTier) -> (Vec<f32>, Vec<Repaint>, u64, Scene) {
    let mut h = harness();
    h.tier = tier;
    let log: Rc<RefCell<Vec<(f32, bool)>>> = Rc::default();
    for ms in [0, 300] {
        let log = log.clone();
        h.at_ms(ms).frame(|ui| {
            ui.overlay(
                "d",
                &modal(),
                move |p, progress, transforms| {
                    log.borrow_mut().push((progress, transforms));
                    let a = ((1.0 - progress) * 255.0) as u8;
                    p.fill(CARD, 8.0, [255, 255, 255, a]);
                },
                |ui, _| ui.painter().fill(CARD, 8.0, [255, 255, 255, 255]),
            );
        });
    }
    let t0 = 1000;
    let mut repaints = Vec::new();
    let mut t = t0;
    loop {
        let (scene, out) = h.at_ms(t).frame(|_| {});
        if out.repaint == Repaint::Idle {
            let ps = log.borrow().iter().map(|(p, _)| *p).collect();
            return (ps, repaints, t - t0, scene);
        }
        repaints.push(out.repaint);
        let modal_layer = &scene.layers[LayerId::Modal as usize];
        assert_eq!(
            modal_layer.geometry.under.len(),
            1,
            "the exit frame draws at {t}"
        );
        t += 10;
        assert!(t < t0 + 1000, "never settled");
    }
}

#[test]
fn an_overlay_the_app_stops_calling_keeps_drawing_until_its_close_ends() {
    for (tier, ms) in [
        (MotionTier::Full, 180),
        (MotionTier::Reduced, 120),
        (MotionTier::Off, 0),
    ] {
        let (ps, repaints, idle_after, scene) = close_run(tier);
        assert_eq!(idle_after, ms, "{tier:?}");
        assert!(repaints.iter().all(|r| *r == Repaint::Now), "{tier:?}");
        assert_eq!(repaints.len() as u64, ms / 10, "{tier:?}");
        assert_eq!(ps.len() as u64, ms / 10, "{tier:?}");
        if let Some(first) = ps.first() {
            assert_eq!(
                *first, 0.0,
                "the first exit frame is at p 0 (full visibility)"
            );
        }
        assert!(ps.windows(2).all(|w| w[1] > w[0]), "{tier:?}: rising p");
        assert!(ps.iter().all(|p| *p < 1.0));
        let modal_layer = &scene.layers[LayerId::Modal as usize];
        assert!(
            modal_layer.geometry.under.is_empty(),
            "{tier:?}: nothing on its layer"
        );
    }
}

#[test]
fn the_exit_painter_gets_transforms_false_under_the_reduced_fade() {
    for (tier, want) in [(MotionTier::Full, true), (MotionTier::Reduced, false)] {
        let mut h = harness();
        h.tier = tier;
        let log: Rc<RefCell<Vec<bool>>> = Rc::default();
        let l = log.clone();
        let mut opening = None;
        h.at_ms(0).frame(|ui| {
            let (f, _) = ui.overlay(
                "d",
                &modal(),
                move |_, _, t| l.borrow_mut().push(t),
                |_, _| {},
            );
            opening = Some(f.transforms);
        });
        assert_eq!(opening, Some(want), "{tier:?} open");
        h.at_ms(500).frame(|_| {});
        assert_eq!(*log.borrow(), [want], "{tier:?} close");
    }
}

#[test]
fn a_reopened_closing_overlay_continues_from_the_matching_visibility() {
    let mut h = harness();
    let open = Easing::QuadOut; // Appear's easing
    let log: Rc<RefCell<Vec<f32>>> = Rc::default();
    let call = |h: &mut UiHarness, ms: u64, log: &Rc<RefCell<Vec<f32>>>| {
        let l = log.clone();
        let mut phase = None;
        h.at_ms(ms).frame(|ui| {
            let (f, _) = ui.overlay(
                "d",
                &modal(),
                move |_, p, _| l.borrow_mut().push(p),
                |_, _| {},
            );
            phase = Some(f.phase);
        });
        phase.expect("called")
    };
    call(&mut h, 0, &log);
    call(&mut h, 300, &log);
    h.at_ms(1000).frame(|_| {});
    h.at_ms(1080).frame(|_| {});
    let last_p = *log.borrow().last().expect("exit drew");
    // Visibility the close reaches at the reopen instant (1090 ms): 1 − E(p).
    let at_reopen = 1.0 - DIALOG_CLOSE.easing.eval(0.09 / 0.18);
    let closing_before = 1.0 - DIALOG_CLOSE.easing.eval(last_p);
    assert!(closing_before > at_reopen);
    let OverlayPhase::Opening(p0) = call(&mut h, 1090, &log) else {
        panic!("reopens opening");
    };
    let after = open.eval(p0);
    assert!((after - at_reopen).abs() <= 0.01, "{after} vs {at_reopen}");
    // And it keeps opening from there.
    let OverlayPhase::Opening(p1) = call(&mut h, 1100, &log) else {
        panic!("still opening");
    };
    assert!(p1 > p0 && (open.eval(p1) - after).abs() < 0.2);
}

#[test]
fn recorded_content_replays_in_the_exit_painter() {
    let mut h = harness();
    let mut open_scene = None;
    for ms in [0u64, 300] {
        let (scene, _) = h.at_ms(ms).frame(|ui| {
            let ((), ops) = ui.record(|ui| {
                ui.painter().on(LayerId::Modal);
                ui.painter().fill(CARD, 8.0, [10, 20, 30, 200]);
                ui.painter().ring(CARD, 8.0, 1.0, [255, 255, 255, 40], true);
            });
            // The overlay hands the recording to its exit painter.
            let opts = modal();
            ui.overlay("d", &opts, move |p, _, _| p.replay(&ops), |_, _| {});
        });
        open_scene = Some(scene);
    }
    let (closing, _) = h.at_ms(1000).frame(|_| {});
    let open_scene = open_scene.expect("ran");
    assert_eq!(
        closing.layers[LayerId::Modal as usize],
        open_scene.layers[LayerId::Modal as usize],
        "the first exit frame replays the content as drawn"
    );
}

#[test]
fn exit_painter_hits_are_suppressed_and_closing_overlays_are_inert() {
    let mut h = harness();
    let run = |h: &mut UiHarness, ms: u64, open: bool| {
        let mut out = None;
        h.at_ms(ms).frame(|ui| {
            let b = page_button(ui);
            if open {
                ui.overlay(
                    "d",
                    &modal(),
                    |p, _, _| {
                        p.hit(
                            Rect::new(0.0, 0.0, 400.0, 300.0),
                            lune_ui_core::ui::WidgetId::ROOT,
                        )
                    },
                    |_, _| {},
                );
            }
            out = Some(b);
        });
        out.expect("frame ran")
    };
    run(&mut h, 0, true);
    run(&mut h, 300, true);
    run(&mut h, 1000, false); // closing: exit draws a window-sized hit
    h.input.pointer_to(20.0, 20.0).click(P);
    let b = run(&mut h, 1010, false);
    assert!(b.hovered && b.clicked, "a closing modal captures nothing");
}

#[test]
fn a_modal_opening_under_a_still_pointer_asks_for_the_frame_with_the_right_hover() {
    // Review F3 (a), D81: Off tier, nothing animates; the open frame resolved hover against the
    // page's regions, so it asks for one more frame, which hovers the dialog's OK.
    let mut h = harness();
    h.tier = MotionTier::Off;
    let run = |h: &mut UiHarness, open: bool| {
        let (mut page, mut ok) = (false, false);
        let (_, out) = h.frame(|ui| {
            let id = ui.id("under");
            page = ui
                .interact(id, Rect::new(150.0, 170.0, 100.0, 50.0), Sense::CLICK)
                .hovered;
            if open {
                ok = dialog(ui, "d", &modal()).1.hovered;
            }
        });
        (page, ok, out.repaint)
    };
    run(&mut h, false);
    h.input.pointer_to(170.0, 190.0);
    assert_eq!(run(h.at_ms(10), false), (true, false, Repaint::Idle));
    let (page, ok, repaint) = run(h.at_ms(20), true);
    assert!(page && !ok, "the open frame's hover is the page's");
    assert_eq!(repaint, Repaint::Now);
    assert_eq!(run(h.at_ms(30), true), (false, true, Repaint::Idle));
}

#[test]
fn a_nested_record_keeps_the_outer_recording_whole() {
    // Review F7: the inner calls land in both recordings.
    let mut h = harness();
    h.frame(|ui| {
        let ((_, inner), outer) = ui.record(|ui| {
            ui.painter()
                .fill(Rect::new(0.0, 0.0, 10.0, 10.0), 0.0, [255, 0, 0, 255]);
            let r = ui.record(|ui| {
                ui.painter()
                    .fill(Rect::new(0.0, 0.0, 5.0, 5.0), 0.0, [0, 255, 0, 255]);
            });
            ui.painter()
                .fill(Rect::new(0.0, 0.0, 2.0, 2.0), 0.0, [0, 0, 255, 255]);
            r
        });
        assert_eq!(inner.len(), 1);
        assert_eq!(outer.len(), 3);
    });
}

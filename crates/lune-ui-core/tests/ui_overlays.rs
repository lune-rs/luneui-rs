//! The overlay stack (C4): modal capture, Escape and outside-press dismissal on the top overlay,
//! phases, exit retention per tier through the exit painter, reopening a closing overlay,
//! replaying recorded content, and overlays above a modal: the `Raised` layer, chosen at open.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use lune_ui_core::access::{Node, Role};
use lune_ui_core::input::{FrameOutput, Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{Easing, Kind, Motion, MotionTier, Reduce, Token};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::{
    Dismissal, OverlayFrame, OverlayOptions, OverlayPhase, Response, Sense, Ui, WidgetId,
};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::dialog::{Cancel, Dialog, DialogGlass};
use lune_ui_core::widgets::menu::MenuRow;
use lune_ui_core::widgets::popover::PopoverOptions;

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

// Overlays above a modal: real widgets inside a real dialog's body.

const WIN: (f32, f32) = (800.0, 600.0);
const GLASS: DialogGlass = DialogGlass {
    panel: [0.1, 0.1, 0.12, 1.0],
    fill: [0.2, 0.2, 0.2, 0.62],
    noise: false,
};
const DIALOG: Dialog<'static> = Dialog {
    title: "Title",
    message: "Message",
    extra_h: 220.0,
    confirm: "Yes",
    cancel: Cancel::Default,
};
const PAGE: Rect = Rect::new(20.0, 20.0, 100.0, 30.0);
/// A point on the scrim, off the card and off the page button.
const ON_SCRIM: (f32, f32) = (40.0, 400.0);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Child {
    Dropdown,
    Popover,
    Menu,
}

const CHILDREN: [Child; 3] = [Child::Dropdown, Child::Popover, Child::Menu];

/// A page button, a dialog, and in its body an anchor, a button under where the child opens, one
/// off to the side, and the child with an item named "two".
#[derive(Debug)]
struct Bed {
    kind: Child,
    dialog: bool,
    child: bool,
    page_clicks: u32,
    under_clicks: u32,
    side_clicks: u32,
    picked: u32,
    cancelled: u32,
    anchor: Option<WidgetId>,
    side: Rect,
    under: Rect,
}

impl Bed {
    fn of(kind: Child) -> Self {
        Self {
            kind,
            dialog: false,
            child: false,
            page_clicks: 0,
            under_clicks: 0,
            side_clicks: 0,
            picked: 0,
            cancelled: 0,
            anchor: None,
            side: Rect::default(),
            under: Rect::default(),
        }
    }

    fn body(&mut self, ui: &mut Ui<'_>, x: f32, y: f32) {
        let open = ui.button(
            "anchor",
            Rect::new(x, y, 120.0, 24.0),
            "open",
            ButtonKind::Secondary,
        );
        self.child |= open.clicked;
        self.anchor = Some(open.id);
        self.under = Rect::new(x, y + 30.0, 300.0, 120.0);
        self.side = Rect::new(x, y + 170.0, 80.0, 24.0);
        let under = ui.button("under", self.under, "under", ButtonKind::Secondary);
        self.under_clicks += u32::from(under.clicked);
        let side = ui.button("side", self.side, "side", ButtonKind::Secondary);
        self.side_clicks += u32::from(side.clicked);
        if self.child {
            self.draw_child(ui, &open, (x, y + 30.0));
        }
    }

    fn draw_child(&mut self, ui: &mut Ui<'_>, anchor: &Response, at: (f32, f32)) {
        let (picked, dismissed) = match self.kind {
            Child::Dropdown => {
                let rows = [("one", true), ("two", true)];
                let r = ui.dropdown("child", anchor.rect, anchor.id, &rows, 0);
                (r.picked == Some(1), r.dismissed)
            }
            Child::Popover => {
                let options = PopoverOptions {
                    anchor_id: Some(anchor.id),
                    ..PopoverOptions::new(anchor.rect, (200.0, 90.0))
                };
                let (frame, _, two) = ui.popover("child", &options, |ui, rect| {
                    let at = Rect::new(rect.x + 8.0, rect.y + 8.0, 100.0, 24.0);
                    ui.button("two", at, "two", ButtonKind::Secondary)
                });
                (two.clicked, frame.dismissed.is_some())
            }
            Child::Menu => {
                let item = |text| MenuRow::Item {
                    glyph: None,
                    text,
                    shortcut: "",
                };
                let r = ui.menu("child", &[item("one"), item("two")], at);
                (r.picked == Some(1), r.dismissed)
            }
        };
        self.picked += u32::from(picked);
        self.child &= !dismissed;
    }
}

fn bed_frame(h: &mut UiHarness, b: &mut Bed) -> (Scene, FrameOutput) {
    h.frame(|ui| {
        ui.painter().on(LayerId::Page);
        let page = ui.button("page", PAGE, "page", ButtonKind::Secondary);
        b.page_clicks += u32::from(page.clicked);
        if b.dialog {
            let r = ui.dialog("dialog", &DIALOG, &GLASS, |ui, x, y, _| b.body(ui, x, y));
            if r.cancelled {
                b.cancelled += 1;
                b.dialog = false;
            }
        }
    })
}

/// A settled open dialog, focus on the page button when it opened.
fn bed(kind: Child) -> (UiHarness, Bed) {
    let mut h = UiHarness::new(WIN);
    let mut b = Bed::of(kind);
    h.at_ms(0);
    bed_frame(&mut h, &mut b);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let (_, out) = bed_frame(&mut h, &mut b);
    assert_eq!(out.focused, Some(page_id()));
    b.dialog = true;
    bed_frame(&mut h, &mut b);
    settle(&mut h, &mut b, 1_000);
    (h, b)
}

fn page_id() -> WidgetId {
    WidgetId::ROOT.with("page")
}

/// Two frames, the second at `ms`: everything moving at that time is done.
fn settle(h: &mut UiHarness, b: &mut Bed, ms: u64) {
    h.at_ms(ms);
    bed_frame(h, b);
    bed_frame(h, b);
}

/// The child open and settled, opened with a click on the anchor, as a user does.
fn open_child(kind: Child) -> (UiHarness, Bed, FrameOutput) {
    let (mut h, mut b) = bed(kind);
    bed_frame(&mut h, &mut b);
    let r = b.under;
    h.input.pointer_to(r.x + 20.0, r.y - 20.0).click(P);
    bed_frame(&mut h, &mut b);
    assert!(b.child, "{kind:?}: the click on the anchor opens it");
    h.input.pointer_to(ON_SCRIM.0, ON_SCRIM.1);
    settle(&mut h, &mut b, 2_000);
    let (_, out) = bed_frame(&mut h, &mut b);
    (h, b, out)
}

fn has_content(scene: &Scene, layer: LayerId) -> bool {
    let l = &scene.layers[layer as usize];
    !(l.texts.is_empty() && l.geometry.under.is_empty() && l.geometry.acrylic.is_empty())
}

/// How much each kind of draw `layer` holds.
fn counts(scene: &Scene, layer: LayerId) -> (usize, usize, usize, usize) {
    let l = &scene.layers[layer as usize];
    let g = &l.geometry;
    (g.acrylic.len(), g.under.len(), g.over.len(), l.texts.len())
}

fn centre(out: &FrameOutput, name: &str) -> (f32, f32) {
    let r = out
        .tree
        .nodes
        .iter()
        .find(|n| n.name == name)
        .unwrap_or_else(|| panic!("a node named {name}"))
        .bounds;
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

#[test]
fn a_child_of_a_dialog_draws_on_raised_on_its_first_frames() {
    for kind in CHILDREN {
        let (mut h, mut b) = bed(kind);
        let (rest, _) = bed_frame(&mut h, &mut b);
        b.child = true;
        for frame in 1..=2 {
            let (scene, _) = bed_frame(&mut h, &mut b);
            let at = format!("{kind:?}, frame {frame}");
            assert!(has_content(&scene, LayerId::Raised), "{at}: on Raised");
            assert!(!has_content(&scene, LayerId::Popup), "{at}: not on Popup");
            assert_eq!(
                counts(&scene, LayerId::Modal),
                counts(&rest, LayerId::Modal),
                "{at}: the dialog's layer holds the dialog alone"
            );
        }
    }
}

#[test]
fn a_tooltip_in_a_dialog_is_on_raised_from_its_first_frame() {
    let (mut h, mut b) = bed(Child::Dropdown);
    let tip = |h: &mut UiHarness, ms: u64| {
        h.at_ms(ms);
        h.frame(|ui| {
            ui.dialog("dialog", &DIALOG, &GLASS, |ui, x, y, _| {
                let at = Rect::new(x, y, 120.0, 24.0);
                let r = ui.button("anchor", at, "open", ButtonKind::Secondary);
                ui.tooltip(&r, "hint");
            });
        })
        .0
    };
    let (scene, out) = bed_frame(&mut h, &mut b);
    assert!(!has_content(&scene, LayerId::Raised));
    let (x, y) = centre(&out, "open");
    h.input.pointer_to(x, y);
    tip(&mut h, 3_000);
    tip(&mut h, 3_016);
    let rest = counts(&tip(&mut h, 3_032), LayerId::Modal);
    let first = tip(&mut h, 4_000);
    let second = tip(&mut h, 4_016);
    for (i, scene) in [first, second].iter().enumerate() {
        assert!(has_content(scene, LayerId::Raised), "frame {}", i + 1);
        assert!(!has_content(scene, LayerId::Popup), "frame {}", i + 1);
        assert_eq!(counts(scene, LayerId::Modal), rest, "frame {}", i + 1);
    }
}

#[test]
fn a_press_inside_the_child_reaches_it_and_nothing_under_it() {
    for kind in CHILDREN {
        let (mut h, mut b, out) = open_child(kind);
        let (x, y) = centre(&out, "two");
        assert!(b.under.contains((x, y)), "{kind:?}: the button is under it");
        h.input.pointer_to(x, y);
        bed_frame(&mut h, &mut b);
        h.input.click(P);
        bed_frame(&mut h, &mut b);
        assert_eq!(b.picked, 1, "{kind:?}");
        assert_eq!(b.under_clicks + b.cancelled + b.page_clicks, 0, "{kind:?}");
    }
}

#[test]
fn a_press_on_the_card_outside_the_child_dismisses_it_only() {
    for kind in CHILDREN {
        let (mut h, mut b, _) = open_child(kind);
        let r = b.side;
        h.input.pointer_to(r.x + 10.0, r.y + 10.0);
        bed_frame(&mut h, &mut b);
        h.input.click(P);
        bed_frame(&mut h, &mut b);
        bed_frame(&mut h, &mut b);
        assert!(!b.child, "{kind:?}: dismissed");
        assert!(b.dialog, "{kind:?}: the dialog stays");
        assert_eq!(b.side_clicks + b.picked + b.page_clicks, 0, "{kind:?}");
    }
}

#[test]
fn a_press_on_the_scrim_dismisses_the_child_only() {
    for kind in CHILDREN {
        let (mut h, mut b, _) = open_child(kind);
        h.input.click(P);
        bed_frame(&mut h, &mut b);
        bed_frame(&mut h, &mut b);
        assert!(!b.child, "{kind:?}: dismissed");
        assert!(b.dialog && b.cancelled == 0, "{kind:?}: the dialog stays");
    }
}

#[test]
fn escape_closes_the_child_then_cancels_the_dialog_and_focus_walks_back() {
    for kind in CHILDREN {
        let (mut h, mut b, out) = open_child(kind);
        let anchor = b.anchor.expect("drawn");
        assert_ne!(out.focused, Some(anchor), "{kind:?}: the child has focus");

        h.input.key_down(Key::Named(NamedKey::Escape), None);
        bed_frame(&mut h, &mut b);
        assert!(
            !b.child && b.dialog,
            "{kind:?}: the first Escape is the child's"
        );
        let (_, out) = bed_frame(&mut h, &mut b);
        assert_eq!(out.focused, Some(anchor), "{kind:?}: back to the anchor");

        h.input.key_down(Key::Named(NamedKey::Escape), None);
        bed_frame(&mut h, &mut b);
        assert_eq!(
            b.cancelled, 1,
            "{kind:?}: the second Escape is the dialog's"
        );
        let (_, out) = bed_frame(&mut h, &mut b);
        assert_eq!(out.focused, Some(page_id()), "{kind:?}: back to the page");
    }
}

#[test]
fn tab_never_reaches_the_page_from_a_child_of_a_dialog() {
    for kind in CHILDREN {
        let (mut h, mut b, _) = open_child(kind);
        for _ in 0..8 {
            h.input.key_down(Key::Named(NamedKey::Tab), None);
            let (_, out) = bed_frame(&mut h, &mut b);
            assert!(out.focused.is_some(), "{kind:?}");
            assert_ne!(out.focused, Some(page_id()), "{kind:?}");
        }
    }
}

/// An overlay under `key` with `opts`, whose content fills `MENU` and whose exit does too.
fn marked(ui: &mut Ui<'_>, key: &str, opts: &OverlayOptions) -> Response {
    ui.overlay(
        key,
        opts,
        |p, _, _| p.fill(MENU, 0.0, [1, 2, 3, 255]),
        |ui, _| {
            ui.painter().fill(MENU, 0.0, [1, 2, 3, 255]);
            let id = ui.id((key, "hit"));
            ui.interact(id, MENU, Sense::CLICK_FOCUS)
        },
    )
    .1
}

fn fills(scene: &Scene, layer: LayerId) -> usize {
    scene.layers[layer as usize]
        .geometry
        .under
        .iter()
        .filter(|s| s.rect == MENU.phys(1.0))
        .count()
}

#[test]
fn an_overlay_opened_before_the_modal_stays_on_popup() {
    let mut h = harness();
    h.frame(|ui| {
        marked(ui, "early", &popup());
    });
    for _ in 0..3 {
        let (scene, _) = h.frame(|ui| {
            marked(ui, "early", &popup());
            dialog(ui, "dialog", &modal());
        });
        assert_eq!(fills(&scene, LayerId::Popup), 1);
        assert_eq!(fills(&scene, LayerId::Raised), 0);
    }
}

#[test]
fn without_a_modal_every_overlay_is_on_the_layer_it_asks_for() {
    let mut h = harness();
    for layer in [LayerId::Popup, LayerId::Modal, LayerId::Toast] {
        let opts = OverlayOptions { layer, ..popup() };
        for _ in 0..2 {
            let (scene, _) = h.frame(|ui| {
                marked(ui, "overlay", &opts);
            });
            assert_eq!(fills(&scene, layer), 1, "{layer:?}");
        }
        h.at_ms(10_000 * (layer as u64 + 1));
        h.frame(|_| {});
        h.frame(|_| {});
    }
}

#[test]
fn a_toast_layer_overlay_above_a_modal_is_not_raised() {
    let mut h = harness();
    let opts = OverlayOptions {
        layer: LayerId::Toast,
        ..popup()
    };
    let (scene, _) = h.frame(|ui| {
        dialog(ui, "dialog", &modal());
        marked(ui, "toast", &opts);
    });
    assert_eq!(fills(&scene, LayerId::Toast), 1);
}

#[test]
fn the_layer_holds_whatever_later_frames_ask_for() {
    let mut h = harness();
    h.frame(|ui| {
        marked(ui, "overlay", &popup());
    });
    let opts = OverlayOptions {
        layer: LayerId::Modal,
        ..popup()
    };
    let (scene, _) = h.frame(|ui| {
        marked(ui, "overlay", &opts);
    });
    assert_eq!(fills(&scene, LayerId::Popup), 1);
    assert_eq!(fills(&scene, LayerId::Modal), 0);
}

#[derive(Debug, Default)]
struct Nested {
    lower_clicks: u32,
    lower_ended: bool,
    upper_confirmed: u32,
    upper_cancelled: u32,
    lower_body: Rect,
}

fn nested_frame(h: &mut UiHarness, n: &mut Nested) -> (Scene, FrameOutput) {
    h.frame(|ui| {
        let r = ui.dialog("lower", &DIALOG, &GLASS, |ui, x, y, _| {
            n.lower_body = Rect::new(x, y, 300.0, 200.0);
            let b = ui.button("b", n.lower_body, "b", ButtonKind::Secondary);
            n.lower_clicks += u32::from(b.clicked);
            let r = ui.dialog("upper", &DIALOG, &GLASS, |_, _, _, _| {});
            n.upper_confirmed += u32::from(r.confirmed);
            n.upper_cancelled += u32::from(r.cancelled);
        });
        n.lower_ended |= r.confirmed || r.cancelled;
    })
}

#[test]
fn a_dialog_over_a_dialog_is_raised_takes_input_and_the_first_gets_none() {
    let mut h = UiHarness::new(WIN);
    let mut n = Nested::default();
    nested_frame(&mut h, &mut n);
    h.at_ms(1_000);
    nested_frame(&mut h, &mut n);
    let (scene, out) = nested_frame(&mut h, &mut n);
    assert!(has_content(&scene, LayerId::Raised));
    let dialogs = out.tree.nodes.iter().filter(|d| d.role == Role::Dialog);
    assert_eq!(dialogs.count(), 2);

    // The upper confirm takes the press, though the lower card lies under it.
    let yes = out
        .tree
        .nodes
        .iter()
        .rfind(|d| d.name == "Yes")
        .expect("upper confirm")
        .bounds;
    h.input.pointer_to(yes.x + 4.0, yes.y + 4.0);
    nested_frame(&mut h, &mut n);
    h.input.click(P);
    nested_frame(&mut h, &mut n);
    assert_eq!((n.upper_confirmed, n.upper_cancelled), (1, 0));
    assert!(!n.lower_ended);

    // A press on the lower dialog's body lands on the upper card and goes no further.
    let body = n.lower_body;
    h.input.pointer_to(body.x + 150.0, body.y + 100.0);
    nested_frame(&mut h, &mut n);
    h.input.click(P);
    nested_frame(&mut h, &mut n);
    assert_eq!(n.lower_clicks, 0, "the first dialog gets no input");
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    nested_frame(&mut h, &mut n);
    assert_eq!(n.upper_cancelled, 1, "Escape is the upper dialog's");
    assert!(!n.lower_ended);
}

#[test]
fn a_parent_closing_first_leaves_the_child_raised_until_it_closes() {
    let mut h = harness();
    let (scene, _) = h.frame(|ui| {
        dialog(ui, "dialog", &modal());
        marked(ui, "child", &popup());
    });
    assert_eq!(fills(&scene, LayerId::Raised), 1);
    for ms in [10, 1_000, 2_000] {
        h.at_ms(ms);
        let (scene, _) = h.frame(|ui| {
            marked(ui, "child", &popup());
        });
        assert_eq!(fills(&scene, LayerId::Raised), 1, "held at {ms} ms");
        assert_eq!(fills(&scene, LayerId::Popup), 0);
    }
    h.at_ms(2_010);
    let (scene, _) = h.frame(|_| {});
    assert_eq!(
        fills(&scene, LayerId::Raised),
        1,
        "its exit is on Raised too"
    );
    assert_eq!(fills(&scene, LayerId::Popup), 0);
}

#[test]
fn a_child_reopening_after_its_parent_closed_is_on_popup() {
    let mut h = harness();
    h.frame(|ui| {
        dialog(ui, "dialog", &modal());
        marked(ui, "child", &popup());
    });
    h.at_ms(1_000);
    h.frame(|ui| {
        dialog(ui, "dialog", &modal());
        marked(ui, "child", &popup());
    });
    h.at_ms(1_010);
    let (closing, _) = h.frame(|_| {});
    assert_eq!(
        fills(&closing, LayerId::Raised),
        1,
        "still closing, on Raised"
    );
    h.at_ms(1_020);
    for _ in 0..2 {
        let (scene, _) = h.frame(|ui| {
            marked(ui, "child", &popup());
        });
        assert_eq!(fills(&scene, LayerId::Popup), 1);
        assert_eq!(fills(&scene, LayerId::Raised), 0);
    }
}

/// What one frame of the reopen tests' overlays reports.
#[derive(Default)]
struct Reopen {
    scene: Option<Scene>,
    focused: Option<WidgetId>,
    enter: bool,
    clicked: bool,
    confirm_enter: bool,
    dismissed: Option<Dismissal>,
}

/// A dialog when asked for, and after it the popup `child` when asked for. Both read Enter.
fn reopen_frame(h: &mut UiHarness, at: u64, with_dialog: bool, with_popup: bool) -> Reopen {
    let mut r = Reopen::default();
    let (scene, out) = h.at_ms(at).frame(|ui| {
        if with_dialog {
            ui.overlay(
                "dialog",
                &modal(),
                |_, _, _| {},
                |ui, _| {
                    let ok = ui.id("ok");
                    let at = Rect::new(160.0, 180.0, 60.0, 24.0);
                    ui.interact(ok, at, Sense::CLICK_FOCUS);
                    ui.node(Node::new(ok, Role::Button, "ok", at));
                    r.confirm_enter = ui.consume_key(ok, NamedKey::Enter);
                },
            );
        }
        if with_popup {
            let (f, b) = ui.overlay(
                "child",
                &popup(),
                |p, _, _| p.fill(MENU, 0.0, [1, 2, 3, 255]),
                |ui, _| {
                    ui.painter().fill(MENU, 0.0, [1, 2, 3, 255]);
                    let id = ui.id("row");
                    let b = ui.interact(id, MENU, Sense::CLICK_FOCUS);
                    ui.node(Node::new(id, Role::Button, "row", MENU));
                    r.enter = ui.consume_key(id, NamedKey::Enter);
                    b
                },
            );
            r.clicked = b.clicked;
            r.dismissed = f.dismissed;
        }
    });
    r.scene = Some(scene);
    r.focused = out.focused;
    r
}

/// A popup open and settled, closing from 1,000 ms, a dialog opened at 1,010 ms with its button
/// focused, and the popup called again at 1,020 ms, inside its close.
fn reopened_under_a_modal() -> (UiHarness, Reopen) {
    let mut h = harness();
    reopen_frame(&mut h, 0, false, true);
    reopen_frame(&mut h, 500, false, true);
    reopen_frame(&mut h, 1_000, false, false);
    reopen_frame(&mut h, 1_010, true, false);
    let held = reopen_frame(&mut h, 1_015, true, false);
    assert!(held.focused.is_some(), "the dialog took focus");
    assert!(
        fills(held.scene.as_ref().expect("scene"), LayerId::Popup) >= 1,
        "the popup is still closing"
    );
    let r = reopen_frame(&mut h, 1_020, true, true);
    (h, r)
}

#[test]
fn a_popup_reopened_during_its_close_under_a_newer_modal_is_raised_on_every_frame() {
    let (mut h, r) = reopened_under_a_modal();
    let scene = r.scene.expect("scene");
    assert_eq!(fills(&scene, LayerId::Raised), 1, "on the reopen's frame");
    assert_eq!(fills(&scene, LayerId::Popup), 0);
    for t in [1_030, 1_300, 1_600] {
        let r = reopen_frame(&mut h, t, true, true);
        let scene = r.scene.expect("scene");
        assert_eq!(fills(&scene, LayerId::Raised), 1, "at {t}");
        assert_eq!(fills(&scene, LayerId::Popup), 0, "at {t}");
    }
}

#[test]
fn a_popup_reopened_under_a_newer_modal_takes_input_and_gives_focus_back() {
    let (mut h, r) = reopened_under_a_modal();
    let row = WidgetId::ROOT.with("row");
    let ok = WidgetId::ROOT.with("ok");
    assert_eq!(r.focused, Some(row), "a true open takes focus");
    for t in [1_300, 1_600] {
        reopen_frame(&mut h, t, true, true);
    }
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let r = reopen_frame(&mut h, 1_610, true, true);
    assert!(r.enter && !r.confirm_enter, "its keys are live");
    h.input.pointer_to(MENU.x + 5.0, MENU.y + 5.0);
    reopen_frame(&mut h, 1_620, true, true);
    h.input.click(P);
    assert!(reopen_frame(&mut h, 1_630, true, true).clicked);
    // Escape closes it alone, focus goes back to the dialog, and the dialog has the keys.
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let r = reopen_frame(&mut h, 1_640, true, true);
    assert_eq!(r.dismissed, Some(Dismissal::Escape));
    let r = reopen_frame(&mut h, 1_650, true, false);
    assert_eq!(r.focused, Some(ok));
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    assert!(reopen_frame(&mut h, 1_660, true, false).confirm_enter);
}

#[test]
fn a_popup_reopened_with_no_new_modal_keeps_its_place() {
    let mut h = harness();
    reopen_frame(&mut h, 0, false, true);
    reopen_frame(&mut h, 500, false, true);
    reopen_frame(&mut h, 1_000, false, false);
    let r = reopen_frame(&mut h, 1_020, false, true);
    assert_eq!(fills(&r.scene.expect("scene"), LayerId::Popup), 1);
    assert_eq!(r.focused, Some(WidgetId::ROOT.with("row")));
}

#[test]
fn a_child_reopening_under_its_open_parent_stays_raised() {
    let mut h = harness();
    h.frame(|ui| {
        dialog(ui, "dialog", &modal());
        marked(ui, "child", &popup());
    });
    h.at_ms(1_000);
    h.frame(|ui| {
        dialog(ui, "dialog", &modal());
    });
    h.at_ms(1_010);
    let (scene, _) = h.frame(|ui| {
        dialog(ui, "dialog", &modal());
        marked(ui, "child", &popup());
    });
    assert_eq!(fills(&scene, LayerId::Raised), 1);
}

#[test]
fn a_toast_layer_hit_stays_eligible_above_raised() {
    let mut h = harness();
    let run = |h: &mut UiHarness| {
        let mut out = None;
        h.frame(|ui| {
            dialog(ui, "lower", &modal());
            dialog(ui, "upper", &modal());
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

/// One frame of a dialog holding `child` when `open`, in a settled window, then the frame the
/// dialog stops being called, at `ms`.
fn closing_frame(kind: Child, open: bool, ms: u64) -> Scene {
    let (mut h, mut b) = bed(kind);
    b.child = open;
    settle(&mut h, &mut b, 2_000);
    b.dialog = false;
    b.child = false;
    h.at_ms(2_000 + ms);
    bed_frame(&mut h, &mut b).0
}

#[test]
fn a_closing_dialog_replays_none_of_its_child() {
    for kind in CHILDREN {
        let (ga, gu, go, gt) = counts(&closing_frame(kind, true, 30), LayerId::Modal);
        let (wa, wu, wo, wt) = counts(&closing_frame(kind, false, 30), LayerId::Modal);
        assert!(gu > 0, "{kind:?}: still closing");
        // Without the child the dialog's button keeps focus, so its ring is the one shape more.
        assert!(gu <= wu, "{kind:?}: no shape of the child");
        assert_eq!((ga, go, gt), (wa, wo, wt), "{kind:?}: the dialog alone");
    }
}

#[test]
fn a_child_closing_with_its_dialog_leaves_on_its_own_layer() {
    // The dropdown snaps by contract (it has no exit); the other two fade on Raised.
    let snap = closing_frame(Child::Dropdown, true, 30);
    assert!(!has_content(&snap, LayerId::Raised));
    assert!(!has_content(&snap, LayerId::Popup));
    for kind in [Child::Popover, Child::Menu] {
        let scene = closing_frame(kind, true, 30);
        assert!(has_content(&scene, LayerId::Raised), "{kind:?}");
        assert!(!has_content(&scene, LayerId::Popup), "{kind:?}");
    }
    let glass = closing_frame(Child::Menu, true, 30);
    let b = glass.backdrop.expect("the menu's own exit asks for it");
    assert_eq!(b.layer, LayerId::Raised as usize);
}

#[test]
fn a_tooltip_inside_a_popover_is_not_in_the_popover_exit() {
    let mut h = UiHarness::new(WIN);
    let anchor = Rect::new(100.0, 100.0, 50.0, 20.0);
    let inner = Rect::new(108.0, 132.0, 100.0, 24.0);
    let run = |h: &mut UiHarness, open: bool| {
        h.frame(|ui| {
            if open {
                let options = PopoverOptions::new(anchor, (200.0, 90.0));
                ui.popover("pop", &options, |ui, _| {
                    let r = ui.button("in", inner, "in", ButtonKind::Secondary);
                    ui.tooltip(&r, "hint");
                });
            }
        })
        .0
    };
    h.input.pointer_to(inner.x + 4.0, inner.y + 4.0);
    run(&mut h, true);
    h.at_ms(1_000);
    run(&mut h, true);
    h.at_ms(2_000);
    run(&mut h, true);
    h.at_ms(3_000);
    let open = run(&mut h, true);
    let drawn = open.layers[LayerId::Popup as usize].texts.len();
    assert_eq!(drawn, 2, "the button's label and the tooltip's text");
    h.at_ms(3_010);
    let closing = run(&mut h, false);
    assert_eq!(
        closing.layers[LayerId::Popup as usize].texts.len(),
        drawn,
        "each exit draws its own, once"
    );
}

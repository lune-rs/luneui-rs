//! The frame's ordered input (M5 step 1): the focus event queue and its text filter, composing,
//! the keymap, press details, pointer queries, cursor and IME output, capture keys, the Shift
//! wheel, translate and nested clips.

use std::time::Duration;

use lune_ui_core::input::{
    CursorIcon, FocusEvent, ImeEvent, Key, Keymap, Modifiers, NamedKey, PointerButton, Repaint,
};
use lune_ui_core::motion::{Kind, Motion};
use lune_ui_core::paint::{LayerId, Painter, Rect, Xform};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{OverlayOptions, Response, Sense, WidgetId};
use lune_ui_core::widgets::field;

const P: PointerButton = PointerButton::Primary;
const A: Rect = Rect::new(10.0, 10.0, 100.0, 30.0);
const B: Rect = Rect::new(130.0, 10.0, 100.0, 30.0);

fn harness() -> UiHarness {
    UiHarness::new((400.0, 300.0))
}

fn mods(shift: bool, control: bool, alt: bool, logo: bool) -> Modifiers {
    Modifiers {
        shift,
        control,
        alt,
        logo,
    }
}

const CMD: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: true,
};

fn ch(s: &str) -> Key {
    Key::Character(s.to_owned())
}

/// Two focus stops `a` and `b`; focuses `a` with a click and settles. Returns `a`'s id.
fn focused_a(h: &mut UiHarness) -> WidgetId {
    let id = WidgetId::ROOT.with("a");
    let frame = |h: &mut UiHarness| {
        h.frame(|ui| {
            ui.interact(ui.id("a"), A, Sense::CLICK_FOCUS);
            ui.interact(ui.id("b"), B, Sense::CLICK_FOCUS);
        });
    };
    frame(h);
    h.input.pointer_to(20.0, 20.0).click(P);
    frame(h);
    frame(h);
    id
}

/// Runs a frame of `a` and `b` and returns `a`'s focus events.
fn events_of_a(h: &mut UiHarness) -> Vec<FocusEvent> {
    let mut out = Vec::new();
    h.frame(|ui| {
        let a = ui.id("a");
        ui.interact(a, A, Sense::CLICK_FOCUS);
        ui.interact(ui.id("b"), B, Sense::CLICK_FOCUS);
        out = ui.focus_events(a).into_iter().map(|(_, e)| e).collect();
    });
    out
}

fn key(k: Key, modifiers: Modifiers) -> FocusEvent {
    FocusEvent::Key {
        key: k,
        modifiers,
        repeat: false,
    }
}

#[test]
fn a_plain_character_press_yields_the_key_then_the_text() {
    let mut h = harness();
    focused_a(&mut h);
    h.input.key_down(ch("a"), Some("a"));
    assert_eq!(
        events_of_a(&mut h),
        [
            key(ch("a"), Modifiers::default()),
            FocusEvent::Text("a".into())
        ]
    );
}

#[test]
fn command_a_yields_the_key_and_drops_its_text() {
    let mut h = harness();
    focused_a(&mut h);
    h.input.set_modifiers(CMD).key_down(ch("a"), Some("a"));
    assert_eq!(events_of_a(&mut h), [key(ch("a"), CMD)]);
}

#[test]
fn control_without_alt_drops_the_text_and_ctrl_alt_keeps_it() {
    let mut h = harness();
    focused_a(&mut h);
    let ctrl = mods(false, true, false, false);
    h.input.set_modifiers(ctrl).key_down(ch("e"), Some("e"));
    assert_eq!(events_of_a(&mut h), [key(ch("e"), ctrl)]);
    let ctrl_alt = mods(false, true, true, false);
    h.input.set_modifiers(ctrl_alt).key_down(ch("e"), Some("€"));
    assert_eq!(
        events_of_a(&mut h),
        [key(ch("e"), ctrl_alt), FocusEvent::Text("€".into())]
    );
}

#[test]
fn text_that_does_not_follow_a_key_press_is_kept() {
    // An IME-less text event after a modified press of another key still counts as "right after
    // a key" only for the next event: here the text comes after a Release, so it stays.
    let mut h = harness();
    focused_a(&mut h);
    h.input.set_modifiers(CMD).key_down(ch("a"), None);
    h.input.key_up(ch("a"));
    h.input.set_modifiers(Modifiers::default());
    h.input.key_down(ch("b"), Some("b"));
    let got = events_of_a(&mut h);
    assert_eq!(got.len(), 3, "{got:?}");
    assert_eq!(got[2], FocusEvent::Text("b".into()));
}

#[test]
fn events_arrive_in_order_and_only_for_the_focused_widget() {
    let mut h = harness();
    focused_a(&mut h);
    h.input
        .key_down(Key::Named(NamedKey::Backspace), None)
        .key_down(ch("x"), Some("x"))
        .ime(ImeEvent::Preedit {
            text: "か".into(),
            cursor: None,
        })
        .clipboard_text("clip");
    let mut b_events = None;
    let mut ev = Vec::new();
    h.frame(|ui| {
        let (a, b) = (ui.id("a"), ui.id("b"));
        ui.interact(a, A, Sense::CLICK_FOCUS);
        ui.interact(b, B, Sense::CLICK_FOCUS);
        b_events = Some(ui.focus_events(b));
        ev = ui.focus_events(a);
    });
    assert_eq!(b_events, Some(Vec::new()), "b does not hold the key focus");
    let kinds: Vec<&FocusEvent> = ev.iter().map(|(_, e)| e).collect();
    assert_eq!(
        kinds,
        [
            &key(Key::Named(NamedKey::Backspace), Modifiers::default()),
            &key(ch("x"), Modifiers::default()),
            &FocusEvent::Text("x".into()),
            &FocusEvent::Ime(ImeEvent::Preedit {
                text: "か".into(),
                cursor: None
            }),
            &FocusEvent::ClipboardText("clip".into()),
        ]
    );
    let idx: Vec<usize> = ev.iter().map(|(i, _)| *i).collect();
    assert!(idx.windows(2).all(|w| w[0] < w[1]), "{idx:?}");
}

#[test]
fn a_consumed_event_is_gone_and_asks_for_a_catch_up_frame() {
    let mut h = harness();
    focused_a(&mut h);
    h.input.key_down(ch("x"), Some("x"));
    let (_, out) = h.frame(|ui| {
        let a = ui.id("a");
        ui.interact(a, A, Sense::CLICK_FOCUS);
        let evs = ui.focus_events(a);
        assert_eq!(evs.len(), 2);
        ui.consume_focus_event(evs[1].0);
        let rest = ui.focus_events(a);
        assert_eq!(rest.len(), 1);
        assert!(matches!(rest[0].1, FocusEvent::Key { .. }));
    });
    assert_eq!(out.repaint, Repaint::Now);
}

#[test]
fn unconsumed_text_reports_idle() {
    let mut h = harness();
    focused_a(&mut h);
    h.input.key_down(ch("x"), Some("x"));
    let (_, out) = h.frame(|ui| {
        ui.interact(ui.id("a"), A, Sense::CLICK_FOCUS);
    });
    assert_eq!(out.repaint, Repaint::Idle);
}

#[test]
fn a_key_is_taken_once_by_consume_key_or_consume_focus_event() {
    let mut h = harness();
    focused_a(&mut h);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    h.frame(|ui| {
        let a = ui.id("a");
        ui.interact(a, A, Sense::CLICK_FOCUS);
        assert!(ui.consume_key(a, NamedKey::Enter));
        assert!(!ui.consume_key(a, NamedKey::Enter));
        assert!(ui.focus_events(a).is_empty());
    });
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    h.frame(|ui| {
        let a = ui.id("a");
        ui.interact(a, A, Sense::CLICK_FOCUS);
        let evs = ui.focus_events(a);
        ui.consume_focus_event(evs[0].0);
        assert!(!ui.consume_key(a, NamedKey::Enter));
    });
}

#[test]
fn a_character_key_is_not_a_named_key() {
    let mut h = harness();
    focused_a(&mut h);
    h.input.key_down(ch("a"), None);
    h.frame(|ui| {
        let a = ui.id("a");
        ui.interact(a, A, Sense::CLICK_FOCUS);
        assert!(!ui.consume_key(a, NamedKey::Enter));
        assert_eq!(ui.focus_events(a).len(), 1);
    });
}

#[test]
fn repeat_and_modifiers_travel_with_a_key() {
    let mut h = harness();
    focused_a(&mut h);
    h.input
        .set_modifiers(mods(true, false, false, false))
        .key_repeat(Key::Named(NamedKey::ArrowLeft));
    let got = events_of_a(&mut h);
    assert_eq!(
        got,
        [FocusEvent::Key {
            key: Key::Named(NamedKey::ArrowLeft),
            modifiers: mods(true, false, false, false),
            repeat: true,
        }]
    );
}

fn dialog_frame(
    h: &mut UiHarness,
    composing: Option<bool>,
) -> (Option<lune_ui_core::ui::Dismissal>, usize) {
    let mut dismissed = None;
    let mut n = 0;
    h.frame(|ui| {
        let opts = OverlayOptions {
            layer: LayerId::Modal,
            modal: true,
            dismiss_on_escape: true,
            dismiss_on_outside_press: false,
            dismiss_on_focus_loss: false,
            anchor: None,
            initial_focus: None,
            open: Motion::of(Kind::Appear),
            close: Motion::of(Kind::Appear),
        };
        let (f, ok) = ui.overlay(
            "dlg",
            &opts,
            |_, _, _| {},
            |ui, _| {
                let id = ui.id("ok");
                ui.interact(id, Rect::new(150.0, 150.0, 60.0, 24.0), Sense::CLICK_FOCUS);
                id
            },
        );
        dismissed = f.dismissed;
        if let Some(c) = composing {
            ui.set_composing(ok, c);
        }
        n = ui
            .focus_events(ok)
            .iter()
            .filter(|(_, e)| {
                matches!(
                    e,
                    FocusEvent::Key {
                        key: Key::Named(NamedKey::Escape),
                        ..
                    }
                )
            })
            .count();
    });
    (dismissed, n)
}

#[test]
fn escape_while_composing_dismisses_nothing_and_joins_the_queue() {
    let mut h = harness();
    dialog_frame(&mut h, None);
    dialog_frame(&mut h, Some(true));
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    assert_eq!(dialog_frame(&mut h, Some(true)), (None, 1));
    // Not composing: the dialog takes the Escape.
    dialog_frame(&mut h, Some(false));
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (dismissed, n) = dialog_frame(&mut h, None);
    assert_eq!(dismissed, Some(lune_ui_core::ui::Dismissal::Escape));
    assert_eq!(n, 0);
}

fn two_stops(h: &mut UiHarness, composing: bool) -> (bool, bool, usize) {
    let (mut fa, mut fb, mut tabs) = (false, false, 0);
    h.frame(|ui| {
        let (a, b) = (ui.id("a"), ui.id("b"));
        let ra = ui.interact(a, A, Sense::CLICK_FOCUS);
        let rb = ui.interact(b, B, Sense::CLICK_FOCUS);
        ui.set_composing(a, composing);
        (fa, fb) = (ra.focused, rb.focused);
        tabs = ui
            .focus_events(a)
            .iter()
            .filter(|(_, e)| {
                matches!(
                    e,
                    FocusEvent::Key {
                        key: Key::Named(NamedKey::Tab),
                        ..
                    }
                )
            })
            .count();
    });
    (fa, fb, tabs)
}

#[test]
fn tab_while_composing_keeps_focus_and_joins_the_queue() {
    let mut h = harness();
    focused_a(&mut h);
    two_stops(&mut h, true);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    assert_eq!(two_stops(&mut h, true), (true, false, 1));
    // Once the widget says it stopped composing, Tab moves on.
    two_stops(&mut h, false);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    assert_eq!(two_stops(&mut h, false), (false, true, 0));
}

#[test]
fn composing_ends_when_focus_moves() {
    let mut h = harness();
    focused_a(&mut h);
    two_stops(&mut h, true);
    // A pointer press on b moves focus; a later return to a does not revive the composition.
    h.input.pointer_to(140.0, 20.0).click(P);
    two_stops(&mut h, false);
    h.input.pointer_to(20.0, 20.0).click(P);
    two_stops(&mut h, false);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    assert_eq!(two_stops(&mut h, false), (false, true, 0));
}

#[test]
fn keymap_constants_and_modifier_tests() {
    let ctrl = mods(false, true, false, false);
    let alt = mods(false, false, true, false);
    assert!(Keymap::MAC.is_primary(CMD) && !Keymap::MAC.is_primary(ctrl));
    assert!(Keymap::PC.is_primary(ctrl) && !Keymap::PC.is_primary(CMD));
    assert!(Keymap::MAC.is_word(alt) && !Keymap::MAC.is_word(ctrl));
    assert!(Keymap::PC.is_word(ctrl) && !Keymap::PC.is_word(alt));
    assert_ne!(Keymap::MAC.word_motion, Keymap::PC.word_motion);
}

#[test]
fn the_frame_reports_the_keymap_it_was_given() {
    let mut h = harness();
    let mut seen = Vec::new();
    h.frame(|ui| seen.push(ui.keymap()));
    h.keymap = Keymap::PC;
    h.frame(|ui| seen.push(ui.keymap()));
    assert_eq!(seen, [Keymap::MAC, Keymap::PC]);
}

fn press_frame(h: &mut UiHarness) -> Response {
    let mut r = None;
    h.frame(|ui| {
        let a = ui.id("a");
        r = Some(ui.interact(a, A, Sense::CLICK_FOCUS));
        ui.interact(ui.id("b"), B, Sense::CLICK_FOCUS);
    });
    r.expect("frame ran")
}

#[test]
fn a_press_reports_its_position_count_and_modifiers() {
    let mut h = harness();
    press_frame(&mut h);
    let shift = mods(true, false, false, false);
    h.input.pointer_to(30.0, 25.0).set_modifiers(shift).press(P);
    let r = press_frame(&mut h);
    assert!(r.press_started && !r.clicked);
    assert_eq!(r.press_pos, Some((30.0, 25.0)));
    assert_eq!((r.press_count, r.press_modifiers), (1, shift));
    // The next frame has no press edge.
    h.input.release(P);
    let r = press_frame(&mut h);
    assert!(!r.press_started && r.press_pos.is_none() && r.press_count == 0);
    assert!(r.clicked);
}

#[test]
fn press_counts_run_one_two_three_then_one() {
    let mut h = harness();
    press_frame(&mut h);
    h.input.pointer_to(20.0, 20.0);
    let mut counts = Vec::new();
    for ms in [0, 100, 200, 300, 400] {
        h.at_ms(ms).input.press(P);
        counts.push(press_frame(&mut h).press_count);
        h.at_ms(ms + 10).input.release(P);
        press_frame(&mut h);
    }
    assert_eq!(counts, [1, 2, 3, 1, 2]);
}

#[test]
fn a_press_after_the_double_click_time_or_on_another_id_restarts_the_count() {
    let mut h = harness();
    press_frame(&mut h);
    h.input.pointer_to(20.0, 20.0);
    h.at_ms(0).input.press(P);
    assert_eq!(press_frame(&mut h).press_count, 1);
    h.at_ms(10).input.release(P);
    press_frame(&mut h);
    h.at_ms(500).input.press(P);
    assert_eq!(press_frame(&mut h).press_count, 1, "400 ms or more apart");
    h.at_ms(510).input.release(P);
    press_frame(&mut h);
    // A press on b, then back on a within the time: 1 again.
    h.at_ms(520).input.pointer_to(140.0, 20.0).press(P);
    let mut b = 0;
    h.frame(|ui| {
        ui.interact(ui.id("a"), A, Sense::CLICK_FOCUS);
        b = ui.interact(ui.id("b"), B, Sense::CLICK_FOCUS).press_count;
    });
    assert_eq!(b, 1);
    h.at_ms(530).input.release(P);
    press_frame(&mut h);
    h.at_ms(540).input.pointer_to(20.0, 20.0).press(P);
    assert_eq!(press_frame(&mut h).press_count, 1);
}

#[test]
fn pointer_pos_follows_the_pointer_and_clears_when_it_leaves() {
    let mut h = harness();
    let mut seen = Vec::new();
    h.frame(|ui| seen.push(ui.pointer_pos()));
    h.input.pointer_to(33.0, 44.0);
    h.frame(|ui| seen.push(ui.pointer_pos()));
    h.input.pointer_leave();
    h.frame(|ui| seen.push(ui.pointer_pos()));
    assert_eq!(seen, [None, Some((33.0, 44.0)), None]);
}

fn popup_opts() -> OverlayOptions {
    OverlayOptions {
        layer: LayerId::Popup,
        modal: false,
        dismiss_on_escape: false,
        dismiss_on_outside_press: false,
        dismiss_on_focus_loss: false,
        anchor: None,
        initial_focus: None,
        open: Motion::of(Kind::Appear),
        close: Motion::of(Kind::Appear),
    }
}

#[test]
fn pointer_in_ignores_blocking_children_and_disabled_anchors_but_not_a_popup() {
    let mut h = harness();
    let run = |h: &mut UiHarness, popup: bool| {
        let mut got = (false, false);
        h.frame(|ui| {
            // The anchor is disabled; a child that blocks hover sits on top of it.
            ui.with_enabled(false, |ui| {
                ui.interact(ui.id("anchor"), A, Sense::CLICK);
            });
            ui.interact(ui.id("child"), A.inset(20.0, 5.0), Sense::HOVER);
            let far = Rect::new(300.0, 200.0, 40.0, 40.0);
            got = (ui.pointer_in(A), ui.pointer_in(far));
            if popup {
                ui.overlay(
                    "pop",
                    &popup_opts(),
                    |_, _, _| {},
                    |ui, _| {
                        ui.interact(
                            ui.id("card"),
                            Rect::new(0.0, 0.0, 120.0, 60.0),
                            Sense::HOVER,
                        );
                    },
                );
            }
        });
        got
    };
    run(&mut h, false);
    h.input.pointer_to(50.0, 25.0);
    run(&mut h, false);
    assert_eq!(run(&mut h, false), (true, false), "over a blocking child");
    // The popup opens over the anchor.
    run(&mut h, true);
    assert_eq!(run(&mut h, true), (false, false), "under an open popup");
}

#[test]
fn cursor_and_ime_area_reach_the_frame_output_last_call_wins() {
    let mut h = harness();
    let (_, out) = h.frame(|_| {});
    assert_eq!((out.cursor, out.ime_area), (CursorIcon::Default, None));
    let area = Rect::new(1.0, 2.0, 3.0, 4.0);
    let (_, out) = h.frame(|ui| {
        ui.set_cursor(CursorIcon::Pointer);
        ui.set_cursor(CursorIcon::Text);
        ui.set_ime_area(Some(Rect::new(9.0, 9.0, 9.0, 9.0)));
        ui.set_ime_area(Some(area));
    });
    assert_eq!((out.cursor, out.ime_area), (CursorIcon::Text, Some(area)));
    let (_, out) = h.frame(|_| {});
    assert_eq!((out.cursor, out.ime_area), (CursorIcon::Default, None));
}

#[test]
fn consume_key_while_pressed_works_without_focus() {
    let mut h = harness();
    let run = |h: &mut UiHarness| {
        let mut got = (false, false, false);
        h.frame(|ui| {
            let id = ui.id("drag");
            let r = ui.interact(id, A, Sense::DRAG);
            if r.pressed {
                got.0 = ui.consume_key(id, NamedKey::Escape);
                got.1 = ui.consume_key_while_pressed(id, NamedKey::Escape);
                got.2 = ui.consume_key_while_pressed(id, NamedKey::Escape);
            }
        });
        got
    };
    run(&mut h);
    h.input.pointer_to(20.0, 20.0).press(P);
    run(&mut h);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    assert_eq!(run(&mut h), (false, true, false));
    // Released: no capture, nothing to take.
    h.input.release(P);
    run(&mut h);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    assert_eq!(run(&mut h), (false, false, false));
}

fn wheel(h: &mut UiHarness) -> ((f32, f32), (f32, f32)) {
    let mut got = ((0.0, 0.0), (0.0, 0.0));
    h.frame(|ui| {
        let r = ui.interact(ui.id("area"), A, Sense::SCROLL);
        got = (r.scroll_lines, r.scroll_pixels);
    });
    got
}

#[test]
fn shift_turns_a_vertical_line_wheel_into_a_horizontal_one_and_leaves_pixels_alone() {
    let mut h = harness();
    wheel(&mut h);
    h.input.pointer_to(20.0, 20.0);
    h.input.scroll_lines(0.0, -3.0);
    assert_eq!(wheel(&mut h), ((0.0, -3.0), (0.0, 0.0)));
    let shift = mods(true, false, false, false);
    h.input.set_modifiers(shift).scroll_lines(0.0, -3.0);
    assert_eq!(wheel(&mut h), ((-3.0, 0.0), (0.0, 0.0)));
    h.input.scroll_lines(2.0, -3.0);
    assert_eq!(wheel(&mut h), ((2.0, -3.0), (0.0, 0.0)), "x was set");
    h.input.scroll_pixels(0.0, -7.0);
    assert_eq!(wheel(&mut h), ((0.0, 0.0), (0.0, -7.0)));
}

fn painter(text: &mut FakeText) -> Painter<'_, u8> {
    Painter::new(1.0, Rect::new(0.0, 0.0, 400.0, 300.0), text, None, DARK)
}

#[test]
fn translate_moves_draws_and_hits() {
    let mut t = FakeText;
    let mut p = painter(&mut t);
    p.push_xform(Xform::translate(10.0, 5.0));
    assert_eq!(
        p.transformed(Rect::new(0.0, 0.0, 20.0, 20.0)),
        Rect::new(10.0, 5.0, 20.0, 20.0)
    );
    p.fill(Rect::new(1.0, 2.0, 3.0, 4.0), 0.0, [255; 4]);
    p.hit(Rect::new(1.0, 2.0, 3.0, 4.0), 7);
    assert_eq!(
        p.scene.layers[0].geometry.under[0].rect,
        [11.0, 7.0, 3.0, 4.0]
    );
    assert_eq!(p.hits[0].0, Rect::new(11.0, 7.0, 3.0, 4.0));
    // The offset is added after the scale: scale 2 about the origin, then 10 right.
    p.pop_xform();
    p.push_xform(Xform {
        origin: (0.0, 0.0),
        scale: 2.0,
        dx: 10.0,
        dy: 0.0,
        alpha: 1.0,
    });
    assert_eq!(
        p.transformed(Rect::new(5.0, 0.0, 1.0, 1.0)),
        Rect::new(20.0, 0.0, 2.0, 2.0)
    );
}

#[test]
fn push_clip_intersects_and_pop_clip_restores() {
    let mut t = FakeText;
    let mut p = painter(&mut t);
    let fill = |p: &mut Painter<'_, u8>| p.fill(Rect::new(0.0, 0.0, 300.0, 300.0), 0.0, [255; 4]);
    p.push_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
    p.push_clip(Rect::new(50.0, 50.0, 100.0, 100.0));
    fill(&mut p);
    p.pop_clip();
    fill(&mut p);
    p.pop_clip();
    fill(&mut p);
    p.pop_clip(); // no clip pushed: does nothing
    fill(&mut p);
    let clips: Vec<[f32; 4]> = p.scene.layers[0]
        .geometry
        .under
        .iter()
        .map(|s| s.clip)
        .collect();
    assert_eq!(
        clips,
        [
            [50.0, 50.0, 50.0, 50.0],
            [0.0, 0.0, 100.0, 100.0],
            [0.0; 4],
            [0.0; 4]
        ]
    );
}

#[test]
fn a_field_painted_under_an_outer_clip_leaves_that_clip_in_force() {
    let mut t = FakeText;
    let mut p = painter(&mut t);
    let outer = Rect::new(0.0, 0.0, 200.0, 100.0);
    p.push_clip(outer);
    field::field(
        &mut p,
        Rect::new(10.0, 10.0, 120.0, 30.0),
        "hi",
        "ph",
        true,
        1,
    );
    p.fill(Rect::new(0.0, 0.0, 300.0, 300.0), 0.0, [255; 4]);
    let last = p.scene.layers[0]
        .geometry
        .under
        .last()
        .copied()
        .expect("drew");
    assert_eq!(last.clip, [0.0, 0.0, 200.0, 100.0]);
    // The field's own text is clipped to the outer clip and its inset together.
    p.pop_clip();
    p.fill(Rect::new(0.0, 0.0, 300.0, 300.0), 0.0, [255; 4]);
    assert_eq!(
        p.scene.layers[0].geometry.under.last().map(|s| s.clip),
        Some([0.0; 4])
    );
}

#[test]
fn recorded_clips_replay() {
    let mut t = FakeText;
    let mut p = painter(&mut t);
    p.start_recording();
    p.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
    p.fill(Rect::new(0.0, 0.0, 50.0, 50.0), 0.0, [255; 4]);
    p.pop_clip();
    let ops = p.take_recording();
    p.replay(&ops);
    p.fill(Rect::new(0.0, 0.0, 50.0, 50.0), 0.0, [255; 4]);
    let g = &p.scene.layers[0].geometry.under;
    assert_eq!(g[1].clip, [0.0, 0.0, 10.0, 10.0]);
    assert_eq!(g[2].clip, [0.0; 4]);
}

#[test]
fn the_time_a_frame_runs_at_still_orders_press_counts() {
    // Guards the harness itself: at_ms sets absolute app time.
    let mut h = harness();
    h.at_ms(250);
    let mut t = Duration::ZERO;
    h.frame(|ui| t = ui.time());
    assert_eq!(t, Duration::from_millis(250));
}

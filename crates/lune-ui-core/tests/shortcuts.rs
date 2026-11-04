//! The shortcut registry (M5 step 3): the Acceptance list of `infra.shortcut-routing`, one test
//! each, plus scope rules, table order and the hint helper.

use lune_ui_core::input::{FocusEvent, Key, Keymap, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{Kind, Motion};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::shortcuts::{Binding, Chord, CommandId, Scope, Shortcuts};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::{Dismissal, OverlayOptions, Sense, WidgetId};

const P: PointerButton = PointerButton::Primary;
const FIELD: Rect = Rect::new(10.0, 10.0, 100.0, 30.0);
const CMD: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    logo: true,
};
const CTRL: Modifiers = Modifiers {
    shift: false,
    control: true,
    alt: false,
    logo: false,
};

fn ch(s: &str) -> Key {
    Key::Character(s.to_owned())
}

fn named(k: NamedKey) -> Key {
    Key::Named(k)
}

fn cmd(s: &'static str) -> CommandId {
    CommandId::new(s)
}

fn global(chord: Chord, command: &'static str) -> Binding {
    Binding::new(Scope::Global, chord, command)
}

/// What a test field does with the frame's input.
#[derive(Clone, Copy, PartialEq)]
enum Field {
    /// Registers and reads nothing.
    Declines,
    /// Reads its events and leaves them.
    Reads,
    /// Takes every event, as a text input does.
    TakesAll,
}

struct Seen {
    escape: bool,
    dismissed: Option<Dismissal>,
}

/// A page with one focusable field `f`, optionally a modal dialog `dlg` over it.
fn frame(h: &mut UiHarness, field: Field, dialog: bool) -> (Vec<CommandId>, Seen, Repaint) {
    let mut seen = Seen {
        escape: false,
        dismissed: None,
    };
    let (_, out) = h.frame(|ui| {
        let f = ui.id("f");
        ui.interact(f, FIELD, Sense::CLICK_FOCUS);
        if dialog {
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
            let (of, ()) = ui.overlay(
                "dlg",
                &opts,
                |_, _, _| {},
                |ui, _| {
                    let id = ui.id("ok");
                    ui.interact(id, Rect::new(150.0, 150.0, 60.0, 24.0), Sense::CLICK_FOCUS);
                    for (i, e) in ui.focus_events(id) {
                        if field == Field::Declines {
                            break;
                        }
                        if field == Field::TakesAll {
                            ui.consume_focus_event(i);
                        }
                        seen.escape |= matches!(
                            e,
                            FocusEvent::Key {
                                key: Key::Named(NamedKey::Escape),
                                ..
                            }
                        );
                    }
                },
            );
            seen.dismissed = of.dismissed;
        }
        for (i, e) in ui.focus_events(f) {
            if field == Field::Declines {
                break;
            }
            seen.escape |= matches!(
                e,
                FocusEvent::Key {
                    key: Key::Named(NamedKey::Escape),
                    ..
                }
            );
            if field == Field::TakesAll {
                ui.consume_focus_event(i);
            }
        }
    });
    (out.fired, seen, out.repaint)
}

fn focused(h: &mut UiHarness) {
    frame(h, Field::Declines, false);
    h.input.pointer_to(20.0, 20.0).click(P);
    frame(h, Field::Declines, false);
    frame(h, Field::Declines, false);
}

fn press(h: &mut UiHarness, key: Key, mods: Modifiers) {
    h.input.set_modifiers(mods).key_down(key, None);
    h.input.set_modifiers(Modifiers::default());
}

fn fired_after(h: &mut UiHarness, key: Key, mods: Modifiers) -> Vec<CommandId> {
    press(h, key, mods);
    frame(h, Field::Declines, false).0
}

#[test]
fn the_primary_flag_picks_command_or_control_both_ways() {
    for (keymap, yes, no) in [(Keymap::MAC, CMD, CTRL), (Keymap::PC, CTRL, CMD)] {
        let mut h = UiHarness::new((400.0, 300.0));
        h.keymap = keymap;
        h.shortcuts
            .add(global(Chord::key(ch("k")).primary(), "open-search"));
        focused(&mut h);
        assert_eq!(fired_after(&mut h, ch("k"), yes), [cmd("open-search")]);
        assert!(fired_after(&mut h, ch("k"), no).is_empty());
        assert!(
            fired_after(&mut h, ch("k"), Modifiers::default()).is_empty(),
            "exact modifiers"
        );
        // Character keys compare case-insensitively.
        assert_eq!(fired_after(&mut h, ch("K"), yes), [cmd("open-search")]);
    }
}

#[test]
fn a_disabled_binding_never_fires_and_consumes_nothing() {
    let mut h = UiHarness::new((400.0, 300.0));
    h.shortcuts
        .add(global(Chord::key(ch("k")).primary(), "open-search").enabled(false));
    h.shortcuts
        .add(global(Chord::key(ch("k")).primary(), "second"));
    focused(&mut h);
    // The disabled one is skipped; the next enabled binding of the chord fires.
    assert_eq!(fired_after(&mut h, ch("k"), CMD), [cmd("second")]);
    h.shortcuts.set_enabled(&cmd("second"), false);
    press(&mut h, ch("k"), CMD);
    let mut got = Vec::new();
    h.frame(|ui| {
        let f = ui.id("f");
        ui.interact(f, FIELD, Sense::CLICK_FOCUS);
        got = ui.focus_events(f);
    });
    assert_eq!(got.len(), 1, "the press stayed for the focused widget");
    h.shortcuts.set_enabled(&cmd("open-search"), true);
    assert_eq!(fired_after(&mut h, ch("k"), CMD), [cmd("open-search")]);
}

#[test]
fn escape_with_a_dialog_fires_the_overlay_binding_and_the_field_never_sees_it() {
    let mut h = UiHarness::new((400.0, 300.0));
    let dlg = WidgetId::ROOT.with("dlg");
    h.shortcuts.add(
        Binding::new(
            Scope::Overlay(dlg),
            Chord::key(named(NamedKey::Escape)),
            "close-dialog",
        )
        .preempt(),
    );
    focused(&mut h);
    frame(&mut h, Field::Declines, true);
    frame(&mut h, Field::Declines, true);
    press(&mut h, named(NamedKey::Escape), Modifiers::default());
    let (fired, seen, _) = frame(&mut h, Field::Declines, true);
    assert_eq!(fired, [cmd("close-dialog")]);
    assert!(!seen.escape, "nobody saw the Escape");
    assert_eq!(seen.dismissed, None, "and the dialog was not dismissed");
}

#[test]
fn escape_with_no_dialog_reaches_the_field_first() {
    let mut h = UiHarness::new((400.0, 300.0));
    let dlg = WidgetId::ROOT.with("dlg");
    h.shortcuts.add(
        Binding::new(
            Scope::Overlay(dlg),
            Chord::key(named(NamedKey::Escape)),
            "close-dialog",
        )
        .preempt(),
    );
    h.shortcuts
        .add(global(Chord::key(named(NamedKey::Escape)), "clear"));
    focused(&mut h);
    // The field takes Escape: neither binding fires.
    press(&mut h, named(NamedKey::Escape), Modifiers::default());
    let (fired, seen, _) = frame(&mut h, Field::TakesAll, false);
    assert!(seen.escape && fired.is_empty());
    // It reads but declines: it saw the Escape first, then the press bubbles to the global
    // binding (the overlay one is not live without the dialog).
    press(&mut h, named(NamedKey::Escape), Modifiers::default());
    let (fired, seen, _) = frame(&mut h, Field::Reads, false);
    assert!(seen.escape);
    assert_eq!(fired, [cmd("clear")]);
}

fn within_page(h: &mut UiHarness, take: bool) -> Vec<CommandId> {
    let (_, out) = h.frame(|ui| {
        ui.scope("page", |ui| {
            let f = ui.id("f");
            ui.interact(f, FIELD, Sense::CLICK_FOCUS);
            if take {
                for (i, _) in ui.focus_events(f) {
                    ui.consume_focus_event(i);
                }
            }
        });
    });
    out.fired
}

#[test]
fn bubble_fires_when_the_focused_widget_declines_and_not_when_it_consumed() {
    let mut h = UiHarness::new((400.0, 300.0));
    let page = WidgetId::ROOT.with("page");
    h.shortcuts.add(Binding::new(
        Scope::Within(page),
        Chord::key(ch("c")).primary(),
        "copy-diff",
    ));
    within_page(&mut h, false);
    h.input.pointer_to(20.0, 20.0).click(P);
    within_page(&mut h, false);
    within_page(&mut h, false);
    // Declined (read but not taken): fires, from the ancestor scope.
    press(&mut h, ch("c"), CMD);
    let mut saw = false;
    let (_, out) = h.frame(|ui| {
        ui.scope("page", |ui| {
            let f = ui.id("f");
            ui.interact(f, FIELD, Sense::CLICK_FOCUS);
            saw = !ui.focus_events(f).is_empty();
        });
    });
    assert!(saw);
    assert_eq!(out.fired, [cmd("copy-diff")]);
    // Consumed by a text input: does not fire.
    press(&mut h, ch("c"), CMD);
    assert!(within_page(&mut h, true).is_empty());
}

#[test]
fn within_the_focused_widget_itself_and_not_an_unrelated_scope() {
    let mut h = UiHarness::new((400.0, 300.0));
    let f = WidgetId::ROOT.with("page").with("f");
    let other = WidgetId::ROOT.with("other");
    h.shortcuts.add(Binding::new(
        Scope::Within(other),
        Chord::key(ch("x")),
        "nope",
    ));
    h.shortcuts.add(Binding::new(
        Scope::Within(f),
        Chord::key(ch("x")),
        "on-widget",
    ));
    within_page(&mut h, false);
    h.input.pointer_to(20.0, 20.0).click(P);
    within_page(&mut h, false);
    within_page(&mut h, false);
    press(&mut h, ch("x"), Modifiers::default());
    assert_eq!(within_page(&mut h, false), [cmd("on-widget")]);
}

#[test]
fn the_innermost_scope_wins_and_a_press_fires_one_binding() {
    let mut h = UiHarness::new((400.0, 300.0));
    let page = WidgetId::ROOT.with("page");
    let f = page.with("f");
    h.shortcuts.add(global(Chord::key(ch("x")), "global"));
    h.shortcuts.add(Binding::new(
        Scope::Within(page),
        Chord::key(ch("x")),
        "page",
    ));
    h.shortcuts.add(Binding::new(
        Scope::Within(f),
        Chord::key(ch("x")),
        "widget",
    ));
    h.shortcuts.add(Binding::new(
        Scope::Within(f),
        Chord::key(ch("x")),
        "widget-later",
    ));
    within_page(&mut h, false);
    h.input.pointer_to(20.0, 20.0).click(P);
    within_page(&mut h, false);
    within_page(&mut h, false);
    press(&mut h, ch("x"), Modifiers::default());
    assert_eq!(
        within_page(&mut h, false),
        [cmd("widget")],
        "table order, one binding"
    );
}

#[test]
fn a_modal_hides_page_and_global_bindings_that_do_not_preempt() {
    let mut h = UiHarness::new((400.0, 300.0));
    h.shortcuts.add(global(Chord::key(ch("a")), "plain"));
    h.shortcuts
        .add(global(Chord::key(ch("b")), "pre").preempt());
    focused(&mut h);
    frame(&mut h, Field::Declines, true);
    frame(&mut h, Field::Declines, true);
    press(&mut h, ch("a"), Modifiers::default());
    assert!(frame(&mut h, Field::Declines, true).0.is_empty());
    press(&mut h, ch("b"), Modifiers::default());
    assert_eq!(frame(&mut h, Field::Declines, true).0, [cmd("pre")]);
    // Without the dialog the plain one fires again.
    frame(&mut h, Field::Declines, false);
    frame(&mut h, Field::Declines, false);
    press(&mut h, ch("a"), Modifiers::default());
    assert_eq!(frame(&mut h, Field::Declines, false).0, [cmd("plain")]);
}

#[test]
fn an_overlay_binding_fires_for_a_press_the_dialog_declined() {
    let mut h = UiHarness::new((400.0, 300.0));
    let dlg = WidgetId::ROOT.with("dlg");
    h.shortcuts.add(Binding::new(
        Scope::Overlay(dlg),
        Chord::key(ch("d")),
        "in-dialog",
    ));
    h.shortcuts.add(global(Chord::key(ch("d")), "global-d"));
    focused(&mut h);
    frame(&mut h, Field::Declines, true);
    frame(&mut h, Field::Declines, true);
    press(&mut h, ch("d"), Modifiers::default());
    assert_eq!(
        frame(&mut h, Field::Declines, true).0,
        [cmd("in-dialog")],
        "the dialog's own binding, and the hidden global one stays quiet"
    );
}

#[test]
fn ctrl_or_logo_enter_fires_for_both() {
    let mut h = UiHarness::new((400.0, 300.0));
    h.shortcuts.add(global(
        Chord::key(named(NamedKey::Enter)).ctrl_or_logo(),
        "accept",
    ));
    focused(&mut h);
    assert_eq!(
        fired_after(&mut h, named(NamedKey::Enter), CTRL),
        [cmd("accept")]
    );
    assert_eq!(
        fired_after(&mut h, named(NamedKey::Enter), CMD),
        [cmd("accept")]
    );
    assert!(fired_after(&mut h, named(NamedKey::Enter), Modifiers::default()).is_empty());
    let both = Modifiers {
        control: true,
        logo: true,
        ..Modifiers::default()
    };
    assert!(fired_after(&mut h, named(NamedKey::Enter), both).is_empty());
}

#[test]
fn a_binding_without_repeat_fires_once_for_a_held_key() {
    let mut h = UiHarness::new((400.0, 300.0));
    h.shortcuts.add(global(Chord::key(ch("j")), "once"));
    h.shortcuts
        .add(global(Chord::key(ch("l")), "many").repeating());
    focused(&mut h);
    h.input.key_down(ch("j"), None);
    assert_eq!(frame(&mut h, Field::Declines, false).0, [cmd("once")]);
    h.input.key_repeat(ch("j"));
    h.input.key_repeat(ch("j"));
    assert!(frame(&mut h, Field::Declines, false).0.is_empty());
    h.input.key_down(ch("l"), None);
    h.input.key_repeat(ch("l"));
    h.input.key_repeat(ch("l"));
    assert_eq!(
        frame(&mut h, Field::Declines, false).0,
        [cmd("many"), cmd("many"), cmd("many")]
    );
}

#[test]
fn nothing_fires_while_the_focused_widget_composes() {
    let mut h = UiHarness::new((400.0, 300.0));
    let page = WidgetId::ROOT.with("page");
    h.shortcuts.add(global(Chord::key(ch("k")).primary(), "g"));
    h.shortcuts
        .add(global(Chord::key(ch("m")), "pre").preempt());
    h.shortcuts
        .add(Binding::new(Scope::Within(page), Chord::key(ch("n")), "w"));
    let run = |h: &mut UiHarness, composing: bool| {
        h.frame(|ui| {
            ui.scope("page", |ui| {
                let f = ui.id("f");
                ui.interact(f, FIELD, Sense::CLICK_FOCUS);
                ui.set_composing(f, composing);
            });
        })
        .1
        .fired
    };
    run(&mut h, false);
    h.input.pointer_to(20.0, 20.0).click(P);
    run(&mut h, false);
    run(&mut h, true);
    for (key, mods) in [
        (ch("k"), CMD),
        (ch("m"), Modifiers::default()),
        (ch("n"), Modifiers::default()),
    ] {
        press(&mut h, key, mods);
        assert!(run(&mut h, true).is_empty());
    }
    run(&mut h, false);
    press(&mut h, ch("k"), CMD);
    assert_eq!(run(&mut h, false), [cmd("g")]);
}

#[test]
fn a_fired_command_makes_the_frame_repaint_now() {
    let mut h = UiHarness::new((400.0, 300.0));
    h.shortcuts.add(global(Chord::key(ch("j")), "x"));
    h.shortcuts.add(global(Chord::key(ch("m")), "y").preempt());
    focused(&mut h);
    let (_, _, idle) = frame(&mut h, Field::Declines, false);
    assert_eq!(idle, Repaint::Idle);
    for k in ["j", "m"] {
        h.input.key_down(ch(k), None);
        let (fired, _, repaint) = frame(&mut h, Field::Declines, false);
        assert_eq!(fired.len(), 1);
        assert_eq!(repaint, Repaint::Now, "{k}");
    }
}

#[test]
fn a_preempted_press_leaves_no_text_behind() {
    let mut h = UiHarness::new((400.0, 300.0));
    h.shortcuts.add(global(Chord::key(ch("m")), "y").preempt());
    focused(&mut h);
    h.input.key_down(ch("m"), Some("m"));
    let mut events = Vec::new();
    let (_, out) = h.frame(|ui| {
        let f = ui.id("f");
        ui.interact(f, FIELD, Sense::CLICK_FOCUS);
        events = ui.focus_events(f);
    });
    assert_eq!(out.fired, [cmd("y")]);
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn a_press_the_field_took_is_not_offered_to_another_scope() {
    let mut h = UiHarness::new((400.0, 300.0));
    h.shortcuts.add(global(Chord::key(ch("a")), "plain-a"));
    focused(&mut h);
    // A printable key typed into a text input never reaches an unmodified binding.
    h.input.key_down(ch("a"), Some("a"));
    let (fired, _, _) = frame(&mut h, Field::TakesAll, false);
    assert!(fired.is_empty());
    // An unbound chord falls through to the focused widget.
    h.input.set_modifiers(CMD).key_down(ch("z"), Some("z"));
    h.input.set_modifiers(Modifiers::default());
    let mut n = 0;
    h.frame(|ui| {
        let f = ui.id("f");
        ui.interact(f, FIELD, Sense::CLICK_FOCUS);
        n = ui.focus_events(f).len();
    });
    assert_eq!(n, 1);
}

#[test]
fn hints_use_the_fixed_order_and_the_keymap() {
    let n = Chord::key(ch("n")).primary();
    assert_eq!(n.hint(&Keymap::MAC), "Cmd+N");
    assert_eq!(n.hint(&Keymap::PC), "Ctrl+N");
    assert_eq!(Chord::key(ch(",")).primary().hint(&Keymap::MAC), "Cmd+,");
    let all = Chord::key(ch("p")).shift().alt().ctrl().logo();
    assert_eq!(all.hint(&Keymap::PC), "Cmd+Ctrl+Alt+Shift+P");
    assert_eq!(
        Chord::key(named(NamedKey::Enter))
            .ctrl_or_logo()
            .hint(&Keymap::MAC),
        "Cmd+Enter"
    );
    assert_eq!(Chord::key(named(NamedKey::Escape)).hint(&Keymap::PC), "Esc");
    assert_eq!(
        Chord::key(named(NamedKey::ArrowLeft))
            .primary()
            .shift()
            .hint(&Keymap::MAC),
        "Cmd+Shift+Left"
    );
}

#[test]
fn the_table_clears_and_starts_empty() {
    let mut s = Shortcuts::default();
    assert!(s.bindings().is_empty());
    s.add(global(Chord::key(ch("a")), "a"))
        .add(global(Chord::key(ch("b")), "b"));
    assert_eq!(s.bindings().len(), 2);
    s.clear();
    assert!(s.bindings().is_empty());
    let id: CommandId = String::from("dyn").into();
    assert_eq!(id.as_str(), "dyn");
}

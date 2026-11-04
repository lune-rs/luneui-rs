//! Focus (C4, core.focus acceptance): Tab order and wrap, disabled stops, focus-visible, window
//! blur, modal trap, restore on close, the vanish rule and keep-alive, request_focus, initial
//! focus, key consumption and focus-loss dismissal.

use lune_ui_core::input::{Key, Modifiers, NamedKey, PointerButton};
use lune_ui_core::motion::{Kind, Motion};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::{Dismissal, OverlayOptions, Response, Sense, Ui, WidgetId};

const P: PointerButton = PointerButton::Primary;

fn harness() -> UiHarness {
    UiHarness::new((400.0, 300.0))
}

fn rect(i: usize) -> Rect {
    Rect::new(10.0 + i as f32 * 60.0, 10.0, 50.0, 30.0)
}

fn stop(ui: &mut Ui<'_>, key: &str, i: usize) -> Response {
    let id = ui.id(key);
    ui.interact(id, rect(i), Sense::CLICK_FOCUS)
}

fn tab(h: &mut UiHarness) {
    h.input.key_down(Key::Named(NamedKey::Tab), None);
}

fn shift_tab(h: &mut UiHarness) {
    h.input.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.input.set_modifiers(Modifiers::default());
}

/// A, B, (disabled D), C; returns the focused key and whether its ring shows.
fn abc(h: &mut UiHarness) -> (Option<&'static str>, bool) {
    let mut out = (None, false);
    h.frame(|ui| {
        let mut rs = vec![("a", stop(ui, "a", 0)), ("b", stop(ui, "b", 1))];
        rs.push(("d", ui.with_enabled(false, |ui| stop(ui, "d", 2))));
        rs.push(("c", stop(ui, "c", 3)));
        for (k, r) in rs {
            if r.focused {
                assert!(out.0.is_none(), "one focused widget");
                out = (Some(k), r.focus_visible);
            }
        }
    });
    out
}

#[test]
fn tab_goes_a_b_c_a_and_shift_tab_reverses_skipping_disabled() {
    let mut h = harness();
    abc(&mut h);
    let mut order = Vec::new();
    for _ in 0..4 {
        tab(&mut h);
        order.push(abc(&mut h).0);
    }
    assert_eq!(order, [Some("a"), Some("b"), Some("c"), Some("a")]);
    let mut back = Vec::new();
    for _ in 0..4 {
        shift_tab(&mut h);
        back.push(abc(&mut h).0);
    }
    assert_eq!(back, [Some("c"), Some("b"), Some("a"), Some("c")]);
}

#[test]
fn focus_visible_after_tab_off_after_a_pointer_press_and_hidden_on_window_blur() {
    let mut h = harness();
    abc(&mut h);
    tab(&mut h);
    assert_eq!(abc(&mut h), (Some("a"), true));
    h.input.pointer_to(80.0, 20.0).click(P); // B
    assert_eq!(abc(&mut h), (Some("b"), false));
    tab(&mut h);
    assert_eq!(abc(&mut h), (Some("c"), true));
    h.input.focus(false);
    assert_eq!(
        abc(&mut h),
        (Some("c"), false),
        "blur hides the ring, keeps focus"
    );
    h.input.focus(true);
    assert_eq!(abc(&mut h), (Some("c"), true));
}

#[test]
fn a_press_on_a_non_focusable_widget_leaves_focus_alone() {
    let mut h = harness();
    let run = |h: &mut UiHarness| {
        let mut f = false;
        h.frame(|ui| {
            f = stop(ui, "a", 0).focused;
            let id = ui.id("plain");
            ui.interact(id, rect(4), Sense::CLICK);
        });
        f
    };
    run(&mut h);
    tab(&mut h);
    assert!(run(&mut h));
    h.input.pointer_to(rect(4).x + 5.0, 20.0).click(P);
    assert!(run(&mut h));
}

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
        close: Motion::of(Kind::Dismiss),
    }
}

/// The page (A, B) and, when `open`, a modal with X, Y. Returns the focused key.
fn page_and_modal(h: &mut UiHarness, open: bool, opts: &OverlayOptions) -> Option<&'static str> {
    let mut focused = None;
    h.frame(|ui| {
        ui.painter().on(LayerId::Page);
        for (k, i) in [("a", 0), ("b", 1)] {
            if stop(ui, k, i).focused {
                focused = Some(k);
            }
        }
        if open {
            ui.overlay(
                "modal",
                opts,
                |_, _, _| {},
                |ui, _| {
                    for (k, i) in [("x", 2), ("y", 3)] {
                        if stop(ui, k, i).focused {
                            assert!(focused.is_none());
                            focused = Some(k);
                        }
                    }
                },
            );
        }
    });
    focused
}

#[test]
fn tab_inside_a_modal_never_leaves_it() {
    let mut h = harness();
    let o = modal();
    page_and_modal(&mut h, false, &o);
    tab(&mut h);
    assert_eq!(page_and_modal(&mut h, false, &o), Some("a"));
    page_and_modal(&mut h, true, &o); // opens: focus moves to its first stop
    assert_eq!(page_and_modal(&mut h, true, &o), Some("x"));
    for i in 0..20 {
        tab(&mut h);
        let f = page_and_modal(&mut h, true, &o);
        assert!(matches!(f, Some("x" | "y")), "tab {i}: {f:?}");
    }
    shift_tab(&mut h);
    assert!(matches!(page_and_modal(&mut h, true, &o), Some("x" | "y")));
}

#[test]
fn closing_restores_the_previous_focus() {
    let mut h = harness();
    let o = modal();
    page_and_modal(&mut h, false, &o);
    tab(&mut h);
    tab(&mut h);
    assert_eq!(page_and_modal(&mut h, false, &o), Some("b"));
    page_and_modal(&mut h, true, &o);
    tab(&mut h);
    assert_eq!(page_and_modal(&mut h, true, &o), Some("y"));
    page_and_modal(&mut h, false, &o); // closes: focus back at once
    assert_eq!(page_and_modal(&mut h, false, &o), Some("b"));
}

#[test]
fn closing_restores_the_anchor_when_the_previous_widget_is_gone() {
    let mut h = harness();
    let anchor = WidgetId::ROOT.with("a");
    let o = OverlayOptions {
        anchor: Some(anchor),
        ..modal()
    };
    // Focus a lone widget, open the modal, then the widget goes away with the modal open.
    h.frame(|ui| {
        stop(ui, "a", 0);
        stop(ui, "gone", 1);
    });
    h.input.pointer_to(rect(1).x + 5.0, 20.0).click(P);
    h.frame(|ui| {
        stop(ui, "a", 0);
        assert!(stop(ui, "gone", 1).focused);
    });
    let run = |h: &mut UiHarness, open: bool| {
        let mut a = false;
        h.frame(|ui| {
            a = stop(ui, "a", 0).focused;
            if open {
                ui.overlay(
                    "modal",
                    &o,
                    |_, _, _| {},
                    |ui, _| {
                        stop(ui, "x", 2);
                    },
                );
            }
        });
        a
    };
    run(&mut h, true);
    run(&mut h, false);
    assert!(run(&mut h, false), "the anchor took focus");
}

#[test]
fn initial_focus_is_the_named_stop_and_follows_the_input_origin() {
    let mut h = harness();
    let o = OverlayOptions {
        initial_focus: Some(WidgetId::ROOT.with("y")),
        ..modal()
    };
    let mut ring = false;
    let run = |h: &mut UiHarness, ring: &mut bool| {
        let mut f = None;
        h.frame(|ui| {
            ui.overlay(
                "modal",
                &o,
                |_, _, _| {},
                |ui, _| {
                    for (k, i) in [("x", 2), ("y", 3)] {
                        let r = stop(ui, k, i);
                        if r.focused {
                            f = Some(k);
                            *ring = r.focus_visible;
                        }
                    }
                },
            );
        });
        f
    };
    // Opened after a key press: the ring shows.
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    run(&mut h, &mut ring);
    assert_eq!(run(&mut h, &mut ring), Some("y"));
    assert!(ring);
}

#[test]
fn a_vanished_focused_widget_loses_focus_after_one_frame_unless_kept_alive() {
    let mut h = harness();
    let run = |h: &mut UiHarness, draw: bool, alive: bool| {
        let mut f = false;
        h.frame(|ui| {
            if alive {
                ui.keep_alive("tab", |ui| {
                    if draw {
                        f = stop(ui, "a", 0).focused;
                    }
                });
            } else if draw {
                f = stop(ui, "a", 0).focused;
            }
        });
        f
    };
    run(&mut h, true, false);
    tab(&mut h);
    assert!(run(&mut h, true, false));
    run(&mut h, false, false);
    assert!(!run(&mut h, true, false), "lost after one undrawn frame");

    let mut h = harness();
    run(&mut h, true, true);
    tab(&mut h);
    assert!(run(&mut h, true, true));
    for _ in 0..5 {
        run(&mut h, false, true);
    }
    assert!(
        run(&mut h, true, true),
        "kept under a live keep-alive scope"
    );
}

#[test]
fn request_focus_applies_at_the_next_begin_without_a_ring() {
    let mut h = harness();
    let b = WidgetId::ROOT.with("b");
    let mut first = None;
    h.frame(|ui| {
        abc_inline(ui);
        ui.request_focus(b);
        first = Some(stop(ui, "b", 1).focused);
    });
    assert_eq!(first, Some(false));
    assert_eq!(abc(&mut h), (Some("b"), false));
}

fn abc_inline(ui: &mut Ui<'_>) {
    stop(ui, "a", 0);
}

#[test]
fn a_key_press_goes_to_the_focused_widget_once() {
    let mut h = harness();
    let run = |h: &mut UiHarness| {
        let mut got = (0, 0);
        h.frame(|ui| {
            let a = stop(ui, "a", 0);
            let b = stop(ui, "b", 1);
            for _ in 0..3 {
                got.0 += i32::from(ui.consume_key(a.id, NamedKey::Enter));
                got.1 += i32::from(ui.consume_key(b.id, NamedKey::Enter));
            }
        });
        got
    };
    run(&mut h);
    h.input.pointer_to(20.0, 20.0).click(P); // focus A by pointer
    run(&mut h);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    assert_eq!(run(&mut h), (1, 0));
    // A consumed key turns the ring on.
    let mut ring = false;
    h.frame(|ui| ring = stop(ui, "a", 0).focus_visible);
    assert!(ring);
}

#[test]
fn focus_moving_outside_or_window_blur_dismisses_a_focus_loss_overlay() {
    let o = OverlayOptions {
        layer: LayerId::Popup,
        modal: false,
        dismiss_on_focus_loss: true,
        ..modal()
    };
    let run = |h: &mut UiHarness, req: Option<WidgetId>| {
        let mut d = None;
        h.frame(|ui| {
            stop(ui, "a", 0);
            let (f, _) = ui.overlay(
                "flyout",
                &o,
                |_, _, _| {},
                |ui, _| {
                    stop(ui, "x", 2);
                },
            );
            d = f.dismissed;
            if let Some(id) = req {
                ui.request_focus(id);
            }
        });
        d
    };
    let mut h = harness();
    run(&mut h, Some(WidgetId::ROOT.with("x")));
    assert_eq!(run(&mut h, None), None, "focus inside keeps it");
    run(&mut h, Some(WidgetId::ROOT.with("a")));
    assert_eq!(run(&mut h, None), Some(Dismissal::FocusLoss));

    let mut h = harness();
    run(&mut h, None);
    h.input.focus(false);
    assert_eq!(run(&mut h, None), Some(Dismissal::FocusLoss));
}

// Focus across the overlay stack (review F1, F4; D82, D83): opening or closing an overlay never
// moves focus out of a still-open overlay above it, never lands it under an open modal, and
// closing restores along the stack.

#[derive(Debug, Clone, Copy, PartialEq)]
enum Ov {
    /// A light-dismiss popup with one focusable item.
    Menu,
    /// A second popup (a submenu) with one focusable item.
    Sub,
    /// A modal with one focusable OK.
    Dialog,
    /// A Toast-layer overlay with only a hover region and no dismissal: passive.
    Toast,
}

fn ov_opts(o: Ov) -> OverlayOptions {
    let popup = OverlayOptions {
        layer: LayerId::Popup,
        modal: false,
        dismiss_on_outside_press: true,
        ..modal()
    };
    match o {
        Ov::Menu | Ov::Sub => popup,
        Ov::Dialog => modal(),
        Ov::Toast => OverlayOptions {
            layer: LayerId::Toast,
            dismiss_on_escape: false,
            dismiss_on_outside_press: false,
            ..popup
        },
    }
}

/// The page button and the overlays `ovs` in call order. Returns the focused key and the
/// dismissals reported.
fn stack(h: &mut UiHarness, ovs: &[Ov]) -> (Option<&'static str>, Vec<(Ov, Dismissal)>) {
    let mut focused = None;
    let mut dismissed = Vec::new();
    h.frame(|ui| {
        ui.painter().on(LayerId::Page);
        if stop(ui, "page", 0).focused {
            focused = Some("page");
        }
        for &o in ovs {
            let key = format!("{o:?}");
            let (f, _) = ui.overlay(
                key.as_str(),
                &ov_opts(o),
                |_, _, _| {},
                |ui, _| {
                    let (k, i, sense) = match o {
                        Ov::Menu => ("item", 1, Sense::CLICK_FOCUS),
                        Ov::Sub => ("sub-item", 3, Sense::CLICK_FOCUS),
                        Ov::Dialog => ("ok", 2, Sense::CLICK_FOCUS),
                        Ov::Toast => ("toast", 5, Sense::HOVER),
                    };
                    let id = ui.id(k);
                    if ui.interact(id, rect(i), sense).focused {
                        assert!(focused.is_none(), "one focused widget");
                        focused = Some(k);
                    }
                },
            );
            if let Some(d) = f.dismissed {
                dismissed.push((o, d));
            }
        }
    });
    (focused, dismissed)
}

/// Focuses the page button with Tab.
fn page_focused() -> UiHarness {
    let mut h = harness();
    stack(&mut h, &[]);
    tab(&mut h);
    assert_eq!(stack(&mut h, &[]).0, Some("page"));
    h
}

#[test]
fn a_menu_that_stops_under_the_dialog_it_opened_leaves_focus_in_the_dialog() {
    use Ov::{Dialog, Menu};
    let mut h = page_focused();
    stack(&mut h, &[Menu]);
    assert_eq!(stack(&mut h, &[Menu]).0, Some("item"));
    // The item is picked: the app opens the dialog while the menu is still called.
    stack(&mut h, &[Menu, Dialog]);
    assert_eq!(stack(&mut h, &[Menu, Dialog]).0, Some("ok"));
    // The app stops calling the menu.
    assert_eq!(stack(&mut h, &[Dialog]).0, Some("ok"));
    assert_eq!(
        stack(&mut h, &[Dialog]).0,
        Some("ok"),
        "focus stays in the modal"
    );
    // Tab stays trapped in the dialog.
    tab(&mut h);
    assert_eq!(stack(&mut h, &[Dialog]).0, Some("ok"));
    // The dialog closes after the menu already closed: focus returns to the page button the
    // menu had saved (the dialog inherited it).
    stack(&mut h, &[]);
    assert_eq!(stack(&mut h, &[]).0, Some("page"));
}

#[test]
fn a_toast_opened_before_a_dialog_closes_without_moving_focus() {
    use Ov::{Dialog, Toast};
    let mut h = page_focused();
    // A passive toast takes no focus when it opens (D82).
    stack(&mut h, &[Toast]);
    assert_eq!(stack(&mut h, &[Toast]).0, Some("page"));
    stack(&mut h, &[Toast, Dialog]);
    assert_eq!(stack(&mut h, &[Toast, Dialog]).0, Some("ok"));
    // The toast expires under the open dialog.
    stack(&mut h, &[Dialog]);
    assert_eq!(stack(&mut h, &[Dialog]).0, Some("ok"));
    // The dialog closes: back to the page button, which the toast had kept.
    stack(&mut h, &[]);
    assert_eq!(stack(&mut h, &[]).0, Some("page"));
}

#[test]
fn a_toast_over_an_open_dialog_takes_neither_focus_nor_escape() {
    use Ov::{Dialog, Toast};
    let mut h = page_focused();
    stack(&mut h, &[Dialog]);
    assert_eq!(stack(&mut h, &[Dialog]).0, Some("ok"));
    stack(&mut h, &[Dialog, Toast]);
    assert_eq!(stack(&mut h, &[Dialog, Toast]).0, Some("ok"));
    // Escape goes past the passive toast to the dialog.
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (f, d) = stack(&mut h, &[Dialog, Toast]);
    assert_eq!(f, Some("ok"));
    assert_eq!(d, [(Dialog, Dismissal::Escape)]);
    // The toast closes over the dialog: focus stays on OK.
    stack(&mut h, &[Dialog]);
    assert_eq!(stack(&mut h, &[Dialog]).0, Some("ok"));
}

#[test]
fn a_toast_opening_keeps_a_click_focused_page_widget_focused() {
    let mut h = harness();
    stack(&mut h, &[]);
    h.input.pointer_to(20.0, 20.0).click(P);
    assert_eq!(stack(&mut h, &[]).0, Some("page"));
    stack(&mut h, &[Ov::Toast]);
    assert_eq!(stack(&mut h, &[Ov::Toast]).0, Some("page"));
    // An outside press goes past the passive toast too: it is a plain press on the page.
    h.input.pointer_to(20.0, 20.0).click(P);
    let (f, d) = stack(&mut h, &[Ov::Toast]);
    assert_eq!((f, d), (Some("page"), vec![]));
}

#[test]
fn a_menu_and_its_submenu_closing_in_one_frame_restore_the_page() {
    use Ov::{Menu, Sub};
    let mut h = page_focused();
    stack(&mut h, &[Menu]);
    assert_eq!(stack(&mut h, &[Menu]).0, Some("item"));
    stack(&mut h, &[Menu, Sub]);
    assert_eq!(stack(&mut h, &[Menu, Sub]).0, Some("sub-item"));
    stack(&mut h, &[]);
    assert_eq!(stack(&mut h, &[]).0, Some("page"));
}

#[test]
fn closing_never_restores_focus_under_an_open_modal() {
    use Ov::{Dialog, Menu};
    // The menu holds focus and a modal opens over it; a press on the menu item (now under the
    // modal) cannot pull focus back out, and when the menu stops, its saved page button is
    // under the modal, so focus never goes there.
    let mut h = page_focused();
    stack(&mut h, &[Menu]);
    assert_eq!(stack(&mut h, &[Menu]).0, Some("item"));
    stack(&mut h, &[Menu, Dialog]);
    // Focus is moved back into the menu by the app, then the menu stops.
    h.input.pointer_to(rect(1).x + 5.0, 20.0).click(P);
    let (f, _) = stack(&mut h, &[Menu, Dialog]);
    assert_ne!(f, Some("page"));
    stack(&mut h, &[Dialog]);
    let (f, _) = stack(&mut h, &[Dialog]);
    assert_ne!(
        f,
        Some("page"),
        "a page widget never takes focus under the modal"
    );
}

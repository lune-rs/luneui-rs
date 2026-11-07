//! Focus (C4, core.focus acceptance): Tab order and wrap, disabled stops, focus-visible, window
//! blur, modal trap, restore on close, the vanish rule and keep-alive, request_focus, initial
//! focus, key consumption and focus-loss dismissal.

use lune_ui_core::access::{AccessAction, Action, Actions, Node, Role, State, States};
use lune_ui_core::input::{Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::MotionTier;
use lune_ui_core::motion::{Kind, Motion};
use lune_ui_core::paint::{LayerId, Rect};
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::{Dismissal, OverlayOptions, Response, RoveAxis, Sense, Ui, WidgetId};

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

/// A non-modal Popup flyout with `dismiss_on_focus_loss`, stop `x` inside, page stop `a`
/// outside; the app stops calling the flyout once it reports a dismissal. `request_focus(req)`
/// every frame. Returns per frame `(repaint, dismissed, x focused, a focused)`.
fn focus_loss_flyout_requests(req: &str, frames: u64) -> Vec<(Repaint, bool, bool, bool)> {
    let o = OverlayOptions {
        layer: LayerId::Popup,
        modal: false,
        dismiss_on_focus_loss: true,
        ..modal()
    };
    let mut h = harness();
    h.tier = MotionTier::Off;
    let mut open = true;
    let mut seq = Vec::new();
    for i in 0..frames {
        let (mut dismissed, mut x, mut a) = (false, false, false);
        let (_, out) = h.at_ms(i * 10).frame(|ui| {
            a = stop(ui, "a", 0).focused;
            if open {
                let (f, xr) = ui.overlay("flyout", &o, |_, _, _| {}, |ui, _| stop(ui, "x", 2));
                x = xr.focused;
                dismissed = f.dismissed.is_some();
            }
            ui.request_focus(WidgetId::ROOT.with(req));
        });
        open &= !dismissed;
        seq.push((out.repaint, dismissed, x, a));
    }
    seq
}

#[test]
fn a_focus_request_held_inside_a_focus_loss_overlay_settles_to_idle() {
    // Review F13: keeping a flyout's field focused every frame neither dismisses the flyout nor
    // keeps the window redrawing; begin's own containment test decides.
    let seq = focus_loss_flyout_requests("x", 8);
    assert_eq!(
        seq[0].0,
        Repaint::Now,
        "the first request applies next frame"
    );
    assert!(
        seq[1..]
            .iter()
            .all(|&s| s == (Repaint::Idle, false, true, false)),
        "{seq:?}"
    );
}

#[test]
fn a_focus_request_outside_a_focus_loss_overlay_dismisses_it_then_settles() {
    // Review F13, the other side: page stop `a` is requested every frame and already focused when
    // a passive focus-loss overlay (a hover region, no stop) opens on frame 2. Nothing else acts
    // that frame, so only the containment test can ask for the frame whose begin dismisses it.
    let o = OverlayOptions {
        layer: LayerId::Popup,
        modal: false,
        dismiss_on_focus_loss: true,
        ..modal()
    };
    let mut h = harness();
    h.tier = MotionTier::Off;
    let mut open = false;
    let mut seq = Vec::new();
    for i in 0..8u64 {
        let (mut dismissed, mut a) = (false, false);
        let (_, out) = h.at_ms(i * 10).frame(|ui| {
            a = stop(ui, "a", 0).focused;
            if open {
                let (f, _) = ui.overlay(
                    "flyout",
                    &o,
                    |_, _, _| {},
                    |ui, _| {
                        let id = ui.id("tip");
                        ui.interact(id, rect(2), Sense::HOVER);
                    },
                );
                dismissed = f.dismissed.is_some();
            }
            ui.request_focus(WidgetId::ROOT.with("a"));
        });
        seq.push((out.repaint, dismissed, a));
        open = (i == 1 || open) && !dismissed;
    }
    assert_eq!(
        seq[1],
        (Repaint::Idle, false, true),
        "settled before: {seq:?}"
    );
    assert_eq!(
        seq[2],
        (Repaint::Now, false, true),
        "begin would dismiss: {seq:?}"
    );
    // Frame 3 still calls the overlay (the app learns of the dismissal there), so the next
    // begin dismisses it again: frame 3 asks for it, and frame 4 reports that begin's dismissal
    // as acted input (D81) while the app no longer calls the overlay. Idle from frame 5.
    assert_eq!(seq[3], (Repaint::Now, true, true), "dismissed: {seq:?}");
    assert_eq!(seq[4], (Repaint::Now, false, true), "{seq:?}");
    assert!(
        seq[5..].iter().all(|&s| s == (Repaint::Idle, false, true)),
        "{seq:?}"
    );
}

// Roving focus: a three-child composite between two buttons, its children `c0` to `c2`.

const CHILDREN: [&str; 3] = ["c0", "c1", "c2"];

#[derive(Debug, Clone)]
struct Rove {
    active: Option<usize>,
    enabled: [bool; 3],
    shown: [bool; 3],
    axis: RoveAxis,
    wrap: bool,
    /// An overlay with the stops `x` and `y` opens over the page.
    overlay: Option<OverlayOptions>,
    /// The app calls `request_focus` on this child.
    ask: Option<&'static str>,
}

fn rove(active: usize) -> Rove {
    Rove {
        active: Some(active),
        enabled: [true; 3],
        shown: [true; 3],
        axis: RoveAxis::Vertical,
        wrap: false,
        overlay: None,
        ask: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Seen {
    focused: Option<&'static str>,
    ring: bool,
    /// The composite's stop, as an index of its listed children.
    stop: Option<usize>,
    moved: Option<usize>,
    repaint_now: bool,
}

fn note(seen: &mut Seen, key: &'static str, r: &Response) {
    if r.focused {
        assert!(seen.focused.is_none(), "one focused widget");
        seen.focused = Some(key);
        seen.ring = r.focus_visible;
    }
}

fn labelled(ui: &mut Ui<'_>, id: WidgetId, at: Rect) {
    let mut states = States::default();
    states.insert(State::Focusable);
    ui.node(Node {
        id,
        role: Role::Button,
        name: "n".to_owned(),
        description: None,
        value: None,
        text: None,
        bounds: at,
        parent: None,
        children: Vec::new(),
        live: None,
        states,
        actions: Actions::default(),
        sort: None,
        level: None,
    });
}

fn rove_frame(h: &mut UiHarness, c: &Rove) -> Seen {
    let mut seen = Seen::default();
    let (_, out) = h.frame(|ui| {
        ui.painter().on(LayerId::Page);
        let r = stop(ui, "before", 0);
        note(&mut seen, "before", &r);
        let shown: Vec<usize> = (0..3).filter(|i| c.shown[*i]).collect();
        let items: Vec<(WidgetId, bool)> = shown
            .iter()
            .map(|i| (ui.id(CHILDREN[*i]), c.enabled[*i]))
            .collect();
        seen.stop = ui.rove_stop(&items, c.active);
        for (n, i) in shown.iter().enumerate() {
            let sense = if seen.stop == Some(n) {
                Sense::CLICK_FOCUS
            } else {
                Sense::CLICK_ROVE
            };
            let (id, enabled) = items[n];
            let r = ui.with_enabled(enabled, |ui| ui.interact(id, rect(1 + i), sense));
            note(&mut seen, CHILDREN[*i], &r);
            labelled(ui, id, rect(1 + i));
        }
        seen.moved = ui.rove_keys(&items, c.axis, c.wrap);
        if let Some(k) = c.ask {
            let id = ui.id(k);
            ui.request_focus(id);
        }
        let r = stop(ui, "after", 4);
        note(&mut seen, "after", &r);
        if let Some(o) = &c.overlay {
            ui.overlay(
                "over",
                o,
                |_, _, _| {},
                |ui, _| {
                    for (k, i) in [("x", 0), ("y", 1)] {
                        let id = ui.id(k);
                        let r = ui.interact(
                            id,
                            Rect::new(rect(i).x, 100.0, 50.0, 30.0),
                            Sense::CLICK_FOCUS,
                        );
                        note(&mut seen, k, &r);
                    }
                },
            );
        }
    });
    seen.repaint_now = out.repaint == Repaint::Now;
    seen
}

fn key(h: &mut UiHarness, k: NamedKey) {
    h.input.key_down(Key::Named(k), None);
}

/// Tabs from `before` onto the composite's stop.
fn enter(h: &mut UiHarness, c: &Rove) -> Seen {
    rove_frame(h, c);
    tab(h);
    assert_eq!(rove_frame(h, c).focused, Some("before"));
    tab(h);
    rove_frame(h, c)
}

fn press_child(h: &mut UiHarness, i: usize) {
    h.input.pointer_to(rect(1 + i).x + 5.0, 20.0).click(P);
}

#[test]
fn tab_enters_on_active_and_the_second_tab_leaves() {
    for (active, child) in CHILDREN.into_iter().enumerate() {
        let mut h = harness();
        let c = rove(active);
        let seen = enter(&mut h, &c);
        assert_eq!(seen.focused, Some(child));
        assert!(seen.ring);
        tab(&mut h);
        assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
    }
}

#[test]
fn shift_tab_from_after_re_enters_on_the_focused_or_active_child() {
    let mut h = harness();
    let c = rove(1);
    enter(&mut h, &c);
    tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
    shift_tab(&mut h);
    assert_eq!(
        rove_frame(&mut h, &c).focused,
        Some("c1"),
        "the active child"
    );
    key(&mut h, NamedKey::ArrowDown);
    rove_frame(&mut h, &c);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("c2"));
    tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
    shift_tab(&mut h);
    assert_eq!(
        rove_frame(&mut h, &c).focused,
        Some("c1"),
        "focus left: the stop is the active child again"
    );
    shift_tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("before"));
}

/// Each arrow from the middle child, each way out: the old stop is on either side of the new
/// child, and the frame after the arrow still reads the regions of the frame with the old stop.
#[test]
fn an_arrow_and_a_tab_in_adjacent_frames_leave_the_composite() {
    for arrow in [NamedKey::ArrowDown, NamedKey::ArrowUp] {
        for back in [false, true] {
            let mut h = harness();
            let c = rove(1);
            enter(&mut h, &c);
            key(&mut h, arrow);
            assert!(rove_frame(&mut h, &c).moved.is_some());
            if back {
                shift_tab(&mut h);
            } else {
                tab(&mut h);
            }
            let want = if back { "before" } else { "after" };
            assert_eq!(
                rove_frame(&mut h, &c).focused,
                Some(want),
                "{arrow:?} then back={back}"
            );
        }
    }
}

#[test]
fn an_arrow_and_a_tab_in_the_same_frame_leave_the_composite() {
    for order in [[true, false], [false, true]] {
        for arrow in [NamedKey::ArrowDown, NamedKey::ArrowUp] {
            let mut h = harness();
            let c = rove(1);
            enter(&mut h, &c);
            for is_tab in order {
                if is_tab {
                    tab(&mut h);
                } else {
                    key(&mut h, arrow);
                }
            }
            let seen = rove_frame(&mut h, &c);
            assert_eq!(seen.focused, Some("after"), "{order:?} {arrow:?}");
            assert_eq!(seen.moved, None, "the arrow has no focused child to move");
            assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
        }
    }
}

#[test]
fn a_press_on_a_non_stop_child_focuses_it_with_no_ring_then_an_arrow_shows_it() {
    let mut h = harness();
    let c = rove(0);
    rove_frame(&mut h, &c);
    press_child(&mut h, 1);
    let seen = rove_frame(&mut h, &c);
    assert_eq!((seen.focused, seen.ring), (Some("c1"), false));
    assert_eq!(seen.stop, Some(1));
    key(&mut h, NamedKey::ArrowDown);
    assert_eq!(rove_frame(&mut h, &c).moved, Some(2));
    let seen = rove_frame(&mut h, &c);
    assert_eq!((seen.focused, seen.ring), (Some("c2"), true));
}

#[test]
fn a_press_and_a_tab_in_one_frame_leave_the_composite() {
    let mut h = harness();
    let c = rove(2);
    rove_frame(&mut h, &c);
    press_child(&mut h, 0);
    tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
}

#[test]
fn an_access_focus_on_a_non_stop_child_makes_it_the_stop() {
    let mut h = harness();
    let c = rove(2);
    rove_frame(&mut h, &c);
    h.input.access(AccessAction {
        target: WidgetId::ROOT.with("c0"),
        action: Action::Focus,
        value: None,
    });
    let seen = rove_frame(&mut h, &c);
    assert_eq!((seen.focused, seen.ring), (Some("c0"), false));
    assert_eq!(seen.stop, Some(0));
    tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
}

#[test]
fn an_access_focus_and_a_tab_in_one_frame_leave_the_composite() {
    let mut h = harness();
    let c = rove(2);
    rove_frame(&mut h, &c);
    h.input.access(AccessAction {
        target: WidgetId::ROOT.with("c0"),
        action: Action::Focus,
        value: None,
    });
    tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
}

#[test]
fn a_request_focus_on_a_child_and_a_tab_in_the_next_frame_leave_the_composite() {
    let mut h = harness();
    let mut c = rove(2);
    rove_frame(&mut h, &c);
    c.ask = Some("c0");
    rove_frame(&mut h, &c);
    c.ask = None;
    tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
}

#[test]
fn a_disabled_active_falls_to_the_first_enabled() {
    let mut h = harness();
    let mut c = rove(1);
    c.enabled = [false, false, true];
    assert_eq!(enter(&mut h, &c).focused, Some("c2"));
    c.enabled = [true, false, true];
    c.active = Some(1);
    let mut h = harness();
    let seen = enter(&mut h, &c);
    assert_eq!((seen.focused, seen.stop), (Some("c0"), Some(0)));
}

#[test]
fn with_all_children_disabled_there_is_no_stop() {
    let mut h = harness();
    let mut c = rove(1);
    c.enabled = [false; 3];
    rove_frame(&mut h, &c);
    tab(&mut h);
    let seen = rove_frame(&mut h, &c);
    assert_eq!((seen.focused, seen.stop), (Some("before"), None));
    tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
}

#[test]
fn the_focused_child_removed_loses_focus_and_the_stop_returns_to_active() {
    let mut h = harness();
    let mut c = rove(0);
    enter(&mut h, &c);
    key(&mut h, NamedKey::ArrowDown);
    rove_frame(&mut h, &c);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("c1"));
    c.shown = [true, false, true];
    rove_frame(&mut h, &c);
    let seen = rove_frame(&mut h, &c);
    assert_eq!((seen.focused, seen.stop), (None, Some(0)));
}

#[test]
fn tab_from_a_child_disabled_under_focus_leaves_the_composite() {
    for active in [0, 2] {
        let mut h = harness();
        let mut c = rove(active);
        rove_frame(&mut h, &c);
        press_child(&mut h, 1);
        assert_eq!(rove_frame(&mut h, &c).focused, Some("c1"));
        c.enabled[1] = false;
        assert_eq!(rove_frame(&mut h, &c).stop, Some(active));
        rove_frame(&mut h, &c);
        tab(&mut h);
        assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
    }
}

#[test]
fn the_app_moving_active_moves_the_stop_only_while_focus_is_outside() {
    let mut h = harness();
    let mut c = rove(0);
    enter(&mut h, &c);
    c.active = Some(2);
    assert_eq!(rove_frame(&mut h, &c).stop, Some(0), "focus holds the stop");
    tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
    assert_eq!(rove_frame(&mut h, &c).stop, Some(2));
    shift_tab(&mut h);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("c2"));
}

fn popup() -> OverlayOptions {
    OverlayOptions {
        layer: LayerId::Popup,
        modal: false,
        ..modal()
    }
}

#[test]
fn a_menu_opened_from_a_child_and_closed_restores_it_and_tab_then_leaves() {
    for active in [0, 2] {
        let mut h = harness();
        let mut c = rove(active);
        rove_frame(&mut h, &c);
        press_child(&mut h, 1);
        rove_frame(&mut h, &c);
        c.overlay = Some(popup());
        rove_frame(&mut h, &c);
        let seen = rove_frame(&mut h, &c);
        assert_eq!(seen.focused, Some("x"));
        assert_eq!(seen.stop, Some(active), "focus left the composite");
        c.overlay = None;
        rove_frame(&mut h, &c);
        tab(&mut h);
        assert_eq!(rove_frame(&mut h, &c).focused, Some("after"));
    }
}

#[test]
fn a_closing_menu_gives_focus_back_to_a_non_stop_child() {
    let mut h = harness();
    let mut c = rove(2);
    rove_frame(&mut h, &c);
    press_child(&mut h, 0);
    rove_frame(&mut h, &c);
    c.overlay = Some(popup());
    rove_frame(&mut h, &c);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("x"));
    c.overlay = None;
    rove_frame(&mut h, &c);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("c0"));
}

#[test]
fn a_modal_over_a_composite_keeps_its_tab_trap() {
    let mut h = harness();
    let mut c = rove(0);
    enter(&mut h, &c);
    c.overlay = Some(modal());
    rove_frame(&mut h, &c);
    assert_eq!(rove_frame(&mut h, &c).focused, Some("x"));
    for _ in 0..6 {
        tab(&mut h);
        let seen = rove_frame(&mut h, &c);
        assert!(matches!(seen.focused, Some("x" | "y")), "{seen:?}");
    }
    key(&mut h, NamedKey::ArrowDown);
    assert_eq!(rove_frame(&mut h, &c).moved, None);
}

#[test]
fn rove_keys_walk_skips_disabled_stops_at_the_ends_and_wraps_when_asked() {
    let mut c = rove(0);
    c.enabled = [true, false, true];
    let mut h = harness();
    enter(&mut h, &c);
    key(&mut h, NamedKey::ArrowDown);
    assert_eq!(
        rove_frame(&mut h, &c).moved,
        Some(2),
        "over the disabled one"
    );
    rove_frame(&mut h, &c);
    key(&mut h, NamedKey::ArrowDown);
    assert_eq!(rove_frame(&mut h, &c).moved, None, "no wrap");
    assert_eq!(rove_frame(&mut h, &c).focused, Some("c2"));
    c.wrap = true;
    key(&mut h, NamedKey::ArrowDown);
    assert_eq!(rove_frame(&mut h, &c).moved, Some(0), "wraps");
    rove_frame(&mut h, &c);
    key(&mut h, NamedKey::ArrowUp);
    assert_eq!(rove_frame(&mut h, &c).moved, Some(2), "wraps back");
}

#[test]
fn home_end_and_several_presses_in_one_frame_move_in_order() {
    let mut c = rove(0);
    c.axis = RoveAxis::Horizontal;
    c.enabled = [true, true, true];
    let mut h = harness();
    enter(&mut h, &c);
    key(&mut h, NamedKey::End);
    assert_eq!(rove_frame(&mut h, &c).moved, Some(2));
    rove_frame(&mut h, &c);
    key(&mut h, NamedKey::Home);
    assert_eq!(rove_frame(&mut h, &c).moved, Some(0));
    rove_frame(&mut h, &c);
    key(&mut h, NamedKey::ArrowRight);
    key(&mut h, NamedKey::ArrowRight);
    assert_eq!(rove_frame(&mut h, &c).moved, Some(2));
}

/// What stays in the queue for the focused child after `rove_keys`, from `press`.
fn left_after(
    axis: RoveAxis,
    wrap: bool,
    one_child: bool,
    press: NamedKey,
    chord: Modifiers,
) -> usize {
    let mut h = harness();
    let items = |ui: &Ui<'_>| {
        let mut v = vec![(ui.id("c0"), true), (ui.id("c1"), !one_child)];
        v.push((ui.id("c2"), !one_child));
        v
    };
    let mut left = 0;
    let mut run = |h: &mut UiHarness| {
        h.frame(|ui| {
            let list = items(ui);
            let at = ui.rove_stop(&list, Some(0));
            for (i, (id, on)) in list.iter().enumerate() {
                let sense = if Some(i) == at {
                    Sense::CLICK_FOCUS
                } else {
                    Sense::CLICK_ROVE
                };
                ui.with_enabled(*on, |ui| ui.interact(*id, rect(i), sense));
            }
            ui.rove_keys(&list, axis, wrap);
            left = ui.focus_events(list[0].0).len();
        });
    };
    run(&mut h);
    tab(&mut h);
    run(&mut h);
    h.input.set_modifiers(chord);
    key(&mut h, press);
    h.input.set_modifiers(Modifiers::default());
    run(&mut h);
    left
}

#[test]
fn a_key_with_no_move_is_taken_and_keys_off_the_axis_are_left() {
    use NamedKey::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Home};
    let none = Modifiers::default();
    assert_eq!(
        left_after(RoveAxis::Vertical, false, false, ArrowUp, none),
        0
    );
    assert_eq!(left_after(RoveAxis::Vertical, false, false, Home, none), 0);
    assert_eq!(
        left_after(RoveAxis::Vertical, false, true, ArrowDown, none),
        0
    );
    assert_eq!(
        left_after(RoveAxis::Vertical, true, true, ArrowDown, none),
        0
    );
    assert_eq!(
        left_after(RoveAxis::Vertical, false, false, ArrowLeft, none),
        1
    );
    assert_eq!(
        left_after(RoveAxis::Horizontal, false, false, ArrowDown, none),
        1
    );
    assert_eq!(
        left_after(RoveAxis::Both, false, false, ArrowRight, none),
        0
    );
    assert_eq!(left_after(RoveAxis::Both, false, false, ArrowUp, none), 0);
    let chord = Modifiers {
        control: true,
        ..none
    };
    assert_eq!(
        left_after(RoveAxis::Vertical, false, false, ArrowDown, chord),
        1
    );
}

#[test]
fn with_one_enabled_child_nothing_moves() {
    for wrap in [false, true] {
        let mut c = rove(1);
        c.enabled = [false, true, false];
        c.wrap = wrap;
        let mut h = harness();
        enter(&mut h, &c);
        for k in [
            NamedKey::ArrowDown,
            NamedKey::ArrowUp,
            NamedKey::Home,
            NamedKey::End,
        ] {
            key(&mut h, k);
            assert_eq!(rove_frame(&mut h, &c).moved, None, "{k:?}");
            assert_eq!(rove_frame(&mut h, &c).focused, Some("c1"));
        }
    }
}

#[test]
fn rove_focus_shows_the_ring_and_request_focus_clears_it() {
    let mut h = harness();
    let a = WidgetId::ROOT.with("a");
    let run = |h: &mut UiHarness, ask: &dyn Fn(&mut Ui<'_>)| {
        let mut seen = (false, false);
        let (_, out) = h.frame(|ui| {
            let r = stop(ui, "a", 0);
            seen = (r.focused, r.focus_visible);
            ask(ui);
        });
        (seen, out.repaint)
    };
    run(&mut h, &|_| {});
    assert_eq!(run(&mut h, &|ui| ui.request_focus(a)).1, Repaint::Now);
    assert_eq!(run(&mut h, &|_| {}), ((true, false), Repaint::Idle));
    // The same id, the ring off: the request still changes something.
    assert_eq!(run(&mut h, &|ui| ui.rove_focus(a)).1, Repaint::Now);
    assert_eq!(run(&mut h, &|_| {}), ((true, true), Repaint::Idle));
    // The ring on and asked for again: nothing to do.
    assert_eq!(run(&mut h, &|ui| ui.rove_focus(a)).1, Repaint::Idle);
    assert_eq!(run(&mut h, &|_| {}), ((true, true), Repaint::Idle));
    assert_eq!(run(&mut h, &|ui| ui.request_focus(a)).1, Repaint::Now);
    assert_eq!(run(&mut h, &|_| {}), ((true, false), Repaint::Idle));
}

#[test]
fn a_skip_tab_region_is_no_stop_and_tab_continues_from_it_in_call_order() {
    let mut h = harness();
    let run = |h: &mut UiHarness| {
        let mut seen = Seen::default();
        h.frame(|ui| {
            let r = stop(ui, "a", 0);
            note(&mut seen, "a", &r);
            let id = ui.id("text");
            let r = ui.interact(id, rect(1), Sense::CLICK_ROVE);
            note(&mut seen, "text", &r);
            let r = stop(ui, "b", 2);
            note(&mut seen, "b", &r);
        });
        seen.focused
    };
    run(&mut h);
    let mut order = Vec::new();
    for _ in 0..3 {
        tab(&mut h);
        order.push(run(&mut h));
    }
    assert_eq!(order, [Some("a"), Some("b"), Some("a")]);
    h.input.pointer_to(rect(1).x + 5.0, 20.0).click(P);
    assert_eq!(run(&mut h), Some("text"));
    tab(&mut h);
    assert_eq!(run(&mut h), Some("b"));
    h.input.pointer_to(rect(1).x + 5.0, 20.0).click(P);
    assert_eq!(run(&mut h), Some("text"));
    shift_tab(&mut h);
    assert_eq!(run(&mut h), Some("a"));
}

#[test]
fn initial_focus_ignores_skip_tab_regions() {
    let mut h = harness();
    let mut focused = None;
    for _ in 0..2 {
        h.frame(|ui| {
            ui.overlay(
                "m",
                &modal(),
                |_, _, _| {},
                |ui, _| {
                    let id = ui.id("text");
                    if ui.interact(id, rect(0), Sense::CLICK_ROVE).focused {
                        focused = Some("text");
                    }
                    if stop(ui, "x", 1).focused {
                        focused = Some("x");
                    }
                },
            );
        });
    }
    assert_eq!(focused, Some("x"));
}

// --- Tab scopes (`Ui::tab_scope`) ---------------------------------------------------------------

/// `a`, `b` in a scope of order 1 drawn first; `c`, `d` in a scope of order 0 drawn second; `e`
/// outside any scope (order 0) last. Returns the focused key.
fn scoped(h: &mut UiHarness) -> Option<&'static str> {
    let mut focused = None;
    h.frame(|ui| {
        let mut note = |k: &'static str, r: Response| {
            if r.focused {
                focused = Some(k);
            }
        };
        ui.tab_scope(1, |ui| {
            note("a", stop(ui, "a", 0));
            note("b", stop(ui, "b", 1));
        });
        ui.tab_scope(0, |ui| {
            note("c", stop(ui, "c", 2));
            note("d", stop(ui, "d", 3));
        });
        note("e", stop(ui, "e", 4));
    });
    focused
}

#[test]
fn a_scope_of_order_one_drawn_first_is_tabbed_after_the_scope_of_order_zero() {
    let mut h = harness();
    scoped(&mut h);
    let mut order = Vec::new();
    for _ in 0..6 {
        tab(&mut h);
        order.push(scoped(&mut h));
    }
    // Order 0 in registration order (c, d, then e, which is outside any scope), then order 1.
    let want = ["c", "d", "e", "a", "b", "c"].map(Some);
    assert_eq!(order, want);
}

#[test]
fn shift_tab_reverses_the_scoped_order() {
    let mut h = harness();
    scoped(&mut h);
    let mut back = Vec::new();
    for _ in 0..6 {
        shift_tab(&mut h);
        back.push(scoped(&mut h));
    }
    let want = ["b", "a", "e", "d", "c", "b"].map(Some);
    assert_eq!(back, want);
}

#[test]
fn a_nested_scope_replaces_the_outer_order_and_the_outer_comes_back() {
    let mut h = harness();
    let run = |h: &mut UiHarness| {
        let mut focused = None;
        h.frame(|ui| {
            let mut note = |k: &'static str, r: Response| {
                if r.focused {
                    focused = Some(k);
                }
            };
            ui.tab_scope(5, |ui| {
                note("a", stop(ui, "a", 0));
                ui.tab_scope(0, |ui| note("b", stop(ui, "b", 1)));
                note("c", stop(ui, "c", 2));
            });
            note("d", stop(ui, "d", 3));
        });
        focused
    };
    run(&mut h);
    let mut order = Vec::new();
    for _ in 0..5 {
        tab(&mut h);
        order.push(run(&mut h));
    }
    // b and d are order 0, a and c are 5 (the outer scope is back after the nested one).
    assert_eq!(order, ["b", "d", "a", "c", "b"].map(Some));
}

#[test]
fn a_modal_overlays_stops_are_its_own_and_sorted_among_themselves() {
    let mut h = harness();
    let o = modal();
    let run = |h: &mut UiHarness, open: bool| {
        let mut focused = None;
        h.frame(|ui| {
            ui.painter().on(LayerId::Page);
            ui.tab_scope(1, |ui| {
                if stop(ui, "a", 0).focused {
                    focused = Some("a");
                }
            });
            if open {
                ui.overlay(
                    "modal",
                    &o,
                    |_, _, _| {},
                    |ui, _| {
                        // x and z are order 1 and y, drawn between them, is order 0.
                        ui.tab_scope(1, |ui| {
                            if stop(ui, "x", 2).focused {
                                focused = Some("x");
                            }
                        });
                        if stop(ui, "y", 3).focused {
                            focused = Some("y");
                        }
                        ui.tab_scope(1, |ui| {
                            if stop(ui, "z", 4).focused {
                                focused = Some("z");
                            }
                        });
                    },
                );
            }
        });
        focused
    };
    run(&mut h, true);
    run(&mut h, true);
    let mut seen = Vec::new();
    for _ in 0..6 {
        tab(&mut h);
        seen.push(run(&mut h, true));
    }
    // The page's `a` is never reached; the modal's stops go y (order 0), then x and z (order 1).
    let at = seen.iter().position(|f| *f == Some("y")).expect("y");
    let cycle: Vec<_> = (0..3).map(|i| seen[at + i]).collect();
    assert_eq!(cycle, ["y", "x", "z"].map(Some), "{seen:?}");
    assert!(!seen.contains(&Some("a")), "{seen:?}");
}

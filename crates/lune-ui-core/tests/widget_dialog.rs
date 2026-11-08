//! The dialog (C4 retrofit, `overlay.dialog` acceptance): rest identity with the Painter API,
//! behaviour (Cancel, confirm, Escape, Enter, scrim click, Tab trap), pixel probes, the open and
//! close motion sampled at 60 Hz, exit retention with the app's body, the node; then the variants
//! of `Ui::dialog_with` (the form width, the blurred scrim, a disabled confirm) and a dialog over
//! a dialog.

use lune_ui_core::access::{AccessAction, Action, Actions, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{FRAME, MotionTier, Time, conformance, conformance_of};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations, flattened};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, Ui, WidgetId};
use lune_ui_core::widgets::button::{ButtonKind, button};
use lune_ui_core::widgets::dialog::{
    CLOSE, Cancel, Dialog, DialogGlass, DialogHits, DialogOptions, DialogResponse, DialogSpec,
    DialogWidth, OPEN, dialog,
};
use lune_ui_core::widgets::menu::MenuRow;

const WIN: (f32, f32) = (800.0, 600.0);
const P: PointerButton = PointerButton::Primary;
const PAGE_BTN: Rect = Rect::new(20.0, 20.0, 100.0, 32.0);
const BODY_FILL: [u8; 4] = [200, 30, 30, 255];

fn glass() -> DialogGlass {
    DialogGlass {
        fill: [0.2, 0.2, 0.2, 0.62],
        noise: true,
    }
}

fn dialog_id() -> WidgetId {
    WidgetId::ROOT.with("d")
}

fn the_dialog(cancel: Cancel<'static>, extra_h: f32) -> Dialog<'static> {
    Dialog {
        title: "Discard changes",
        message: "The document has unsaved changes. Closing it will discard them.",
        extra_h,
        confirm: "Discard",
        cancel,
    }
}

/// The page under every dialog in these tests: a painted card.
fn page<H: Copy + PartialEq>(p: &mut Painter<'_, H>) {
    p.on(LayerId::Page);
    p.fill(Rect::new(10.0, 10.0, 500.0, 300.0), 6.0, [30, 30, 30, 255]);
}

/// A painter-only body: a fill and a caption.
fn body<H: Copy + PartialEq>(p: &mut Painter<'_, H>, x: f32, y: f32, w: f32) {
    p.fill(Rect::new(x, y + 12.0, w, 20.0), 4.0, BODY_FILL);
}

/// What the Painter-API `dialog` draws.
fn old_scene(cancel: Option<&str>, extra_h: f32) -> Scene {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    page(&mut p);
    button(&mut p, PAGE_BTN, "Page", false, 3);
    let d = the_dialog(Cancel::None, extra_h);
    let spec = DialogSpec {
        title: d.title,
        message: d.message,
        extra_h,
        confirm: d.confirm,
        cancel,
    };
    let hits = DialogHits {
        block: 0u8,
        confirm: 1,
        cancel: 2,
    };
    dialog(&mut p, &spec, &glass(), &hits, |p, x, y, w| {
        if extra_h > 0.0 {
            body(p, x, y, w);
        }
    });
    p.scene
}

/// One frame: the page, a page button (to prove nothing under the modal reacts), the dialog when
/// `open`. Returns the dialog's report and the page button's response too.
fn frame(
    h: &mut UiHarness,
    d: Option<&Dialog<'_>>,
) -> (Scene, FrameOutput, DialogResponse, Response) {
    let mut report = DialogResponse::default();
    let mut page_button = None;
    let (scene, out) = h.frame(|ui| {
        page(ui.painter());
        page_button = Some(ui.button("page", PAGE_BTN, "Page", ButtonKind::Secondary));
        if let Some(d) = d {
            report = ui.dialog("d", d, &glass(), |ui: &mut Ui<'_>, x, y, w| {
                if d.extra_h > 0.0 {
                    body(ui.painter(), x, y, w);
                }
            });
        }
    });
    (scene, out, report, page_button.expect("frame ran"))
}

/// A harness with the dialog open and settled (0 ms, then 1000 ms).
fn opened(d: &Dialog<'_>) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, Some(d));
    h.at_ms(1000);
    let (_, out, _, _) = frame(&mut h, Some(d));
    assert_eq!(out.repaint, Repaint::Idle);
    h
}

fn centre(out: &FrameOutput, name: &str) -> (f32, f32) {
    let n = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Button && n.name == name)
        .unwrap_or_else(|| panic!("no button {name}"));
    (n.bounds.x + n.bounds.w / 2.0, n.bounds.y + n.bounds.h / 2.0)
}

fn card_bounds(out: &FrameOutput) -> Rect {
    out.tree.get(dialog_id()).expect("dialog node").bounds
}

#[test]
fn at_rest_the_ui_dialog_draws_exactly_what_the_painter_dialog_does() {
    let cases = [
        (Cancel::Default, Some("Cancel"), 0.0),
        (Cancel::None, None, 0.0),
        (Cancel::Label("Keep"), Some("Keep"), 0.0),
        (Cancel::Default, Some("Cancel"), 100.0),
    ];
    for (cancel, old_cancel, extra) in cases {
        let d = the_dialog(cancel, extra);
        let mut h = UiHarness::new(WIN);
        frame(&mut h, Some(&d));
        h.at_ms(1000);
        let (scene, out, _, _) = frame(&mut h, Some(&d));
        assert_eq!(out.repaint, Repaint::Idle);
        assert_eq!(
            flattened(&scene),
            old_scene(old_cancel, extra),
            "{cancel:?} extra {extra}"
        );
    }
}

#[test]
fn at_rest_nothing_is_transparent_and_no_transform_is_pushed() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    h.at_ms(2000);
    let (scene, _, _, _) = frame(&mut h, Some(&d));
    let modal = &scene.merged(LayerId::Modal);
    let g = &modal.geometry;
    let frost = g.under.iter().find(|s| s.is_frost()).expect("the card");
    assert_eq!(frost.params[1], 1.0, "at full alpha");
    assert!(g.under.iter().chain(&g.over).all(|s| s.color[3] > 0.0));
    assert!(modal.texts.iter().all(|t| t.scale == 1.0));
}

#[test]
fn cancel_default_uses_the_string_table() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = UiHarness::new(WIN);
    h.strings
        .insert("luneui.dialog.cancel", "Annuler")
        .expect("toolkit key");
    frame(&mut h, Some(&d));
    h.at_ms(1000);
    let (_, out, _, _) = frame(&mut h, Some(&d));
    assert!(
        out.tree
            .nodes
            .iter()
            .any(|n| n.role == Role::Button && n.name == "Annuler")
    );
}

/// Runs `frames` more frames after `script` and sums what the dialog reported.
struct Tally {
    confirmed: u32,
    cancelled: u32,
}

fn run(h: &mut UiHarness, d: &Dialog<'_>, from_ms: u64, frames: u64) -> (Tally, FrameOutput) {
    let mut t = Tally {
        confirmed: 0,
        cancelled: 0,
    };
    let mut last = None;
    for i in 0..frames {
        h.at_ms(from_ms + i * 10);
        let (_, out, r, _) = frame(h, Some(d));
        t.confirmed += u32::from(r.confirmed);
        t.cancelled += u32::from(r.cancelled);
        last = Some(out);
    }
    (t, last.expect("a frame ran"))
}

fn click_on(h: &mut UiHarness, d: &Dialog<'_>, at: (f32, f32)) -> Tally {
    h.input.pointer_to(at.0, at.1);
    let (a, _) = run(h, d, 2000, 2);
    h.input.press(P);
    let (b, _) = run(h, d, 2100, 2);
    h.input.release(P);
    let (c, _) = run(h, d, 2200, 3);
    Tally {
        confirmed: a.confirmed + b.confirmed + c.confirmed,
        cancelled: a.cancelled + b.cancelled + c.cancelled,
    }
}

#[test]
fn a_click_on_confirm_fires_once() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    let (_, out) = run(&mut h, &d, 1100, 1);
    let at = centre(&out, "Discard");
    let t = click_on(&mut h, &d, at);
    assert_eq!((t.confirmed, t.cancelled), (1, 0));
}

#[test]
fn a_click_on_cancel_fires_once() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    let (_, out) = run(&mut h, &d, 1100, 1);
    let at = centre(&out, "Cancel");
    let t = click_on(&mut h, &d, at);
    assert_eq!((t.confirmed, t.cancelled), (0, 1));
}

#[test]
fn escape_cancels_once() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (t, _) = run(&mut h, &d, 1100, 4);
    assert_eq!((t.confirmed, t.cancelled), (0, 1));
}

#[test]
fn enter_confirms_once_at_open() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (t, _) = run(&mut h, &d, 1100, 4);
    assert_eq!((t.confirmed, t.cancelled), (1, 0));
}

#[test]
fn enter_on_the_focused_cancel_cancels() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    // Confirm is focused at open; Shift+Tab moves to Cancel (the first stop).
    h.input.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    h.input.set_modifiers(Modifiers::default());
    let (_, out) = run(&mut h, &d, 1100, 2);
    let cancel = out
        .tree
        .nodes
        .iter()
        .find(|n| n.name == "Cancel")
        .expect("cancel");
    assert_eq!(out.focused, Some(cancel.id));
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (t, _) = run(&mut h, &d, 1200, 4);
    assert_eq!((t.confirmed, t.cancelled), (0, 1));
}

#[test]
fn a_scrim_click_cancels_once_and_a_card_click_does_nothing() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    let (_, out) = run(&mut h, &d, 1100, 1);
    let card = card_bounds(&out);
    // On the card, on its message (not on a button).
    let t = click_on(&mut h, &d, (card.x + 40.0, card.y + 50.0));
    assert_eq!((t.confirmed, t.cancelled), (0, 0), "card click");
    // On the scrim, over the page button's place (so the page button must stay inert too).
    let mut page_clicked = false;
    h.input.pointer_to(30.0, 30.0);
    run(&mut h, &d, 3000, 2);
    h.input.press(P);
    run(&mut h, &d, 3100, 2);
    h.input.release(P);
    let mut t = Tally {
        confirmed: 0,
        cancelled: 0,
    };
    for i in 0..3 {
        h.at_ms(3200 + i * 10);
        let (_, _, r, b) = frame(&mut h, Some(&d));
        t.confirmed += u32::from(r.confirmed);
        t.cancelled += u32::from(r.cancelled);
        page_clicked |= b.clicked || b.hovered || b.focused;
    }
    assert_eq!((t.confirmed, t.cancelled), (0, 1), "scrim click");
    assert!(!page_clicked, "the page under the modal stays inert");
}

#[test]
fn tab_never_leaves_the_dialog() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    let (_, out) = run(&mut h, &d, 1100, 1);
    let ids: Vec<_> = ["Cancel", "Discard"]
        .into_iter()
        .map(|name| {
            out.tree
                .nodes
                .iter()
                .find(|n| n.name == name)
                .expect("button")
                .id
        })
        .collect();
    assert_eq!(out.focused, Some(ids[1]), "Confirm is focused at open");
    let mut seen = Vec::new();
    for i in 0..6 {
        h.input.key_down(Key::Named(NamedKey::Tab), None);
        let (_, out) = run(&mut h, &d, 1200 + i * 20, 2);
        seen.push(out.focused.expect("focus stays"));
    }
    // Cancel is the first stop, Confirm the second: from Confirm Tab wraps to Cancel.
    assert_eq!(seen, [ids[0], ids[1], ids[0], ids[1], ids[0], ids[1]]);
    // Shift+Tab wraps backwards inside as well.
    h.input.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    });
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let (_, out) = run(&mut h, &d, 2000, 2);
    assert!(out.focused == Some(ids[0]) || out.focused == Some(ids[1]));
}

#[test]
fn the_default_action_confirms_and_dismiss_cancels() {
    let d = the_dialog(Cancel::Default, 0.0);
    for (action, want) in [(Action::Default, (1, 0)), (Action::Dismiss, (0, 1))] {
        let mut h = opened(&d);
        h.input.access(AccessAction {
            target: dialog_id(),
            action,
            value: None,
        });
        let (t, _) = run(&mut h, &d, 1100, 3);
        assert_eq!((t.confirmed, t.cancelled), want, "{action:?}");
    }
}

#[test]
fn the_node_is_a_modal_dialog_named_by_its_title() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    let (_, out) = run(&mut h, &d, 1100, 1);
    let node = out.tree.get(dialog_id()).expect("dialog node");
    assert_eq!(node.role, Role::Dialog);
    assert_eq!(node.name, d.title);
    assert_eq!(node.description.as_deref(), Some(d.message));
    assert!(node.states.contains(State::Modal));
    assert!(node.actions.contains(Action::Default));
    assert!(node.actions.contains(Action::Dismiss));
    // Both buttons are its children.
    let buttons = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Button && n.parent == Some(dialog_id()))
        .count();
    assert_eq!(buttons, 2);
}

#[test]
fn pixel_probes_at_scale_two() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    frame(&mut h, Some(&d));
    h.at_ms(1000);
    let (scene, _, _, _) = frame(&mut h, Some(&d));
    let modal = &scene.merged(LayerId::Modal);
    // Scrim: the window, #00000080.
    let scrim = modal.geometry.under[0];
    assert_eq!(scrim.rect, [0.0, 0.0, WIN.0 * 2.0, WIN.1 * 2.0]);
    assert_eq!(DARK.dialog_backdrop, [0, 0, 0, 0x80]);
    assert_eq!(scrim.color, premul([0, 0, 0, 0x80]));
    // Card: 896 wide, corner radius 16.
    let card = modal.geometry.under.iter().find(|s| s.is_frost());
    let card = card.expect("the card");
    assert_eq!(card.rect[2], 896.0);
    assert_eq!(card.radii, [16.0; 4]);
    assert_eq!(card.params[1], 1.0);
    // Footer: 136 high, bottom corners 16.
    let footer = modal
        .geometry
        .under
        .iter()
        .find(|s| s.radii == [0.0, 0.0, 16.0, 16.0])
        .expect("footer");
    assert_eq!(footer.rect[3], 136.0);
    assert_eq!(footer.rect[2], 896.0);
    // The confirm button's fill, 64 high; the cancel button rests clear and draws no fill (R4).
    let buttons: Vec<_> = modal
        .geometry
        .under
        .iter()
        .filter(|s| s.radii == [12.0; 4])
        .collect();
    assert_eq!(buttons.len(), 1);
    assert!(buttons.iter().all(|s| s.rect[3] == 64.0));
    assert_eq!(buttons[0].color, premul(DARK.surface_input));
}

#[test]
fn a_footer_button_is_at_least_80_wide_primary_right_and_secondary_left() {
    let two_letters = Dialog {
        title: "Quit",
        message: "Really?",
        extra_h: 0.0,
        confirm: "OK",
        cancel: Cancel::Label("No"),
    };
    let mut h = opened(&two_letters);
    let (_, out) = run(&mut h, &two_letters, 1200, 1);
    let bounds = |name: &str| {
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == Role::Button && n.name == name)
            .unwrap_or_else(|| panic!("no button {name}"))
            .bounds
    };
    let (ok, no) = (bounds("OK"), bounds("No"));
    assert_eq!(
        (ok.w, no.w),
        (80.0, 80.0),
        "two letters are an 80 px button"
    );
    assert!(no.right() < ok.x, "the secondary is left of the primary");
    assert_eq!(ok.x - no.right(), 8.0, "spacing 8");
    // The card's right padding is 24, whatever the width.
    assert_eq!(card_bounds(&out).right() - ok.right(), 24.0);
    // A label wider than 80 keeps its own width: "Discard" plus 16 each side.
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    let (_, out) = run(&mut h, &d, 1200, 1);
    let discard = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Button && n.name == "Discard")
        .expect("the confirm");
    assert!(discard.bounds.w >= 80.0);
}

/// The sampler of a dialog's visibility on the 60 Hz grid: the scrim's alpha as a fraction of its
/// rest alpha (0 when nothing is drawn) and the card's scale (the title text's glyph scale; 0.96
/// when nothing is drawn yet).
fn measure(scene: &Scene) -> (f32, f32) {
    let modal = &scene.merged(LayerId::Modal);
    let rest = premul(DARK.dialog_backdrop)[3];
    let opacity = modal
        .geometry
        .under
        .first()
        .map_or(0.0, |s| s.color[3] / rest);
    let scale = modal.texts.first().map_or(0.96, |t| t.scale);
    (opacity, scale)
}

/// Opens (`open`) or closes a dialog on the 60 Hz grid under `tier` and returns the conformance
/// samples of (opacity, scale) as `(time, value, moving)` lists.
type Samples = Vec<(Time, f32, bool)>;

fn sample(tier: MotionTier, open: bool) -> (Samples, Samples) {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let t0: u64 = if open { 0 } else { 5000 };
    if !open {
        frame(&mut h, Some(&d));
        h.at_ms(2000);
        frame(&mut h, Some(&d));
    }
    let (mut opacity, mut scale) = (Vec::new(), Vec::new());
    for k in 0..40u64 {
        let at = Time::from_nanos(k * 1_000_000_000 / 60) + Time::from_millis(t0);
        h.input.set_time(at);
        let (scene, out, _, _) = frame(&mut h, open.then_some(&d));
        let (o, s) = measure(&scene);
        let moving = out.repaint != Repaint::Idle;
        opacity.push((at - Time::from_millis(t0), o, moving));
        scale.push((at - Time::from_millis(t0), s, moving));
    }
    (opacity, scale)
}

fn replay(samples: &[(Time, f32, bool)]) -> impl FnMut(Time) -> (f32, bool) + '_ {
    move |t| {
        let s = samples
            .iter()
            .find(|(at, _, _)| *at == t)
            .unwrap_or_else(|| panic!("no sample at {t:?}"));
        (s.1, s.2)
    }
}

#[test]
fn open_reaches_opacity_one_and_scale_one_at_250_ms() {
    let (opacity, scale) = sample(MotionTier::Full, true);
    assert_eq!(opacity[0].1, 0.0, "starts transparent");
    assert!((scale[0].1 - 0.96).abs() < 1e-6, "starts at 0.96");
    let end = Time::from_millis(250);
    conformance_of(replay(&opacity), end, 1.0, 0.35).expect("opacity conforms");
    conformance_of(replay(&scale), end, 1.0, 0.35 * 0.04).expect("scale conforms");
    // The same row through the shared harness.
    conformance(&OPEN, &DARK.motion, MotionTier::Full, 0.0_f32, 1.0).expect("row conforms");
    assert_eq!(FRAME.as_nanos(), 16_666_666);
}

#[test]
fn reduced_open_is_a_fade_without_scale_at_the_fast_length() {
    let (opacity, scale) = sample(MotionTier::Reduced, true);
    conformance_of(replay(&opacity), Time::from_millis(180), 1.0, 0.35).expect("opacity");
    assert!(
        scale
            .iter()
            .all(|s| s.1 == 1.0 || s.1 == 0.96 && s.0 == Time::ZERO),
        "no scale: {scale:?}"
    );
    // Opacity strictly between 0 and 1 somewhere: a fade, not a snap.
    assert!(opacity.iter().any(|s| s.1 > 0.0 && s.1 < 1.0));
}

#[test]
fn off_opens_at_once() {
    let (opacity, scale) = sample(MotionTier::Off, true);
    assert!(opacity.iter().all(|s| s.1 == 1.0));
    // Only the open frame asks for one more frame, because focus moved in after it drew (D81).
    assert!(opacity[0].2 && opacity.iter().skip(1).all(|s| !s.2));
    assert!(scale.iter().all(|s| s.1 == 1.0));
}

#[test]
fn close_hides_at_180_120_and_0_ms() {
    for (tier, ms) in [
        (MotionTier::Full, 180u64),
        (MotionTier::Reduced, 120),
        (MotionTier::Off, 0),
    ] {
        let (opacity, scale) = sample(tier, false);
        let end = Time::from_millis(ms);
        if ms == 0 {
            // Off: gone at once; only the close frame asks for one more frame, because focus
            // returned after the page drew (D81).
            assert!(opacity.iter().all(|s| s.1 == 0.0), "{tier:?}");
            assert!(opacity[0].2 && opacity.iter().skip(1).all(|s| !s.2));
        } else {
            conformance_of(replay(&opacity), end, 0.0, 0.35)
                .unwrap_or_else(|e| panic!("{tier:?}: {e}"));
        }
        // Scale: 1 at the start; shrinks to 0.96 only when transforms apply (Full).
        if tier == MotionTier::Full {
            assert_eq!(scale[0].1, 1.0);
            let min = scale
                .iter()
                .take_while(|s| s.0 < end)
                .map(|s| s.1)
                .fold(1.0, f32::min);
            assert!((0.96..1.0).contains(&min), "{min}");
        } else {
            assert!(scale.iter().filter(|s| s.1 != 0.96).all(|s| s.1 == 1.0));
        }
        // After the end nothing is drawn and nothing moves (Off's close frame is the D81
        // catch-up checked above).
        assert!(
            opacity
                .iter()
                .filter(|s| s.0 >= end && (ms > 0 || s.0 > end))
                .all(|s| s.1 == 0.0 && !s.2),
            "{tier:?}"
        );
    }
    conformance(&CLOSE, &DARK.motion, MotionTier::Full, 1.0_f32, 0.0).expect("row conforms");
}

#[test]
fn a_closing_dialog_draws_the_apps_body_and_fades_it_with_the_card() {
    let d = the_dialog(Cancel::Default, 100.0);
    let mut h = opened(&d);
    // The body is the only shape of the Modal layer with radius 4 (at most; the card scales it).
    let find = |scene: &Scene| {
        scene
            .merged(LayerId::Modal)
            .geometry
            .under
            .iter()
            .find(|s| s.radii[0] > 3.0 && s.radii[0] <= 4.0)
            .copied()
    };
    h.at_ms(2000);
    let (open_scene, _, _, _) = frame(&mut h, Some(&d));
    let open_body = find(&open_scene).expect("the body is drawn while open");
    // The app stops calling: 90 ms into the close the body is still there, partly faded.
    h.at_ms(3000);
    let (first, out, _, _) = frame(&mut h, None);
    assert_eq!(out.repaint, Repaint::Now);
    let first_body = find(&first).expect("the body draws in the first closing frame");
    assert!((first_body.color[3] - open_body.color[3]).abs() < 1e-6);
    h.at_ms(3090);
    let (mid, out, _, _) = frame(&mut h, None);
    assert_eq!(out.repaint, Repaint::Now);
    let mid_body = find(&mid).expect("the body draws mid-close");
    assert!(mid_body.color[3] < open_body.color[3] && mid_body.color[3] > 0.0);
    // And the card shrinks about its centre.
    assert!(mid_body.rect[2] < open_body.rect[2]);
    h.at_ms(3180);
    let (end, out, _, _) = frame(&mut h, None);
    assert_eq!(out.repaint, Repaint::Idle);
    assert!(end.merged(LayerId::Modal).geometry.under.is_empty());
}

#[test]
fn a_closing_dialog_is_inert() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened(&d);
    h.at_ms(3000);
    frame(&mut h, None);
    // During the close the page button is live again: a click on it lands.
    h.input.pointer_to(30.0, 30.0);
    h.at_ms(3010);
    frame(&mut h, None);
    h.input.press(P);
    h.at_ms(3020);
    frame(&mut h, None);
    h.input.release(P);
    h.at_ms(3030);
    let (_, _, _, b) = frame(&mut h, None);
    assert!(b.clicked, "a closing modal captures nothing");
}

// The variants: `Ui::dialog_with`.

const FORM: DialogOptions = DialogOptions {
    width: DialogWidth::Form,
    scrim_blur: false,
    confirm_enabled: true,
};
const BLUR: DialogOptions = DialogOptions {
    width: DialogWidth::Confirm,
    scrim_blur: true,
    confirm_enabled: true,
};
const PLAIN: DialogOptions = DialogOptions {
    width: DialogWidth::Confirm,
    scrim_blur: false,
    confirm_enabled: true,
};
const OFF: DialogOptions = DialogOptions {
    width: DialogWidth::Confirm,
    scrim_blur: false,
    confirm_enabled: false,
};

/// [`frame`] with options, and the body's `(x, y, w)` when it ran.
fn frame_with(
    h: &mut UiHarness,
    d: Option<(&Dialog<'_>, &DialogOptions)>,
) -> (Scene, FrameOutput, DialogResponse, Option<(f32, f32, f32)>) {
    let mut report = DialogResponse::default();
    let mut at = None;
    let (scene, out) = h.frame(|ui| {
        page(ui.painter());
        ui.button("page", PAGE_BTN, "Page", ButtonKind::Secondary);
        if let Some((d, o)) = d {
            report = ui.dialog_with("d", d, o, &glass(), |ui: &mut Ui<'_>, x, y, w| {
                at = Some((x, y, w));
                if d.extra_h > 0.0 {
                    body(ui.painter(), x, y, w);
                }
            });
        }
    });
    (scene, out, report, at)
}

/// A harness with the dialog open under `o` and settled.
fn opened_with(d: &Dialog<'_>, o: &DialogOptions) -> UiHarness {
    let mut h = UiHarness::new(WIN);
    frame_with(&mut h, Some((d, o)));
    h.at_ms(1000);
    let (_, out, _, _) = frame_with(&mut h, Some((d, o)));
    assert_eq!(out.repaint, Repaint::Idle);
    h
}

/// [`run`] with options.
fn run_with(
    h: &mut UiHarness,
    d: &Dialog<'_>,
    o: &DialogOptions,
    from_ms: u64,
    frames: u64,
) -> (Tally, FrameOutput) {
    let mut t = Tally {
        confirmed: 0,
        cancelled: 0,
    };
    let mut last = None;
    for i in 0..frames {
        h.at_ms(from_ms + i * 10);
        let (_, out, r, _) = frame_with(h, Some((d, o)));
        t.confirmed += u32::from(r.confirmed);
        t.cancelled += u32::from(r.cancelled);
        last = Some(out);
    }
    (t, last.expect("a frame ran"))
}

fn button_node<'o>(out: &'o FrameOutput, name: &str) -> &'o lune_ui_core::access::Node {
    out.tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Button && n.name == name)
        .unwrap_or_else(|| panic!("no button {name}"))
}

#[test]
fn the_default_options_draw_what_dialog_draws() {
    assert_eq!(
        DialogOptions::default(),
        DialogOptions {
            width: DialogWidth::Confirm,
            scrim_blur: true,
            confirm_enabled: true
        }
    );
    assert_eq!(DialogWidth::Confirm.px(), 448.0);
    assert_eq!(DialogWidth::Form.px(), 600.0);
    for extra in [0.0, 100.0] {
        let d = the_dialog(Cancel::Default, extra);
        let mut a = UiHarness::new(WIN);
        let mut b = UiHarness::new(WIN);
        for ms in [0, 125, 1000] {
            a.at_ms(ms);
            b.at_ms(ms);
            let (sa, _, _, _) = frame(&mut a, Some(&d));
            let (sb, _, _, _) = frame_with(&mut b, Some((&d, &DialogOptions::default())));
            assert_eq!(sa, sb, "extra {extra} at {ms} ms");
        }
    }
}

#[test]
fn the_form_card_is_600_wide_and_centred_and_holds_two_columns() {
    let d = the_dialog(Cancel::Default, 120.0);
    let mut h = opened_with(&d, &FORM);
    let (scene, out, _, at) = frame_with(&mut h, Some((&d, &FORM)));
    let card = card_bounds(&out);
    assert_eq!(card.w, 600.0);
    assert_eq!(card.x, (WIN.0 - 600.0) / 2.0, "centred");
    let (x, _, w) = at.expect("the body ran");
    assert_eq!(
        (x, w),
        (card.x + 24.0, 552.0),
        "the body is the card less 24 each side"
    );
    // The footer is as wide as the card and the buttons stay right-aligned in it.
    let modal = &scene.merged(LayerId::Modal);
    let footer = modal
        .geometry
        .under
        .iter()
        .find(|s| s.radii == [0.0, 0.0, 8.0, 8.0])
        .expect("footer");
    assert_eq!(footer.rect[2], 600.0);
    let confirm = button_node(&out, "Discard").bounds;
    assert_eq!(confirm.right(), card.right() - 24.0);
    // Two columns of the body's width fit side by side inside it.
    let col = (w - 16.0) / 2.0;
    assert!(x + col + 16.0 + col <= card.right() - 24.0 + 1e-3);
    // A confirm dialog is still 448.
    let mut c = opened_with(&d, &DialogOptions::default());
    let (_, out, _, _) = frame_with(&mut c, Some((&d, &DialogOptions::default())));
    assert_eq!(card_bounds(&out).w, 448.0);
}

#[test]
fn the_form_scale_two_probes() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    frame_with(&mut h, Some((&d, &FORM)));
    h.at_ms(1000);
    let (scene, _, _, _) = frame_with(&mut h, Some((&d, &FORM)));
    let modal = &scene.merged(LayerId::Modal);
    let card = modal.geometry.under.iter().find(|s| s.is_frost());
    let card = card.expect("the card");
    assert_eq!(card.rect[2], 1200.0);
    assert_eq!(card.radii, [16.0; 4]);
}

#[test]
fn every_close_path_of_the_form_fires_once() {
    let d = the_dialog(Cancel::Default, 120.0);
    for (name, want) in [("Discard", (1, 0)), ("Cancel", (0, 1))] {
        let mut h = opened_with(&d, &FORM);
        let (_, out) = run_with(&mut h, &d, &FORM, 1100, 1);
        let c = button_node(&out, name).bounds;
        h.input.pointer_to(c.x + c.w / 2.0, c.y + c.h / 2.0);
        let (a, _) = run_with(&mut h, &d, &FORM, 2000, 2);
        h.input.press(P);
        let (b, _) = run_with(&mut h, &d, &FORM, 2100, 2);
        h.input.release(P);
        let (e, _) = run_with(&mut h, &d, &FORM, 2200, 3);
        let got = (
            a.confirmed + b.confirmed + e.confirmed,
            a.cancelled + b.cancelled + e.cancelled,
        );
        assert_eq!(got, want, "{name}");
    }
    let mut h = opened_with(&d, &FORM);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (t, _) = run_with(&mut h, &d, &FORM, 1100, 4);
    assert_eq!((t.confirmed, t.cancelled), (0, 1), "Escape");
    let mut h = opened_with(&d, &FORM);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (t, _) = run_with(&mut h, &d, &FORM, 1100, 4);
    assert_eq!((t.confirmed, t.cancelled), (1, 0), "Enter");
}

/// The blur request of a frame, or `None`.
fn blur_of(scene: &Scene) -> Option<lune_ui_core::scene::BackdropRequest> {
    scene.backdrops().next().copied()
}

#[test]
fn the_scrim_blur_asks_for_the_window_at_sigma_eight_on_the_modal_layer() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut plain = opened_with(&d, &PLAIN);
    let (scene, _, _, _) = frame_with(&mut plain, Some((&d, &PLAIN)));
    assert_eq!(blur_of(&scene), None, "no blur with `scrim_blur: false`");
    let mut dflt = opened_with(&d, &DialogOptions::default());
    let (scene, _, _, _) = frame_with(&mut dflt, Some((&d, &DialogOptions::default())));
    assert!(blur_of(&scene).is_some(), "the blur is the default");

    let mut h = opened_with(&d, &BLUR);
    let (scene, _, _, _) = frame_with(&mut h, Some((&d, &BLUR)));
    let b = blur_of(&scene).expect("the blur is asked for");
    assert_eq!(scene.merged(LayerId::Modal).backdrops, [b], "on the dialog");
    assert_eq!((b.sigma, b.saturation), (8.0, 1.0));
    assert_eq!(b.rect, [0.0, 0.0, WIN.0, WIN.1]);
    assert_eq!(b.radius, 0.0);
    assert_eq!(b.alpha, 1.0);
    // The scrim keeps its colour over the blur.
    let scrim = scene.merged(LayerId::Modal).geometry.under[0];
    assert_eq!(scrim.color, premul(DARK.dialog_backdrop));
}

#[test]
fn the_blur_fades_with_the_scrim_opening_and_closing() {
    let d = the_dialog(Cancel::Default, 0.0);
    let rest = premul(DARK.dialog_backdrop)[3];
    let fade_of = |scene: &Scene| {
        let b = blur_of(scene).expect("blur asked for").alpha;
        let s = scene.merged(LayerId::Modal).geometry.under[0].color[3] / rest;
        (b, s)
    };
    // Opening, 125 ms in (half way).
    let mut h = UiHarness::new(WIN);
    frame_with(&mut h, Some((&d, &BLUR)));
    h.at_ms(125);
    let (scene, _, _, _) = frame_with(&mut h, Some((&d, &BLUR)));
    let (b, s) = fade_of(&scene);
    assert!(b > 0.0 && b < 1.0, "opening: {b}");
    assert!(
        (b - s).abs() < 1e-5,
        "the blur follows the scrim: {b} vs {s}"
    );
    // Closing: the app stops calling; the exit painter asks for it too, faded.
    let mut h = opened_with(&d, &BLUR);
    h.at_ms(3000);
    let (scene, _, _, _) = frame_with(&mut h, None);
    let (b, s) = fade_of(&scene);
    assert!(
        (b - 1.0).abs() < 1e-5 && (s - 1.0).abs() < 1e-5,
        "first closing frame"
    );
    h.at_ms(3090);
    let (scene, out, _, _) = frame_with(&mut h, None);
    assert_eq!(out.repaint, Repaint::Now);
    let (b, s) = fade_of(&scene);
    assert!(b > 0.0 && b < 1.0, "closing: {b}");
    assert!((b - s).abs() < 1e-5, "{b} vs {s}");
    assert_eq!(scene.merged(LayerId::Modal).backdrops.len(), 1);
    h.at_ms(3180);
    let (scene, _, _, _) = frame_with(&mut h, None);
    assert_eq!(blur_of(&scene), None, "gone with the dialog");
}

fn menu_rows() -> [MenuRow<'static>; 2] {
    [
        MenuRow::Item {
            glyph: None,
            text: "one",
            shortcut: "",
        },
        MenuRow::Item {
            glyph: None,
            text: "two",
            shortcut: "",
        },
    ]
}

/// One frame of `o`'s dialog with a glass menu in its body.
fn menu_frame(h: &mut UiHarness, o: &DialogOptions) -> Scene {
    h.frame(|ui| {
        page(ui.painter());
        let d = the_dialog(Cancel::Default, 160.0);
        ui.dialog_with("d", &d, o, &glass(), |ui, x, y, _| {
            ui.menu("m", &menu_rows(), (x, y + 20.0));
        });
    })
    .0
}

/// Whether the Raised layer holds the menu's glass fill (radius 9, `menu_glass`).
fn raised_menu_fill(scene: &Scene) -> bool {
    let want = premul(DARK.menu_glass);
    scene
        .merged(LayerId::Raised)
        .geometry
        .under
        .iter()
        .any(|s| s.radii == [9.0; 4] && s.color == want)
}

#[test]
fn a_glass_menu_above_a_blurred_scrim_keeps_a_blur_of_its_own() {
    let mut h = UiHarness::new(WIN);
    menu_frame(&mut h, &BLUR);
    h.at_ms(1000);
    menu_frame(&mut h, &BLUR);
    let scene = menu_frame(&mut h, &BLUR);
    let scrim = &scene.merged(LayerId::Modal).backdrops;
    assert_eq!(scrim.len(), 1);
    assert_eq!(scrim[0].sigma, 8.0, "the scrim's sigma");
    let menu = &scene.merged(LayerId::Raised).backdrops;
    assert_eq!(menu.len(), 1);
    assert_eq!(menu[0].sigma, 0.0, "the frame's sigma, for the menu");
    assert!(
        raised_menu_fill(&scene),
        "the menu still draws its fill, on Raised"
    );
    // Without the scrim blur the menu's own blur is the one, on its own layer.
    let mut h = UiHarness::new(WIN);
    menu_frame(&mut h, &PLAIN);
    h.at_ms(1000);
    menu_frame(&mut h, &PLAIN);
    let scene = menu_frame(&mut h, &PLAIN);
    assert_eq!(scene.backdrops().count(), 1, "the menu's blur");
    let menu = &scene.merged(LayerId::Raised).backdrops;
    assert_eq!(menu[0].sigma, 0.0, "the frame's sigma");
    assert!(raised_menu_fill(&scene));
}

#[test]
fn a_disabled_confirm_is_the_buttons_own_disabled_node_and_keeps_its_place() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut on = opened_with(&d, &DialogOptions::default());
    let (_, enabled, _, _) = frame_with(&mut on, Some((&d, &DialogOptions::default())));
    let mut h = opened_with(&d, &OFF);
    let (_, out, _, _) = frame_with(&mut h, Some((&d, &OFF)));
    let confirm = button_node(&out, "Discard");
    assert!(confirm.states.contains(State::Disabled));
    assert!(!confirm.states.contains(State::Focusable));
    assert_eq!(confirm.actions, Actions::default());
    assert_eq!(disabled_violations(&out.tree, confirm.id), []);
    assert_eq!(confirm.bounds, button_node(&enabled, "Discard").bounds);
    // Cancel is untouched.
    let cancel = button_node(&out, "Cancel");
    assert!(cancel.states.contains(State::Focusable));
    assert!(cancel.actions.contains(Action::Default));
    // The dialog does not offer the default action.
    let node = out.tree.get(dialog_id()).expect("dialog");
    assert!(!node.actions.contains(Action::Default));
    assert!(node.actions.contains(Action::Dismiss));
}

#[test]
fn a_disabled_confirm_draws_at_the_disabled_opacity_open_and_closing() {
    let d = the_dialog(Cancel::Default, 0.0);
    let confirm_fill = |scene: &Scene, out: &FrameOutput| {
        let r = button_node(out, "Discard").bounds;
        scene
            .merged(LayerId::Modal)
            .geometry
            .under
            .iter()
            .find(|s| s.rect == [r.x, r.y, r.w, r.h])
            .map(|s| s.color[3])
            .expect("the confirm button's fill")
    };
    let mut on = opened_with(&d, &DialogOptions::default());
    on.input.key_down(Key::Named(NamedKey::Tab), None);
    let (scene, out, _, _) = frame_with(&mut on, Some((&d, &DialogOptions::default())));
    let full = confirm_fill(&scene, &out);
    let mut h = opened_with(&d, &OFF);
    let (scene, out, _, _) = frame_with(&mut h, Some((&d, &OFF)));
    let dim = confirm_fill(&scene, &out);
    assert!((dim / full - 0.4).abs() < 1e-4, "{dim} of {full}");
    // The first closing frame draws it the same.
    h.at_ms(3000);
    let (scene, _, _, _) = frame_with(&mut h, None);
    let r = button_node(&out, "Discard").bounds;
    let closing = scene
        .merged(LayerId::Modal)
        .geometry
        .under
        .iter()
        .find(|s| s.rect == [r.x, r.y, r.w, r.h])
        .map(|s| s.color[3])
        .expect("the closing confirm");
    assert!((closing - dim).abs() < 1e-4, "{closing} vs {dim}");
}

#[test]
fn a_disabled_confirm_opens_focus_on_cancel_or_nowhere() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened_with(&d, &OFF);
    let (_, out, _, _) = frame_with(&mut h, Some((&d, &OFF)));
    assert_eq!(out.focused, Some(button_node(&out, "Cancel").id));
    // Enter then cancels, and never confirms.
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (t, _) = run_with(&mut h, &d, &OFF, 1100, 4);
    assert_eq!((t.confirmed, t.cancelled), (0, 1));

    let alone = the_dialog(Cancel::None, 0.0);
    let mut h = opened_with(&alone, &OFF);
    let (_, out, _, _) = frame_with(&mut h, Some((&alone, &OFF)));
    assert_eq!(out.focused, None, "no tab stop, no focus");
    assert!(
        button_node(&out, "Discard")
            .states
            .contains(State::Disabled)
    );
}

/// Opens enabled (focus on confirm), turns confirm off, presses `keys` with a Global binding on
/// each and returns (the dialog's tally, the commands that fired).
fn keys_on_disabled_confirm(keys: &[NamedKey]) -> (u32, u32, usize) {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = UiHarness::new(WIN);
    for k in keys {
        h.shortcuts.add(Binding::new(
            Scope::Global,
            Chord::key(Key::Named(*k)),
            "go",
        ));
    }
    let on = DialogOptions::default();
    frame_with(&mut h, Some((&d, &on)));
    h.at_ms(1000);
    let (_, out, _, _) = frame_with(&mut h, Some((&d, &on)));
    assert_eq!(
        out.focused,
        Some(button_node(&out, "Discard").id),
        "confirm has focus"
    );
    run_with(&mut h, &d, &OFF, 1100, 2);
    for k in keys {
        h.input.key_down(Key::Named(*k), None);
    }
    let (t, out) = run_with(&mut h, &d, &OFF, 1200, 1);
    (t.confirmed, t.cancelled, out.fired.len())
}

#[test]
fn enter_and_space_on_a_disabled_confirm_do_nothing_and_reach_no_shortcut() {
    for keys in [
        &[NamedKey::Enter][..],
        &[NamedKey::Space],
        &[NamedKey::Enter, NamedKey::Space],
    ] {
        assert_eq!(keys_on_disabled_confirm(keys), (0, 0, 0), "{keys:?}");
    }
}

#[test]
fn a_click_and_the_default_action_on_a_disabled_confirm_do_nothing() {
    let d = the_dialog(Cancel::Default, 0.0);
    let mut h = opened_with(&d, &OFF);
    let (_, out) = run_with(&mut h, &d, &OFF, 1100, 1);
    let c = button_node(&out, "Discard").bounds;
    h.input.pointer_to(c.x + c.w / 2.0, c.y + c.h / 2.0);
    let (a, _) = run_with(&mut h, &d, &OFF, 2000, 2);
    h.input.press(P);
    let (b, _) = run_with(&mut h, &d, &OFF, 2100, 2);
    h.input.release(P);
    let (e, _) = run_with(&mut h, &d, &OFF, 2200, 3);
    let confirmed = a.confirmed + b.confirmed + e.confirmed;
    let cancelled = a.cancelled + b.cancelled + e.cancelled;
    assert_eq!((confirmed, cancelled), (0, 0), "a click on it");

    h.input.access(AccessAction {
        target: dialog_id(),
        action: Action::Default,
        value: None,
    });
    let (t, _) = run_with(&mut h, &d, &OFF, 3000, 3);
    assert_eq!((t.confirmed, t.cancelled), (0, 0), "the default action");
    // Dismiss still works.
    h.input.access(AccessAction {
        target: dialog_id(),
        action: Action::Dismiss,
        value: None,
    });
    let (t, _) = run_with(&mut h, &d, &OFF, 4000, 3);
    assert_eq!((t.confirmed, t.cancelled), (0, 1), "dismiss");
}

// A dialog over a dialog.

const INNER_MESSAGE: &str = "The document has unsaved changes. Closing it will discard them.";

/// What a nested bed reports.
#[derive(Debug, Default, Clone, Copy)]
struct Nest {
    outer: (u32, u32),
    inner: (u32, u32),
}

/// One frame of an outer dialog whose body opens `inner_open`'s dialog: same size and place as the
/// outer, so their buttons overlap.
fn nest_frame(h: &mut UiHarness, inner_open: bool, tally: &mut Nest) -> (Scene, FrameOutput) {
    h.frame(|ui| {
        page(ui.painter());
        let outer = the_dialog(Cancel::Default, 0.0);
        let mut inner_r = DialogResponse::default();
        let r = ui.dialog("d", &outer, &glass(), |ui, _, _, _| {
            if inner_open {
                let inner = Dialog {
                    title: "Sure",
                    message: INNER_MESSAGE,
                    extra_h: 0.0,
                    confirm: "Yes",
                    cancel: Cancel::Label("Keep"),
                };
                inner_r = ui.dialog("inner", &inner, &glass(), |_, _, _, _| {});
            }
        });
        tally.outer.0 += u32::from(r.confirmed);
        tally.outer.1 += u32::from(r.cancelled);
        tally.inner.0 += u32::from(inner_r.confirmed);
        tally.inner.1 += u32::from(inner_r.cancelled);
    })
}

fn nested_bed() -> (UiHarness, Nest, FrameOutput) {
    let mut h = UiHarness::new(WIN);
    let mut t = Nest::default();
    nest_frame(&mut h, false, &mut t);
    h.at_ms(500);
    nest_frame(&mut h, false, &mut t);
    nest_frame(&mut h, true, &mut t);
    h.at_ms(1500);
    nest_frame(&mut h, true, &mut t);
    let (_, out) = nest_frame(&mut h, true, &mut t);
    (h, t, out)
}

fn nest_run(h: &mut UiHarness, t: &mut Nest, from: u64, frames: u64) {
    for i in 0..frames {
        h.at_ms(from + i * 10);
        nest_frame(h, true, t);
    }
}

#[test]
fn a_nested_dialog_draws_on_raised_over_the_dialog() {
    let (_, _, out) = nested_bed();
    let mut h = UiHarness::new(WIN);
    let mut t = Nest::default();
    nest_frame(&mut h, false, &mut t);
    h.at_ms(500);
    let (rest, _) = nest_frame(&mut h, false, &mut t);
    assert!(rest.merged(LayerId::Raised).geometry.under.is_empty());
    nest_frame(&mut h, true, &mut t);
    h.at_ms(1500);
    nest_frame(&mut h, true, &mut t);
    let (scene, _) = nest_frame(&mut h, true, &mut t);
    let raised = &scene.merged(LayerId::Raised);
    assert!(
        raised.geometry.under.iter().any(|s| s.is_frost()),
        "the inner card is on Raised"
    );
    assert!(!raised.texts.is_empty());
    // Both are modal dialogs in the tree, the inner one a node of its own.
    let dialogs = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Dialog)
        .count();
    assert_eq!(dialogs, 2);
}

#[test]
fn confirm_and_cancel_inside_a_nested_dialog_reach_only_the_top() {
    for (name, want) in [("Yes", ((0, 0), (1, 0))), ("Keep", ((0, 0), (0, 1)))] {
        let (mut h, mut t, out) = nested_bed();
        let target = button_node(&out, name).bounds;
        let (x, y) = (target.x + target.w / 2.0, target.y + target.h / 2.0);
        // The outer dialog's own confirm sits under that point: the setup is a real overlap.
        let under = out
            .tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::Button && (n.name == "Discard" || n.name == "Cancel"))
            .any(|n| n.bounds.contains((x, y)));
        assert!(under, "{name}: an outer button is under the point");
        h.input.pointer_to(x, y);
        nest_run(&mut h, &mut t, 2000, 2);
        h.input.press(P);
        nest_run(&mut h, &mut t, 2100, 2);
        h.input.release(P);
        nest_run(&mut h, &mut t, 2200, 3);
        assert_eq!((t.outer, t.inner), want, "{name}");
    }
}

#[test]
fn escape_and_a_scrim_click_close_the_top_dialog_only() {
    let (mut h, mut t, _) = nested_bed();
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    nest_run(&mut h, &mut t, 2000, 3);
    assert_eq!((t.outer, t.inner), ((0, 0), (0, 1)), "Escape");

    let (mut h, mut t, _) = nested_bed();
    h.input.pointer_to(30.0, 30.0);
    nest_run(&mut h, &mut t, 2000, 2);
    h.input.press(P);
    nest_run(&mut h, &mut t, 2100, 2);
    h.input.release(P);
    nest_run(&mut h, &mut t, 2200, 3);
    assert_eq!(
        (t.outer, t.inner),
        ((0, 0), (0, 1)),
        "the scrim of the top one"
    );
}

#[test]
fn the_dialog_under_works_again_when_the_nested_one_is_gone() {
    let (mut h, mut t, _) = nested_bed();
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    nest_run(&mut h, &mut t, 2000, 3);
    // The app stops calling the inner dialog; its close runs out, then Escape reaches the outer.
    for i in 0..30 {
        h.at_ms(3000 + i * 10);
        nest_frame(&mut h, false, &mut t);
    }
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    h.at_ms(4000);
    nest_frame(&mut h, false, &mut t);
    h.at_ms(4010);
    nest_frame(&mut h, false, &mut t);
    assert_eq!(t.outer, (0, 1), "the outer dialog closes by itself");
}

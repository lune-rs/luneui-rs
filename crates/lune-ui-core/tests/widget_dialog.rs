//! The dialog (C4 retrofit, `overlay.dialog` acceptance): rest identity with the Painter API,
//! behaviour (Cancel, confirm, Escape, Enter, scrim click, Tab trap), pixel probes, the open and
//! close motion sampled at 60 Hz, exit retention with the app's body, the node.

use lune_ui_core::access::{AccessAction, Action, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{FRAME, MotionTier, Time, conformance, conformance_of};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Response, Ui, WidgetId};
use lune_ui_core::widgets::button::{ButtonKind, button};
use lune_ui_core::widgets::dialog::{
    CLOSE, Cancel, Dialog, DialogGlass, DialogHits, DialogResponse, DialogSpec, OPEN, dialog,
};

const WIN: (f32, f32) = (800.0, 600.0);
const P: PointerButton = PointerButton::Primary;
const PAGE_BTN: Rect = Rect::new(20.0, 20.0, 100.0, 32.0);
const BODY_FILL: [u8; 4] = [200, 30, 30, 255];

fn glass() -> DialogGlass {
    DialogGlass {
        panel: [0.1, 0.1, 0.12, 1.0],
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
            scene,
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
    let modal = &scene.layers[LayerId::Modal as usize];
    assert!(modal.geometry.acrylic.iter().all(|a| a.opacity == 1.0));
    let g = &modal.geometry;
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
    let modal = &scene.layers[LayerId::Modal as usize];
    // Scrim: the window, #00000080.
    let scrim = modal.geometry.under[0];
    assert_eq!(scrim.rect, [0.0, 0.0, WIN.0 * 2.0, WIN.1 * 2.0]);
    assert_eq!(DARK.dialog_backdrop, [0, 0, 0, 0x80]);
    assert_eq!(scrim.color, premul([0, 0, 0, 0x80]));
    // Card: 896 wide, corner radius 16.
    let card = modal.geometry.acrylic[0];
    assert_eq!(card.shape.rect[2], 896.0);
    assert_eq!(card.shape.radii, [16.0; 4]);
    assert_eq!(card.opacity, 1.0);
    // Footer: 136 high, bottom corners 16.
    let footer = modal
        .geometry
        .under
        .iter()
        .find(|s| s.radii == [0.0, 0.0, 16.0, 16.0])
        .expect("footer");
    assert_eq!(footer.rect[3], 136.0);
    assert_eq!(footer.rect[2], 896.0);
    // Two buttons, 64 high.
    let buttons: Vec<_> = modal
        .geometry
        .under
        .iter()
        .filter(|s| s.radii == [12.0; 4])
        .collect();
    assert_eq!(buttons.len(), 2);
    assert!(buttons.iter().all(|s| s.rect[3] == 64.0));
}

/// The sampler of a dialog's visibility on the 60 Hz grid: the scrim's alpha as a fraction of its
/// rest alpha (0 when nothing is drawn) and the card's scale (the title text's glyph scale; 0.96
/// when nothing is drawn yet).
fn measure(scene: &Scene) -> (f32, f32) {
    let modal = &scene.layers[LayerId::Modal as usize];
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
    assert!(opacity.iter().all(|s| s.1 == 1.0 && !s.2));
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
        conformance_of(replay(&opacity), end, 0.0, 0.35)
            .unwrap_or_else(|e| panic!("{tier:?}: {e}"));
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
        // After the end nothing is drawn and nothing moves.
        assert!(
            opacity
                .iter()
                .filter(|s| s.0 >= end)
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
        scene.layers[LayerId::Modal as usize]
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
    assert!(
        end.layers[LayerId::Modal as usize]
            .geometry
            .under
            .is_empty()
    );
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

//! The badge (`widget.badge` acceptance): 18 px high, radius 8, a one px border in the tone's 44,
//! 11 px Medium text on a 16 px line, an optional leading dot, static, a `StaticText` node.

use lune_ui_core::access::{Action, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{Key, NamedKey, Repaint};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::badge;
use lune_ui_core::widgets::tone::Tone;

const WIN: (f32, f32) = (300.0, 100.0);
const TONES: [Tone; 5] = [Tone::Neutral, Tone::Ok, Tone::Warn, Tone::Err, Tone::Data];

fn painted(scale: f32, tone: Tone, dot: bool) -> (Scene, Rect) {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(scale, win, &mut text, None, DARK);
    let r = badge::paint(&mut p, (10.0, 20.0), "added", tone, dot);
    (p.scene, r)
}

fn medium_spec(tone: Tone) -> Spec {
    let style = Style {
        line: 16.0,
        ..Style::ui(11.0, tone.text(&DARK)).medium()
    };
    Spec::plain("added", style)
}

#[test]
fn at_scale_2_the_badge_is_36_px_high_with_a_16_px_radius_and_a_2_px_border() {
    let (scene, r) = painted(2.0, Tone::Ok, false);
    let g = &scene.layers[0].geometry;
    assert_eq!(g.under.len(), 2, "fill and ring");
    let (fill, ring) = (g.under[0], g.under[1]);
    assert_eq!(fill.rect[3], 36.0);
    assert_eq!(fill.radii, [16.0; 4]);
    assert_eq!(fill.rect[2], r.w * 2.0);
    assert_eq!(ring.rect, fill.rect);
    assert_eq!(ring.params[0], 1.0, "a ring");
    assert_eq!(ring.params[1], 2.0, "2 px border");
    assert_eq!(r.h, 18.0);
}

#[test]
fn every_tone_uses_the_one_recipe() {
    for tone in TONES {
        let (scene, _) = painted(1.0, tone, false);
        let g = &scene.layers[0].geometry;
        assert_eq!(g.under[0].color, premul(tone.fill(&DARK)), "{tone:?} fill");
        assert_eq!(
            g.under[1].color,
            premul(tone.border(&DARK)),
            "{tone:?} border"
        );
        let texts = &scene.layers[0].texts;
        assert_eq!(texts.len(), 1);
        assert_eq!(
            texts[0].key.0,
            medium_spec(tone).cache_key(1.0),
            "{tone:?} text in the tone, Medium, 16 px line"
        );
    }
    // Fill at 1A, border at 44, in the tone; data text is data_hi; neutral is the overlay.
    assert_eq!(Tone::Ok.fill(&DARK)[3], 0x1A);
    assert_eq!(Tone::Ok.border(&DARK)[3], 0x44);
    assert_eq!(Tone::Data.text(&DARK), DARK.data_hi);
    assert_eq!(Tone::Neutral.fill(&DARK), DARK.surface_overlay);
    assert_eq!(Tone::Neutral.border(&DARK), DARK.border_subtle);
    assert_eq!(Tone::Neutral.text(&DARK), DARK.fg_secondary);
}

#[test]
fn the_text_sits_7_px_in_and_centred_on_the_16_px_line() {
    let (scene, r) = painted(1.0, Tone::Ok, false);
    let t = &scene.layers[0].texts[0];
    assert_eq!((t.x, t.y), (r.x + 7.0, r.y + 1.0));
}

#[test]
fn width_counts_padding_text_and_the_dot() {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win, &mut text, None, DARK);
    let plain = badge::width(&mut p, "added", false);
    let dotted = badge::width(&mut p, "added", true);
    let text_w = p.measure(&medium_spec(Tone::Neutral)).0;
    assert_eq!(plain, 7.0 + text_w + 7.0);
    assert_eq!(dotted, plain + 8.0 + 6.0);
    let (_, r) = painted(1.0, Tone::Ok, true);
    assert_eq!(r.w, dotted);
}

#[test]
fn a_leading_dot_adds_one_circle_in_the_tone_and_moves_the_text() {
    let (scene, r) = painted(1.0, Tone::Warn, true);
    let g = &scene.layers[0].geometry;
    assert_eq!(g.under.len(), 3, "fill, ring, dot");
    let dot = g.under[2];
    assert_eq!(dot.rect, [r.x + 7.0, r.y + 5.0, 8.0, 8.0]);
    assert_eq!(dot.color, premul(DARK.warn));
    let t = &scene.layers[0].texts[0];
    assert_eq!(t.x, r.x + 7.0 + 8.0 + 6.0);
}

#[test]
fn the_ui_badge_is_static_text_named_by_its_text_and_takes_no_input() {
    let mut h = UiHarness::new(WIN);
    let mut r = Rect::new(0.0, 0.0, 0.0, 0.0);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let (_, out) = h.frame(|ui| r = ui.badge("b", (10.0, 20.0), "added", Tone::Ok, false));
    assert_eq!(out.tree.nodes.len(), 1);
    let n = &out.tree.nodes[0];
    assert_eq!(n.role, Role::StaticText);
    assert_eq!(n.name, "added");
    assert_eq!(n.value, Some(Value::Text("added".to_owned())));
    assert_eq!(n.bounds, r);
    assert!(!n.states.contains(State::Focusable));
    assert!(!n.actions.contains(Action::Default));
    assert!(out.focused.is_none(), "Tab finds nothing to focus");
    h.at_ms(500);
    let (_, out) = h.frame(|ui| {
        ui.badge("b", (10.0, 20.0), "added", Tone::Ok, false);
    });
    assert_eq!(out.repaint, Repaint::Idle);
}

#[test]
fn the_ui_badge_draws_what_paint_draws() {
    let mut h = UiHarness::new(WIN);
    let (scene, _) = h.frame(|ui| {
        ui.badge("b", (10.0, 20.0), "added", Tone::Err, true);
    });
    let (want, _) = painted(1.0, Tone::Err, true);
    assert_eq!(scene, want);
}

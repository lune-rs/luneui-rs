//! The status dot (`widget.status-dot` acceptance): the 8 px circle in each tone, the word that
//! always comes with it, the hidden image node, and the rule that a dot never stands alone.

use lune_ui_core::access::{Action, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::Repaint;
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::status_dot;
use lune_ui_core::widgets::tone::Tone;

const WIN: (f32, f32) = (300.0, 100.0);
const TONES: [Tone; 5] = [Tone::Ok, Tone::Warn, Tone::Err, Tone::Data, Tone::Paused];

fn painted(scale: f32, tone: Tone) -> Scene {
    let win = Rect::new(0.0, 0.0, WIN.0, WIN.1);
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(scale, win, &mut text, None, DARK);
    status_dot::paint(&mut p, (20.0, 30.0), tone);
    p.scene
}

#[test]
fn every_tone_is_a_16_px_circle_at_scale_2_in_the_tone_colour() {
    for tone in TONES {
        let scene = painted(2.0, tone);
        let g = &scene.layers[0].geometry;
        assert_eq!(g.under.len(), 1, "{tone:?}: one shape");
        let s = g.under[0];
        assert_eq!(s.rect, [32.0, 52.0, 16.0, 16.0], "{tone:?}");
        assert_eq!(s.radii, [8.0; 4], "{tone:?}: a circle");
        assert_eq!(s.color, premul(tone.color(&DARK)), "{tone:?}");
        assert!(scene.layers.iter().all(|l| l.texts.is_empty()));
    }
}

#[test]
fn the_tones_are_the_token_colours() {
    let want = [DARK.ok, DARK.warn, DARK.err, DARK.data, DARK.paused];
    for (tone, c) in TONES.into_iter().zip(want) {
        assert_eq!(tone.color(&DARK), c, "{tone:?}");
    }
}

fn drawn(word: &str) -> (Scene, lune_ui_core::access::SemanticTree, Rect, Repaint) {
    let mut h = UiHarness::new(WIN);
    let mut r = Rect::new(0.0, 0.0, 0.0, 0.0);
    let (scene, out) = h.frame(|ui| r = ui.status_dot("dot", (10.0, 10.0), Tone::Ok, word));
    (scene, out.tree, r, out.repaint)
}

#[test]
fn the_ui_dot_draws_the_circle_then_its_word_and_settles() {
    let (scene, tree, r, repaint) = drawn("Healthy");
    let g = &scene.layers[0].geometry;
    assert_eq!(g.under.len(), 1);
    let theme = DARK;
    let style = Style::ui(theme.font_base, theme.fg_primary);
    // The dot is centred on the word's line.
    assert_eq!(g.under[0].rect[1], 10.0 + (style.line - 8.0) / 2.0);
    assert_eq!(g.under[0].rect[0], 10.0);
    let texts: Vec<_> = scene.layers.iter().flat_map(|l| &l.texts).collect();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].x, 10.0 + 8.0 + 6.0);
    assert_eq!(texts[0].key.0, Spec::plain("Healthy", style).cache_key(1.0));
    assert_eq!((r.x, r.y), (10.0, 10.0));
    assert!(r.w > 14.0 && r.h == style.line, "{r:?}");
    assert_eq!(repaint, Repaint::Idle);
    assert_eq!(tree.nodes.len(), 2);
}

#[test]
fn the_dot_node_is_a_hidden_image_and_the_word_is_static_text() {
    let (_, tree, _, _) = drawn("Healthy");
    let dot = &tree.nodes[0];
    assert_eq!(dot.role, Role::Image);
    assert!(dot.states.contains(State::Hidden));
    assert!(!dot.states.contains(State::Focusable));
    let word = &tree.nodes[1];
    assert_eq!(word.role, Role::StaticText);
    assert_eq!(word.name, "Healthy");
    assert_eq!(word.value, Some(Value::Text("Healthy".to_owned())));
    assert!(!word.states.contains(State::Focusable));
    assert!(!word.actions.contains(Action::Default));
}

#[test]
fn an_empty_word_draws_nothing_and_emits_no_node() {
    let (scene, tree, r, repaint) = drawn("");
    assert_eq!(scene, Scene::default());
    assert!(tree.nodes.is_empty());
    assert_eq!((r.w, r.h), (0.0, 0.0));
    assert_eq!(repaint, Repaint::Idle);
}

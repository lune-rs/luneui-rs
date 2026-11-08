//! The callout (`widget.callout` acceptance): the box (radius 6, a 1 px border, padding 8 10), the
//! 14 px icon in a 14 by 20 px slot, the tone recipe, the figure and the action at the right, wrapping,
//! the `Note` node and its live region, and that the callout takes no input of its own.

use lune_ui_core::access::{Live, Role, SemanticTree, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::icon;
use lune_ui_core::input::{Key, NamedKey, PointerButton, Repaint};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape, TextItem};
use lune_ui_core::testing::{FakeText, ScriptedInput, UiHarness};
use lune_ui_core::text::{Spec, Style, TextSystem};
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::callout::{CalloutOptions, CalloutResponse};
use lune_ui_core::widgets::tone::Tone;

const WIN: (f32, f32) = (600.0, 300.0);
const RECT: Rect = Rect::new(20.0, 30.0, 400.0, 0.0);

fn opts(tone: Tone) -> CalloutOptions<'static> {
    CalloutOptions {
        tone,
        icon: icon::INFO,
        text: "The proxy is offline",
        figure: None,
        action: None,
        announce: false,
    }
}

struct Drawn {
    scene: Scene,
    tree: SemanticTree,
    resp: CalloutResponse,
    repaint: Repaint,
}

fn draw_in(h: &mut UiHarness, rect: Rect, o: &CalloutOptions<'_>) -> Drawn {
    let mut resp = None;
    let (scene, out) = h.frame(|ui| resp = Some(ui.callout("c", rect, o)));
    Drawn {
        scene,
        tree: out.tree,
        resp: resp.expect("frame ran"),
        repaint: out.repaint,
    }
}

fn draw(o: &CalloutOptions<'_>) -> Drawn {
    draw_in(&mut UiHarness::new(WIN), RECT, o)
}

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene
        .in_order()
        .into_iter()
        .flat_map(|l| l.texts.clone())
        .collect()
}

fn under(scene: &Scene) -> Vec<Shape> {
    scene.layers[0].geometry.under.clone()
}

fn width(spec: &Spec) -> f32 {
    FakeText.measure(spec, 1.0).0
}

fn body_style() -> Style {
    Style {
        line: 20.0,
        ..Style::ui(13.0, DARK.fg_primary)
    }
}

#[test]
fn the_box_is_radius_6_with_a_1_px_border_and_padding_8_by_10() {
    let d = draw(&opts(Tone::Neutral));
    // 1 + 8 + one 20 px line + 8 + 1.
    assert_eq!(d.resp.height, 38.0);
    let s = under(&d.scene);
    assert_eq!(s[0].rect, [20.0, 30.0, 400.0, 38.0]);
    assert_eq!(s[0].radii, [6.0; 4]);
    let ring = d.scene.layers[0]
        .geometry
        .under
        .iter()
        .find(|s| s.params[0] == 1.0)
        .expect("the border");
    assert_eq!(ring.params[1], 1.0, "1 px wide");
    assert_eq!(ring.radii, [6.0; 4]);
}

#[test]
fn at_scale_2_the_radius_is_12_px_the_border_2_px_and_the_icon_slot_40_px() {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let d = draw_in(&mut h, RECT, &opts(Tone::Neutral));
    let s = under(&d.scene);
    assert_eq!(s[0].radii, [12.0; 4]);
    assert_eq!(s[1].params[1], 2.0);
    let t = texts(&d.scene);
    // The 14 px glyph is 7 px wide on the fake text, centred in the 14 px wide slot after the
    // border and the 10 px padding; the text follows the slot and the 8 px gap.
    let slot_x = 20.0 + 1.0 + 10.0;
    assert_eq!(t[0].x, slot_x + (14.0 - 7.0) / 2.0);
    assert_eq!(
        (t[1].x - slot_x) * 2.0,
        28.0 + 16.0,
        "slot 28 px, gap 16 px"
    );
}

#[test]
fn the_icon_is_14_px_in_a_14_by_20_slot_and_the_text_follows_after_8() {
    let d = draw(&opts(Tone::Warn));
    let t = texts(&d.scene);
    assert_eq!(t.len(), 2);
    let glyph = icon::glyph(icon::INFO, 14.0, DARK.warn);
    assert_eq!(
        t[0].key.0,
        glyph.cache_key(1.0),
        "a tone-coloured 14 px glyph"
    );
    let slot = 20.0 + 1.0 + 10.0;
    assert_eq!(t[0].x, slot + (14.0 - width(&glyph)) / 2.0);
    assert_eq!(t[0].y, 30.0 + 1.0 + 8.0 + (20.0 - 14.0 * 1.25) / 2.0);
    assert_eq!((t[1].x, t[1].y), (slot + 22.0, 39.0));
    let want = Spec::plain("The proxy is offline", body_style()).wrapped(400.0 - 22.0 - 22.0);
    assert_eq!(
        t[1].key.0,
        want.cache_key(1.0),
        "13 on a 20 px line, primary"
    );
}

#[test]
fn neutral_is_surface_input_with_border_subtle_and_a_tone_is_the_badge_recipe() {
    let d = draw(&opts(Tone::Neutral));
    let s = under(&d.scene);
    assert_eq!(s[0].color, premul(DARK.surface_input));
    assert_eq!(s[1].color, premul(DARK.border_subtle));
    let t = texts(&d.scene);
    let glyph = icon::glyph(icon::INFO, 14.0, DARK.fg_tertiary);
    assert_eq!(t[0].key.0, glyph.cache_key(1.0), "a tertiary icon");
    for tone in [Tone::Data, Tone::Warn, Tone::Err] {
        let d = draw(&opts(tone));
        let s = under(&d.scene);
        assert_eq!(s[0].color, premul(tone.fill(&DARK)), "{tone:?} fill");
        assert_eq!(s[1].color, premul(tone.border(&DARK)), "{tone:?} border");
        assert_eq!(
            s[0].color[3],
            f32::from(0x1A_u8) / 255.0,
            "{tone:?} fill at 1A"
        );
        let color = if tone == Tone::Data {
            DARK.data_hi
        } else {
            tone.color(&DARK)
        };
        let glyph = icon::glyph(icon::INFO, 14.0, color);
        assert_eq!(
            texts(&d.scene)[0].key.0,
            glyph.cache_key(1.0),
            "{tone:?} icon"
        );
    }
}

#[test]
fn a_figure_sits_at_the_right_in_mono_bright_and_never_gold() {
    let o = CalloutOptions {
        figure: Some("$ 12.40"),
        ..opts(Tone::Warn)
    };
    let d = draw(&o);
    let t = texts(&d.scene);
    assert_eq!(t.len(), 3);
    let style = Style::mono(13.0, 20.0, DARK.fg_bright).medium();
    let figure = Spec::plain("$ 12.40", style);
    assert_eq!(t[2].key.0, figure.cache_key(1.0));
    let gold = Spec::plain(
        "$ 12.40",
        Style {
            color: DARK.gold,
            ..style
        },
    );
    assert_ne!(t[2].key.0, gold.cache_key(1.0));
    assert_eq!(
        t[2].x,
        20.0 + 400.0 - 11.0 - width(&figure),
        "flush with the padding"
    );
    // The text gives up the figure's width and the gap.
    let want = Spec::plain("The proxy is offline", body_style())
        .wrapped(400.0 - 22.0 - 22.0 - width(&figure) - 8.0);
    assert_eq!(t[1].key.0, want.cache_key(1.0));
}

#[test]
fn an_action_is_a_small_button_at_the_right_edge_and_reports_clicks() {
    let o = CalloutOptions {
        action: Some("Retry"),
        ..opts(Tone::Err)
    };
    let mut h = UiHarness::new(WIN);
    let d = draw_in(&mut h, RECT, &o);
    // The 24 px button sets the content height: 1 + 8 + 24 + 8 + 1.
    assert_eq!(d.resp.height, 42.0);
    let button = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Button)
        .expect("button");
    assert_eq!(button.name, "Retry");
    let w = width(&Spec::plain("Retry", Style::ui(13.0, DARK.fg_primary))) + 24.0;
    assert_eq!(
        button.bounds,
        Rect::new(20.0 + 400.0 - 11.0 - w, 39.0, w, 24.0)
    );
    assert!(d.resp.action.is_some());
    let (cx, cy) = (
        button.bounds.x + button.bounds.w / 2.0,
        button.bounds.y + 12.0,
    );
    h.input.pointer_to(cx, cy);
    draw_in(&mut h, RECT, &o);
    h.input.click(PointerButton::Primary);
    let d = draw_in(&mut h, RECT, &o);
    assert!(d.resp.action.expect("action").clicked);
    // No action, no response and no button.
    let d = draw(&opts(Tone::Err));
    assert!(d.resp.action.is_none());
    assert!(d.tree.nodes.iter().all(|n| n.role != Role::Button));
}

#[test]
fn two_callouts_in_one_scope_have_their_own_action_buttons() {
    let o = CalloutOptions {
        action: Some("Go"),
        ..opts(Tone::Neutral)
    };
    let mut h = UiHarness::new(WIN);
    let (_, out) = h.frame(|ui| {
        ui.callout("a", RECT, &o);
        ui.callout("b", Rect::new(20.0, 100.0, 400.0, 0.0), &o);
    });
    let buttons: Vec<_> = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::Button)
        .collect();
    assert_eq!(buttons.len(), 2);
    assert_ne!(buttons[0].id, buttons[1].id);
}

#[test]
fn a_long_text_wraps_and_grows_the_box() {
    let long = "word ".repeat(40);
    let o = CalloutOptions {
        text: long.trim(),
        ..opts(Tone::Data)
    };
    let d = draw(&o);
    assert!(d.resp.height > 38.0);
    assert_eq!(
        (d.resp.height - 18.0) % 20.0,
        0.0,
        "whole lines: {}",
        d.resp.height
    );
    assert_eq!(under(&d.scene)[0].rect[3], d.resp.height);
}

#[test]
fn the_node_is_a_note_named_by_the_text_with_a_polite_live_region_when_announced() {
    let quiet = draw(&opts(Tone::Warn));
    let note = quiet
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Note)
        .expect("note");
    assert_eq!(note.name, "The proxy is offline");
    assert_eq!(note.live, None);
    assert_eq!(note.value, None);
    assert!(!note.states.contains(State::Focusable));
    let o = CalloutOptions {
        figure: Some("$ 12.40"),
        announce: true,
        ..opts(Tone::Warn)
    };
    let loud = draw(&o);
    let note = loud
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Note)
        .expect("note");
    assert_eq!(note.live, Some(Live::Polite));
    assert_eq!(note.value, Some(Value::Text("$ 12.40".to_owned())));
    assert_eq!(note.bounds.h, loud.resp.height);
}

#[test]
fn a_callout_without_an_action_is_static() {
    let mut h = UiHarness::new(WIN);
    let d = draw_in(&mut h, RECT, &opts(Tone::Neutral));
    assert!(
        d.tree
            .nodes
            .iter()
            .all(|n| !n.states.contains(State::Focusable))
    );
    h.input.pointer_to(100.0, 45.0);
    draw_in(&mut h, RECT, &opts(Tone::Neutral));
    h.input.click(PointerButton::Primary);
    draw_in(&mut h, RECT, &opts(Tone::Neutral));
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    let d = draw_in(&mut h, RECT, &opts(Tone::Neutral));
    assert!(
        d.tree
            .nodes
            .iter()
            .all(|n| !n.states.contains(State::Focused))
    );
    for ms in [0, 16, 900] {
        h.at_ms(ms);
        let d = draw_in(&mut h, RECT, &opts(Tone::Neutral));
        assert_eq!(d.repaint, Repaint::Idle, "{ms}");
    }
}

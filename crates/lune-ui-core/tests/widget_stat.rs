//! The stat cell (`data.stat` acceptance, D126): padding, the label, value and sub-line, gold and
//! no data, the compact and text forms, the meter, the strip and its hairlines, the value's tween
//! (once, from the old number to the new, exact at rest) and that a live or text-only value never
//! animates.

use lune_ui_core::access::{Role, SemanticTree, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::Repaint;
use lune_ui_core::motion::{MotionTier, conformance};
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, Shape, TextItem};
use lune_ui_core::testing::{ScriptedInput, UiHarness};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::stat::{StatOptions, VALUE};
use lune_ui_core::widgets::tone::Tone;

const WIN: (f32, f32) = (600.0, 300.0);
const RECT: Rect = Rect::new(20.0, 30.0, 200.0, 0.0);

fn thousands(n: f64) -> String {
    format!("{n:.0} tok")
}

fn base() -> StatOptions<'static> {
    StatOptions {
        label: "Saved",
        value: "1,240",
        sub: Some("+12% vs last week"),
        sub_tone: Tone::Ok,
        ..StatOptions::default()
    }
}

struct Drawn {
    scene: Scene,
    tree: SemanticTree,
    height: f32,
    repaint: Repaint,
}

fn draw_in(h: &mut UiHarness, rect: Rect, o: &StatOptions<'_>) -> Drawn {
    let mut height = 0.0;
    let (scene, out) = h.frame(|ui| height = ui.stat("s", rect, o));
    Drawn {
        scene,
        tree: out.tree,
        height,
        repaint: out.repaint,
    }
}

fn draw(o: &StatOptions<'_>) -> Drawn {
    draw_in(&mut UiHarness::new(WIN), RECT, o)
}

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers.iter().flat_map(|l| l.texts.clone()).collect()
}

fn under(scene: &Scene) -> Vec<Shape> {
    scene.layers[0].geometry.under.clone()
}

fn label_spec() -> Spec {
    Spec::plain(
        "Saved",
        Style {
            line: 16.0,
            ..Style::ui(11.0, DARK.fg_tertiary)
        },
    )
}

fn value_spec(text: &str, color: [u8; 4]) -> Spec {
    Spec::plain(text, Style::mono(20.0, 28.0, color).medium())
}

fn sub_spec(text: &str, color: [u8; 4]) -> Spec {
    Spec::plain(
        text,
        Style {
            line: 16.0,
            ..Style::ui(11.0, color)
        },
    )
}

#[test]
fn the_cell_is_padding_12_16_label_value_and_sub_line() {
    let d = draw(&base());
    let t = texts(&d.scene);
    assert_eq!(t.len(), 3);
    assert_eq!((t[0].x, t[0].y), (36.0, 42.0), "label 12 down, 16 in");
    assert_eq!(
        (t[1].x, t[1].y),
        (36.0, 42.0 + 16.0 + 4.0),
        "value 4 under the label's line"
    );
    assert_eq!(
        (t[2].x, t[2].y),
        (36.0, 62.0 + 28.0),
        "sub-line under the value"
    );
    assert_eq!(d.height, 12.0 + 16.0 + 4.0 + 28.0 + 16.0 + 12.0);
    assert_eq!(t[0].key.0, label_spec().cache_key(1.0));
    assert_eq!(
        t[1].key.0,
        value_spec("1,240", DARK.fg_bright).cache_key(1.0)
    );
    assert_eq!(
        t[2].key.0,
        sub_spec("+12% vs last week", DARK.ok).cache_key(1.0)
    );
}

#[test]
fn at_scale_2_the_padding_is_24_px_by_32_px() {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let d = draw_in(&mut h, RECT, &base());
    let t = texts(&d.scene);
    assert_eq!(((t[0].x - 20.0) * 2.0, (t[0].y - 30.0) * 2.0), (32.0, 24.0));
    assert_eq!(
        d.height * 2.0,
        2.0 * (12.0 + 16.0 + 4.0 + 28.0 + 16.0 + 12.0)
    );
    // Height less the text is the padding top and bottom.
    assert_eq!((d.height - (16.0 + 4.0 + 28.0 + 16.0)) * 2.0, 48.0);
}

#[test]
fn the_sub_line_is_tinted_ok_or_warn_and_is_otherwise_tertiary() {
    for (tone, color) in [
        (Tone::Ok, DARK.ok),
        (Tone::Warn, DARK.warn),
        (Tone::Neutral, DARK.fg_tertiary),
        (Tone::Err, DARK.fg_tertiary),
    ] {
        let o = StatOptions {
            sub_tone: tone,
            ..base()
        };
        let t = texts(&draw(&o).scene);
        assert_eq!(
            t[2].key.0,
            sub_spec("+12% vs last week", color).cache_key(1.0),
            "{tone:?}"
        );
    }
    let none = StatOptions {
        sub: None,
        ..base()
    };
    let d = draw(&none);
    assert_eq!(texts(&d.scene).len(), 2);
    assert_eq!(d.height, 12.0 + 16.0 + 4.0 + 28.0 + 12.0);
}

#[test]
fn gold_colours_the_value_only_and_never_a_cell_with_no_data() {
    let gold = StatOptions {
        gold: true,
        ..base()
    };
    let t = texts(&draw(&gold).scene);
    assert_eq!(t[1].key.0, value_spec("1,240", DARK.gold).cache_key(1.0));
    assert_eq!(
        t[0].key.0,
        label_spec().cache_key(1.0),
        "the label stays tertiary"
    );
    assert_eq!(
        t[2].key.0,
        sub_spec("+12% vs last week", DARK.ok).cache_key(1.0)
    );
    let plain = texts(&draw(&base()).scene);
    assert_ne!(t[1].key, plain[1].key);

    let empty = StatOptions {
        gold: true,
        no_data: true,
        ..base()
    };
    let d = draw(&empty);
    let t = texts(&d.scene);
    assert_eq!(
        t[1].key.0,
        value_spec("\u{2014}", DARK.fg_bright).cache_key(1.0),
        "bright, not gold"
    );
    assert_eq!(
        t[2].key.0,
        sub_spec("No data", DARK.fg_tertiary).cache_key(1.0)
    );
}

#[test]
fn no_data_shows_a_dash_and_the_toolkit_string_through_the_table() {
    let d = draw(&StatOptions {
        no_data: true,
        ..base()
    });
    let value = d
        .tree
        .nodes
        .iter()
        .find(|n| n.value == Some(Value::Text("\u{2014}".to_owned())));
    assert!(value.is_some(), "the value node is the dash");
    assert!(
        d.tree
            .nodes
            .iter()
            .any(|n| n.value == Some(Value::Text("No data".to_owned())))
    );
    let mut h = UiHarness::new(WIN);
    h.strings
        .insert("luneui.stat.no-data", "Tiada data")
        .expect("a toolkit key");
    let d = draw_in(
        &mut h,
        RECT,
        &StatOptions {
            no_data: true,
            ..base()
        },
    );
    assert_eq!(
        texts(&d.scene)[2].key.0,
        sub_spec("Tiada data", DARK.fg_tertiary).cache_key(1.0)
    );
}

#[test]
fn a_long_value_is_elided_to_the_cell() {
    let o = StatOptions {
        value: "123456789012345678901234567890",
        ..base()
    };
    let t = texts(&draw(&o).scene);
    assert_ne!(
        t[1].key.0,
        value_spec("123456789012345678901234567890", DARK.fg_bright).cache_key(1.0)
    );
}

#[test]
fn compact_is_padding_6_8_with_a_14_on_20_value_and_no_gap_above_it() {
    let o = StatOptions {
        compact: true,
        ..base()
    };
    let d = draw(&o);
    let t = texts(&d.scene);
    assert_eq!((t[0].x, t[0].y), (28.0, 36.0));
    assert_eq!((t[1].x, t[1].y), (28.0, 36.0 + 16.0));
    assert_eq!(
        t[1].key.0,
        Spec::plain("1,240", Style::mono(14.0, 20.0, DARK.fg_bright).medium()).cache_key(1.0)
    );
    assert_eq!(d.height, 6.0 + 16.0 + 20.0 + 16.0 + 6.0);
}

#[test]
fn a_text_value_is_sans_14_on_24() {
    let o = StatOptions {
        text: true,
        value: "Healthy",
        ..base()
    };
    let t = texts(&draw(&o).scene);
    let want = Spec::plain(
        "Healthy",
        Style {
            line: 24.0,
            ..Style::ui(14.0, DARK.fg_bright).medium()
        },
    );
    assert_eq!(t[1].key.0, want.cache_key(1.0));
}

#[test]
fn a_meter_closes_the_cell_8_px_below_the_text() {
    let o = StatOptions {
        meter: Some(0.5),
        ..base()
    };
    let d = draw(&o);
    let text_bottom = 12.0 + 16.0 + 4.0 + 28.0 + 16.0;
    assert_eq!(d.height, text_bottom + 8.0 + 4.0 + 12.0);
    let s = under(&d.scene);
    assert_eq!(s.len(), 2, "track and fill");
    assert_eq!(s[0].rect, [36.0, 30.0 + text_bottom + 8.0, 168.0, 4.0]);
    assert_eq!(s[1].rect, [36.0, 30.0 + text_bottom + 8.0, 84.0, 4.0]);
    assert_eq!(s[1].color, premul(DARK.accent));
    let meter = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Progress)
        .expect("meter");
    assert!(
        matches!(meter.value, Some(Value::Number { value, .. }) if (value - 50.0).abs() < 1e-9)
    );
}

#[test]
fn the_node_is_a_group_named_by_the_label_over_the_value_and_the_sub_line() {
    let o = StatOptions {
        number: Some((1240.0, &thousands)),
        ..base()
    };
    let d = draw(&o);
    let group = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Group)
        .expect("group");
    assert_eq!(group.name, "Saved");
    assert_eq!(group.children.len(), 2);
    let value = d.tree.get(group.children[0]).expect("value");
    assert_eq!(
        (value.role, value.name.as_str()),
        (Role::StaticText, "1,240")
    );
    assert_eq!(value.value, Some(Value::Text("1,240".to_owned())));
    let sub = d.tree.get(group.children[1]).expect("sub");
    assert_eq!(
        (sub.role, sub.name.as_str()),
        (Role::StaticText, "+12% vs last week")
    );
    assert_eq!(group.bounds.h, d.height);
}

fn numbered(n: f64, text: &'static str) -> StatOptions<'static> {
    StatOptions {
        number: Some((n, &thousands)),
        value: text,
        ..base()
    }
}

/// The value text drawn in `scene`: the one of the three whose key matches one of `candidates`.
fn drawn_value(scene: &Scene, candidates: &[String]) -> Option<String> {
    let t = texts(scene);
    candidates
        .iter()
        .find(|c| t[1].key.0 == value_spec(c, DARK.fg_bright).cache_key(1.0))
        .cloned()
}

fn candidates(from: i32, to: i32) -> Vec<String> {
    (from..=to).map(|n| thousands(f64::from(n))).collect()
}

#[test]
fn at_rest_the_drawn_text_is_the_value_exactly_not_the_formatted_number() {
    let mut h = UiHarness::new(WIN);
    for ms in [0, 16, 5000] {
        h.at_ms(ms);
        let d = draw_in(&mut h, RECT, &numbered(1240.0, "1,240"));
        assert_eq!(
            texts(&d.scene)[1].key.0,
            value_spec("1,240", DARK.fg_bright).cache_key(1.0)
        );
        assert_eq!(d.repaint, Repaint::Idle, "{ms}");
    }
}

#[test]
fn a_change_of_number_animates_once_from_the_old_to_the_new() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(1000);
    draw_in(&mut h, RECT, &numbered(100.0, "100"));
    h.at_ms(1000);
    let start = draw_in(&mut h, RECT, &numbered(200.0, "200"));
    assert_eq!(start.repaint, Repaint::Now);
    assert_eq!(
        drawn_value(&start.scene, &candidates(100, 100)).as_deref(),
        Some("100 tok")
    );
    // Half of fast (180 ms), linear: halfway.
    h.at_ms(1090);
    let mid = draw_in(&mut h, RECT, &numbered(200.0, "200"));
    assert_eq!(mid.repaint, Repaint::Now);
    assert!(drawn_value(&mid.scene, &candidates(149, 151)).is_some());
    // Settled: exactly the app's text, and the frame is idle.
    h.at_ms(1180);
    let end = draw_in(&mut h, RECT, &numbered(200.0, "200"));
    assert_eq!(
        texts(&end.scene)[1].key.0,
        value_spec("200", DARK.fg_bright).cache_key(1.0)
    );
    assert_eq!(end.repaint, Repaint::Idle);
    // Once: nothing moves afterwards.
    h.at_ms(1400);
    let after = draw_in(&mut h, RECT, &numbered(200.0, "200"));
    assert_eq!(after.repaint, Repaint::Idle);
    assert_eq!(after.scene, end.scene);
}

#[test]
fn a_new_number_mid_flight_retargets_from_where_it_is() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    draw_in(&mut h, RECT, &numbered(0.0, "0"));
    draw_in(&mut h, RECT, &numbered(100.0, "100"));
    h.at_ms(90);
    let mid = draw_in(&mut h, RECT, &numbered(100.0, "100"));
    assert!(drawn_value(&mid.scene, &candidates(49, 51)).is_some());
    let back = draw_in(&mut h, RECT, &numbered(0.0, "0"));
    assert!(
        drawn_value(&back.scene, &candidates(49, 51)).is_some(),
        "no jump on retarget"
    );
}

#[test]
fn a_live_or_text_only_value_never_puts_the_frame_in_flight() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    draw_in(&mut h, RECT, &numbered(100.0, "100"));
    h.at_ms(10);
    let live = StatOptions {
        live: true,
        ..numbered(200.0, "200")
    };
    let d = draw_in(&mut h, RECT, &live);
    assert_eq!(d.repaint, Repaint::Idle);
    assert_eq!(
        texts(&d.scene)[1].key.0,
        value_spec("200", DARK.fg_bright).cache_key(1.0)
    );
    h.at_ms(20);
    let live = StatOptions {
        live: true,
        ..numbered(300.0, "300")
    };
    let d = draw_in(&mut h, RECT, &live);
    assert_eq!(d.repaint, Repaint::Idle);
    assert_eq!(
        texts(&d.scene)[1].key.0,
        value_spec("300", DARK.fg_bright).cache_key(1.0)
    );

    let mut h = UiHarness::new(WIN);
    draw_in(&mut h, RECT, &base());
    h.at_ms(10);
    let d = draw_in(
        &mut h,
        RECT,
        &StatOptions {
            value: "9,999",
            ..base()
        },
    );
    assert_eq!(d.repaint, Repaint::Idle);
    assert_eq!(
        texts(&d.scene)[1].key.0,
        value_spec("9,999", DARK.fg_bright).cache_key(1.0)
    );
}

#[test]
fn a_live_value_that_was_animating_snaps_and_holds() {
    let mut h = UiHarness::new(WIN);
    h.at_ms(0);
    draw_in(&mut h, RECT, &numbered(0.0, "0"));
    draw_in(&mut h, RECT, &numbered(100.0, "100"));
    h.at_ms(30);
    let live = StatOptions {
        live: true,
        ..numbered(500.0, "500")
    };
    let d = draw_in(&mut h, RECT, &live);
    assert_eq!(d.repaint, Repaint::Idle);
    assert_eq!(
        texts(&d.scene)[1].key.0,
        value_spec("500", DARK.fg_bright).cache_key(1.0)
    );
}

#[test]
fn off_snaps_and_reduced_keeps_a_short_tween() {
    let mut off = UiHarness::new(WIN);
    off.tier = MotionTier::Off;
    draw_in(&mut off, RECT, &numbered(100.0, "100"));
    off.at_ms(10);
    let d = draw_in(&mut off, RECT, &numbered(200.0, "200"));
    assert_eq!(d.repaint, Repaint::Idle);
    assert_eq!(
        texts(&d.scene)[1].key.0,
        value_spec("200", DARK.fg_bright).cache_key(1.0)
    );

    let mut reduced = UiHarness::new(WIN);
    reduced.tier = MotionTier::Reduced;
    draw_in(&mut reduced, RECT, &numbered(100.0, "100"));
    reduced.at_ms(10);
    draw_in(&mut reduced, RECT, &numbered(200.0, "200"));
    reduced.at_ms(70);
    let mid = draw_in(&mut reduced, RECT, &numbered(200.0, "200"));
    assert_eq!(
        mid.repaint,
        Repaint::Now,
        "kept, at the Reduced map's 120 ms"
    );
    reduced.at_ms(140);
    let end = draw_in(&mut reduced, RECT, &numbered(200.0, "200"));
    assert_eq!(end.repaint, Repaint::Idle);
}

#[test]
fn the_value_row_conforms_under_every_tier() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        conformance(&VALUE, &DARK.motion, tier, 0.0_f32, 1.0)
            .unwrap_or_else(|e| panic!("value {tier:?}: {e}"));
    }
}

#[test]
fn a_strip_shares_the_width_with_1_px_hairlines_between_cells() {
    let cells = [base(), base(), base()];
    let mut h = UiHarness::new(WIN);
    let rect = Rect::new(20.0, 30.0, 602.0, 0.0);
    let mut height = 0.0;
    let (scene, out) = h.frame(|ui| height = ui.stat_strip("strip", rect, &cells));
    let w = (602.0 - 2.0) / 3.0;
    assert_eq!(w, 200.0);
    let t = texts(&scene);
    for i in 0..3 {
        assert_eq!(t[3 * i].x, 20.0 + i as f32 * 201.0 + 16.0, "cell {i}");
    }
    let hair = premul(DARK.border_subtle);
    let lines: Vec<_> = under(&scene)
        .into_iter()
        .filter(|s| s.color == hair)
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].rect, [220.0, 30.0, 1.0, height]);
    assert_eq!(lines[1].rect, [421.0, 30.0, 1.0, height]);
    assert_eq!(height, 12.0 + 16.0 + 4.0 + 28.0 + 16.0 + 12.0);
    assert_eq!(
        out.tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::Group)
            .count(),
        3
    );
    // At scale 2 a hairline is 2 px.
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let (scene, _) = h.frame(|ui| {
        ui.stat_strip("strip", rect, &cells);
    });
    let line = under(&scene)
        .into_iter()
        .find(|s| s.color == hair)
        .expect("hairline");
    assert_eq!(line.rect[2], 2.0);
}

#[test]
fn a_strip_is_as_tall_as_its_tallest_cell_and_an_empty_strip_is_nothing() {
    let tall = StatOptions {
        meter: Some(0.3),
        ..base()
    };
    let short = StatOptions {
        sub: None,
        ..base()
    };
    let mut h = UiHarness::new(WIN);
    let mut height = 0.0;
    h.frame(|ui| height = ui.stat_strip("strip", RECT, &[short, tall]));
    let want = draw(&tall).height;
    assert_eq!(height, want);
    h.frame(|ui| height = ui.stat_strip("none", RECT, &[]));
    assert_eq!(height, 0.0);
}

#[test]
fn a_stat_is_static_and_takes_no_input() {
    let mut h = UiHarness::new(WIN);
    draw_in(&mut h, RECT, &base());
    h.input.pointer_to(60.0, 60.0);
    draw_in(&mut h, RECT, &base());
    h.input.click(lune_ui_core::input::PointerButton::Primary);
    let d = draw_in(&mut h, RECT, &base());
    assert_eq!(d.repaint, Repaint::Idle);
    assert!(
        d.tree
            .nodes
            .iter()
            .all(|n| !n.states.contains(lune_ui_core::access::State::Focusable))
    );
}

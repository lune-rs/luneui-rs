//! The verdict row (`widget.verdict-row` acceptance): the 40 / 92 / rest grid, the mark in its
//! tone only, the stacked layout with its hairline, wrapping, the string table's mark words, and
//! the `Row` over `Cell`s node.

use lune_ui_core::access::{Action, Role, SemanticTree, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::Repaint;
use lune_ui_core::paint::Rect;
use lune_ui_core::scene::{Scene, TextItem};
use lune_ui_core::testing::{ScriptedInput, UiHarness};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::widgets::verdict_row::Verdict;

const WIN: (f32, f32) = (600.0, 300.0);
const RECT: Rect = Rect::new(20.0, 30.0, 400.0, 0.0);

struct Drawn {
    scene: Scene,
    tree: SemanticTree,
    height: f32,
    repaint: Repaint,
}

fn draw_in(
    h: &mut UiHarness,
    rect: Rect,
    v: Verdict,
    name: &str,
    text: &str,
    stacked: bool,
) -> Drawn {
    let mut height = 0.0;
    let (scene, out) = h.frame(|ui| height = ui.verdict_row("r", rect, v, name, text, stacked));
    Drawn {
        scene,
        tree: out.tree,
        height,
        repaint: out.repaint,
    }
}

fn draw(v: Verdict, name: &str, text: &str, stacked: bool) -> Drawn {
    draw_in(&mut UiHarness::new(WIN), RECT, v, name, text, stacked)
}

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers.iter().flat_map(|l| l.texts.clone()).collect()
}

fn mark_spec(word: &str, color: [u8; 4]) -> Spec {
    Spec::plain(word, Style::mono(11.0, 20.0, color).medium())
}

fn name_spec(name: &str) -> Spec {
    Spec::plain(
        name,
        Style {
            line: 20.0,
            ..Style::ui(13.0, DARK.fg_tertiary)
        },
    )
}

#[test]
fn the_wide_row_is_a_40_92_rest_grid_at_least_24_high() {
    let d = draw(Verdict::Ok, "Quant", "Fits in memory", false);
    assert_eq!(d.height, 24.0);
    let t = texts(&d.scene);
    assert_eq!(t.len(), 3);
    assert_eq!((t[0].x, t[0].y), (20.0, 32.0), "mark");
    assert_eq!((t[1].x, t[1].y), (60.0, 32.0), "name after 40");
    assert_eq!((t[2].x, t[2].y), (152.0, 32.0), "text after 40 + 92");
}

#[test]
fn at_scale_2_the_row_is_48_px_with_columns_of_80_and_184_px() {
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let d = draw_in(&mut h, RECT, Verdict::Ok, "Quant", "Fits", false);
    assert_eq!(d.height * 2.0, 48.0);
    let t = texts(&d.scene);
    // Text items sit in logical px; two device px each.
    assert_eq!((t[1].x - t[0].x) * 2.0, 80.0);
    assert_eq!((t[2].x - t[1].x) * 2.0, 184.0);
    let c = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Row)
        .expect("row");
    assert_eq!(c.bounds.h, 24.0);
}

#[test]
fn only_the_mark_carries_the_tone() {
    for (v, word, c) in [
        (Verdict::Ok, "OK", DARK.ok),
        (Verdict::Warn, "WARN", DARK.warn),
        (Verdict::No, "NO", DARK.err),
    ] {
        let d = draw(v, "Quant", "Fits", false);
        let t = texts(&d.scene);
        assert_eq!(t[0].key.0, mark_spec(word, c).cache_key(1.0), "{word} mark");
        assert_eq!(t[1].key.0, name_spec("Quant").cache_key(1.0), "{word} name");
        let text = Spec::plain(
            "Fits",
            Style {
                line: 20.0,
                ..Style::ui(13.0, DARK.fg_primary)
            },
        )
        .wrapped(400.0 - 132.0);
        assert_eq!(t[2].key.0, text.cache_key(1.0), "{word} text");
    }
}

#[test]
fn the_mark_words_come_through_the_string_table() {
    let d = draw(Verdict::Warn, "Quant", "Fits", false);
    let row = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Row)
        .expect("row");
    assert_eq!(row.value, Some(Value::Text("WARN".to_owned())));

    let mut h = UiHarness::new(WIN);
    h.strings
        .insert("luneui.verdict-row.warn", "ATTENTION")
        .expect("a toolkit key");
    let d = draw_in(&mut h, RECT, Verdict::Warn, "Quant", "Fits", false);
    let row = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Row)
        .expect("row");
    assert_eq!(row.value, Some(Value::Text("ATTENTION".to_owned())));
    assert_eq!(
        texts(&d.scene)[0].key.0,
        mark_spec("ATTENTION", DARK.warn).cache_key(1.0)
    );
}

#[test]
fn a_long_text_wraps_and_grows_the_row() {
    // 268 px of text column at 6.5 px a glyph: 41 per line.
    let long = "word ".repeat(30);
    let d = draw(Verdict::Ok, "Quant", long.trim(), false);
    assert!(d.height > 24.0);
    assert_eq!(
        d.height % 20.0,
        4.0,
        "whole 20 px lines plus 2 + 2: {}",
        d.height
    );
    let row = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Row)
        .expect("row");
    assert_eq!(row.bounds.h, d.height);
}

#[test]
fn an_empty_text_keeps_the_minimum_height_and_draws_no_text() {
    let d = draw(Verdict::No, "Quant", "", false);
    assert_eq!(d.height, 24.0);
    assert_eq!(texts(&d.scene).len(), 2);
}

#[test]
fn a_long_name_is_elided_and_the_node_keeps_the_whole_name() {
    let name = "a-very-long-check-name-indeed";
    let d = draw(Verdict::Ok, name, "Fits", false);
    let t = texts(&d.scene);
    assert_ne!(t[1].key.0, name_spec(name).cache_key(1.0));
    let row = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Row)
        .expect("row");
    assert_eq!(row.name, name);
}

#[test]
fn the_stacked_row_has_mark_and_name_above_the_small_text_and_a_hairline() {
    let d = draw(Verdict::Ok, "Quant", "Fits in memory", true);
    let t = texts(&d.scene);
    assert_eq!(
        (t[0].x, t[0].y),
        (20.0, 34.0),
        "mark after the 4 px padding"
    );
    assert_eq!((t[1].x, t[1].y), (60.0, 34.0));
    assert_eq!((t[2].x, t[2].y), (60.0, 54.0), "text below, under the name");
    let small = Spec::plain(
        "Fits in memory",
        Style {
            line: 14.0,
            ..Style::ui(11.0, DARK.fg_secondary)
        },
    )
    .wrapped(360.0);
    assert_eq!(t[2].key.0, small.cache_key(1.0));
    assert_eq!(d.height, 4.0 + 20.0 + 14.0 + 4.0 + 1.0);
    let g = &d.scene.layers[0].geometry;
    assert_eq!(g.under.len(), 1, "the hairline");
    assert_eq!(g.under[0].rect, [20.0, 30.0 + d.height - 1.0, 400.0, 1.0]);
    assert_eq!(g.under[0].color, premul(DARK.border_subtle));
    // The wide layout has none.
    assert!(
        draw(Verdict::Ok, "Quant", "Fits", false).scene.layers[0]
            .geometry
            .under
            .is_empty()
    );
}

#[test]
fn the_stacked_text_wraps_in_a_narrow_column() {
    let narrow = Rect::new(20.0, 30.0, 214.0, 0.0);
    let mut h = UiHarness::new(WIN);
    let d = draw_in(
        &mut h,
        narrow,
        Verdict::Warn,
        "Quant",
        &"abcd ".repeat(20),
        true,
    );
    // 174 px at 5.5 px a glyph: 31 per line, 99 glyphs: 4 lines.
    let lines = (d.height - 4.0 - 20.0 - 4.0 - 1.0) / 14.0;
    assert!(lines >= 3.0 && lines.fract() == 0.0, "{lines}");
}

#[test]
fn the_node_is_a_row_over_three_cells_and_the_row_is_static() {
    let d = draw(Verdict::No, "Quant", "Does not fit", false);
    let nodes = &d.tree.nodes;
    assert_eq!(nodes.len(), 4);
    let row = &nodes[0];
    assert_eq!(row.role, Role::Row);
    assert_eq!(row.name, "Quant");
    assert_eq!(row.value, Some(Value::Text("NO".to_owned())));
    assert_eq!(row.children.len(), 3);
    let cells: Vec<_> = nodes[1..]
        .iter()
        .map(|n| (n.role, n.name.as_str(), n.parent))
        .collect();
    assert_eq!(
        cells,
        [
            (Role::Cell, "NO", Some(row.id)),
            (Role::Cell, "Quant", Some(row.id)),
            (Role::Cell, "Does not fit", Some(row.id)),
        ]
    );
    for n in nodes {
        assert!(!n.states.contains(State::Focusable));
        assert!(!n.actions.contains(Action::Default));
    }
    assert_eq!(row.bounds, Rect::new(20.0, 30.0, 400.0, 24.0));
}

#[test]
fn it_settles_and_takes_no_tab_stop() {
    let mut h = UiHarness::new(WIN);
    let a = draw_in(&mut h, RECT, Verdict::Ok, "Quant", "Fits", false);
    assert_eq!(a.repaint, Repaint::Idle);
    h.at_ms(700);
    let b = draw_in(&mut h, RECT, Verdict::Ok, "Quant", "Fits", false);
    assert_eq!(b.repaint, Repaint::Idle);
    assert_eq!(a.scene, b.scene);
}

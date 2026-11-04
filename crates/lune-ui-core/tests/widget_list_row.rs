//! The list row (`widget.list-row-badge` acceptance): drawn inside the virtual list's `draw`, the
//! head with its dot and badge, the value and sub lines, the padding of the two lists, the name
//! colour by selection with the `selection` motion, no animation while the list scrolls, the
//! public `paint` the specimens use, and the row's `StaticText` nodes under the list's `ListItem`.

use lune_ui_core::access::{Role, SemanticTree, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{PointerButton, Repaint};
use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::{Scene, Shape, TextItem};
use lune_ui_core::testing::{FakeText, UiHarness};
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::Ui;
use lune_ui_core::widgets::list_row::{ListRow, MASTER_PADDING, SIDE_PANEL_PADDING, height, paint};
use lune_ui_core::widgets::tone::Tone;
use lune_ui_core::widgets::virtual_list::{
    Heights, ListOptions, RowCtx, RowKey, SelectionMode, row_fill,
};

const WIN: (f32, f32) = (400.0, 600.0);
const LIST: Rect = Rect::new(20.0, 20.0, 300.0, 400.0);
const P: PointerButton = PointerButton::Primary;

fn key(i: usize) -> RowKey {
    RowKey(i as u64)
}

fn full() -> ListRow<'static> {
    ListRow {
        name: "main",
        value: Some("a1b2c3d"),
        sub: Some("2 commits ahead"),
        meta: Some("3 h"),
        badge: Some(("open", Tone::Ok)),
        dot: Some(Tone::Warn),
    }
}

fn name_only() -> ListRow<'static> {
    ListRow {
        name: "main",
        ..ListRow::default()
    }
}

struct Run {
    scene: Scene,
    tree: SemanticTree,
    repaint: Repaint,
    height: f32,
}

fn frame_with(h: &mut UiHarness, rows: usize, row: &ListRow<'_>, padding: (f32, f32)) -> Run {
    let o = ListOptions {
        count: rows,
        heights: Heights::Fixed(height(row, padding)),
        selection: SelectionMode::Single,
        index_of: None,
        label: "rows",
    };
    let mut got = 0.0;
    let (scene, out) = h.frame(|ui: &mut Ui<'_>| {
        ui.virtual_list("l", LIST, &o, key, |ui, ctx: &RowCtx| {
            got = ui.list_row(ctx, row, padding);
            got
        });
    });
    Run {
        scene,
        tree: out.tree,
        repaint: out.repaint,
        height: got,
    }
}

fn frame(h: &mut UiHarness, row: &ListRow<'_>) -> Run {
    frame_with(h, 20, row, SIDE_PANEL_PADDING)
}

fn texts(scene: &Scene) -> Vec<TextItem> {
    scene.layers.iter().flat_map(|l| l.texts.clone()).collect()
}

fn under(scene: &Scene) -> Vec<Shape> {
    scene.layers[0].geometry.under.clone()
}

fn name_key(text: &str, color: [u8; 4]) -> u64 {
    Spec::plain(
        text,
        Style {
            line: 20.0,
            ..Style::ui(13.0, color).medium()
        },
    )
    .cache_key(1.0)
}

/// The first row's name item: the first text drawn.
fn first_name(scene: &Scene) -> TextItem {
    texts(scene)[0]
}

#[test]
fn the_height_is_the_inset_the_padding_and_the_lines() {
    // 2 * 1 (the highlight's inset) + 2 * 6 + 20.
    assert_eq!(height(&name_only(), SIDE_PANEL_PADDING), 34.0);
    // Plus the sub-line and the meta line, 4 px apart: the value shares the head line.
    assert_eq!(
        height(&full(), SIDE_PANEL_PADDING),
        2.0 + 12.0 + 20.0 + 4.0 + 16.0 + 4.0 + 16.0
    );
    assert_eq!(height(&full(), SIDE_PANEL_PADDING), 74.0);
    let meta_only = ListRow {
        meta: Some("3 h"),
        ..name_only()
    };
    assert_eq!(height(&meta_only, SIDE_PANEL_PADDING), 34.0 + 20.0);
    let mut h = UiHarness::new(WIN);
    assert_eq!(
        frame(&mut h, &full()).height,
        height(&full(), SIDE_PANEL_PADDING)
    );
}

#[test]
fn the_two_lists_pad_8_and_12_horizontally_and_6_vertically() {
    assert_eq!(SIDE_PANEL_PADDING, (8.0, 6.0));
    assert_eq!(MASTER_PADDING, (12.0, 6.0));
    for (pad, x) in [(SIDE_PANEL_PADDING, 8.0), (MASTER_PADDING, 12.0)] {
        let mut h = UiHarness::new(WIN);
        let d = frame_with(&mut h, 20, &name_only(), pad);
        let name = first_name(&d.scene);
        // The slot starts at the list's left; the highlight 4 px in; the content `x` further.
        assert_eq!(name.x, LIST.x + 4.0 + x);
        assert_eq!(name.y, LIST.y + 1.0 + 6.0);
    }
}

#[test]
fn at_scale_2_the_padding_is_16_by_12_px_the_radius_12_and_selected_is_surface_selected() {
    use lune_ui_core::testing::ScriptedInput;
    let mut h = UiHarness::new(WIN);
    h.input = ScriptedInput::new(WIN, 2.0);
    h.input.focus(true);
    let row = name_only();
    frame(&mut h, &row);
    h.input.pointer_to(100.0, LIST.y + 17.0).click(P);
    h.at_ms(10);
    frame(&mut h, &row);
    h.input.pointer_leave();
    h.at_ms(20);
    frame(&mut h, &row);
    h.at_ms(1000);
    let d = frame(&mut h, &row);
    let fill = under(&d.scene)
        .into_iter()
        .find(|s| s.color == premul(DARK.surface_selected))
        .expect("the selected fill");
    assert_eq!(fill.radii, [12.0; 4]);
    let name = first_name(&d.scene);
    // Highlight x in logical px is the fill's physical x over 2.
    assert_eq!((name.x - fill.rect[0] / 2.0) * 2.0, 16.0);
    assert_eq!((name.y - fill.rect[1] / 2.0) * 2.0, 12.0);
}

#[test]
fn the_name_is_secondary_at_rest_and_bright_once_selected() {
    let mut h = UiHarness::new(WIN);
    let row = name_only();
    let d = frame(&mut h, &row);
    assert_eq!(
        first_name(&d.scene).key.0,
        name_key("main", DARK.fg_secondary)
    );
    h.input.pointer_to(100.0, LIST.y + 17.0).click(P);
    h.at_ms(10);
    frame(&mut h, &row);
    h.at_ms(1000);
    let d = frame(&mut h, &row);
    assert_eq!(first_name(&d.scene).key.0, name_key("main", DARK.fg_bright));
    assert_eq!(d.repaint, Repaint::Idle);
    assert_eq!(
        row_fill(&DARK, false, true),
        DARK.surface_selected,
        "the list's own fill"
    );
}

#[test]
fn selecting_fades_the_name_at_fast_and_ends_exact() {
    let mut h = UiHarness::new(WIN);
    let row = name_only();
    frame(&mut h, &row);
    h.input.pointer_to(100.0, LIST.y + 17.0).click(P);
    h.at_ms(10);
    frame(&mut h, &row);
    h.at_ms(100);
    let mid = frame(&mut h, &row);
    let k = first_name(&mid.scene).key.0;
    assert_eq!(mid.repaint, Repaint::Now);
    assert!(
        k != name_key("main", DARK.fg_secondary) && k != name_key("main", DARK.fg_bright),
        "between the two"
    );
    h.at_ms(1000);
    let end = frame(&mut h, &row);
    assert_eq!(
        first_name(&end.scene).key.0,
        name_key("main", DARK.fg_bright)
    );
}

#[test]
fn nothing_animates_while_the_list_scrolls() {
    let mut h = UiHarness::new(WIN);
    let row = name_only();
    frame(&mut h, &row);
    h.input.pointer_to(100.0, LIST.y + 17.0).click(P);
    h.at_ms(10);
    frame(&mut h, &row);
    // The name colour is mid-fade when the wheel turns: the rows snap.
    h.at_ms(60);
    let fading = frame(&mut h, &row);
    assert_eq!(fading.repaint, Repaint::Now);
    h.input.scroll_pixels(0.0, -2.0);
    h.at_ms(70);
    let scrolled = frame(&mut h, &row);
    let first = texts(&scrolled.scene)
        .into_iter()
        .find(|t| t.key.0 == name_key("main", DARK.fg_bright));
    assert!(first.is_some(), "the selected row's name snapped to bright");
    for t in texts(&scrolled.scene) {
        assert!(
            t.key.0 == name_key("main", DARK.fg_bright)
                || t.key.0 == name_key("main", DARK.fg_secondary),
            "no row is between colours while the list moves"
        );
    }
}

#[test]
fn the_head_has_the_dot_the_name_the_badge_and_the_value_then_the_sub_line_and_the_meta() {
    let mut h = UiHarness::new(WIN);
    let d = frame(&mut h, &full());
    let t = texts(&d.scene);
    let x = LIST.x + 4.0 + 8.0;
    // The dot is 8 px, centred on the name's 20 px line, in the tone's colour.
    let dot = under(&d.scene)
        .into_iter()
        .find(|s| s.rect == [x, LIST.y + 7.0 + 6.0, 8.0, 8.0])
        .expect("the dot");
    assert_eq!(dot.color, premul(DARK.warn));
    // The name follows the dot and its 8 px gap (`status_dot::GAP`).
    assert_eq!((t[0].x, t[0].y), (x + 16.0, LIST.y + 7.0));
    // The value takes the right edge of the content width (300 - 24); mono 13 is 6.5 px a glyph
    // on the fake text. The badge ends 8 px before it, 18 px high, centred on the line.
    let value = Spec::plain("a1b2c3d", Style::mono(13.0, 20.0, DARK.fg_secondary));
    let value_item = t
        .iter()
        .find(|i| i.key.0 == value.cache_key(1.0))
        .expect("value");
    let right = LIST.x + 4.0 + 8.0 + 300.0 - 24.0;
    assert_eq!(value_item.x, right - 7.0 * 6.5);
    assert_eq!(value_item.y, LIST.y + 7.0);
    let badge = under(&d.scene)
        .into_iter()
        .find(|s| s.rect[3] == 18.0 && s.radii == [8.0; 4])
        .expect("the badge");
    assert_eq!(badge.rect[0] + badge.rect[2], value_item.x - 8.0);
    assert_eq!(badge.rect[1], LIST.y + 7.0 + 1.0);
    // Then the sub-line (mono 11 tertiary) and the meta (11 tertiary) at the left, 4 px apart.
    let sub = Spec::plain("2 commits ahead", Style::mono(11.0, 16.0, DARK.fg_tertiary));
    let sub_item = t
        .iter()
        .find(|i| i.key.0 == sub.cache_key(1.0))
        .expect("sub");
    assert_eq!((sub_item.x, sub_item.y), (x, LIST.y + 7.0 + 24.0));
    let meta = Spec::plain(
        "3 h",
        Style {
            line: 16.0,
            ..Style::ui(11.0, DARK.fg_tertiary)
        },
    );
    let meta_item = t
        .iter()
        .find(|i| i.key.0 == meta.cache_key(1.0))
        .expect("meta");
    assert_eq!((meta_item.x, meta_item.y), (x, LIST.y + 7.0 + 44.0));
}

#[test]
fn a_long_name_value_and_sub_are_elided_never_wrapped() {
    let mut h = UiHarness::new(WIN);
    let long = "n".repeat(120);
    let row = ListRow {
        name: &long,
        value: Some(&long),
        sub: Some(&long),
        ..full()
    };
    let d = frame(&mut h, &row);
    let whole = name_key(&long, DARK.fg_secondary);
    assert!(texts(&d.scene).iter().all(|t| t.key.0 != whole));
    assert_eq!(
        d.height,
        height(&row, SIDE_PANEL_PADDING),
        "the height does not depend on the text"
    );
}

#[test]
fn the_public_paint_draws_what_the_list_row_draws_at_rest() {
    let mut h = UiHarness::new(WIN);
    let d = frame(&mut h, &full());
    let slot = Rect::new(
        LIST.x,
        LIST.y,
        LIST.w - 0.0,
        height(&full(), SIDE_PANEL_PADDING),
    );
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(
        1.0,
        Rect::new(0.0, 0.0, WIN.0, WIN.1),
        &mut text,
        None,
        DARK,
    );
    let got = paint(&mut p, slot, &full(), SIDE_PANEL_PADDING, DARK.fg_secondary);
    assert_eq!(got, slot.h);
    let from_paint: Vec<(u64, f32, f32)> = texts(&p.scene)
        .iter()
        .map(|t| (t.key.0, t.x, t.y))
        .collect();
    let from_ui: Vec<(u64, f32, f32)> = texts(&d.scene)
        .iter()
        .take(from_paint.len())
        .map(|t| (t.key.0, t.x, t.y))
        .collect();
    assert_eq!(from_paint, from_ui);
}

#[test]
fn the_row_adds_its_name_and_value_under_the_lists_list_item() {
    let mut h = UiHarness::new(WIN);
    let d = frame(&mut h, &full());
    let item = d
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::ListItem)
        .expect("a row");
    let kids: Vec<_> = d
        .tree
        .nodes
        .iter()
        .filter(|n| n.parent == Some(item.id))
        .collect();
    let name = kids
        .iter()
        .find(|n| n.value == Some(Value::Text("main".into())))
        .expect("name");
    assert_eq!(name.role, Role::StaticText);
    assert_eq!(name.name, "main");
    let value = kids
        .iter()
        .find(|n| n.value == Some(Value::Text("a1b2c3d".into())))
        .expect("value");
    assert_eq!(value.name, "main", "the value is named by the row");
    assert!(!item.states.contains(State::Selected));
}

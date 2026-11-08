//! The dropdown (C4 retrofit, `overlay.dropdown` acceptance): rest identity with the Painter-API `dropdown`, keyboard highlight, disabled rows,
//! dismissal, focus restore to the combo, the semantic tree, the glass surface and the row fills.

use lune_ui_core::access::{Action, Role, State};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, Modifiers, NamedKey, PointerButton};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{FakeText, UiHarness, disabled_violations, flattened};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::dropdown::{
    DropdownOptions, DropdownResponse, dropdown, panel_rect, row_rect,
};

const WIN: (f32, f32) = (1040.0, 660.0);
const ANCHOR: Rect = Rect::new(34.0, 100.0, 210.0, 32.0);
/// Near the bottom edge: the list opens above it.
const FLIPPED: Rect = Rect::new(34.0, 560.0, 210.0, 32.0);
const P: PointerButton = PointerButton::Primary;
const OPTIONS: [(&str, bool); 4] = [
    ("RznSans", true),
    ("Sans Serif", true),
    ("Monospace", true),
    ("Coming soon", false),
];

fn win() -> Rect {
    Rect::new(0.0, 0.0, WIN.0, WIN.1)
}

fn old_scene(anchor: Rect, selected: usize, hover: Option<u8>) -> Scene {
    let mut text = FakeText;
    let mut p = Painter::<u8>::new(1.0, win(), &mut text, hover, DARK);
    p.on(LayerId::Popup);
    dropdown(&mut p, anchor, &OPTIONS, selected, |i| i as u8);
    p.scene
}

fn row_centre(anchor: Rect, i: usize) -> (f32, f32) {
    let mut text = FakeText;
    let p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let r = row_rect(panel_rect(&p, anchor, OPTIONS.len()), i);
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

fn anchor_id() -> WidgetId {
    WidgetId::ROOT.with("combo")
}

fn frame(
    h: &mut UiHarness,
    ms: u64,
    anchor: Rect,
    open: bool,
    selected: usize,
) -> (Scene, DropdownResponse) {
    let mut resp = DropdownResponse::default();
    let (scene, _) = h.at_ms(ms).frame(|ui| {
        if open {
            resp = ui.dropdown("dd", anchor, anchor_id(), &OPTIONS, selected);
        }
    });
    (scene, resp)
}

#[test]
fn at_rest_the_lists_are_the_old_lists() {
    for anchor in [ANCHOR, FLIPPED] {
        let mut h = UiHarness::new(WIN);
        frame(&mut h, 0, anchor, true, 0);
        let (scene, _) = frame(&mut h, 1000, anchor, true, 0);
        assert!(
            flattened(&scene) == old_scene(anchor, 0, None),
            "{anchor:?}"
        );
    }
    // The flipped list really is above its anchor.
    let mut text = FakeText;
    let p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    assert!(panel_rect(&p, FLIPPED, 4).bottom() <= FLIPPED.y);
    assert!(panel_rect(&p, ANCHOR, 4).y > ANCHOR.bottom());
}

#[test]
fn a_hovered_enabled_row_gets_the_hover_fill_and_a_disabled_one_does_not() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    let (x, y) = row_centre(ANCHOR, 1);
    h.input.pointer_to(x, y);
    frame(&mut h, 10, ANCHOR, true, 0);
    let (scene, _) = frame(&mut h, 1000, ANCHOR, true, 0);
    assert!(flattened(&scene) == old_scene(ANCHOR, 0, Some(1)));
    // Over the disabled row: no hover fill, same as the old list with that hit absent.
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    let (x, y) = row_centre(ANCHOR, 3);
    h.input.pointer_to(x, y);
    frame(&mut h, 10, ANCHOR, true, 0);
    let (scene, _) = frame(&mut h, 1000, ANCHOR, true, 0);
    assert!(flattened(&scene) == old_scene(ANCHOR, 0, Some(3)));
}

#[test]
fn a_click_picks_an_enabled_row_once_and_never_a_disabled_one() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    let (x, y) = row_centre(ANCHOR, 2);
    h.input.pointer_to(x, y).click(P);
    let (_, r) = frame(&mut h, 10, ANCHOR, true, 0);
    assert_eq!(r.picked, Some(2));
    let (_, r) = frame(&mut h, 20, ANCHOR, true, 0);
    assert_eq!(r.picked, None);
    let (x, y) = row_centre(ANCHOR, 3);
    h.input.pointer_to(x, y).click(P);
    let (_, r) = frame(&mut h, 30, ANCHOR, true, 0);
    assert!(
        r.picked.is_none() && !r.dismissed,
        "inside the panel, nothing fires"
    );
}

#[test]
fn the_keyboard_highlight_starts_at_the_selected_row_and_skips_disabled_ones() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 1);
    frame(&mut h, 10, ANCHOR, true, 1);
    // Enter at once picks the selected row.
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, r) = frame(&mut h, 20, ANCHOR, true, 1);
    assert_eq!(r.picked, Some(1));
    // Down to row 2 (drawn with the hover fill), Down again stays (row 3 is disabled).
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    let (scene, _) = frame(&mut h, 30, ANCHOR, true, 0);
    assert!(
        flattened(&scene) == old_scene(ANCHOR, 0, Some(2)),
        "row 2 highlighted"
    );
    // One press moves one row per frame; Down again stays (row 3 is disabled).
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    frame(&mut h, 40, ANCHOR, true, 0);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, r) = frame(&mut h, 50, ANCHOR, true, 0);
    assert_eq!(r.picked, Some(2), "the disabled row 3 is skipped");
    for ms in [60, 70, 80] {
        h.input.key_down(Key::Named(NamedKey::ArrowUp), None);
        frame(&mut h, ms, ANCHOR, true, 0);
    }
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, r) = frame(&mut h, 90, ANCHOR, true, 0);
    assert_eq!(r.picked, Some(0), "Up stops at the first row");
}

#[test]
fn escape_and_an_outside_press_dismiss() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (_, r) = frame(&mut h, 10, ANCHOR, true, 0);
    assert!(r.dismissed);
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    h.input.pointer_to(700.0, 400.0).click(P);
    let (_, r) = frame(&mut h, 10, ANCHOR, true, 0);
    assert!(r.dismissed);
}

#[test]
fn it_opens_and_closes_at_once_and_focus_returns_to_the_combo() {
    let mut h = UiHarness::new(WIN);
    let mut open = false;
    let run = |h: &mut UiHarness, ms: u64, open: &mut bool| {
        let mut out = DropdownResponse::default();
        let mut focused = false;
        let (scene, _) = h.at_ms(ms).frame(|ui| {
            let c = ui.combo("combo", ANCHOR, "RznSans", open, "Font");
            focused = c.focused;
            if *open {
                out = ui.dropdown("dd", ANCHOR, c.id, &OPTIONS, 0);
                if out.dismissed || out.picked.is_some() {
                    *open = false;
                }
            }
        });
        (out, focused, scene)
    };
    run(&mut h, 0, &mut open);
    h.input.key_down(Key::Named(NamedKey::Tab), None);
    run(&mut h, 10, &mut open);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, focused, _) = run(&mut h, 20, &mut open);
    assert!(open && focused);
    // The list takes focus on the next frame; keys go to it, not to the combo.
    let (_, focused, _) = run(&mut h, 30, &mut open);
    assert!(!focused, "focus moved into the list");
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    let (out, ..) = run(&mut h, 40, &mut open);
    assert!(
        open && out.picked.is_none(),
        "Down moves the highlight, it does not reopen"
    );
    h.input.key_down(Key::Named(NamedKey::Escape), None);
    let (out, ..) = run(&mut h, 50, &mut open);
    assert!(out.dismissed && !open);
    // No exit frame: the first frame without the app's call draws nothing on the popup layer.
    let (_, _, scene) = run(&mut h, 60, &mut open);
    assert!(scene.merged(LayerId::Popup).geometry.under.is_empty());
    // Focus returns when the close settles, one frame later.
    let (_, focused, _) = run(&mut h, 70, &mut open);
    assert!(focused, "focus is back on the combo");
}

#[test]
fn the_semantic_tree_is_a_list_of_list_items() {
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 2);
    let (_, out) = h.at_ms(10).frame(|ui| {
        ui.dropdown("dd", ANCHOR, anchor_id(), &OPTIONS, 2);
    });
    let list = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::List)
        .expect("list");
    let items: Vec<_> = out
        .tree
        .nodes
        .iter()
        .filter(|n| n.role == Role::ListItem)
        .collect();
    assert_eq!(items.len(), 4);
    assert_eq!(list.children.len(), 4);
    assert_eq!(items[2].name, "Monospace");
    for (i, n) in items.iter().enumerate() {
        assert_eq!(n.states.contains(State::Selected), i == 2, "{i}");
        assert_eq!(n.states.contains(State::Disabled), i == 3, "{i}");
        assert_eq!(n.actions.contains(Action::Default), i != 3, "{i}");
    }
}

fn many() -> Vec<(String, bool)> {
    (0..30).map(|i| (format!("Option {i}"), true)).collect()
}

fn with_options(
    h: &mut UiHarness,
    ms: u64,
    options: &[(String, bool)],
    selected: usize,
    opts: &DropdownOptions,
) -> (Scene, DropdownResponse, FrameOutput) {
    let mut resp = DropdownResponse::default();
    let (scene, out) = h.at_ms(ms).frame(|ui| {
        resp = ui.dropdown_with("dd", ANCHOR, anchor_id(), options, selected, opts);
    });
    (scene, resp, out)
}

const SCROLLING: DropdownOptions = DropdownOptions { max_rows: Some(8) };

fn row_id(i: usize) -> WidgetId {
    WidgetId::ROOT.with("dd").with("list").with(i.to_string())
}

/// The row `i` node's rect in the frame.
fn row_bounds(out: &FrameOutput, i: usize) -> Rect {
    out.tree.get(row_id(i)).expect("row node").bounds
}

/// The panel of 8 rows under the anchor, and the viewport the rows scroll in.
fn scrolled_panel() -> (Rect, Rect) {
    let mut text = FakeText;
    let p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let panel = panel_rect(&p, ANCHOR, 8);
    (
        panel,
        Rect::new(panel.x, panel.y + 4.0, panel.w, panel.h - 8.0),
    )
}

fn inside(r: Rect, view: Rect) -> bool {
    r.y >= view.y && r.bottom() <= view.bottom()
}

#[test]
fn the_list_is_the_glass_surface() {
    let opts = [
        ("RznSans".to_owned(), true),
        ("Sans Serif".to_owned(), true),
    ];
    let mut h = UiHarness::new(WIN);
    let plain = DropdownOptions::default();
    with_options(&mut h, 0, &opts, 0, &plain);
    let (scene, ..) = with_options(&mut h, 500, &opts, 0, &plain);
    let popup = LayerId::Popup;
    let under = &scene.merged(popup).geometry.under;
    // The frost of `menu_glass` over the blurred scene, then the ring.
    assert_eq!(under[0].color, premul(DARK.menu_glass));
    assert_eq!(under[0].params[2], 2.0, "a frost");
    assert_eq!(under[0].radii, [9.0; 4]);
    assert_eq!(under[1].color, premul(DARK.menu_popup_border), "the ring");
    let b = scene.merged(popup).backdrops[0];
    assert_eq!(b.rect, under[0].rect);
    assert_eq!(b.radius, 9.0);
}

#[test]
fn a_row_wears_the_menus_hover_and_pressed_fills_at_radius_4() {
    let fill_of = |scene: &Scene, color| {
        scene
            .merged(LayerId::Popup)
            .geometry
            .under
            .iter()
            .copied()
            .find(|s| s.color == premul(color) && s.params[2] != 2.0)
    };
    let mut text = FakeText;
    let p = Painter::<u8>::new(1.0, win(), &mut text, None, DARK);
    let row = row_rect(panel_rect(&p, ANCHOR, OPTIONS.len()), 1);
    let mut h = UiHarness::new(WIN);
    frame(&mut h, 0, ANCHOR, true, 0);
    let (x, y) = row_centre(ANCHOR, 1);
    h.input.pointer_to(x, y);
    frame(&mut h, 10, ANCHOR, true, 0);
    let (hover, _) = frame(&mut h, 20, ANCHOR, true, 0);
    let fill = fill_of(&hover, DARK.menu_item_hover).expect("the hover fill");
    assert_eq!(fill.radii, [4.0; 4]);
    assert_eq!(fill.rect, [row.x, row.y, row.w, row.h]);
    assert!(fill_of(&hover, DARK.menu_item_pressed).is_none());
    h.input.press(P);
    let (down, _) = frame(&mut h, 30, ANCHOR, true, 0);
    let fill = fill_of(&down, DARK.menu_item_pressed).expect("the pressed fill");
    assert_eq!(fill.radii, [4.0; 4]);
    assert!(
        fill_of(&down, DARK.menu_item_hover).is_none(),
        "pressed replaces hover"
    );
}

#[test]
fn the_default_options_draw_what_dropdown_draws() {
    let owned: Vec<(String, bool)> = OPTIONS.iter().map(|(t, e)| ((*t).to_owned(), *e)).collect();
    for ms in [0, 90, 500] {
        let mut a = UiHarness::new(WIN);
        let mut b = UiHarness::new(WIN);
        let (old, ..) = frame(&mut a, ms, ANCHOR, true, 1);
        let (new, ..) = with_options(&mut b, ms, &owned, 1, &DropdownOptions::default());
        assert!(old == new, "{ms}");
    }
}

#[test]
fn max_rows_caps_the_panel_and_a_short_list_does_not_scroll() {
    let (panel, view) = scrolled_panel();
    assert_eq!(panel.h, 8.0 * 32.0 + 8.0);
    let mut h = UiHarness::new(WIN);
    with_options(&mut h, 0, &many(), 0, &SCROLLING);
    let (scene, _, out) = with_options(&mut h, 500, &many(), 0, &SCROLLING);
    let fill = scene.merged(LayerId::Popup).geometry.under[0];
    assert_eq!(fill.rect[3], panel.h, "eight rows and the padding");
    assert!(inside(row_bounds(&out, 7), view));
    assert!(!inside(row_bounds(&out, 8), view), "the ninth is below");
    // A list no longer than the cap draws as it always did.
    let four: Vec<(String, bool)> = OPTIONS.iter().map(|(t, e)| ((*t).to_owned(), *e)).collect();
    let mut a = UiHarness::new(WIN);
    let mut b = UiHarness::new(WIN);
    with_options(&mut a, 0, &four, 0, &SCROLLING);
    with_options(&mut b, 0, &four, 0, &DropdownOptions::default());
    let (x, ..) = with_options(&mut a, 500, &four, 0, &SCROLLING);
    let (y, ..) = with_options(&mut b, 500, &four, 0, &DropdownOptions::default());
    assert!(x == y);
}

#[test]
fn down_past_the_last_visible_row_reveals_it_and_enter_picks_it() {
    let (_, view) = scrolled_panel();
    let mut h = UiHarness::new(WIN);
    with_options(&mut h, 0, &many(), 0, &SCROLLING);
    with_options(&mut h, 10, &many(), 0, &SCROLLING);
    let mut ms = 20;
    // Row 0 is lit at open (keys come to the list; the first Down moves to 1).
    for _ in 0..9 {
        h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
        with_options(&mut h, ms, &many(), 0, &SCROLLING);
        ms += 10;
    }
    // Let the reveal's motion run out.
    let (_, _, out) = with_options(&mut h, ms + 1000, &many(), 0, &SCROLLING);
    assert!(
        inside(row_bounds(&out, 9), view),
        "{:?}",
        row_bounds(&out, 9)
    );
    assert!(!inside(row_bounds(&out, 0), view), "row 0 left the top");
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    let (_, r, _) = with_options(&mut h, ms + 1010, &many(), 0, &SCROLLING);
    assert_eq!(r.picked, Some(9));
}

#[test]
fn a_selected_row_below_the_fold_opens_in_view_on_the_first_frame() {
    let (_, view) = scrolled_panel();
    for selected in [20, 29, 7, 3] {
        let mut h = UiHarness::new(WIN);
        let (_, _, out) = with_options(&mut h, 0, &many(), selected, &SCROLLING);
        assert!(
            inside(row_bounds(&out, selected), view),
            "{selected}: {:?}",
            row_bounds(&out, selected)
        );
    }
    // A selected row already in view moves nothing.
    let mut h = UiHarness::new(WIN);
    let (_, _, out) = with_options(&mut h, 0, &many(), 3, &SCROLLING);
    assert_eq!(row_bounds(&out, 0).y, view.y);
}

#[test]
fn the_wheel_scrolls_and_a_click_picks_the_row_under_the_pointer() {
    let (_, view) = scrolled_panel();
    let mut h = UiHarness::new(WIN);
    with_options(&mut h, 0, &many(), 0, &SCROLLING);
    h.input.pointer_to(view.x + 20.0, view.y + 20.0);
    with_options(&mut h, 10, &many(), 0, &SCROLLING);
    h.input.scroll_lines(0.0, -2.0);
    with_options(&mut h, 20, &many(), 0, &SCROLLING);
    let (_, _, out) = with_options(&mut h, 1000, &many(), 0, &SCROLLING);
    // Two wheel lines are 96 px: row 3 is at the top.
    assert_eq!(row_bounds(&out, 3).y, view.y);
    let r = row_bounds(&out, 5);
    h.input.pointer_to(r.x + 10.0, r.y + 10.0).click(P);
    let (_, resp, _) = with_options(&mut h, 1010, &many(), 0, &SCROLLING);
    assert_eq!(resp.picked, Some(5));
    // A row scrolled out of the panel takes no click outside it.
    let above = row_bounds(&out, 0);
    h.input.pointer_to(above.x + 10.0, above.y + 10.0).click(P);
    let (_, resp, _) = with_options(&mut h, 1030, &many(), 0, &SCROLLING);
    assert_eq!(resp.picked, None);
}

#[test]
fn a_chord_on_the_lists_keys_stays_a_shortcut() {
    for k in [NamedKey::Enter, NamedKey::ArrowDown, NamedKey::ArrowUp] {
        let opts = DropdownOptions::default();
        let mut h = UiHarness::new(WIN);
        h.shortcuts.add(Binding::new(
            Scope::Global,
            Chord::key(Key::Named(k)).ctrl(),
            "chord",
        ));
        with_options(&mut h, 0, &many(), 0, &opts);
        with_options(&mut h, 10, &many(), 0, &opts);
        h.input.set_modifiers(Modifiers {
            control: true,
            ..Modifiers::default()
        });
        h.input.key_down(Key::Named(k), None);
        let mut resp = DropdownResponse::default();
        let (_, out) = h.at_ms(20).frame(|ui| {
            resp = ui.dropdown_with("dd", ANCHOR, anchor_id(), &many(), 0, &opts);
        });
        assert_eq!(out.fired.len(), 1, "{k:?}");
        assert_eq!(resp.picked, None, "{k:?}");
    }
}

#[test]
fn a_disabled_dropdown_takes_the_keys_picks_nothing_and_marks_every_node() {
    let keys = [NamedKey::Enter, NamedKey::ArrowDown, NamedKey::ArrowUp];
    for opts in [DropdownOptions::default(), SCROLLING] {
        let mut h = UiHarness::new(WIN);
        with_options(&mut h, 0, &many(), 0, &opts);
        with_options(&mut h, 10, &many(), 0, &opts);
        for k in keys {
            h.shortcuts
                .add(Binding::new(Scope::Global, Chord::key(Key::Named(k)), "go"));
        }
        let frame = |h: &mut UiHarness, ms: u64| {
            let mut resp = DropdownResponse::default();
            let (_, out) = h.at_ms(ms).frame(|ui| {
                ui.with_enabled(false, |ui| {
                    resp = ui.dropdown_with("dd", ANCHOR, anchor_id(), &many(), 0, &opts);
                });
            });
            (out, resp)
        };
        frame(&mut h, 20);
        for (i, k) in keys.iter().enumerate() {
            h.input.key_down(Key::Named(*k), None);
            let (out, resp) = frame(&mut h, 30 + 10 * i as u64);
            assert!(out.fired.is_empty(), "{k:?} {opts:?}");
            assert_eq!(resp.picked, None, "{k:?} {opts:?}");
        }
        let (out, _) = frame(&mut h, 100);
        let list = WidgetId::ROOT.with("dd").with("list");
        assert!(out.tree.get(list).is_some());
        assert_eq!(disabled_violations(&out.tree, list), [], "{opts:?}");
    }
}

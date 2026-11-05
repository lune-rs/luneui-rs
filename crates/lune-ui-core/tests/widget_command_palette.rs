//! The command palette (`overlay.command-palette` acceptance): the rows and their nodes, the
//! selection (two sections, wrap, kept across a change, the first row otherwise), the keys it
//! takes before the search box, activation by Enter and click, "show more", scrolling the
//! selection into view, the chord hint, shortcuts around it, the probes at scale 2, the dialog's
//! open and close curves, the disabled scope and a dialog opened from an activation.

use lune_ui_core::access::{Action, Live, Node, Role, State, Value};
use lune_ui_core::color::premul;
use lune_ui_core::input::{FrameOutput, Key, Keymap, Modifiers, NamedKey, PointerButton, Repaint};
use lune_ui_core::motion::{MotionTier, Time};
use lune_ui_core::paint::{LayerId, Painter, Rect};
use lune_ui_core::scene::Scene;
use lune_ui_core::shortcuts::{Binding, Chord, Scope};
use lune_ui_core::testing::{UiHarness, disabled_violations};
use lune_ui_core::theme::DARK;
use lune_ui_core::ui::{Ui, WidgetId};
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::command_palette::{
    CARD_W, Palette, PaletteResponse, PaletteRow, PaletteSection, RESULTS_MAX_H,
};
use lune_ui_core::widgets::dialog::{Cancel, Dialog, DialogGlass};

const WIN: (f32, f32) = (1000.0, 900.0);
const P: PointerButton = PointerButton::Primary;
const PAGE_BTN: Rect = Rect::new(20.0, 20.0, 100.0, 32.0);

fn glass() -> DialogGlass {
    DialogGlass {
        panel: [0.1, 0.1, 0.12, 1.0],
        fill: [0.2, 0.2, 0.2, 0.62],
        noise: true,
    }
}

fn pid() -> WidgetId {
    WidgetId::ROOT.with("p")
}

fn search_id() -> WidgetId {
    WidgetId::ROOT.with((pid(), "search"))
}

fn row_id(key: u64) -> WidgetId {
    pid().with("rows").with(("row", key))
}

fn more_id(section: usize) -> WidgetId {
    pid().with("rows").with(("more", section))
}

const fn row(key: u64, text: &'static str) -> PaletteRow<'static> {
    PaletteRow {
        key,
        text,
        detail: None,
        icon: None,
        hint: None,
        destructive: false,
    }
}

const FILES: [PaletteRow<'static>; 3] = [
    PaletteRow {
        detail: Some("src/"),
        ..row(1, "main.rs")
    },
    PaletteRow {
        detail: Some("src/"),
        ..row(2, "lib.rs")
    },
    row(3, "Cargo.toml"),
];

const COMMANDS: [PaletteRow<'static>; 2] = [
    PaletteRow {
        hint: Some("Cmd+R"),
        ..row(10, "Reload")
    },
    PaletteRow {
        destructive: true,
        ..row(11, "Delete cache")
    },
];

const fn sections() -> [PaletteSection<'static>; 2] {
    [
        PaletteSection {
            title: "FILES",
            rows: &FILES,
            more: true,
        },
        PaletteSection {
            title: "COMMANDS",
            rows: &COMMANDS,
            more: false,
        },
    ]
}

fn palette<'a>(sections: &'a [PaletteSection<'a>], error: Option<&'a str>) -> Palette<'a> {
    Palette {
        sections,
        placeholder: "Search files and commands",
        footer_left: "Up/Down select",
        error,
        name: "",
    }
}

/// The page under the palette: a card and a page button, to prove nothing under the modal reacts.
fn page<H: Copy + PartialEq>(p: &mut Painter<'_, H>) {
    p.on(LayerId::Page);
    p.fill(Rect::new(10.0, 10.0, 500.0, 300.0), 6.0, [30, 30, 30, 255]);
}

/// One frame with the palette open over `sections`.
fn frame(
    h: &mut UiHarness,
    q: &mut String,
    secs: &[PaletteSection<'_>],
) -> (Scene, FrameOutput, PaletteResponse) {
    frame_with(h, Some(q), secs, None)
}

fn frame_with(
    h: &mut UiHarness,
    q: Option<&mut String>,
    secs: &[PaletteSection<'_>],
    error: Option<&str>,
) -> (Scene, FrameOutput, PaletteResponse) {
    let mut r = PaletteResponse::default();
    let (scene, out) = h.frame(|ui| {
        page(ui.painter());
        ui.button("page", PAGE_BTN, "Page", ButtonKind::Secondary);
        if let Some(q) = q {
            r = ui.command_palette("p", q, &palette(secs, error), &glass());
        }
    });
    (scene, out, r)
}

/// A harness with the palette open over `secs` and settled.
fn opened(secs: &[PaletteSection<'_>]) -> (UiHarness, String) {
    let mut h = UiHarness::new(WIN);
    let mut q = String::new();
    frame(&mut h, &mut q, secs);
    // The open motion, then the search box's focus-in, take a few frames.
    h.at_ms(500);
    frame(&mut h, &mut q, secs);
    h.at_ms(1000);
    let (_, out, _) = frame(&mut h, &mut q, secs);
    assert_eq!(
        out.repaint,
        Repaint::Idle,
        "an open palette at rest asks for nothing"
    );
    (h, q)
}

/// Presses `key` and runs one frame at `ms`.
fn press(
    h: &mut UiHarness,
    q: &mut String,
    secs: &[PaletteSection<'_>],
    ms: u64,
    key: NamedKey,
) -> (Scene, FrameOutput, PaletteResponse) {
    h.input.key_down(Key::Named(key), None);
    h.at_ms(ms);
    frame(h, q, secs)
}

fn node(out: &FrameOutput, id: WidgetId) -> &Node {
    out.tree.get(id).unwrap_or_else(|| panic!("no node {id:?}"))
}

/// The names of the rows and show-more buttons drawn selected.
fn selected(out: &FrameOutput) -> Vec<String> {
    out.tree
        .nodes
        .iter()
        .filter(|n| {
            matches!(n.role, Role::ListItem | Role::Button) && n.states.contains(State::Selected)
        })
        .map(|n| n.name.clone())
        .collect()
}

fn centre(n: &Node) -> (f32, f32) {
    (n.bounds.x + n.bounds.w / 2.0, n.bounds.y + n.bounds.h / 2.0)
}

fn card(out: &FrameOutput) -> Rect {
    node(out, pid()).bounds
}

// Rows and nodes.

#[test]
fn the_sections_are_groups_and_the_rows_list_items() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    h.at_ms(1100);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let dialog = node(&out, pid());
    assert_eq!(dialog.role, Role::Dialog);
    assert_eq!(dialog.name, "Search files and commands", "the placeholder");
    assert!(dialog.states.contains(State::Modal));
    for (title, rows) in [("FILES", 3usize), ("COMMANDS", 2)] {
        let group = out
            .tree
            .nodes
            .iter()
            .find(|n| n.role == Role::Group && n.name == title)
            .unwrap_or_else(|| panic!("no group {title}"));
        let items = group
            .children
            .iter()
            .filter_map(|c| out.tree.get(*c))
            .filter(|n| n.role == Role::ListItem)
            .count();
        assert_eq!(items, rows, "{title}");
    }
    let main = node(&out, row_id(1));
    assert_eq!(main.role, Role::ListItem);
    assert_eq!(main.name, "main.rs");
    assert_eq!(main.description.as_deref(), Some("src/"));
    assert!(main.actions.contains(Action::Default));
    assert!(
        !main.states.contains(State::Focusable),
        "rows take no focus"
    );
    let text = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::TextInput)
        .expect("the search box");
    assert_eq!(text.name, "Search files and commands");
}

#[test]
fn a_card_name_overrides_the_placeholder() {
    let secs = sections();
    let mut h = UiHarness::new(WIN);
    let mut q = String::new();
    let (_, out) = h.frame(|ui| {
        let p = Palette {
            name: "Command palette",
            ..palette(&secs, None)
        };
        ui.command_palette("p", &mut q, &p, &glass());
    });
    assert_eq!(node(&out, pid()).name, "Command palette");
}

#[test]
fn a_section_with_no_rows_is_not_drawn_and_no_rows_means_a_short_card() {
    let none = [PaletteSection {
        title: "EMPTY",
        rows: &[],
        more: false,
    }];
    let (mut h, mut q) = opened(&none);
    h.at_ms(1100);
    let (_, out, _) = frame(&mut h, &mut q, &none);
    assert!(out.tree.nodes.iter().all(|n| n.role != Role::Group));
    assert!(out.tree.nodes.iter().all(|n| n.role != Role::ScrollArea));
    let short = card(&out).h;
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    h.at_ms(1100);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert!(card(&out).h > short + 100.0, "the card follows the rows");
    // 18 + 28 + 10 for the search row, the footer's 10 + 14 + 12.
    assert_eq!(short, 56.0 + 36.0);
}

#[test]
fn the_toolkit_strings_come_from_the_table() {
    let secs = sections();
    let mut h = UiHarness::new(WIN);
    h.strings
        .insert("luneui.command-palette.show-more", "Plus")
        .expect("toolkit key");
    h.strings
        .insert("luneui.command-palette.destructive-tooltip", "Careful")
        .expect("toolkit key");
    let mut q = String::new();
    frame(&mut h, &mut q, &secs);
    h.at_ms(100);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(node(&out, more_id(0)).name, "Plus");
    assert_eq!(node(&out, more_id(0)).role, Role::Button);
    assert_eq!(
        node(&out, row_id(11)).description.as_deref(),
        Some("Careful")
    );
}

// Selection and keys.

#[test]
fn the_first_row_is_selected_at_open() {
    let secs = sections();
    let (_, mut q) = opened(&secs);
    let mut h = UiHarness::new(WIN);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(selected(&out), ["main.rs"]);
}

#[test]
fn down_and_up_walk_both_sections_and_the_show_more_row_and_wrap() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    let mut names = Vec::new();
    for i in 0..6 {
        let (_, out, r) = press(&mut h, &mut q, &secs, 1100 + i * 20, NamedKey::ArrowDown);
        assert_eq!(r, PaletteResponse::default(), "moving activates nothing");
        names.push(selected(&out).join("|"));
    }
    // Rows, the section's "show more", the next section, then back to the first.
    assert_eq!(
        names,
        [
            "lib.rs",
            "Cargo.toml",
            "Show more results",
            "Reload",
            "Delete cache",
            "main.rs"
        ]
    );
    let (_, out, _) = press(&mut h, &mut q, &secs, 1300, NamedKey::ArrowUp);
    assert_eq!(selected(&out), ["Delete cache"], "Up wraps to the last");
    let (_, out, _) = press(&mut h, &mut q, &secs, 1320, NamedKey::ArrowUp);
    assert_eq!(selected(&out), ["Reload"]);
}

#[test]
fn hover_fills_without_moving_the_selection_and_a_click_activates() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let at = centre(node(&out, row_id(3)));
    h.input.pointer_to(at.0, at.1);
    h.at_ms(1100);
    let (_, out, r) = frame(&mut h, &mut q, &secs);
    assert_eq!(selected(&out), ["main.rs"], "hover leaves the selection");
    assert_eq!(r.activated, None);
    h.input.press(P);
    h.at_ms(1120);
    frame(&mut h, &mut q, &secs);
    h.input.release(P);
    h.at_ms(1140);
    let (_, _, r) = frame(&mut h, &mut q, &secs);
    assert_eq!(r.activated, Some(3));
    assert_eq!(r.show_more, None);
}

#[test]
fn enter_activates_the_selected_row_once() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    press(&mut h, &mut q, &secs, 1100, NamedKey::ArrowDown);
    let (_, _, r) = press(&mut h, &mut q, &secs, 1120, NamedKey::Enter);
    assert_eq!(r.activated, Some(2));
    h.at_ms(1140);
    let (_, _, r) = frame(&mut h, &mut q, &secs);
    assert_eq!(r.activated, None);
}

#[test]
fn down_then_enter_in_one_frame_activates_the_row_down_chose() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    h.input.key_down(Key::Named(NamedKey::ArrowDown), None);
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    h.at_ms(1100);
    let (_, _, r) = frame(&mut h, &mut q, &secs);
    assert_eq!(r.activated, Some(3));
}

#[test]
fn show_more_reports_its_section_by_enter_and_by_click() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    for i in 0..3 {
        press(&mut h, &mut q, &secs, 1100 + i * 20, NamedKey::ArrowDown);
    }
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(selected(&out), ["Show more results"]);
    let (_, _, r) = press(&mut h, &mut q, &secs, 1300, NamedKey::Enter);
    assert_eq!((r.activated, r.show_more), (None, Some(0)));

    let (mut h, mut q) = opened(&secs);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let at = centre(node(&out, more_id(0)));
    h.input.pointer_to(at.0, at.1);
    h.at_ms(1100);
    frame(&mut h, &mut q, &secs);
    h.input.press(P);
    h.at_ms(1120);
    frame(&mut h, &mut q, &secs);
    h.input.release(P);
    h.at_ms(1140);
    let (_, out, r) = frame(&mut h, &mut q, &secs);
    assert_eq!((r.activated, r.show_more), (None, Some(0)));
    assert_eq!(
        selected(&out),
        ["main.rs"],
        "the click frame draws the old selection"
    );
    h.at_ms(1160);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(selected(&out), ["Show more results"], "a click selects too");
}

#[test]
fn enter_with_no_rows_does_nothing_and_is_consumed() {
    let none: [PaletteSection<'static>; 0] = [];
    let mut h = UiHarness::new(WIN);
    h.shortcuts.add(Binding::new(
        Scope::Overlay(pid()),
        Chord::key(Key::Named(NamedKey::Enter)),
        "overlay-enter",
    ));
    let mut q = String::new();
    frame(&mut h, &mut q, &none);
    h.at_ms(1000);
    frame(&mut h, &mut q, &none);
    for key in [NamedKey::Enter, NamedKey::ArrowDown, NamedKey::ArrowUp] {
        h.input.key_down(Key::Named(key), None);
        h.at_ms(1100);
        let (_, out, r) = frame(&mut h, &mut q, &none);
        assert_eq!(r, PaletteResponse::default(), "{key:?}");
        assert!(out.fired.is_empty(), "{key:?} reached no binding");
    }
    // The same Enter on a palette with rows is taken too.
    let secs = sections();
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    h.at_ms(1200);
    let (_, out, r) = frame(&mut h, &mut q, &secs);
    assert_eq!(r.activated, Some(1));
    assert!(out.fired.is_empty());
}

#[test]
fn typing_edits_the_query_and_up_down_enter_do_not_move_the_caret() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    for (i, c) in "main".chars().enumerate() {
        let s = c.to_string();
        h.input.key_down(Key::Character(s.clone()), Some(&s));
        h.at_ms(1100 + i as u64 * 20);
        frame(&mut h, &mut q, &secs);
    }
    assert_eq!(q, "main");
    h.input.key_down(Key::Named(NamedKey::ArrowLeft), None);
    h.at_ms(1300);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let caret = |out: &FrameOutput| {
        out.tree
            .nodes
            .iter()
            .find(|n| n.role == Role::TextInput)
            .and_then(|n| n.text.clone())
            .expect("the field's text")
    };
    let before = caret(&out);
    assert_eq!(before.text, "main");
    assert_eq!(before.selection, (3, 3));
    for (i, key) in [NamedKey::ArrowUp, NamedKey::ArrowDown, NamedKey::Enter]
        .into_iter()
        .enumerate()
    {
        let (_, out, _) = press(&mut h, &mut q, &secs, 1400 + i as u64 * 20, key);
        assert_eq!(caret(&out), before, "{key:?} left the caret alone");
    }
    // Home and End still belong to the field.
    h.input.key_down(Key::Named(NamedKey::End), None);
    h.at_ms(1500);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(caret(&out).selection, (4, 4));
}

#[test]
fn a_changed_query_is_reported_once() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    h.input.key_down(Key::Character("x".into()), Some("x"));
    h.at_ms(1100);
    let (_, _, r) = frame(&mut h, &mut q, &secs);
    assert!(r.changed);
    h.at_ms(1120);
    let (_, _, r) = frame(&mut h, &mut q, &secs);
    assert!(!r.changed);
}

#[test]
fn the_first_row_is_selected_when_the_rows_change_and_a_present_key_stays() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    for i in 0..4 {
        press(&mut h, &mut q, &secs, 1100 + i * 20, NamedKey::ArrowDown);
    }
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(selected(&out), ["Reload"]);

    // A query narrows the rows: the selected key is still there, so it stays.
    let kept = [PaletteSection {
        title: "COMMANDS",
        rows: &COMMANDS,
        more: false,
    }];
    h.at_ms(1300);
    let (_, out, _) = frame(&mut h, &mut q, &kept);
    assert_eq!(selected(&out), ["Reload"], "a kept selection");

    // Rows without it: the first row.
    let others = [PaletteSection {
        title: "FILES",
        rows: &FILES[1..],
        more: false,
    }];
    h.at_ms(1320);
    let (_, out, _) = frame(&mut h, &mut q, &others);
    assert_eq!(selected(&out), ["lib.rs"]);
    // Back to everything: lib.rs is in it, so it stays.
    h.at_ms(1340);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(selected(&out), ["lib.rs"]);

    // No rows at all: nothing selected; rows again: the first.
    let none: [PaletteSection<'static>; 0] = [];
    h.at_ms(1360);
    let (_, out, _) = frame(&mut h, &mut q, &none);
    assert!(selected(&out).is_empty());
    h.at_ms(1380);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(selected(&out), ["main.rs"]);
}

// Scrolling.

fn many() -> Vec<PaletteRow<'static>> {
    static NAMES: [&str; 40] = [
        "row 00", "row 01", "row 02", "row 03", "row 04", "row 05", "row 06", "row 07", "row 08",
        "row 09", "row 10", "row 11", "row 12", "row 13", "row 14", "row 15", "row 16", "row 17",
        "row 18", "row 19", "row 20", "row 21", "row 22", "row 23", "row 24", "row 25", "row 26",
        "row 27", "row 28", "row 29", "row 30", "row 31", "row 32", "row 33", "row 34", "row 35",
        "row 36", "row 37", "row 38", "row 39",
    ];
    NAMES
        .iter()
        .enumerate()
        .map(|(i, t)| row(100 + i as u64, t))
        .collect()
}

#[test]
fn the_selected_row_is_in_view_the_frame_it_is_chosen() {
    let rows = many();
    let secs = [PaletteSection {
        title: "ROWS",
        rows: &rows,
        more: false,
    }];
    let mut h = UiHarness::new(WIN);
    h.tier = MotionTier::Off;
    let mut q = String::new();
    frame(&mut h, &mut q, &secs);
    h.at_ms(1000);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let area = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::ScrollArea)
        .expect("the results")
        .bounds;
    assert_eq!(area.h, RESULTS_MAX_H);
    let in_view = |out: &FrameOutput, key: u64| {
        let r = node(out, row_id(key)).bounds;
        r.y >= area.y - 0.5 && r.bottom() <= area.bottom() + 0.5
    };
    assert!(in_view(&out, 100));
    // The reveal is asked for in the frame the key is taken, and the scroll area applies it after
    // its content drew: that frame asks for the next at once (`Repaint::Now`), which shows it.
    let choose = |h: &mut UiHarness, q: &mut String, ms: u64, key: NamedKey| {
        let (_, first, _) = press(h, q, &secs, ms, key);
        assert_eq!(first.repaint, Repaint::Now, "{key:?} at {ms}");
        let (_, out, _) = frame(h, q, &secs);
        out
    };
    // Up from the first wraps to the last, which is far below.
    let out = choose(&mut h, &mut q, 1100, NamedKey::ArrowUp);
    assert_eq!(selected(&out), ["row 39"]);
    assert!(in_view(&out, 139), "the last row is revealed");
    // Down wraps back to the top.
    let out = choose(&mut h, &mut q, 1120, NamedKey::ArrowDown);
    assert_eq!(selected(&out), ["row 00"]);
    assert!(in_view(&out, 100));
    // Walk down past the bottom edge, one row at a time: each is in view when chosen.
    for i in 1..25u64 {
        let out = choose(&mut h, &mut q, 1200 + i * 20, NamedKey::ArrowDown);
        assert!(in_view(&out, 100 + i), "row {i}");
    }
}

#[test]
fn the_selected_row_scrolls_into_view_with_motion_too() {
    let rows = many();
    let secs = [PaletteSection {
        title: "ROWS",
        rows: &rows,
        more: false,
    }];
    let (mut h, mut q) = opened(&secs);
    press(&mut h, &mut q, &secs, 1100, NamedKey::ArrowUp);
    // The reveal is a scroll animation: settled well inside a second.
    h.at_ms(2000);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let area = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::ScrollArea)
        .expect("results")
        .bounds;
    let r = node(&out, row_id(139)).bounds;
    assert!(r.y >= area.y - 0.5 && r.bottom() <= area.bottom() + 0.5);
}

// Hints, shortcuts and the scrim.

#[test]
fn a_hint_is_the_chord_hint_and_reaches_the_node() {
    let chord = Chord::key(Key::Character("r".into())).primary();
    assert_eq!(chord.hint(&Keymap::MAC), "Cmd+R");
    assert_eq!(chord.hint(&Keymap::PC), "Ctrl+R");
    let mac = chord.hint(&Keymap::MAC);
    let rows = [PaletteRow {
        hint: Some(&mac),
        ..row(1, "Reload")
    }];
    let secs = [PaletteSection {
        title: "COMMANDS",
        rows: &rows,
        more: false,
    }];
    let (mut h, mut q) = opened(&secs);
    h.at_ms(1100);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(
        node(&out, row_id(1)).value,
        Some(Value::Text("Cmd+R".into()))
    );
}

#[test]
fn a_binding_opens_the_palette_and_only_overlay_bindings_fire_inside_it() {
    let secs = sections();
    let mut h = UiHarness::new(WIN);
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Character("k".into())).primary(),
        "palette.open",
    ));
    h.shortcuts.add(Binding::new(
        Scope::Global,
        Chord::key(Key::Character("x".into())),
        "global-x",
    ));
    h.shortcuts.add(Binding::new(
        Scope::Overlay(pid()),
        Chord::key(Key::Named(NamedKey::F5)),
        "palette.refresh",
    ));
    let mut q = String::new();
    let mut open = false;

    // Closed: Cmd+K fires; the app opens the palette from `fired`.
    h.input.set_modifiers(Modifiers {
        logo: true,
        ..Modifiers::default()
    });
    h.input.key_down(Key::Character("k".into()), None);
    h.input.set_modifiers(Modifiers::default());
    h.at_ms(10);
    let (_, out, _) = frame_with(&mut h, None, &secs, None);
    assert_eq!(out.fired.len(), 1);
    open |= out.fired[0].as_str() == "palette.open";
    assert!(open);

    // Open: settle, then a Global binding on a typed letter does not fire: the letter is typed.
    frame(&mut h, &mut q, &secs);
    h.at_ms(1000);
    frame(&mut h, &mut q, &secs);
    h.input.key_down(Key::Character("x".into()), Some("x"));
    h.at_ms(1100);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert!(out.fired.is_empty(), "{:?}", out.fired);
    assert_eq!(q, "x");

    // An overlay binding fires inside the palette.
    h.input.key_down(Key::Named(NamedKey::F5), None);
    h.at_ms(1200);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(out.fired.len(), 1);
    assert_eq!(out.fired[0].as_str(), "palette.refresh");
    // The global opener does not fire while it is open.
    h.input.set_modifiers(Modifiers {
        logo: true,
        ..Modifiers::default()
    });
    h.input.key_down(Key::Character("k".into()), None);
    h.input.set_modifiers(Modifiers::default());
    h.at_ms(1300);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert!(out.fired.is_empty());
}

#[test]
fn escape_and_a_scrim_click_dismiss_once_and_a_card_click_does_not() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    let (_, _, r) = press(&mut h, &mut q, &secs, 1100, NamedKey::Escape);
    assert!(r.dismissed);
    h.at_ms(1120);
    let (_, _, r) = frame(&mut h, &mut q, &secs);
    assert!(!r.dismissed, "once");

    let (mut h, mut q) = opened(&secs);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let c = card(&out);
    // On the card's footer: nothing.
    h.input.pointer_to(c.x + 40.0, c.bottom() - 10.0);
    h.at_ms(1100);
    frame(&mut h, &mut q, &secs);
    h.input.press(P);
    h.at_ms(1120);
    frame(&mut h, &mut q, &secs);
    h.input.release(P);
    h.at_ms(1140);
    let (_, _, r) = frame(&mut h, &mut q, &secs);
    assert!(!r.dismissed && r.activated.is_none());
    // On the scrim, over the page button: dismisses, and the page stays inert.
    h.input.pointer_to(30.0, 30.0);
    h.at_ms(1200);
    frame(&mut h, &mut q, &secs);
    h.input.press(P);
    h.at_ms(1220);
    frame(&mut h, &mut q, &secs);
    h.input.release(P);
    h.at_ms(1240);
    let (_, _, r) = frame(&mut h, &mut q, &secs);
    assert!(r.dismissed);
}

#[test]
fn tab_stays_in_the_card_and_never_lands_on_a_row() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(out.focused, Some(search_id()));
    for i in 0..4 {
        let (_, out, _) = press(&mut h, &mut q, &secs, 1100 + i * 20, NamedKey::Tab);
        assert_eq!(
            out.focused,
            Some(search_id()),
            "one stop with an empty query"
        );
    }
    q.push_str("ma");
    let mut seen = Vec::new();
    for i in 0..4 {
        let (_, out, _) = press(&mut h, &mut q, &secs, 1300 + i * 20, NamedKey::Tab);
        let f = out.focused.expect("focus stays in the card");
        assert!(
            [search_id(), search_id().with("clear")].contains(&f),
            "{f:?}"
        );
        seen.push(f);
    }
    assert_ne!(seen[0], seen[1], "the clear button is the only other stop");
}

#[test]
fn the_page_under_the_palette_stays_inert() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    h.input.pointer_to(30.0, 30.0);
    h.at_ms(1100);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let page_button = out
        .tree
        .nodes
        .iter()
        .find(|n| n.name == "Page")
        .expect("page button");
    assert!(!page_button.states.contains(State::Focused));
}

#[test]
fn the_error_line_is_a_polite_status_above_the_results() {
    let secs = sections();
    let mut h = UiHarness::new(WIN);
    let mut q = String::new();
    let (_, out, _) = frame_with(
        &mut h,
        Some(&mut q),
        &secs,
        Some("Could not parse the filter"),
    );
    h.at_ms(500);
    let (_, out2, _) = frame_with(
        &mut h,
        Some(&mut q),
        &secs,
        Some("Could not parse the filter"),
    );
    let _ = out;
    let status = out2
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::Status)
        .expect("the error line");
    assert_eq!(status.name, "Could not parse the filter");
    assert_eq!(status.live, Some(Live::Polite));
    let area = out2
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::ScrollArea)
        .expect("results");
    assert!(status.bounds.bottom() <= area.bounds.y, "above the results");
    let search = node(&out2, search_id()).bounds;
    assert!(status.bounds.y >= search.bottom(), "under the search box");
    // Without an error there is no status node.
    h.at_ms(600);
    let (_, out3, _) = frame(&mut h, &mut q, &secs);
    assert!(out3.tree.nodes.iter().all(|n| n.role != Role::Status));
}

#[test]
fn the_destructive_row_describes_itself_shows_a_tooltip_above_the_card_and_still_activates() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    h.at_ms(1100);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let n = node(&out, row_id(11));
    assert_eq!(
        n.description.as_deref(),
        Some("Destructive action \u{2014} requires confirmation")
    );
    let at = centre(n);
    h.input.pointer_to(at.0, at.1);
    h.at_ms(1200);
    frame(&mut h, &mut q, &secs);
    h.at_ms(2100);
    let (scene, out, _) = frame(&mut h, &mut q, &secs);
    assert!(
        out.tree.nodes.iter().any(|n| n.role == Role::Tooltip),
        "the tooltip shows after the dwell"
    );
    // Above the modal: on the Raised layer, not the Popup layer.
    let raised = &scene.layers[LayerId::Raised as usize];
    assert!(!raised.geometry.acrylic.is_empty() || !raised.geometry.under.is_empty());
    assert!(
        scene.layers[LayerId::Popup as usize]
            .geometry
            .under
            .is_empty()
    );
    // It still reports the activation.
    h.input.press(P);
    h.at_ms(2120);
    frame(&mut h, &mut q, &secs);
    h.input.release(P);
    h.at_ms(2140);
    let (_, _, r) = frame(&mut h, &mut q, &secs);
    assert_eq!(r.activated, Some(11));
}

// Probes.

#[test]
fn the_card_is_640_wide_and_the_results_stop_at_560_at_scale_two() {
    let rows = many();
    let secs = [PaletteSection {
        title: "ROWS",
        rows: &rows,
        more: false,
    }];
    let mut h = UiHarness::new(WIN);
    h.input.set_scale(2.0);
    let mut q = String::new();
    frame(&mut h, &mut q, &secs);
    h.at_ms(1000);
    let (scene, out, _) = frame(&mut h, &mut q, &secs);
    let modal = &scene.layers[LayerId::Modal as usize];
    let acrylic = modal.geometry.acrylic[0];
    assert_eq!(acrylic.shape.rect[2], 1280.0, "640 px wide");
    assert!(acrylic.shape.rect[3] <= 1440.0, "at most 720 px high");
    assert_eq!(acrylic.shape.radii, [16.0; 4]);
    assert_eq!(CARD_W * 2.0, 1280.0);
    let area = out
        .tree
        .nodes
        .iter()
        .find(|n| n.role == Role::ScrollArea)
        .expect("results")
        .bounds;
    assert_eq!(area.h * 2.0, 1120.0, "the results stop at 560 px");
    assert_eq!(area.w, 604.0, "inside the 18 px margins");
    // The scrim is the dialog's.
    let scrim = modal.geometry.under[0];
    assert_eq!(scrim.rect, [0.0, 0.0, WIN.0 * 2.0, WIN.1 * 2.0]);
    assert_eq!(scrim.color, premul(DARK.dialog_backdrop));
    // The search bar is 28 px high, the footer block draws its fill at the bottom.
    let search = node(&out, search_id()).bounds;
    assert_eq!(search.h, 28.0);
    assert_eq!(
        node(&out, pid()).bounds.x,
        (WIN.0 - CARD_W) / 2.0,
        "centred"
    );
    assert!(
        modal
            .geometry
            .under
            .iter()
            .any(|s| s.radii == [0.0, 0.0, 16.0, 16.0]),
        "the footer block"
    );
}

#[test]
fn a_short_window_keeps_the_card_inside_it() {
    let rows = many();
    let secs = [PaletteSection {
        title: "ROWS",
        rows: &rows,
        more: false,
    }];
    let mut h = UiHarness::new((1000.0, 400.0));
    let mut q = String::new();
    frame(&mut h, &mut q, &secs);
    h.at_ms(1000);
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    let c = card(&out);
    assert!(c.y >= 0.0 && c.bottom() <= 400.0, "{c:?}");
}

// Motion.

/// The scrim's opacity as a fraction of rest, and the card's scale (the first text's glyph scale).
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

/// The palette's open (or close) samples on the 60 Hz grid under `tier`, against a dialog's.
fn curves(tier: MotionTier, open: bool, palette: bool) -> Vec<(f32, f32)> {
    let secs = sections();
    let d = Dialog {
        title: "Discard changes",
        message: "The document has unsaved changes.",
        extra_h: 0.0,
        confirm: "Discard",
        cancel: Cancel::Default,
    };
    let mut h = UiHarness::new(WIN);
    h.tier = tier;
    let mut q = String::new();
    let draw = |h: &mut UiHarness, q: &mut String, on: bool| {
        h.frame(|ui: &mut Ui<'_>| {
            if on {
                if palette {
                    ui.command_palette("p", q, &palette_of(&secs), &glass());
                } else {
                    ui.dialog("p", &d, &glass(), |_, _, _, _| {});
                }
            }
        })
    };
    let t0: u64 = if open { 0 } else { 5000 };
    if !open {
        draw(&mut h, &mut q, true);
        h.at_ms(2000);
        draw(&mut h, &mut q, true);
    }
    (0..40u64)
        .map(|k| {
            let at = Time::from_nanos(k * 1_000_000_000 / 60) + Time::from_millis(t0);
            h.input.set_time(at);
            let (scene, _) = draw(&mut h, &mut q, open);
            measure(&scene)
        })
        .collect()
}

fn palette_of<'a>(secs: &'a [PaletteSection<'a>]) -> Palette<'a> {
    palette(secs, None)
}

#[test]
fn opening_and_closing_follow_the_dialogs_curves_under_every_tier() {
    for tier in [MotionTier::Full, MotionTier::Reduced, MotionTier::Off] {
        for open in [true, false] {
            let a = curves(tier, open, true);
            let b = curves(tier, open, false);
            assert_eq!(a, b, "{tier:?} open={open}");
            if tier == MotionTier::Full && open {
                assert!(a.iter().any(|s| s.0 > 0.0 && s.0 < 1.0), "it moves");
            }
        }
    }
}

#[test]
fn a_closing_palette_keeps_drawing_its_rows_then_vanishes() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    h.at_ms(2000);
    let (open_scene, _, _) = frame(&mut h, &mut q, &secs);
    let open_texts = open_scene.layers[LayerId::Modal as usize].texts.len();
    assert!(open_texts > 8, "{open_texts}");
    h.at_ms(3000);
    let (first, out, _) = frame_with(&mut h, None, &secs, None);
    assert_eq!(out.repaint, Repaint::Now);
    assert_eq!(
        first.layers[LayerId::Modal as usize].texts.len(),
        open_texts,
        "the rows and the footer draw in the first closing frame"
    );
    h.at_ms(3090);
    let (mid, out, _) = frame_with(&mut h, None, &secs, None);
    assert_eq!(out.repaint, Repaint::Now);
    assert_eq!(mid.layers[LayerId::Modal as usize].texts.len(), open_texts);
    assert!(
        mid.layers[LayerId::Modal as usize]
            .texts
            .iter()
            .all(|t| t.scale < 1.0),
        "the card shrinks about its centre"
    );
    h.at_ms(3180);
    let (end, out, _) = frame_with(&mut h, None, &secs, None);
    assert_eq!(out.repaint, Repaint::Idle);
    assert!(
        end.layers[LayerId::Modal as usize]
            .geometry
            .under
            .is_empty()
    );
}

// The disabled scope and a dialog from an activation.

#[test]
fn a_disabled_palette_takes_its_keys_activates_nothing_and_passes_the_walk() {
    let secs = sections();
    let mut h = UiHarness::new(WIN);
    for key in [NamedKey::Enter, NamedKey::ArrowDown, NamedKey::ArrowUp] {
        h.shortcuts.add(Binding::new(
            Scope::Overlay(pid()),
            Chord::key(Key::Named(key)),
            format!("ov-{key:?}"),
        ));
    }
    let mut q = String::new();
    // Open enabled, so the search box holds the key focus; then disable it.
    frame(&mut h, &mut q, &secs);
    h.at_ms(1000);
    frame(&mut h, &mut q, &secs);
    let mut last = None;
    for (i, key) in [NamedKey::Enter, NamedKey::ArrowDown, NamedKey::ArrowUp]
        .into_iter()
        .enumerate()
    {
        h.input.key_down(Key::Named(key), None);
        h.at_ms(1100 + i as u64 * 20);
        let mut r = PaletteResponse::default();
        let (_, out) = h.frame(|ui| {
            page(ui.painter());
            r = ui.with_enabled(false, |ui| {
                ui.command_palette("p", &mut q, &palette(&secs, Some("Bad filter")), &glass())
            });
        });
        assert_eq!(r, PaletteResponse::default(), "{key:?}");
        assert!(
            out.fired.is_empty(),
            "{key:?} was consumed: {:?}",
            out.fired
        );
        last = Some(out);
    }
    let out = last.expect("a frame ran");
    assert_eq!(disabled_violations(&out.tree, pid()), []);
    assert!(
        out.tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::ListItem)
            .count()
            == 5,
        "the rows are still in the walk"
    );
}

#[test]
fn a_dialog_opened_from_an_activation_draws_above_and_takes_the_keys() {
    let secs = sections();
    let (mut h, mut q) = opened(&secs);
    for i in 0..5 {
        press(&mut h, &mut q, &secs, 1100 + i * 20, NamedKey::ArrowDown);
    }
    let (_, out, _) = frame(&mut h, &mut q, &secs);
    assert_eq!(selected(&out), ["Delete cache"]);
    // Enter activates the destructive row; the app opens a confirm dialog and keeps the palette.
    let confirm = Dialog {
        title: "Delete the cache",
        message: "This cannot be undone.",
        extra_h: 0.0,
        confirm: "Delete",
        cancel: Cancel::Default,
    };
    let mut confirmed = 0;
    let mut cancelled = 0;
    let mut activated = Vec::new();
    let mut dialog_open = false;
    let mut last_scene = None;
    let mut last_out = None;
    h.input.key_down(Key::Named(NamedKey::Enter), None);
    for i in 0..60u64 {
        h.at_ms(1500 + i * 20);
        if i == 20 {
            // After the dialog settled: Enter confirms it, not the palette's row.
            h.input.key_down(Key::Named(NamedKey::Enter), None);
        }
        let (scene, out) = h.frame(|ui| {
            page(ui.painter());
            let r = ui.command_palette("p", &mut q, &palette(&secs, None), &glass());
            if let Some(k) = r.activated {
                activated.push(k);
                dialog_open = true;
            }
            if dialog_open {
                let d = ui.dialog("confirm", &confirm, &glass(), |_, _, _, _| {});
                confirmed += u32::from(d.confirmed);
                cancelled += u32::from(d.cancelled);
                if d.confirmed || d.cancelled {
                    dialog_open = false;
                }
            }
        });
        if i == 10 {
            last_scene = Some(scene);
            last_out = Some(out);
        }
    }
    assert_eq!(activated, [11], "the palette reported the row once");
    assert_eq!((confirmed, cancelled), (1, 0), "Enter confirmed the dialog");
    let scene = last_scene.expect("a frame ran");
    let out = last_out.expect("a frame ran");
    // While it was open the dialog was above the palette's card.
    assert_eq!(
        out.tree
            .nodes
            .iter()
            .filter(|n| n.role == Role::Dialog)
            .count(),
        2
    );
    let raised = &scene.layers[LayerId::Raised as usize];
    assert!(
        !raised.geometry.acrylic.is_empty(),
        "the dialog's card is on Raised"
    );
}

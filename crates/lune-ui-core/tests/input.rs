//! The input seam in core: events the toolkit only carries (file drops, D20) reach the app through
//! `ui.input()` and change nothing the frame draws or owns.

use std::path::PathBuf;

use lune_ui_core::input::{Event, Repaint};
use lune_ui_core::paint::Rect;
use lune_ui_core::testing::UiHarness;
use lune_ui_core::ui::WidgetId;
use lune_ui_core::widgets::button::ButtonKind;
use lune_ui_core::widgets::field::FieldState;

const BUTTON: Rect = Rect::new(20.0, 20.0, 100.0, 32.0);
const FIELD: Rect = Rect::new(20.0, 70.0, 200.0, 32.0);

struct App {
    text: String,
    toggled: bool,
    clicks: u32,
    seen: Vec<Event>,
}

fn frame(
    h: &mut UiHarness,
    app: &mut App,
) -> (lune_ui_core::scene::Scene, Repaint, Option<WidgetId>) {
    let (scene, out) = h.frame(|ui| {
        app.seen = ui.input().events.clone();
        app.clicks += u32::from(ui.button("b", BUTTON, "Go", ButtonKind::Secondary).clicked);
        ui.text_field("f", FIELD, &mut app.text, "Name", FieldState::default());
        ui.toggle(
            "t",
            Rect::new(20.0, 120.0, 40.0, 24.0),
            &mut app.toggled,
            "On",
        );
    });
    (scene, out.repaint, out.focused)
}

fn app() -> App {
    App {
        text: "hello".to_owned(),
        toggled: false,
        clicks: 0,
        seen: Vec::new(),
    }
}

#[test]
fn file_events_reach_the_app_and_change_no_widget_state() {
    // Two windows with the same history; one also gets file events over the button and the field.
    let (mut quiet, mut busy) = (
        UiHarness::new((400.0, 300.0)),
        UiHarness::new((400.0, 300.0)),
    );
    let (mut qa, mut ba) = (app(), app());
    for (h, a) in [(&mut quiet, &mut qa), (&mut busy, &mut ba)] {
        h.at_ms(0);
        frame(h, a);
        h.at_ms(1000);
        frame(h, a);
    }

    // Focus the field in both, then settle.
    for h in [&mut quiet, &mut busy] {
        h.input
            .pointer_to(30.0, 85.0)
            .click(lune_ui_core::input::PointerButton::Primary);
    }
    for t in [1100, 2100] {
        quiet.at_ms(t);
        busy.at_ms(t);
        let q = frame(&mut quiet, &mut qa);
        let b = frame(&mut busy, &mut ba);
        assert_eq!(q.2, b.2);
    }

    busy.input
        .pointer_to(60.0, 36.0)
        .file_hover("/tmp/a.txt")
        .file_hover("/tmp/b.txt")
        .file_hover_cancelled()
        .file_drop("/tmp/a.txt");
    quiet.input.pointer_to(60.0, 36.0);
    busy.at_ms(3000);
    quiet.at_ms(3000);
    let b = frame(&mut busy, &mut ba);
    let q = frame(&mut quiet, &mut qa);

    // The app sees all five events in order, with the pointer position.
    let at = (60.0, 36.0);
    assert!(matches!(ba.seen.first(), Some(Event::PointerMoved { .. })));
    assert_eq!(
        ba.seen[1..],
        [
            Event::FileHover {
                pos: at,
                path: PathBuf::from("/tmp/a.txt")
            },
            Event::FileHover {
                pos: at,
                path: PathBuf::from("/tmp/b.txt")
            },
            Event::FileHoverCancelled,
            Event::FileDrop {
                pos: at,
                path: PathBuf::from("/tmp/a.txt")
            },
        ]
    );
    // Nothing a widget owns changed: same scene, same focus, same widget state, and the files
    // asked for no frame.
    assert_eq!(b.0, q.0, "the same scene");
    assert_eq!(b.2, q.2, "the same focus");
    assert_eq!(b.1, q.1, "the same repaint request");
    assert_eq!(
        (ba.text.as_str(), ba.toggled, ba.clicks),
        (qa.text.as_str(), qa.toggled, qa.clicks)
    );
    assert_eq!(
        (ba.text.as_str(), ba.toggled, ba.clicks),
        ("hello", false, 0)
    );
}

#[test]
fn a_file_event_alone_asks_for_no_repaint() {
    let mut h = UiHarness::new((400.0, 300.0));
    let mut a = app();
    h.at_ms(0);
    frame(&mut h, &mut a);
    h.at_ms(1000);
    let (_, repaint, _) = frame(&mut h, &mut a);
    assert_eq!(repaint, Repaint::Idle);
    h.input.file_drop("/tmp/x");
    h.at_ms(1016);
    let (_, repaint, _) = frame(&mut h, &mut a);
    assert_eq!(repaint, Repaint::Idle);
    assert_eq!(a.seen.len(), 1);
}

// A downstream crate must be able to match every `#[non_exhaustive]` enum of the seam with a
// wildcard arm and keep compiling when a variant is added. This file is outside the crate, so
// each match below compiles only because the wildcard arm is there (and, were a variant added, only
// because the enum is `non_exhaustive`).

fn event_kind(e: &Event) -> &'static str {
    match e {
        Event::PointerLeft => "pointer-left",
        Event::PointerMoved { .. } => "pointer-moved",
        _ => "other",
    }
}

fn named_key_kind(k: lune_ui_core::input::NamedKey) -> &'static str {
    use lune_ui_core::input::NamedKey;
    match k {
        NamedKey::Enter => "enter",
        NamedKey::Escape => "escape",
        _ => "other",
    }
}

fn window_command_kind(c: lune_ui_core::input::WindowCommand) -> &'static str {
    use lune_ui_core::input::WindowCommand;
    match c {
        WindowCommand::Close => "close",
        WindowCommand::Minimize => "minimize",
        _ => "other",
    }
}

fn cursor_kind(c: lune_ui_core::input::CursorIcon) -> &'static str {
    use lune_ui_core::input::CursorIcon;
    match c {
        CursorIcon::Default => "default",
        CursorIcon::Text => "text",
        _ => "other",
    }
}

#[test]
fn non_exhaustive_seam_enums_match_with_a_wildcard_downstream() {
    use lune_ui_core::input::{CursorIcon, NamedKey, WindowCommand};
    assert_eq!(event_kind(&Event::PointerLeft), "pointer-left");
    assert_eq!(
        event_kind(&Event::PointerMoved { pos: (1.0, 2.0) }),
        "pointer-moved"
    );
    assert_eq!(event_kind(&Event::Focus(true)), "other");
    assert_eq!(named_key_kind(NamedKey::Enter), "enter");
    assert_eq!(named_key_kind(NamedKey::Tab), "other");
    assert_eq!(window_command_kind(WindowCommand::Close), "close");
    assert_eq!(window_command_kind(WindowCommand::Redraw), "other");
    assert_eq!(cursor_kind(CursorIcon::Default), "default");
    assert_eq!(cursor_kind(CursorIcon::Wait), "other");
}

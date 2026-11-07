//! The input seam's contract: winit's `Translator` and the `ScriptedInput` fake produce equal
//! `RawInput`s for the same script, plus the winit mapping tables.

use std::time::Duration;

use lune_ui_core::input::{CursorIcon as C, ImeEvent, Key, Modifiers, NamedKey, PointerButton};
use lune_ui_core::testing::ScriptedInput;
use lune_ui_core::window::ResizeEdge;
use lune_ui_winit::input::{Translator, cursor_icon};
use lune_ui_winit::window::resize_direction;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{
    DeviceId, ElementState::Pressed, ElementState::Released, Ime, MouseButton, MouseScrollDelta,
    TouchPhase, WindowEvent,
};
use winit::keyboard::{Key as WKey, ModifiersState, NamedKey as WNamed};
use winit::window::{CursorIcon as WC, ResizeDirection as D};

const FRAME: Duration = Duration::from_millis(16);

fn scripted() -> ScriptedInput {
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    let ctrl = Modifiers {
        control: true,
        ..Modifiers::default()
    };
    let mut s = ScriptedInput::new((400.0, 300.0), 2.0);
    s.set_time(FRAME)
        .pointer_to(100.0, 50.0)
        .set_modifiers(shift)
        .click(PointerButton::Primary)
        .scroll_lines(0.0, -1.0)
        .scroll_pixels(0.0, -15.0)
        .key_down(Key::Named(NamedKey::Escape), None)
        .key_up(Key::Named(NamedKey::Escape))
        .key_down(Key::Character("a".into()), Some("a"))
        .set_modifiers(ctrl)
        .key_down(Key::Character("a".into()), Some("\u{1}"))
        .ime(ImeEvent::Enabled)
        .ime(ImeEvent::Preedit {
            text: "か".into(),
            cursor: Some((0, 3)),
        })
        .ime(ImeEvent::Commit("か".into()))
        .ime(ImeEvent::Disabled)
        .focus(false)
        .pointer_leave();
    s
}

fn translated() -> lune_ui_core::input::RawInput {
    let id = DeviceId::dummy();
    let mut t = Translator::new(PhysicalSize::new(1, 1), 2.0);
    t.window_event(&WindowEvent::Resized(PhysicalSize::new(800, 600)));
    t.window_event(&WindowEvent::CursorMoved {
        device_id: id,
        position: PhysicalPosition::new(200.0, 100.0),
    });
    t.window_event(&WindowEvent::ModifiersChanged(ModifiersState::SHIFT.into()));
    for state in [Pressed, Released] {
        t.window_event(&WindowEvent::MouseInput {
            device_id: id,
            state,
            button: MouseButton::Left,
        });
    }
    for delta in [
        MouseScrollDelta::LineDelta(0.0, -1.0),
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -30.0)),
    ] {
        t.window_event(&WindowEvent::MouseWheel {
            device_id: id,
            delta,
            phase: TouchPhase::Moved,
        });
    }
    let esc = WKey::Named(WNamed::Escape);
    t.key(&esc, Pressed, false, None);
    t.key(&esc, Released, false, None);
    let a = WKey::Character("a".into());
    t.key(&a, Pressed, false, Some("a"));
    t.window_event(&WindowEvent::ModifiersChanged(
        ModifiersState::CONTROL.into(),
    ));
    t.key(&a, Pressed, false, Some("\u{1}"));
    for ime in [
        Ime::Enabled,
        Ime::Preedit("か".into(), Some((0, 3))),
        Ime::Commit("か".into()),
        Ime::Disabled,
    ] {
        t.window_event(&WindowEvent::Ime(ime));
    }
    t.window_event(&WindowEvent::Focused(false));
    t.window_event(&WindowEvent::CursorLeft { device_id: id });
    t.take(FRAME)
}

#[test]
fn translator_and_scripted_input_agree() {
    let a = translated();
    let b = scripted().end_frame();
    assert_eq!(a, b);
    assert_eq!(a.size, (400.0, 300.0));
    assert_eq!(a.scale, 2.0);
    // The control-character text is dropped; the plain one is kept.
    let texts: Vec<_> = a
        .events
        .iter()
        .filter_map(|e| match e {
            lune_ui_core::input::Event::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["a"]);
}

#[test]
fn state_persists_and_events_drain() {
    let mut t = Translator::new(PhysicalSize::new(800, 600), 2.0);
    t.cursor_moved(20.0, 20.0);
    t.focused(true);
    let first = t.take(FRAME);
    assert_eq!(first.events.len(), 2);
    t.mouse_input(Pressed, MouseButton::Right);
    let second = t.take(FRAME * 2);
    assert!(second.focused);
    assert_eq!(second.time, FRAME * 2);
    assert_eq!(
        second.events,
        [lune_ui_core::input::Event::PointerButton {
            pos: (10.0, 10.0),
            button: PointerButton::Secondary,
            pressed: true,
            modifiers: Modifiers::default(),
        }]
    );
}

#[test]
fn unmapped_keys_are_unidentified() {
    let mut t = Translator::new(PhysicalSize::new(100, 100), 1.0);
    t.key(&WKey::Named(WNamed::Shift), Pressed, false, None);
    t.key(&WKey::Dead(Some('`')), Pressed, false, None);
    t.key(
        &WKey::Unidentified(winit::keyboard::NativeKey::Unidentified),
        Released,
        false,
        None,
    );
    let keys: Vec<_> = t
        .take(Duration::ZERO)
        .events
        .into_iter()
        .filter_map(|e| match e {
            lune_ui_core::input::Event::Key { key, .. } => Some(key),
            _ => None,
        })
        .collect();
    assert_eq!(
        keys,
        [Key::Unidentified, Key::Unidentified, Key::Unidentified]
    );
}

#[test]
fn other_mouse_button_keeps_its_number() {
    let mut t = Translator::new(PhysicalSize::new(100, 100), 1.0);
    t.mouse_input(Pressed, MouseButton::Other(7));
    let e = t.take(Duration::ZERO).events;
    assert!(matches!(
        e[..],
        [lune_ui_core::input::Event::PointerButton {
            button: PointerButton::Other(7),
            ..
        }]
    ));
}

#[test]
fn resize_edges_map_to_the_same_named_direction() {
    let all = [
        (ResizeEdge::North, D::North),
        (ResizeEdge::South, D::South),
        (ResizeEdge::East, D::East),
        (ResizeEdge::West, D::West),
        (ResizeEdge::NorthEast, D::NorthEast),
        (ResizeEdge::NorthWest, D::NorthWest),
        (ResizeEdge::SouthEast, D::SouthEast),
        (ResizeEdge::SouthWest, D::SouthWest),
    ];
    for (edge, dir) in all {
        assert_eq!(resize_direction(edge), dir, "{edge:?}");
    }
}

#[test]
fn every_cursor_icon_maps_to_the_same_named_winit_icon() {
    for icon in C::ALL {
        if icon != C::Default {
            assert_ne!(cursor_icon(icon), WC::Default, "{icon:?}");
        }
    }
    let all = [
        (C::Default, WC::Default),
        (C::ContextMenu, WC::ContextMenu),
        (C::Help, WC::Help),
        (C::Pointer, WC::Pointer),
        (C::Progress, WC::Progress),
        (C::Wait, WC::Wait),
        (C::Cell, WC::Cell),
        (C::Crosshair, WC::Crosshair),
        (C::Text, WC::Text),
        (C::VerticalText, WC::VerticalText),
        (C::Alias, WC::Alias),
        (C::Copy, WC::Copy),
        (C::Move, WC::Move),
        (C::NoDrop, WC::NoDrop),
        (C::NotAllowed, WC::NotAllowed),
        (C::Grab, WC::Grab),
        (C::Grabbing, WC::Grabbing),
        (C::EResize, WC::EResize),
        (C::NResize, WC::NResize),
        (C::NeResize, WC::NeResize),
        (C::NwResize, WC::NwResize),
        (C::SResize, WC::SResize),
        (C::SeResize, WC::SeResize),
        (C::SwResize, WC::SwResize),
        (C::WResize, WC::WResize),
        (C::EwResize, WC::EwResize),
        (C::NsResize, WC::NsResize),
        (C::NeswResize, WC::NeswResize),
        (C::NwseResize, WC::NwseResize),
        (C::ColResize, WC::ColResize),
        (C::RowResize, WC::RowResize),
        (C::AllScroll, WC::AllScroll),
        (C::ZoomIn, WC::ZoomIn),
        (C::ZoomOut, WC::ZoomOut),
        (C::DndAsk, WC::DndAsk),
        (C::AllResize, WC::AllResize),
    ];
    assert_eq!(all.len(), C::ALL.len());
    for (core, winit) in all {
        assert_eq!(cursor_icon(core), winit, "{core:?}");
    }
}

#[test]
fn control_flow_maps_each_repaint_without_reading_the_clock() {
    use lune_ui_core::input::Repaint;
    use lune_ui_winit::input::control_flow;
    use winit::event_loop::ControlFlow;

    // The origin is a test-owned instant; the function only adds to it.
    let origin = std::time::Instant::now();
    assert_eq!(
        control_flow(Repaint::Idle, origin),
        (ControlFlow::Wait, false)
    );
    assert_eq!(
        control_flow(Repaint::Now, origin),
        (ControlFlow::Wait, true)
    );
    let at = Duration::from_millis(250);
    assert_eq!(
        control_flow(Repaint::At(at), origin),
        (ControlFlow::WaitUntil(origin + at), false)
    );
    assert_eq!(
        control_flow(Repaint::At(Duration::ZERO), origin),
        (ControlFlow::WaitUntil(origin), false)
    );
    // A deadline past what `Instant` can hold waits for input instead of panicking.
    assert_eq!(
        control_flow(Repaint::At(Duration::MAX), origin),
        (ControlFlow::Wait, false)
    );
}

// --- File drop ------------------------------------------------------------------------------

mod files {
    use super::*;
    use lune_ui_core::access::{AccessAction, Action};
    use lune_ui_core::input::{CursorIcon, Event, FrameOutput, RawInput, WindowCommand};
    use lune_ui_core::paint::Rect;
    use lune_ui_core::ui::WidgetId;
    use lune_ui_winit::input::{ImePlan, OutputState, ime_area_physical};
    use std::path::PathBuf;

    fn at_scale_2() -> Translator {
        let mut t = Translator::new(PhysicalSize::new(800, 600), 2.0);
        t.window_event(&WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(200.0, 100.0),
        });
        t
    }

    fn events(mut t: Translator) -> Vec<Event> {
        t.take(FRAME).events
    }

    #[test]
    fn hover_cancel_and_drop_give_the_three_events_at_the_last_pointer_position() {
        let mut t = at_scale_2();
        let _ = t.take(FRAME); // the pointer move
        t.window_event(&WindowEvent::HoveredFile(PathBuf::from("/tmp/a.txt")));
        t.window_event(&WindowEvent::HoveredFileCancelled);
        t.window_event(&WindowEvent::DroppedFile(PathBuf::from("/tmp/b.txt")));
        assert_eq!(
            events(t),
            [
                Event::FileHover {
                    pos: (100.0, 50.0),
                    path: PathBuf::from("/tmp/a.txt")
                },
                Event::FileHoverCancelled,
                Event::FileDrop {
                    pos: (100.0, 50.0),
                    path: PathBuf::from("/tmp/b.txt")
                },
            ],
            "physical (200, 100) at scale 2 is logical (100, 50)"
        );
    }

    #[test]
    fn before_any_pointer_move_the_position_is_the_origin() {
        let mut t = Translator::new(PhysicalSize::new(800, 600), 2.0);
        t.window_event(&WindowEvent::HoveredFile(PathBuf::from("a")));
        t.window_event(&WindowEvent::DroppedFile(PathBuf::from("a")));
        assert_eq!(
            events(t),
            [
                Event::FileHover {
                    pos: (0.0, 0.0),
                    path: PathBuf::from("a")
                },
                Event::FileDrop {
                    pos: (0.0, 0.0),
                    path: PathBuf::from("a")
                },
            ]
        );
    }

    #[test]
    fn two_files_give_two_events_in_order() {
        let mut t = at_scale_2();
        let _ = t.take(FRAME);
        for name in ["one", "two"] {
            t.window_event(&WindowEvent::DroppedFile(PathBuf::from(name)));
        }
        let paths: Vec<PathBuf> = events(t)
            .into_iter()
            .filter_map(|e| match e {
                Event::FileDrop { path, .. } => Some(path),
                _ => None,
            })
            .collect();
        assert_eq!(paths, [PathBuf::from("one"), PathBuf::from("two")]);
    }

    #[test]
    fn the_translator_and_the_script_agree_on_file_events() {
        let mut s = ScriptedInput::new((400.0, 300.0), 2.0);
        s.set_time(FRAME)
            .pointer_to(100.0, 50.0)
            .file_hover("/tmp/a.txt")
            .file_hover("/tmp/b.txt")
            .file_hover_cancelled()
            .file_drop("/tmp/a.txt")
            .file_drop("/tmp/b.txt");
        let scripted = s.end_frame();

        let mut t = at_scale_2();
        t.window_event(&WindowEvent::HoveredFile(PathBuf::from("/tmp/a.txt")));
        t.window_event(&WindowEvent::HoveredFile(PathBuf::from("/tmp/b.txt")));
        t.window_event(&WindowEvent::HoveredFileCancelled);
        t.window_event(&WindowEvent::DroppedFile(PathBuf::from("/tmp/a.txt")));
        t.window_event(&WindowEvent::DroppedFile(PathBuf::from("/tmp/b.txt")));
        let translated: RawInput = t.take(FRAME);
        // The script's events start at the file events; the translator also has the pointer move.
        assert_eq!(
            translated.events[0],
            Event::PointerMoved { pos: (100.0, 50.0) }
        );
        assert_eq!(&translated.events[1..], &scripted.events[1..]);
        assert_eq!(translated.events[1..], scripted.events[1..]);
    }

    #[test]
    fn an_access_action_queued_on_the_translator_appears_in_the_next_take() {
        let action = AccessAction {
            target: WidgetId::ROOT.with("button"),
            action: Action::Default,
            value: None,
        };
        let mut t = Translator::new(PhysicalSize::new(100, 100), 1.0);
        t.access(action.clone());
        assert_eq!(t.take(FRAME).events, [Event::Access(action)]);
        assert!(t.take(FRAME).events.is_empty(), "drained");
        // And the script builds the same event.
        let mut s = ScriptedInput::new((100.0, 100.0), 1.0);
        s.access(AccessAction {
            target: WidgetId::ROOT.with("button"),
            action: Action::Default,
            value: None,
        });
        assert_eq!(s.end_frame().events.len(), 1);
    }

    // --- Output plan ------------------------------------------------------------------------

    fn out(cursor: CursorIcon, ime: Option<Rect>) -> FrameOutput {
        FrameOutput {
            cursor,
            ime_area: ime,
            commands: vec![WindowCommand::Redraw],
            ..FrameOutput::default()
        }
    }

    #[test]
    fn plan_output_is_public_pure_and_starts_from_nothing_set() {
        let state = OutputState::new();
        let area = Rect::new(1.0, 2.0, 3.0, 4.0);
        let first = state.plan_output(&out(CursorIcon::Default, None));
        assert_eq!(first.cursor, Some(CursorIcon::Default));
        assert_eq!(first.ime, ImePlan::Keep);
        let p = state.plan_output(&out(CursorIcon::Text, Some(area)));
        assert_eq!(p.cursor, Some(CursorIcon::Text));
        assert_eq!(p.ime, ImePlan::Area { enable: true, area });
        // Planning changed nothing: the same plan again. The sequences of changes are the unit
        // tests in `src/input.rs` (they can record a plan without a window).
        assert_eq!(state.plan_output(&out(CursorIcon::Text, Some(area))), p);
    }

    #[test]
    fn the_ime_area_leaves_in_physical_pixels_at_the_uis_scale() {
        let area = Rect::new(10.0, 20.0, 30.0, 40.0);
        // Under a zoom the UI's scale is not the window's: area x scale, whatever the OS says.
        assert_eq!(
            ime_area_physical(area, 4.0),
            Some((40.0, 80.0, 120.0, 160.0))
        );
        // At the OS scale it is what the logical position of the same area was: area x 2.
        assert_eq!(ime_area_physical(area, 2.0), Some((20.0, 40.0, 60.0, 80.0)));
        assert_eq!(ime_area_physical(area, 1.0), Some((10.0, 20.0, 30.0, 40.0)));
    }

    #[test]
    fn a_non_finite_ime_area_or_scale_sends_nothing() {
        let ok = Rect::new(10.0, 20.0, 30.0, 40.0);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for area in [
                Rect::new(bad, 20.0, 30.0, 40.0),
                Rect::new(10.0, bad, 30.0, 40.0),
                Rect::new(10.0, 20.0, bad, 40.0),
                Rect::new(10.0, 20.0, 30.0, bad),
            ] {
                assert_eq!(ime_area_physical(area, 2.0), None, "{area:?}");
            }
            assert_eq!(ime_area_physical(ok, bad), None, "scale {bad}");
        }
        assert_eq!(ime_area_physical(ok, 0.0), None);
        assert_eq!(ime_area_physical(ok, -1.0), None);
    }
}

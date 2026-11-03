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

//! The input seam on winit: [`Translator`] turns `WindowEvent`s into [`RawInput`], and
//! [`OutputState`] applies a [`FrameOutput`] to the window. One of each per window.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::keyboard::{Key as WKey, ModifiersState, NamedKey as WNamed};
use winit::window::{CursorIcon as WCursor, Window};

use lune_ui_core::access::AccessAction;
use lune_ui_core::input::{
    CursorIcon, Event, FrameOutput, ImeEvent, Key, Keymap, Modifiers, NamedKey, PointerButton,
    RawInput, Repaint, ScrollDelta, WindowCommand, printable,
};
use lune_ui_core::paint::Rect;

use crate::platform;
use crate::window::resize_direction;

/// Collects one window's winit events into [`RawInput`] frames.
///
/// Positions and sizes are divided by the latest scale factor. Events that arrive before the
/// first pointer move carry the position (0, 0). The window starts unfocused until winit says
/// otherwise. A hovered, cancelled or dropped file becomes a file event (one per file) at the last
/// pointer position, in logical pixels: winit gives none. Touch, gestures, theme, moved, occluded,
/// axis motion and touchpad pressure events are ignored.
#[derive(Debug)]
pub struct Translator {
    size: PhysicalSize<u32>,
    scale: f64,
    pointer: (f32, f32),
    modifiers: Modifiers,
    focused: bool,
    events: Vec<Event>,
}

impl Translator {
    /// A translator for a window of `size` physical pixels at `scale_factor`.
    pub fn new(size: PhysicalSize<u32>, scale_factor: f64) -> Self {
        Self {
            size,
            scale: scale_factor,
            pointer: (0.0, 0.0),
            modifiers: Modifiers::default(),
            focused: false,
            events: Vec::new(),
        }
    }

    fn logical(&self, x: f64, y: f64) -> (f32, f32) {
        ((x / self.scale) as f32, (y / self.scale) as f32)
    }

    /// Feeds one window event.
    pub fn window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Resized(size) => self.resized(*size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor(*scale_factor);
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor_moved(position.x, position.y),
            WindowEvent::CursorLeft { .. } => self.cursor_left(),
            WindowEvent::MouseInput { state, button, .. } => self.mouse_input(*state, *button),
            WindowEvent::MouseWheel { delta, .. } => self.mouse_wheel(*delta),
            WindowEvent::KeyboardInput { event, .. } => self.key(
                &event.logical_key,
                event.state,
                event.repeat,
                event.text.as_deref(),
            ),
            WindowEvent::ModifiersChanged(m) => self.modifiers(m.state()),
            WindowEvent::Ime(ime) => self.ime(ime),
            WindowEvent::Focused(f) => self.focused(*f),
            WindowEvent::HoveredFile(path) => self.file_hovered(path.clone()),
            WindowEvent::HoveredFileCancelled => self.file_hover_cancelled(),
            WindowEvent::DroppedFile(path) => self.file_dropped(path.clone()),
            _ => {}
        }
    }

    /// The window was resized to `size` physical pixels.
    pub fn resized(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
    }

    /// The scale factor changed.
    pub fn scale_factor(&mut self, scale_factor: f64) {
        self.scale = scale_factor;
    }

    /// The pointer moved to a physical position.
    pub fn cursor_moved(&mut self, x: f64, y: f64) {
        self.pointer = self.logical(x, y);
        self.events.push(Event::PointerMoved { pos: self.pointer });
    }

    /// The pointer left the window.
    pub fn cursor_left(&mut self) {
        self.events.push(Event::PointerLeft);
    }

    /// A mouse button changed state.
    pub fn mouse_input(&mut self, state: ElementState, button: MouseButton) {
        self.events.push(Event::PointerButton {
            pos: self.pointer,
            button: pointer_button(button),
            pressed: state == ElementState::Pressed,
            modifiers: self.modifiers,
        });
    }

    /// The wheel or touchpad scrolled.
    pub fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let delta = match delta {
            MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines { x, y },
            MouseScrollDelta::PixelDelta(p) => {
                let (x, y) = self.logical(p.x, p.y);
                ScrollDelta::Pixels { x, y }
            }
        };
        self.events.push(Event::Scroll {
            pos: self.pointer,
            delta,
            modifiers: self.modifiers,
        });
    }

    /// A key changed state; `text` is what the press types, if anything. Taken apart from
    /// `KeyEvent` because that type cannot be built outside winit.
    pub fn key(&mut self, key: &WKey, state: ElementState, repeat: bool, text: Option<&str>) {
        let pressed = state == ElementState::Pressed;
        self.events.push(Event::Key {
            key: map_key(key),
            pressed,
            repeat,
            modifiers: self.modifiers,
        });
        if pressed && let Some(t) = text.and_then(printable) {
            self.events.push(Event::Text(t));
        }
    }

    /// The modifier keys changed.
    pub fn modifiers(&mut self, state: ModifiersState) {
        self.modifiers = Modifiers {
            shift: state.shift_key(),
            control: state.control_key(),
            alt: state.alt_key(),
            logo: state.super_key(),
        };
        self.events.push(Event::ModifiersChanged(self.modifiers));
    }

    /// An input-method event; offsets stay in bytes.
    pub fn ime(&mut self, ime: &Ime) {
        let e = match ime {
            Ime::Enabled => ImeEvent::Enabled,
            Ime::Preedit(text, cursor) => ImeEvent::Preedit {
                text: text.clone(),
                cursor: *cursor,
            },
            Ime::Commit(text) => ImeEvent::Commit(text.clone()),
            Ime::Disabled => ImeEvent::Disabled,
        };
        self.events.push(Event::Ime(e));
    }

    /// A file is dragged over the window. The path passes unchanged (it may not be UTF-8).
    pub fn file_hovered(&mut self, path: PathBuf) {
        self.events.push(Event::FileHover {
            pos: self.pointer,
            path,
        });
    }

    /// The dragged file left the window.
    pub fn file_hover_cancelled(&mut self) {
        self.events.push(Event::FileHoverCancelled);
    }

    /// A file was dropped on the window. The path passes unchanged.
    pub fn file_dropped(&mut self, path: PathBuf) {
        self.events.push(Event::FileDrop {
            pos: self.pointer,
            path,
        });
    }

    /// An assistive technology asked for an action on a node.
    pub fn access(&mut self, action: AccessAction) {
        self.events.push(Event::Access(action));
    }

    /// The clipboard's text, answering a `ClipboardRequest::Read`.
    pub fn clipboard_text(&mut self, text: &str) {
        self.events.push(Event::ClipboardText(text.to_owned()));
    }

    /// The clipboard holds no text or cannot be read, answering a `ClipboardRequest::Read`.
    pub fn clipboard_unavailable(&mut self) {
        self.events.push(Event::ClipboardUnavailable);
    }

    /// The window gained or lost focus.
    pub fn focused(&mut self, focused: bool) {
        self.focused = focused;
        self.events.push(Event::Focus(focused));
    }

    /// The frame's input at app time `time`; the event list starts empty again, the window state
    /// (size, scale, pointer, modifiers, focus) stays.
    pub fn take(&mut self, time: Duration) -> RawInput {
        let (w, h) = self.logical(f64::from(self.size.width), f64::from(self.size.height));
        RawInput {
            size: (w, h),
            scale: self.scale as f32,
            time,
            focused: self.focused,
            events: std::mem::take(&mut self.events),
        }
    }
}

/// The keyboard conventions of the platform this adapter was built for: `Keymap::MAC` on macOS,
/// `Keymap::PC` elsewhere. The app passes it in `FrameConfig` (the core never reads the OS, D7).
pub fn keymap() -> Keymap {
    platform::keymap()
}

fn pointer_button(b: MouseButton) -> PointerButton {
    match b {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Back => PointerButton::Back,
        MouseButton::Forward => PointerButton::Forward,
        MouseButton::Other(n) => PointerButton::Other(n),
    }
}

fn map_key(key: &WKey) -> Key {
    let WKey::Named(n) = key else {
        return match key {
            WKey::Character(s) => Key::Character(s.to_string()),
            _ => Key::Unidentified,
        };
    };
    let named = match n {
        WNamed::Enter => NamedKey::Enter,
        WNamed::Tab => NamedKey::Tab,
        WNamed::Space => NamedKey::Space,
        WNamed::Backspace => NamedKey::Backspace,
        WNamed::Delete => NamedKey::Delete,
        WNamed::Insert => NamedKey::Insert,
        WNamed::Escape => NamedKey::Escape,
        WNamed::Home => NamedKey::Home,
        WNamed::End => NamedKey::End,
        WNamed::PageUp => NamedKey::PageUp,
        WNamed::PageDown => NamedKey::PageDown,
        WNamed::ArrowLeft => NamedKey::ArrowLeft,
        WNamed::ArrowRight => NamedKey::ArrowRight,
        WNamed::ArrowUp => NamedKey::ArrowUp,
        WNamed::ArrowDown => NamedKey::ArrowDown,
        WNamed::F1 => NamedKey::F1,
        WNamed::F2 => NamedKey::F2,
        WNamed::F3 => NamedKey::F3,
        WNamed::F4 => NamedKey::F4,
        WNamed::F5 => NamedKey::F5,
        WNamed::F6 => NamedKey::F6,
        WNamed::F7 => NamedKey::F7,
        WNamed::F8 => NamedKey::F8,
        WNamed::F9 => NamedKey::F9,
        WNamed::F10 => NamedKey::F10,
        WNamed::F11 => NamedKey::F11,
        WNamed::F12 => NamedKey::F12,
        _ => return Key::Unidentified,
    };
    Key::Named(named)
}

/// The winit cursor icon of `icon`.
pub fn cursor_icon(icon: CursorIcon) -> WCursor {
    match icon {
        CursorIcon::Default => WCursor::Default,
        CursorIcon::ContextMenu => WCursor::ContextMenu,
        CursorIcon::Help => WCursor::Help,
        CursorIcon::Pointer => WCursor::Pointer,
        CursorIcon::Progress => WCursor::Progress,
        CursorIcon::Wait => WCursor::Wait,
        CursorIcon::Cell => WCursor::Cell,
        CursorIcon::Crosshair => WCursor::Crosshair,
        CursorIcon::Text => WCursor::Text,
        CursorIcon::VerticalText => WCursor::VerticalText,
        CursorIcon::Alias => WCursor::Alias,
        CursorIcon::Copy => WCursor::Copy,
        CursorIcon::Move => WCursor::Move,
        CursorIcon::NoDrop => WCursor::NoDrop,
        CursorIcon::NotAllowed => WCursor::NotAllowed,
        CursorIcon::Grab => WCursor::Grab,
        CursorIcon::Grabbing => WCursor::Grabbing,
        CursorIcon::EResize => WCursor::EResize,
        CursorIcon::NResize => WCursor::NResize,
        CursorIcon::NeResize => WCursor::NeResize,
        CursorIcon::NwResize => WCursor::NwResize,
        CursorIcon::SResize => WCursor::SResize,
        CursorIcon::SeResize => WCursor::SeResize,
        CursorIcon::SwResize => WCursor::SwResize,
        CursorIcon::WResize => WCursor::WResize,
        CursorIcon::EwResize => WCursor::EwResize,
        CursorIcon::NsResize => WCursor::NsResize,
        CursorIcon::NeswResize => WCursor::NeswResize,
        CursorIcon::NwseResize => WCursor::NwseResize,
        CursorIcon::ColResize => WCursor::ColResize,
        CursorIcon::RowResize => WCursor::RowResize,
        CursorIcon::AllScroll => WCursor::AllScroll,
        CursorIcon::ZoomIn => WCursor::ZoomIn,
        CursorIcon::ZoomOut => WCursor::ZoomOut,
        CursorIcon::DndAsk => WCursor::DndAsk,
        CursorIcon::AllResize => WCursor::AllResize,
        // `CursorIcon` is `#[non_exhaustive]`: an icon added in core shows the default cursor
        // until this adapter maps it.
        _ => WCursor::Default,
    }
}

/// The event loop's control flow for a frame's [`Repaint`], and whether to request a redraw now:
/// `Idle` waits for input, `Now` waits and asks for a redraw, `At(t)` waits until `origin + t`,
/// where `origin` is the instant the app's time `0` was taken (a `t` too far to represent waits
/// for input). Reads no clock.
pub fn control_flow(repaint: Repaint, origin: Instant) -> (ControlFlow, bool) {
    match repaint {
        Repaint::Idle => (ControlFlow::Wait, false),
        Repaint::Now => (ControlFlow::Wait, true),
        Repaint::At(t) => (
            origin
                .checked_add(t)
                .map_or(ControlFlow::Wait, ControlFlow::WaitUntil),
            false,
        ),
    }
}

/// The IME area `area`, which the frame measured in the units of the UI's own scale, as the window
/// takes it: `(x, y, w, h)` in physical pixels, `area * ui_scale`. `None` when the area or the
/// scale is not finite, or the scale is not above 0: nothing is sent for it.
///
/// The UI's scale is the display's scale times the page's zoom, so under a zoom the area is not
/// in the window's own logical units; physical pixels are the one unit both agree on.
pub fn ime_area_physical(area: Rect, ui_scale: f32) -> Option<(f32, f32, f32, f32)> {
    let [x, y, w, h] = [area.x, area.y, area.w, area.h].map(|v| v * ui_scale);
    let finite = [x, y, w, h].iter().all(|v| v.is_finite());
    (finite && ui_scale > 0.0).then_some((x, y, w, h))
}

/// What [`OutputState::apply`] does with the IME for a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImePlan {
    /// The area is as the last frame left it: touch nothing.
    Keep,
    /// An area is wanted; `enable` is true when the IME was off and must be allowed first.
    Area { enable: bool, area: Rect },
    /// The IME was on and is not wanted any more.
    Disable,
}

/// What [`OutputState::apply`] changes on the window for a frame, apart from its commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutputPlan {
    /// The cursor to set, when it differs from the one set last (always on the first frame).
    pub cursor: Option<CursorIcon>,
    pub ime: ImePlan,
}

/// Applies a window's [`FrameOutput`], touching the cursor and the IME only when they changed.
#[derive(Debug, Default)]
pub struct OutputState {
    cursor: Option<CursorIcon>,
    ime_area: Option<Rect>,
}

impl OutputState {
    /// A state that has set no cursor and no IME area yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// What `out` changes against the state the last [`Self::apply`] left. Pure: it reads no
    /// window and changes nothing, so it is tested without one.
    pub fn plan_output(&self, out: &FrameOutput) -> OutputPlan {
        let cursor = (self.cursor != Some(out.cursor)).then_some(out.cursor);
        let ime = if self.ime_area == out.ime_area {
            ImePlan::Keep
        } else {
            match out.ime_area {
                Some(area) => ImePlan::Area {
                    enable: self.ime_area.is_none(),
                    area,
                },
                None => ImePlan::Disable,
            }
        };
        OutputPlan { cursor, ime }
    }

    /// Records what the plan for `out` set, so the next plan compares against it.
    fn commit(&mut self, plan: &OutputPlan, out: &FrameOutput) {
        if let Some(icon) = plan.cursor {
            self.cursor = Some(icon);
        }
        self.ime_area = out.ime_area;
    }

    /// Applies `out` to `window`. `ui_scale` is the scale the frame was laid out at (the display's
    /// scale times the page's zoom): the IME area leaves in physical pixels, [`ime_area_physical`].
    /// Returns true when the frame asked to close the window (the app closes it).
    pub fn apply(&mut self, window: &Window, out: &FrameOutput, ui_scale: f32) -> bool {
        let plan = self.plan_output(out);
        if let Some(icon) = plan.cursor {
            window.set_cursor(cursor_icon(icon));
        }
        match plan.ime {
            ImePlan::Keep => {}
            ImePlan::Area { enable, area: r } => {
                if enable {
                    window.set_ime_allowed(true);
                }
                if let Some((x, y, w, h)) = ime_area_physical(r, ui_scale) {
                    window
                        .set_ime_cursor_area(PhysicalPosition::new(x, y), PhysicalSize::new(w, h));
                }
            }
            ImePlan::Disable => window.set_ime_allowed(false),
        }
        self.commit(&plan, out);
        let mut close = false;
        for c in &out.commands {
            match c {
                WindowCommand::DragMove => {
                    let _ = window.drag_window();
                }
                WindowCommand::DragResize(e) => {
                    let _ = window.drag_resize_window(resize_direction(*e));
                }
                WindowCommand::Minimize => window.set_minimized(true),
                WindowCommand::ToggleMaximize => window.set_maximized(!window.is_maximized()),
                WindowCommand::Close => close = true,
                WindowCommand::Redraw => window.request_redraw(),
                // `WindowCommand` is `#[non_exhaustive]`: a variant added later is a no-op here
                // until this adapter learns it.
                #[allow(unreachable_patterns)]
                _ => {}
            }
        }
        close
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(cursor: CursorIcon, ime: Option<Rect>) -> FrameOutput {
        FrameOutput {
            cursor,
            ime_area: ime,
            ..FrameOutput::default()
        }
    }

    /// Plans `o`, then records it as `apply` does.
    fn step(state: &mut OutputState, o: &FrameOutput) -> OutputPlan {
        let plan = state.plan_output(o);
        state.commit(&plan, o);
        plan
    }

    #[test]
    fn the_cursor_is_set_on_the_first_frame_and_on_a_change_only() {
        let mut s = OutputState::new();
        let first = step(&mut s, &out(CursorIcon::Default, None));
        assert_eq!(first.cursor, Some(CursorIcon::Default));
        assert_eq!(step(&mut s, &out(CursorIcon::Default, None)).cursor, None);
        assert_eq!(
            step(&mut s, &out(CursorIcon::Text, None)).cursor,
            Some(CursorIcon::Text)
        );
        assert_eq!(step(&mut s, &out(CursorIcon::Text, None)).cursor, None);
        assert_eq!(
            step(&mut s, &out(CursorIcon::Default, None)).cursor,
            Some(CursorIcon::Default)
        );
    }

    #[test]
    fn the_ime_is_touched_when_its_area_appears_moves_or_goes() {
        let mut s = OutputState::new();
        let (a, b) = (Rect::new(1.0, 2.0, 3.0, 4.0), Rect::new(5.0, 6.0, 3.0, 4.0));
        let d = CursorIcon::Default;
        assert_eq!(step(&mut s, &out(d, None)).ime, ImePlan::Keep);
        assert_eq!(
            step(&mut s, &out(d, Some(a))).ime,
            ImePlan::Area {
                enable: true,
                area: a
            }
        );
        assert_eq!(step(&mut s, &out(d, Some(a))).ime, ImePlan::Keep);
        assert_eq!(
            step(&mut s, &out(d, Some(b))).ime,
            ImePlan::Area {
                enable: false,
                area: b
            },
            "a move does not enable it again"
        );
        assert_eq!(step(&mut s, &out(d, None)).ime, ImePlan::Disable);
        assert_eq!(step(&mut s, &out(d, None)).ime, ImePlan::Keep);
    }

    #[test]
    fn planning_changes_nothing() {
        let s = OutputState::new();
        let o = out(CursorIcon::Text, Some(Rect::new(0.0, 0.0, 1.0, 1.0)));
        assert_eq!(s.plan_output(&o), s.plan_output(&o));
    }
}

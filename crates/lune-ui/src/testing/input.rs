//! The input seam's fake: a scripted window.

use std::time::Duration;

use crate::input::{
    Event, ImeEvent, Key, Modifiers, PointerButton, RawInput, ScrollDelta, printable,
};

/// Builds a window's [`RawInput`] frames by hand. Size, scale, time, focus, pointer position and
/// modifiers persist across frames; [`ScriptedInput::end_frame`] takes the events and keeps the
/// state. It follows the same rules as the winit translator: positions are logical, the pointer
/// starts at (0, 0), the window starts unfocused, button and scroll events carry the pointer
/// position and modifiers current at that point, and a key press with printable text is followed
/// by a `Text` event.
#[derive(Debug, Clone)]
pub struct ScriptedInput {
    size: (f32, f32),
    scale: f32,
    time: Duration,
    focused: bool,
    pointer: (f32, f32),
    modifiers: Modifiers,
    events: Vec<Event>,
}

impl ScriptedInput {
    /// A window of logical `size` at `scale`.
    pub fn new(size: (f32, f32), scale: f32) -> Self {
        Self {
            size,
            scale,
            time: Duration::ZERO,
            focused: false,
            pointer: (0.0, 0.0),
            modifiers: Modifiers::default(),
            events: Vec::new(),
        }
    }

    /// Resizes the window (logical pixels). Not an event.
    pub fn resize(&mut self, size: (f32, f32)) -> &mut Self {
        self.size = size;
        self
    }

    /// Sets the scale. Not an event.
    pub fn set_scale(&mut self, scale: f32) -> &mut Self {
        self.scale = scale;
        self
    }

    /// Sets the time the following frames report.
    pub fn set_time(&mut self, time: Duration) -> &mut Self {
        self.time = time;
        self
    }

    /// Moves the pointer to a logical position.
    pub fn pointer_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.pointer = (x, y);
        self.events.push(Event::PointerMoved { pos: self.pointer });
        self
    }

    /// The pointer leaves the window.
    pub fn pointer_leave(&mut self) -> &mut Self {
        self.events.push(Event::PointerLeft);
        self
    }

    /// Presses `button` at the current pointer position.
    pub fn press(&mut self, button: PointerButton) -> &mut Self {
        self.button(button, true)
    }

    /// Releases `button` at the current pointer position.
    pub fn release(&mut self, button: PointerButton) -> &mut Self {
        self.button(button, false)
    }

    /// Press then release.
    pub fn click(&mut self, button: PointerButton) -> &mut Self {
        self.press(button).release(button)
    }

    fn button(&mut self, button: PointerButton, pressed: bool) -> &mut Self {
        self.events.push(Event::PointerButton {
            pos: self.pointer,
            button,
            pressed,
            modifiers: self.modifiers,
        });
        self
    }

    /// Scrolls by lines.
    pub fn scroll_lines(&mut self, x: f32, y: f32) -> &mut Self {
        self.scroll(ScrollDelta::Lines { x, y })
    }

    /// Scrolls by logical pixels.
    pub fn scroll_pixels(&mut self, x: f32, y: f32) -> &mut Self {
        self.scroll(ScrollDelta::Pixels { x, y })
    }

    fn scroll(&mut self, delta: ScrollDelta) -> &mut Self {
        self.events.push(Event::Scroll {
            pos: self.pointer,
            delta,
            modifiers: self.modifiers,
        });
        self
    }

    /// Presses `key`; `text` is what the press types, if anything.
    pub fn key_down(&mut self, key: Key, text: Option<&str>) -> &mut Self {
        self.events.push(Event::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers: self.modifiers,
        });
        if let Some(t) = text.and_then(printable) {
            self.events.push(Event::Text(t));
        }
        self
    }

    /// Releases `key`.
    pub fn key_up(&mut self, key: Key) -> &mut Self {
        self.events.push(Event::Key {
            key,
            pressed: false,
            repeat: false,
            modifiers: self.modifiers,
        });
        self
    }

    /// Sets the held modifiers.
    pub fn set_modifiers(&mut self, modifiers: Modifiers) -> &mut Self {
        self.modifiers = modifiers;
        self.events.push(Event::ModifiersChanged(modifiers));
        self
    }

    /// Pushes an input-method event.
    pub fn ime(&mut self, event: ImeEvent) -> &mut Self {
        self.events.push(Event::Ime(event));
        self
    }

    /// The window gains or loses focus.
    pub fn focus(&mut self, focused: bool) -> &mut Self {
        self.focused = focused;
        self.events.push(Event::Focus(focused));
        self
    }

    /// The frame's input; the event list starts empty again, the state stays.
    pub fn end_frame(&mut self) -> RawInput {
        RawInput {
            size: self.size,
            scale: self.scale,
            time: self.time,
            focused: self.focused,
            events: std::mem::take(&mut self.events),
        }
    }
}

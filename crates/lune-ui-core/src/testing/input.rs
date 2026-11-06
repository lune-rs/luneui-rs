//! The input seam's fake: a scripted window.

use std::path::PathBuf;
use std::time::Duration;

use crate::access::AccessAction;
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

    /// Presses `key` as an auto-repeat (no text).
    pub fn key_repeat(&mut self, key: Key) -> &mut Self {
        self.events.push(Event::Key {
            key,
            pressed: true,
            repeat: true,
            modifiers: self.modifiers,
        });
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

    /// Pushes an assistive technology's action request.
    pub fn access(&mut self, action: AccessAction) -> &mut Self {
        self.events.push(Event::Access(action));
        self
    }

    /// Pushes the clipboard's text, answering a read.
    pub fn clipboard_text(&mut self, text: &str) -> &mut Self {
        self.events.push(Event::ClipboardText(text.to_owned()));
        self
    }

    /// Pushes the clipboard-unavailable answer to a read.
    pub fn clipboard_unavailable(&mut self) -> &mut Self {
        self.events.push(Event::ClipboardUnavailable);
        self
    }

    /// A file is dragged over the window; the event carries the current pointer position.
    pub fn file_hover(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.events.push(Event::FileHover {
            pos: self.pointer,
            path: path.into(),
        });
        self
    }

    /// The dragged file left the window.
    pub fn file_hover_cancelled(&mut self) -> &mut Self {
        self.events.push(Event::FileHoverCancelled);
        self
    }

    /// A file is dropped on the window; the event carries the current pointer position.
    pub fn file_drop(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.events.push(Event::FileDrop {
            pos: self.pointer,
            path: path.into(),
        });
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

/// Checks the properties of the input seam's fake; the error names the first that fails.
/// `take` ends a frame of the script it is given: [`ScriptedInput::end_frame`] is the real answer,
/// and a fake that only wraps or forwards it must pass the same suite.
///
/// - I1: size, scale, time and focus appear in the frame as set, and time passes as given (it
///   does not advance by itself, and a later frame keeps it until it is set again);
/// - I2: events come out in the order they were added, within one frame and from frame to frame;
/// - I3: each event ends up in exactly one frame (the next frame starts empty), while size, scale,
///   time, focus, pointer position and modifiers persist;
/// - I4: a button or scroll event carries the pointer position and modifiers current at that
///   point of the script, not the ones at the end of the frame;
/// - I5: a key press with printable text is followed by one `Text` event; control characters are
///   dropped and a press with nothing left emits none.
pub fn scripted_input_contract(
    take: &mut dyn FnMut(&mut ScriptedInput) -> RawInput,
) -> Result<(), String> {
    let mut s = ScriptedInput::new((320.0, 200.0), 2.0);

    // I1
    s.set_time(Duration::from_millis(5)).focus(true);
    let f = take(&mut s);
    if f.size != (320.0, 200.0) || f.scale != 2.0 {
        return Err(format!(
            "I1: size/scale came out {:?} at {}",
            f.size, f.scale
        ));
    }
    if f.time != Duration::from_millis(5) || !f.focused {
        return Err(format!(
            "I1: time {:?} focused {} (set 5 ms, true)",
            f.time, f.focused
        ));
    }
    if f.events != [Event::Focus(true)] {
        return Err(format!(
            "I1: focus(true) came out as {:?}, not one Focus(true)",
            f.events
        ));
    }
    let f = take(&mut s);
    if !f.events.is_empty() {
        return Err(format!(
            "I3: the first frame's events came out again: {:?}",
            f.events
        ));
    }
    if f.time != Duration::from_millis(5) {
        return Err(format!(
            "I1: time moved to {:?} with nothing setting it",
            f.time
        ));
    }
    s.resize((100.0, 50.0)).set_scale(1.0);
    let f = take(&mut s);
    if f.size != (100.0, 50.0) || f.scale != 1.0 {
        return Err(format!(
            "I1: resize/set_scale came out {:?} at {}",
            f.size, f.scale
        ));
    }

    // I2, I3, I4: a script where the pointer and the modifiers change mid-frame.
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    s.pointer_to(10.0, 20.0)
        .press(PointerButton::Primary)
        .set_modifiers(shift)
        .pointer_to(30.0, 40.0)
        .scroll_lines(0.0, -1.0)
        .release(PointerButton::Primary);
    let want = vec![
        Event::PointerMoved { pos: (10.0, 20.0) },
        Event::PointerButton {
            pos: (10.0, 20.0),
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::default(),
        },
        Event::ModifiersChanged(shift),
        Event::PointerMoved { pos: (30.0, 40.0) },
        Event::Scroll {
            pos: (30.0, 40.0),
            delta: ScrollDelta::Lines { x: 0.0, y: -1.0 },
            modifiers: shift,
        },
        Event::PointerButton {
            pos: (30.0, 40.0),
            button: PointerButton::Primary,
            pressed: false,
            modifiers: shift,
        },
    ];
    let f = take(&mut s);
    if f.events != want {
        return Err(format!(
            "I2/I4: events out of order or with stale position/modifiers: {:?}",
            f.events
        ));
    }
    let f = take(&mut s);
    if !f.events.is_empty() {
        return Err(format!(
            "I3: events came out in a second frame: {:?}",
            f.events
        ));
    }
    // The pointer and modifiers persisted: a press with no move still sits at (30, 40) under shift.
    s.press(PointerButton::Secondary);
    let f = take(&mut s);
    let want = vec![Event::PointerButton {
        pos: (30.0, 40.0),
        button: PointerButton::Secondary,
        pressed: true,
        modifiers: shift,
    }];
    if f.events != want {
        return Err(format!(
            "I3: pointer position or modifiers did not persist: {:?}",
            f.events
        ));
    }
    if f.time != Duration::from_millis(5) || !f.focused || f.size != (100.0, 50.0) {
        return Err("I3: size, time or focus did not persist across frames".into());
    }

    // I5
    let a = Key::Character("a".into());
    s.key_down(a.clone(), Some("a\u{7}"))
        .key_down(a.clone(), Some("\u{7}"))
        .key_down(a.clone(), None);
    let f = take(&mut s);
    let key = |pressed| Event::Key {
        key: a.clone(),
        pressed,
        repeat: false,
        modifiers: shift,
    };
    let want = vec![key(true), Event::Text("a".into()), key(true), key(true)];
    if f.events != want {
        return Err(format!("I5: key and text events came out {:?}", f.events));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ScriptedInput, scripted_input_contract};
    use crate::access::{AccessAction, AccessValue, Action};
    use crate::input::Event;
    use crate::ui::WidgetId;

    #[test]
    fn the_accessibility_and_clipboard_builders_append_their_events() {
        let target = WidgetId::ROOT.with("field");
        let mut input = ScriptedInput::new((100.0, 50.0), 1.0);
        input
            .access(AccessAction {
                target,
                action: Action::SetValue,
                value: Some(AccessValue::Number(2.5)),
            })
            .clipboard_text("pasted")
            .clipboard_unavailable();
        let frame = input.end_frame();
        assert_eq!(
            frame.events,
            vec![
                Event::Access(AccessAction {
                    target,
                    action: Action::SetValue,
                    value: Some(AccessValue::Number(2.5)),
                }),
                Event::ClipboardText("pasted".to_owned()),
                Event::ClipboardUnavailable,
            ]
        );
        assert!(input.end_frame().events.is_empty());
    }

    #[test]
    fn scripted_input_meets_its_contract() {
        assert_eq!(
            scripted_input_contract(&mut |s: &mut ScriptedInput| s.end_frame()),
            Ok(())
        );
    }

    /// A fake that keeps the events (a second frame repeats them) must fail I3; one that advances
    /// the clock itself must fail I1; one that reorders the events must fail I2/I4.
    #[test]
    fn contract_catches_broken_scripted_inputs() {
        use crate::input::RawInput;
        use std::time::Duration;

        let mut kept: Vec<Event> = Vec::new();
        let repeats = scripted_input_contract(&mut |s: &mut ScriptedInput| {
            let mut f = s.end_frame();
            kept.extend(f.events.drain(..));
            f.events = kept.clone();
            f
        });
        assert!(repeats.is_err_and(|e| e.starts_with("I3")));

        let mut n = 0u32;
        let clock = scripted_input_contract(&mut |s: &mut ScriptedInput| {
            n += 1;
            let mut f: RawInput = s.end_frame();
            f.time += Duration::from_millis(u64::from(n));
            f
        });
        assert!(clock.is_err_and(|e| e.starts_with("I1")));

        let reversed = scripted_input_contract(&mut |s: &mut ScriptedInput| {
            let mut f = s.end_frame();
            f.events.reverse();
            f
        });
        assert!(reversed.is_err_and(|e| e.starts_with("I2") || e.starts_with("I5")));
    }
}

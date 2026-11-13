//! The input seam: what the app hands the toolkit each frame ([`RawInput`]) and what the toolkit
//! hands back ([`FrameOutput`]). Plain data and std only: a windowing adapter translates its
//! events into [`RawInput`] and applies [`FrameOutput`] to its window; tests script both.
//!
//! All positions, sizes and pixel scroll amounts are logical (physical ÷ scale). Time is whatever
//! the app supplies; the toolkit never reads a clock. File drops arrive as events that carry a path
//! the toolkit never opens. Touch, gestures, theme and occlusion events are not modelled yet.

use std::path::PathBuf;
use std::time::Duration;

use crate::access::{AccessAction, SemanticTree};
use crate::motion::{Cadence, Time};
use crate::paint::Rect;
use crate::shortcuts::CommandId;
use crate::ui::WidgetId;
use crate::window::ResizeEdge;

/// Everything the app knows about one window at the start of a frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RawInput {
    /// Window content size in logical pixels.
    pub size: (f32, f32),
    /// Device pixels per logical pixel.
    pub scale: f32,
    /// App-supplied time since an app-chosen origin.
    pub time: Duration,
    /// Whether the window has keyboard focus.
    pub focused: bool,
    /// Events since the previous frame, in arrival order.
    pub events: Vec<Event>,
}

/// One input event. Positions are logical pixels.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The pointer moved inside the window.
    PointerMoved { pos: (f32, f32) },
    /// The pointer left the window.
    PointerLeft,
    /// A pointer button changed state at `pos`.
    PointerButton {
        pos: (f32, f32),
        button: PointerButton,
        pressed: bool,
        modifiers: Modifiers,
    },
    /// A scroll gesture with the pointer at `pos`.
    Scroll {
        pos: (f32, f32),
        delta: ScrollDelta,
        modifiers: Modifiers,
    },
    /// A key changed state.
    Key {
        key: Key,
        pressed: bool,
        repeat: bool,
        modifiers: Modifiers,
    },
    /// Text typed (see [`printable`]); follows the `Key` press that produced it.
    Text(String),
    /// Input-method editor event.
    Ime(ImeEvent),
    /// The window gained or lost keyboard focus.
    Focus(bool),
    /// The modifier keys changed.
    ModifiersChanged(Modifiers),
    /// An assistive technology requested an action on a node.
    Access(AccessAction),
    /// The text the clipboard held, answering a [`ClipboardRequest::Read`].
    ClipboardText(String),
    /// The clipboard holds no text or cannot be read, answering a [`ClipboardRequest::Read`].
    ClipboardUnavailable,
    /// A file is being dragged over the window, with the pointer at `pos` (the adapter's last
    /// pointer position: the OS gives none). One event per file. The toolkit only carries the
    /// path; it never opens it (D20).
    FileHover { pos: (f32, f32), path: PathBuf },
    /// The dragged file left the window or the drag was cancelled.
    FileHoverCancelled,
    /// A file was dropped on the window, with the pointer at `pos`. One event per file.
    FileDrop { pos: (f32, f32), path: PathBuf },
}

/// A pointer button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Back,
    Forward,
    Other(u16),
}

/// A scroll amount, signed as the platform gives it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDelta {
    /// Notched wheel: lines.
    Lines { x: f32, y: f32 },
    /// Touchpad: logical pixels.
    Pixels { x: f32, y: f32 },
}

/// A logical key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Named(NamedKey),
    /// A character key, as the layout produces it.
    Character(String),
    /// Anything else: unmapped named keys, dead keys, unidentified keys.
    Unidentified,
}

/// Named keys the toolkit models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NamedKey {
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,
    Escape,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

/// Modifier keys held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    /// Windows / Command / Super.
    pub logo: bool,
}

/// An input-method editor event. Offsets are byte offsets, as the platform gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeEvent {
    Enabled,
    /// Composing text; `cursor` is the selection inside `text`, `None` hides the cursor. An empty
    /// `text` clears the preedit.
    Preedit {
        text: String,
        cursor: Option<(usize, usize)>,
    },
    Commit(String),
    Disabled,
}

/// One event a focused widget reads through `Ui::focus_events`, in arrival order. Keys carry
/// their modifiers and repeat flag; characters come as [`Key::Character`], named keys as
/// [`Key::Named`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusEvent {
    Key {
        key: Key,
        modifiers: Modifiers,
        repeat: bool,
    },
    /// Typed text. Dropped at frame begin after a Command or Control chord (see `Ui::begin`).
    Text(String),
    /// An input-method event.
    Ime(ImeEvent),
    /// The clipboard's text, answering a read.
    ClipboardText(String),
    /// The clipboard could not answer a read.
    ClipboardUnavailable,
}

/// The modifier that makes a chord "primary" on a platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primary {
    /// Command / Windows / Super.
    Logo,
    Control,
}

/// The modifier that turns arrow keys into word motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordModifier {
    Alt,
    Control,
}

/// Where a word-right motion lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordMotion {
    /// The end of the current word (macOS).
    EndOfWord,
    /// The start of the next word (Windows, Linux).
    StartOfNext,
}

/// The platform's keyboard conventions, handed in by the app (the toolkit never reads the OS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keymap {
    pub primary: Primary,
    pub word: WordModifier,
    pub word_motion: WordMotion,
}

impl Keymap {
    /// macOS: Command is primary, Option moves by word, to the end of the word.
    pub const MAC: Keymap = Keymap {
        primary: Primary::Logo,
        word: WordModifier::Alt,
        word_motion: WordMotion::EndOfWord,
    };
    /// Windows and Linux: Control is primary and the word modifier, to the next word's start.
    pub const PC: Keymap = Keymap {
        primary: Primary::Control,
        word: WordModifier::Control,
        word_motion: WordMotion::StartOfNext,
    };

    /// Whether the primary modifier is held (other modifiers are the caller's concern).
    pub fn is_primary(&self, m: Modifiers) -> bool {
        match self.primary {
            Primary::Logo => m.logo,
            Primary::Control => m.control,
        }
    }

    /// Whether the word modifier is held.
    pub fn is_word(&self, m: Modifiers) -> bool {
        match self.word {
            WordModifier::Alt => m.alt,
            WordModifier::Control => m.control,
        }
    }
}

/// A clipboard operation the toolkit asks of the adapter. Requests are answered, if at all, by
/// an `Event` on a later frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardRequest {
    /// Put this text on the clipboard.
    Write(String),
    /// Read the clipboard text; answered once.
    Read,
}

/// When the app should draw its next frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repaint {
    /// Nothing is animating or changing, and the frame's input changed nothing it drew before
    /// reacting to; the app may wait for input.
    #[default]
    Idle,
    /// Draw again as soon as possible: something is in flight, or this frame's input changed
    /// state after the widgets drew (one catch-up frame, D81).
    Now,
    /// Draw again at this app time (an animation's next frame).
    At(Time),
}

impl Repaint {
    /// Puts a `Now` on the grid of `rate`: it becomes the first tick after `now`, or stays `Now`
    /// under [`Cadence::EveryFrame`]. `Idle` and `At(_)` are returned unchanged.
    ///
    /// An app that would wait in the surface's acquire between frames uses this to draw a run of
    /// `Now` frames at the display's rate instead. A grid at or above the display's real rate
    /// under a queueing present mode (`Fifo`) can fill the swapchain queue and wait in acquire
    /// again (D299).
    pub fn paced(self, now: Time, rate: Cadence) -> Repaint {
        match self {
            Repaint::Now => rate.next_tick(now).map_or(Repaint::Now, Repaint::At),
            other => other,
        }
    }
}

/// What the toolkit asks of the window after a frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameOutput {
    /// The cursor icon to show.
    pub cursor: CursorIcon,
    /// Where text is being edited, in logical pixels; `None` when no text field has focus (the
    /// adapter then disables IME).
    pub ime_area: Option<Rect>,
    /// Window operations, in order.
    pub commands: Vec<WindowCommand>,
    /// When the next frame is due.
    pub repaint: Repaint,
    /// Clipboard operations, in order; at most one write survives a frame.
    pub clipboard: Vec<ClipboardRequest>,
    /// The semantic tree of the frame, for assistive technology.
    pub tree: SemanticTree,
    /// The widget that has keyboard focus, if any.
    pub focused: Option<WidgetId>,
    /// The shortcut commands that fired this frame, in order.
    pub fired: Vec<CommandId>,
}

/// A window operation the app performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowCommand {
    /// Start moving the window with the pointer.
    DragMove,
    /// Start resizing from an edge.
    DragResize(ResizeEdge),
    Minimize,
    ToggleMaximize,
    /// The app closes the window.
    Close,
    /// Draw another frame.
    Redraw,
    /// Resize the window's content to this size in the frame's logical pixels (the units of
    /// `RawInput::size`); a zero side does nothing.
    SetSize {
        /// The new content width.
        width: u32,
        /// The new content height.
        height: u32,
    },
    /// Keep the window above other apps' windows or not; for a window group, route it through
    /// `WindowGroup::set_always_on_top`.
    SetAlwaysOnTop(bool),
}

/// Cursor icons: the CSS cursor names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum CursorIcon {
    #[default]
    Default,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
    AllScroll,
    ZoomIn,
    ZoomOut,
    DndAsk,
    AllResize,
}

impl CursorIcon {
    /// Every icon, in declaration order.
    pub const ALL: [CursorIcon; 36] = [
        CursorIcon::Default,
        CursorIcon::ContextMenu,
        CursorIcon::Help,
        CursorIcon::Pointer,
        CursorIcon::Progress,
        CursorIcon::Wait,
        CursorIcon::Cell,
        CursorIcon::Crosshair,
        CursorIcon::Text,
        CursorIcon::VerticalText,
        CursorIcon::Alias,
        CursorIcon::Copy,
        CursorIcon::Move,
        CursorIcon::NoDrop,
        CursorIcon::NotAllowed,
        CursorIcon::Grab,
        CursorIcon::Grabbing,
        CursorIcon::EResize,
        CursorIcon::NResize,
        CursorIcon::NeResize,
        CursorIcon::NwResize,
        CursorIcon::SResize,
        CursorIcon::SeResize,
        CursorIcon::SwResize,
        CursorIcon::WResize,
        CursorIcon::EwResize,
        CursorIcon::NsResize,
        CursorIcon::NeswResize,
        CursorIcon::NwseResize,
        CursorIcon::ColResize,
        CursorIcon::RowResize,
        CursorIcon::AllScroll,
        CursorIcon::ZoomIn,
        CursorIcon::ZoomOut,
        CursorIcon::DndAsk,
        CursorIcon::AllResize,
    ];
}

/// The text a key press types: `text` without control characters, or `None` when nothing is left.
/// Adapters and fakes use this one rule to decide whether a press emits [`Event::Text`].
pub fn printable(text: &str) -> Option<String> {
    let t: String = text.chars().filter(|c| !c.is_control()).collect();
    (!t.is_empty()).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::{CursorIcon, CursorIcon as C, printable};

    /// The declaration index of every icon. No wildcard: a variant added without an entry here
    /// stops the build.
    fn index(icon: CursorIcon) -> usize {
        match icon {
            C::Default => 0,
            C::ContextMenu => 1,
            C::Help => 2,
            C::Pointer => 3,
            C::Progress => 4,
            C::Wait => 5,
            C::Cell => 6,
            C::Crosshair => 7,
            C::Text => 8,
            C::VerticalText => 9,
            C::Alias => 10,
            C::Copy => 11,
            C::Move => 12,
            C::NoDrop => 13,
            C::NotAllowed => 14,
            C::Grab => 15,
            C::Grabbing => 16,
            C::EResize => 17,
            C::NResize => 18,
            C::NeResize => 19,
            C::NwResize => 20,
            C::SResize => 21,
            C::SeResize => 22,
            C::SwResize => 23,
            C::WResize => 24,
            C::EwResize => 25,
            C::NsResize => 26,
            C::NeswResize => 27,
            C::NwseResize => 28,
            C::ColResize => 29,
            C::RowResize => 30,
            C::AllScroll => 31,
            C::ZoomIn => 32,
            C::ZoomOut => 33,
            C::DndAsk => 34,
            C::AllResize => 35,
        }
    }

    #[test]
    fn cursor_icon_all_lists_every_variant_in_declaration_order() {
        assert_eq!(CursorIcon::ALL.len(), 36);
        for (i, icon) in CursorIcon::ALL.iter().enumerate() {
            assert_eq!(index(*icon), i, "{icon:?}");
        }
    }

    #[test]
    fn printable_drops_control_characters() {
        assert_eq!(printable("\u{7}"), None);
        assert_eq!(printable("a\tb"), Some("ab".to_owned()));
        assert_eq!(printable(""), None);
        assert_eq!(printable("é"), Some("é".to_owned()));
    }
}

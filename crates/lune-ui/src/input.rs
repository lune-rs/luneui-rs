//! The input seam: what the app hands the toolkit each frame ([`RawInput`]) and what the toolkit
//! hands back ([`FrameOutput`]). Plain data and std only: a windowing adapter translates its
//! events into [`RawInput`] and applies [`FrameOutput`] to its window; tests script both.
//!
//! All positions, sizes and pixel scroll amounts are logical (physical ÷ scale). Time is whatever
//! the app supplies; the toolkit never reads a clock. Touch, gestures, file drops, theme and
//! occlusion events are not modelled yet.

use std::time::Duration;

use crate::paint::Rect;
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

/// The text a key press types: `text` without control characters, or `None` when nothing is left.
/// Adapters and fakes use this one rule to decide whether a press emits [`Event::Text`].
pub fn printable(text: &str) -> Option<String> {
    let t: String = text.chars().filter(|c| !c.is_control()).collect();
    (!t.is_empty()).then_some(t)
}

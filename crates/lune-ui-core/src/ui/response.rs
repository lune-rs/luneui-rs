//! What a widget asks of a hit region ([`Sense`]) and what the frame reports back ([`Response`]).

use crate::access::AccessValue;
use crate::paint::Rect;

use super::WidgetId;

/// What a hit region responds to. Every region blocks the regions under it, whatever it senses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sense {
    /// Reports `hovered` when it is the topmost region under the pointer.
    pub hover: bool,
    /// Captures a primary press; reports clicks.
    pub click: bool,
    /// Captures a primary press; reports drags past the 4 px threshold (a drag is then no click).
    pub drag: bool,
    /// Is a tab stop and takes focus on a press.
    pub focusable: bool,
    /// Receives scroll gestures.
    pub scroll: bool,
}

impl Sense {
    /// Hover only: a card or panel that blocks what is under it.
    pub const HOVER: Sense = Sense {
        hover: true,
        click: false,
        drag: false,
        focusable: false,
        scroll: false,
    };
    /// Hover and click, not a tab stop.
    pub const CLICK: Sense = Sense {
        hover: true,
        click: true,
        drag: false,
        focusable: false,
        scroll: false,
    };
    /// Hover, click and focus: a button, a toggle.
    pub const CLICK_FOCUS: Sense = Sense {
        hover: true,
        click: true,
        drag: false,
        focusable: true,
        scroll: false,
    };
    /// Hover, click and drag.
    pub const DRAG: Sense = Sense {
        hover: true,
        click: true,
        drag: true,
        focusable: false,
        scroll: false,
    };
    /// Scroll only.
    pub const SCROLL: Sense = Sense {
        hover: false,
        click: false,
        drag: false,
        focusable: false,
        scroll: true,
    };
}

/// One widget's reading of the frame's input, from [`Ui::interact`](super::Ui::interact).
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub id: WidgetId,
    /// The registered hit rect: clipped, then transformed, in logical pixels.
    pub rect: Rect,
    /// False inside `with_enabled(false, ..)`; every other field is then false or empty.
    pub enabled: bool,
    pub hovered: bool,
    /// Holds the pointer capture: from the press until the release, inside the rect or not.
    pub pressed: bool,
    pub clicked: bool,
    pub double_clicked: bool,
    pub secondary_clicked: bool,
    pub drag_started: bool,
    pub dragged: bool,
    /// Pointer movement since the previous frame while dragging (the start frame: the travel from
    /// the press point).
    pub drag_delta: (f32, f32),
    pub drag_stopped: bool,
    pub focused: bool,
    pub focus_visible: bool,
    /// Set by the widget ([`Response::mark_changed`]) when it changed the value it was handed.
    pub changed: bool,
    pub scroll_lines: (f32, f32),
    pub scroll_pixels: (f32, f32),
    /// A value an assistive technology set.
    pub set_value: Option<AccessValue>,
    /// An assistive technology asked to scroll this widget into view.
    pub scroll_into_view: bool,
}

impl Response {
    /// A response with every interaction field false or empty.
    pub(crate) fn inert(id: WidgetId, rect: Rect, enabled: bool) -> Response {
        Response {
            id,
            rect,
            enabled,
            hovered: false,
            pressed: false,
            clicked: false,
            double_clicked: false,
            secondary_clicked: false,
            drag_started: false,
            dragged: false,
            drag_delta: (0.0, 0.0),
            drag_stopped: false,
            focused: false,
            focus_visible: false,
            changed: false,
            scroll_lines: (0.0, 0.0),
            scroll_pixels: (0.0, 0.0),
            set_value: None,
            scroll_into_view: false,
        }
    }

    /// Records that the widget changed the app-owned value it was handed.
    pub fn mark_changed(&mut self) {
        self.changed = true;
    }
}

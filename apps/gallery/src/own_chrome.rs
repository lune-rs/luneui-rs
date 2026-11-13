//! The window chrome the gallery draws itself where the OS draws none. On Windows the glass
//! window is borderless (`glass_attributes`), so without this the gallery could not be moved,
//! resized, minimised, maximised or closed with the mouse. macOS keeps its own title bar and
//! Linux its decorations, so there it draws nothing.

use std::time::Instant;

use lune_ui::input::{CursorIcon, WindowCommand};
use lune_ui::paint::Rect;
use lune_ui::ui::{Sense, Ui};
use lune_ui::widgets::caption::{CaptionButton, CaptionOptions};
use lune_ui::window::{CAPTION_BUTTON_WIDTH, DoubleClick, EDGE_BORDER, ResizeEdge, edge_at};

use crate::scenes::TITLE_BAND;

/// Whether this platform's glass window has no OS chrome.
pub(crate) const OWN_CHROME: bool = cfg!(target_os = "windows");

/// The window's state the chrome reads: the caption's middle glyph and whether edges resize.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowLook {
    pub(crate) maximized: bool,
    pub(crate) focused: bool,
}

/// The title band's double-click detector, kept across frames.
#[derive(Debug, Default)]
pub(crate) struct OwnChrome {
    title_clicks: DoubleClick,
}

impl OwnChrome {
    /// The title band as a drag strip, registered before the page draws so any widget the page
    /// puts in the band takes its press first. A press starts the OS move; a second press within
    /// the double-click time maximises or restores instead.
    pub(crate) fn under(&mut self, ui: &mut Ui<'_>) -> Option<WindowCommand> {
        let win = ui.painter().window;
        let strip = Rect::new(
            win.x,
            win.y,
            (win.w - 3.0 * CAPTION_BUTTON_WIDTH).max(0.0),
            TITLE_BAND,
        );
        let id = ui.id("gallery.own-chrome.title");
        let r = ui.interact(id, strip, Sense::CLICK);
        if !r.press_started {
            return None;
        }
        Some(if self.title_clicks.click(Instant::now()) {
            WindowCommand::ToggleMaximize
        } else {
            WindowCommand::DragMove
        })
    }

    /// The resize edges and the caption buttons, registered after the page so they are on top.
    pub(crate) fn over(&mut self, ui: &mut Ui<'_>, look: WindowLook) -> Option<WindowCommand> {
        let win = ui.painter().window;
        let mut command = None;
        if !look.maximized {
            command = edges(ui, win);
        }
        let opts = CaptionOptions {
            height: TITLE_BAND - 2.0,
            maximized: look.maximized,
            active: look.focused,
        };
        match ui.caption_buttons("gallery.own-chrome.caption", win, opts) {
            Some(CaptionButton::Minimize) => Some(WindowCommand::Minimize),
            Some(CaptionButton::Maximize) => Some(WindowCommand::ToggleMaximize),
            Some(CaptionButton::Close) => Some(WindowCommand::Close),
            None => command,
        }
    }
}

/// The four border strips of a borderless window: the resize cursor on hover, the OS resize on
/// a press.
fn edges(ui: &mut Ui<'_>, win: Rect) -> Option<WindowCommand> {
    let b = EDGE_BORDER;
    let strips = [
        ("n", Rect::new(win.x, win.y, win.w, b)),
        ("s", Rect::new(win.x, win.bottom() - b, win.w, b)),
        ("w", Rect::new(win.x, win.y, b, win.h)),
        ("e", Rect::new(win.right() - b, win.y, b, win.h)),
    ];
    let mut command = None;
    for (key, rect) in strips {
        let id = ui.id(("gallery.own-chrome.edge", key));
        let r = ui.interact(id, rect, Sense::CLICK);
        let Some(at) = r.press_pos.or(ui.pointer_pos()) else {
            continue;
        };
        let local = (at.0 - win.x, at.1 - win.y);
        let Some(edge) = edge_at(local, win.w, win.h) else {
            continue;
        };
        if r.hovered || r.pressed {
            ui.set_cursor(cursor(edge));
        }
        if r.press_started {
            command = Some(WindowCommand::DragResize(edge));
        }
    }
    command
}

const fn cursor(edge: ResizeEdge) -> CursorIcon {
    match edge {
        ResizeEdge::East | ResizeEdge::West => CursorIcon::EwResize,
        ResizeEdge::North | ResizeEdge::South => CursorIcon::NsResize,
        ResizeEdge::NorthEast | ResizeEdge::SouthWest => CursorIcon::NeswResize,
        ResizeEdge::NorthWest | ResizeEdge::SouthEast => CursorIcon::NwseResize,
    }
}

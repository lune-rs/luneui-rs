//! The window a tray surface draws in: borderless, above everything, shown and hidden by the app.
//!
//! On macOS the window is turned into a non-activating panel, so it takes keys and reports focus
//! loss without taking the front from the app the user is in. That rests on swapping the live
//! window's class, and a window freed in the panel class aborts the process. [`PopoverWindow`]
//! holds a window handle and restores the class in its own `Drop`, so while the guard exists the
//! window cannot be freed, and the class is back before the guard lets go. Keep the guard alive for
//! as long as the window is in use and let it drop normally.
//!
//! winit activates the app when it first shows a window unless the event loop is built with
//! `EventLoopBuilderExtMacOS::with_activate_ignoring_other_apps(false)`, and the panel cannot undo
//! that: an app that shows popovers needs that call to leave the front alone.
//!
//! On Windows the window takes focus when shown (a window that does not gets no keys and no focus
//! loss); elsewhere it is a plain window.

use std::marker::PhantomData;
use std::sync::Arc;

use winit::window::Window;

use crate::platform;

/// The guard of a popover window. See the module docs.
#[derive(Debug)]
pub struct PopoverWindow {
    window: Arc<Window>,
    panel: platform::Panel,
    /// Keeps the type `!Send` and `!Sync` on every platform: `Drop` must run on the main thread.
    main_thread: PhantomData<*const ()>,
}

impl PopoverWindow {
    /// Makes `window`, still hidden, a popover window. Build it with
    /// `lune_ui_winit::popover_attributes`, on the main thread. A visible window, or any failure
    /// of the OS step, is an `Err` and leaves the window as it was.
    pub fn wrap(window: Arc<Window>) -> Result<Self, String> {
        if window.is_visible() == Some(true) {
            return Err("a popover window must be wrapped while hidden".into());
        }
        let panel = platform::Panel::wrap(&window)?;
        Ok(Self {
            window,
            panel,
            main_thread: PhantomData,
        })
    }

    /// Shows the window and gives it the keys.
    pub fn show(&self) {
        self.panel.show(&self.window);
    }

    /// Hides the window.
    pub fn hide(&self) {
        self.panel.hide(&self.window);
    }

    /// The window.
    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }
}

impl Drop for PopoverWindow {
    /// Hides the window and gives it back as it was. This runs before the `window` field goes, so
    /// the handle is still alive throughout.
    fn drop(&mut self) {
        self.panel.restore(&self.window);
    }
}

/// The name of the app at the front on macOS, `None` elsewhere: how a platform smoke test sees that
/// showing a popover took nothing from the user's app.
pub fn frontmost_app() -> Option<String> {
    platform::frontmost_app()
}

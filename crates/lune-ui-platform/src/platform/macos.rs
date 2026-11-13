//! macOS: the popover window as a non-activating panel, and the status item's appearance.
//!
//! A winit window is an `NSWindow`: it cannot hold `NSWindowStyleMaskNonactivatingPanel`, and it
//! does not become key while the app is inactive. So the live object's class is swapped to an
//! `NSPanel` subclass. The class at that moment is the key-value observing subclass the runtime made
//! for winit's and AppKit's observers (whose `-class` answers the plain name, so only the runtime's
//! `object_getClass` sees it), and those observers must find it again when they go: a window
//! released in the panel class aborts the process.

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, NSObjectProtocol};
use objc2::{ClassType, MainThreadMarker, MainThreadOnly, define_class};
use objc2_app_kit::{
    NSAppearanceCustomization, NSPanel, NSPopUpMenuWindowLevel, NSView, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tray_icon::TrayIcon;
use winit::window::Window;

define_class!(
    // SAFETY: `NSPanel` has no subclassing rule this breaks: the class adds no ivars and overrides
    // two read-only properties with their documented types, and it implements no `Drop`.
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    #[name = "LuneUiPopoverPanel"]
    struct PopoverPanel;

    // SAFETY: the signatures match `NSWindow`'s `canBecomeKeyWindow` and `canBecomeMainWindow`.
    impl PopoverPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool {
            false
        }
    }

    // SAFETY: `NSPanel` conforms to `NSObject`, and the class overrides none of its methods.
    unsafe impl NSObjectProtocol for PopoverPanel {}
);

/// A window in the panel class, and what it needs to go back.
#[derive(Debug)]
pub(crate) struct Panel {
    ns: Retained<NSWindow>,
    saved_class: &'static AnyClass,
    saved_mask: NSWindowStyleMask,
}

impl Panel {
    pub(crate) fn wrap(window: &Window) -> Result<Self, String> {
        MainThreadMarker::new().ok_or("a popover window is wrapped on the main thread")?;
        let ns = ns_window(window).ok_or("no NSWindow behind this window")?;
        let panel_class = PopoverPanel::class();
        let saved_class = live_class(&ns);
        // A class under the panel means a guard already holds this window: a second one would save
        // the panel as the class to restore.
        let mut class = Some(saved_class);
        while let Some(c) = class {
            if c == panel_class {
                return Err("this window is already a popover window".into());
            }
            class = c.superclass();
        }
        // The swap keeps the object's memory, so the new class must not need more of it.
        if panel_class.instance_size() > saved_class.instance_size() {
            return Err("the panel class is larger than the window's class".into());
        }

        let saved_mask = ns.styleMask();
        set_live_class(&ns, panel_class);
        ns.setStyleMask(saved_mask | NSWindowStyleMask::NonactivatingPanel);
        let panel = Self {
            ns,
            saved_class,
            saved_mask,
        };
        if !panel
            .ns
            .styleMask()
            .contains(NSWindowStyleMask::NonactivatingPanel)
        {
            panel.give_back();
            return Err("the window kept no non-activating bit".into());
        }
        panel.ns.setLevel(NSPopUpMenuWindowLevel);
        panel.ns.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        panel.ns.setHidesOnDeactivate(false);
        Ok(panel)
    }

    /// Never `NSApp.activate`: the app at the front stays there.
    pub(crate) fn show(&self, _window: &Window) {
        self.ns.orderFrontRegardless();
        self.ns.makeKeyWindow();
    }

    pub(crate) fn hide(&self, _window: &Window) {
        self.ns.orderOut(None);
    }

    pub(crate) fn restore(&self, _window: &Window) {
        self.ns.orderOut(None);
        self.give_back();
    }

    fn give_back(&self) {
        self.ns.setStyleMask(self.saved_mask);
        set_live_class(&self.ns, self.saved_class);
    }
}

/// The class the runtime dispatches on, not what `-class` answers.
fn live_class(ns: &NSWindow) -> &'static AnyClass {
    let object: &AnyObject = ns.as_ref();
    object.class()
}

fn set_live_class(ns: &NSWindow, class: &AnyClass) {
    let object: &AnyObject = ns.as_ref();
    // SAFETY: `class` is the panel class or the class the object had before, both of them under
    // `NSWindow`; the object's memory is as large as either (`Panel::wrap` compares the instance
    // sizes, and the panel class adds no ivars), and every caller is on the main thread.
    unsafe { AnyObject::set_class(object, class) };
}

pub(crate) fn ns_window(window: &Window) -> Option<Retained<NSWindow>> {
    let RawWindowHandle::AppKit(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    // SAFETY: winit guarantees `ns_view` is a live NSView on the main thread while the window
    // exists, and `Panel::wrap` checked the main thread.
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    view.window()
}

pub(crate) fn frontmost_app() -> Option<String> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    Some(app.localizedName()?.to_string())
}

pub(crate) fn bar_is_dark(item: &TrayIcon) -> Option<bool> {
    let mtm = MainThreadMarker::new()?;
    let button = item.ns_status_item()?.button(mtm)?;
    // Every dark appearance (`DarkAqua`, `VibrantDark`, their high-contrast forms) says so in its name.
    Some(
        button
            .effectiveAppearance()
            .name()
            .to_string()
            .contains("Dark"),
    )
}

//! The tray item: the notification-area icon on Windows, the menu-bar status item on macOS. It
//! holds an image and a tooltip and reports clicks with its rect; the surfaces it opens are the
//! app's, drawn in a [`crate::popover::PopoverWindow`] and placed by the core's `tray` module.
//!
//! macOS and Windows go through `tray-icon`, with no menu attached and a non-template image (the
//! coloured dot could not survive a template). Other platforms have no tray: [`Tray::new`] is
//! [`TrayError::Unsupported`].

use std::marker::PhantomData;

use crate::platform;

/// A straight-alpha RGBA picture, `width * height * 4` bytes, row-major from the top left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The tray item's rect: physical pixels, top-left origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrayRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The mouse button of a click.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrayButton {
    Left,
    Right,
    Other,
}

/// What the tray item reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayEvent {
    /// A button came up over the item, with the item's rect at that moment.
    Click { button: TrayButton, rect: TrayRect },
}

/// Why a tray call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayError {
    /// This platform has no tray.
    Unsupported,
    /// The picture's byte count is not `width * height * 4`, or it has no pixels.
    BadImage,
    /// The OS refused; the text is its reason.
    Os(String),
}

impl std::fmt::Display for TrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => f.write_str("no tray on this platform"),
            Self::BadImage => f.write_str("the tray image's size does not match its bytes"),
            Self::Os(reason) => write!(f, "the OS refused the tray item: {reason}"),
        }
    }
}

impl std::error::Error for TrayError {}

/// The tray item. It lives on the main thread, and the OS item goes when this drops.
#[derive(Debug)]
pub struct Tray {
    item: platform::TrayItem,
    /// Keeps the type `!Send` and `!Sync` on every platform, whatever `item` is.
    main_thread: PhantomData<*const ()>,
}

impl Tray {
    /// Puts `image` in the tray with `tooltip`. Call [`set_event_handler`] first: without a handler
    /// no click reaches the app.
    pub fn new(image: TrayImage, tooltip: &str) -> Result<Tray, TrayError> {
        check_image(&image)?;
        Ok(Tray {
            item: platform::TrayItem::new(image, tooltip)?,
            main_thread: PhantomData,
        })
    }

    /// Replaces the picture.
    pub fn set_image(&self, image: TrayImage) -> Result<(), TrayError> {
        check_image(&image)?;
        self.item.set_image(image)
    }

    /// Replaces the tooltip. Windows cuts a long one: its field holds 128 UTF-16 units.
    pub fn set_tooltip(&self, tooltip: &str) -> Result<(), TrayError> {
        self.item.set_tooltip(tooltip)
    }

    /// The item's rect now, or `None` while the OS has not placed it. On macOS that lasts until the
    /// event loop has run: straight after [`Tray::new`] the status item has no height.
    pub fn rect(&self) -> Option<TrayRect> {
        self.item.rect().and_then(placed)
    }

    /// Whether the bar behind the item is dark, which picks the glyph's tone: the status button's
    /// effective appearance on macOS, `None` elsewhere. `None` too until the item is placed, as
    /// [`Tray::rect`] is, because an item not yet on the bar answers for no bar.
    pub fn bar_is_dark(&self) -> Option<bool> {
        self.rect()?;
        self.item.bar_is_dark()
    }
}

/// Sets the handler of the tray's events. Only the first call takes effect, and it must come before
/// the first [`Tray::new`]: the OS layer fixes its route at the first event. The handler runs on the
/// thread the OS reports on, so it should only pass the event to the app's event loop.
pub fn set_event_handler(f: impl Fn(TrayEvent) + Send + Sync + 'static) {
    platform::set_event_handler(f);
}

/// `rect`, if it has an area: an item the OS has not laid out yet has none.
fn placed(rect: TrayRect) -> Option<TrayRect> {
    (rect.w > 0.0 && rect.h > 0.0).then_some(rect)
}

/// `Ok` when the bytes are exactly the pixels the size says, and there is at least one.
fn check_image(image: &TrayImage) -> Result<(), TrayError> {
    let bytes = u64::from(image.width) * u64::from(image.height) * 4;
    if bytes == 0 || bytes != image.rgba.len() as u64 {
        return Err(TrayError::BadImage);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: TrayRect = TrayRect {
        x: 1863.0,
        y: -1.0,
        w: 34.0,
        h: 33.0,
    };

    #[test]
    fn a_rect_with_no_area_is_not_placed() {
        assert_eq!(placed(RECT), Some(RECT));
        // Measured on macOS straight after the status item is made.
        let unplaced = TrayRect {
            x: 0.0,
            y: 2700.0,
            w: 68.0,
            h: 0.0,
        };
        assert_eq!(placed(unplaced), None);
        assert_eq!(placed(TrayRect { w: 0.0, ..RECT }), None);
    }

    fn image(width: u32, height: u32, bytes: usize) -> TrayImage {
        TrayImage {
            width,
            height,
            rgba: vec![0; bytes],
        }
    }

    #[test]
    fn an_image_of_the_right_size_passes() {
        assert_eq!(check_image(&image(18, 18, 18 * 18 * 4)), Ok(()));
        assert_eq!(check_image(&image(36, 18, 36 * 18 * 4)), Ok(()));
    }

    #[test]
    fn a_size_mismatch_is_a_bad_image() {
        assert_eq!(
            check_image(&image(18, 18, 18 * 18 * 4 - 1)),
            Err(TrayError::BadImage)
        );
        assert_eq!(
            check_image(&image(18, 18, 18 * 18 * 4 + 4)),
            Err(TrayError::BadImage)
        );
        assert_eq!(check_image(&image(0, 18, 0)), Err(TrayError::BadImage));
        assert_eq!(check_image(&image(18, 0, 0)), Err(TrayError::BadImage));
    }

    #[test]
    fn a_size_past_u32_does_not_wrap_to_a_match() {
        // 2^16 * 2^16 * 4 wraps to 0 in 32 bits.
        assert_eq!(
            check_image(&image(1 << 16, 1 << 16, 0)),
            Err(TrayError::BadImage)
        );
    }
}

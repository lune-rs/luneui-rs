//! The tray item on macOS and Windows, over `tray-icon`.

use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::tray::{TrayButton, TrayError, TrayEvent, TrayImage, TrayRect};

pub(crate) struct TrayItem(TrayIcon);

impl std::fmt::Debug for TrayItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TrayItem")
    }
}

impl TrayItem {
    pub(crate) fn new(image: TrayImage, tooltip: &str) -> Result<Self, TrayError> {
        TrayIconBuilder::new()
            .with_icon(icon(image)?)
            .with_tooltip(tooltip)
            .with_icon_as_template(false)
            .with_menu_on_left_click(false)
            .build()
            .map(Self)
            .map_err(os)
    }

    pub(crate) fn set_image(&self, image: TrayImage) -> Result<(), TrayError> {
        self.0.set_icon(Some(icon(image)?)).map_err(os)
    }

    pub(crate) fn set_tooltip(&self, tooltip: &str) -> Result<(), TrayError> {
        self.0.set_tooltip(Some(tooltip)).map_err(os)
    }

    pub(crate) fn rect(&self) -> Option<TrayRect> {
        self.0.rect().map(tray_rect)
    }

    pub(crate) fn bar_is_dark(&self) -> Option<bool> {
        super::bar_is_dark(&self.0)
    }
}

pub(crate) fn set_event_handler(f: impl Fn(TrayEvent) + Send + Sync + 'static) {
    TrayIconEvent::set_event_handler(Some(move |event| {
        if let TrayIconEvent::Click {
            rect,
            button,
            button_state,
            ..
        } = event
        {
            let button = match button {
                MouseButton::Left => TrayButton::Left,
                MouseButton::Right => TrayButton::Right,
                MouseButton::Middle => TrayButton::Other,
            };
            let phase = match button_state {
                MouseButtonState::Down => ClickPhase::Press,
                MouseButtonState::Up => ClickPhase::Release,
            };
            if let Some(event) = click_event(button, phase, tray_rect(rect)) {
                f(event);
            }
        }
    }));
}

fn tray_rect(rect: tray_icon::Rect) -> TrayRect {
    TrayRect {
        x: rect.position.x,
        y: rect.position.y,
        w: f64::from(rect.size.width),
        h: f64::from(rect.size.height),
    }
}

fn icon(image: TrayImage) -> Result<Icon, TrayError> {
    Icon::from_rgba(image.rgba, image.width, image.height).map_err(|_| TrayError::BadImage)
}

fn os(error: tray_icon::Error) -> TrayError {
    TrayError::Os(error.to_string())
}

/// Whether a mouse report is the press or the release of its button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickPhase {
    Press,
    Release,
}

/// The tray's click rule: one event per release, none for a press.
fn click_event(button: TrayButton, phase: ClickPhase, rect: TrayRect) -> Option<TrayEvent> {
    match phase {
        ClickPhase::Press => None,
        ClickPhase::Release => Some(TrayEvent::Click { button, rect }),
    }
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
    fn a_press_sends_nothing() {
        for button in [TrayButton::Left, TrayButton::Right, TrayButton::Other] {
            assert_eq!(click_event(button, ClickPhase::Press, RECT), None);
        }
    }

    #[test]
    fn a_release_sends_one_click_with_the_button_and_the_rect() {
        for button in [TrayButton::Left, TrayButton::Right, TrayButton::Other] {
            assert_eq!(
                click_event(button, ClickPhase::Release, RECT),
                Some(TrayEvent::Click { button, rect: RECT })
            );
        }
    }
}

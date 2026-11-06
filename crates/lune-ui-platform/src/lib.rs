//! OS layer of the glass UI toolkit: the platform glass effect behind a transparent window, behind
//! the core's `Backdrop` capability ([`material`]), the tray item ([`tray`]) and the popover window
//! a tray surface draws in ([`popover`]). The OS code lives in `material/` and `platform/`, one
//! file per platform, selected by `cfg`.

// The platform modules name the core's backdrop types as `crate::backdrop::…`.
use lune_ui_core::backdrop;

pub mod material;
mod platform;
pub mod popover;
pub mod tray;

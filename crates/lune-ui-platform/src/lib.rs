//! OS backdrop material of the glass UI toolkit: the platform glass effect behind a transparent
//! window, per platform, behind the core's `Backdrop` capability. The platform code lives in
//! `material/`, one file per platform, selected by `cfg`.

// The platform modules name the core's backdrop types as `crate::backdrop::…`.
use lune_ui_core::backdrop;

pub mod material;

//! winit adapter of the glass UI toolkit: the glass window attributes and the resize edge → winit
//! direction mapping ([`window`]), and the translation of winit events into the core's `RawInput`
//! and the application of its `FrameOutput` ([`input`]), the OS clipboard ([`clipboard`]) and,
//! behind the `accesskit` feature, the bridge to the OS accessibility API (`access`). The app owns
//! the event loop and calls these from it.

#![forbid(unsafe_code)]

#[cfg(feature = "accesskit")]
pub mod access;
pub mod clipboard;
pub mod input;
mod platform;
pub mod window;

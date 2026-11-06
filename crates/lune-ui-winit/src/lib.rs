//! winit adapter of the glass UI toolkit: the glass window attributes and the resize edge → winit
//! direction mapping ([`window`]), and the translation of winit events into the core's `RawInput`
//! and the application of its `FrameOutput` ([`input`]), and the OS clipboard ([`clipboard`]). The
//! app owns the event loop and calls these from it.

#![forbid(unsafe_code)]

pub mod clipboard;
pub mod input;
mod platform;
pub mod window;

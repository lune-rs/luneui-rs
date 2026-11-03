//! Fakes of the toolkit's seams, for tests in this crate and in apps.
//!
//! - [`RecordingRenderer`]: the display-list seam's fake; records the draw calls a renderer
//!   issues for a [`Scene`](crate::scene::Scene), in the documented draw order.
//! - [`FakeText`]: the text seam's fake, deterministic sizes with no fonts; and
//!   [`text_system_contract`], the properties every [`TextSystem`](crate::text::TextSystem)
//!   must have.
//! - [`ScriptedInput`]: the input seam's fake, a scripted window producing [`RawInput`](crate::input::RawInput)
//!   frames.

mod input;
mod recording;
mod text;

pub use input::ScriptedInput;
pub use recording::{DrawCall, RecordedFrame, RecordingRenderer};
pub use text::{FakeText, text_system_contract};

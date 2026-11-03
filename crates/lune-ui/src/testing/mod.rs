//! Fakes of the toolkit's seams, for tests in this crate and in apps.
//!
//! - [`RecordingRenderer`]: the display-list seam's fake; records the draw calls a renderer
//!   issues for a [`Scene`](crate::scene::Scene), in the documented draw order.

mod recording;

pub use recording::{DrawCall, RecordedFrame, RecordingRenderer};

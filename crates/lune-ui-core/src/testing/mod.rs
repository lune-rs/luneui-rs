//! Fakes of the toolkit's seams, for tests in this crate and in apps.
//!
//! - [`RecordingRenderer`]: the display-list seam's fake; records the draw calls a renderer
//!   issues for a [`Scene`](crate::scene::Scene), in the documented draw order.
//! - [`FakeText`]: the text seam's fake, deterministic sizes with no fonts; and
//!   [`text_system_contract`], the properties every [`TextSystem`](crate::text::TextSystem)
//!   must have.
//! - [`NullMaterial`]: the material seam's fake, a backdrop with no OS glass; and
//!   [`backdrop_contract`], the properties every [`Backdrop`](crate::backdrop::Backdrop) must have.
//! - [`FakeImages`]: the image seam's fake, a store that keeps sizes; and
//!   [`image_store_contract`], the properties every [`ImageStore`](crate::image::ImageStore) must have.
//! - [`ScriptedInput`]: the input seam's fake, a scripted window producing [`RawInput`](crate::input::RawInput)
//!   frames; and [`scripted_input_contract`], the properties its frames must have.
//! - [`UiHarness`]: a `Ui` test bench (store, fake text, default strings, scripted input); and
//!   [`disabled_violations`], the nodes under a root that a disabled scope left enabled.

mod backdrop;
mod images;
mod input;
mod recording;
mod text;
mod ui;

pub use backdrop::{NullMaterial, backdrop_contract};
pub use images::{FakeImages, image_store_contract};
pub use input::{ScriptedInput, scripted_input_contract};
pub use recording::{DrawCall, RecordedFrame, RecordingRenderer};
pub use text::{FakeText, SEGMENT_TABLE, SegmentCase, spec_line_contract, text_system_contract};
pub use ui::{UiHarness, disabled_violations};

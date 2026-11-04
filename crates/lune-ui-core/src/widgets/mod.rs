//! Controls, styled after the house look's overrides of the Fluent theme (`DarkTheme.axaml`). Every
//! widget is generic over the app's hit type and reads its tokens from the painter's [`Theme`](crate::theme::Theme).

pub mod button;
pub mod checkbox;
pub mod combo;
pub mod dialog;
pub mod dropdown;
pub mod field;
pub mod focus_ring;
pub mod label;
pub mod menu;
pub mod popover;
pub mod popup_panel;
pub mod scroll_area;
pub mod scrollbar;
pub mod stepper;
pub mod text_area;
pub mod toggle;
pub mod tooltip;
pub mod virtual_list;

mod input_box;

pub use button::{button, button_width};
pub use checkbox::checkbox;
pub use combo::combo;
pub use field::field;
pub use label::label;
pub use popup_panel::popup_panel;
pub use stepper::stepper;
pub use toggle::toggle;

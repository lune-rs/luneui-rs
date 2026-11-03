//! App-agnostic glass UI toolkit: an immediate-mode painter on wgpu, widgets, OS backdrop
//! materials and window helpers, styled after the house look (Fluent/Avalonia; see `docs/glass-styling.md`).
//!
//! lune-ui knows nothing about the app that uses it. The app owns the winit event loop and passes
//! in everything else: fonts, the [`theme::Theme`], hit values (the painter and widgets are
//! generic over the app's hit type), sizes, scale and time. The crate reads neither the clock nor
//! the environment, so frames are deterministic.
//!
//! Modules:
//! - [`color`], [`theme`] (the `Theme` tokens and the dark preset with its glass
//!   layers), [`acrylic`] (Avalonia acrylic colour math);
//! - [`paint`] (the `Painter`), [`scene`] (the display list it builds: plain data and the
//!   draw-order contract), [`custom`] (app-owned GPU drawing inside a layer), [`text`] (specs and the
//!   `TextSystem` seam), [`text_cache`] (its glyphon implementation), [`icon`],
//!   [`widgets`] (controls, menus, dropdowns, dialogs), [`chrome`] (tab outline geometry);
//! - [`gpu`] (the GPU every target shares), [`frame`] (one target's renderer: the scene → blur →
//!   target pass sequence), [`blur`] (backdrop blur), [`surface`] (a window's swapchain), [`dump`]
//!   (offscreen targets and PNG readback);
//! - [`input`] (the input seam: `RawInput` in, `FrameOutput` out, plain data), [`winit_input`]
//!   (its winit translation and output application);
//! - [`backdrop`] (the material seam: [`GlassLayer`], [`BackdropKind`], `Material` and the
//!   [`Backdrop`] capability), [`material`] (its OS implementation per platform), [`window`] (edge resize, double click, caption buttons),
//!   [`winit_window`] (glass window attributes, edge → winit direction);
//! - [`testing`] (fakes of the seams for tests: the display-list recorder, a fake text system
//!   and its contract suite, a scripted input source, a null material and its contract suite).
//!
//! Values in the dark preset are copied from the house look's `DarkTheme.axaml` and `App.axaml`, so
//! apps styled after it match.

pub mod acrylic;
pub mod backdrop;
pub mod blur;
pub mod chrome;
pub mod color;
pub mod custom;
pub mod dump;
pub mod frame;
pub mod gpu;
pub mod icon;
pub mod input;
pub mod material;
pub mod paint;
pub mod scene;
pub mod surface;
pub mod testing;
pub mod text;
pub mod text_cache;
pub mod theme;
pub mod widgets;
pub mod window;
pub mod winit_input;
pub mod winit_window;

pub use backdrop::{Backdrop, BackdropKind, GlassLayer};
pub use theme::lunegit_dark;

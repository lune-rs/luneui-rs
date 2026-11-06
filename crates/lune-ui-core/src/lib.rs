//! App-agnostic glass UI toolkit, core: an immediate-mode painter, the display list it builds, widgets,
//! the theme, and the seam types (text, input, backdrop material) that adapter crates implement.
//! Styled after the house look (Fluent/Avalonia; see `docs/glass-styling.md`).
//!
//! This crate has no dependencies: no wgpu, winit or glyphon types appear in its signatures. The
//! renderer (`lune-ui-wgpu`), the window and input adapter (`lune-ui-winit`) and the OS material
//! (`lune-ui-platform`) depend on it, and the `lune-ui` facade re-exports all of them.
//!
//! lune-ui knows nothing about the app that uses it. The app owns the winit event loop and passes
//! in everything else: fonts, the [`theme::Theme`], hit values (the painter and widgets are
//! generic over the app's hit type), sizes, scale and time. The crate reads neither the clock nor
//! the environment, so frames are deterministic.
//!
//! Modules:
//! - `color`, `theme` (the `Theme` tokens and the dark preset with its glass layers), `acrylic`
//!   (Avalonia acrylic colour math);
//! - `image` (the `ImageStore` seam and image fit geometry),
//!   `paint` (the `Painter`), `scene` (the display list it builds: plain data and the draw-order
//!   contract), `text` (specs and the `TextSystem` seam), `icon`, `widgets` (controls, menus,
//!   dropdowns, dialogs), `chrome` (tab outline geometry);
//! - `text_edit` (the pure editing engine: caret, selection, undo, clipboard commands, preedit);
//! - `input` (the input seam: `RawInput` in, `FrameOutput` out, plain data), `layout` (pure stack,
//!   row, column and grid helpers);
//! - `json` (the bounded JSON reader behind theme and locale files);
//! - `shortcuts` (the scoped key-chord registry the frame resolves in two phases);
//! - `drag` (in-window drag and drop: the pure reorder and free machines and their `Ui` glue);
//! - `ui` (the per-window frame: ids, retained state, layered hits, overlays, focus, semantic
//!   nodes, `Response`, `FrameOutput`), `access` (the semantic tree and accessibility actions),
//!   `motion` (the closed-form motion system and the time type), `strings` (the string table);
//! - `backdrop` (the material seam: `GlassLayer`, `BackdropKind`, `Material` and the `Backdrop`
//!   capability);
//! - `window` (edge resize, double click, caption buttons);
//! - `tray` (the tray icon rasterised on the CPU, where its flyout and menu go, the click rules);
//! - `testing` (fakes of the seams for tests: the display-list recorder, a fake text system
//!   and its contract suite, a scripted input source, a null material and its contract suite).
//!
//! Values in the dark preset are copied from the house look's `DarkTheme.axaml` and `App.axaml`, so
//! apps styled after it match.

#![forbid(unsafe_code)]

pub mod access;
pub mod acrylic;
pub mod backdrop;
pub mod chrome;
pub mod code_doc;
pub mod color;
pub mod drag;
pub mod icon;
pub mod image;
pub mod input;
pub mod json;
pub mod layout;
pub mod motion;
pub mod paint;
pub mod scene;
pub mod shortcuts;
pub mod strings;
pub mod testing;
pub mod text;
pub mod text_edit;
pub mod theme;
pub mod tray;
pub mod ui;
pub mod widgets;
pub mod window;

pub use backdrop::{Backdrop, BackdropKind, GlassLayer};
pub use theme::lunegit_dark;

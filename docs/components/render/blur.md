---
id: render.blur
title: Backdrop blur pass
area: render
status: in-progress
consumers: luneterm
source: crates/lune-ui-wgpu/src/blur.rs
owner: orch/toolkit-v1
summary: Backdrop blur pass with its WGSL shader
---

## What it is

`Blur` (public: `scene_view`, `dump`; the rest is crate-private, driven by `FrameRenderer`). It reads the core `BackdropRequest` that the painter records. The private uniform struct is `BackdropUniform`, so no name clashes with the `Backdrop` trait in `lune-ui-core`.

## Needs

- Backdrop blur (R-render-2).

## Existing code

`crates/lune-ui-wgpu/src/blur.rs:190` (`Blur`), `:315` (`scene_view`), `:383` (`dump`), `:20` (`BackdropUniform`); 405 lines; `blur.wgsl` 95 lines. No tests of its own; covered by the gallery's blur scenes.

## Open questions

- None: the old name clash is gone (the struct is private `BackdropUniform`; the trait is `lune-ui-core`'s `Backdrop`, implemented by `WindowBackdrop` and `NullMaterial`).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

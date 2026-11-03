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

`Blur` (new, prepare, scene_view, run, blit_scene, set_backdrop, draw_backdrop, dump) and a `Backdrop` struct. The struct shares a name with the `Backdrop` trait in `lib.rs`.

## Needs

- Backdrop blur (R-render-2).

## Existing code

`crates/lune-ui-wgpu/src/blur.rs:50` (`Blur`), `:29` (`Backdrop` struct); 377 lines; `blur.wgsl` 95 lines. No tests.

## Open questions

- Two items named `Backdrop` (struct here, trait in `lib.rs:62`); whether to rename on extraction is for the planner. Whether any type implements the trait is unverified.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

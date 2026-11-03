---
id: render.gpu
title: wgpu instance, surface and pipelines
area: render
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/gpu.rs
owner: —
summary: wgpu instance, surface, pipelines and the noise texture
---

## What it is

`Gpu` (resize, acquire, present, upload, draw_below_text, draw_above_text), `Globals`, `Shape`, `StrokeVertex`, `LayerGeometry`, `Uploaded`, `instance`. The noise texture decodes `assets/acrylic-noise.png` through `include_bytes!`.

## Needs

- wgpu renderer for the painter's layers (R-render-1).
- Bundled noise texture with licence (R-render-5, R-ext-4).

## Existing code

`crates/lune-ui/src/gpu.rs:40` (`Gpu`), `:436` (noise texture); 481 lines; shaders `src/shader.wgsl` (161 lines). No tests.

## Open questions

- How the `.wgsl` files are compiled in was not checked.
- Provenance of `assets/acrylic-noise.png` beyond the comment at `gpu.rs:431` is unverified.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

---
id: render.gpu
title: Shared GPU
area: render
status: in-progress
consumers: luneterm
source: crates/lune-ui-wgpu/src/gpu.rs
owner: orch/toolkit-v1
summary: One shared Gpu (device, queue, noise texture, per-format pipelines and glyph atlas) for any number of targets
---

## What it is

`Gpu` (`new`, `upload`, `end_frame`), `Globals`, `Uploaded`, `instance`. One `Gpu` serves every target: it owns the device, the queue, the noise texture and, per colour format, the pipelines and the glyph atlas; each target (`render.frame`, `render.surface`) owns everything a draw writes. `Gpu::new` needs no window (headless works); `Gpu::end_frame` ages the text cache once per app frame. `Shape`, `StrokeVertex` and `LayerGeometry` are core plain data (`core.scene`); the adapter copies them into private Pod structs. The noise texture decodes `assets/acrylic-noise.png` through `include_bytes!`.

## Needs

- wgpu renderer for the painter's layers (R-render-1).
- Bundled noise texture with licence (R-render-5, R-ext-4).

## Existing code

`crates/lune-ui-wgpu/src/gpu.rs:90` (`Gpu`), `:143` (`Gpu::new`), `:331` (`end_frame`), `:340` (`upload`), `:131` (`instance`), `:435` (noise texture); 544 lines; shaders `src/shader.wgsl` (161 lines). Two-target contract tests in `crates/lune-ui-wgpu/tests/targets.rs`.

## Open questions

- How the `.wgsl` files are compiled in was not checked.
- Provenance of `assets/acrylic-noise.png` beyond the comment at `gpu.rs:434` is unverified.

## Notes for the planner

The one-GPU, many-targets shape landed in M2 steps 3 and 4 (DECISIONS D28, D30).

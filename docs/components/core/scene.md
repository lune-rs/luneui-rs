---
id: core.scene
title: Display list
area: core
status: in-progress
consumers: any
source: crates/lune-ui-core/src/scene.rs
owner: orch/toolkit-v1
summary: Plain-data Scene the painter builds and a renderer consumes; the draw-order contract
---

## What it is

`Scene` (four `Layer`s and an optional `BackdropRequest`), `LayerGeometry` (acrylic, under, over shapes and strokes), `Shape`, `StrokeVertex`, `TextItem`, `CustomItem`, `LayerId`, `LAYER_COUNT`, `SCENE_LAYERS` and `FrameParams`. Std only, in physical pixels; the records mirror the GPU's field by field and the wgpu adapter copies them into its own Pod structs. This is the renderer seam: the data, not a trait. The draw-order contract is in the module doc: layers back to front; in a layer acrylic, under shapes, custom items, text, over shapes, strokes; layers under `SCENE_LAYERS` render offscreen, which a backdrop request blurs.

## Needs

- A display list a renderer can consume without the painter (R-render-1, R-nfr-8).
- A fake renderer for tests of painting code (R-nfr-8).

## Existing code

`crates/lune-ui-core/src/scene.rs:99` (`Scene`), `:31` (`Shape`), `:55` (`LayerGeometry`), `:74` (`CustomItem`), `:91` (`BackdropRequest`), `:107` (`FrameParams`); 116 lines. Consumers: `crates/lune-ui-wgpu/src/gpu.rs:340` (`upload`), `crates/lune-ui-core/src/testing/recording.rs:57` (the fake).

## Open questions

- A second renderer (a software one, say) is the real test of the data shape; none exists. No renderer trait on purpose (DECISIONS D25).

## Notes for the planner

Added in M2 step 2. Zero pixel delta against M1 on the 15 gallery scenes. `core.testing` holds the fake.

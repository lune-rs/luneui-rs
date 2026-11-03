---
id: render.custom-draw
title: App-owned drawing hook
area: render
status: in-progress
consumers: luneterm
source: crates/lune-ui-wgpu/src/custom.rs
owner: orch/toolkit-v1
summary: CustomDraw trait: app-owned GPU drawing inside a layer
---

## What it is

`CustomFrame`, the `CustomDraw` trait and `CustomItem`: the app draws its own GPU content inside a layer. Luneterm's terminal grid renderer builds on it.

## Needs

- App-specific surfaces draw through this hook, never inside the toolkit (R-render-3, D8).

## Existing code

`crates/lune-ui-wgpu/src/custom.rs:13` (`CustomFrame`), `:24` (`CustomDraw`), `:9` (`CustomItem`, re-exported from `crates/lune-ui-core/src/scene.rs:74`); 33 lines. Used by lune-termview (`src/render.rs:12`) and the glass-ui spike (`src/main.rs:29`).

## Open questions

- The hook lives in `lune-ui-wgpu` and its signature exposes wgpu types by design (it is GPU drawing); only the recorded `CustomItem` (slot, rect, clip) is core plain data.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it. See `luneterm.terminal-grid` for the consumer.

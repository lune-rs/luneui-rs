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

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-wgpu/src/custom.rs:1-33` (module doc, `CustomFrame`, `CustomDraw`); `crates/lune-ui-wgpu/src/frame.rs:119` (`draw_with`); `crates/lune-ui-core/src/paint.rs:254` (`custom`) and its tests `paint.rs` (`custom_records_physical_clip_on_base`, `custom_is_cut_by_the_current_clip`, `custom_outside_the_window_records_nothing`, `custom_lands_on_the_current_layer`).
Conflicts: none.

### Anatomy

- Recorded by `Painter::custom(slot, rect)` as a `CustomItem` [R crates/lune-ui-core/src/paint.rs:254], re-exported from the scene module [R crates/lune-ui-wgpu/src/custom.rs:9].
- Drawn after the layer's acrylic, under shapes and before the layer's text, with the scissor set to the item's clip [R crates/lune-ui-wgpu/src/custom.rs:4-6].
- App hook: the `CustomDraw` trait with a `CustomFrame` argument [R crates/lune-ui-wgpu/src/custom.rs:13,24].

### States

n/a — the hook draws what the app draws each frame.

### Transitions

n/a — no motion; the app may animate inside its own hook.

### Input and focus

n/a — the hook takes no input.

### Retained state

- App-owned: everything the hook draws from (the app's GPU state).
- Transient: none in the toolkit.

### Accessibility

n/a — the app provides the node for what it draws.

### Strings

n/a — no text in the hook.

### Gallery scenes

- `custom-draw` (existing): the custom item on its layer, settled. [R apps/gallery/src/scenes.rs:109-115]

### Painter primitives

- Existing: custom. [R crates/lune-ui-core/src/paint.rs:254]
- New: none.

### Acceptance

- Existing tests (by path, `crates/lune-ui-core/src/paint.rs`): `custom_records_physical_clip_on_base`, `custom_is_cut_by_the_current_clip`, `custom_outside_the_window_records_nothing`, `custom_lands_on_the_current_layer`.
- Done standard still needs: a pixel gate on `custom-draw` at zero delta, and the Luneterm terminal grid as the consumer's cite (`render/custom-draw.md` Notes).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.frame, core.scene.

## Open questions

- The hook lives in `lune-ui-wgpu` and its signature exposes wgpu types by design (it is GPU drawing); only the recorded `CustomItem` (slot, rect, clip) is core plain data.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it. See `luneterm.terminal-grid` for the consumer.

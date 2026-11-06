---
id: render.custom-draw
title: App-owned drawing hook
area: render
status: done
consumers: luneterm
source: crates/lune-ui-wgpu/src/custom.rs
owner: —
summary: CustomDraw trait: app-owned GPU drawing inside a layer
---

## What it is

`CustomFrame`, the `CustomDraw` trait and `CustomItem`: the app draws its own GPU content inside a layer. Luneterm's terminal grid renderer builds on it.

## Needs

- App-specific surfaces draw through this hook, never inside the toolkit (R-render-3, D8).

## Existing code

`crates/lune-ui-wgpu/src/custom.rs:13` (`CustomFrame`), `:24` (`CustomDraw`), `:9` (`CustomItem`, re-exported from `crates/lune-ui-core/src/scene.rs:109`); 33 lines. Used by lune-termview (`src/render.rs:12`) and the glass-ui spike (`src/main.rs:29`).

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

- Recording (`crates/lune-ui-core/src/paint.rs`, module `tests`): `custom_records_physical_clip_on_base`, `custom_is_cut_by_the_current_clip`, `custom_outside_the_window_records_nothing`, `custom_lands_on_the_current_layer`; the draw order (customs after the layer's fills, before its images and text) is `crates/lune-ui-core/src/testing/recording.rs::records_the_documented_draw_order`.
- The hook through the real renderer: `crates/lune-ui-wgpu/tests/custom_draw.rs::every_slot_is_prepared_once_and_rendered_once_per_item` (every supplied implementation is prepared once, one with no item too; `render` runs once per item of its slot; both get the frame's size, scale and format). The scissor and the pixels the gallery's `Fill` draws are the pixel gate on `custom-draw` at zero delta (the orchestrator's gate; M11's pre-audit run held 413 of 413).
- The consumer: Luneterm's `lune-termview` imports `lune_ui::custom::{CustomDraw, CustomFrame}` and checks against this repo with no source change (`docs/consuming.md`, "lune-termview"; `luneterm.terminal-grid`).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.frame, core.scene.
- Done (D47, M11): tests `crates/lune-ui-wgpu/tests/custom_draw.rs::every_slot_is_prepared_once_and_rendered_once_per_item, crates/lune-ui-core/src/paint.rs::custom_records_physical_clip_on_base, crates/lune-ui-core/src/paint.rs::custom_is_cut_by_the_current_clip, crates/lune-ui-core/src/paint.rs::custom_lands_on_the_current_layer, crates/lune-ui-core/src/testing/recording.rs::records_the_documented_draw_order`; page n/a (an app hook; the scene `custom-draw` draws the gallery's `Fill` through it); unverified here: none (the terminal grid itself is Luneterm's).

## Open questions

- The hook lives in `lune-ui-wgpu` and its signature exposes wgpu types by design (it is GPU drawing); only the recorded `CustomItem` (slot, rect, clip) is core plain data.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it. See `luneterm.terminal-grid` for the consumer.

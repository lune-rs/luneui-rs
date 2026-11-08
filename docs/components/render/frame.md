---
id: render.frame
title: Frame pass sequence
area: render
status: done
consumers: luneterm
source: crates/lune-ui-wgpu/src/frame.rs
owner: —
summary: FrameRenderer: one target's scene, blur, target pass sequence over the core Scene
---

## What it is

`FrameRenderer` (`new`, `format`, `blur`, `draw`, `draw_with`) draws a core `Scene` into one target; `FrameParams` is a core type re-exported here; its `noise_opacity` and `blur: BlurLimits` (kernel cap in texels, default 48, at most 128; divisor 2, 4 or 8, default 8) are values of the frame, and a bad one draws as the default (D264). Each renderer owns what a draw writes (globals, viewport, text renderers, blur textures), so several targets share one `Gpu` without ordering hazards.

## Needs

- Scene, blur and target pass sequence (R-render-1).

## Existing code

`crates/lune-ui-wgpu/src/frame.rs:22` (`FrameRenderer`), `:47` (`new`), `:104` (`draw`), `:119` (`draw_with`), `FrameParams` re-exported from `crates/lune-ui-core/src/scene.rs:197`; 302 lines. Tests: `crates/lune-ui-wgpu/tests/targets.rs`; the fake is `testing::RecordingRenderer` (`core.testing`).

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-wgpu/src/frame.rs:1-9` (module doc), `:19,22,47,92,97,104,119` (`FrameParams`, `FrameRenderer`, `new`, `format`, `blur`, `draw`, `draw_with`); `crates/lune-ui-wgpu/tests/targets.rs` (test names).
Conflicts: none.

### Anatomy

- One renderer per target: uniforms, viewport, text renderers and blur textures [R crates/lune-ui-wgpu/src/frame.rs:1-2].
- The pass sequence: scene, blur, target [R crates/lune-ui-wgpu/src/frame.rs:1-2].
- Renders into any target view at an explicit size and scale: a window surface or an offscreen texture [R crates/lune-ui-wgpu/src/frame.rs:5-7].
- Shares one `Gpu` with other renderers; owns its draw writes, except the glyph atlas per format and the rasteriser cache [R crates/lune-ui-wgpu/src/frame.rs:7-9].

### States

n/a — a frame is drawn or not.

### Transitions

n/a — no motion.

### Input and focus

n/a — no input.

### Retained state

- Persistent: the renderer's targets and buffers, per format and target.
- Transient: none per frame beyond the scene.
- App-owned: the `Scene` and `FrameParams` passed to `draw`.

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

- Every gallery scene draws through this renderer: `controls` (existing) [R apps/gallery/src/scenes.rs:82-93].
- `ui-controls`: the M4 controls composition (`controls_ui`) at scale 1.5, settled, dumped at 1560 by 990 physical px: a scale that is not a whole number of device pixels per logical one (R-nfr-6, M11) [R apps/gallery/src/pages/ui.rs].

### Painter primitives

n/a — the renderer consumes the scene; it adds no primitive.

### Acceptance

- Tests (`crates/lune-ui-wgpu/tests/targets.rs`): `two_targets_on_one_gpu_match_each_alone`, `second_format_on_one_gpu_matches_a_fresh_gpu`, `one_renderer_across_two_sizes_matches_a_fresh_renderer_at_each`. The hook: `custom_draw.rs::every_slot_is_prepared_once_and_rendered_once_per_item`. A draw on a lost device never reports success (wgpu may panic, so the doc says to ask `Gpu::is_lost` first) and a rebuilt renderer on a new `Gpu` draws: `device_lost.rs::a_destroyed_device_is_lost_a_draw_on_it_never_succeeds_and_a_rebuild_draws`. The fake is `RecordingRenderer` (`core.testing`), the draw order `records_the_documented_draw_order`.
- The 1.5x scene: `apps/gallery/src/pages/ui.rs::the_controls_scene_at_1_5_is_named_sized_and_runs` (name `ui-controls@1.5x`, 1560 by 990 physical px, draws the controls); the gate dumps it through this renderer.
- The pixel gate through this renderer: every scene identical against 55cfaaf (the orchestrator's gate; M11's pre-audit run held 413 of 413).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.gpu, render.surface, render.blur, core.scene.
- Done (D47, M11): tests `crates/lune-ui-wgpu/tests/targets.rs::two_targets_on_one_gpu_match_each_alone, crates/lune-ui-wgpu/tests/targets.rs::second_format_on_one_gpu_matches_a_fresh_gpu, crates/lune-ui-wgpu/tests/targets.rs::one_renderer_across_two_sizes_matches_a_fresh_renderer_at_each, crates/lune-ui-wgpu/tests/custom_draw.rs::every_slot_is_prepared_once_and_rendered_once_per_item, crates/lune-ui-wgpu/tests/device_lost.rs::a_destroyed_device_is_lost_a_draw_on_it_never_succeeds_and_a_rebuild_draws, apps/gallery/src/pages/ui.rs::the_controls_scene_at_1_5_is_named_sized_and_runs`; page `ui` (the scene `ui-controls@1.5x`; every page draws through this renderer); unverified here: none.

## Open questions

- Pass extensibility (Luneterm's architecture names a `Pass` trait in lune-termview, not here).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

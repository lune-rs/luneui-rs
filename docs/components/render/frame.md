---
id: render.frame
title: Frame pass sequence
area: render
status: in-progress
consumers: luneterm
source: crates/lune-ui-wgpu/src/frame.rs
owner: orch/toolkit-v1
summary: FrameRenderer: one target's scene, blur, target pass sequence over the core Scene
---

## What it is

`FrameRenderer` (`new`, `format`, `blur`, `draw`, `draw_with`) draws a core `Scene` into one target; `FrameParams` is a core type re-exported here. Each renderer owns what a draw writes (globals, viewport, text renderers, blur textures), so several targets share one `Gpu` without ordering hazards.

## Needs

- Scene, blur and target pass sequence (R-render-1).

## Existing code

`crates/lune-ui-wgpu/src/frame.rs:19` (`FrameParams`, re-exported from `crates/lune-ui-core/src/scene.rs:107`), `:22` (`FrameRenderer`), `:47` (`new`), `:104` (`draw`), `:119` (`draw_with`); 302 lines. Tests: `crates/lune-ui-wgpu/tests/targets.rs`; the fake is `testing::RecordingRenderer` (`core.testing`).

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

- Existing tests (by path, `crates/lune-ui-wgpu/tests/targets.rs`): `two_targets_on_one_gpu_match_each_alone`, `second_format_on_one_gpu_matches_a_fresh_gpu`.
- Done standard still needs: the pixel gate at zero delta on the 14 scenes through this renderer.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: render.gpu, render.surface, render.blur, core.scene.

## Open questions

- Pass extensibility (Luneterm's architecture names a `Pass` trait in lune-termview, not here).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

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

## Open questions

- Pass extensibility (Luneterm's architecture names a `Pass` trait in lune-termview, not here).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

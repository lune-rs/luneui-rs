---
id: render.frame
title: Frame pass sequence
area: render
status: in-progress
consumers: luneterm
source: crates/lune-ui/src/frame.rs
owner: orch/toolkit-v1
summary: FrameRenderer: scene, blur, target pass sequence
---

## What it is

`FrameParams` and `FrameRenderer` (new, blur, draw, draw_with, end_frame).

## Needs

- Scene, blur and target pass sequence (R-render-1).

## Existing code

`crates/lune-ui/src/frame.rs:22` (`FrameParams`), `:33` (`FrameRenderer`); 273 lines. No tests.

## Open questions

- Pass extensibility (Luneterm's architecture names a `Pass` trait in lune-termview, not here).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

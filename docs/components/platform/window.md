---
id: platform.window
title: Glass window helpers
area: platform
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/window.rs
owner: —
summary: Glass window attributes, edge resize, double-click detection and caption buttons
---

## What it is

`glass_attributes`, `EDGE_BORDER`, `edge_at`, `DoubleClick` and `caption_buttons`.

## Needs

- Custom caption, edge resize and double-click handling (R-platform-4).

## Existing code

`crates/lune-ui/src/window.rs:52` (`glass_attributes`), `:71` (`edge_at`), `:93` (`DoubleClick`), `:115` (`caption_buttons`); 144 lines. Integration test `crates/lune-ui/tests/window.rs` (edges, 6 px border, double-click gap under 400 ms).

## Open questions

- It uses winit types (the integration test imports `winit::window::ResizeDirection`); whether that stays is a seam question.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

---
id: render.dump
title: Offscreen frame dump
area: render
status: in-progress
consumers: luneterm
source: crates/lune-ui/src/dump.rs
owner: orch/toolkit-v1
summary: Offscreen target, GPU readback and PNG writer for frame goldens
---

## What it is

`target`, `read_rgba` and `write_png`: render offscreen, read back, write a PNG, with no window or screen capture.

## Needs

- Frame dump for goldens and the pixel gate (R-render-4, R-ext-2).

## Existing code

`crates/lune-ui/src/dump.rs:9` (`target`), `:31` (`read_rgba`), `:102` (`write_png`); 146 lines; tests at `:113`.

## Open questions

- The scene list and the compare tool live in the glass-ui spike, not in lune-ui; where they go is the extraction plan's call.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

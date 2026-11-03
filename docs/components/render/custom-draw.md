---
id: render.custom-draw
title: App-owned drawing hook
area: render
status: in-progress
consumers: luneterm
source: crates/lune-ui/src/custom.rs
owner: orch/toolkit-v1
summary: CustomDraw trait: app-owned GPU drawing inside a layer
---

## What it is

`CustomFrame`, the `CustomDraw` trait and `CustomItem`: the app draws its own GPU content inside a layer. Luneterm's terminal grid renderer builds on it.

## Needs

- App-specific surfaces draw through this hook, never inside the toolkit (R-render-3, D8).

## Existing code

`crates/lune-ui/src/custom.rs:11` (`CustomFrame`), `:22` (`CustomDraw`), `:35` (`CustomItem`); 40 lines. Used by lune-termview (`src/render.rs:12`) and the glass-ui spike (`src/main.rs:29`).

## Open questions

- Whether the hook's signature exposes wgpu types to apps is a seam question for the extraction plan.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it. See `luneterm.terminal-grid` for the consumer.

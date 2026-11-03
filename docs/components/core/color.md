---
id: core.color
title: Colour helpers
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui/src/color.rs
owner: orch/toolkit-v1
summary: Straight-alpha RGBA values with premultiply, blend-over and alpha helpers
---

## What it is

`Rgba` as straight-alpha RGBA plus `premul`, `over` and `with_alpha` helpers.

## Needs

- Straight-alpha RGBA, premultiply and blend-over, used by theme, painter and acrylic math (R-core-3).

## Existing code

`crates/lune-ui/src/color.rs:6` (`premul`), `:16` (`over`), `:26` (`with_alpha`); 28 lines.

## Open questions

- Whether a colour-space or float-colour type is wanted later is not decided.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

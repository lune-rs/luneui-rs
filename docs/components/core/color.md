---
id: core.color
title: Colour helpers
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/color.rs
owner: orch/toolkit-v1
summary: Straight-alpha RGBA values with premultiply, blend-over and alpha helpers
---

## What it is

`Rgba` as straight-alpha RGBA plus `premul`, `over` and `with_alpha` helpers.

## Needs

- Straight-alpha RGBA, premultiply and blend-over, used by theme, painter and acrylic math (R-core-3).

## Existing code

`crates/lune-ui-core/src/color.rs:6` (`premul`), `:16` (`over`), `:26` (`with_alpha`); 28 lines.

## Spec

### Evidence

Pinned: this repo fa2958b. Read: `crates/lune-ui-core/src/color.rs:1-28` (type and three functions).
Conflicts: none.

### Anatomy

- `Rgba = [u8; 4]` [R crates/lune-ui-core/src/color.rs:4].
- `premul(c)` converts an `Rgba` to premultiplied `[f32; 4]` [R crates/lune-ui-core/src/color.rs:6].
- `over(top, below)` composes two premultiplied colours [R crates/lune-ui-core/src/color.rs:16].
- `with_alpha(c, k)` scales alpha by `k` [R crates/lune-ui-core/src/color.rs:26].

### States

n/a — no state; colours are values the caller passes.

### Transitions

n/a — no motion.

### Input and focus

n/a — no input.

### Retained state

n/a — pure functions.

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

n/a — no scene draws a colour function by itself; every scene uses `premul` through the painter (`controls`, [R apps/gallery/src/scenes.rs:82-93]).

### Painter primitives

- Existing: none.
- Note: The painter takes the premultiplied values.
- New: none.

### Acceptance

- Existing tests: none in `crates/lune-ui-core/src/color.rs`.
- Done standard still needs: known-answer tests for `premul`, `over` and `with_alpha` at the alpha extremes (0, 128, 255).

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: none.

## Open questions

- Whether a colour-space or float-colour type is wanted later is not decided.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

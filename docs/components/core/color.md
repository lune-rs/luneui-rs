---
id: core.color
title: Colour helpers
area: core
status: done
consumers: luneterm
source: crates/lune-ui-core/src/color.rs
owner: —
summary: Straight-alpha RGBA values with premultiply, blend-over and alpha helpers
---

## What it is

`Rgba` as straight-alpha RGBA plus `premul`, `over` and `with_alpha` helpers.

## Needs

- Straight-alpha RGBA, premultiply and blend-over, used by theme, painter and acrylic math (R-core-3).

## Existing code

`crates/lune-ui-core/src/color.rs:7` (`premul`), `:18` (`over`), `:29` (`with_alpha`); 31 lines.

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

- Known answers worked out by hand from the formulas and written as literals (`crates/lune-ui-core/tests/color.rs`): `premul_at_alpha_zero_is_all_zero`, `premul_at_alpha_255_is_straight_over_255`, `premul_at_alpha_128_scales_colour_by_128_over_255`; `over_with_transparent_top_is_the_layer_below`, `over_with_opaque_top_is_the_top`, `over_with_half_top_keeps_half_of_below`, `over_at_alpha_128_of_a_premultiplied_top`; `with_alpha_scales_only_alpha`, `with_alpha_at_alpha_0_128_255_rounds_and_saturates`.

### Build

- Milestone: M11 (done standard; code exists).
- Depends on: none.
- Done (D47, M11): tests `crates/lune-ui-core/tests/color.rs::premul_at_alpha_zero_is_all_zero, crates/lune-ui-core/tests/color.rs::premul_at_alpha_128_scales_colour_by_128_over_255, crates/lune-ui-core/tests/color.rs::over_at_alpha_128_of_a_premultiplied_top, crates/lune-ui-core/tests/color.rs::with_alpha_at_alpha_0_128_255_rounds_and_saturates`; page n/a (pure functions; every scene draws through `premul`); unverified here: none.

## Open questions

- Whether a colour-space or float-colour type is wanted later is not decided.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

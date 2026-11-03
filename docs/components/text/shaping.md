---
id: text.shaping
title: Shaped-text cache
area: text
status: in-progress
consumers: luneterm
source: crates/lune-ui/src/text.rs
owner: orch/toolkit-v1
summary: Shaped-text cache over glyphon keyed by content; the app registers its fonts
---

## What it is

`TextCache` with `new`, `set_scale`, `prepare`, `split` and `end_frame`; `Font`, `Style` and `Spec` describe the text to shape.

## Needs

- Shape and measure text with app-supplied fonts (R-text-1).
- Cache by content; scale setting (R-text-2, R-nfr-6).

## Existing code

`crates/lune-ui/src/text.rs:110` (`TextCache`), `:72` (`Spec`), `:123` (`TextCache::new`); 224 lines. No tests.

## Open questions

- Swap point: shaping behind a trait without glyphon types in its signatures is the planned seam, not true today.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

---
id: core.acrylic
title: Acrylic colour math and noise opacity
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui/src/acrylic.rs
owner: orch/toolkit-v1
summary: Avalonia acrylic colour math with tests, and the noise opacity
---

## What it is

`AcrylicColors` and `acrylic_colors` (Avalonia's acrylic formulas), `premultiplied`, and `NOISE_OPACITY`.

## Needs

- Avalonia acrylic colour math, tested (R-core-5).
- The Avalonia formulas are bundled third-party logic and need a licence row (R-ext-4).

## Existing code

`crates/lune-ui/src/acrylic.rs:16` (`NOISE_OPACITY`), `:20` (`AcrylicColors`), `:30` (`acrylic_colors`), `:52` (`premultiplied`); tests at `:99`.

## Open questions

- Licence text for the Avalonia formulas: Luneterm has `licenses/avalonia-LICENSE`; this repo has none yet.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

---
id: core.tab-outline
title: Tab outline geometry
area: core
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/chrome.rs
owner: —
summary: Tab-chrome outline loop geometry
---

## What it is

`outline` builds the tab chrome loop from `TabSpan`s, with `CARD_RADIUS`, `SEAM_RADIUS`, `FLARE_RADIUS` and `STROKE`.

## Needs

- Tab outline geometry (R-core-7).

## Existing code

`crates/lune-ui/src/chrome.rs:6-9` (constants), `:14` (`TabSpan`), `:20` (`outline`); 226 lines; tests at `:172`.

## Open questions

- The name `chrome` and its tab-bar assumption read as app-shaped; whether it is generic is for the planner.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

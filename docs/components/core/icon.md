---
id: core.icon
title: Codicon glyph constants
area: core
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/icon.rs
owner: —
summary: Codicon glyph constants and a glyph helper; the app supplies the icon font
---

## What it is

Constants such as `ADD`, `GEAR`, `ELLIPSIS`, `CHROME_CLOSE`, `CHECK`, `CHROME_MINIMIZE` and `glyph`.

## Needs

- Icon glyphs as constants; the font itself comes from the app (R-core-6, R-text-1).

## Existing code

`crates/lune-ui/src/icon.rs:7-19` (constants), `:22` (`glyph`); 24 lines.

## Open questions

- The Codicon font licence is not in this repo's plans yet; whether the toolkit bundles the font is open (R-ext-4).

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

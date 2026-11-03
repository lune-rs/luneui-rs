---
id: core.theme
title: Theme tokens and Lunegit dark preset
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/theme.rs
owner: orch/toolkit-v1
summary: Theme token struct and the lunegit_dark preset with the WINDOW, PANEL and CARD glass layers
---

## What it is

`Theme` (tokens the widgets read) and the `lunegit_dark` module: `THEME`, `TINT`, `WINDOW`/`PANEL`/`CARD` layers, `INACTIVE_FALLBACK`, `CARD_CORNER_RADIUS`, `TAB_HEADER_HEIGHT`, `BODY_PADDING`.

## Needs

- Look is a `Theme` value; further looks are values, not forks (R-core-4).
- Boundary rule 3 allows the word `lunegit` only in the theme preset module (reserved slot in `scripts/boundaries.allow`; no live entry yet).

## Existing code

`crates/lune-ui-core/src/theme.rs:11` (`Theme`), `:47` (`DARK`, the neutral alias), `:50` (`lunegit_dark`), `:55` (`THEME`), `:88` (`TINT`), `:91`/`:98`/`:105` (layers); 118 lines. No tests.

## Open questions

- The theme has no JSON loader; see `infra.theme-hot-reload`.
- `TAB_HEADER_HEIGHT` and `BODY_PADDING` read as terminal-app values; whether they belong in the toolkit theme is unchecked.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

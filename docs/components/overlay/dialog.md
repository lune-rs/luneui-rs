---
id: overlay.dialog
title: Modal dialog
area: overlay
status: in-luneterm
consumers: luneterm
source: crates/lune-ui/src/widgets/dialog.rs
owner: —
summary: Modal dialog card: dim backdrop, 448 px acrylic card, darker footer, app-supplied body
---

## What it is

`dialog` with `DialogSpec`, `DialogGlass` and `DialogHits`: dim backdrop, 448 px acrylic card, 24 px padding, darker footer; a `body` closure draws the app content.

## Needs

- Modal dialog (R-overlay-1).

## Existing code

`crates/lune-ui/src/widgets/dialog.rs:42` (`dialog`), `:10` (`DialogSpec`), `:22` (`DialogGlass`), `:32` (`DialogHits`); 94 lines. No tests.

## Open questions

- Whether the card width is configurable is unverified; Lunegit has many dialogs under `Views/Dialogs/` and whether one width suits them is unchecked.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

---
id: text.code-block
title: Code block
area: text
status: planned
consumers: wane
source: —
owner: —
summary: Read-only mono block with a copy button
---

## What it is

Read-only mono block with a copy button. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Select and copy text; the copy button copies the whole block to the clipboard (platform need). Used for the Claude Code env block and the data folder listing.

## Needs

- Wane — Variants: none; token classes: key (secondary), op (quaternary), value (primary); no syntax colours (R-text-3)
- Wane — States: none; copy feedback (a short checkmark or toast) is not mocked. (R-text-3)
- Wane — Sizes: `--surface-input` fill, 1 px `--border-subtle`, radius 6; pre padding 10 12; RznMono 13/20, no wrap, horizontal scroll; copy icon button at top 6, right 6. (R-text-3)
- Wane — Behaviour: Select and copy text; the copy button copies the whole block to the clipboard (platform need). Used for the Claude Code env block and the data folder listing. (R-text-3)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Wane `docs/ui/mocks/css/components.css` (text.code-block); no lune-ui source.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: Long lines scroll horizontally with `widget.scrollbar`; selectable mono text needs the text layer's selection support.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `text.code-block`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, routes, settings.

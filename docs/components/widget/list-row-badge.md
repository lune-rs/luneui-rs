---
id: widget.list-row-badge
title: List rows with status badges
area: widget
status: stub
consumers: lunegit, wane
source: Lunegit: src/Lunegit.Desktop/App.axaml
owner: —
summary: List item rows and status badge borders styled in App.axaml (rough stub)
---

## What it is

Rough stub — not analysed. The style survey counted about 25 `ListBoxItem` styles and several `Border` badge styles in `App.axaml`.

## Needs

- Row and badge styles from a theme (R-core-4, R-widget-2).
- Wane — Variants: none; the contents vary (dot, tag, badge in the head) (R-widget-2)
- Wane — States: rest (transparent); hover (`--surface-hover`); selected (`--surface-selected`, bright name; never a border or accent bar). (R-widget-2)
- Wane — Sizes: Padding 8 12, radius 6, rows 2 apart, gap 4; name 13/20 Medium; value RznMono 13; sub-line RznMono 11/16 tertiary; meta 11/16 tertiary. (R-widget-2)
- Wane — Behaviour: Click selects the row and swaps the detail pane; Up and Down move the selection (requirement). Used on Sessions, Upstreams, Overview (live sessions) and Pricing. (R-widget-2)

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/App.axaml`
- `src/Lunegit.Desktop/ViewModels/SidePanelRow.cs`

## Open questions

- Which badge kinds exist and whether they are one component.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

Wane (2026-11-04): lune-ui status for Wane's use: missing; Wane priority P0. Full spec: Wane `docs/ui/components.md`, section `widget.list-row-badge`; mock: Wane `docs/ui/mocks/components.html`.

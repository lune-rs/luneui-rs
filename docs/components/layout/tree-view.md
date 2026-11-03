---
id: layout.tree-view
title: Tree view and tree guides
area: layout
status: stub
consumers: lunegit, wane
source: Lunegit: src/Lunegit.Desktop/Controls/TreeGuideControl.cs
owner: —
summary: Tree rows with guide lines for branches, tags, stash and files (rough stub)
---

## What it is

Rough stub — not analysed. `TreeGuideControl` draws tree guide lines; `SidePanelRow` models rows for the branch tree, tags and stash. A `TreeView` selector was not found in the style survey.

## Needs

- Tree rows with guide lines (R-layout-3).
- Wane — Variants: child (`__row--child`, depth 1; indent 16 per depth through `--depth`); has-next continues the guide to the next sibling. (R-layout-3)
- Wane — States: rest; hover (`--surface-hover`). Not expandable or collapsible in the mocks. (R-layout-3)
- Wane — Sizes: Rows 28, radius 6, padding-left 8 + depth x 16; guide at x 16, 1 px `--border`; elbow 8 x 14 with radius 3.5 (`--radius-elbow`); parts: name 13, model RznMono 11 tertiary, tokens 11, cost RznMono 13 right-aligned (min 72). (R-layout-3)
- Wane — Behaviour: Static in the mocks. Expand and collapse, selection and keyboard navigation come with the catalogue stub's real requirements. (R-layout-3)

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Controls/TreeGuideControl.cs`
- `src/Lunegit.Desktop/ViewModels/SidePanelRow.cs`
- `src/Lunegit.Desktop/Views/SidePanels/BranchesPanelView.axaml`

## Open questions

- Expand and collapse behaviour and keyboard navigation are unverified.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

Wane (2026-11-04): lune-ui status for Wane's use: missing; Wane priority P1. Native note: Guide lines are painter strokes; no CSS-only effect. Full spec: Wane `docs/ui/components.md`, section `layout.tree-view`; mock: Wane `docs/ui/mocks/components.html`.

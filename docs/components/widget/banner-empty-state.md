---
id: widget.banner-empty-state
title: Banners, empty states and loading gates
area: widget
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Controls/IndexingBanner.axaml
owner: —
summary: Indexing banner, empty-repository view, loading view and gate views (rough stub)
---

## What it is

Rough stub — not analysed. An indexing banner plus empty, loading and gate full-page views; `empty-state-action` is a button style.

## Needs

- Simple composed views from existing widgets (R-widget-1, R-layout-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Controls/IndexingBanner.axaml`
- `src/Lunegit.Desktop/Views/EmptyRepositoryView.axaml`
- `src/Lunegit.Desktop/Views/StartupRepositoryLoadingView.axaml`
- `src/Lunegit.Desktop/Views/EulaGateView.axaml`
- `src/Lunegit.Desktop/Views/GitRequiredGateView.axaml`

## Open questions

- Likely app code, not toolkit; confirm.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

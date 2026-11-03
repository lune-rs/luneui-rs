---
id: lunegit.diff-view
title: Diff view (unified, split, hunk actions)
area: lunegit
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/DiffView.axaml
owner: —
summary: Unified and split diff views with hunk actions (rough stub)
---

## What it is

Rough stub — not analysed. Diff, unified and split views, plus a line background highlighter in the AvaloniaEdit rendering layer. The `hunk-action-btn` button style exists. The split view uses drag and drop.

## Needs

- Text view with per-line backgrounds (R-text-3, R-data-1).
- Keyboard bindings (R-infra-2).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/DiffView.axaml`
- `src/Lunegit.Desktop/Views/UnifiedDiffView.axaml`
- `src/Lunegit.Desktop/Views/SplitDiffView.axaml`
- `src/Lunegit.Desktop/Rendering/LineBackgroundHighlighter.cs`

## Open questions

- Is the diff a text view with overlays, or custom drawn?
- Syntax highlighting inside diffs: shared with `text.code-editor-view`?

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

---
id: text.markdown-preview
title: Markdown preview
area: text
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml
owner: —
summary: Markdown preview rendered from Markdig output (rough stub)
---

## What it is

Rough stub — not analysed. A preview view and an inline renderer; Lunegit uses Markdig for parsing.

## Needs

- Rich text layout (R-text-3, R-layout-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml`
- `src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs`

## Open questions

- Which Markdown features are rendered (tables, images, code blocks)?

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

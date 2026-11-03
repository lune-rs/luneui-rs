---
id: text.code-editor-view
title: Code viewer and editor with syntax highlighting
area: text
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Markdown/TextMateEditorSupport.cs
owner: —
summary: Code text view and editor with TextMate syntax highlighting (AvaloniaEdit equivalent) (rough stub)
---

## What it is

Rough stub — not analysed. Lunegit uses AvaloniaEdit (`TextEditor`) with `AvaloniaEdit.TextMate` in the diff, file, conflict and Markdown views.

## Needs

- Multi-line text view and editor with highlighting (R-text-3).
- Selection, caret, clipboard (R-text-5) and IME (R-text-4) if it edits.

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Markdown/TextMateEditorSupport.cs`
- `src/Lunegit.Desktop/Markdown/TextMateInlineHighlighter.cs`
- `src/Lunegit.Desktop/Views/FileViewerView.axaml`

## Open questions

- Read-only viewer or editor? Folding, search and line numbers?
- Syntax grammar source: TextMate grammars in a Rust engine, or something else.

## Notes for the planner

Likely the largest single piece; the planner should decide build, adopt a crate or keep it app-side.

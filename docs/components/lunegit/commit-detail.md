---
id: lunegit.commit-detail
title: Commit detail view
area: lunegit
status: external
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/CommitDetailView.axaml
owner: —
summary: Commit detail side panel (app composition): header, copyable fields, changed-files tree with filter; built from library parts
---

## What it is

Lunegit's commit detail panel: a 32 px glass header with a title and close button, a block of fields
(signed-commit notice, hash, author, date, message; a click copies a field), a "Changed Files" heading
with a filter box, and a virtualized list of changed files shown as a tree with guide lines (folder
and file rows 24 px high, a context menu "View file at this commit"). It is git-domain composition and
stays in Lunegit (D42). Every piece is a library part; the panel's content swap is Lunegit's.

## Needs

- Detail fields, lists and a diff area (R-layout-1, R-data-1): the diff area is `text.diff-view`, shown beside this panel, not inside it.

## Existing code

External: Lunegit `src/Lunegit.Desktop/Views/CommitDetailView.axaml` (294 lines) and `.axaml.cs`; `Views/CommitContentView.axaml` (64 lines) hosts the diff or the conflict resolver beside it.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/CommitDetailView.axaml:1-140,150-294, Views/CommitDetailView.axaml.cs:29-51,
Views/CommitContentView.axaml:1-64, src/Lunegit.Infrastructure/Localization/en.json:183-188. Decision: `external`, an app composition (D42). No conflicts among sources.
The stub's question "what it embeds" is answered: the panel embeds no diff and no graph; the diff or the conflict resolver sits next to it in `CommitContentView` [L src/Lunegit.Desktop/Views/CommitContentView.axaml:37-53].

### Reusable parts

- `layout.side-panel`: the panel host with its show motion (fade and slide from the right on open) [L src/Lunegit.Desktop/Views/CommitDetailView.axaml:25-28], [layout.side-panel].
- Header: a 32 px strip with a title and a 20 px close button with a 12 px icon: `widget.toolbar` and `widget.button` [L src/Lunegit.Desktop/Views/CommitDetailView.axaml:43-64].
- `data.kv-list`: the detail fields, each a quaternary label above a value (hash in mono, author as name and email, date, message), spaced 4, padding 10 and 8 [L src/Lunegit.Desktop/Views/CommitDetailView.axaml:78-133].
- `platform.clipboard`: a click on a field copies its text (hash, author email, date, full message) [L src/Lunegit.Desktop/Views/CommitDetailView.axaml.cs:29-51].
- `widget.badge` or `widget.callout`: the signed-commit notice (a verified icon and the text in the success colour) [L src/Lunegit.Desktop/Views/CommitDetailView.axaml:80-92].
- `layout.scroll-area`: the field block scrolls on its own with a bounded height so the file list keeps a minimum [L src/Lunegit.Desktop/Views/CommitDetailView.axaml:70-76].
- `widget.filter-bar`: the file filter box and its placeholder [L src/Lunegit.Desktop/Views/CommitDetailView.axaml:171-195].
- `layout.virtualized-list` and `layout.tree-view`: the changed-files list with tree guide lines, folder rows (expand and collapse) and file rows 24 px high [L src/Lunegit.Desktop/Views/CommitDetailView.axaml:215-274].
- `overlay.menu`: the file row's context menu [L src/Lunegit.Desktop/Views/CommitDetailView.axaml:259-267].
- Stays in Lunegit: the commit model, what each field shows, the changed-file rows and their status badges, "view file at this commit" [L src/Lunegit.Infrastructure/Localization/en.json:183-188].

### Build

- Milestone: external — stays in Lunegit
- Depends on: layout.side-panel, widget.toolbar, widget.button, data.kv-list, platform.clipboard, widget.badge, layout.scroll-area, widget.filter-bar, layout.virtualized-list, layout.tree-view, overlay.menu

## Open questions

- None.

## Notes for the planner

Do not build this here. It exercises the library parts together; the gallery's page registry (M4) may carry a composition page for it if Lunegit's rebuild asks.

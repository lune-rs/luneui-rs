---
id: lunegit.conflict-strip
title: Conflict resolver and hunk strip
area: lunegit
status: external
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/ConflictResolverView.axaml
owner: —
summary: Three-way merge conflict resolver (app composition); its line bands, synced scroll and anchored strips are text.code-editor-view
---

## What it is

Lunegit's merge conflict resolver: a header (path, a "Conflicted" badge, reset and close), three
read-only panes (Ours, Base, Theirs) that scroll together, an editable Result pane with a floating
strip per conflict hunk (take ours, take theirs, take both; a quiet "Resolved: side, Change" chip once
chosen), an unresolved-count summary and "Open external merge tool" and "Mark resolved" buttons. It
is merge-conflict domain composition and stays in Lunegit (D42). The reusable parts are text-view
mechanisms: full-row line bands, panes that scroll together, and overlays pinned to a line, all of
which live in `text.code-editor-view`.

## Needs

- Overlay on a text view (R-text-3, R-data-1): met by `text.code-editor-view` (anchors and bands).
- Keyboard bindings (R-infra-2): the app's.

## Existing code

External: Lunegit `src/Lunegit.Desktop/Views/ConflictResolverView.axaml` (369 lines), `Views/ConflictResolverView.axaml.cs` (98), `Rendering/ConflictHunkStripOverlay.cs` (325),
`Rendering/LineBackgroundHighlighter.cs` (195). The hunk and choice models are in `ViewModels` (`ConflictHunkViewModel`, `ConflictChoiceOption`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/ConflictResolverView.axaml:36-205,288-355, Views/ConflictResolverView.axaml.cs:13-98,
Rendering/ConflictHunkStripOverlay.cs:13-75,300-310, Rendering/LineBackgroundHighlighter.cs:12-60, src/Lunegit.Infrastructure/Localization/en.json:673-699.
Decision: the entry is `external`, with its line bands and synced scroll folded into `text.code-editor-view` (D42), stated in both entries. No conflicts among sources.

### Reusable parts

- `text.code-editor-view`, line bands: a full-row tint for 1-based lines in a set, out-of-range numbers skipped; one brush per pane (added, modified, renamed and conflicted tints) [L src/Lunegit.Desktop/Rendering/LineBackgroundHighlighter.cs:12-58], [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml:199-200,227-228,269-270,300-301].
- `text.code-editor-view`, synced scroll: the Ours, Base and Theirs panes follow one pixel offset with a re-entrancy guard; the Result pane scrolls alone [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml.cs:19-23,38-42,69-86].
- `text.code-editor-view`, line gutters, read-only panes without a caret, and the editable Result pane [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml:190-198,292-299], [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml.cs:30-36,45-59].
- `text.code-editor-view`, overlay anchors: strips placed at a hunk's first Result line from the text view's line geometry, hidden when scrolled out, a 16 px right inset, gaps not hit-testable [L src/Lunegit.Desktop/Rendering/ConflictHunkStripOverlay.cs:13-33,300-310].
- `widget.button`: take ours, take theirs, take both, mark resolved, open merge tool, and the "Change" button [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml:105-110,318-326,354-355].
- `widget.badge`: the "Conflicted" badge in the header [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml:82-91].
- `widget.callout`: the unresolved-hunks summary row (a warning icon and the text) [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml:114-124].
- `overlay.dialog`: the reset-all and mark-resolved confirmations (title, message, confirm button) [L src/Lunegit.Infrastructure/Localization/en.json:693-698].
- `widget.toolbar`: the header's icon buttons (reset, close) [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml:52-71].
- `layout.stack`: the three-pane grid with 6 px gutters (`*, 6, *, 6, *`) [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml:161], and `layout.panel` for the pane frames [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml:204].
- Stays in Lunegit: the three-way merge model, hunk states and choices (ours, theirs, both, resolved), the strip's chip text, saving and marking resolved [L src/Lunegit.Desktop/Rendering/ConflictHunkStripOverlay.cs:13-25], [L src/Lunegit.Infrastructure/Localization/en.json:677-699].

### Build

- Milestone: external — stays in Lunegit
- Depends on: text.code-editor-view, widget.button, widget.badge, widget.callout, overlay.dialog, widget.toolbar, layout.stack, layout.panel

## Open questions

- None.

## Notes for the planner

Do not build this here. The mechanisms it needs are specified once, in `text.code-editor-view` (line bands, synced panes, anchors); the gallery shows them as `code-editor-view-bands`, `code-editor-view-synced` and `code-editor-view-anchor`.

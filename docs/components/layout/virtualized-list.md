---
id: layout.virtualized-list
title: Virtualized list
area: layout
status: in-progress
consumers: lunegit, wane
source: crates/lune-ui-core/src/widgets/virtual_list.rs
owner: orch/toolkit-v1
summary: Builds only the visible rows; selection as a set of row keys (single, multi, range); keyboard navigation; no row animation while scrolling
---

## What it is

A list that builds only the rows in view (fixed row height first, measured heights next), inside a
`layout.scroll-area`. Selection is a set of row keys (single, multi and range), so a selected row that
scrolls out of view stays selected. Lunegit uses an Avalonia `ListBox` with a `VirtualizingStackPanel`
in five views; its commit list is a `ListBox` of 45 px rows with multi-selection. Rows never animate
while the list scrolls.

## Needs

- Virtualized lists (R-layout-3).
- Selection and expansion as keyed sets owned by the container, kept under keep-alive (R-core-8).
- Wane — master list and table rows: row hover and selected fills, Up and Down move the selection (R-layout-3).

## Existing code

Built in M5 (D97, D100): `crates/lune-ui-core/src/widgets/virtual_list.rs` (`Ui::virtual_list`, `ListState`). Design source (not code to move): Lunegit `src/Lunegit.Desktop/Views/RepositoryContentView.axaml:146-153,349-373` (commit list),
`MainWindow.axaml:404-418`, `Views/UnifiedDiffView.axaml:44-56`, `Views/SplitDiffView.axaml.cs:133-215` (selection), `Controls/BoundedLru.cs` (bounded geometry cache).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/RepositoryContentView.axaml:140-153,346-376, Views/RepositoryContentView.axaml.cs:74-75,101-152,
MainWindow.axaml:395-418, Views/UnifiedDiffView.axaml:44-56, Views/SplitDiffView.axaml.cs:15-30,102-113,133-275, Views/Settings/LogsSettingsView.axaml:208-226,
Views/SidePanels/NotificationHistoryPanelView.axaml:70-95, Controls/BoundedLru.cs:1-30, Controls/GraphRowControl.cs:24,96-99, Controls/AutoScrollBehavior.cs:70-103, App.axaml:215-219,
Themes/Animations.axaml:266-281, .claude/docs/animation-rules.md:143-150; Wane docs/ui/components.md:473-485,599-611.
Conflict: Lunegit's split diff keeps its selection as a set of integer row indices [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:21]; the toolkit keeps a set of row keys because rows scroll out of view and
lists are replaced [core.ui]. The key set wins (C4); an index is only the row's position this frame.

### Anatomy

- Rows have a fixed height; the commit list uses 45 px for every row, set once as a resource and used by both the row and the scroll-to-centre maths [L src/Lunegit.Desktop/App.axaml:217-219], [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:24].
- A row's highlight is inset 4 px left and right and 1 px top and bottom inside its 45 px slot, with a corner radius token [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:350-360].
- Only realized rows exist: the list builds the visible window and nothing else (the margin is a toolkit choice) [L src/Lunegit.Desktop/MainWindow.axaml:404-408] (virtualization keeps streamed appends bounded; a plain `ItemsControl` froze the UI).
- The scroll area, offset and thumb are `layout.scroll-area` and `widget.scrollbar`; the horizontal bar is Disabled on the commit list [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:153].
- Per-row caches are bounded: Lunegit's graph row geometry cache is an LRU capped at a fixed capacity "regardless of how far the user scrolls" [L src/Lunegit.Desktop/Controls/BoundedLru.cs:5-10].
- Scale target: Lunegit sizes its virtualization for about 2 million commits [L .claude/docs/animation-rules.md:145]; the toolkit's budget is [D41].
- Append at a capped tail: the list coalesces many appends into one scroll-to-last and re-reads the count at apply time [L src/Lunegit.Desktop/Controls/AutoScrollBehavior.cs:80-100]; the toolkit's follow-tail is a `layout.scroll-area` mode.
- Variable (measured) row heights: C5 requires it and no Lunegit list uses it in the files read; see Open questions.

### States

| state | look | cite |
|---|---|---|
| row rest | transparent | [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:356] |
| row hover | `surface_hover` fill | [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:361-364] |
| row selected | `surface_selected` (white at alpha 0x18) | [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:365-368], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:28], [R crates/lune-ui-core/src/theme.rs:11-20] |
| row selected and hovered | Lunegit's `SurfaceActive`; the `Theme` has no such token, so the token `surface_active` is added in M4 ([D58], [widget.button]) | [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:369-372] |
| row pressed | as hover [derived: Lunegit's row style has no pressed rule] | [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:350-373] |
| focus-visible (keyboard cursor row) | outline on the cursor row after keyboard navigation | [core.focus] |
| disabled | list inert, rows at opacity 0.4 | [derived: 0.4 is Lunegit's disabled opacity on its toolbar buttons, App.axaml:482] |
| empty | no rows; the app supplies an empty-state widget | [derived: the list builds zero rows; see widget.banner-empty-state] |
| loading | the app appends rows; the list adds nothing of its own | [derived: Lunegit's commit list pages on scroll in app code, RepositoryContentView.axaml.cs:74-75] |

### Transitions

n/a for rows while the list scrolls: virtualized rows never animate on scroll, because recycled rows would tween spuriously [L src/Lunegit.Desktop/Themes/Animations.axaml:269-272], [L .claude/docs/animation-rules.md:145]. Lunegit's commit list has no hover or
selection transition at all [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:350-373], and its only selection tween (workspace list) is scoped to a small non-virtualized list [L src/Lunegit.Desktop/Themes/Animations.axaml:273-281].
The table below is for a pointer that is still and a list that is not scrolling; a row whose hover state changes because the content moved under the pointer snaps.

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| row hover (list not scrolling) | row fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [D40] |
| row selection change (list not scrolling) | row fill | selection | fast | linear | keep | [L .claude/docs/animation-rules.md:110], [D40] |

Never animated: any row property while the offset changes, row recycling, expand of a single row, high-frequency rows (logs) [L .claude/docs/animation-rules.md:145,148].

### Input and focus

- Pointer: click selects that row only and sets the anchor [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:168-173]; Meta-click toggles the row in the set [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:153-159]; Shift-click selects the range between the anchor and the row, replacing the set [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:160-167]; press then drag selects the range from the anchor to the row under the pointer [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:201-215]. The toggle modifier is Meta in Lunegit; Ctrl on Windows and Linux is [derived: the platform convention, no source states it].
- Keys: Up and Down move the selection [W docs/ui/components.md:479]; Shift with Up or Down extends the range, PageUp, PageDown, Home and End move by a viewport and to the ends [derived: the `ListBox` defaults Lunegit inherits are not in its source; no Wane or Lunegit line states them]; the moved row scrolls into view [layout.scroll-area].
- Copy: Cmd or Ctrl+C copies the selected rows in key order [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:253-275].
- Tab stop: yes, one for the list; arrow keys move the cursor row inside it [core.focus]. Layer: Base or Page. No focus scope.
- A press that starts on a button or scroll bar inside a row is not a row press [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:135-136,185-189].
- Context menu on a row opens through `overlay.menu` (right press) [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml:154-156].

### Retained state

- Transient: row hover, row press, any row fade [core.ui].
- Persistent: the selection as a set of row keys, the anchor key, the cursor row key and the scroll offset (owned by the list and its scroll area). A selected row scrolled out of view stays selected; keys of rows no longer in the app's data are pruned when the app passes the new key list [core.ui].
- App-owned: the row count, the row key and content for each visible index, the sort order. The app passes `row(index) -> key` for the window being built each frame.
- Exit retention: no.
- Under `ui.keep_alive` a hidden list keeps its selection and offset for 600 undrawn frames [core.ui].

### Accessibility

- Role `list`; children `list-item` per built row; name = the label the app passes; states per item: selected, focused, disabled; the list exposes `focusable` and, for a multi-select list, no extra flag (the role list has no multi-select state) [infra.accessibility].
- Item name = the row's primary text the app passes; actions: default (select), focus, scroll-into-view, show-menu; the semantic tree holds only built rows [infra.accessibility].
- `role-gap: multi-selectable` — SF8 has no state for a multi-select list; M4 decides how the list's selection mode is reported.

### Strings

None: rows carry app text.

### Gallery scenes

- `virtualized-list-rest`: 1000 rows of 45 px, none selected, offset 0, pointer away, settled.
- `virtualized-list-hover`: pointer over row 3, settled.
- `virtualized-list-selected`: rows 2 and 5 selected, settled.
- `virtualized-list-selected-hover`: row 2 selected and hovered, settled.
- `virtualized-list-range`: rows 2 to 6 selected by Shift-click, anchor 2, settled.
- `virtualized-list-keyboard`: focus-visible on row 4 after Down keys, settled.
- `virtualized-list-disabled`: list disabled, settled.
- `virtualized-list-empty`: zero rows with the empty-state widget, settled.
- `virtualized-list-scrolled`: offset 40000 px with row 900 selected before the scroll, settled.
- `virtualized-list-hover-fade`: pointer enters row 3, t = 60 ms (half of `instant`).
- `virtualized-list-select-fade`: row 5 becomes selected, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill-radii (row highlight), fill-over, text, clip-rect, xform.
- New: none.

### Acceptance

- Behaviour (scripted input): click selects one row; Meta-click toggles; Shift-click selects the anchor-to-row range; Down moves the cursor and scrolls it into view; copy returns the selected rows in key order.
- Retention: a list selection survives scrolling the selected row out of view and back (C4 contract test) [core.ui]; a hidden list under keep-alive keeps selection and offset for 600 frames.
- Virtualization: with 1,000,000 rows the number of built rows per frame is at most `ceil(viewport / 45) + 2 * margin` and frame cost does not grow with the row count; measured under the D9 budget [D41].
- Pixel probes at scale 2: row slot 90 device px, highlight inset 8 px left and right and 2 px top and bottom, selected fill = the `selected` token.
- Conformance: the two transitions pass `motion::conformance`; a scripted scroll over 600 frames starts no transition (zero rows in flight) [motion].
- Semantic node: role `list`, one `list-item` per built row, `selected` follows the key set; an injected `AccessAction` default selects the row.

### Build

- Milestone: M5. Built in M5 (steps 8 and 10; the page is `virtualized-list`, 11 scenes, 1,000,000 rows in window mode and the bench). Open for `done` (D47): M11 re-checks the whole bar on every entry; the Windows and Linux run time is unverified (the Linux and Windows lint is `scripts/check-targets.sh`).
- Depends on: layout.scroll-area, widget.scrollbar, core.ui, core.focus, infra.accessibility, motion.transitions

## Open questions

- Answered (D97, D100): measured (variable) row heights: no Lunegit list read needs them (the commit list is a fixed 45 px); the strategy (a height cache with an estimated height) is for M5.
- Still open (D100: no new state): `role-gap: multi-selectable` (see Accessibility).
- Answered (D100, the app): whether the commit list pages data in the toolkit or in the app: Lunegit pages in app code on scroll [L src/Lunegit.Desktop/Views/RepositoryContentView.axaml.cs:74-75]; this entry assumes the app.

## Notes for the planner

Replaces the stub that listed five `VirtualizingStackPanel` uses without reading them. `BoundedLru` caches graph row geometry, not rows (`data.lane-graph` owns it).

---
id: layout.split-panes
title: Resizable split panes
area: layout
status: planned
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/MainWindow.axaml
owner: —
summary: Resizable columns and rows between panels and content, with a 4 px splitter that snaps columns shown or hidden
---

## What it is

Resizable split layout between the side-panel column and the content, and between the content and the terminal panel. Lunegit uses Avalonia `GridSplitter` controls, 4 px wide or high, that resize columns (side panel) and rows (terminal); the column snaps when shown or hidden [L src/Lunegit.Desktop/MainWindow.axaml:120-121,211-216], [L src/Lunegit.Desktop/Themes/Animations.axaml:314-317].

## Needs

- Resizable split panes (R-layout-2).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/MainWindow.axaml`
- `src/Lunegit.Desktop/Views/RepositoryContentView.axaml`
- `src/Lunegit.Desktop/Views/ExplorerContentView.axaml`

Moved code: none. This repo has no splitter (see Spec, Evidence).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/MainWindow.axaml:118-124,197-218`; `src/Lunegit.Desktop/Themes/Animations.axaml:314-317`
(column width snaps); `src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:160-163`; `grep -rn -i splitter crates apps` (no hits, checked).
Conflicts: none (no Wane entry; the stub's "which splitter" is answered: `GridSplitter`).

### Anatomy

- Column splitter: `GridSplitter` in column 2, width 4, transparent background, resizes columns, shown only while the side-panel column is visible [L src/Lunegit.Desktop/MainWindow.axaml:120-121].
- Row splitter: `GridSplitter` in row 1, height 4, stretched, resizes rows, shown only while the terminal panel is open [L src/Lunegit.Desktop/MainWindow.axaml:211-216].
- Terminal row height: driven by `TerminalPanelHeight` [L src/Lunegit.Desktop/MainWindow.axaml:218].

### States

| state | look | cite |
|---|---|---|
| splitter rest | transparent 4 px strip | [L src/Lunegit.Desktop/MainWindow.axaml:120-121,211-216] |
| column shown | the side-panel column and the splitter | [L src/Lunegit.Desktop/MainWindow.axaml:120-121] |
| column hidden | no column and no splitter (snap) | [L src/Lunegit.Desktop/Themes/Animations.axaml:314-317] |
| splitter hover and drag | not in the sources read; open question | open question |
| focus-visible on the splitter | not in the sources read; open question | open question |

### Transitions

n/a — column width snaps when shown or hidden; Lunegit never animates it because it drives the splitter [L src/Lunegit.Desktop/Themes/Animations.axaml:314-317].
Never animated: the split sizes during a drag (layout properties on large surfaces, [L .claude/docs/animation-rules.md:149]).

### Input and focus

- Pointer: press and drag the 4 px splitter to resize [L src/Lunegit.Desktop/MainWindow.axaml:120-121,211-216].
- Keyboard: none in the sources read; open question.
- Tab stop: open question.
- Layer: Base [R crates/lune-ui-core/src/scene.rs:17-22].
- Focus scope: none.
- Pointer capture while dragging: the `GridSplitter` captures during drag; the capture code is not read; open question.

### Retained state

- Transient: splitter hover and drag offset.
- Persistent: the split sizes (column width, terminal height) owned by the container; survive under `ui.keep_alive` [core.ui]. Lunegit's persistence of the sizes was not read: open question.
- App-owned: the shown and hidden flags (`IsSidePanelColumnVisible`, `IsTerminalPanelOpen`) [L src/Lunegit.Desktop/MainWindow.axaml:121,216].
- Exit retention: no.

### Accessibility

- The splitter is a separator: no role in the SF8 list. `role-gap: separator (splitter, resizable)`.
- Live region: none.

### Strings

None: the splitter draws no text.

### Gallery scenes

- `split-panes-two` (new): two columns, splitter between, settled.
- `split-panes-collapsed` (new): second column hidden, settled.
- `split-panes-drag-mid` (new): splitter dragged to a mid position, settled.

### Painter primitives

- Existing: fill (splitter strip), clip-rect (columns clip to their rects). [R crates/lune-ui-core/src/paint.rs:215,167]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): a drag of N px changes the column width by N px, clamped to the minimum and maximum (open question); a hidden column snaps shown with no animation. None exists.
- Pixel probes at scale 2: splitter 8 px wide or high; hit area open question.
- Conformance: the column snap is instant under all tiers.
- Semantic node: `role-gap` above; the sizes are exposed as a value (open question).

### Build

- Milestone: M7.
- Depends on: core.ui, core.focus, infra.accessibility.

## Open questions

- Minimum and maximum column and row sizes: not read; open.
- Keyboard resize and the splitter's Tab stop: not in the sources read; open.
- Whether the sizes persist across launches (Lunegit `TerminalPanelHeight` persistence): not read; open.
- The stub named `infra.drag-and-drop` for drag handling; a splitter drag is pointer input, not DnD: confirm with the planner; open.
- `role-gap: separator`: add the role to the SF8 list or accept the `group` fallback; open.
- Hit area of the 4 px splitter (wider than the paint): not read; open.

## Notes for the planner

The stub's survey guess is confirmed: Avalonia `GridSplitter`. The pixel gate has no split scene; the M7 gallery adds them.

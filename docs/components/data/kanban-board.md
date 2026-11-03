---
id: data.kanban-board
title: Kanban board
area: data
status: planned
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml
owner: —
summary: Horizontal board of columns with card lists, card drag between columns by manual pointer capture, and an error state for a corrupt board
---

## What it is

Lunegit's scratchpad kanban board: a horizontal scroll of glass-panel columns (title, card count, add and a rename or delete menu), each with a vertical card list; cards drag between columns by manual pointer capture; a corrupt or newer board shows an error state [L src/Lunegit.Desktop/STRUCTURE.md:319], [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:97-174]. The drag uses `infra.drag-and-drop`'s pointer-capture model (the code-behind, not OS drag).

## Needs

- Board columns with card lists (R-data-1).
- Card drag between columns (R-data-2, infra.drag-and-drop).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml`
- `src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs`

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:61,97,104-116,115-116,171-174,192-204` (layout and columns);
`src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:15-28,56,68,88` (drag design, threshold, capture);
`src/Lunegit.Desktop/STRUCTURE.md:319` (the board's parts, the error state, the insertion rule).
Conflicts: none (no Wane kanban; this repo has no board code).

### Anatomy

- Frame: acrylic at radius 8 while the window is active [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:61]; solid fallback while inactive [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:71].
- Toolbar margin 12 [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:97].
- Columns: a horizontal scroll (auto horizontal, no vertical) of an items control of columns [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:115-116].
- Column header: title, card count, an add control (15 px glyph in the toolbar, 14 px in the column), and an ellipsis menu (14 px) for rename and delete [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:104,133,156] [L src/Lunegit.Desktop/STRUCTURE.md:319].
- Card list: a vertical scroll per column [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:171-174].
- Card: title, tags, assignee and a description indicator; edit and delete icons (13 px) in a flyout; a vertical stack with spacing 6 [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:192-204], [L src/Lunegit.Desktop/STRUCTURE.md:319].
- Error state: a warning icon 40 px, a message and the raw file path [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:77-78] [L src/Lunegit.Desktop/STRUCTURE.md:319].

### States

| state | look | cite |
|---|---|---|
| board | columns and cards | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:115-174] |
| error (corrupt or newer board) | warning icon, message, raw path | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:77-78], [L src/Lunegit.Desktop/STRUCTURE.md:319] |
| card pressed (drag candidate) | captured; drag starts past the threshold | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:56] |
| dragging | the drop slot follows the pointer | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:15-16] |
| inactive window | solid fallback | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:71] |
| focus-visible | not in the markup read; open question | open question |

### Transitions

n/a — no motion in the board markup read; the drag follows the pointer and the drop slot is drawn at once
(open question: whether the toolkit adds a drop-slot motion).

### Input and focus

- Pointer: press a card to capture the pointer; a move beyond 4 px starts the drag; moves hit-test the columns and cards for the drop slot; release commits; capture loss cancels [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:15-16,28,56,68,88]. A press under 4 px is a click.
- Drop slot: the insertion index is the count of card midpoints above the pointer, so it does not depend on order [L src/Lunegit.Desktop/STRUCTURE.md:319].
- Keyboard: none in the sources read; open question.
- Tab stop: the add, menu and card controls [derived: buttons in the markup, ScratchpadBoardView.axaml:104-156].
- Layer: Page [derived: the board is page content].
- Focus scope: none.
- Pointer capture while pressed: yes, manual (on the card's border) [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:56].

### Retained state

- Transient: the drag ghost, the drop slot, the hovered card.
- Persistent: the card order and the column order; owned by the app's model, saved to the board file.
- App-owned: columns, cards, the board's state (ok, corrupt, newer) [L src/Lunegit.Desktop/STRUCTURE.md:319].
- Exit retention: no.

### Accessibility

- Board: role `list` of columns; each column `group` named by its title with the card count as description [infra.accessibility].
- Card: `list-item` with `button` default (open) and `delete` and `edit` actions; the card under a drag has the `dragging` state [infra.accessibility], [D55]; a keyboard move action stays open with `infra.drag-and-drop`.
- Error state: `alert` text for the message (open question: `alert` is not the status role of the other entries).
- Live region: none.

### Strings

None in the sources read: column, card and error labels come from the view model (not read).

### Gallery scenes

- `kanban-board-columns` (new): three columns with cards, settled.
- `kanban-board-dragging` (new): one card dragged between columns with the drop slot shown, settled.
- `kanban-board-error` (new): the corrupt-board error state, settled.

### Painter primitives

- Existing: acrylic (the frame), fill (columns and cards), text, icon, clip-rect (the scroll areas). [R crates/lune-ui-core/src/paint.rs:243,215,274,167]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): a press and a move under 4 px is a click; a move of 4 px or more starts a drag; release in a column commits at the computed index; capture loss cancels the drag. None exists.
- Pixel probes at scale 2: column header and card list at the markup sizes; the 4 px threshold is 8 px at scale 2.
- Conformance: none (no motion).
- Semantic node: list of columns; each card a `list-item`; a card under a drag has `dragging` [D55].

### Build

- Milestone: M8.
- Depends on: infra.drag-and-drop, layout.scroll-area, widget.button, core.focus, infra.accessibility, core.acrylic.

## Open questions

- Keyboard moves between columns: not in the sources read; open.
- The alert role for the error state: `alert` or `status`; open.
- Drop-slot motion: none in the source; open.
- Column and card labels, the board's strings: in the view model, not read; open.
- Whether the toolkit owns the drag model or the app does: `infra.drag-and-drop` decides; open.

## Notes for the planner

Lunegit's own note: the board chose manual pointer capture over `DragDrop.DoDragDrop` (STRUCTURE.md:319). The toolkit follows the same model (no OS drag, D43).

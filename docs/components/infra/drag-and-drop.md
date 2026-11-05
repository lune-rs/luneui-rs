---
id: infra.drag-and-drop
title: In-window drag and drop
area: infra
status: in-progress
consumers: lunegit
source: crates/lune-ui-core/src/drag.rs
owner: orch/toolkit-v1
summary: In-window drag state machine (press, threshold, drag, release or cancel), nearest-slot target, edge auto-scroll; no OS drag
---

## What it is

A pure, deterministic drag state machine for reordering inside one window: press arms it, travel past
a threshold starts the drag, the pointer moves the dragged item while a target slot and the neighbours'
offsets are recomputed, release commits or cancels, and an edge zone auto-scrolls the strip. Lunegit
uses it for terminal tab reorder (a Chrome-style tab strip) and the scratchpad kanban board; both use
pointer capture and never the OS drag-and-drop API. Outbound OS drag and cross-window drag are out of
v1 [D43]; OS file drop is an input event (`core.input`).

## Needs

- Pointer capture and drag state (R-infra-5).
- Persistent container state and layers for the thing being reordered (R-core-8).
- Edge auto-scroll drives `layout.scroll-area` (R-layout-2).

## Existing code

Built in M5 (D89, D104): `crates/lune-ui-core/src/drag.rs` (`ReorderDrag`, `FreeDrag`, `Ui::drag_gesture`, `free_drag_gesture`, `drag_offset`, `drag_dim`, `paint_drop_line`), `edge_step` and the crate's `Ui::drag_ghost` added for the board in M8 (D168). Design source (not code to move): Lunegit `src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs` (458 lines, the pure state machine),
`Views/TerminalPanelView.axaml.cs:262-480` (the event translator), `ViewModels/Scratchpad/ScratchpadBoardViewModel.cs:253-327` and
`Views/Scratchpad/ScratchpadBoardView.axaml.cs:44-103` (kanban). `Views/SplitDiffView.axaml.cs` drags only to select rows (see `layout.virtualized-list`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:1-458,
Views/TerminalPanelView.axaml.cs:95-106,255-475, Views/Scratchpad/ScratchpadBoardView.axaml.cs:1-148, Views/Scratchpad/ScratchpadBoardView.axaml:10-42,
ViewModels/Scratchpad/ScratchpadBoardViewModel.cs:253-327, ViewModels/Scratchpad/KanbanCardViewModel.cs:12,49, Views/SplitDiffView.axaml.cs:133-215, .claude/docs/animation-rules.md:80-110.
Wane has no drag section (docs/ui/components.md has no draggable component).
Conflicts: (1) the threshold is 4 DIPs in both Lunegit sites but measured differently: the tab controller uses horizontal travel only, so a vertical wobble stays a click
[L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:162-165,180], the kanban uses the Euclidean distance `dx*dx + dy*dy < 4*4` [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:66-68];
the toolkit takes the axis as a parameter (horizontal or both), each site cites its own. (2) Lunegit's controller is index and width based; the toolkit's reorder reports row or item keys
[core.ui]. The tab controller's numbers win for the strip; the kanban's for the free two-axis drag.

### Anatomy

- Constants, strip drag: threshold 4 DIPs horizontal [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:29-31]; auto-scroll edge zone 24 DIPs [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:33-34]; auto-scroll 12 DIPs per 16 ms tick at full ramp, about 750 px/s [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:36-37]; neighbour slide 180 ms [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:39-44].
- Constants, kanban drag: threshold 4 DIPs, Euclidean [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:28,66-68].
- Dragged item: its left edge is `pointer - grabOffset`, clamped to `[0, total - width]`, where `grabOffset` is where inside the item the pointer grabbed [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:307-318]; the grab offset is re-taken on every update while armed, so it is measured in the layout frame the drag begins in [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:191-223].
- Target slot: of the n insertion positions pick the one whose left edge is nearest the dragged item's left edge, lowest index on a tie; it stays correct for unequal widths (tabs run 108 to 270 px) [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:325-355].
- Offsets: every other item's translate is the difference between its slot now and its slot in the reordered layout; the dragged item's offset is `left - x[d]` and tracks the pointer 1:1 [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:357-392].
- Auto-scroll delta: pointer within 24 of the left viewport edge and the strip can scroll left gives `-12 * ramp((24 - x) / 24)`; within 24 of the right edge and `offset < extent - viewport - 0.5` gives `+12 * ramp((x - (viewport - 24)) / 24)`; else 0; the ramp is linear and clamped to [0, 1] on purpose, un-eased [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:394-417].
- Auto-scroll tick: the pointer is parked but the content moves, so the content X is re-derived as `viewportX + newOffset` each tick [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:225-244]; Lunegit drives it with a 16 ms timer started only while a delta is non-zero [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:99-103,444-475]. The toolkit has no timer: the frame time drives it (about 750 px/s times the frame's dt) and the gesture reports `Repaint::Now` while a delta is non-zero [core.ui].
- Release result: `Click` (a drag that ended where it started, or armed and never dragged), `Commit(from, to)`, or `None`; the indices travel in the result because release clears all state [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:65-82,250-272].
- Totality is a contract: any input (empty or null widths, a single item, an out-of-range index, NaN or infinite coordinates, zero or negative widths) returns a usable result and never throws; the degenerate answer is not dragging, all offsets zero, delta zero [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:16-20].
- Kanban: a free drag where the card stays in place dimmed while a 2 px accent drop line shows the insertion slot and the target column's background becomes `surface_hover`; Lunegit has no ghost, the toolkit's board adds one and a landing (D168) [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:17-19,38-42], [L src/Lunegit.Desktop/ViewModels/Scratchpad/KanbanCardViewModel.cs:49]. Insertion index = the count of cards whose vertical midpoint is above the pointer, in the column under the pointer's X [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:107-147].
- Decision state lives in a pure object and the view is a thin event translator [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:6-10], [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:11-24]; the toolkit keeps that shape: the state machine is a pure type, `ui.interact` feeds it.
- No OS drag and drop: Lunegit uses manual pointer capture because `DragDrop.DoDragDrop` runs a native nested loop whose macOS reliability was unverified [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:18-24].

### States

| state | look | cite |
|---|---|---|
| idle | nothing | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:65-69] |
| armed | pressed below the threshold: identical to a press, no offsets applied | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:107-108,204-209] |
| dragging | dragged item follows the pointer at 100 percent; neighbours shifted by their offsets | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:119-124,390-391] |
| dragging, kanban | source card at opacity 0.4; target column `surface_hover`; 2 px `accent` drop line | [L src/Lunegit.Desktop/ViewModels/Scratchpad/KanbanCardViewModel.cs:49], [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:17-19,38-42] |
| auto-scrolling | dragging with a non-zero delta at an edge zone | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:394-414] |
| no target (kanban) | pointer outside every column: no drop line; a drop is a no-op | [L src/Lunegit.Desktop/ViewModels/Scratchpad/ScratchpadBoardViewModel.cs:275-293] |
| committed | the app applies the move; offsets clear at once | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:348-355] |
| cancelled | offsets clear, nothing moves | [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:330-341] |

Hover, focus-visible and disabled belong to the items being reordered, not to the gesture.

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| neighbour slides to its new slot | item x offset | move | fast | sine-in-out | snap | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:39-44], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:419-431] |
| release: dragged item settles into its slot | item x offset | settle | fast | sine-in-out | snap | [D40], [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:39-44] |
| kanban source card dims | card opacity | selection | fast | linear | keep | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:28-32] |

The dragged item gets no transition so it tracks the pointer exactly: a 180 ms transition would lag behind the cursor [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:413-418]; it follows 1:1 with no animation and so has no row in the table.
Reduced and Off snap the neighbour slide: Lunegit skips the transition when `AnimateTabReorder` is false [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:421].
Lunegit applies the move and clears the offsets in one step on release, with transitions removed first so nothing double-moves [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:369-371]; the animated settle is a [D40] addition (settle = 180 ms per motion.md).
The kanban dim is a `DoubleTransition` on card opacity with `AnimDurationFast`; the style names no easing, so linear [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:28-32], [derived: no easing set].
Never animated: auto-scroll (linear and un-eased on purpose [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:416-417]), layout of the strip, items other than the neighbours.

### Input and focus

- Pointer: press on an item arms (left button only; right press opens the context menu, middle is ignored [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:48]); the item activates on press and the gesture never activates anything [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:136-140]; crossing the threshold takes pointer capture from the pressed button, so that button gets capture-lost and does not fire a click [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:284-290]; capture stays until release, even outside the strip [core.ui].
- Cancel paths, all through one teardown: Escape while dragging (swallowed only then, and before an overlay around the drag dismisses on it, D175) [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:336-341]; capture lost [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:330-334]; the button no longer down, or the host hidden mid-drag [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:266-278]; a collection change mid-drag [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:343-347]. Kanban: capture lost cancels [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:88-95].
- Keys: Escape only; no keyboard drag in Lunegit (see Open questions). Tab stop: none, the items are the stops [core.focus].
- Layer: the layer of the strip; the dragged item is drawn in place (no overlay) [derived: Lunegit transforms the tab in place, TerminalPanelView.axaml.cs:404-409]. A free drag's ghost is the caller's: `Ui::drag_ghost` runs it one layer group up, out of every clip and recording; the board's is the one user (D168).
- The free gesture only sees an item that still calls it: when the source leaves the items, the caller cancels the machine. A press and its release in one frame are a click.
- Focus scope: none.

### Retained state

- Transient: armed and dragging flags, press index, grab offset, current pointer X, the offsets array and the auto-scroll delta; held only while the gesture holds pointer capture and cleared on every end path [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:84-103,274-288].
- Persistent: none; the item order is app-owned and the commit is reported, not applied [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:348-355].
- App-owned: item order, item widths, the strip's scroll metrics (re-read every update because widths change during a gesture [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:48-53]).
- Exit retention: yes: the settle keeps the dragged item and its neighbours drawing from retained offsets until it ends: Full 180 / Reduced 0 / Off 0 ms [core.ui], [D40].

### Accessibility

- No node of its own. The item widget's node carries `pressed` while the gesture is armed or dragging [infra.accessibility].
- A keyboard or assistive-technology reorder has no source: Lunegit has none and the action list has no move action. `role-gap: move/reorder action for list-item and tab`.

### Strings

None: the gesture has no text.

### Gallery scenes

- `dnd-idle`: three tabs of widths 100, 200 and 100 with gap 5, settled.
- `dnd-armed`: tab 1 pressed, pointer 3 px from the press point, settled.
- `dnd-dragging`: tab 1 (the first tab) dragged to left edge 300, neighbours shifted, settled.
- `dnd-autoscroll`: pointer 6 px from the right viewport edge with room to scroll, settled. Built `Moving`: a parked pointer in the edge zone keeps scrolling, so a frame is always due.
- `dnd-kanban-target`: a card dragged over column 2 below card 1: dimmed source, drop line, column hover, settled.
- `dnd-kanban-no-target`: pointer between columns, no drop line, settled.
- `dnd-cancelled`: after Escape, offsets cleared, settled.
- `dnd-slide`: neighbour sliding into its slot, t = 90 ms (half of `fast`).
- `dnd-settle`: released item settling, t = 90 ms.
- `dnd-dim`: kanban source card dimming, t = 90 ms.

### Painter primitives

- Existing: fill, fill-radii, ring, xform (translate), clip-rect.
- New: none.

### Acceptance

- State machine, scripted pointer, no clock (hand-computed from the code at TerminalTabDragController.cs:167-189,250-272; M5 must reproduce them): horizontal travel 3.9 stays armed; 4.0 starts a drag; vertical-only travel never starts one; release after a drag that ends at the start index is `Click`; ending at another index is `Commit(from, to)`; release while only armed is `Click`; release with nothing in flight is `None`.
- Nearest slot, widths 100, 200, 100, spacing 5, dragging item 0: left edge 300 gives target 2 and offsets (300, -105, -105); left edge 250 gives target 1; the left edge is clamped to `[0, 310]` [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:307-318].
- Auto-scroll: viewport 500, pointer at x = 0 with room to the left gives -12; x = 12 gives -6; x = 24 gives 0; x = 500 with room to the right gives +12; at offset 0 the left delta is 0; at `offset >= extent - viewport - 0.5` the right delta is 0 [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:403-414].
- Totality: NaN or infinite coordinates, zero widths, an empty list and an out-of-range index never panic and answer not dragging.
- Cancel: Escape, capture loss, button up and a collection change each end the drag with offsets zero and no commit; Escape with no drag in flight is not consumed.
- Pixel probes at scale 2: kanban drop line 4 device px high, source card at alpha 0.4.
- Conformance: slide and settle pass `motion::conformance` at 60 Hz (monotonic, exact end, settled within 180 ms plus one frame); Reduced and Off snap [motion].
- Semantic node: the item node reports `pressed` during the gesture [infra.accessibility].

### Build

- Milestone: M5. Built in M5 (steps 14 and 15; the page is `drag-and-drop`, 10 scenes). Known look limit: text draws above fills, so tab labels show through a sliding or settling tab during `dnd-slide` and `dnd-settle`. Open for `done` (D47): M11 re-checks the whole bar on every entry; the Windows and Linux run time is unverified (the Linux and Windows lint is `scripts/check-targets.sh`).
- Depends on: core.ui, core.input, layout.scroll-area, motion.transitions

## Open questions

- Still open (D104, no keyboard reorder in v1): `role-gap: move/reorder action for list-item and tab` (see Accessibility); keyboard reordering is not in Lunegit and is left to M5 to decide.
- Answered (D104, the free two-axis form): whether the kanban board (data.kanban-board) takes the free two-axis form of this machine or its own: it takes it, through `free_drag_gesture`.
- Answered (D168): who owns the board's drag: the toolkit's `FreeDrag` and `Ui::kanban_board`; the app owns the order and applies the reported move. Edge auto-scroll is `edge_step`, shared with the strip.
- Answered in part (D169): keyboard moves exist on the board (Shift with an arrow); the `role-gap` above stays.

## Notes for the planner

Replaces the stub's question "OS drag-and-drop API or pointer tracking": it is pointer tracking only [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:18-24]. The split diff view's drag selects rows and is covered by `layout.virtualized-list`.

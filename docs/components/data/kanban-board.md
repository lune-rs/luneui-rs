---
id: data.kanban-board
title: Kanban board
area: data
status: in-progress
consumers: lunegit
source: crates/lune-ui-core/src/widgets/kanban_board.rs
owner: toolkit-v1 M8
summary: Horizontal board of columns with card lists, card drag between columns by manual pointer capture, and an error state for a corrupt board
---

## What it is

Lunegit's scratchpad kanban board: a horizontal scroll of glass-panel columns (title, card count, add and a rename or delete menu), each with a vertical card list; cards drag between columns by manual pointer capture; a corrupt or newer board shows an error state [L src/Lunegit.Desktop/STRUCTURE.md:319], [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:97-174]. The drag uses `infra.drag-and-drop`'s pointer-capture model (the code-behind, not OS drag).

## Needs

- Board columns with card lists (R-data-1).
- Card drag between columns (R-data-2, infra.drag-and-drop).

## Existing code

Built in M8 (D167 to D169): `crates/lune-ui-core/src/widgets/kanban_board.rs` (`Ui::kanban_board`, `BoardOptions`, `KanbanColumn`, `KanbanCard`, `BoardError`, `BoardResponse`, `CardMove`, `landed_index`, `paint_card`, `card_height`); the gesture is `FreeDrag` in `drag.rs`. Page `kanban-board`, tests `crates/lune-ui-core/tests/widget_kanban_board.rs`.

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml`
- `src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs`
- `src/Lunegit.Application/Services/Scratchpad/KanbanOperations.cs` (the move index)

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:61,97,104-116,115-116,171-174,192-204` (layout and columns);
`src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:15-28,56,68,88` (drag design, threshold, capture);
`src/Lunegit.Desktop/STRUCTURE.md:319` (the board's parts, the error state, the insertion rule).
Conflicts: none (no Wane kanban; this repo has no board code).

### Anatomy

- Sizes (`kanban_board.rs` constants): margin 12, header 28 and gap 10; column 272 with 1 px border, gap 10, padding 10, header 24 and gap 8, card list at most 640; card 250 wide, title wraps at 228, padding 10, gap 6, radius 6, stack spacing 6; tag text 9, radius 4, padding 6,1, room 2 above and 4 right; icon buttons 24 square, radius 6 [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:10-52,97-110,127-204,215-250]. SemiBold text is `.medium()`: the bundled faces are Regular and Medium.
- A column hugs its cards up to 640 and the room left, then its list scrolls; every card keeps the gap under it. A card's footer row always has its place in the stack, so a card with no assignee and no note keeps the stack's 6 under its last row, as the source's empty dock does.
- Frame: acrylic at radius 8 while the window is active [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:61]; solid fallback while inactive [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:71].
- Toolbar margin 12 [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:97].
- Columns: a horizontal scroll (auto horizontal, no vertical) of an items control of columns [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:115-116].
- Column header: title, card count, an add control (15 px glyph in the toolbar, 14 px in the column), and an ellipsis menu (14 px) for rename and delete [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:104,133,156] [L src/Lunegit.Desktop/STRUCTURE.md:319].
- Card list: a vertical scroll per column [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:171-174].
- Card: title, tags, assignee and a description indicator; edit and delete icons (13 px) in a flyout; a vertical stack with spacing 6 [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:192-204], [L src/Lunegit.Desktop/STRUCTURE.md:319].
- Error state: only a centred stack, spacing 10, at most 420 wide: a warning icon 40 px, the title, the message and the raw file path in mono, which wraps [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:75-93] [L src/Lunegit.Desktop/STRUCTURE.md:319]. A wrapped text is one block centred as a whole: `Spec` has no per-line alignment.

### States

| state | look | cite |
|---|---|---|
| board | columns and cards | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:115-174] |
| error (corrupt or newer board) | only the warning, title, message and raw path; any drag and landing end | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:75-93], [L src/Lunegit.Desktop/STRUCTURE.md:319], [D167] |
| card hover | fill `surface_hover` | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:33-35] |
| card pressed (drag candidate) | captured; drag starts at 4 px | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:56] |
| dragging | source in place at `DIM_ALPHA`; a ghost at the pointer, one layer group up, over an opaque `window_fallback`; target column `surface_hover`; `paint_drop_line` in the 6 px gap of the slot, nothing pushed aside | [D168] |
| no target | pointer between or past the columns, or above or below the viewport: no fill, no line | [D168] |
| landing | the ghost alone, on its way to the slot the model now gives the card; the body is back when it arrives | [D168] |
| empty column | the hint, `fg_faint`, 2 and 6 from the list's top left | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:257-262] |
| inactive window | solid `window_fallback` | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:71] |
| focus-visible | `RingKind::Edge` on the card at its radius; the ghost carries it while landing | [D169] |
| disabled | drawn at 0.4; no drag, no key acts, no landing | [D140] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| card hover, column target | fill | hover | instant | linear | keep | [D40], [D168] |
| card focus | ring opacity | focus | fast | linear | keep | [D40] |
| source dims | card opacity | selection | fast | linear | fade | [D168] |
| a card's slot changes in its column | body `y` | move | fast | sine-in-out | snap | [D168] |
| drop line inside one column | line `y`; across columns it snaps | move | fast | sine-in-out | snap | [D168] |
| landing | ghost position | settle | fast | sine-in-out | snap | [D168] |

The ghost tracks the pointer 1:1 and has no row. Auto-scroll is never eased. Reduced: dims fade, moves snap, so there is no landing and a released card is simply in its slot. Off: everything snaps. `docs/motion.md`'s neighbours row says `settle` where the code runs `SLIDE`, `move`; the two have the same token and easing.

Every way a card's position could jump is one path: an accepted drop, a refused one, a drop outside, Escape, capture loss and a keyboard move across columns all start a landing from where the card was drawn. A keyboard move inside a column is a slide. The landing reads the slot each frame, so scrolling under it is followed; a card or column deleted under it ends it; a press on the landing card holds it there, and from there it drags or goes on. A card is drawn exactly once a frame, the dimmed source only while dragging.

### Input and focus

- Pointer: a primary press arms; 4 px of travel starts the drag; under it, a release is a click and reports `opened`; moves hit-test the columns; a release in a column reports `moved`; a secondary click reports `context` [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:15-16,28,56,68,88], [D168].
- Drop slot: the insertion index is the count of card midpoints above the pointer, so it does not depend on order [L src/Lunegit.Desktop/STRUCTURE.md:319]. The midpoints are the slots', never animated positions. The columns are in window space, cut to the board's viewport; only the x decides the column, and the pointer must be inside the viewport's height. The columns come from this frame's model and the offsets the scroll areas left last frame: the areas take one closure each, so a wheel step moves the target one frame after the picture.
- `CardMove.index` counts the list as it stands, the card still in it; `landed_index` is the same-column correction [L src/Lunegit.Application/Services/Scratchpad/KanbanOperations.cs:13-20,87-120]. A drop outside every column, or at the card's own place (its index or the next), reports nothing.
- Wheel and swipe: a column whose cards fit registers no scroll sense, so a wheel or trackpad over its cards scrolls the board; a column that overflows scrolls its own list under the single-topmost rule, so a sideways swipe over it moves nothing (the board scrolls from the header, the margins or a column that fits) [D175].
- Cancel: Escape while dragging (a first Escape cancels the drag even in a dialog, and only a second one closes the dialog, D175), capture loss, a change of the column count, the window losing focus, the scope turning disabled, the error state, and the source leaving `columns` (the board's own check: the gesture sees only a card that still calls it).
- Auto-scroll: a 24 px zone at the board's left and right and the target column's top and bottom, up to 12 px per 16 ms by `dt`; the first frame in a zone has no `dt`, so idle time before it is no step; it asks a frame while it moves and stops at the content's end [D168].
- Keyboard (D169): all cards are one roving stop; the add and menu buttons are plain stops. Up, Down, Home, End within a column; Left, Right to the nearest index in the next column with cards; Enter and Space open; Shift with an arrow moves the card and reports the same `CardMove` as a drop, one move a frame, nothing at an end; a column with no cards is skipped by Left and Right and is a Shift target at index 0. Shift with Home or End is Home or End. Focus stays on a moved card by key, and the card is revealed in its column and the board. Control, Alt and Command with any of these stay shortcuts. PageUp and PageDown are the column area's; a column that does not overflow leaves them to the board's area. During a pointer drag the keys are taken and do nothing.
- Disabled, focused first: every key above is taken and nothing acts (D140).
- Layer: the board's; the ghost is on the next layer group (Base and Page to `Popup`; Popup and Modal to `Raised`; `Raised` to `Toast`), hits nothing, and belongs to no `record`.
- Focus scope: none.
- Pointer capture while pressed: yes, through `Sense::DRAG` [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs:56].

### Retained state

- Retained, in one `BoardMem`: the gesture, the grab point, the landings, the roving cursor, a pending reveal, the last scroll offsets; each card's `y`, hover and dim transitions by card id. Scroll offsets are the areas' own.
- Persistent: the card order and the column order; owned by the app's model, saved to the board file.
- App-owned: columns, cards, the board's state (ok, corrupt, newer) [L src/Lunegit.Desktop/STRUCTURE.md:319].
- Exit retention: no.

### Accessibility

- Board: role `list` named `name`, or `title`; each column `group` named by its title with the card count as description [infra.accessibility].
- Card: `list-item` named by its title with `Default` (open), `Focus`, `ScrollIntoView` and `ShowMenu` (the app's edit and delete live in its menu); the card under a drag has `dragging` [infra.accessibility], [D55]; a keyboard move action stays open with `infra.drag-and-drop`.
- Buttons: `button` named by the strings below. The empty hint is `static-text`.
- Error state: `alert`, assertive, named by the title with the message as description, as an error toast (D146, D167); the path is a `static-text` child.

### Strings

`luneui.kanban-board.add-column` "Add column", `.add-card` "Add card", `.column-actions` "Column actions", `.empty-column` "No cards yet." [L src/Lunegit.Infrastructure/Localization/en.json:900-907], [D167]. The app passes the column, card and error text.

### Gallery scenes

- `kanban-board-columns` (new): three columns with cards, settled.
- `kanban-board-dragging` (new): one card dragged between columns with the drop slot shown, settled.
- `kanban-board-error` (new): the corrupt-board error state, settled.
- `kanban-board-focus` (new): the cards' stop with its ring, settled.
- `kanban-board-inactive` (new): the fallback fill, settled.
- `kanban-board-empty` (new): three empty columns with their hints, settled.
- `kanban-board-settle-mid` (new): a card landing half way, in motion.

### Painter primitives

- Existing: acrylic (the frame), fill (columns and cards), text, icon, clip-rect (the scroll areas). [R crates/lune-ui-core/src/paint.rs:243,215,274,167]
- New: none.

### Acceptance

`crates/lune-ui-core/tests/widget_kanban_board.rs`, scripted input and injected time:

- Behaviour: a press and a move under 4 px is a click and reports `opened`, a press and release in one frame too; 4 px starts a drag; a release in a column reports the move, down and up its own column and across columns against a model using `landed_index`; a drop at its own place or outside reports nothing; drop line at the slot, target fill; `context`, the buttons.
- The two classes: every jump (accepted, refused, outside, Escape, capture loss, a column added, keyboard in a column and across, delete, a press on a landing card, scrolling, a tier and a scale change) sampled at 60 Hz with the title drawn once a frame and no step above the conformance bound; every input a drag can outlive (Escape, capture loss, a column added, the source deleted, the window lost, disabled, the error state) with no `dragging` after it, no ghost past its landing and `Idle`.
- The ghost on `Popup` over a `Page` board and on `Raised` in a dialog body, unclipped, hitting nothing; a dialog closing mid-drag replays none.
- Auto-scroll by `dt` on both axes, to the end, then `Idle`.
- Keys, chords on three modifiers and disabled; one tab stop for all cards; page keys scroll a column and die disabled.
- Pixel probes at scale 2: column 544 px, card radius 12, threshold 8 px.
- Conformance: `SLIDE`, `SETTLE`, `DIM` on three tiers.
- Semantic nodes: the tree above, `testing::disabled_violations`.

### Build

- Milestone: M8.
- Built in M8 (step 5).
- Depends on: infra.drag-and-drop, layout.scroll-area, widget.button, core.focus, infra.accessibility, core.acrylic.

## Open questions

- Answered (D169): keyboard moves between columns: Shift with an arrow, reporting the same `CardMove`; the access tree's move action stays open with `infra.drag-and-drop`'s `role-gap` (D55).
- Answered (D167): the alert role: `alert`, assertive, as an error toast.
- Answered (D168): drop-slot motion: the line slides inside a column and snaps across; the ghost lands with `settle`.
- Answered (D167): the board's strings: four `luneui.kanban-board.*` keys; the app passes the rest.
- Answered (D167, D168): the drag model is the toolkit's `FreeDrag`; the app owns the order and applies `CardMove`.

## Notes for the planner

Lunegit's own note: the board chose manual pointer capture over `DragDrop.DoDragDrop` (STRUCTURE.md:319). The toolkit follows the same model (no OS drag, D43).

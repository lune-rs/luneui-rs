---
id: layout.split-panes
title: Resizable split panes
area: layout
status: done
consumers: lunegit
source: crates/lune-ui-core/src/widgets/split_panes.rs
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
| splitter hover and drag | no look: transparent, as at rest; the cursor is `ColResize` (columns) or `RowResize` (rows) while hovered or held | [D150] |
| focus-visible on the splitter | the 4 px strip fills with `accent` (fade `focus`, snaps under Off) | [D150] |

### Transitions

n/a — column width snaps when shown or hidden; Lunegit never animates it because it drives the splitter [L src/Lunegit.Desktop/Themes/Animations.axaml:314-317].
Never animated: the split sizes during a drag (layout properties on large surfaces, [L .claude/docs/animation-rules.md:149]).

### Input and focus

- Pointer: press and drag the splitter to resize [L src/Lunegit.Desktop/MainWindow.axaml:120-121,211-216]. The hit is 8 px (2 px into each pane) over the 4 px paint. A press follows at once (no drag threshold): size = the size at the press plus the travel, clamped to `[min, max]` and to the room the other pane's `min_other` leaves; `min` wins when both cannot fit [D150]. The pointer is captured from the press to the release; a release ends the drag.
- Keyboard: the strip is one tab stop. The arrows along the axis (Left and Right for columns, Up and Down for rows) move the strip 16 px; Home and End set the sized pane to its smallest and largest; Escape during a drag restores the size at the press and is consumed (with nothing in flight it is not), before a dialog around the strip closes on it (D175). Up and Down on columns grow and shrink the sized pane. The accessibility Increment and Decrement raise and lower the value (the sized pane's size) by 16 on both axes and both sides, whichever way that moves the strip. Control, Alt and Command chords on the arrows, Home and End stay shortcuts [D157]. Keys are dropped while the pointer holds the strip [D150].
- Tab stop: the strip, after the panes' own stops (panes draw first, the strip last, so its hit is on top).
- Layer: Base [R crates/lune-ui-core/src/scene.rs:17-22].
- Focus scope: none.
- Pointer capture while dragging: the strip holds the capture (`Sense` drag plus focusable), so a drag that crosses a pane's button clicks nothing.

### Retained state

- Transient: the drag's press point, press size and whether Escape cancelled it.
- Persistent: the sized pane's stored size (`SplitState.size`, `Ui::split_state(key)`), which survives undrawn frames under `ui.keep_alive` [core.ui]. It changes only on user input or the app's own write: a smaller window clamps what is drawn, not what is stored, so the size comes back when the window grows [D150]. A NaN or infinite size, written by the app or asked by an assistive technology, counts as unset or changes nothing.
- App-owned: the shown and hidden flags (`SplitOptions.hidden`; `IsSidePanelColumnVisible`, `IsTerminalPanelOpen`) [L src/Lunegit.Desktop/MainWindow.axaml:121,216], the bounds (`min`, `max`, `min_other`, `default`; Lunegit's side panel 310 to 700 and terminal 120 to 900 are the gallery's) and whether the size persists across launches: the app loads and stores `SplitState.size`.
- Exit retention: no.

### Accessibility

- The splitter has role `separator` (resizable, with a value) [infra.accessibility], [D55]: `Value::Number` with the sized pane's size in px, its effective `min` and `max`, `step` 16, and the actions Increment, Decrement and SetValue. A set value is clamped; NaN and infinity change nothing. Disabled: `Disabled`, not focusable, no action. Hidden: no node.
- Live region: none.

### Strings

None: the splitter draws no text.

### Gallery scenes

- `split-panes-two` (new): a column split and a row split, the strips transparent, settled.
- `split-panes-collapsed` (new): the first column hidden, the second takes the whole width, settled.
- `split-panes-drag-mid` (new): the column strip dragged 200 px to the right and held, settled.
- `split-panes-rows` (new): the row strip dragged 70 px up, the panel below grown, settled.
- `split-panes-focus` (new): Tab onto the column strip, the 4 px strip filled with `accent`, settled.

### Painter primitives

- Existing: fill (splitter strip), clip-rect (columns clip to their rects). [R crates/lune-ui-core/src/paint.rs:215,167]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): `crates/lune-ui-core/tests/widget_split_panes.rs`: a drag of N changes the size by N on both axes and sides; clamps at `min`, `max` and `min_other`, `min` winning when nothing fits; shrinking then growing the window returns the stored size; arrows, Home, End; increment, decrement and set value, NaN ignored; Escape; hidden snaps in all tiers and hits nothing; the size survives `keep_alive` and undrawn frames; the 8 px hit over the 4 px strip; a drag across a pane's button clicks nothing.
- Pixel probes at scale 2: the strip is 8 device px (4 logical) and its hit 16 (8 logical); the focus fill is the 4 px strip in `accent`.
- Conformance: the size never animates, so a hidden pane snaps in every tier; only the focus fill fades (`focus`, snaps under Off).
- Semantic node: role `separator` [D55]; the size is exposed as a number value.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_split_panes.rs`: behaviour `a_drag_of_n_changes_the_size_by_n_on_both_axes_and_sides`, `the_drag_clamps_at_min_max_and_the_room_left_for_the_other_pane`, `min_wins_when_nothing_fits`, `a_smaller_window_clamps_what_is_drawn_and_not_what_is_stored`, `the_arrows_along_the_axis_move_the_strip_16_px`, `home_and_end_go_to_the_sized_panes_ends_and_keys_clamp`, `escape_during_a_drag_restores_the_press_size_and_is_consumed_and_a_release_ends_it`, `a_drag_across_a_panes_button_clicks_nothing`, `the_size_survives_keep_alive_and_frames_it_was_not_drawn`; pixel probes `pixel_probes_at_scale_two`, `the_hit_is_8_px_over_the_4_px_strip_and_a_pane_beside_it_keeps_its_own`, `focus_visible_fills_the_4_px_strip_with_accent_and_nothing_else_does`; conformance `the_focus_fill_conforms_in_three_tiers`, `hidden_gives_the_other_pane_the_whole_rect_with_no_strip_no_node_and_no_hit`; node `the_node_is_a_separator_with_the_sizes_as_a_number_and_the_three_actions`; disabled `a_disabled_split_does_not_drag_or_move_and_the_walk_holds`.

### Build

- Milestone: M7.
- Depends on: core.ui, core.focus, infra.accessibility.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_split_panes.rs::a_drag_of_n_changes_the_size_by_n_on_both_axes_and_sides, crates/lune-ui-core/tests/widget_split_panes.rs::the_drag_clamps_at_min_max_and_the_room_left_for_the_other_pane, crates/lune-ui-core/tests/widget_split_panes.rs::the_arrows_along_the_axis_move_the_strip_16_px, crates/lune-ui-core/tests/widget_split_panes.rs::escape_during_a_drag_restores_the_press_size_and_is_consumed_and_a_release_ends_it, crates/lune-ui-core/tests/widget_split_panes.rs::the_size_survives_keep_alive_and_frames_it_was_not_drawn, crates/lune-ui-core/tests/widget_split_panes.rs::pixel_probes_at_scale_two, crates/lune-ui-core/tests/widget_split_panes.rs::the_focus_fill_conforms_in_three_tiers, crates/lune-ui-core/tests/widget_split_panes.rs::the_node_is_a_separator_with_the_sizes_as_a_number_and_the_three_actions`; page `split-panes`; unverified here: none.

## Open questions

None. Closed by [D150]: the splitter is a toolkit component (`Ui::split_panes`, `SplitOptions`, `SplitState`). Minimum and maximum are the app's arguments (Lunegit's 310 to 700 and 120 to 900 are the gallery's); keyboard resize is the arrows (16 px), Home and End, with the splitter one tab stop; persistence is the app's (it reads and writes `SplitState.size`); the stub's `infra.drag-and-drop` does not apply, a splitter drag is pointer input; the hit is 8 px over the 4 px paint. The strip is a 4 px gutter between the panes: the sized pane gets exactly its size and the other what is left. Panes are drawn with the size the frame ends with (D281): the pointer's grab and the strip's keys are worked out before they draw, from the strip's own reading (`Ui::peek_response`, which registers nothing), and the strip still registers after them, so it is on top. Only an accessibility set-value arrives with the strip's region, after the panes, and shows in them on the catch-up frame the widget then asks for. Tests: `a_drag_step_below_the_threshold_shows_in_the_panes_in_that_frame`, `an_arrow_key_shows_in_the_panes_in_that_frame`, `an_escape_during_a_drag_puts_the_panes_back_in_that_frame`, `a_set_value_still_asks_for_a_catch_up_frame_and_a_settled_split_is_idle`.

## Notes for the planner

The stub's survey guess is confirmed: Avalonia `GridSplitter`. The pixel gate has no split scene; the M7 gallery adds them.

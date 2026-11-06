---
id: data.table
title: Data table
area: data
status: done
consumers: wane
source: crates/lune-ui-core/src/widgets/table.rs
owner: —
summary: 28 px rows with hairlines, numeric columns in mono, sortable header look, selectable rows
---

## What it is

28 px rows with hairlines, numeric columns in mono, sortable header look, selectable rows. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Click selects a row (master-detail); header click sorts (the look only is mocked); column resize, reorder and keyboard row navigation are requirements, not mocked. Long text ellipsizes.

## Needs

- Wane — Variants: cell tones: muted, warn, err, dim (no value or a reference figure), strong (the total); wrap (a label cell may take two lines); mono; num (RznMono tabular, right-aligned, header too); ellipsis. Controls inside: `widget.combo --sm`, `widget.button --sm`, `widget.toggle`, `widget.badge`. (R-data-2)
- Wane — States: row rest; hover (`--surface-hover`); selected (`--surface-selected`, bright text); sorted header (secondary text, 12 px arrow). (R-data-2)
- Wane — Sizes: Header 28 (11/16 Medium tertiary, hairline); rows 28 with 1 px `--border-subtle` hairlines, no zebra, no row radius; cell padding 0 8, first cell 16, last cell 16; body 13; wrap cells padding 4 vertical. (R-data-2)
- Wane — Behaviour: Click selects a row (master-detail); header click sorts (the look only is mocked); column resize, reorder and keyboard row navigation are requirements, not mocked. Long text ellipsizes. (R-data-2)

## Existing code

Built in M8: `crates/lune-ui-core/src/widgets/table.rs` (`Ui::table`) on the list frame of `virtual_list.rs` (D161, D162); page `table`. Design source (not code to move): Wane `docs/ui/mocks/css/components.css` (data.table); no lune-ui source. Density from Luneterm glass-ui dump 06 row rhythm.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Wane `docs/ui/components.md:599-612`; `crates/lune-ui-core/src/theme.rs:55-84`
(`surface_hover`, `surface_selected`, `border_subtle`, `fg_*`, `font_*`); `grep -rn -i datagrid src/Lunegit.Desktop` at 33a1dd41: no hits (Lunegit has no data grid; its list rows are the side-panel lists, [L .claude/docs/animation-rules.md:145]).
Conflicts: none between sources. Wane's header 11/16 and body 13 are the only values; the repo has `font_s` 11 [R crates/lune-ui-core/src/theme.rs:82] and `font_base` 13 [R crates/lune-ui-core/src/theme.rs:83], which agree.

### Anatomy

- Header: 28 high; text 11, Medium, `fg_tertiary`; hairline below [W docs/ui/components.md:599-612]. Repo sizes: `font_s` 11 [R crates/lune-ui-core/src/theme.rs:82]; tertiary `fg_tertiary` #FFFFFF88 [R crates/lune-ui-core/src/theme.rs:58].
- Row: 28 high; hairline 1 px `border_subtle` #FFFFFF1A at the bottom [W docs/ui/components.md:599-612], [R crates/lune-ui-core/src/theme.rs:63]; no zebra; no row radius [W docs/ui/components.md:599-612].
- Cell: padding 0 8; first and last cell 16 [W docs/ui/components.md:599-612]; body text 13 [W docs/ui/components.md:599-612], [R crates/lune-ui-core/src/theme.rs:83].
- Numeric cell: RznMono tabular, right-aligned, header too [W docs/ui/components.md:599-612].
- Wrap cell: padding 4 vertical [W docs/ui/components.md:599-612].
- Sort arrow: 12 px in the sorted header [W docs/ui/components.md:599-612].

### States

| state | look | cite |
|---|---|---|
| row rest | no fill; text `fg_primary` | [W docs/ui/mocks/css/components.css:538-563], [R crates/lune-ui-core/src/theme.rs:55-56] |
| row hover | fill `surface_hover` across the slot at radius 0; the row stays lit while the pointer is over a control in it, and not under a dialog (`pointer_in`, [D127], [D161]) | [W components.css:538-563] |
| row selected | fill `surface_selected`; all text `fg_bright` whatever the tone (the selector outranks the tone class) | [W components.css:538-563] |
| row selected and hovered | `surface_active` ([D58]) | [derived: the list frame's fill] |
| header sorted | text `fg_secondary`, a 12 px arrow 4 px after the text (at the right end of a `num` column, the text before it); up for ascending, down for descending | [W components.css:538-563] |
| header sortable, rest | text `fg_tertiary`; no hover look | [W components.css:538-563] |
| cell tone | `fg_primary`, muted `fg_secondary`, warn `warn`, err `err`, dim `fg_tertiary`, strong `fg_bright` | [W components.css:538-563], [D162] |
| focus-visible, body | the list frame's ring on the cursor row, inside the slot | [D161] |
| focus-visible, header | a 1 px `fg_secondary` outline on the focused header cell | [D162] |
| disabled | inert, drawn at opacity 0.4, every node `disabled` | [derived: the list frame] |

### Transitions

The sorted header's text colour and arrow opacity fade over `selection` (`table::HEADER_FADE`); a change of direction snaps the glyph. Rows animate only what the list frame does while it is still: hover and selection fills (`hover`, `selection`). Rows never animate on a sort or a scroll ([L .claude/docs/animation-rules.md:145]). A deviation from the earlier "n/a" ([D40], [D164]).

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| sorted header on or off | header text colour, arrow opacity | selection | fast | linear | keep | [D40], [D162] |
| row hover, selection (still) | row fill | hover, selection | instant, fast | linear | keep | [D40], [D161] |

### Input and focus

- Wheel and swipe: the body registers no scroll sense while it has nothing to scroll, so the delta reaches the scroll area around it; one that overflows keeps it (D176).
- Pointer: a click on a row selects it, as the list frame (Meta toggles, Shift ranges and a drag ranges under `Multi`); a press on a control in a cell reaches the control and not the row. A click on a sortable header reports a sort: ascending on a new column, flipped on the sorted one; there is no unsorted third click (the app may clear `TableState.sort`) [D162].
- Keys: the body is the list frame's one stop (Up, Down, Home, End, PageUp, PageDown, Shift extends, Command+C copies). The sortable headers are one roving stop (Left, Right, Home, End move over the sortable ones without wrapping; Enter and Space sort), so a table with a sortable column has two stops, and one with none has one. A Control, Alt or Command chord on any of these stays a shortcut; focused and then disabled, the keys are taken and act on nothing.
- No cell focus: a control in a cell is its own tab stop and has its own keys; one that is not yet hardened for disabled keys or chords (stepper, slider, search picker, text fields) inherits the limit listed in `docs/architecture.md`, "Disabled keys".
- Layer: Base or Page, whatever the owner draws it on. Pointer capture: the list frame's drag selection.

### Retained state

- Transient: row hover, the header fades, the arrow's last direction.
- Persistent: the selection as a set of row keys (`Ui::table_selection`, the body list's `ListState`), the anchor and cursor, the scroll offset, and the sort (`Ui::table_state`, `TableState.sort`); all survive under `ui.keep_alive`. A sort keeps the selection because it is by key.
- App-owned: the rows and their order (the app sorts when `TableResponse.sort` is `Some`), the columns and their widths and order, the cell values and tones, `ListState::retain` for rows that left the data.
- Exit retention: no.

### Accessibility

- `Table` (name: `TableOptions.label`) holds a header `Row` of `ColumnHeader`s and, in the body's scroll area, a `Group` (the list frame's focusable node, the same name) of `Row`s of `Cell`s. A text cell's name is its full text, so an elided cell loses nothing; a control cell's children are the widget's nodes.
- A selected row has `selected`. `ColumnHeader.sort` is `Ascending` or `Descending` on the sorted column, `None` on a sortable one and absent otherwise; a sortable header has `Default` (sorts) and is focusable. A row has `Default` (selects), `Focus`, `ScrollIntoView` and `ShowMenu`.
- The semantic tree holds only the built rows. The `Group` between `Table` and `Row` follows from the list frame; AccessKit mapping waits for M10.
- Live region: none.

### Strings

None: column titles and cell values are app text.

### Gallery scenes

All at scale 2 on the `table` page (100,000 rows live; six rows or fewer in the scenes):

- `table-rows`: six rows, one selected, the pointer on another, settled.
- `table-sorted`: Tab, Right and Enter sort by Requests: the sorted header with its arrow and its focus outline, settled.
- `table-tones`: one column a tone, default, muted, warn, err, dim and strong, settled.
- `table-focus`: the body focused by Tab and two Down keys, the cursor row selected and ringed, settled. One focus exists per window, so the header's ring is in `table-sorted`.
- `table-controls`: a combo, a toggle, a badge and a button in cells, settled.
- `table-wrap`: wrapping route and note cells over a mono path that ellipsizes, settled.
- `table-disabled`: the table disabled with a selected row, settled.

### Painter primitives

- Existing: fill (row fills and hairlines), text, elide, glyph (the sort arrow), ring (the focus outlines). New: none.

### Acceptance

- Behaviour (`crates/lune-ui-core/tests/widget_table.rs`, scripted input, injected time): a click selects one row; the list keys move it; a press on a control cell reaches the control; the row stays lit over a control and is not lit under a dialog; header clicks, Enter and Space report ascending, descending, and a new column ascending; the headers rove over the sortable ones; two stops, or one with nothing sortable; the selection survives a re-sort and scrolling out of view; 100,000 rows build only the window; an access `Default` sorts a header and selects a row.
- Pixel probes at scale 2: row 56 px; header 56 px; hairline 2 px (none under the last row); the first cell inset 32 px, a middle cell 16 px; the `num` text ends at the cell's padded edge; the flat fill spans the slot at radius 0 while a plain list keeps its inset; tone colours by text key.
- Conformance: `HEADER_FADE` on the Full, Reduced and Off tiers; the table is idle at rest.
- Semantic node: as Accessibility, checked through the tree; the disabled and chord rules for the header keys and through the body.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_table.rs`: behaviour `a_click_selects_one_row_and_a_second_replaces_it`, `the_keys_move_the_selection_through_the_list_frame`, `a_press_on_a_control_cell_reaches_the_control_and_not_the_selection`, `the_row_stays_lit_while_the_pointer_is_over_a_control_in_it`, `a_dialog_over_the_table_lights_no_row`, `header_clicks_report_ascending_then_descending_and_another_column_ascending`, `enter_and_space_on_a_focused_header_sort`, `left_right_home_and_end_rove_over_the_sortable_headers_only`, `the_sorted_header_is_the_stop_and_a_table_has_two_stops`, `with_nothing_sortable_the_table_has_one_stop`, `selection_survives_a_re_sort_and_scrolling_out_of_view`, `a_hundred_thousand_rows_build_only_the_window`, `an_access_default_action_sorts_on_a_header_and_selects_on_a_row`; pixel probes `at_scale_2_the_header_and_rows_are_56_the_hairlines_2_and_the_insets_32_and_16`, `the_last_row_has_no_hairline`, `mono_and_num_cells_use_the_mono_face_and_num_ends_at_the_right_edge`, `the_flat_fill_spans_the_slot_at_radius_0_and_a_plain_list_keeps_its_inset`, `each_tone_has_its_colour_and_a_selected_rows_text_is_bright_whatever_the_tone`; conformance `the_header_fades_over_fast_and_the_reduced_and_off_tiers_agree_at_rest`, `a_table_at_rest_is_idle_and_stays_idle_with_a_sort_and_a_selection`; node `the_tree_is_a_table_with_a_header_row_of_column_headers_and_a_group_of_rows_of_cells`, `a_control_cells_nodes_nest_under_its_cell`; disabled `a_header_focused_then_disabled_takes_its_keys_and_changes_nothing`, `a_body_focused_then_disabled_takes_its_keys_and_changes_nothing`, `a_chord_on_the_tables_keys_stays_a_shortcut`, `a_disabled_table_is_inert_and_its_nodes_are_disabled`.

### Build

- Milestone: M8 (steps 1 and 2, D160 to D163).
- Depends on: layout.virtualized-list, core.painter, core.focus, core.ui, infra.accessibility, widget.combo, widget.button, widget.toggle, widget.badge
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_table.rs::a_click_selects_one_row_and_a_second_replaces_it, crates/lune-ui-core/tests/widget_table.rs::a_press_on_a_control_cell_reaches_the_control_and_not_the_selection, crates/lune-ui-core/tests/widget_table.rs::header_clicks_report_ascending_then_descending_and_another_column_ascending, crates/lune-ui-core/tests/widget_table.rs::left_right_home_and_end_rove_over_the_sortable_headers_only, crates/lune-ui-core/tests/widget_table.rs::selection_survives_a_re_sort_and_scrolling_out_of_view, crates/lune-ui-core/tests/widget_table.rs::a_hundred_thousand_rows_build_only_the_window, crates/lune-ui-core/tests/widget_table.rs::at_scale_2_the_header_and_rows_are_56_the_hairlines_2_and_the_insets_32_and_16, crates/lune-ui-core/tests/widget_table.rs::the_header_fades_over_fast_and_the_reduced_and_off_tiers_agree_at_rest, crates/lune-ui-core/tests/widget_table.rs::the_tree_is_a_table_with_a_header_row_of_column_headers_and_a_group_of_rows_of_cells, crates/lune-ui-core/tests/widget_table.rs::a_chord_on_the_tables_keys_stays_a_shortcut`; page `table`; unverified here: none (limit: no column resize or reorder, Q15).

## Open questions

- Answered (D162): tone colours: Wane's, muted `fg_secondary`, warn, err, dim `fg_tertiary`, strong `fg_bright`; keyboard row navigation: the list frame's keys, with the header as a second roving stop; toolkit or app: a toolkit component, the app passing columns and a cell closure.
- Still open, a limit (Q15, D163: deferred, reversible; D4): column resize and reorder. Wane calls them requirements, not mocked, and no screen shows them; `TableColumn.width` and the column order are the app's arguments, so both gestures are additive later with no API break.
- Native rendering: plain drawing; the page's bench runs scroll, select and sort over 100,000 rows.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.table`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, local-models, overview, pricing, requests, routes, sessions, upstreams.

Built in M8 (D161 to D163, D174). Deviations and limits: a wrapping cell ignores `num` and is left-aligned; the header's focus ring snaps (`HEADER_FADE` fades the text colour and the arrow); there is no horizontal scroll (`Fill` columns shrink, then the last columns clip); a `Measured` height after a sort takes effect on the next frame; rows out of view are not built, so a control in one cannot keep the focus; no column resize or reorder (D163). The table stays `in-progress` for M11's done standard.

---
id: data.kv-list
title: Key-value list
area: data
status: done
consumers: wane
source: crates/lune-ui-core/src/widgets/kv_list.rs
owner: —
summary: Aligned key and value pairs: facts, paths, ids, figures
---

## What it is

Aligned key and value pairs: facts, paths, ids, figures. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static; values are selectable for copy (ids, paths).

## Needs

- Wane — Variants: default (key column sized by the longest key); stacked (two columns, key 11 over a 13 mono value); mono and wrap values. (R-data-2)
- Wane — Sizes: Grid with a max-content key column and 16 px gap; rows 24; key 13 tertiary, value 13 primary with ellipsis; wrap value 20 line height with 2 px padding; stacked key 11/16. (R-data-2)
- Wane — Behaviour: Static; values are selectable for copy (ids, paths). (R-data-2)

## Existing code

`crates/lune-ui-core/src/widgets/kv_list.rs` (`KvOptions`, `Ui::kv_list`) and `crates/lune-ui-core/src/widgets/selectable.rs` (`Ui::selectable_text`), M6. Gallery page `apps/gallery/src/pages/kv_list.rs`; tests `crates/lune-ui-core/tests/widget_kv_list.rs`. Design source (not code moved): Wane `docs/ui/mocks/css/components.css` (data.kv-list); no lune-ui source.

## Spec

### Evidence

Pinned: Wane 69931d6, this repo fa2958b. Read: Wane `docs/ui/components.md:613-626`; `crates/lune-ui-core/src/theme.rs:55-84` (`fg_secondary`, `fg_tertiary`, `font_s`, `font_base`);.
Conflicts: none. Lunegit has no kv-list source read; not used for values.

### Anatomy

- Grid: key column max-content; 16 px gap between columns [W docs/ui/components.md:613-626].
- Row: 24 high [W docs/ui/components.md:613-626].
- Key: 13, `fg_tertiary` [W docs/ui/components.md:613-626], [R crates/lune-ui-core/src/theme.rs:58,83].
- Value: 13, `fg_primary`, ellipsis when long [W docs/ui/components.md:613-626], [R crates/lune-ui-core/src/theme.rs:56].
- Wrap value: 20 line height with 2 px padding [W docs/ui/components.md:613-626].
- Stacked: key 11 over value 13 mono, two columns [W docs/ui/components.md:613-626], [R crates/lune-ui-core/src/theme.rs:82].

### States

| state | look | cite |
|---|---|---|
| rest (the only interaction state) | key and value as above | [W docs/ui/components.md:613-626] |
| value selected for copy | `text_selection` fill under the text, while the value has focus [D125] | [D125] |

### Transitions

n/a — static (Wane: "Static"); a value change follows the app's data without motion.

### Input and focus

- Pointer: values accept selection for copy [W docs/ui/components.md:613-626]; a press focuses the value and places the caret, a drag, a double press and Shift extend it. The rows, the keys and the space beside a value take no input: the hit region is the drawn text. An elided value selects as a whole: any press selects all of it and Copy writes all of the full text [D125].
- Keyboard: the platform's copy and select-all on the focused value; the caret keys move the selection of a value shown in full; an elided value ignores them, so a partial selection of it cannot exist.
- Tab stop: no; the values are not controls [derived: static text, W docs/ui/components.md:613-626]. With `KvOptions::copy` each row has one: its copy control, a button of 21 px at the row's right end, shown while the pointer is on the row or focus is on the row's value or on the control, and a Tab stop even while hidden. A click, Enter or Space puts that row's value on the clipboard ([D275]); the value's room narrows by the control and a 4 px gap.
- Layer: whatever layer the owner draws it on [derived: a container].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: the text selection (while a value is selected).
- Persistent: with `copy`, per row, when the check mark began and the value it copied (1,500 ms, `COPY_FEEDBACK_MS`); none otherwise.
- App-owned: the keys, the values, the column mode (default, stacked, mono, wrap) [W docs/ui/components.md:613-626].
- Exit retention: no.

### Accessibility

- Each pair is a `group` named by its key with a `static-text` key and a `static-text` value [infra.accessibility]; the list itself has no node of its own.
- Values are selectable text: name = key, value = the full text, with the selection [infra.accessibility].
- Live region: none, except a polite `status` "Copied" while a row's check mark shows (`KvOptions::copy`). The control is a `button` named `Copy <key>`.

### Strings

`luneui.kv-list.copy` ("Copy {0}", the key as the argument) and `luneui.kv-list.copied` ("Copied"), used only with `KvOptions::copy`. Keys and values are app text.

### Gallery scenes

- `kv-list-default` (new): six pairs with the longest key sizing the column, settled.
- `kv-list-stacked` (new): stacked two-column variant, settled.
- `kv-list-wrap` (new): a wrapped value over two lines, settled.
- `kv-list-selected` (new): the commit value selected whole, focused by a press, settled.
- `kv-list-copy` (new, [D275]): `KvOptions::copy` on, the pointer on the second row, whose copy control shows at its right end; the others stay hidden.

### Painter primitives

- Existing: text, clip-rect (the ellipsis). [R crates/lune-ui-core/src/paint.rs:274,167]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): a value is selectable and copies its text; rows take no clicks. Tests `widget_kv_list.rs` (a drag then Copy writes the substring; a press on an elided value then Copy writes the whole; Tab skips every value; a press outside a value does nothing).
- Pixel probes at scale 2: row 48 px; key-value gap 32 px; key text at 26 px font size; the key column equals the longest key's measured width.
- Conformance: none (no motion).
- Semantic node: `group` per pair; key and value `static-text`.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_kv_list.rs`: behaviour `a_drag_across_a_value_then_copy_writes_the_substring`, `a_press_on_an_elided_value_selects_it_whole_and_copy_writes_all_of_it`, `a_value_is_not_a_tab_stop`, `a_press_outside_a_value_does_nothing`, `the_selection_draws_while_focused_and_a_focus_loss_clears_it`; pixel probes `at_scale_2_the_row_is_48_px_and_the_gap_32_px`, `the_key_column_is_the_longest_keys_measured_width_and_the_gap_is_16`, `rows_are_24_high_with_the_text_on_a_20_px_line_2_px_down`, `key_and_value_colours_and_the_mono_option`; `a_long_value_is_elided_to_the_value_column_and_a_key_never_is`; node `each_pair_is_a_group_over_a_key_and_a_selectable_value`; no motion `the_list_is_static_and_settles`; disabled `a_value_focused_then_disabled_takes_its_keys_and_changes_nothing`.

- Copy control ([D275]), tests in `widget_kv_list.rs`: `copy_off_draws_and_registers_what_it_did`, `a_hover_shows_one_control_on_its_own_row`, `a_click_writes_that_rows_value_and_no_other`, `the_name_carries_the_key_and_the_control_is_a_button`, `tab_reaches_a_hidden_control_and_shows_it`, `disabled_the_controls_are_inert`, `the_value_gives_up_the_controls_room`.

### Build

- Milestone: M6.
- Depends on: core.painter, core.theme, infra.accessibility.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_kv_list.rs::a_drag_across_a_value_then_copy_writes_the_substring, crates/lune-ui-core/tests/widget_kv_list.rs::a_press_on_an_elided_value_selects_it_whole_and_copy_writes_all_of_it, crates/lune-ui-core/tests/widget_kv_list.rs::a_value_is_not_a_tab_stop, crates/lune-ui-core/tests/widget_kv_list.rs::at_scale_2_the_row_is_48_px_and_the_gap_32_px, crates/lune-ui-core/tests/widget_kv_list.rs::each_pair_is_a_group_over_a_key_and_a_selectable_value, crates/lune-ui-core/tests/widget_kv_list.rs::the_list_is_static_and_settles`; page `kv-list`; unverified here: the OS clipboard the copy reaches (`platform.clipboard`).

## Open questions

- None. The selection look (`text_selection`) and a toolkit component are settled by [D125]. The copy control is an option, off by default ([D275]). Column sizing measures the longest key with `Painter::measure`.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.kv-list`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, requests, routes, sessions, settings, tray, upstreams.

---
id: widget.search-picker
title: Search picker
area: widget
status: done
consumers: lunegit
source: crates/lune-ui-core/src/widgets/search_picker.rs
owner: —
summary: Search picker: a field-style or link-style trigger that opens a searchable list; Escape closes, Enter picks the first match
---

## What it is

`search-picker`: a trigger that opens a popup with a search box over a filtered list. Two Lunegit variants: a field trigger in filter-bar chrome (`FieldSearchPicker.axaml`) and a link trigger with an underlined text (`LinkSearchPicker.axaml`). Both share `SearchPickerBase` [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:31].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-2); popup (R-overlay-4 through `overlay.popover`).
- Keyboard: Escape and Enter in the search box; Tab stop (R-widget-3); semantic node (R-infra-4).

## Existing code

`crates/lune-ui-core/src/widgets/search_picker.rs` (`Ui::search_picker`, `PickerOptions`, `Trigger`, `matches`), on `Ui::popover`, `Ui::filter_bar` and the scroll area.

`src/Lunegit.Desktop/Controls/SearchPickerBase.cs`, `src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml`, `src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `src/Lunegit.Desktop/Controls/SearchPickerBase.cs:31-53,143,154,160-163,302-318`; `src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:18-53`; `src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:8-56`.
Wane has no search-picker component; the Wane combo (`widget.combo`) is not searchable. Lunegit alone is the source.

### Anatomy

- Field trigger: `Border.filter-bar` chrome, a 24 px row, text 13 px `ForegroundPrimary` with character ellipsis, search icon at the right with left margin 8 [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:35-50].
- Link trigger: leading 14 px icon, spacing 6, text 13 px `ForegroundSecondary` with a 1 px underline stroked dashed `1,2` [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:16-31].
- Popup: the glass surface (`menu_glass` frost over the blurred scene, `menu_popup_border` ring, radius 9; Lunegit's `BackdropDark` fill and `SurfaceBorder` ring at radius 8 are replaced, D8), padding 6, min width 220, max height 320 [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:42-48].
- Search bar at the top of the popup: radius 4, `SurfaceInput`, padding 8 horizontal and 6 vertical, bottom margin 6 [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:52-56].
- Popup width tracks the field trigger [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:53]. Its height follows the filtered count, from the frame's count before the popup is drawn, up to 320 and the window; a popup flipped above the trigger at open stays above it while the list narrows.
- Rows: 32 high, radius 4, padding 8, `surface_selected` on the selected row, else `surface_hover` under the pointer; an overlong text ends in an ellipsis [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:99-112].
- Placeholder: `SearchPlaceholder`, default empty [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:50].

### States

| state | look | cite |
|---|---|---|
| field, rest | filter-bar surface, value text | [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:18-50] |
| field, hover | no change: the trigger's pointerover clears its background | [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:26-29] |
| link, rest | secondary text with dashed underline | [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:19-31] |
| link, hover | text and underline `fg_primary` | [D124] |
| link, pressed | as hover | [D124] |
| open | popup shown; trigger state unchanged | [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:143], [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:38-41] |
| focus-visible | link: `RingKind::Button` [D124]; field: the same line at the trigger's radius, `RingKind::Edge`, so keyboard focus shows on both | [D124], [core.focus] |
| disabled | opacity 0.4, inert | [widget.button], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

n/a — Lunegit styles no motion on either trigger; the popup's open and close motion belongs to `overlay.popover`.

### Input and focus

- Pointer: a trigger click toggles the popup [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:143]; light dismiss on an outside click [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:41].
- Keyboard on the trigger: Enter and Space toggle the popup.
- Keyboard in the search box: Escape closes; Enter selects the first filtered item and closes [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:302-318]; with no match it does nothing and the popup stays. The arrows move the caret: there is no row highlight [D124].
- Filter: trimmed, case-insensitive contains (Unicode `to_lowercase`); an empty or all-space search shows all [D124]. The filter runs each open frame over all items.
- Tab stop: the trigger; in the popup the box, and its clear button while it shows; no row is one [core.focus].
- Layer: trigger `Base`; popup `Popup` (`overlay.popover`).
- Focus scope: the popup pushes a scope on open and closing restores focus to the trigger [overlay.popover].
- Dismiss: Escape, outside click and focus loss act on the open popup only [overlay.popover]. Closing the popup by any route syncs the open flag [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:160-163].
- Open flag: the popover is called exactly while `open` is true; a dismissal clears it on the frame that reports it, so that frame is the last that calls the popover. Paths: trigger click, Enter or Space on the trigger, accessibility default, expand and collapse, Escape, outside press, a press on the open trigger, focus loss, a row click, Enter in the box, disabling the picker (it does not reopen on enabling), a keep-alive gap (the Ui closed it; it does not come back). Items replaced while open keep it open.
- A press on the open trigger is an outside press: the Ui consumes it, so it closes once and reaches nothing [D77].
- Every open is a new overlay, so an open on the frame after a close starts clean: empty search, focus in the box.
- Every close gives focus back to the trigger, also when an accessibility action opened the popup with focus elsewhere; the ring stays as it was. The exception is a close caused by focus moving to another widget (`request_focus`, an accessibility focus action): focus stays on that widget, by `core.focus`'s rule that a closing overlay hands focus back only when focus is in it or gone.
- A `selected` out of range reads as none and is left as it is. A pick writes an index into `items`, not the filtered list; picking the selected row reports `picked` without `changed`.

### Retained state

- Transient: hover, press [core.ui].
- Persistent: the open flag and the search text, held by the picker. The search is cleared on open, not on close, so the closing popup replays what it drew [D124].
- App-owned: `ItemsSource`, `SelectedItem`, `DisplayMemberBinding`, `Icon`, `SearchPlaceholder` (Lunegit properties [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:35-53]).
- Exit retention: no.

### Accessibility

- Trigger: role `combo-box`; name = the owner's label; value = the selected item's display text; states expanded (while open), focusable, focused, disabled; actions `expand`, `collapse`, `default` [infra.accessibility].
- Search box: role `text-input`; name = `SearchPlaceholder`, or the trigger's name without one.
- List: role `list` of `list-item`s, named like the trigger; the visible rows of the filtered items are the list; a row's `default` action picks it [infra.accessibility].

### Strings

None: placeholder and item text are owner data (`SearchPlaceholder`, `DisplayMemberBinding`).

### Gallery scenes

- `search-picker-field`: field trigger with a value, settled.
- `search-picker-link`: link trigger with a value, settled.
- `search-picker-focus-field`: Tab on the field trigger: the `RingKind::Edge` ring at the trigger's radius, settled.
- `search-picker-focus-link`: Tab on the link trigger: the `RingKind::Button` ring, settled.
- `search-picker-disabled`: both triggers disabled (0.4 opacity) with the pointer on the link and a click: no hover, no popup, settled.
- `search-picker-open`: link trigger with the popup open, search text, filtered list; settled.

### Painter primitives

- Existing: fill, ring, text, icon, clip-rect
- New: dash (the dashed underline, `src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:30`)

### Acceptance

- Scripted: trigger click opens and closes once; Escape closes; Enter picks the first match and closes; an outside click closes the popup.
- Pixel probes at scale 2: popup radius 16 px; search bar radius 8 px; dash 1 px stroke with a 2 px / 4 px pattern.
- Conformance: n/a — no motion on the trigger.
- Semantic node: role combo-box; expanded follows the open flag.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_search_picker.rs`: behaviour `a_trigger_click_opens_once_with_an_empty_search_and_focus_in_the_box`, `a_press_on_the_open_trigger_closes_once_and_does_not_reopen`, `escape_closes`, `enter_in_the_box_picks_the_first_match_as_an_index_into_items_and_closes`, `an_outside_press_closes_and_reaches_nothing`, `window_focus_loss_closes`; pixel probes at scale 2 `at_scale_2_the_popup_radius_is_18_the_search_bar_8_and_the_dash_is_1_px_in_a_2_4_pattern` (the 2 on, 4 off pattern is `paint_vector.rs::dash_one_two_at_scale_two_is_two_on_four_off`), `the_link_trigger_is_secondary_primary_on_hover_with_a_dashed_underline_as_wide_as_the_text`, `the_field_trigger_shows_the_value_on_the_surface_with_an_ellipsis`, `rows_show_selection_and_hover_at_radius_4`; no motion on the trigger `the_rest_trigger_settles_to_idle_and_the_ring_shows_on_the_keyboard`, `reduced_motion_opens_and_closes_all_the_same`; node `the_nodes_are_a_combo_box_a_text_input_and_a_list_of_items`.

### Build

- Milestone: M6
- Depends on: overlay.popover, widget.filter-bar, text.text-edit, core.focus, infra.accessibility
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_search_picker.rs::a_trigger_click_opens_once_with_an_empty_search_and_focus_in_the_box, crates/lune-ui-core/tests/widget_search_picker.rs::escape_closes, crates/lune-ui-core/tests/widget_search_picker.rs::enter_in_the_box_picks_the_first_match_as_an_index_into_items_and_closes, crates/lune-ui-core/tests/widget_search_picker.rs::an_outside_press_closes_and_reaches_nothing, crates/lune-ui-core/tests/widget_search_picker.rs::at_scale_2_the_popup_radius_is_18_the_search_bar_8_and_the_dash_is_1_px_in_a_2_4_pattern, crates/lune-ui-core/tests/widget_search_picker.rs::the_nodes_are_a_combo_box_a_text_input_and_a_list_of_items`; page `search-picker`; unverified here: real typing with an input method (Luneterm's M9 run, D15).

## Open questions

- Resolved [D124] `behaviour`: open clears the search and focuses the box.
- Resolved [D124] `variant`: link hover text `fg_primary`, ring `RingKind::Button`.

## Notes for the planner

Lunegit-only; consumers lunegit. Two triggers share one base; the spec keeps both.

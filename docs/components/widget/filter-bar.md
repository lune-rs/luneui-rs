---
id: widget.filter-bar
title: Filter bar
area: widget
status: done
consumers: lunegit
source: crates/lune-ui-core/src/widgets/filter_bar.rs
owner: —
summary: Filter bar: search field in an inset surface with a 14 px search icon, a clear button when text is present, and no focus ring
---

## What it is

`filter-bar`: input container above file and branch lists. A 14 px search icon at the left, a transparent text box with a watermark, and a clear button (18 px) that shows when text is present [L src/Lunegit.Desktop/Controls/FilterBar.axaml:8-26].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-2).
- Text entry and caret (`text.text-edit`); Tab stop (R-widget-3); semantic node (R-infra-4).

## Existing code

`crates/lune-ui-core/src/widgets/filter_bar.rs` (`Ui::filter_bar`, `FilterOptions`), on `FieldChrome::Bare` and `Ui::icon_button`.

`src/Lunegit.Desktop/Controls/FilterBar.axaml` (47 lines, one user control ; the watermark is bound from the owner at line 22; the code-behind `FilterBar.axaml.cs` was not read). Surface styles: `src/Lunegit.Desktop/App.axaml:302-309` (`Border.filter-bar`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `src/Lunegit.Desktop/Controls/FilterBar.axaml:1-47`; `src/Lunegit.Desktop/App.axaml:302-309`; Wane `docs/ui/components.md:347-359` (text field search variant).
Conflicts: (1) Icon: Lunegit draws a 14 px icon [L src/Lunegit.Desktop/Controls/FilterBar.axaml:10]; Wane's search variant is 16 px at the left with padding-left 33 [W docs/ui/components.md:350,352]; Lunegit wins for this control. (2) Radius: the inline `CornerRadius="4"` [L src/Lunegit.Desktop/Controls/FilterBar.axaml:8] overrides the style's 6 [L src/Lunegit.Desktop/App.axaml:303]; the inline 4 wins. Wane's field radius is 6 [W docs/ui/components.md:352]; Lunegit wins. (3) Focus: Lunegit removes the text box border on focus and draws no ring [L src/Lunegit.Desktop/Controls/FilterBar.axaml:33-37]; Wane draws an accent border [W docs/ui/components.md:351]; Lunegit wins for the filter bar.

### Anatomy

- Surface: fill `surface_input` [L src/Lunegit.Desktop/Controls/FilterBar.axaml:8]; radius 4; padding 8 horizontal, 6 vertical; top margin 4 [L src/Lunegit.Desktop/Controls/FilterBar.axaml:8].
- Search icon: 14 px, `ForegroundDim`, left, right margin 8 [L src/Lunegit.Desktop/Controls/FilterBar.axaml:10-12].
- Text: UI font at `FontSizeBase` 13 [L src/Lunegit.Desktop/Controls/FilterBar.axaml:24], `ForegroundPrimary`; caret `ForegroundTertiary` [L src/Lunegit.Desktop/Controls/FilterBar.axaml:26]; placeholder = `Watermark` [L src/Lunegit.Desktop/Controls/FilterBar.axaml:22].
- Clear button: 18 x 18, shown while the text is non-empty, close icon 12 px `ForegroundDim` [L src/Lunegit.Desktop/Controls/FilterBar.axaml:13-19].
- Text box chrome: transparent background and border in every pseudo-state [L src/Lunegit.Desktop/Controls/FilterBar.axaml:28-41].

### States

| state | look | cite |
|---|---|---|
| rest, empty | surface, watermark in tertiary, no clear button | [L src/Lunegit.Desktop/Controls/FilterBar.axaml:8,13-22] |
| rest, with text | as rest; clear button visible | [L src/Lunegit.Desktop/Controls/FilterBar.axaml:16] |
| hover | no change in the surface | [L src/Lunegit.Desktop/Controls/FilterBar.axaml:38-41] |
| focus | caret only; no border or ring | [L src/Lunegit.Desktop/Controls/FilterBar.axaml:33-37,26] |
| disabled | opacity 0.4, inert | [widget.button], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

n/a — Lunegit has no motion selector for the filter bar and the clear button appears without one (`src/Lunegit.Desktop/Themes/Animations.axaml` names no filter-bar selector); Wane's text field has none [W docs/ui/components.md:353].

### Input and focus

- Pointer: click focuses the text box; a press on the surface outside it (the padding, the icon) focuses it too; click on the clear button clears the text [L src/Lunegit.Desktop/Controls/FilterBar.axaml:13-17].
- Clear: empties the text, marks the response `changed` and leaves focus on the field. The button sits after the field in the Tab order, so the clear lands after the field drew: the frame reports the change and the frame the input asks for after it shows the empty field [D81]. While the field composes a clear does nothing.
- Keyboard: typing edits the text [text.text-edit]; Tab stops: the field, then the clear button while it shows, Enter and Space clear [core.focus]. `FilterOptions::clear_unfocusable` takes the button out of focus and the Tab order, for a composite whose keys go through the field (the command palette).
- Layer: `Base`; no overlay, no focus scope.

### Retained state

- Transient: hover [core.ui]. The caret does not blink: it is solid [D103].
- Persistent: the text and the caret position, owned by `text.text-edit`.
- App-owned: `Watermark` (passed by the owner).
- Exit retention: no.

### Accessibility

- Role `text-input`; name = the owner's label; value = the text; states focusable, focused, disabled; action `set-value`; the clear button is a `button` named `luneui.text-field.clear` [infra.accessibility].

### Strings

`luneui.text-field.clear` names the clear button (`widget.text-field`). `Watermark` is passed by the owner (`src/Lunegit.Desktop/Controls/FilterBar.axaml:22`).

### Gallery scenes

- `filter-bar-states`: empty with watermark, with text and clear button, focused caret, disabled; settled.

### Painter primitives

- Existing: fill, icon, text
- New: none

### Acceptance

- Scripted: typing sets the text; the clear button clears it and hides; a disabled filter bar takes no text.
- Pixel probes at scale 2: radius 8 px; icon 28 px at the left; clear button 36 px.
- Conformance: n/a — no transition.
- Semantic node: role text-input; value follows the text.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_filter_bar.rs`: `typing_sets_the_text_and_marks_it_changed`, `the_clear_button_shows_only_while_there_is_text`, `a_click_on_clear_empties_the_text_hides_it_and_leaves_focus_on_the_field`, `the_tab_order_is_the_field_then_clear_and_enter_clears`, `the_default_action_on_clear_clears`, `a_disabled_bar_takes_no_text_no_clear_and_no_focus`, pixel probes `pixel_probes_at_scale_2`, `focus_shows_the_caret_only_no_border_and_no_ring`, node `the_node_is_a_text_input_named_by_the_label_or_the_placeholder`.

### Build

- Milestone: M6
- Depends on: text.text-edit, widget.button, core.painter, text.shaping
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_filter_bar.rs::typing_sets_the_text_and_marks_it_changed, crates/lune-ui-core/tests/widget_filter_bar.rs::a_click_on_clear_empties_the_text_hides_it_and_leaves_focus_on_the_field, crates/lune-ui-core/tests/widget_filter_bar.rs::a_disabled_bar_takes_no_text_no_clear_and_no_focus, crates/lune-ui-core/tests/widget_filter_bar.rs::pixel_probes_at_scale_2, crates/lune-ui-core/tests/widget_filter_bar.rs::the_node_is_a_text_input_named_by_the_label_or_the_placeholder`; page `filter-bar`; unverified here: real typing with an input method (Luneterm's M9 run, D15).

## Open questions

- None.

## Notes for the planner

Wane's text field search variant (`widget.text-field`) is the nearest Wane look; this entry keeps Lunegit's drawn values where they differ.

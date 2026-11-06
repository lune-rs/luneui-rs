---
id: widget.list-row-badge
title: List rows with status badges
area: widget
status: done
consumers: lunegit, wane
source: crates/lune-ui-core/src/widgets/list_row.rs
owner: —
summary: Master list row: name, value and sub-line with a badge in the head; rest transparent, hover surface-hover, selected surface-selected
---

## What it is

`list-row-badge`: master list row with a head (name and a badge), a value, and a sub-line or meta. Rest is transparent; hover is `surface_hover`; selected is `surface_selected` with a bright name, never a border or accent bar. Click selects and swaps the detail pane; Up and Down move the selection [W docs/ui/components.md:477-479].

## Needs

- Wane controls the toolkit lacks (R-widget-4); list selection and keys (R-widget-3); semantic node (R-infra-4).

## Existing code

`crates/lune-ui-core/src/widgets/list_row.rs` (M6): `ListRow`, `Ui::list_row`, `paint`, `height`, `SIDE_PANEL_PADDING`, `MASTER_PADDING`. Gallery page `apps/gallery/src/pages/list_row.rs`; tests `crates/lune-ui-core/tests/widget_list_row.rs`. Lunegit side-panel list styles (design source): `src/Lunegit.Desktop/App.axaml:352-365,687-698`. `src/Lunegit.Desktop/ViewModels/SidePanelRow.cs` is named in the survey and was not read. Luneterm has no list-row widget.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:473-485`; `docs/ui/mocks/css/tokens.css:13-16,20-23`; Lunegit `src/Lunegit.Desktop/App.axaml:209,352-365,687-698`, `src/Lunegit.Desktop/Themes/Animations.axaml:123-125`.
Conflicts: (1) Padding: Lunegit's side-panel item pads 8 horizontal and 6 vertical [L src/Lunegit.Desktop/App.axaml:360]; Wane pads 8 12 [W docs/ui/components.md:478]. Lunegit draws this list, so Lunegit wins for the side-panel list. The Wane 12 applies to master lists outside the side panel and is an Open question. (2) Selection: Lunegit's list item is transparent and the `Border.side-panel-item-active` carries the fill [L src/Lunegit.Desktop/App.axaml:687-695,357-358]; Wane's selected fill is `--surface-selected` [W docs/ui/components.md:477]; they agree on the fill. (3) Radius 6 agrees [L src/Lunegit.Desktop/App.axaml:209].

### Anatomy

- Row radius 6 [L src/Lunegit.Desktop/App.axaml:209], rows 2 apart and gap 4 [W docs/ui/components.md:478].
- Padding 8 horizontal, 6 vertical in the side panel [L src/Lunegit.Desktop/App.axaml:360].
- Name 13/20 Medium [W docs/ui/components.md:478]; Medium weight as the gallery draws it [R apps/gallery/src/scenes.rs:328].
- Layout (the Lunegit source gives no line plan, so the mock is the source [W docs/ui/mocks/css/components.css, `c-row` sessions and upstreams]; visual batch 3): a head line of the optional dot, the name (elided), the badge and the value (mono 13, `fg_secondary`) at the far right edge, 8 px between them; then the sub-line (mono 11/16, tertiary); then the meta (11/16, tertiary) at the left; 4 px between lines. With both a badge and a value the value takes the right edge, so figures line up down a list (taste call; the mock has no row with both). The name stays `fg_secondary` at rest, cited to Lunegit [L src/Lunegit.Desktop/App.axaml:687-690] (hard rule 8); the value is `fg_secondary`, so it is not brighter than the name. A full row is 74 high with the side-panel padding (2 + 12 + 20 + 4 + 16 + 4 + 16). The padding is the pair (horizontal, vertical): `SIDE_PANEL_PADDING` (8, 6), `MASTER_PADDING` (12, 6), inside the list's 4 px by 1 px highlight inset. `height(row, padding)` is the slot height for `Heights::Fixed`.
- Value RznMono 13; sub-line RznMono 11/16 tertiary; meta 11/16 tertiary [W docs/ui/components.md:478].
- Badge in the head: `widget.badge`; a lead dot: `widget.status-dot` [W docs/ui/components.md:483], 8 px before the name (the dot's own gap, `status_dot::GAP`; the mock's dot-to-name distance is 16 px at scale 2).

### States

| state | look | cite |
|---|---|---|
| rest | transparent fill; secondary name | [L src/Lunegit.Desktop/App.axaml:687-690], [W docs/ui/components.md:477] |
| hover | fill `surface_hover` | [L src/Lunegit.Desktop/App.axaml:352-353], [W docs/ui/components.md:477] |
| selected | fill `surface_selected`; bright name; no bar | [L src/Lunegit.Desktop/App.axaml:357-358], [W docs/ui/components.md:477] |
| selected + hover | fill `surface_active` | [L src/Lunegit.Desktop/App.axaml:363-364] |
| focus-visible | 1 px outline, offset 2 | [widget.button] (Wane gives no focus value for rows) |
| disabled | opacity 0.4, inert | [widget.button], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | background | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:123-125], [L .claude/docs/animation-rules.md:102] |
| unselected ↔ selected | background and name | selection | fast | linear | keep | [L .claude/docs/animation-rules.md:110], [D40] |
| focus-visible on/off | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |

Never animated: the rows of a virtualized list on scroll [L .claude/docs/animation-rules.md:145].

### Input and focus

- Pointer: click selects and swaps the detail pane [W docs/ui/components.md:479]; hover over the row.
- Keyboard: Up and Down move the selection [W docs/ui/components.md:479]; Enter selects.
- Tab stop: the list is one stop; the selected row takes it [core.focus].
- Layer: `Base`; no overlay, no focus scope.
- Pointer capture while pressed [core.input].

### Retained state

- Transient: hover, press, focus ring [core.ui].
- Persistent: selection as a set of row keys, scroll offset (owned by the list container) [core.ui].
- App-owned: items, the detail pane content, the selected key when the owner sets it.
- Exit retention: no.

### Accessibility

- Role `list-item` inside a `list`, made by the virtual list (which owns selected, focused, disabled and the actions `default`, `focus`, `scroll-into-view`, `show-menu`); the row adds a `static-text` child valued by the name and, with a value, one named by the name and valued by the value text, which is how the list names its row [infra.accessibility], [W docs/ui/components.md:479].

### Strings

None: names, values and meta are app data.

### Gallery scenes

- `list-row-badge-states`: rest, hover, selected, selected + hover, focus-visible, disabled; settled.
- `list-row-badge-hover-mid`: rest → hover, t = 60 ms (half of `instant`).
- `list-row-badge-select-mid`: unselected → selected, t = 90 ms (half of `fast`).
- `list-row-badge-focus-mid`: focus-visible on, t = 90 ms (half of `fast`). The virtual list draws its ring at full opacity at once, so the frame at 90 ms is settled.

### Painter primitives

- Existing: fill, text
- New: none

### Acceptance

- Scripted: Up and Down move the selection through the rows; click selects once and reports the key.
- Pixel probes at scale 2: row radius 12 px; padding 16 horizontal, 12 vertical in the side-panel form; selected fill = `surface_selected`.
- Conformance: hover background over 60 Hz samples is monotonic, ends exact, settles within 120 ms + 1 frame.
- Semantic node: role list-item; selected follows the selection key; injected `AccessAction` default selects.
- Tests, bullet by bullet. The row draws the content; the list owns selection, keys and the hover fill (its Acceptance is `layout.virtualized-list`'s): Up and Down, click and the default action are `widget_virtual_list.rs::up_and_down_move_the_cursor_and_the_selection`, `a_click_selects_one_row_and_sets_the_anchor`, `an_access_default_action_selects_the_row`; the hover fill conforms in `the_two_transitions_conform_at_every_tier`. The row itself (`crates/lune-ui-core/tests/widget_list_row.rs`): pixel probes `at_scale_2_the_padding_is_16_by_12_px_the_radius_12_and_selected_is_surface_selected`, `the_height_is_the_inset_the_padding_and_the_lines`, `the_two_lists_pad_8_and_12_horizontally_and_6_vertically` (`conflict-padding`, D127); the name fade `the_name_is_secondary_at_rest_and_bright_once_selected`, `selecting_fades_the_name_at_fast_and_ends_exact`, `nothing_animates_while_the_list_scrolls`; content `the_head_has_the_dot_the_name_the_badge_and_the_value_then_the_sub_line_and_the_meta`, `a_long_name_value_and_sub_are_elided_never_wrapped`; node `the_row_adds_its_name_and_value_under_the_lists_list_item`; `the_public_paint_draws_what_the_list_row_draws_at_rest`.

### Build

- Milestone: M6
- Depends on: widget.badge, widget.status-dot, core.focus, core.ui, infra.accessibility, motion.transitions
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_list_row.rs::at_scale_2_the_padding_is_16_by_12_px_the_radius_12_and_selected_is_surface_selected, crates/lune-ui-core/tests/widget_list_row.rs::selecting_fades_the_name_at_fast_and_ends_exact, crates/lune-ui-core/tests/widget_list_row.rs::nothing_animates_while_the_list_scrolls, crates/lune-ui-core/tests/widget_list_row.rs::the_row_adds_its_name_and_value_under_the_lists_list_item, crates/lune-ui-core/tests/widget_virtual_list.rs::up_and_down_move_the_cursor_and_the_selection, crates/lune-ui-core/tests/widget_virtual_list.rs::an_access_default_action_selects_the_row, crates/lune-ui-core/tests/widget_virtual_list.rs::the_two_transitions_conform_at_every_tier`; page `list-row`; unverified here: none.

## Open questions

- None. `conflict-padding` is answered by [D127]: the side panel pads 8 (Lunegit wins, `SIDE_PANEL_PADDING`); a master list outside it pads 12 (`MASTER_PADDING`), a named constant the app picks.

## Notes for the planner

Wane P0. Lunegit's side-panel list is the drawn source; `SidePanelRow.cs` is not read, so its behaviour is not specified here.

---
id: overlay.command-palette
title: Command palette
area: overlay
status: done
consumers: lunegit
source: crates/lune-ui-core/src/widgets/command_palette.rs
owner: —
summary: 640 px search dialog over commits, branches, tags, stash, reflog, repos, settings and commands, with prefix filters
---

## What it is

Lunegit's global search dialog, renamed to the command palette (D11, D42): a 640 px dialog with one search box and grouped result sections; a footer lists the keys [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:10-16,77,127,179,238,298]. `Ui::command_palette(key, query, &Palette, glass) -> PaletteResponse` is generic [D147]: the app passes the sections and rows it found for the query, a placeholder, the footer's left text and an error line, and reads `changed`, `activated`, `show_more` and `dismissed`. Prefixes, debounce and date parsing are the app's (Lunegit's `GlobalSearchService`), not the toolkit's. It sits on `dialog::modal_host`, so it opens, closes and traps focus as the dialog does, and it draws the dialog's glass: a blurred scrim by default, the card `frost` of `DialogGlass.fill` with the `menu_popup_border` ring (D8, D9).

## Needs

- Modal dialog (R-overlay-1) with a text field and a result list (R-layout-3).

## Existing code

`crates/lune-ui-core/src/widgets/command_palette.rs` (`Ui::command_palette`, `Palette`, `PaletteSection`, `PaletteRow`, `PaletteResponse`), on `Ui::filter_bar`, `Ui::scroll_area`, `Ui::tooltip` and `dialog::modal_host`. Tests: `crates/lune-ui-core/tests/widget_command_palette.rs`. Page: `apps/gallery/src/pages/command_palette.rs`.

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml`, `GlobalSearchDialogView.axaml.cs:19-42` (Escape, Up and Down with wrap, Enter)

Moved code: none.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:1-547`
(sizes, sections, styles; `grep` for Transition, Animation and KeyDown: none); `src/Lunegit.Infrastructure/Localization/en.json:281-294`;
`src/Lunegit.Desktop/Themes/Animations.axaml` (no selector for the view's classes; `row-button` only at :172).
Conflicts: none between sources (Wane has no command palette entry; the Wane `overlay.dialog` is the host). One inside this entry, resolved by the contract: the earlier text gave the results area a margin of 18 0 18 10 and a padding of 10 0 10 0, but the axaml puts that margin on the indexing banner (`:60-62`) and only the padding and the 560 max height on the results `ScrollViewer` (`:63-67`); the visual batch ruled for the axaml (P1): the results area spans the card, rows sit 10 in (width 620) and their text 18 in, with no gap under the results. The error line sits in the axaml's footer block above the key text (`:523-532`); the visual batch ruled for the axaml (C18), so the build draws it there. Hosting:
the view is in `Views/Dialogs/`, which the dialog overlay hosts [L src/Lunegit.Desktop/MainWindow.axaml:232]; the host call was not read (Open question).

### Anatomy

- Dialog: width 640, max height 720 (and never taller than the window less 40 px) [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:10-12]; centred as the dialog is, its height follows the rows up to the maximum and snaps (never animated) [R crates/lune-ui-core/src/widgets/command_palette.rs `Geometry::of`].
- Search row: padding 18 18 18 10, a 40 px `Ui::filter_bar` (a 28 px dock in the filter bar's 8 by 6 padding: `:17-18`, `App.axaml:302-306`; its 14 px search glyph and clear button; the axaml's glyph is 16) [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:14-25]; placeholder is the app's `Palette.placeholder`.
- Error line (optional): the first line of the footer, one line of `font_s` in `status_error` #F44336 (`DarkTheme.axaml:63`), 6 px above the key line, inside the footer's padding (the footer grows by the line and the 6) [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:523-532].
- Results: a scroll area spanning the card (the `18 0 18 10` margin at `:60` is the indexing banner's, which the toolkit does not have), at most 560 high, no gap under it; the content is inset 10 (`LIST_PAD`), so rows are 620 wide at card x + 10 and header and row text start at x + 18; sections 10 apart [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:60-71]. With no rows there is no results area and no gap.
- Section header: `font_s` weight 600 `fg_tertiary`, inset 8, 4 px above its rows (the axaml's letter spacing 1 is not drawn: the painter has none) [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:72-76]. The title is the app's text (caps included).
- Row: radius 6, padding 8 6, a line of `font_base` `fg_primary` and, with `detail`, a `font_s` `fg_secondary` line under it; an icon of 14 with 8 to its right in `fg_dim`; `hint` right-aligned in `font_s` `fg_dim` (a `Chord::hint`), and for a destructive row a 6 px dot in `status_error` at the right edge, 8 from what is left of it [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:80-98,136-140; src/Lunegit.Desktop/App.axaml:400-428].
- "Show more": a row of the same padding after its section's rows, 4 px apart, `font_s` `fg_secondary` [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:117-128].
- Footer: padding 18 10 18 12 on `dialog_footer` (bottom corners 8); `font_s` `fg_secondary`; the app's `footer_left` at the left (cut to fit), the string table's `luneui.command-palette.footer-right` at the right [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:535-542].

### States

| state | look | cite |
|---|---|---|
| search box rest, hover, focus | the filter bar's: no ring, the caret is the focus cue | [R crates/lune-ui-core/src/widgets/filter_bar.rs] |
| row rest | no fill | [L src/Lunegit.Desktop/App.axaml:409-412] |
| row hover | `surface_hover` #FFFFFF10; hover fills without moving the selection | [L src/Lunegit.Desktop/App.axaml:413-416] |
| row pressed | `surface_active` #FFFFFF20 | [L src/Lunegit.Desktop/App.axaml:417-420] |
| row selected | `surface_selected` #FFFFFF18; selected and hovered `surface_active` | [L src/Lunegit.Desktop/App.axaml:421-428] |
| destructive row | a 6 px `status_error` dot [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:489-496]; hover shows the tooltip "Destructive action \u{2014} requires confirmation" (above the card: `Raised`); activation is reported as any row's | [L src/Lunegit.Infrastructure/Localization/en.json:291], [D147] |
| section with more results | a "Show more results" row after its rows, in the selection order | [L src/Lunegit.Infrastructure/Localization/en.json:290] |
| error | the error line | [L src/Lunegit.Infrastructure/Localization/en.json:292] |
| focus-visible on a row | none: rows take no focus; the selected row is the cue | [D147] |
| disabled | `with_enabled(false)`: nothing activates, Up, Down and Enter are taken but the search box's other keys are not (`docs/architecture.md`, "Disabled keys"), every node passes `disabled_violations` | [D140] |

### Transitions

Opening and closing are the dialog's (`pop`, base, quad-out; dismiss, fast; Reduced fades; Off snaps) through `modal_host`; a closing card draws what it last drew (the whole card is recorded each frame). The card's height snaps with the rows; hover and selection fills are instant (the view has no transition, D40). A selection that moves out of view scrolls with the scroll area's `INTO_VIEW` motion (snapped in Reduced and Off).

### Input and focus

- Pointer: a click on a row activates it and selects it; a click on the scrim reports `dismissed`; a click on the card does nothing.
- Keyboard (focus stays in the search box): Up and Down move the selection, wrapping, over rows and "show more" rows in order; Enter activates the selected row (or reports `show_more`); Escape reports `dismissed`. Up, Down and Enter are taken before the box draws, so they never reach it, and they are taken, but do nothing, in a disabled scope or with no rows. Control, Alt and Command chords stay shortcuts (an `Overlay` binding fires inside the palette; a `Global` one does not while it is open). While an input method composes, the keys are its own.
- Tab stop: the search box only; rows have none, and the clear button is none (`FilterOptions::clear_unfocusable`: the pointer and accessibility reach it, and a click leaves focus in the box), so Tab never leaves the box [D147].
- Layer: Modal (an overlay above it, a tooltip or a dialog the app opens, goes to `Raised`) [D137].
- Focus scope: the modal host's: opens focused in the search box; Escape and scrim click act on the top overlay only.
- Pointer capture while pressed: a row's, as any click region.

### Retained state

- Transient: the selected row's key and a hash of the row keys last shown (`PaletteMem`), dropped when the palette is not called, so each open starts at the first row. Row hover and press.
- Persistent: none.
- App-owned: the query, the rows (and the filtering that chose them), the error line, what an activation does.
- Exit retention: yes, the dialog's (180 / 120 / 0 ms).

### Accessibility

- The card: role `dialog`, modal; name = `Palette.name`, else the placeholder [R crates/lune-ui-core/src/widgets/command_palette.rs].
- The search box: role `text-input`, name = the placeholder, value = the query.
- Each section: `group` named by its title; rows: `list-item` named by the text, description the detail (a destructive row's is the destructive tooltip text), value the hint, `selected` on the selected row, action default (activate); "show more": a `button` with the same state and action.
- The error line: `status`, a polite live region, named by its text.
- Result-count announcement: none (no source) [D147].

### Strings

The toolkit's three, default text in `strings.rs` (D147):
- `luneui.command-palette.show-more`: "Show more results" [L src/Lunegit.Infrastructure/Localization/en.json:290].
- `luneui.command-palette.footer-right`: "Enter open    Esc close" [L src/Lunegit.Infrastructure/Localization/en.json:294].
- `luneui.command-palette.destructive-tooltip`: "Destructive action \u{2014} requires confirmation" [L src/Lunegit.Infrastructure/Localization/en.json:291].

App text, not toolkit keys (hard rule 1, as D128; D147): the placeholder ("Search commits, branches, tags, stash, repos...", en.json:281), the section titles (COMMITS, BRANCHES, TAGS, STASH, REFLOG, REPOS, SETTINGS, COMMANDS; en.json:282-289), the date filter error (en.json:292), and the footer's left text ("Up/Down select    # branches    : settings    > commands    date:7d"; en.json:293). They pass through `Palette`.

### Gallery scenes

The gallery's data is neutral (files and commands), not git words.
- `command-palette-empty`: empty query, search box focused, no rows, the footer, settled.
- `command-palette-results`: files and commands with rows, details, icons and hints, a "show more", the first row selected, settled.
- `command-palette-selected`: one Down moves the selection to the second row, settled.
- `command-palette-destructive`: the pointer on the destructive row, its tooltip above the card, settled.
- `command-palette-date-error`: the error line and no rows, settled.
- `command-palette-disabled`: the palette in a disabled scope with the pointer on the destructive row and Enter pressed: no dialog opens and no row activates (the overlay layer is not dimmed by the scope; the tooltip still shows), settled.
- `command-palette-opening`: half way through the open (125 ms).

### Painter primitives

- Existing: fill, fill-radii, text, icon glyph, clip and translate (through the scroll area).
- New: none.

### Acceptance

- Behaviour (scripted input, injected time; `crates/lune-ui-core/tests/widget_command_palette.rs`): Up and Down walk the sections and the "show more" rows and wrap; Enter activates the selected row once, a click activates and selects, hover moves nothing; the first row is selected at open and when the rows change, a selected key still present stays; Enter, Up and Down with no rows do nothing and reach no binding; typing edits the query while Up, Down and Enter leave the caret; "show more" by Enter and by click; a selection out of view is revealed; Escape and a scrim click dismiss once, a card click does nothing; Tab never lands on a row; a destructive row shows its tooltip on `Raised` and still reports; the page under it stays inert.
- Pixel probes at scale 2: card 1280 px wide, at most 1440 high, results at most 1120 high, the footer block's bottom corners 16, search bar 40 px (80 in the probe's logical px), scrim #00000080.
- Conformance: opening and closing sampled at 60 Hz under every tier equal the dialog's.
- Semantic node: `dialog` modal; `text-input` with the placeholder name; sections as groups; `list-item` rows with `selected`; the disabled walk.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_command_palette.rs` holds the behaviour bullet's every clause: `down_and_up_walk_both_sections_and_the_show_more_row_and_wrap`, `enter_activates_the_selected_row_once`, `hover_fills_without_moving_the_selection_and_a_click_activates`, `the_first_row_is_selected_at_open`, `the_first_row_is_selected_when_the_rows_change_and_a_present_key_stays`, `enter_with_no_rows_does_nothing_and_is_consumed`, `typing_edits_the_query_and_up_down_enter_do_not_move_the_caret`, `show_more_reports_its_section_by_enter_and_by_click`, `the_selected_row_is_in_view_the_frame_it_is_chosen`, `escape_and_a_scrim_click_dismiss_once_and_a_card_click_does_not`, `tab_stays_in_the_card_and_never_lands_on_a_row`, `the_destructive_row_describes_itself_shows_a_tooltip_above_the_card_and_still_activates`, `the_page_under_the_palette_stays_inert`; pixel probes `the_card_is_640_wide_and_the_results_stop_at_560_at_scale_two` (1280 and 1120 device px), `the_results_span_the_card_with_rows_ten_in_and_no_gap_under`; conformance `opening_and_closing_follow_the_dialogs_curves_under_every_tier`, `a_closing_palette_keeps_drawing_its_rows_then_vanishes`; node `the_sections_are_groups_and_the_rows_list_items`, `a_card_name_overrides_the_placeholder`, `a_disabled_palette_takes_its_keys_activates_nothing_and_passes_the_walk`.

### Build

- Milestone: M7.
- Depends on: overlay.dialog, widget.filter-bar, layout.scroll-area, overlay.tooltip, core.focus, infra.accessibility, infra.shortcut-routing.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_command_palette.rs::down_and_up_walk_both_sections_and_the_show_more_row_and_wrap, crates/lune-ui-core/tests/widget_command_palette.rs::enter_activates_the_selected_row_once, crates/lune-ui-core/tests/widget_command_palette.rs::escape_and_a_scrim_click_dismiss_once_and_a_card_click_does_not, crates/lune-ui-core/tests/widget_command_palette.rs::the_destructive_row_describes_itself_shows_a_tooltip_above_the_card_and_still_activates, crates/lune-ui-core/tests/widget_command_palette.rs::the_card_is_640_wide_and_the_results_stop_at_560_at_scale_two, crates/lune-ui-core/tests/widget_command_palette.rs::opening_and_closing_follow_the_dialogs_curves_under_every_tier, crates/lune-ui-core/tests/widget_command_palette.rs::the_sections_are_groups_and_the_rows_list_items, crates/lune-ui-core/tests/widget_command_palette.rs::a_disabled_palette_takes_its_keys_activates_nothing_and_passes_the_walk`; page `command-palette`; unverified here: real typing with an input method (Luneterm's M9 run, D15).

## Open questions

All five are closed (D147):
- Keyboard routing and prefix behaviour: Up, Down, Enter and Escape as above [L GlobalSearchDialogView.axaml.cs:19-42]; prefixes are the app's logic [L src/Lunegit.Application/Services/Search/GlobalSearchService.cs:68,289-310,341-348].
- Hosting: the dialog host; at the toolkit level a palette is its own component on `modal_host`.
- Row and section colours and the hover look: the `row-button` values above [L src/Lunegit.Desktop/App.axaml:400-428].
- Empty-query content: no rows; the footer stays [L GlobalSearchDialogViewModel.cs:179-182].
- Result-count announcement: none; no source.

## Notes for the planner

Renamed by `git mv` from `lunegit.global-search-dialog` (D10 mapping, D42). The axaml's letter spacing on the section headers is not drawn (no painter support), and the error line sits in the footer (C18).

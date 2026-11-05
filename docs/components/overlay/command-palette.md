---
id: overlay.command-palette
title: Command palette
area: overlay
status: in-progress
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml
owner: toolkit-v1 M7
summary: 640 px search dialog over commits, branches, tags, stash, reflog, repos, settings and commands, with prefix filters
---

## What it is

Lunegit's global search dialog, renamed to the command palette (D11, D42): a 640 px dialog with one search box and grouped result sections; a footer lists the keys [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:10-16,77,127,179,238,298]. `Ui::command_palette(key, query, &Palette, glass) -> PaletteResponse` is generic [D147]: the app passes the sections and rows it found for the query, a placeholder, the footer's left text and an error line, and reads `changed`, `activated`, `show_more` and `dismissed`. Prefixes, debounce and date parsing are the app's (Lunegit's `GlobalSearchService`), not the toolkit's. It sits on `dialog::modal_host`, so it opens, closes and traps focus as the dialog does.

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
Conflicts: none between sources (Wane has no command palette entry; the Wane `overlay.dialog` is the host). One inside this entry, resolved by the contract: the earlier text gave the results area a margin of 18 0 18 10 and a padding of 10 0 10 0, but the axaml puts that margin on the indexing banner (`:60-62`) and only the padding and the 560 max height on the results `ScrollViewer` (`:63-67`); the build keeps the entry's reading (viewport inset 18, rows inset a further 10), and the visual batch decides whether the rows go to 10. The error line sits in the axaml's footer block above the key text (`:526-533`); the build draws it above the results, as the contract says. Hosting:
the view is in `Views/Dialogs/`, which the dialog overlay hosts [L src/Lunegit.Desktop/MainWindow.axaml:232]; the host call was not read (Open question).

### Anatomy

- Dialog: width 640, max height 720 (and never taller than the window less 40 px) [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:10-12]; centred as the dialog is, its height follows the rows up to the maximum and snaps (never animated) [R crates/lune-ui-core/src/widgets/command_palette.rs `Geometry::of`].
- Search row: padding 18 18 18 10, a 28 px `Ui::filter_bar` (its 14 px search glyph and clear button; the axaml's glyph is 16) [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:14-25]; placeholder is the app's `Palette.placeholder`.
- Error line (optional): one line of `font_s` in `status_error_light`, under the search row, then a 10 px gap [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:526-533].
- Results: a scroll area inset 18 each side, at most 560 high, 10 px gap under it; the content is inset a further 10; sections 10 apart [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:60-71]. With no rows there is no results area and no gap.
- Section header: `font_s` weight 600 `fg_tertiary`, inset 8, 4 px above its rows (the axaml's letter spacing 1 is not drawn: the painter has none) [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:72-76]. The title is the app's text (caps included).
- Row: radius 6, padding 8 6, a line of `font_base` `fg_primary` and, with `detail`, a `font_s` `fg_secondary` line under it; an icon of 14 with 8 to its right in `fg_dim`; `hint` right-aligned in `font_s` `fg_dim` (a `Chord::hint`), and for a destructive row the `icon::WARNING` glyph (`warn`) at the right edge [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:80-98,136-140; src/Lunegit.Desktop/App.axaml:400-428].
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
| destructive row | the warning glyph; hover shows the tooltip "Destructive action \u{2014} requires confirmation" (above the card: `Raised`); activation is reported as any row's | [L src/Lunegit.Infrastructure/Localization/en.json:291], [D147] |
| section with more results | a "Show more results" row after its rows, in the selection order | [L src/Lunegit.Infrastructure/Localization/en.json:290] |
| error | the error line | [L src/Lunegit.Infrastructure/Localization/en.json:292] |
| focus-visible on a row | none: rows take no focus; the selected row is the cue | [D147] |
| disabled | `with_enabled(false)`: nothing activates, the keys are taken, every node passes `disabled_violations` | [D140] |

### Transitions

Opening and closing are the dialog's (`pop`, base, quad-out; dismiss, fast; Reduced fades; Off snaps) through `modal_host`; a closing card draws what it last drew (the whole card is recorded each frame). The card's height snaps with the rows; hover and selection fills are instant (the view has no transition, D40). A selection that moves out of view scrolls with the scroll area's `INTO_VIEW` motion (snapped in Reduced and Off).

### Input and focus

- Pointer: a click on a row activates it and selects it; a click on the scrim reports `dismissed`; a click on the card does nothing.
- Keyboard (focus stays in the search box): Up and Down move the selection, wrapping, over rows and "show more" rows in order; Enter activates the selected row (or reports `show_more`); Escape reports `dismissed`. Up, Down and Enter are taken before the box draws, so they never reach it, and they are taken, but do nothing, in a disabled scope or with no rows. Control, Alt and Command chords stay shortcuts (an `Overlay` binding fires inside the palette; a `Global` one does not while it is open). While an input method composes, the keys are its own.
- Tab stop: the search box only; rows have none. With text in the box the clear button is the second stop, as the filter bar's; Tab never leaves the card.
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
- `command-palette-opening`: half way through the open (125 ms).

### Painter primitives

- Existing: fill, fill-radii, text, icon glyph, clip and translate (through the scroll area).
- New: none.

### Acceptance

- Behaviour (scripted input, injected time; `crates/lune-ui-core/tests/widget_command_palette.rs`): Up and Down walk the sections and the "show more" rows and wrap; Enter activates the selected row once, a click activates and selects, hover moves nothing; the first row is selected at open and when the rows change, a selected key still present stays; Enter, Up and Down with no rows do nothing and reach no binding; typing edits the query while Up, Down and Enter leave the caret; "show more" by Enter and by click; a selection out of view is revealed; Escape and a scrim click dismiss once, a card click does nothing; Tab never lands on a row; a destructive row shows its tooltip on `Raised` and still reports; the page under it stays inert.
- Pixel probes at scale 2: card 1280 px wide, at most 1440 high, results at most 1120 high, the footer block's bottom corners 16, search bar 28 px (56 in the probe's logical px), scrim #00000080.
- Conformance: opening and closing sampled at 60 Hz under every tier equal the dialog's.
- Semantic node: `dialog` modal; `text-input` with the placeholder name; sections as groups; `list-item` rows with `selected`; the disabled walk.

### Build

- Milestone: M7.
- Depends on: overlay.dialog, widget.filter-bar, layout.scroll-area, overlay.tooltip, core.focus, infra.accessibility, infra.shortcut-routing.

## Open questions

All five are closed (D147):
- Keyboard routing and prefix behaviour: Up, Down, Enter and Escape as above [L GlobalSearchDialogView.axaml.cs:19-42]; prefixes are the app's logic [L src/Lunegit.Application/Services/Search/GlobalSearchService.cs:68,289-310,341-348].
- Hosting: the dialog host; at the toolkit level a palette is its own component on `modal_host`.
- Row and section colours and the hover look: the `row-button` values above [L src/Lunegit.Desktop/App.axaml:400-428].
- Empty-query content: no rows; the footer stays [L GlobalSearchDialogViewModel.cs:179-182].
- Result-count announcement: none; no source.

## Notes for the planner

Renamed by `git mv` from `lunegit.global-search-dialog` (D10 mapping, D42). The axaml's letter spacing on the section headers is not drawn (no painter support), and the results margin and the error line's place are the two readings recorded under Conflicts.

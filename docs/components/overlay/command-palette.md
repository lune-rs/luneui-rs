---
id: overlay.command-palette
title: Command palette
area: overlay
status: planned
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml
owner: —
summary: 640 px search dialog over commits, branches, tags, stash, reflog, repos, settings and commands, with prefix filters
---

## What it is

Lunegit's global search dialog, renamed to the command palette (D11, `git mv` from `lunegit.global-search-dialog`): a 640 px dialog with one search box and grouped result sections (commits, branches, tags, stash, reflog, repos, settings, commands); a footer lists the keys and the prefix filters [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:10-16,77,127,179,238,298], [L src/Lunegit.Infrastructure/Localization/en.json:281-294].

## Needs

- Modal dialog (R-overlay-1) with a text field and a result list (R-layout-3).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml`

Moved code: none. The view's code-behind and view model were not read; the keyboard routing is an Open question.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:1-547`
(sizes, sections, styles; `grep` for Transition, Animation and KeyDown: none); `src/Lunegit.Infrastructure/Localization/en.json:281-294`;
`src/Lunegit.Desktop/Themes/Animations.axaml` (no selector for the view's classes; `row-button` only at :172).
Conflicts: none between sources (Wane has no command palette entry; the Wane `overlay.dialog` is the host). Hosting:
the view is in `Views/Dialogs/`, which the dialog overlay hosts [L src/Lunegit.Desktop/MainWindow.axaml:232]; the host call was not read (Open question).

### Anatomy

- Dialog: width 640, max height 720 [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:10-12]; rows Auto, Auto, `*`, Auto [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:10].
- Search box: padding 18 18 18 10 [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:14-16]; a 28 px filter-bar row [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:17-18]; search glyph 16 px, 8 px to the right, `ForegroundDim` [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:19-25]; text at `FontSizeM` [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:35].
- Results area: margin 18 0 18 10; max height 560; padding 10 0 10 0 [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:60-67].
- Section header: `FontSizeS`, margin 8 0 [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:72-76]; section titles in caps (en.json:282-289).
- Result row: a `row-button` button [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:80,130,182,241]; base-size main text and small secondary text [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:87,93,98]; icons 14 px, 8 px right margin, for branches, tags and stashes [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:136-140,189-193,247-251].
- Footer: small text, `ForegroundSecondary`, left and right columns [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:535-542].

### States

| state | look | cite |
|---|---|---|
| search box rest | transparent box, no border | [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:29-37] |
| search box focus | focus border style on the template border | [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:44-48] |
| search box hover | pointer-over border style | [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:49-53] |
| result row rest and hover | the `row-button` state; hover look not read | [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:80] |
| result row selected (Up/Down) | keyboard selection; look not read | [L src/Lunegit.Infrastructure/Localization/en.json:293] (footer "Up/Down select"); open question |
| destructive row | tooltip "Destructive action — requires confirmation" | [L src/Lunegit.Infrastructure/Localization/en.json:291] |
| section with more results | "Show more results" control | [L src/Lunegit.Infrastructure/Localization/en.json:290] |
| date filter error | text "Couldn't parse date filter — showing no commits." | [L src/Lunegit.Infrastructure/Localization/en.json:292] |
| focus-visible on a row | not read | open question |
| disabled | not in the view | open question |

### Transitions

n/a — the view has no transition or animation of its own (`grep` of GlobalSearchDialogView.axaml for Transition and
Animation is empty); if hosted by the dialog overlay it takes [overlay.dialog]'s open and close.

### Input and focus

- Pointer: click a result row to open it (the row is a button, [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:80]); click the destructive row shows the confirmation tooltip (en.json:291).
- Keyboard: Up and Down select; Enter opens; Esc closes [L src/Lunegit.Infrastructure/Localization/en.json:293-294]. Prefix filters: `#` branches, `:` settings, `>` commands, `date:7d` [L src/Lunegit.Infrastructure/Localization/en.json:293]. The routing lives in the view model, not read (Open question).
- Tab stop: the search box, then the rows (derived: the rows are buttons, [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:80]) [core.focus].
- Layer: Modal (the dialog overlay, [L src/Lunegit.Desktop/MainWindow.axaml:232]) [derived].
- Focus scope: push on open; the search box takes focus; pop restores focus to the previous widget [core.focus]. Esc closes the top overlay only [core.focus].
- Pointer capture while pressed: none in the view.

### Retained state

- Transient: row hover; keyboard selection index.
- Persistent: the query text and the selected row are kept while the palette is open [derived: the query is bound, `Text="{Binding QueryText}"`, GlobalSearchDialogView.axaml:27].
- App-owned: the result lists (commits, branches, tags, stash, reflog, repos, settings, commands), the query, the date filter error (bindings at [L src/Lunegit.Desktop/Views/Dialogs/GlobalSearchDialogView.axaml:27,77,127,179,238,298]).
- Exit retention: no (no motion of its own).

### Accessibility

- Dialog: role `dialog`, modal; name = the search placeholder "Search commits, branches, tags, stash, repos..." [L src/Lunegit.Infrastructure/Localization/en.json:281] [infra.accessibility].
- Search box: role `text-input`, name = placeholder, value = query [infra.accessibility].
- Each section: `group` named by the section title (en.json:282-289); rows `button` or `list-item` with `selected` state on the keyboard row [infra.accessibility].
- Destructive row: description = the destructive tooltip (en.json:291).
- Live region: the result count is not announced in any source; open question.

### Strings

- `luneui.command-palette.placeholder`: "Search commits, branches, tags, stash, repos..." (Lunegit `dialog.search.placeholder`) [L src/Lunegit.Infrastructure/Localization/en.json:281].
- `luneui.command-palette.section-commits`: "COMMITS" [L src/Lunegit.Infrastructure/Localization/en.json:282].
- `luneui.command-palette.section-branches`: "BRANCHES" [L src/Lunegit.Infrastructure/Localization/en.json:283].
- `luneui.command-palette.section-tags`: "TAGS" [L src/Lunegit.Infrastructure/Localization/en.json:284].
- `luneui.command-palette.section-stash`: "STASH" [L src/Lunegit.Infrastructure/Localization/en.json:285].
- `luneui.command-palette.section-reflog`: "REFLOG" [L src/Lunegit.Infrastructure/Localization/en.json:286].
- `luneui.command-palette.section-repos`: "REPOS" [L src/Lunegit.Infrastructure/Localization/en.json:287].
- `luneui.command-palette.section-settings`: "SETTINGS" [L src/Lunegit.Infrastructure/Localization/en.json:288].
- `luneui.command-palette.section-commands`: "COMMANDS" [L src/Lunegit.Infrastructure/Localization/en.json:289].
- `luneui.command-palette.show-more`: "Show more results" [L src/Lunegit.Infrastructure/Localization/en.json:290].
- `luneui.command-palette.destructive-tooltip`: "Destructive action — requires confirmation" [L src/Lunegit.Infrastructure/Localization/en.json:291].
- `luneui.command-palette.date-filter-error`: "Couldn't parse date filter — showing no commits." [L src/Lunegit.Infrastructure/Localization/en.json:292].
- `luneui.command-palette.footer-left`: "Up/Down select    # branches    : settings    > commands    date:7d" [L src/Lunegit.Infrastructure/Localization/en.json:293].
- `luneui.command-palette.footer-right`: "Enter open    Esc close" [L src/Lunegit.Infrastructure/Localization/en.json:294].

### Gallery scenes

- `command-palette-empty` (new): empty query, search box focused, settled.
- `command-palette-results` (new): commits, branches and tags sections with rows, settled.
- `command-palette-selected` (new): one row selected by Down, settled.
- `command-palette-destructive` (new): a destructive row with its tooltip, settled.
- `command-palette-date-error` (new): the date filter error text, settled.

### Painter primitives

- Existing: fill, ring, text, icon.
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): Up and Down move the selection; Enter opens the selected row; Esc closes; a `#`, `:`, `>` or `date:` prefix filters the sections the view model names. None exists; the prefix behaviour awaits the view model read.
- Pixel probes at scale 2: dialog width 1280 px, max height 1440 px, results max height 1120 px, footer text at `FontSizeS`. Source values in Anatomy.
- Conformance: none (no motion of its own).
- Semantic node: dialog modal; text-input with the placeholder name; sections as groups; selected row state.

### Build

- Milestone: M7.
- Depends on: overlay.dialog, widget.text-field, widget.button, core.focus, infra.accessibility, infra.shortcut-routing, layout.scroll-area.

## Open questions

- Keyboard routing and prefix behaviour: in the view model, not read; open.
- How the view is hosted (DialogService or the dialog overlay), and whether it is a dialog or a palette at the toolkit level; open.
- Row and section colours and the hover look: the row styles are in `row-button` (Animations.axaml:172) and the theme; not read for values; open.
- Empty-query content (recent items or nothing): not in the strings; open.
- Result-count announcement for screen readers: not in any source; open.

## Notes for the planner

The stub said "rough stub — not analysed"; this entry reads the view's markup and the English strings. The view model is the next read before M7 builds the keyboard path.

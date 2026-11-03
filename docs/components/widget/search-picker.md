---
id: widget.search-picker
title: Search picker
area: widget
status: planned
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Controls/SearchPickerBase.cs
owner: —
summary: Search picker: a field-style or link-style trigger that opens a searchable list; Escape closes, Enter picks the first match
---

## What it is

`search-picker`: a trigger that opens a popup with a search box over a filtered list. Two Lunegit variants: a field trigger in filter-bar chrome (`FieldSearchPicker.axaml`) and a link trigger with an underlined text (`LinkSearchPicker.axaml`). Both share `SearchPickerBase` [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:31].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-2); popup (R-overlay-4 through `overlay.dropdown`).
- Keyboard: Escape and Enter in the search box; Tab stop (R-widget-3); semantic node (R-infra-4).

## Existing code

`src/Lunegit.Desktop/Controls/SearchPickerBase.cs`, `src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml`, `src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `src/Lunegit.Desktop/Controls/SearchPickerBase.cs:31-53,143,154,160-163,302-318`; `src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:18-53`; `src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:8-56`.
Wane has no search-picker component; the Wane combo (`widget.combo`) is not searchable. Lunegit alone is the source.

### Anatomy

- Field trigger: `Border.filter-bar` chrome, a 24 px row, text 13 px `ForegroundPrimary` with character ellipsis, search icon at the right with left margin 8 [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:35-50].
- Link trigger: leading 14 px icon, spacing 6, text 13 px `ForegroundSecondary` with a 1 px underline stroked dashed `1,2` [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:16-31].
- Popup: `BackdropDark` fill, `SurfaceBorder` 1 px, radius 8, padding 6, min width 220, max height 320 [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:42-48].
- Search bar at the top of the popup: radius 4, `SurfaceInput`, padding 8 horizontal and 6 vertical, bottom margin 6 [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:52-56].
- Popup width tracks the field trigger [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:53].
- Placeholder: `SearchPlaceholder`, default empty [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:50].

### States

| state | look | cite |
|---|---|---|
| field, rest | filter-bar surface, value text | [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:18-50] |
| field, hover | no change: the trigger's pointerover clears its background | [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:26-29] |
| link, rest | secondary text with dashed underline | [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:19-31] |
| link, hover, pressed | not styled by Lunegit (Fluent default); open | [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:11-15] |
| open | popup shown; trigger state unchanged | [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:143], [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:38-41] |
| focus-visible | not styled; open | [L src/Lunegit.Desktop/Controls/FieldSearchPicker.axaml:18-34] |
| disabled | opacity 0.4, inert | [widget.button], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

n/a — Lunegit styles no motion on either trigger; the popup's open and close motion belongs to `overlay.dropdown`.

### Input and focus

- Pointer: a trigger click toggles the popup [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:143]; light dismiss on an outside click [L src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:41].
- Keyboard in the search box: Escape closes; Enter selects the first filtered item and closes [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:302-318].
- Tab stop: the trigger [core.focus].
- Layer: trigger `Base`; popup `Popup` (`overlay.dropdown`).
- Focus scope: the popup pushes a scope on open and closing restores focus to the trigger [overlay.dropdown].
- Dismiss: Escape and outside click act on the open popup only [overlay.dropdown]. Closing the popup by any route syncs the open flag [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:160-163].

### Retained state

- Transient: hover, press, the open flag while the popup is up [core.ui].
- Persistent: the search text is held by the picker (Open questions: whether it survives a close).
- App-owned: `ItemsSource`, `SelectedItem`, `DisplayMemberBinding`, `Icon`, `SearchPlaceholder` (Lunegit properties [L src/Lunegit.Desktop/Controls/SearchPickerBase.cs:35-53]).
- Exit retention: no.

### Accessibility

- Trigger: role `combo-box`; name = the owner's label; value = the selected item's display text; states expanded (while open), focusable, focused, disabled; actions `expand`, `collapse`, `default` [infra.accessibility].
- Search box: role `text-input`; name = `SearchPlaceholder`.
- List: role `list` of `list-item`s; the filtered items are the list [infra.accessibility].

### Strings

None: placeholder and item text are owner data (`SearchPlaceholder`, `DisplayMemberBinding`).

### Gallery scenes

- `search-picker-field`: field trigger with a value, settled.
- `search-picker-link`: link trigger with a value, settled.
- `search-picker-open`: link trigger with the popup open, search text, filtered list; settled.

### Painter primitives

- Existing: fill, ring, text, icon, clip-rect
- New: dash (the dashed underline, `src/Lunegit.Desktop/Controls/LinkSearchPicker.axaml:30`)

### Acceptance

- Scripted: trigger click opens and closes once; Escape closes; Enter picks the first match and closes; an outside click closes the popup.
- Pixel probes at scale 2: popup radius 16 px; search bar radius 8 px; dash 1 px stroke with a 2 px / 4 px pattern.
- Conformance: n/a — no motion on the trigger.
- Semantic node: role combo-box; expanded follows the open flag.

### Build

- Milestone: M6
- Depends on: overlay.dropdown, widget.filter-bar, text.text-edit, core.focus, infra.accessibility

## Open questions

- `behaviour`: whether the search text is cleared on close and whether the search box takes focus on open. Lunegit's code shows neither as explicit lines in the read range; M6 checks the rest of `SearchPickerBase.cs`.
- `variant`: the link trigger's hover and focus looks are Fluent defaults; the spec needs a value or a decision before M6.

## Notes for the planner

Lunegit-only; consumers lunegit. Two triggers share one base; the spec keeps both.

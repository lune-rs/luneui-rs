---
id: widget.text-field
title: Text field
area: widget
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/field.rs
owner: orch/toolkit-v1
summary: Single-line text box: 32 px, radius 6, placeholder, caret when focused; variants fill, small, mono, password, search and date; editing engine from text.text-edit
---

## What it is

`field`: a 32 px TextBox with a placeholder and a caret when focused, in the existing code. This entry
is its widget spec: the box look (fill, ring, radius), the variants Wane names (fill, small, mono,
password, search, date), the focus and hover states, and how it uses the editing engine
(`text.text-edit`) and composition (`text.ime`). Caret, selection, undo and clipboard are the
engine's, not the box's.

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Text selection, caret movement and clipboard (R-text-5): `text.text-edit`.
- Wane — Variants: fill (100% width); sm (24 high); mono (ids, URLs, paths); password (fixed 16-dot mask); search (16 px search icon at the left); date (calendar icon at the right, mono) (R-widget-1)
- Wane — States: rest; hover (`--surface-hover`); focus (border `--accent`); disabled (opacity .4); error (set by `widget.form-field`: border `--err`). (R-widget-1)
- Wane — Sizes: 240 x 32 (`--control-w-field`, `--control-h`), radius 6, 1 px `--border`, fill `--surface-input`, input padding 0 9, text 13, placeholder tertiary; search padding-left 33; date padding-right 28. (R-widget-1)
- Wane — Behaviour: Caret, selection, clipboard (cut, copy, paste), Esc, Tab. The password mask has a fixed length so it does not leak the key length, and the value cannot be copied. IME composition is a requirement (`text.ime` is planned). Not mocked: caret, selection, validation timing. (R-widget-1)

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:120` (`field`), box at `:50` (`input_box`); the glass-ui spike calls it from `src/widgets.rs`. `widgets/mod.rs` has 203 lines and mostly no tests.
Drawn in the gallery's `controls` scene (`apps/gallery/src/scenes.rs:289-304`) and in the `dialog-body` scene (`apps/gallery/src/scenes.rs:511-518`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: this repo crates/lune-ui-core/src/widgets/mod.rs:45-69,120-144, crates/lune-ui-core/src/theme.rs:11-45,56-90, apps/gallery/src/scenes.rs:284-306,500-520;
Lunegit src/Lunegit.Desktop/Controls/FilterBar.axaml:8-44, Views/Settings/GitSettingsView.axaml:56-84, App.axaml:478-484 (disabled opacity 0.4), .claude/docs/animation-rules.md:98-110; Wane docs/ui/components.md:345-360,445-458.
Lunegit styles no `TextBox` globally (`grep -n TextBox src/Lunegit.Desktop/App.axaml Themes/*.axaml` finds nothing): each view wraps a transparent `TextBox` in its own `Border` [L src/Lunegit.Desktop/Controls/FilterBar.axaml:8-44].
Conflicts: (1) radius: Lunegit's filter box 4 [L src/Lunegit.Desktop/Controls/FilterBar.axaml:8], the moved box and Wane 6 [R crates/lune-ui-core/src/theme.rs:85], [W docs/ui/components.md:352]; the moved code wins (pixel-gated).
(2) text inset: Wane's padding 0 9 [W docs/ui/components.md:352] against the moved box's 10 [R crates/lune-ui-core/src/widgets/field.rs:60-116]; the moved code wins. (3) width: Wane 240 [W docs/ui/components.md:352], the gallery draws 210 and a dialog's inner width [R apps/gallery/src/scenes.rs:291-298,513]; the width is the caller's, 240 is the Wane default.
(4) caret colour: see `text.text-edit` (the moved `fg_primary` wins).

### Anatomy

- Box: height 32 (the caller's rect), radius `item_radius` 6, fill `surface_input` at rest and `surface_hover` when hovered and not focused, 1 px ring `border`, `accent` when focused [R crates/lune-ui-core/src/widgets/input_box.rs:29-34] [R crates/lune-ui-core/src/widgets/stepper.rs:80] [R crates/lune-ui-core/src/widgets/combo.rs:53], [R crates/lune-ui-core/src/theme.rs:85]; Wane: 1 px `--border`, fill `--surface-input`, focus border `--accent` [W docs/ui/components.md:351-352].
- Text: drawn 10 px from the left edge, clipped to the box inset 6 px; placeholder in `fg_tertiary` while empty [R crates/lune-ui-core/src/widgets/field.rs:60-116]; text size 13 (`font_base`) [W docs/ui/components.md:352], [R crates/lune-ui-core/src/theme.rs:83].
- Caret: `text.text-edit` Anatomy (1.5 px, `fg_primary`) [R crates/lune-ui-core/src/widgets/field.rs:60-116].
- Variant `fill`: width 100 percent of the parent; `sm`: 24 high; default width 240 [W docs/ui/components.md:350,352].
- Variant `mono`: the mono face for ids, URLs and paths [W docs/ui/components.md:350].
- Variant `password`: a fixed mask of 16 dots, never the real length; not copyable [W docs/ui/components.md:350,353].
- Variant `search`: a 16 px search icon at the left, text padding-left 33 [W docs/ui/components.md:350,352]. Lunegit's filter box: icon 14 px in `ForegroundDim`, margin right 8, and a clear button 18 x 18 with a 12 px icon shown while the text is non-empty [L src/Lunegit.Desktop/Controls/FilterBar.axaml:10-20]. Wane's 16 px icon and 33 px inset win for the Wane look; Lunegit's clear button is taken as is (Wane has none).
- Variant `date`: a calendar icon at the right, mono text, padding-right 28 [W docs/ui/components.md:350,352]. Only the look: no picker (none in either source).
- Error: border in the error colour, set by the form field around it [W docs/ui/components.md:351], [widget.form-field].
- Disabled: opacity 0.4 [W docs/ui/components.md:351], [L src/Lunegit.Desktop/App.axaml:482].

### States

| state | look | cite |
|---|---|---|
| rest | fill `surface_input`, ring `border`, text `fg_primary`, placeholder `fg_tertiary` | [R crates/lune-ui-core/src/widgets/field.rs:60-116] [R crates/lune-ui-core/src/widgets/input_box.rs:29-34] [R crates/lune-ui-core/src/widgets/stepper.rs:80] [R crates/lune-ui-core/src/widgets/combo.rs:53] |
| hover | fill `surface_hover` (not while focused) | [R crates/lune-ui-core/src/widgets/field.rs:60-116] [R crates/lune-ui-core/src/widgets/input_box.rs:29-34] [R crates/lune-ui-core/src/widgets/stepper.rs:80] [R crates/lune-ui-core/src/widgets/combo.rs:53] |
| pressed | as hover until the press places the caret and focuses it | [derived: the press focuses the field; Lunegit and Wane draw no separate pressed look] |
| focus-visible | ring `accent`, caret shown | [R crates/lune-ui-core/src/widgets/field.rs:60-116] [R crates/lune-ui-core/src/widgets/input_box.rs:29-34], [W docs/ui/components.md:351] |
| disabled | opacity 0.4, inert | [W docs/ui/components.md:351], [L src/Lunegit.Desktop/App.axaml:482] |
| error | ring in the error colour | [W docs/ui/components.md:351] |
| empty | placeholder in `fg_tertiary` | [R crates/lune-ui-core/src/widgets/field.rs:60-116] |
| password | fixed 16-dot mask | [W docs/ui/components.md:350] |
| composing | preedit with an underline | [text.ime] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest and hover | box fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102] |
| focus gained or lost | ring colour `border` and `accent` | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |
| error set or cleared | ring colour | selection | fast | linear | keep | [D40] |

Lunegit's filter box makes the inner TextBox's template border transparent in every state, so it draws no focus ring [L src/Lunegit.Desktop/Controls/FilterBar.axaml:28-41]; the focus ring colour change is therefore Wane's look under the animation rules' focus row. Typing and caret movement never animate [L .claude/docs/animation-rules.md:148].

### Input and focus

- Pointer: click focuses and places the caret; hover only changes the fill; see `text.text-edit` for selection by drag, double-click and triple-click [text.text-edit].
- Keys: everything the engine lists in `text.text-edit`; Tab moves to the next field and does not insert a tab; Esc and Enter are the engine's open questions [W docs/ui/components.md:353], [text.text-edit].
- Tab stop: yes, in call order [core.focus]. Layer: the layer of the field's parent. No focus scope.
- Pointer capture while dragging a selection [core.ui].
- Clear button: a press clears the text, keeps the focus in the field [derived: Lunegit's clear button handler is `ClearButton.Click`, FilterBar.axaml.cs:37; what it does beyond clearing is not read].

### Retained state

- Transient: hover, the ring transition [core.ui].
- Persistent: caret, selection, undo history and horizontal scroll, owned by the engine [text.text-edit].
- App-owned: the text, the placeholder, the variant, read-only, disabled, error and password flags, the width.
- Exit retention: no.

### Accessibility

- Role `text-input` (a `multiline-text` for the multi-line form in `text.text-edit`); name = the form field's label or the placeholder when no label is passed; description = the hint; value = the text, withheld for a password; states focusable, focused, disabled, read-only, required, invalid; actions focus and set-value [infra.accessibility].
- Wane's form field passes label, hint and the error text [W docs/ui/components.md:445-458].

### Strings

- `luneui.text-field.clear`: "Clear", the clear button's name [derived: the clear button has no name or tooltip in the Lunegit view, FilterBar.axaml:13-20; no en.json key].

The placeholder and the label are app text.

### Gallery scenes

- `controls` (existing): empty and focused field at scales 1 and 2.
- `dialog-body` (existing): the focused field in a dialog.
- `text-field-states`: rest, hover, focus-visible, disabled, error, each with the text "lune", settled.
- `text-field-empty`: empty with the placeholder, settled.
- `text-field-variants`: fill, sm, mono, password, search and date side by side, settled.
- `text-field-search-clear`: the search variant with text and the clear button, settled.
- `text-field-composing`: a focused field with a preedit (see `ime-preedit`), settled.
- `text-field-hover-fade`: pointer enters, t = 60 ms (half of `instant`).
- `text-field-focus-fade`: focus gained, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill-radii, fill-over, ring, text, icon, clip-rect.
- New: none.

### Acceptance

- Behaviour (scripted input): a click focuses and places the caret; typing changes the text and reports `changed`; Tab moves focus away; a disabled field ignores the click; the password variant copies nothing; the clear button empties the text and keeps focus [core.ui], [text.text-edit].
- Pixel probes at scale 2: box 64 device px high, radius 12, ring 2 px; fill `surface_input` at rest, `surface_hover` hovered; ring `accent` when focused; the existing `controls` and `dialog-body` dumps stay at zero delta at settled frames [R apps/gallery/src/scenes.rs:289-304].
- Sizes: `sm` is 48 device px high; the search variant's text starts 66 device px from the left edge [W docs/ui/components.md:350,352].
- Conformance: the hover and focus transitions pass `motion::conformance`; Reduced keeps them, Off snaps [motion].
- Semantic node: role `text-input`; value follows the text; invalid follows the error flag; an injected `AccessAction` set-value changes the text.

### Build

- Milestone: M4 (retrofit: focus, semantic node, transitions) and M5 (the editing engine, variants, IME); done by M11. Deferred to M5 [D74]: variants, typing, search clear and IME composition, with the scenes `text-field-variants`, `text-field-search-clear` and `text-field-composing`. M5 built the engine wiring, the variants, search clear and IME composition (`text_field_with`, `text_area`; D88); the deferred scenes `text-field-variants`, `text-field-search-clear` and `text-field-composing` exist. Open for `done` (D47): M11 re-checks the whole bar on every entry; real typing with an input method (R1) and bidi caret placement are unverified here, and so is the Windows and Linux run time.
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions, text.text-edit, text.ime, platform.clipboard

## Open questions

- Answered (D103, D88): none beyond the engine's (selection colour, Esc, undo depth: see `text.text-edit`); the date variant has no picker in either source.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it, so the retrofit must keep the settled `controls` and `dialog-body` frames identical.

Wane (2026-11-04): lune-ui status for Wane's use: partial: the text field exists (widgets/mod.rs:120, box widgets/mod.rs:50); masked, search-icon, date and mono variants are missing; Wane priority P1. Native note: Native text input is not CSS-approximable: caret, selection, IME and clipboard are real work in luneui-rs. The mock's dots are U+2022 at 24 px in RznSans; luneui-rs draws its own mask glyphs. Full spec: Wane `docs/ui/components.md`, section `widget.text-field`; mock: Wane `docs/ui/mocks/components.html`.

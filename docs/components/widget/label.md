---
id: widget.label
title: Label
area: widget
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/mod.rs
owner: orch/toolkit-v1
summary: Label: UI-font text at the theme base size; colour from the caller; section and title roles from the font scale
---

## What it is

`label`: UI-font text at the theme's base size, colour passed by the caller. Wane's text roles (secondary, small, section, title, mono, strong) are specified here; the moved code has the base size only [W docs/ui/components.md:280].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Text shaping with RznSans for UI text and RznMono for ids and numbers (`text.shaping`).
- Semantic node with the static-text role (R-infra-4).

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:15` (`label`, 3 lines). Used by every control in the moved code, and drawn in the gallery's `controls` scene under LABELS (`apps/gallery/src/scenes.rs:314-331`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:15-17`;
`crates/lune-ui-core/src/theme.rs:82-84`; `apps/gallery/src/scenes.rs:314-331`; Wane `docs/ui/components.md:277-289`;
Lunegit `src/Lunegit.Desktop/App.axaml:158-160` (font scale), `src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:151-156` (TextBlock use).
Conflicts: (1) Wane gives the base size 13/20 at weight 400 in primary text [W docs/ui/components.md:282]; the moved code uses `font_base` 13 with the caller's colour [R crates/lune-ui-core/src/widgets/mod.rs:16]; the code wins for the base role. (2) Wane's section role is 14 Medium and title 18/24 Medium [W docs/ui/components.md:280,282]; the moved theme has `font_m` 14 [R crates/lune-ui-core/src/theme.rs:84] and the gallery draws the medium weight at `font_m` [R apps/gallery/src/scenes.rs:328]; no title size exists in the moved theme (Open questions).

### Anatomy

- Font: RznSans for UI text, RznMono for ids, paths, money and numbers [W docs/ui/components.md:282,288]; the moved code draws every label in the UI style [R crates/lune-ui-core/src/widgets/mod.rs:16].
- Base: 13 px at `font_base` [R crates/lune-ui-core/src/theme.rs:83]; Lunegit's `FontSizeBase` is 13 [L src/Lunegit.Desktop/App.axaml:160].
- Small: 11 px at `font_s`, tertiary colour [R crates/lune-ui-core/src/theme.rs:82]; Lunegit `FontSizeS` 11 [L src/Lunegit.Desktop/App.axaml:159]; Wane small 11/16 tertiary [W docs/ui/components.md:280].
- Section: 14 px Medium at `font_m` [R crates/lune-ui-core/src/theme.rs:84]; Wane 14 Medium [W docs/ui/components.md:280].
- Title: 18 px / 24 Medium `fg_bright` [W docs/ui/components.md:280,282]; not in the moved theme, added by [D58].
- Mono, strong: Wane RznMono and 500 `fg_bright` [W docs/ui/components.md:280].
- Colour: the caller's `Rgba`; Lunegit's labels use `ForegroundPrimary` and `ForegroundMuted` [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:152,155].

### States

| state | look | cite |
|---|---|---|
| static | text only; no states | [W docs/ui/components.md:281] |

### Transitions

n/a — static text has no state change.

### Input and focus

n/a — static text takes no pointer or key input. Selectable copy of ids and paths is a Wane requirement, not mocked [W docs/ui/components.md:283].

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: the text, the colour, the size role (passed every frame) [R crates/lune-ui-core/src/widgets/mod.rs:15].
- Exit retention: no.

### Accessibility

- Role `static-text`; name and value = the text; hidden when not drawn [infra.accessibility].

### Strings

None: the label text is app text.

### Gallery scenes

- `controls` (existing): LABELS block, primary, secondary, tertiary and small caption, settled.
- `label-roles`: base, small, section and title (where the theme has it), mono, strong; settled.

### Painter primitives

- Existing: text
- New: none

### Acceptance

- Scripted: none (no input).
- Pixel probes at scale 2: 13 px base text cap height and line box match `controls` at zero delta.
- Conformance: n/a — no transition.
- Semantic node: role static-text; name equals the text passed.

### Build

- Milestone: M4 (retrofit: transitions, focus, semantic node); done by M11.
- Depends on: text.shaping, infra.accessibility.

## Open questions

- `variant-gap`: the title (18/24 Medium) and mono and strong roles have no moved code or theme field (`crates/lune-ui-core/src/theme.rs:12-41`); M4 adds the sizes or the roles.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers the base label through `controls`. Wane P0; the roles beyond base are additions.

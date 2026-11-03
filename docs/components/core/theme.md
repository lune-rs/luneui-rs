---
id: core.theme
title: Theme tokens and Lunegit dark preset
area: core
status: in-progress
consumers: luneterm
source: crates/lune-ui-core/src/theme.rs
owner: orch/toolkit-v1
summary: Theme token struct and the lunegit_dark preset with the WINDOW, PANEL and CARD glass layers
---

## What it is

`Theme` (tokens the widgets read) and the `lunegit_dark` module: `THEME`, `TINT`, `WINDOW`/`PANEL`/`CARD` layers, `INACTIVE_FALLBACK`, `CARD_CORNER_RADIUS`, `TAB_HEADER_HEIGHT`, `BODY_PADDING`.

## Needs

- Look is a `Theme` value; further looks are values, not forks (R-core-4).
- Boundary rule 3 allows the word `lunegit` only in the theme preset module (reserved slot in `scripts/boundaries.allow`; no live entry yet).

## Existing code

`crates/lune-ui-core/src/theme.rs:11` (`Theme`), `:47` (`DARK`, the neutral alias), `:50` (`lunegit_dark`), `:55` (`THEME`), `:88` (`TINT`), `:91`/`:98`/`:105` (layers); 118 lines. No tests.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `crates/lune-ui-core/src/theme.rs:11-117` (`Theme` struct, `lunegit_dark::THEME`, glass layers, constants);
Lunegit `src/Lunegit.Desktop/Themes/DarkTheme.axaml:226-233` (glass levels), `src/Lunegit.Desktop/App.axaml:151-155` (animation durations), `src/Lunegit.Desktop/SampleThemes/dark-modern.json:95-96` (the JSON theme format, for M10).
Conflicts: the repo menu padding (6) and Lunegit's (MenuItem 10,6) differ; overlay.menu names the winner (the moved code). The theme's values are the moved code's (pixel-gated).

### Anatomy

- `Theme` struct: colours (`fg_*`, `surface_*`, `border`, `accent`, dialog and menu tokens) and font sizes (`font_s` 11, `font_base` 13, `font_m` 14) [R crates/lune-ui-core/src/theme.rs:11-41].
- `DARK` = `lunegit_dark::THEME` [R crates/lune-ui-core/src/theme.rs:47].
- Glass layers `WINDOW`, `PANEL`, `CARD` [R crates/lune-ui-core/src/theme.rs:91-109]; `INACTIVE_FALLBACK` [R crates/lune-ui-core/src/theme.rs:113]; `CARD_CORNER_RADIUS` 8 [R crates/lune-ui-core/src/theme.rs:115]; `TAB_HEADER_HEIGHT` 32 [R crates/lune-ui-core/src/theme.rs:116]; `BODY_PADDING` 10 [R crates/lune-ui-core/src/theme.rs:117].
- Motion tokens to add in M4 (SF5 names): durations `instant` 120, `fast` 180, `base` 250, `moderate` 350, `slow` 500 [L src/Lunegit.Desktop/App.axaml:151-155]; easings `linear`, `quad-out`, `quad-in`, `sine-in-out`, `back-out`, `cubic-out` (0.215, 0.61, 0.355, 1), `ease-in` (0.42, 0, 1, 1) [R crates/lune-ui-core/src/widgets/menu.rs:46-58]. Kinds are not theme values.
- JSON theme (M10): the Lunegit sample theme keys, for example `MenuFlyoutPresenterBackground` [L src/Lunegit.Desktop/SampleThemes/dark-modern.json:95-96].

### States

| state | look | cite |
|---|---|---|
| dark (the only theme today) | `lunegit_dark::THEME` | [R crates/lune-ui-core/src/theme.rs:47,55] |
| inactive window | `INACTIVE_FALLBACK` | [R crates/lune-ui-core/src/theme.rs:113] |

### Transitions

n/a — the theme is data; theme swaps are not animated in the moved code.

### Input and focus

n/a — no input.

### Retained state

- App-owned: the theme value, passed in (the toolkit never reads the environment, CLAUDE.md hard rule 2).
- Persistent: none.

### Accessibility

n/a — no UI node.

### Strings

n/a — no text.

### Gallery scenes

- `controls` (existing): all theme tokens in use, scale 2. [R apps/gallery/src/scenes.rs:82-93]

### Painter primitives

- Existing: none.
- Note: The painter takes the theme's colours..
- New: none.

### Acceptance

- Existing tests: none in `crates/lune-ui-core/src/theme.rs`.
- Done standard still needs: a test that `THEME` equals the Lunegit values (`DarkTheme.axaml:200-233`) for every token the pixel gate uses; the M4 motion tokens with a test per token; the M10 JSON loader with a bounded reader (R-infra-6).

### Build

- Milestone: M4 (motion tokens), M10 (JSON).
- Depends on: core.color, core.painter, motion.transitions.

## Open questions

- The theme has no JSON loader; see `infra.theme-hot-reload`.
- `TAB_HEADER_HEIGHT` and `BODY_PADDING` read as terminal-app values; whether they belong in the toolkit theme is unchecked.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

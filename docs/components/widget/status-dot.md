---
id: widget.status-dot
title: Status dot
area: widget
status: in-progress
consumers: wane
source: —
owner: toolkit-v1 M6
summary: Status dot: static 8 px circle in ok, warn, err, data or paused tone; never alone, a word beside it carries the meaning
---

## What it is

`status-dot`: 8 px circle in a tone (ok, warn, err, data, paused). Static: no glow, ring or pulse. Paused also means "not checked" and "ended" [W docs/ui/components.md:392-393].

## Needs

- Wane controls the toolkit lacks (R-widget-4).

## Existing code

`crates/lune-ui-core/src/widgets/status_dot.rs` (M6): `status_dot::paint` for widgets that embed a dot (the badge) and `Ui::status_dot` for dot and word. Gallery page `apps/gallery/src/pages/status_dot.rs`; tests `crates/lune-ui-core/tests/widget_status_dot.rs`. Wane is the only source read for this entry.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:389-401,677`; `docs/ui/mocks/css/tokens.css:12-14,49-61,107`. No Lunegit or Luneterm line for a dot was read. Conflict: Wane's tone hexes (`--ok` #6fd39a, `--warn` #f2a65a) against Lunegit's tones for the same meanings (#73C991, #E2B93D, [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:115-116]); Lunegit's win, as in `widget.badge` [D58].

### Anatomy

- Gap 8 px between the dot and its word (the mock; no Lunegit line was read for it; the badge's own dot gap stays 6 [W docs/ui/components.md:436]); visual batch 1 C2. Circle 8 px [W docs/ui/components.md:394]; radius `--radius-round` 50% [W docs/ui/mocks/css/tokens.css:107].
- Tone fills, the tokens of [D58]: `ok` #73C991 [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:116]; `warn` #E2B93D [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:115]; `err` #E06C75 [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:117]; `data` #5c8fe6 [W docs/ui/mocks/css/tokens.css:58].
- Paused: neutral; the token is `paused` #8a9099 [W docs/ui/mocks/css/tokens.css:62], added by [D58] (Wane's components text names none, [W docs/ui/components.md:392]).
- The tray icon's dot is a separate part of `platform.tray-icon` [W docs/ui/components.md:394].

### States

| state | look | cite |
|---|---|---|
| ok | circle, fill `ok` | [W docs/ui/components.md:392], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:116], [D58] |
| warn | circle, fill `warn` | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:115], [D58] |
| err | circle, fill `err` | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:117], [D58] |
| data | circle, fill `data` | [W docs/ui/mocks/css/tokens.css:58], [D58] |
| paused | circle, fill `paused` | [W docs/ui/components.md:392-393,677], [W docs/ui/mocks/css/tokens.css:62], [D58] |

Static: none; no glow, ring or pulse [W docs/ui/components.md:393].

### Transitions

n/a — static; Wane forbids glow, ring and pulse [W docs/ui/components.md:393,395].

### Input and focus

n/a — not interactive; the dot never stands alone [W docs/ui/components.md:395].

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: the tone (paused covers "not checked" and "ended" rows [W docs/ui/components.md:677]).
- Exit retention: no.

### Accessibility

- Role `image`; state `hidden` when the adjacent word carries the meaning, which Wane requires [W docs/ui/components.md:395]; name = that word otherwise.

### Strings

None: the status words come from the app.

### Gallery scenes

- `status-dot-tones`: ok, warn, err, data, paused, each beside its word, scale 2 and 1; settled.

### Painter primitives

- Existing: fill
- New: none

### Acceptance

- Pixel probes at scale 2: 16 px circle; centre colour equals the tone token.
- Conformance: n/a — no transition.
- Semantic node: decorative dot is hidden; a dot with no adjacent word is a bug the test catches.

### Build

- Milestone: M6
- Depends on: core.painter

## Open questions

- `token-gap` resolved by [D58]: Wane's components text gives paused the neutral tone and names no token [W docs/ui/components.md:392]; its CSS defines `--paused` #8a9099 [W docs/ui/mocks/css/tokens.css:62], which becomes the `paused` token (M4). Nothing open.

## Notes for the planner

Wane P0. Nothing to port from Lunegit.

---
id: widget.badge
title: Badge
area: widget
status: in-progress
consumers: wane
source: —
owner: toolkit-v1 M6
summary: Badge: static 18 px tag with tone fill and border; neutral, ok, warn, err, data; Lunegit status-badge shape
---

## What it is

`badge`: static tag, 18 px high, radius 8, 1 px tone border, 11 px Medium text; tones neutral, ok, warn, err, data. Wane's luneui-rs entry is new and the list-row stub splits out the badge half [W docs/ui/components.md:440-441].

## Needs

- Wane controls the toolkit lacks (R-widget-4).

## Existing code

None in luneui-rs or Luneterm's `lune-ui` [W docs/ui/components.md:440]. Design source, not code to move: Lunegit `src/Lunegit.Desktop/Views/DiffView.axaml:12-16` (shape).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:431-443` (Wane); `docs/ui/mocks/css/tokens.css:12-16,20-27,49-61,111`; Lunegit `src/Lunegit.Desktop/Views/DiffView.axaml:12-16`, `src/Lunegit.Desktop/Themes/DarkTheme.axaml:115-129`, `src/Lunegit.Desktop/App.axaml:221-223`; this repo `apps/gallery/src/scenes.rs:328` (Medium weight).
Conflict: (1) tone values. Wane tokens `--ok` #6fd39a, `--warn` #f2a65a, `--err` #e06c75, `--data` #5c8fe6 [W docs/ui/mocks/css/tokens.css:49,52,55,58]; Lunegit's file-status tones are #73C991 (added), #E2B93D (modified), #E06C75 (deleted), #61AFEF (renamed), #F14C4C (conflicted) [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:115-119]. Lunegit draws this badge, so its values win [D58]: `ok` #73C991, `warn` #E2B93D, `err` #E06C75 (the added, modified and deleted tones, [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:115-117]); Wane's #6fd39a and #f2a65a are not used. Lunegit has no tone for the `data` meaning, so it keeps Wane's `data` #5c8fe6 and `data-hi` #8fbbff [D58]. Which status stands for which tone beyond that, and Conflicted, is an Open question. (2) Height: Lunegit gives padding 6,1 [L src/Lunegit.Desktop/Views/DiffView.axaml:15]; Wane's 18 px height governs the vertical size.

### Anatomy

- Height 18; horizontal padding 6; radius 8; 1 px border [W docs/ui/components.md:436], [L src/Lunegit.Desktop/Views/DiffView.axaml:13-15], [W docs/ui/mocks/css/tokens.css:111].
- Text 11 px at `font_s` [R crates/lune-ui-core/src/theme.rs:82], Medium weight [R apps/gallery/src/scenes.rs:328]; line 16 [W docs/ui/components.md:436].
- Leading dot optional, 6 px gap [W docs/ui/components.md:436]; the dot is `widget.status-dot`.
- Tone recipe: fill at 1A, border at 44, text in the tone (data uses `data_hi`) [W docs/ui/components.md:436], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:120-127]; the tone tokens `ok`, `warn`, `err`, `data`, `data_hi` are added by [D58]; `data_hi` is #8fbbff [W docs/ui/mocks/css/tokens.css:59].
- Neutral: fill `--surface-overlay` #ffffff15 [W docs/ui/mocks/css/tokens.css:22], border `--border-subtle` [W docs/ui/mocks/css/tokens.css:25], text secondary [W docs/ui/mocks/css/tokens.css:13].

### States

| state | look | cite |
|---|---|---|
| neutral | fill `surface_overlay`, border `border_subtle`, text `fg_secondary` | [W docs/ui/components.md:436], [W docs/ui/mocks/css/tokens.css:13,22,25] |
| ok | fill `ok` #73C991 at 1A, border at 44, text `ok` | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:116,121,126], [D58] |
| warn | fill `warn` #E2B93D at 1A, border at 44, text `warn` | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:115,120,125], [D58] |
| err | fill `err` #E06C75 at 1A, border at 44, text `err` | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:117,122,127], [D58] |
| data | fill `data` #5c8fe6 at 1A, border at 44, text `data_hi` | [W docs/ui/mocks/css/tokens.css:58-61], [D58] |

Static: no interactive states [W docs/ui/components.md:435].

### Transitions

n/a — static label; Wane lists no state change [W docs/ui/components.md:435,437].

### Input and focus

n/a — not interactive; no pointer, key or Tab stop [W docs/ui/components.md:437].

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: text, tone, leading dot flag (passed every frame).
- Exit retention: no.

### Accessibility

- Role `static-text`; name = the text; the word carries the meaning, colour is not the only channel [W docs/ui/components.md:395]; no live region.

### Strings

None: the badge text is app data (live, upstream kind, verdict, risk level) [W docs/ui/components.md:437].

### Gallery scenes

- `badge-tones`: neutral, ok, warn, err, data, each with text, scale 2 and 1; settled.

### Painter primitives

- Existing: fill, ring, text
- New: none

### Acceptance

- Pixel probes at scale 2: height 36 px; radius 16 px; border 2 px at the 44 alpha tone; text 22 px (11 px at scale 1).
- Conformance: n/a — no transition.
- Semantic node: role static-text; name equals the text passed.

### Build

- Milestone: M6
- Depends on: core.painter, text.shaping

## Open questions

- `tone-map`: the tone values are settled by [D58] (ok = added, warn = modified, err = deleted; data is Wane's). Open: which Lunegit file status the app maps to which tone, and Renamed (#61AFEF) and Conflicted (#F14C4C), which have no tone. M6 confirms the mapping before drawing.

## Notes for the planner

Wane P0. Lunegit's shape and tone recipe are the source; Wane's neutral tone is Wane-only.

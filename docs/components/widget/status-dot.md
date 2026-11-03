---
id: widget.status-dot
title: Status dot
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Status dot: static 8 px circle in ok, warn, err, data or paused tone; never alone, a word beside it carries the meaning
---

## What it is

`status-dot`: 8 px circle in a tone (ok, warn, err, data, paused). Static: no glow, ring or pulse. Paused also means "not checked" and "ended" [W docs/ui/components.md:392-393].

## Needs

- Wane controls the toolkit lacks (R-widget-4).

## Existing code

None in luneui-rs or Luneterm's `lune-ui` [W docs/ui/components.md:398]. Wane is the only source read for this entry.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:389-401,677`; `docs/ui/mocks/css/tokens.css:12-14,49-61,107`. No Lunegit or Luneterm line for a dot was read; no conflict to resolve.

### Anatomy

- Circle 8 px [W docs/ui/components.md:394]; radius `--radius-round` 50% [W docs/ui/mocks/css/tokens.css:107].
- Tone fills: ok `--ok` #6fd39a [W docs/ui/mocks/css/tokens.css:49]; warn `--warn` #f2a65a [W docs/ui/mocks/css/tokens.css:52]; err `--err` #e06c75 [W docs/ui/mocks/css/tokens.css:55]; data `--data` #5c8fe6 [W docs/ui/mocks/css/tokens.css:58].
- Paused: neutral, no token named [W docs/ui/components.md:392] (see Open questions).
- The tray icon's dot is a separate part of `platform.tray-icon` [W docs/ui/components.md:394].

### States

| state | look | cite |
|---|---|---|
| ok | circle, fill `ok` | [W docs/ui/components.md:392], [W docs/ui/mocks/css/tokens.css:49] |
| warn | circle, fill `warn` | [W docs/ui/mocks/css/tokens.css:52] |
| err | circle, fill `err` | [W docs/ui/mocks/css/tokens.css:55] |
| data | circle, fill `data` | [W docs/ui/mocks/css/tokens.css:58] |
| paused | circle, neutral fill (token open) | [W docs/ui/components.md:392-393,677] |

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

- `token-gap`: Wane gives paused the neutral tone but names no token [W docs/ui/components.md:392]. M6 picks a token and cites it.

## Notes for the planner

Wane P0. Nothing to port from Lunegit.

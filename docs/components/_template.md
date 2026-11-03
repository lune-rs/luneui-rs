---
id: <area>.<slug>
title: <Human title>
area: <area>
status: planned
consumers: any
source: —
owner: —
summary: <one line, what it is>
---

## What it is

<Two or three sentences. State only what you verified; put unknowns under Open questions.>

## Needs

Requirements the component must meet, each citing an `R-` id from `docs/requirements.md`.

- <need> (R-<area>-<n>)

## Existing code

<path:line, or "none">

## Spec

Every subsection below is present. One that does not apply reads `n/a — <reason>`. An `external`
entry keeps only Evidence, Reusable parts and Build (see the README).

### Evidence

Pinned: Lunegit <rev>, Wane <rev>, this repo <rev>. Read: <files and line ranges>.
Conflicts: <each conflict between sources and the winner, or "none">.

### Anatomy

- <part, size, token> [L <path>:<n>] [W <path>:<n>] [R <path>:<n>]

### States

| state | look | cite |
|---|---|---|
| rest | <look> | [L <path>:<n>] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| <change> | <property> | <kind> | <duration> | <easing> | <keep, fade or snap> | [L <path>:<n>] |

Never animated: <what>.

### Input and focus

- Pointer: <senses and actions>. Keys: <keys per state>. Tab stop: <yes or no>. Layer: <Base, Page, Popup, Modal or Toast>.
- Overlays: focus scope, dismiss rules, pointer capture [core.focus] [core.ui].

### Retained state

- Transient: <hover, press, animation phases>.
- Persistent: <scroll offset, selection keys, expansion keys, edit state, or none>.
- App-owned: <values the app passes each frame>.
- Exit retention: <yes: Full / Reduced / Off ms, or no>.

### Accessibility

- Role <role>; name <source>; value <source>; states <list>; actions <list> [infra.accessibility].

### Strings

- `luneui.<slug>.<name>`: "<English default>" [L src/Lunegit.Infrastructure/Localization/en.json:<n>]

### Gallery scenes

- `<slug>-<what>`: <state setup>, t = <ms or settled>

### Painter primitives

- Existing: <from the list in core.painter>
- New: <from the list in core.painter, or none>

### Acceptance

- <behaviour test, pixel probe, conformance line, semantic-node test>

### Build

- Milestone: <M4 to M11>
- Depends on: <entry ids>

## Open questions

- <question, or "None.">

## Notes for the planner

<Anything a planner picking this up should know first.>

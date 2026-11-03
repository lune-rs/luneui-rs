---
id: infra.localization
title: Localization
area: infra
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Infrastructure/Localization/en.json
owner: —
summary: JSON localization with en plus nine sample languages (rough stub)
---

## What it is

Rough stub — not analysed. `en.json` plus sample languages de, es, fr, ja, ko, ms, pt-BR, ru, zh-CN; a loader and a `Tr` markup extension on the desktop side.

## Needs

- Localization (R-infra-1).
- Fonts and shaping for CJK text (R-text-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Infrastructure/Localization/en.json`
- `src/Lunegit.Infrastructure/Localization/JsonLanguageRegistry.cs`
- `src/Lunegit.Desktop/Localization/LocalizationSource.cs`
- `src/Lunegit.Desktop/SampleLanguages/`

## Open questions

- Does the toolkit own string lookup, or only accept strings from the app?
- Plural and format rules.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

---
id: motion.animation-tiers
title: Animation tiers and reduced motion
area: motion
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Themes/Animations.axaml
owner: —
summary: Normal and reduced animation tiers applied at run time (rough stub)
---

## What it is

Rough stub — not analysed. About 120 transition and animation entries in `Animations.axaml`, a reduced tier, and an applicator that switches them.

## Needs

- Transitions and a reduced-motion tier (R-motion-2).
- Time comes from the app (R-nfr-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Themes/Animations.axaml`
- `src/Lunegit.Desktop/Themes/AnimationsReduced.axaml`
- `src/Lunegit.Desktop/Theming/AnimationApplicator.cs`

## Open questions

- Which properties animate and which easing curves are used.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

---
id: widget.star-rating
title: Star rating
area: widget
status: in-progress
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Controls/StarRatingControl.axaml
owner: toolkit-v1 M6
summary: Star rating: row of star buttons (default five) in the warning tone; click N sets N, click again clears
---

## What it is

`star-rating`: a row of star buttons, `Max` stars (default 5). Filled stars use the warning tone, empty stars the tertiary tone. Clicking star N sets the value to N; clicking the current value sets it to 0 (unrated) [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml.cs:9-11,21,53-67].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-2).
- Keyboard selection (R-widget-3); semantic node (R-infra-4).

## Existing code

`src/Lunegit.Desktop/Controls/StarRatingControl.axaml` (styles and a `StackPanel`), `src/Lunegit.Desktop/Controls/StarRatingControl.axaml.cs` (stars built in code, `Value` and `Max` properties). No other Lunegit or Wane star component.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `src/Lunegit.Desktop/Controls/StarRatingControl.axaml:8-36`; `src/Lunegit.Desktop/Controls/StarRatingControl.axaml.cs:9-69`.
Instrument: Wane has no star component: `git grep -i star` over `docs/ui/components.md` at 69931d6 shows no star widget. No conflict.

### Anatomy

- Star: `StarFull` icon, 20 x 20 [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml.cs:51-55].
- Star button: transparent, no border, padding 2, centred content, hand cursor [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:9-15].
- Spacing between stars: 2 [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:36].
- Default count: `Max` = 5 [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml.cs:21].
- Filled star: `StatusWarning` [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:29]; empty star: `ForegroundTertiary` [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:32].
- Glyphs: `STAR_FULL` for the stars within the value, `STAR_EMPTY` (the outlined star) for the rest, both 20 px [D120].
- Hover: the hovered star button's background `SurfaceHover`, radius 4 [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:21-23].

### States

| state | look | cite |
|---|---|---|
| empty star | `fg_tertiary` glyph | [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:31-33] |
| filled star | warning-tone glyph | [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:28-30] |
| hover (star) | `surface_hover` behind that star, radius 4 | [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:21-23] |
| pressed | not styled by Lunegit (Fluent default); open | [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:8-24] |
| focus-visible | not styled by Lunegit (Fluent default); open | [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:8-24] |
| disabled | not styled by Lunegit; open | [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml:8-24] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | star background | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [D40] |

Lunegit animates no star button (no selector in `src/Lunegit.Desktop/Themes/Animations.axaml`), so the row carries [D40]. A value change snaps: it is a discrete choice, not a sweep. Never animated: nothing here is high-frequency [L .claude/docs/animation-rules.md:148].

### Input and focus

- Pointer: click star N sets `Value` to N; clicking the current value sets 0 [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml.cs:64-67].
- Keyboard: Lunegit's stars are buttons that take Enter and Space, one tab stop per star [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml.cs:59-62]. The toolkit's row is one roving stop (a radio group has one) [D120]: Tab enters once, on the star of the value (the first when unrated); Left, Right, Home and End move focus only, not the value, and do not wrap; Enter and Space set the focused star, or clear it when it is the current value.
- Layer: `Base`; no overlay, no focus scope.

### Retained state

- Transient: hover, press [core.ui].
- Persistent: none.
- App-owned: `Value` (two-way bindable int) and `Max` (default 5) [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml.cs:16-34].
- Exit retention: no.

### Accessibility

- Role `radio-group` of `radio` items, one per star; each radio's name = "N of Max"; checked follows `Value`; action `default` sets the value [infra.accessibility]. Proposed: Lunegit's stars are buttons with a toggle-off on re-click, which a radio cannot express; see Open questions.

### Strings

None: the star names are numbers, and the control has no text keys [L src/Lunegit.Desktop/Controls/StarRatingControl.axaml.cs:9-69].

### Gallery scenes

- `star-rating-states`: value 0, value 3 of 5, hover on star 4, max 5; settled.
- `star-rating-hover-mid`: star 4 rest → hover, t = 60 ms (half of `instant`).

### Painter primitives

- Existing: fill, icon
- New: none

### Acceptance

- Scripted: click star N sets N; a second click on N sets 0; `Max` changes the count.
- Pixel probes at scale 2: star 40 x 40 px button area (20 px glyph plus padding 2); hover fill radius 8 px; spacing 4 px.
- Conformance: hover fill over 60 Hz samples settles within 120 ms + 1 frame.
- Semantic node: role radio-group; checked follows `Value`; injected `AccessAction` default sets the value.

### Build

- Milestone: M6
- Depends on: widget.button, core.focus, infra.accessibility, core.ui, motion.transitions

## Open questions

- Resolved [D120] `role-choice`: role `radio-group` of `radio`s; setting the current value clears it to 0, by click, Enter, Space or the default action (no separate clear action).
- Resolved [D120] `key-gap`: the arrows move focus only; they never set the value (a rating is a deliberate choice, and an arrow pass over the stars must not rate).

## Notes for the planner

Lunegit-only (consumers lunegit). Pinned values are the drawn Lunegit ones.

---
id: widget.button
title: Button
area: widget
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/button.rs
owner: orch/toolkit-v1
summary: Button: primary and secondary fills, measured width, hover and press motion
---

## What it is

`button` (label button in the rect the caller gives) and `button_width` (its measured width). Wane adds ghost, icon, sm and fill variants [W docs/ui/components.md:294].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Enter and Space activation, Tab stop and focus ring (R-widget-3).
- Semantic node with the button role (R-infra-4).
- Hover and press motion under the animation tiers (R-motion-2).

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:146` (`button`, 24 lines) and `:171` (`button_width`). Drawn in the gallery's `controls` scene (`apps/gallery/src/scenes.rs:240-248`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:146-174`;
`crates/lune-ui-core/src/theme.rs:12-41,67,83,85`; `apps/gallery/src/scenes.rs:240-248`; Wane `docs/ui/components.md:291-303`;
Lunegit `.claude/docs/animation-rules.md:95,102-104,143-150`, `src/Lunegit.Desktop/Themes/Animations.axaml:39-61`,
`src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:16-29`, `src/Lunegit.Desktop/App.axaml:460-483`,
`src/Lunegit.Desktop/Themes/DarkTheme.axaml:25-29`. Lunegit has no generic `Button` style or template (rg over `src/Lunegit.Desktop` at 33a1dd41: only classed `Button.*` selectors).
Conflicts: (1) Wane lists no rest fill for secondary; the moved code fills rest with `surface_hover` [R crates/lune-ui-core/src/widgets/button.rs:93-143]; the code wins. (2) Wane's `active` (`--surface-active`) is the pressed fill [W docs/ui/components.md:295]; the moved code draws no pressed state; this spec takes the Lunegit value [L src/Lunegit.Desktop/App.axaml:473-476] and records a token gap in Open questions. (3) Primary hover agrees across sources [R crates/lune-ui-core/src/widgets/button.rs:93-143] = [W docs/ui/components.md:295].

### Anatomy

- Rect supplied by the caller; corner radius `item_radius` 6 [R crates/lune-ui-core/src/widgets/button.rs:93-143], [R crates/lune-ui-core/src/theme.rs:85]. Height 32 (`--control-h`) [W docs/ui/components.md:296]; the gallery passes 32 [R apps/gallery/src/scenes.rs:240].
- Label: 13 px UI text at `font_base`, centred in the rect [R crates/lune-ui-core/src/widgets/button.rs:93-143] [R crates/lune-ui-core/src/widgets/label.rs:15], [R crates/lune-ui-core/src/theme.rs:83].
- Width: `button_width` = measured label width + 32 [R crates/lune-ui-core/src/widgets/button.rs:141]; Wane states the same (text + 32) [W docs/ui/components.md:296].
- Primary label colour `on_accent_text` [R crates/lune-ui-core/src/widgets/button.rs:93-143], value #FFFFFFF0 [R crates/lune-ui-core/src/theme.rs:67].
- Wane variants not in the moved code (M4 adds them): sm 24 high, radius 4, padding 0 12; icon 28 x 28 (sm 24 x 24) with 16 px glyph and 8 gap; ghost transparent with secondary text; fill = container width [W docs/ui/components.md:294,296]. Lunegit's ghost is the toolbar action: transparent at rest [L src/Lunegit.Desktop/App.axaml:460-464].

### States

| state | look | cite |
|---|---|---|
| rest, secondary | fill `surface_hover`, label `fg_primary` | [R crates/lune-ui-core/src/widgets/button.rs:93-143] |
| hover, secondary | fill `surface_selected` | [R crates/lune-ui-core/src/widgets/button.rs:93-143], [W docs/ui/components.md:295] |
| rest, primary | fill `accent`, label `on_accent_text` | [R crates/lune-ui-core/src/widgets/button.rs:93-143] |
| hover, primary | fill `accent_hover` | [R crates/lune-ui-core/src/widgets/button.rs:93-143] |
| pressed | fill `surface_active` (#20FFFFFF); not drawn by the moved code | [W docs/ui/components.md:295], [L src/Lunegit.Desktop/App.axaml:473-476], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:29] |
| focus-visible | 1 px ring `fg_secondary`, inset; not drawn by the moved code | [W docs/ui/components.md:295], [R crates/lune-ui-core/src/widgets/toggle.rs:103-147] (ring colour mapping) |
| disabled | opacity 0.4, inert | [W docs/ui/components.md:295], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [L src/Lunegit.Desktop/Themes/Animations.axaml:39-47], [W docs/ui/components.md:297] |
| press ↔ release | scale 0.98 | press | fast | quad-out | snap | [L .claude/docs/animation-rules.md:95,103], [L src/Lunegit.Desktop/Themes/Animations.axaml:49-61], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:16]; generic Button has no selector [D40] |
| focus-visible on/off | ring opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |

Lunegit animates classed buttons only (`toolbar-action`, `commit-primary`, `empty-state-action`); the generic button gets the same rows under [D40]. Never animated: virtualized rows on scroll, high-frequency content, layout of large surfaces [L .claude/docs/animation-rules.md:145-149].

### Input and focus

- Pointer: click activates; hover over the rect [R crates/lune-ui-core/src/widgets/button.rs:136]; pointer capture while pressed [core.input].
- Keyboard: Enter and Space activate; Tab stop yes; focus-visible after keyboard navigation [W docs/ui/components.md:297], [core.focus].
- Layer: whatever layer the app draws it on; no overlay, no focus scope.
- Disabled: no hit, no activation [W docs/ui/components.md:295].

### Retained state

- Transient: hover, press scale, focus-ring opacity [core.ui].
- Persistent: none.
- App-owned: label text, `primary` flag, the hit value (passed every frame) [R crates/lune-ui-core/src/widgets/button.rs:118-123].
- Exit retention: no.

### Accessibility

- Role `button`; name = the label text the app passes; description none; states focusable, focused, disabled; action `default` activates [infra.accessibility], [W docs/ui/components.md:297].

### Strings

None: the label is app text.

### Gallery scenes

- `controls` (existing, scale 2 and 1): primary and secondary at rest.
- `button-states`: primary and secondary at rest, secondary hovered, pressed, focus-visible, disabled; settled.
- `button-hover-mid`: secondary rest → hover, t = 60 ms (half of `instant`).
- `button-press-mid`: pressed, t = 90 ms (half of `fast`).
- `button-focus-mid`: focus-visible on, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill, text
- New: none

### Acceptance

- Scripted Enter and Space each activate once; a disabled button reports no activation [core.ui].
- Pixel probes at scale 2: radius 12 px (6 at scale 1); primary rest centre fill = `accent`; secondary rest fill = `surface_hover` over the glass.
- Conformance: hover fill over 60 Hz samples is monotonic, ends exact, settles within 120 ms + 1 frame; Reduced snaps the press scale and keeps the hover tween.
- Semantic node: role button; injected `AccessAction` default activates.

### Build

- Milestone: M4 (retrofit: transitions, focus, semantic node); done by M11. Deferred [D74]: the variants (small, icon, ghost), which no M4 scene needs.
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions.

## Open questions

- `token-gap` resolved by [D58] and built in M4: `Theme.surface_active` exists (`crates/lune-ui-core/src/theme.rs`), #FFFFFF20 [W docs/ui/mocks/css/tokens.css:24], equal to Lunegit's `SurfaceActive` #20FFFFFF [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:29]. Nothing open.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers the primary and secondary fills in `controls`. Wane P0; the variants in Anatomy are M4 additions.

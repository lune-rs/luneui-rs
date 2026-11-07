---
id: widget.button
title: Button
area: widget
status: done
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/button.rs
owner: —
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
- Primary label colour `fg_primary`, secondary and ghost `fg_secondary` (Lunegit's `dialog-action`, `App.axaml:539-586`, R4, D269).
- Wane variants not in the moved code (M4 adds them): sm 24 high, radius 4, padding 0 12; icon 28 x 28 (sm 24 x 24) with 16 px glyph and 8 gap; ghost transparent with secondary text; fill = container width [W docs/ui/components.md:294,296]. Lunegit's ghost is the toolbar action: transparent at rest [L src/Lunegit.Desktop/App.axaml:460-464].

### States

| state | look | cite |
|---|---|---|
| rest, primary | fill `surface_input` (#0AFFFFFF), label `fg_primary` | [L src/Lunegit.Desktop/App.axaml:539-586] |
| rest, secondary and ghost | no fill (clear, in the hover colour), label `fg_secondary` | [L src/Lunegit.Desktop/App.axaml:539-586] |
| hover, any kind | fill `surface_hover` (#10FFFFFF) | [L src/Lunegit.Desktop/App.axaml:539-586] |
| pressed, any kind | fill `surface_active` (#20FFFFFF); the ghost also scales by 0.98, the others do not | [L src/Lunegit.Desktop/App.axaml:539-586], [L src/Lunegit.Desktop/Themes/Animations.axaml:49-60] |
| focus-visible | the 1 px ring `fg_secondary`, inset, and nothing else changes | [R crates/lune-ui-core/src/widgets/focus_ring.rs] |
| disabled | opacity 0.4, inert | [W docs/ui/components.md:295], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [L src/Lunegit.Desktop/Themes/Animations.axaml:39-47], [W docs/ui/components.md:297] |
| press ↔ release | scale 0.98, ghost only | press | fast | quad-out | snap | [L .claude/docs/animation-rules.md:95,103], [L src/Lunegit.Desktop/Themes/Animations.axaml:49-61], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:16]; generic Button has no selector [D40] |
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
- `button-variants`: ghost, small, icon (28 and 24), icon with label, each at rest and hovered, and a fill-width button; settled.
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
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_button.rs`: activation `enter_and_space_activate_once_each`, `an_enabled_button_takes_one_activation_key_and_leaves_the_other`, `a_pointer_click_and_the_default_action_activate`, `a_disabled_button_reports_no_activation_and_is_marked_disabled`, and a chord on Enter or Space stays a shortcut (`a_chord_on_enter_or_space_is_a_shortcut_and_clicks_nothing`, `a_disabled_button_leaves_a_chord_to_the_shortcuts_as_an_enabled_one_does`, `a_disabled_widget_keeps_the_keys_it_handles_from_the_shortcuts`). Pixel probes at scale 2: `pixel_probes_at_scale_two`, `pressed_scales_about_the_centre_and_the_ring_follows_focus_visible`. Conformance: `every_motion_row_conforms_under_every_tier`, `the_ghost_and_icon_rows_conform_in_every_tier`. Node: `the_node_names_the_button_and_follows_focus`. Variants: `a_small_button_is_24_high_with_radius_4_and_keeps_the_top`, `the_ghost_look_is_transparent_hover_active_with_secondary_text`, `an_icon_button_is_a_named_ghost_that_clicks_once_by_pointer_enter_and_space`, `a_labelled_icon_draws_the_glyph_and_the_label_and_is_named_by_the_label`. The `Ui` form equals the painter form at rest: `at_rest_the_ui_button_draws_exactly_what_the_painter_button_does`.

### Build

- Milestone: M4 (retrofit: transitions, focus, semantic node); done by M11. The variants (small, icon, ghost) were deferred by [D74] and are built in M6 step 3 ([D116]): `ButtonKind::Ghost`, `ButtonOptions { kind, small, icon }` with `Ui::button_with` and `button_width_with`, `IconButton` with `Ui::icon_button`; the small button is 24 high (the top of the caller's rect kept), radius 4, padding 0 12; a labelled icon is a 16 px glyph, an 8 gap; fill width needs no code (the caller's rect is the width). The ring follows the corner radius (`RingKind::Edge`).
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_button.rs::enter_and_space_activate_once_each, crates/lune-ui-core/tests/widget_button.rs::a_disabled_button_reports_no_activation_and_is_marked_disabled, crates/lune-ui-core/tests/widget_button.rs::pixel_probes_at_scale_two, crates/lune-ui-core/tests/widget_button.rs::every_motion_row_conforms_under_every_tier, crates/lune-ui-core/tests/widget_button.rs::the_node_names_the_button_and_follows_focus, crates/lune-ui-core/tests/widget_button.rs::a_chord_on_enter_or_space_is_a_shortcut_and_clicks_nothing, crates/lune-ui-core/tests/widget_button.rs::a_small_button_is_24_high_with_radius_4_and_keeps_the_top, crates/lune-ui-core/tests/widget_button.rs::at_rest_the_ui_button_draws_exactly_what_the_painter_button_does`; page `button`; unverified here: none.

## Open questions

- `token-gap` resolved by [D58] and built in M4: `Theme.surface_active` exists (`crates/lune-ui-core/src/theme.rs`), #FFFFFF20 [W docs/ui/mocks/css/tokens.css:24], equal to Lunegit's `SurfaceActive` #20FFFFFF [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:29]. Nothing open.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers the primary and secondary fills in `controls`. Wane P0; the variants in Anatomy are M4 additions.

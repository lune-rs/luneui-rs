---
id: widget.checkbox
title: Checkbox
area: widget
status: done
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/checkbox.rs
owner: —
summary: Checkbox: 16 px box, accent fill and check when checked, label beside it
---

## What it is

`checkbox`: 16 px box with a label beside it, accent fill and check mark when checked. Picks items from a list; an on/off setting is a `widget.toggle` [W docs/ui/components.md:311].

## Needs

- Drawn through the painter and generic over the app's hit type (R-widget-1).
- Space toggles; Tab stop and focus-visible ring (R-widget-3).
- Semantic node with the checkbox role (R-infra-4).
- Hover and focus motion under the animation tiers (R-motion-2).

## Existing code

`crates/lune-ui-core/src/widgets/mod.rs:176` (`checkbox`, 22 lines). Drawn in the gallery's `controls` scene (`apps/gallery/src/scenes.rs:262-275`, checked and unchecked). The glass-ui spike calls it from `src/widgets.rs`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/mod.rs:176-197`;
`crates/lune-ui-core/src/theme.rs:62,67,83`; `apps/gallery/src/scenes.rs:262-275`; Wane `docs/ui/components.md:319-331`;
Lunegit `src/Lunegit.Desktop/Themes/DarkTheme.axaml:174-181`, `src/Lunegit.Desktop/Views/Dialogs/StashDialogView.axaml:97-100` (use site), `.claude/docs/animation-rules.md:102,104,110`.
Lunegit has no `CheckBox` style or template override (rg over `src/Lunegit.Desktop` at 33a1dd41: only the `CheckBox*` colour keys).
Conflicts: (1) Wane gives a hover fill `--surface-hover` and a focus outline [W docs/ui/components.md:323]; the moved code draws neither, so Wane supplies them. (2) Wane's min height 24 [W docs/ui/components.md:324] is a layout rule the moved code does not set; the caller's rect decides. (3) Lunegit's checked fill and border are the accent in every pointer state [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:174-179] and the moved code agrees [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132].

### Anatomy

- Box 16 x 16, vertically centred in the rect, at the rect's left edge; corner radius 3 [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132]; Wane 16 x 16 radius 3 (`--radius-check`) [W docs/ui/components.md:324].
- Unchecked: 1 px ring `fg_secondary` [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132].
- Checked: fill `accent` [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132] with a 12 px check glyph in `on_accent` [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132]; Wane checked: 12 px check [W docs/ui/components.md:323].
- Label: 13 px secondary UI text (`fg_secondary`), starts 8 px after the box [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132]; Wane label 13 secondary, gap 8 [W docs/ui/components.md:324].
- Hit area: 24 px plus the label width, full rect height [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132].
- Min height 24 [W docs/ui/components.md:324] (rect height is set by the caller).

### States

| state | look | cite |
|---|---|---|
| unchecked | 1 px ring `fg_secondary`, no fill | [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132] |
| checked | fill `accent`, check glyph `on_accent` | [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132] |
| hover (unchecked) | fill `surface_hover` behind the box | [W docs/ui/components.md:323]; not drawn by the moved code |
| hover (checked) | accent fill unchanged | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:175,178] |
| focus-visible | 1 px outline, offset 2 | [W docs/ui/components.md:323]; not drawn by the moved code |
| disabled | opacity 0.4, inert | [W docs/ui/components.md:323], [L src/Lunegit.Desktop/App.axaml:481-483] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| unchecked ↔ checked | fill and check | selection | fast | linear | keep | [L .claude/docs/animation-rules.md:110], [D40] |
| rest ↔ hover | box fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [D40] |
| focus-visible on/off | outline opacity | focus | fast | linear | keep | [L .claude/docs/animation-rules.md:104], [D40] |

Lunegit has no `CheckBox` motion selector (`Themes/Animations.axaml`), so every row is the general rule under [D40]. Never animated: virtualized rows on scroll, high-frequency content [L .claude/docs/animation-rules.md:145-149].

### Input and focus

- Pointer: click toggles; hover over the hit area [R crates/lune-ui-core/src/widgets/checkbox.rs:87-132]; pointer capture while pressed [core.input].
- Keyboard: Space toggles [W docs/ui/components.md:325]; Tab stop yes; focus-visible after keyboard navigation [core.focus].
- Layer: whatever layer the app draws it on; no overlay, no focus scope.
- Disabled: no hit, no toggle [W docs/ui/components.md:323].

### Retained state

- Transient: hover, focus-ring opacity, fill transitions [core.ui].
- Persistent: none.
- App-owned: `checked` (passed every frame), the label text [R crates/lune-ui-core/src/widgets/checkbox.rs:117-122].
- Exit retention: no.

### Accessibility

- Role `checkbox`; name = the label the app passes; description none; states checked (mixed not used), focusable, focused, disabled; action `default` toggles [infra.accessibility], [W docs/ui/components.md:325].

### Strings

None: the label is app text.

### Gallery scenes

- `controls` (existing): checked and unchecked, settled.
- `checkbox-states`: unchecked, checked, unchecked hovered, checked hovered, focus-visible, disabled; settled.
- `checkbox-fill-mid`: unchecked → checked, t = 90 ms (half of `fast`).
- `checkbox-hover-mid`: unchecked rest → hover, t = 60 ms (half of `instant`).
- `checkbox-focus-mid`: focus-visible on, t = 90 ms (half of `fast`).

### Painter primitives

- Existing: fill, ring, icon, text
- New: none

### Acceptance

- Scripted Space and click each flip `checked` once; a disabled checkbox reports no toggle [core.ui].
- Pixel probes at scale 2: box 32 x 32 px, radius 6 px; checked centre fill = `accent`; check glyph 24 px; label starts 16 px past the box edge.
- Conformance: fill change over 60 Hz samples is monotonic, ends exact, settles within 180 ms + 1 frame; Reduced keeps the fill change.
- Semantic node: role checkbox, checked follows `checked`, injected `AccessAction` default toggles.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_checkbox.rs`: `click_and_space_flip_once_each_and_mark_changed`, `a_disabled_checkbox_reports_no_click`, `pixel_probes_at_scale_two`, conformance `every_motion_row_conforms_under_every_tier`, `the_fill_rises_monotonically_and_settles_within_180_ms_and_a_frame`, `reduced_keeps_the_fill_change_and_off_snaps_everything`, node `the_node_is_a_checkbox_whose_checked_state_follows_the_value` (the default action toggles: `ui_access.rs::default_clicks_a_checkbox_and_sets_changed`), and `at_rest_the_ui_checkbox_draws_exactly_what_the_painter_checkbox_does`.

### Build

- Milestone: M4 (retrofit: transitions, focus, semantic node); done by M11.
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_checkbox.rs::click_and_space_flip_once_each_and_mark_changed, crates/lune-ui-core/tests/widget_checkbox.rs::a_disabled_checkbox_reports_no_click, crates/lune-ui-core/tests/widget_checkbox.rs::pixel_probes_at_scale_two, crates/lune-ui-core/tests/widget_checkbox.rs::the_fill_rises_monotonically_and_settles_within_180_ms_and_a_frame, crates/lune-ui-core/tests/widget_checkbox.rs::the_node_is_a_checkbox_whose_checked_state_follows_the_value`; page `checkbox`; unverified here: none.

## Open questions

- None.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers the checked and unchecked boxes in `controls`. Wane P1.

---
id: widget.toolbar
title: Main toolbar
area: widget
status: done
consumers: lunegit
source: crates/lune-ui-core/src/widgets/toolbar.rs
owner: —
summary: Toolbar: three-column command bar of 40 px toolbar-action buttons with 20 px icons on a glass material
---

## What it is

`toolbar`: Lunegit's main command bar. A three-column grid; the left group holds the 40 x 40 action buttons (fetch, pull, push, branch) with 20 px icons. Commands and labels are the app's.

## Needs

- Drawn through the painter and the glass material (R-platform-1 through `material`); generic over the app's hit type (R-widget-1).
- Action buttons: keyboard and focus (R-widget-3); semantic node (R-infra-4).

## Existing code

`src/Lunegit.Desktop/Views/ToolbarView.axaml` (220 lines; the left group and resources read, the centre and right groups not read). Action styles: `src/Lunegit.Desktop/App.axaml:460-483`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `src/Lunegit.Desktop/Views/ToolbarView.axaml:10-53,79`; `src/Lunegit.Desktop/App.axaml:460-483`; `src/Lunegit.Desktop/Themes/Animations.axaml:39-61`; `src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:20-29`.
Wane has no toolbar component (Wane lists only the ghost action, which Lunegit's toolbar style supplies [W docs/ui/components.md:299]). No conflict.
Not read: `ToolbarView.axaml:82-220` (the centre and right groups, the branch and status badges' use).

### Anatomy

- Bar: padding 0 6 top and bottom, 8 right [L src/Lunegit.Desktop/Views/ToolbarView.axaml:38]; three columns `*, Auto, *` [L src/Lunegit.Desktop/Views/ToolbarView.axaml:39].
- Left group: the actions touch (icon spacing 0), and the branch button sits one group gap (8) after the last action, in the same stack [L src/Lunegit.Desktop/Views/ToolbarView.axaml:13-14,38,41-46,96-110]. The gallery page places them so; `GROUP_GAP` is the spacing between groups, not between buttons.
- Action button: 40 x 40, padding 6, centred content [L src/Lunegit.Desktop/Views/ToolbarView.axaml:13,19,48-50].
- Action icon: 20 px, `ForegroundMuted` [L src/Lunegit.Desktop/Views/ToolbarView.axaml:11,53-54].
- Utility icon 16 px [L src/Lunegit.Desktop/Views/ToolbarView.axaml:12]; branch icon 14 px, content spacing 4, max width 420 [L src/Lunegit.Desktop/Views/ToolbarView.axaml:16-18]; branch button padding 6 horizontal, 4 vertical [L src/Lunegit.Desktop/Views/ToolbarView.axaml:20]. The branch label draws `fg_subtle` #808080 at `font_m` 14 [L src/Lunegit.Desktop/Views/ToolbarView.axaml:113,116-117], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:14], [D133].
- Glass material: acrylic with the dark tint and opacity (`GlassTintDarkOpacity`, `GlassMaterialDarkOpacity`: the preset's `WINDOW` layer, which the gallery page passes), visible while the window is active; when inactive a solid `WindowFallbackBackground` replaces it [L src/Lunegit.Desktop/Views/ToolbarView.axaml:27-37]; the fallback rule is `material` (M3 `material` entry).
- Button chrome: class `toolbar-action`, see States [L src/Lunegit.Desktop/App.axaml:460-464].

### States

| state | look | cite |
|---|---|---|
| rest | transparent | [L src/Lunegit.Desktop/App.axaml:465-468] |
| hover | fill `surface_hover`, radius 6 | [L src/Lunegit.Desktop/App.axaml:469-472] |
| pressed | fill `surface_active` | [L src/Lunegit.Desktop/App.axaml:473-476] |
| focus-visible | not styled by Lunegit (Fluent default); open | [L src/Lunegit.Desktop/App.axaml:460-483] |
| disabled | transparent, opacity 0.4, inert | [L src/Lunegit.Desktop/App.axaml:477-483] |
| branch disabled | disabled when the repository cannot be written | [L src/Lunegit.Desktop/Views/ToolbarView.axaml:79] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | background | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:39-47], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:20-29] |
| press ↔ release | scale 0.98 | press | fast | quad-out | snap | [L src/Lunegit.Desktop/Themes/Animations.axaml:49-61], [L .claude/docs/animation-rules.md:103] |

Reduced keeps the background tween and drops the press scale [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:16]. Nothing else in the bar is animated; the rules that bar motion are [L .claude/docs/animation-rules.md:143-150].

### Input and focus

- Pointer: click runs the button's command; hover over the 40 x 40 rect.
- Keyboard: Tab through the buttons; Enter and Space activate as buttons do [core.focus].
- Tooltip on each button (`ToolTip.Tip`) [L src/Lunegit.Desktop/Views/ToolbarView.axaml:52].
- Layer: `Base`; no overlay, no focus scope.
- Pointer capture while pressed [core.input].

### Retained state

- Transient: hover, press [core.ui].
- Persistent: none.
- App-owned: each command and its enabled flag (the branch button's `CanWrite`), the window's active flag for the material (passed every frame).
- Exit retention: no.

### Accessibility

- Role `toolbar` containing `button`s; each button's name = its tooltip text the owner passes; states focusable, focused, disabled; action `default` runs the command [infra.accessibility].

### Strings

None. The tooltips and labels are the app's commands: `Ui::toolbar_action` and `Ui::toolbar_label` take them as arguments, so the entry's `luneui.toolbar.*` keys are not added ([D128]).

### Gallery scenes

- `toolbar-actions`: three action buttons and the branch label button, rest and hover on the second, its tooltip shown; settled.
- `toolbar-inactive`: the window inactive, fallback background shown; settled.
- `toolbar-action-mid`: press on the first action, t = 90 ms (half of `fast`).
- `toolbar-hover-mid`: second action rest → hover, t = 60 ms (half of `instant`).

### Painter primitives

- Existing: fill, icon, text, clip-rect
- New: none

### Acceptance

- Scripted: each action runs its command once; disabled buttons do not run.
- Pixel probes at scale 2: action button 80 x 80 px; icon 40 px; hover radius 12 px.
- Conformance: press scale over 60 Hz samples settles within 180 ms + 1 frame; Reduced keeps the background tween and snaps the scale.
- Semantic node: role toolbar; each button named by its tooltip.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_toolbar.rs`: `each_action_runs_once_by_click_and_by_key`, `a_disabled_action_does_not_run_and_is_marked_disabled`, `pixel_probes_at_scale_two`, `the_columns_are_the_padded_grid_with_a_zero_wide_middle`, `active_draws_the_glass_layer_and_no_fallback_fill`, `inactive_draws_the_window_fallback_fill_and_no_acrylic`, conformance `press_and_hover_conform_and_reduced_snaps_only_the_scale`, tooltip `a_tooltip_shows_after_the_delay_and_names_the_action`, label button `a_short_label_is_drawn_whole_and_a_long_one_is_elided`, `the_label_button_runs_once_and_has_a_node_named_by_the_owner`, node `the_bar_is_a_toolbar_node_holding_the_named_actions`.

### Build

- Milestone: M6
- Depends on: widget.button, material, core.focus, infra.accessibility, motion.transitions
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_toolbar.rs::each_action_runs_once_by_click_and_by_key, crates/lune-ui-core/tests/widget_toolbar.rs::a_disabled_action_does_not_run_and_is_marked_disabled, crates/lune-ui-core/tests/widget_toolbar.rs::pixel_probes_at_scale_two, crates/lune-ui-core/tests/widget_toolbar.rs::inactive_draws_the_window_fallback_fill_and_no_acrylic, crates/lune-ui-core/tests/widget_toolbar.rs::press_and_hover_conform_and_reduced_snaps_only_the_scale, crates/lune-ui-core/tests/widget_toolbar.rs::the_bar_is_a_toolbar_node_holding_the_named_actions`; page `toolbar`; unverified here: none.

## Open questions

None. Closed by [D128]: `read-gap` (the centre and right groups are the caller's: `Ui::toolbar` hands over the `*, Auto, *` columns and the branch button's shape is `Ui::toolbar_label`; the middle column is zero wide, a centred group is placed around its x), and `inactive-fallback` (`Theme.window_fallback` is #333333FF, Lunegit `DarkTheme.axaml:245`). Focus-visible stays the button's ring.

## Notes for the planner

Renamed from `lunegit.toolbar` (D10 mapping, step 8). Lunegit's commands are app code and stay there.

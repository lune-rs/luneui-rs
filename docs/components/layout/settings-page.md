---
id: layout.settings-page
title: Settings page layout
area: layout
status: done
consumers: luneterm
source: crates/lune-ui-core/src/widgets/settings_page.rs
owner: —
summary: Settings page shell: a 20 px content host for the current section, a restart banner that reveals on demand, and the rows and sections built from toolkit widgets
---

## What it is

The settings page shell: a glass page with a content host (20 px inset) that shows the current section, and a bottom restart banner that reveals when a change needs a restart [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:9-60]. Luneterm M10 lists a live settings page with persistence. The widgets exist (`widget.toggle`, `widget.combo`, `widget.stepper`, `widget.text-field`); a page layout for them is not in `lune-ui` today per the inventory.

## Needs

- Existing form widgets (R-widget-1).
- Layout for sections and rows (R-layout-1).

## Existing code

The glass-ui spike has `spikes/glass-ui/src/settings_view.rs` (file exists; read for the nav and group geometry in this entry's Evidence). Lunegit: `src/Lunegit.Desktop/Views/SettingsPageView.axaml` and `Views/Settings/*SettingsView.axaml`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b, Luneterm 296cfdc. Read: Lunegit `src/Lunegit.Desktop/Views/SettingsPageView.axaml:1-62`;
`src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:146-164` (a settings row: `settings-item`, text, toggle);
`src/Lunegit.Desktop/Themes/Animations.axaml:113-121` (`settings-item` hover), `:383-394` (`reveal-anim`, Full);
`src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:253-265` (`reveal-anim`, Reduced);
Luneterm `spikes/glass-ui/src/settings_view.rs:209` (nav column) and `:280` (`draw_group`) (`git show 296cfdc`).
Conflicts: none between sources; this repo has no settings page. The eight section views are described in `lunegit.settings-pages`.

### Anatomy

- Glass: acrylic at the top-left corner radius 8 only, PANEL levels (tint 0.8, material 0.7) [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:10-18]; Lunegit's solid fallback while the window is inactive [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:20] is not ported (R13, D262).
- Banner: docked bottom; padding 20 by 10; fill `StatusWarningDark`; text `StatusWarningLight`; 14 px warning icon 8 px from the text [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:24-40].
- Banner action: a button with padding 12 by 6, radius 6, 1 px `StatusWarningLight` border [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:41-51].
- Content host: 20 px margin on all sides, showing the current section [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:56-58].
- Row: a bordered row with padding 12 by 12 and a 12 px side margin, title at base size, description at small size in muted colour, control at the right with a 16 px left margin [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:148-163].
- Sections: eight section views in `Views/Settings/` (advanced, AI sessions, appearance, general, git, logs, storage, terminal) [L src/Lunegit.Desktop/Views/Settings] (directory listing at 33a1dd41).
- Luneterm spike: nav column at 10 px inset, 190 wide [LT spikes/glass-ui/src/settings_view.rs:209]; group draw [LT spikes/glass-ui/src/settings_view.rs:280].

### States

| state | look | cite |
|---|---|---|
| page, rest | glass page; content host | [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:9-58] |
| inactive window | the same glass (no `active` field) | [D262] |
| restart required | banner revealed at the bottom | [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:24-27] |
| row hover | `settings-item` background tween | [L src/Lunegit.Desktop/Themes/Animations.axaml:113-121] |
| focus-visible | the focused control (widget.*) | [core.focus] |
| disabled row | the row is `with_enabled(false, ..)`: the row's and its control's nodes are `Disabled`, the hover fill and the control's input are off (`widget.row`); the banner's action takes the same rule | [D152] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| restart banner shown | opacity 0→1 | appear | base | quad-out | keep (opacity; Fast in Reduced) | [L src/Lunegit.Desktop/Themes/Animations.axaml:383-394], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:253-265] |
| row rest ↔ hover | background | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:113-121] |

The banner hides with the same fade back (`REVEAL`, retargeted to 0): it keeps its space until it is gone, then the content's height snaps. A banner hidden in flight takes no input and has no node. The first frame with the flag set shows it at once.

Never animated: the section content swap (no motion in the view, [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:56-58]) and the controls' values (high-frequency content, [L .claude/docs/animation-rules.md:148]).

### Input and focus

- Pointer: rows and their controls take the pointer; the banner's action runs the restart command [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:41-51]: `SettingsResponse.restart` is true on the frame it was pressed, once. The action's hover and press are the button fills (`surface_hover`, `surface_active`) over its `status_warning_dark`, which the source does not state [D152].
- Keyboard: none on the page; Tab moves through the section's controls in order, then the banner's action (Enter and Space press it) [core.focus].
- Tab stop: the controls; the banner's action when shown [core.focus]; its focus ring is the button's 1 px edge ring.
- Layer: Page [derived: the page is the content layer, crates/lune-ui-core/src/scene.rs:17-22].
- Focus scope: none; a section change moves focus to its first control: open question.
- Pointer capture while pressed: none.

### Retained state

- Transient: row hover; banner reveal progress [motion].
- Persistent: none on the page; the scroll offset of a long section is owned by the section's scroll area (layout.scroll-area) [core.ui].
- App-owned: the current section, the restart flag, every setting value; persistence (JSON, atomic, versioned) is app logic, out of scope [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:25,56-58].
- Exit retention: no.

### Accessibility

- Page: role `group`, name = the section title (app text) [infra.accessibility].
- Banner: `status` node, polite, shown with the restart message; its action is a `button` [infra.accessibility].
- Rows: `group` with the title as name and the description as description; the control keeps its own role (switch, combo-box, spin-button, text-input) [infra.accessibility].
- Live region: the banner (polite).

### Strings

- Banner text and action: Lunegit `settings.restart.message` and `settings.restart.action` [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:36,42]; the English values are not read here; the key names are the mapping (`luneui.settings-page.restart-message`, `luneui.settings-page.restart-action`). Default text: open question.
- Row title and description: app text (Lunegit `settings.general.updates.checkOnStartup.title` and `.desc`, [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:151-155]).

### Gallery scenes

- `settings-page-rows` (new): a page with three rows (a toggle, a combo, a stepper), no banner, settled.
- `settings-page-restart` (new): the banner revealed at the bottom, settled.
- `settings-page-restart-mid` (new): the banner 125 ms into its reveal (half of base, 250 ms) [L src/Lunegit.Desktop/App.axaml:153].
- `settings-page-disabled` (new): the first row in `with_enabled(false, ..)` (0.4 opacity), the pointer and a click on its toggle: no hover fill, the toggle unchanged, no restart banner, settled.

### Painter primitives

- Existing: `acrylic_radii` (the corner-only radius; built in M7 step 1), `fill_radii` (the fallback's corner), fill, ring, text, icon. [R crates/lune-ui-core/src/paint.rs]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): `crates/lune-ui-core/tests/widget_settings_page.rs`: a row hover fills it; the banner reveals when the restart flag is set and hides when cleared (keeping its space until gone); the banner action fires once, by pointer, Enter or Space; a hidden banner takes no height and has no node; the content scrolls through a scroll area; a three-row section keeps the toggle, combo and stepper roles under `Group` rows and the combo's list draws on the popup layer above the page; a disabled page passes the walk.
- Pixel probes at scale 2: content inset 40 px; banner padding 40 px sides and 20 px top and bottom; page corner radius 16 px at the top-left only.
- Conformance: the banner reveals in 250 ms, quad-out, opacity only; Reduced reveals in 180 ms; Off snaps.
- Semantic node: `group` page; banner `status` (polite) holding the action `button`; rows `group`.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_settings_page.rs`: behaviour `a_row_lights_on_hover_and_the_control_keeps_the_press`, `a_hidden_banner_takes_no_height_and_no_node`, `the_action_fires_once_per_press`, `enter_and_space_press_the_focused_action`, `the_content_scrolls_through_a_scroll_area`, `the_section_keeps_its_roles_under_group_rows`, `a_combos_list_draws_above_the_page`; pixel probes `pixel_probes_at_scale_two`, `the_glass_is_the_acrylic_with_one_round_corner`, `the_host_is_inset_20_and_the_banner_docks_below_it`, ; conformance `the_banner_fades_in_over_base_with_quad_out`, `reveal_conforms_in_three_tiers`, `reduced_fades_over_fast_and_off_snaps`; node `the_banner_is_a_polite_status_holding_the_action_button`; disabled `a_disabled_page_is_disabled_all_the_way_down`.

### Build

- Milestone: M7.
- Depends on: layout.sidebar, layout.panel, widget.row, widget.toggle, widget.combo, widget.stepper, widget.text-field, widget.button, motion.transitions.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_settings_page.rs::a_row_lights_on_hover_and_the_control_keeps_the_press, crates/lune-ui-core/tests/widget_settings_page.rs::the_action_fires_once_per_press, crates/lune-ui-core/tests/widget_settings_page.rs::the_section_keeps_its_roles_under_group_rows, crates/lune-ui-core/tests/widget_settings_page.rs::pixel_probes_at_scale_two, crates/lune-ui-core/tests/widget_settings_page.rs::reveal_conforms_in_three_tiers, crates/lune-ui-core/tests/widget_settings_page.rs::the_banner_is_a_polite_status_holding_the_action_button, crates/lune-ui-core/tests/widget_settings_page.rs::a_disabled_page_is_disabled_all_the_way_down`; page `settings-page`; unverified here: none.

## Open questions

None. Closed by [D152]: the page is a shell (`Ui::settings_page`): the glass, the 20 px content host and the restart banner; the section nav is the app's (Lunegit's is in the side panel), the rows are `Ui::setting_row`, and persistence is the app's. The banner strings are `luneui.settings-page.restart-message` and `.restart-action` with Lunegit's English (`en.json:74-75`). A disabled row is `with_enabled(false, ..)` around the row. The content closure returns the section's height, which is what the scroll area scrolls.

## Notes for the planner

See `lunegit.settings-pages` for the Lunegit side (external, Reusable parts). Roadmap scope: M7 for the layout; the Luneterm settings UI is M10 in the app's roadmap.

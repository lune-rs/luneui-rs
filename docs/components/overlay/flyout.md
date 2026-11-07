---
id: overlay.flyout
title: Tray flyout
area: overlay
status: done
consumers: wane
source: crates/lune-ui-core/src/widgets/flyout.rs
owner: —
summary: Windows tray left-click surface: header, stats, route list, action rows
---

## What it is

Windows tray left-click surface: header, stats, route list, action rows. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Left click on the tray icon toggles it; an outside click or losing focus closes it; the action rows behave as menu items (Open Wane, Profile, Pause compression, Unload local model, Quit Wane in the danger tone).

## Needs

- Wane — Variants: none; parts: header (mark 16, wordmark 14 bold gold, state with status dot at the right), stats (`data.stat --compact` strip), routes (`data.kv-list`, ids mono, a state note such as "loaded"), separator, action rows (`overlay.menu` items). (R-overlay-3)
- Wane — States: open (menu motion); the state text follows the tray state (Serving, degraded, paused). (R-overlay-3)
- Wane — Sizes: 340 wide, padding 5, radius 9, 1 px `--popup-border`, fill `--menu-glass` with blur 30 / saturate 1.8; header padding 6 8; stats margin 8 0; routes padding 0 8. Windows: 12 px above the taskbar and 12 px from the right edge. (R-overlay-3)
- Wane — Behaviour: Left click on the tray icon toggles it; an outside click or losing focus closes it; the action rows behave as menu items (Open Wane, Profile, Pause compression, Unload local model, Quit Wane in the danger tone). (R-overlay-3)

## Existing code

`crates/lune-ui-core/src/widgets/flyout.rs` (`Flyout`, `Ui::flyout`, `flyout_size`, `flyout_body`) over `Ui::popover` with `Surface::Glass`, `menu_rows` (the menu's row frame), `Ui::stat_strip` and `Ui::kv_list` [D145]; tests in `crates/lune-ui-core/tests/widget_flyout.rs`. M10 step 7 added `Ui::flyout_window`, the same body in a window of its own (the card fills the window, `tray::place_surface` places the window; D220). Before M7 there was none in luneui-rs or Luneterm's `lune-ui` (Wane: partial: the glass popup surface exists (`glass_panel`, widgets/popover.rs); the stats block and the separate window are missing). Design source (not code to move): Luneterm crates/lune-ui/src/widgets/menu.rs:109-111 (backdrop, glass fill, ring); Luneium src/renderer/shell/components/Popup.css:2; docs/mockups/Tray.dc.html (content).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b, Luneterm 296cfdc. Read: Wane `docs/ui/components.md:571-584`;
Luneterm `crates/lune-ui/src/widgets/menu.rs:109-111` (`git show 296cfdc`); this repo: no flyout or tray code
(`grep -rn -i 'flyout\|tray' crates apps` empty); Lunegit `src/Lunegit.Desktop/App.axaml:512` (a commit split-button flyout; no tray flyout
exists); `docs/components/overlay/menu.md` (motion).
Conflicts: none between sources. Wane `--popup-border` (flyout, 1 px) differs in name from `--popup-border-soft`
(menu, dropdown) [W docs/ui/components.md:571-584]; the Wane token for the flyout is used as written.

### Anatomy

- Window content: 340 wide [W docs/ui/components.md:571-584]; padding 5; radius 9; 1 px border `--popup-border` [W docs/ui/components.md:571-584].
- Fill: `--menu-glass`, backdrop blur 30, saturate 1.8 [W docs/ui/components.md:571-584]; the glass fill and ring are the Luneterm menu's [LT crates/lune-ui/src/widgets/menu.rs:109-111].
- Header: padding 6 8; mark 16, wordmark 14 bold gold, state text with a status dot at the right [W docs/ui/components.md:571-584].
- Stats strip: margin 8 0; the `data.stat` compact cells [W docs/ui/components.md:571-584].
- Routes: padding 0 8; `data.kv-list` rows [W docs/ui/components.md:571-584].
- Action rows: `overlay.menu` items [overlay.menu].
- Placement on Windows: 12 px above the taskbar, 12 px from the right edge [W docs/ui/components.md:571-584]; the window is placed from the tray icon's rect and kept on screen [W docs/ui/components.md:571-584]. M7 places the card from the app's anchor rect with `place`: gap 12 from the anchor, margin 12 from the window's edges, shifted inside them; M10 passes the tray rect.
- Built geometry (card width 340, content 328 wide at 6 px in: the 5 px padding and the 1 px ring): the header is 32 high (6 + a 20 px line + 6), its text 8 px in; the mark is a 16 px glyph of the icon font in `fg_primary`, 8 px before the wordmark (14 px bold `gold`); the state is 11 px on a 16 px line in `fg_secondary`, right-aligned 8 px in, with its dot 6 px before it. The stats strip (compact cells) is the content width, 50 high: a 1 px `border_subtle` outline at radius 8 (Wane's `c-panel c-panel--strip`) drawn after the cells, which sit 1 px inside it, with 8 px above and below; the routes are `Ui::kv_list` mono, inset 8 more, with 8 px above and below, their keys at `font_s` 11 (`.c-flyout__routes .c-kv dt`) and the values at 13; neighbouring margins collapse into one 8 px, as in the mock. Empty `stats` or `routes` draw nothing and take no height. A separator (the menu's 11 px row) and the rows follow when there are rows. All text is at least 14 px from the card's edge.

### States

| state | look | cite |
|---|---|---|
| closed | not drawn | [W docs/ui/components.md:571-584] |
| open, serving | header state text and dot follow the tray state "Serving" | [W docs/ui/components.md:571-584] |
| open, degraded | header state text "degraded" | [W docs/ui/components.md:571-584] |
| open, paused | header state text "paused" | [W docs/ui/components.md:571-584] |
| item hover, pressed, disabled | the `overlay.menu` item states | [overlay.menu] |
| focus-visible | the highlight: the hover fill on the lit row, no ring | [D145], [overlay.menu] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| open | as `overlay.menu` open: opacity, y offset 4→0, scale 0.97→1 | pop | fast | cubic-out | fade | [overlay.menu], [W docs/ui/components.md:571-584] |
| close | as `overlay.menu` close | dismiss | instant | ease-in | fade | [overlay.menu], [W docs/ui/components.md:571-584] |

Never animated: the stats and routes content (values change, [motion] list: high-frequency content).

### Input and focus

- Pointer: left click on the tray icon toggles the flyout (the icon is `platform.tray-icon`); an outside click closes [W docs/ui/components.md:571-584].
- Keyboard: the menu's [D145]: Up and Down wrap over the enabled rows, Home and End go to the ends, Enter and Space pick, Escape closes. Focus lands on the first enabled row at open.
- Tab stop: one, the rows' (roving), as the menu's; the route values are selectable but not tab stops.
- Layer: Popup, in its own borderless, always-on-top, non-activating window [W docs/ui/components.md:571-584]; the in-app blur draws over the real desktop (platform work).
- Focus scope: losing focus closes the flyout [W docs/ui/components.md:571-584] [core.focus]; push on open and pop on close restore the previous focus owner [core.focus].
- Pointer capture while pressed: none in the source.

### Retained state

- Transient: row hover, open and close progress [motion].
- Persistent: none.
- App-owned: tray state (text, status dot), the stats values, the routes list, action rows, the open flag [W docs/ui/components.md:571-584].
- Exit retention: yes: Full 120 / Reduced 120 / Off 0 ms [core.ui] (the menu close, [overlay.menu]).

### Accessibility

- Role `menu` for the action rows (`menu-item`, actions default and dismiss) [overlay.menu] [infra.accessibility], named by `Flyout.name` (empty uses `luneui.menu.name`).
- The state text is a `status` node with a polite live region [infra.accessibility].
- Stats and routes: `group` nodes (named by their labels and keys) with the cells' and pairs' own nodes under them; no actions [infra.accessibility].
- Name of the flyout's rows: the app name and state, supplied by the app [infra.accessibility]. In a disabled scope every node is disabled and the rows' keys are taken, but a focused route value's are not (`docs/architecture.md`, "Disabled keys"); Escape still closes.

### Strings

None of its own: the header, stats, route and action labels are app text (Wane mock content, [W docs/ui/components.md:571-584]). The rows' name falls back to the menu's `luneui.menu.name`.

### Gallery scenes

- `flyout-open-serving` (new): header with state "Serving", stats and routes, settled.
- `flyout-open-degraded` (new): state "degraded", settled.
- `flyout-open-paused` (new): state "paused", settled.
- `flyout-closed` (new): nothing drawn over the controls scene, settled.
- `flyout-opening` (new): t = 90 ms (half of fast, 180 ms).
- `flyout-closing` (new): t = 60 ms (half of instant, 120 ms).
- `tray-flyout-window` (M10, `platform.tray-icon`): `Ui::flyout_window`, the same body filling a window of its own from its corner, over a flat stand-in for the desktop, settled.

### Painter primitives

- Existing: backdrop, fill, ring, text, icon (glyph in the header mark; the menu's primitives, [overlay.menu]).
- New: none. `popover::glass_panel` is the one function that draws the menu's glass, the glass dropdown, the glass popover and this card [D143].
- The saturate 1.8 of the glass is the frame's `FrameParams.saturation`: no painter parameter [D145].

### Acceptance

- Behaviour (scripted input, injected time): an outside click, Escape and a focus loss close; a click, Enter and Space pick a row once; arrows wrap and skip a disabled row; the size equals the drawn card at the three states; empty stats or routes take no height (`tests/widget_flyout.rs`). The tray click that toggles it is `TrayToggle`'s (M10, `platform.tray-icon`); the window form is checked in `crates/lune-ui-core/tests/tray.rs` and, live, by `gallery --smoke-platform` (its menu surface, through the same path).
- Pixel probes at scale 2: window width 680 px; radius 18 px; header padding 12 px vertical and 16 px horizontal.
- Conformance: the open and close curves match `overlay.menu`.
- Semantic node: role `menu` for the actions; the status node polite; the state text follows the tray state.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_flyout.rs`: behaviour `escape_an_outside_press_and_focus_loss_dismiss`, `a_click_picks_once_and_a_disabled_row_picks_nothing`, `focus_lands_on_the_first_row_and_enter_and_space_pick_once`, `arrows_wrap_and_skip_a_disabled_row`, `the_size_is_the_drawn_card_at_the_three_states`, `empty_stats_and_routes_take_no_height_and_neighbours_share_a_margin`; pixel probes `pixel_probes_at_scale_2`, `no_text_is_closer_than_9_px_to_the_card`; conformance `the_curves_are_the_menus_under_every_tier`, `a_closing_flyout_replays_its_blur_with_the_fade`; node `the_nodes_are_a_menu_a_status_and_two_groups`, `the_state_node_is_polite_and_follows_the_state_text`; the window form `a_window_flyout_card_fills_the_window_at_the_three_states`, `a_window_flyout_dismisses_on_escape_and_focus_loss_and_not_on_its_own_card`, `a_window_flyout_opens_and_closes_with_the_menus_curves_and_goes_idle`; disabled `a_disabled_flyout_closes_on_escape_picks_nothing_and_marks_every_node`.

### Build

- Milestone: M7 step 4 built the surface (header, stats strip, routes, action rows, open and close motion, drawn from the app's anchor point; `flyout_body` is the body M10 runs in its own window) and M10 (placement from the tray icon's rect and the separate glass window, with `platform.tray-icon`). M7 uses `data.stat` and `data.kv-list` (both M6) and `overlay.menu` (M7).
- Depends on: overlay.popover, overlay.menu, data.stat, data.kv-list, platform.tray-icon, platform.window, render.blur.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_flyout.rs::escape_an_outside_press_and_focus_loss_dismiss, crates/lune-ui-core/tests/widget_flyout.rs::focus_lands_on_the_first_row_and_enter_and_space_pick_once, crates/lune-ui-core/tests/widget_flyout.rs::arrows_wrap_and_skip_a_disabled_row, crates/lune-ui-core/tests/widget_flyout.rs::the_size_is_the_drawn_card_at_the_three_states, crates/lune-ui-core/tests/widget_flyout.rs::pixel_probes_at_scale_2, crates/lune-ui-core/tests/widget_flyout.rs::the_curves_are_the_menus_under_every_tier, crates/lune-ui-core/tests/widget_flyout.rs::the_nodes_are_a_menu_a_status_and_two_groups, crates/lune-ui-core/tests/widget_flyout.rs::a_window_flyout_card_fills_the_window_at_the_three_states`; page `flyout`; unverified here: the OS glass window behind the tray, a real tray click (`platform.tray-icon`).

## Open questions

- Still open, an API gap (D237): a route's state note ("loaded", Wane's `.c-flyout__note`, sans 11 tertiary after the id) has no place in `routes: &[(&str, &str)]`.
- Resolved [D145]: a toolkit component, so every consumer gets the surface and the keys.
- Answered (M10): native rendering is `Ui::flyout_window` in its own borderless window placed from the tray icon's rect (`platform.tray-icon`, `place_surface`); the in-app blur over the real desktop is the OS material (`platform.material`), which is not run here.
- Resolved [D145]: the menu's keys carry over, and focus-visible is the highlight.

## Notes for the planner

Wane priority P2 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `overlay.flyout`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, tray.

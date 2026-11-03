---
id: overlay.flyout
title: Tray flyout
area: overlay
status: planned
consumers: wane
source: —
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

None in luneui-rs or Luneterm's `lune-ui` (Wane: partial: the glass popup surface exists (popup_panel widgets/mod.rs:200; menu glass widgets/menu.rs:109); the stats block and the separate window are missing). Design source (not code to move): Luneterm crates/lune-ui/src/widgets/menu.rs:109-111 (backdrop, glass fill, ring); Luneium src/renderer/shell/components/Popup.css:2; docs/mockups/Tray.dc.html (content).

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
- Placement on Windows: 12 px above the taskbar, 12 px from the right edge [W docs/ui/components.md:571-584]; the window is placed from the tray icon's rect and kept on screen [W docs/ui/components.md:571-584].

### States

| state | look | cite |
|---|---|---|
| closed | not drawn | [W docs/ui/components.md:571-584] |
| open, serving | header state text and dot follow the tray state "Serving" | [W docs/ui/components.md:571-584] |
| open, degraded | header state text "degraded" | [W docs/ui/components.md:571-584] |
| open, paused | header state text "paused" | [W docs/ui/components.md:571-584] |
| item hover, pressed, disabled | the `overlay.menu` item states | [overlay.menu] |
| focus-visible | not in the source; open question | open question |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| open | as `overlay.menu` open: opacity, y offset 4→0, scale 0.97→1 | pop | fast | cubic-out | fade | [overlay.menu], [W docs/ui/components.md:571-584] |
| close | as `overlay.menu` close | dismiss | instant | ease-in | fade | [overlay.menu], [W docs/ui/components.md:571-584] |

Never animated: the stats and routes content (values change, [motion] list: high-frequency content).

### Input and focus

- Pointer: left click on the tray icon toggles the flyout (the icon is `platform.tray-icon`); an outside click closes [W docs/ui/components.md:571-584].
- Keyboard: none in the source; Esc and arrow keys are open questions.
- Tab stop: no (open question).
- Layer: Popup, in its own borderless, always-on-top, non-activating window [W docs/ui/components.md:571-584]; the in-app blur draws over the real desktop (platform work).
- Focus scope: losing focus closes the flyout [W docs/ui/components.md:571-584] [core.focus]; push on open and pop on close restore the previous focus owner [core.focus].
- Pointer capture while pressed: none in the source.

### Retained state

- Transient: row hover, open and close progress [motion].
- Persistent: none.
- App-owned: tray state (text, status dot), the stats values, the routes list, action rows, the open flag [W docs/ui/components.md:571-584].
- Exit retention: yes: Full 120 / Reduced 120 / Off 0 ms [core.ui] (the menu close, [overlay.menu]).

### Accessibility

- Role `menu` for the action rows (`menu-item`, actions default and dismiss) [overlay.menu] [infra.accessibility].
- The state text is a `status` node with a polite live region [infra.accessibility].
- Stats and routes: `group` nodes with `static-text` children; no actions [infra.accessibility].
- Name of the flyout: the app name and state, supplied by the app [infra.accessibility].

### Strings

None: the header, stats, route and action labels are app text (Wane mock content, [W docs/ui/components.md:571-584]).

### Gallery scenes

- `flyout-open-serving` (new): header with state "Serving", stats and routes, settled.
- `flyout-open-degraded` (new): state "degraded", settled.
- `flyout-open-paused` (new): state "paused", settled.
- `flyout-closed` (new): nothing drawn over the controls scene, settled.
- `flyout-opening` (new): t = 90 ms (half of fast, 180 ms).
- `flyout-closing` (new): t = 60 ms (half of instant, 120 ms).

### Painter primitives

- Existing: backdrop, fill, ring, text, icon (glyph in the header mark; the menu's primitives, [overlay.menu]).
- New: none.
- The saturate 1.8 of the glass has no painter parameter: an Open question (see Open questions).

### Acceptance

- Behaviour (scripted input, injected time): a tray click toggles; an outside click and a focus loss close; the action rows fire once and close. None exists.
- Pixel probes at scale 2: window width 680 px; radius 18 px; header padding 12 px vertical and 16 px horizontal.
- Conformance: the open and close curves match `overlay.menu`.
- Semantic node: role `menu` for the actions; the status node polite; the state text follows the tray state.

### Build

- Milestone: M7 (the surface: header, stats strip, routes, action rows, open and close motion, drawn from the app's anchor point) and M10 (placement from the tray icon's rect and the separate glass window, with `platform.tray-icon`). M7 uses `data.stat` and `data.kv-list` (both M6) and `overlay.menu` (M7).
- Depends on: overlay.popup-panel, overlay.menu, data.stat, data.kv-list, platform.tray-icon, platform.window, render.blur.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: needs a separate borderless, always-on-top, non-activating glass window anchored to the tray rect, with the in-app blur over the real desktop (platform work, section 5). The mock fixes its position; the real window must be placed from the tray icon's rect and kept on screen.
- Keyboard: Esc and arrow-key navigation are not in any source; role-gap for the action rows if the menu's keys do not carry over.
- Focus-visible on a row: not in any source.

## Notes for the planner

Wane priority P2 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `overlay.flyout`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, tray.

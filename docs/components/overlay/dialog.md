---
id: overlay.dialog
title: Modal dialog
area: overlay
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/dialog.rs
owner: toolkit-v1 M7
summary: Modal dialog card: dim backdrop, 448 px acrylic card, darker footer, app-supplied body
---

## What it is

`dialog` with `DialogSpec`, `DialogGlass` and `DialogHits`: dim backdrop, 448 px acrylic card, 24 px padding, darker footer; a `body` closure draws the app content [R crates/lune-ui-core/src/widgets/dialog.rs:1,39-48]. Variants: confirm (title, message, Cancel and one action) [W docs/ui/components.md:557-570]. `Ui::dialog_with` takes `DialogOptions { width, scrim_blur, confirm_enabled }`: the 600 px form width, a blurred scrim and a disabled confirm [D144]. The shell (scrim, the card's open and close motion, exit retention, the node) is `dialog::modal_host`, which the command palette shares.

## Needs

- Modal dialog (R-overlay-1).
- Wane — Variants: confirm (448 wide: title, message, Cancel + one action; Disconnect, Delete upstream, Re-price history); form (`c-dialog--form`, 600 wide, fields in two columns in the extra slot; Add / Edit upstream, Add rule, Test a request) (R-overlay-1)
- Wane — States: closed (hidden); open. Buttons use `widget.button`; a reversible action gets the primary, not a danger tone. (R-overlay-1)
- Wane — Sizes: Scrim fills the window rect (`--dialog-backdrop`, black at .22, lighter than lune-ui's .5) and the window content behind it blurs (`--dialog-backdrop-blur` 8 px; user, 2026-11-03); card 448 wide (`--dialog-w`), form 600 (`--dialog-form-w`), radius 8, 1 px `--popup-border-soft` (white at .08; user, 2026-11-04), fill `--glass-dialog` (the popup glass level, rgba 44 44 44 .616, as `overlay.menu`; user, 2026-11-03: the dialog itself is glass) with blur 40, body padding 23; title 14/20 Medium, message 13/20 secondary 10 px below, extra slot 12 px; footer 68 high (18 + 32 + 18) on `--dialog-footer`, buttons right-aligned 8 apart, confirm rightmost. (R-overlay-1)
- Wane — Behaviour: Opens centred in the window; Cancel, confirm, a click on the scrim and Esc close it. Focus is trapped inside while open and Enter confirms (requirements, not mocked). (R-overlay-1)

## Existing code

`crates/lune-ui-core/src/widgets/dialog.rs:42` (`dialog`), `:10` (`DialogSpec`), `:22` (`DialogGlass`), `:32` (`DialogHits`); 94 lines. No tests.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `crates/lune-ui-core/src/widgets/dialog.rs:1-94`;
`crates/lune-ui-core/src/theme.rs:55-84`; `crates/lune-ui-core/src/scene.rs:17-27`; Lunegit
`src/Lunegit.Desktop/Services/DialogService.cs:78,87-92`, `.claude/docs/dialog-exit-timing.md`,
`src/Lunegit.Desktop/Themes/Animations.axaml:414-470`, `src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:285-309`,
`.claude/docs/animation-rules.md:82-110`, `src/Lunegit.Infrastructure/Localization/en.json:8`; Wane `docs/ui/components.md:557-570`.
Conflicts (winner):
- Scrim: Wane black .22 [W docs/ui/components.md:557-570] vs `dialog_backdrop` #00000080 [R crates/lune-ui-core/src/theme.rs:69]. The moved code wins (pixel gate).
- Body padding: Wane 23 vs 24 [R crates/lune-ui-core/src/widgets/dialog.rs:63,77]. The moved code wins.
- Exit easing: `animation-rules.md:83,106` says QuadraticEaseIn; the shipped dialog transition is QuadraticEaseOut both ways [L src/Lunegit.Desktop/Themes/Animations.axaml:430-431]. Lunegit's shipped value is recorded in Transitions; the doc conflict is an Open question.
- Card fill: Wane `--glass-dialog` (rgba 44 44 44 .616) vs the toolkit taking the fill from the app [R crates/lune-ui-core/src/widgets/dialog.rs:22-28]. The app wins: the toolkit draws what `DialogGlass` passes.

### Anatomy

- Scrim: whole window rect; with `scrim_blur`, `backdrop_with(window, 0, 8.0, 1.0)` on the Modal layer under it (the window content blurs, faded with the scrim, in the exit painter too); `dialog_backdrop` #00000080 [R crates/lune-ui-core/src/widgets/dialog.rs:53, crates/lune-ui-core/src/theme.rs:69]; on the Modal layer and hit-blocked so clicks do not reach content below [R crates/lune-ui-core/src/widgets/dialog.rs:50-52].
- Card: width 448, or 600 with `DialogWidth::Form` (the body is then 552 wide; its columns are the app's) [R crates/lune-ui-core/src/widgets/dialog.rs, W docs/ui/components.md:557-570]; horizontally centred [R crates/lune-ui-core/src/widgets/dialog.rs:66]; top at `(window h − body − footer) / 2`, at least 20 [R crates/lune-ui-core/src/widgets/dialog.rs:67]; radius 8 [R crates/lune-ui-core/src/widgets/dialog.rs:73-74]; acrylic fill from `DialogGlass.fill` over the panel layer [R crates/lune-ui-core/src/widgets/dialog.rs:72-73]; 1 px ring `border_subtle` #FFFFFF1A [R crates/lune-ui-core/src/widgets/dialog.rs:74, crates/lune-ui-core/src/theme.rs:63].
- Body: 24 px inset on all sides [R crates/lune-ui-core/src/widgets/dialog.rs:63,77-78,82]; content width 400 (448 − 48) [R crates/lune-ui-core/src/widgets/dialog.rs:56]; title 14 px Medium, `fg_primary` [R crates/lune-ui-core/src/widgets/dialog.rs:57, crates/lune-ui-core/src/theme.rs:84]; message wrapped to 400, 13 px, `fg_secondary` [R crates/lune-ui-core/src/widgets/dialog.rs:58,60, crates/lune-ui-core/src/theme.rs:83]; title-to-message gap 10 [R crates/lune-ui-core/src/widgets/dialog.rs:79]; app body slot of height `extra_h` directly below the message [R crates/lune-ui-core/src/widgets/dialog.rs:63,81-82].
- Footer: 68 high (18 + 32 + 18), full card width, fill `dialog_footer` #00000024, bottom radii 8 [R crates/lune-ui-core/src/widgets/dialog.rs:64,84-85, crates/lune-ui-core/src/theme.rs:70].
- Buttons: 32 high; confirm right edge 24 in from the card right, 18 down from the footer top [R crates/lune-ui-core/src/widgets/dialog.rs:87]; cancel 8 px left of confirm [R crates/lune-ui-core/src/widgets/dialog.rs:91]; confirm is the primary button, cancel is not [R crates/lune-ui-core/src/widgets/dialog.rs:88,92]. Button width from `button_width` [R crates/lune-ui-core/src/widgets/dialog.rs:86,90].

### States

| state | look | cite |
|---|---|---|
| closed | nothing drawn | [W docs/ui/components.md:557-570] |
| open, confirm only | card, one primary button (`cancel: None`) | [R crates/lune-ui-core/src/widgets/dialog.rs:89-93] |
| open, confirm and cancel | card, cancel (plain) left of confirm (primary) | [R crates/lune-ui-core/src/widgets/dialog.rs:88-92] |
| focus-visible on a button | the button's own state (widget.button) | [R crates/lune-ui-core/src/widgets/dialog.rs:88,92] |
| disabled confirm (`confirm_enabled: false`) | the button's own disabled look (opacity 0.4, `with_enabled(false)`), same place; Enter, Space, a click and the default action do nothing and are consumed; the dialog node offers no Default; open focus on Cancel, or none | [D144] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| open | card opacity 0→1 and scale 0.96→1; scrim opacity 0→1 | pop | base | quad-out | fade (opacity only, Fast) | [L .claude/docs/animation-rules.md:109], [L src/Lunegit.Desktop/Themes/Animations.axaml:441-470], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:285-309] |
| close | card and scrim opacity 1→0 (scale 1→0.96 on the card); host hidden at the exit delay | dismiss | fast | quad-out | fade | [L src/Lunegit.Desktop/Services/DialogService.cs:90-91], [L src/Lunegit.Desktop/Themes/Animations.axaml:430-440] |

Teardown runs at the exit delay (180 ms Full), before the base-length fade ends, so the card is
near-invisible when it disappears [L .claude/docs/dialog-exit-timing.md]. Never animated: the card's
size and the content inside it (laid out once per open, [R crates/lune-ui-core/src/widgets/dialog.rs:63-70]).

### Input and focus

- Pointer: click on Cancel or confirm [R crates/lune-ui-core/src/widgets/dialog.rs:88,92]; a click on the scrim closes it [W docs/ui/components.md:557-570] (the code only hit-blocks the window, dialog.rs:52). No pointer capture in the moved code.
- Keyboard: Esc closes; Enter confirms [W docs/ui/components.md:557-570] (requirements, not mocked). Tab stop order: Cancel then confirm [derived: order of drawing, dialog.rs:88,92] [core.focus].
- Layer: Modal [R crates/lune-ui-core/src/widgets/dialog.rs:50, crates/lune-ui-core/src/scene.rs:17-22].
- Focus scope: push on open; a modal traps Tab; pop restores focus to the previous widget or anchor [core.focus]. Escape and scrim click act on the top overlay only [core.focus].

### Retained state

- Transient: hover and press on buttons; open and close fade progress [motion].
- Persistent: none.
- App-owned: `DialogSpec` (title, message, extra_h, confirm, cancel) [R crates/lune-ui-core/src/widgets/dialog.rs:10-18], the open flag, and the body's own state (the app passes each frame, dialog.rs:44).
- Exit retention: yes: Full 180 / Reduced 120 / Off 0 ms [L src/Lunegit.Desktop/Services/DialogService.cs:87-92].

### Accessibility

- Role `dialog`, modal state; name = `DialogSpec.title` [R crates/lune-ui-core/src/widgets/dialog.rs:11]; description = `DialogSpec.message` [R crates/lune-ui-core/src/widgets/dialog.rs:12] [infra.accessibility].
- Actions: default (confirm), dismiss (Esc, Cancel) [infra.accessibility].
- Cancel and confirm are `button` nodes (widget.button) [infra.accessibility].
- Live region: none (a dialog is not announced by a live region).

### Strings

- `luneui.dialog.cancel`: "Cancel" (Lunegit `common.cancel`) [L src/Lunegit.Infrastructure/Localization/en.json:8].
- Confirm label: None: the app passes it (`DialogSpec.confirm`, [R crates/lune-ui-core/src/widgets/dialog.rs:15]).
- Title and message: None: the app passes them ([R crates/lune-ui-core/src/widgets/dialog.rs:11-12]).

### Gallery scenes

- `dialog-confirm` (existing): title "Discard changes", confirm and Cancel, settled. [R apps/gallery/src/scenes.rs:470-481]
- `dialog-message` (existing): confirm only, settled. [R apps/gallery/src/scenes.rs:483-494]
- `dialog-body` (existing): extra body 100 px with a field and a checkbox, settled. [R apps/gallery/src/scenes.rs:496-527]
- `dialog-closed`: no dialog drawn over the controls scene, settled (new).
- `dialog-opening`: confirm and Cancel, t = 125 ms (half of base) (new).
- `dialog-closing`: confirm and Cancel, t = 90 ms (half of fast) (new).
- `dialog-form`: 600 px card, two columns of fields in the body, settled.
- `dialog-scrim-blur`: the confirm dialog over a blurred window, settled.
- `dialog-confirm-disabled`: the confirm button disabled, settled.
- `dialog-combo-open`, `dialog-popover`, `dialog-tooltip`, `dialog-menu-glass`, `dialog-nested`: an overlay opened by the body, settled; each draws on `Raised` above the dialog's own text (the gallery tests probe it through the tree and the layer) [D137].

### Painter primitives

- Existing: fill, fill-radii, ring, acrylic, text. [R crates/lune-ui-core/src/paint.rs:215,219,231,243,274]
- New: none (`backdrop_with` lands in M7 step 1).

### Acceptance

- Behaviour (scripted input, injected time): Cancel, confirm, Esc, Enter and scrim click each fire once; Tab does not leave the dialog. None exists yet (dialog.rs has no tests).
- Pixel probes at scale 2: card 896 px wide, footer 136 px high, buttons 64 px high, card corner radius 16 px, scrim colour #00000080. Source values in Anatomy.
- Conformance: open reaches opacity 1 and scale 1 at 250 ms; close hides the host at 180 ms (Full), 120 ms (Reduced), 0 ms (Off).
- Semantic node: role `dialog`, modal, name = title; default action fires confirm.
- Variants: the form card is 600 wide and centred (1200 at scale 2); the blur request is on Modal at sigma 8, saturation 1, faded at half open and half close; a glass menu above a blurred scrim loses the arbitration (the scrim's blur stays) and draws its fill; a disabled confirm is `Disabled`, not focusable, offers no action, draws at 0.4 open and closing, and Enter and Space on it reach no Global binding; a confirm, cancel, Escape or scrim click inside a nested dialog reaches only the top (`widget_dialog`, gallery `dialog` tests).

### Build

- Milestone: M4 (retrofit), M7 (variants, overlays above the dialog).
- Depends on: core.ui, core.focus, infra.accessibility, motion.transitions, widget.button.

## Open questions

- Resolved [D75]: initial focus is Confirm, Cancel's tab stop comes first, Enter activates the focused button.
- Resolved [D144]: the toolkit owns the form width (`DialogWidth::Form`, 600); the columns stay the app's body.
- Resolved [D144]: the scrim blur is `DialogOptions::scrim_blur` (sigma 8, saturation 1), off by default.
- Resolved [D144]: exit easing stays quad-out, the shipped Lunegit value (the doc's quad-in is a doc conflict, not a source of the look).
- Resolved [D144]: a disabled confirm uses the button's own disabled look; the dialog's open focus goes to Cancel, or none.

## Limits

- Replaces the carried limit that an overlay opened inside a dialog drew and took input under it: an overlay opened above an active modal draws and takes input on `Raised`, one level [D137]. Overlays that stack further share `Raised`, so a third level that overlaps its parent interleaves (fills under text).
- One blur per frame [D138]: a glass menu over a dialog with a blurred scrim has no blur of its own (the scrim's keeps), and draws its fill.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: partial: widgets/dialog.rs:42 has the confirm dialog; missing: the blurred scrim and the form variant; Wane priority P0. Native note: The card's blur is CSS here; luneui-rs uses the acrylic card layer. The scrim must block input to the whole window (the Modal layer does) and blur what is behind it, which needs a backdrop pass over the window content in luneui-rs. Full spec: Wane `docs/ui/components.md`, section `overlay.dialog`; mock: Wane `docs/ui/mocks/components.html`.

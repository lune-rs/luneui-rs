---
id: overlay.dialog
title: Modal dialog
area: overlay
status: in-progress
consumers: luneterm, wane
source: crates/lune-ui-core/src/widgets/dialog.rs
owner: orch/toolkit-v1
summary: Modal dialog card: dim backdrop, 448 px acrylic card, darker footer, app-supplied body
---

## What it is

`dialog` with `DialogSpec`, `DialogGlass` and `DialogHits`: dim backdrop, 448 px acrylic card, 24 px padding, darker footer; a `body` closure draws the app content.

## Needs

- Modal dialog (R-overlay-1).
- Wane — Variants: confirm (448 wide: title, message, Cancel + one action; Disconnect, Delete upstream, Re-price history); form (`c-dialog--form`, 600 wide, fields in two columns in the extra slot; Add / Edit upstream, Add rule, Test a request) (R-overlay-1)
- Wane — States: closed (hidden); open. Buttons use `widget.button`; a reversible action gets the primary, not a danger tone. (R-overlay-1)
- Wane — Sizes: Scrim fills the window rect (`--dialog-backdrop`, black at .22, lighter than lune-ui's .5) and the window content behind it blurs (`--dialog-backdrop-blur` 8 px; user, 2026-11-03); card 448 wide (`--dialog-w`), form 600 (`--dialog-form-w`), radius 8, 1 px `--popup-border-soft` (white at .08; user, 2026-11-04), fill `--glass-dialog` (the popup glass level, rgba 44 44 44 .616, as `overlay.menu`; user, 2026-11-03: the dialog itself is glass) with blur 40, body padding 23; title 14/20 Medium, message 13/20 secondary 10 px below, extra slot 12 px; footer 68 high (18 + 32 + 18) on `--dialog-footer`, buttons right-aligned 8 apart, confirm rightmost. (R-overlay-1)
- Wane — Behaviour: Opens centred in the window; Cancel, confirm, a click on the scrim and Esc close it. Focus is trapped inside while open and Enter confirms (requirements, not mocked). (R-overlay-1)

## Existing code

`crates/lune-ui-core/src/widgets/dialog.rs:42` (`dialog`), `:10` (`DialogSpec`), `:22` (`DialogGlass`), `:32` (`DialogHits`); 94 lines. No tests.

## Open questions

- Whether the card width is configurable is unverified; Lunegit has many dialogs under `Views/Dialogs/` and whether one width suits them is unchecked.

## Notes for the planner

Moves in the behaviour-preserving extraction (R-ext-1); the pixel gate covers it only where a glass-ui scene draws it.

Wane (2026-11-04): lune-ui status for Wane's use: partial: widgets/dialog.rs:42 has the confirm dialog; missing: the blurred scrim and the form variant; Wane priority P0. Native note: The card's blur is CSS here; luneui-rs uses the acrylic card layer. The scrim must block input to the whole window (the Modal layer does) and blur what is behind it, which needs a backdrop pass over the window content in luneui-rs. Full spec: Wane `docs/ui/components.md`, section `overlay.dialog`; mock: Wane `docs/ui/mocks/components.html`.

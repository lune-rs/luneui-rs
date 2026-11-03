---
id: luneterm.ime-composition
title: IME composition input
area: luneterm
status: planned
consumers: luneterm
source: —
owner: —
summary: IME composition input for the terminal window (Luneterm M9 lists IME)
---

## What it is

Luneterm M9 lists IME among its scope items. `lune-ui` has no IME module per the inventory. Whether composition is handled in the toolkit or by the app on winit events is undecided.

## Needs

- IME composition input (R-text-4).
- Text field behaviour that composition must integrate with (R-text-5).

## Existing code

none in `lune-ui`. How Luneterm will handle composition is not decided in the roadmap.

## Open questions

- Toolkit or app responsibility?
- Which platforms need it first (macOS is Luneterm's first target).

## Notes for the planner

Marked open requirement R-text-4. Roadmap scope: M9 row.

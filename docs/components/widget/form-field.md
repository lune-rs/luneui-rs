---
id: widget.form-field
title: Form field (label, control, hint)
area: widget
status: in-progress
consumers: wane
source: —
owner: toolkit-v1 M6
summary: Form field: label above a control, hint below; error state rings the control and colours the hint in err
---

## What it is

`form-field`: layout wrapper for a label, a control and a hint. Stacked by default, or inline with the label at the left; the control keeps its own size [W docs/ui/components.md:448,450].

## Needs

- Wane controls the toolkit lacks (R-widget-4).

## Existing code

`crates/lune-ui-core/src/widgets/form_field.rs` (M6): `FormField`, `Ui::form_field`. Gallery page `apps/gallery/src/pages/form_field.rs`; tests `crates/lune-ui-core/tests/widget_form_field.rs`. Wane listed it as missing in lune-ui [W docs/ui/components.md:454]. No Lunegit form-field line was read; Wane is the only source for this entry.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:445-457`; `docs/ui/mocks/css/tokens.css:13-14,55,124-125`.
No conflict: one source.

### Anatomy

- Column gap 4 [W docs/ui/components.md:450].
- Label 13/20 `--text-secondary` [W docs/ui/components.md:450], [W docs/ui/mocks/css/tokens.css:13,125].
- Hint 11/16 `--text-tertiary` [W docs/ui/components.md:450], [W docs/ui/mocks/css/tokens.css:14,124].
- Inline variant: label at the left of the control [W docs/ui/components.md:448].
- Control: its own size and its own states [W docs/ui/components.md:450]; the field is given `control_size` (a width of 0 fills) because the closure draws the control and the field cannot measure it. Inline: the label sits 8 px before the control, centred on it, and the note starts at the control's left.

### States

| state | look | cite |
|---|---|---|
| normal | label secondary, hint tertiary, control as its own state | [W docs/ui/components.md:449-450] |
| error | 1 px `--err` ring on the control; hint in `--err`; no glow | [W docs/ui/components.md:449], [W docs/ui/mocks/css/tokens.css:55] |

### Transitions

n/a — the field has no motion of its own; no glow [W docs/ui/components.md:449]. The control's rows apply (`widget.text-field`, `widget.combo`).

### Input and focus

- Pointer: click on the label focuses the control [W docs/ui/components.md:451]: the label is a `Sense::CLICK` region (no tab stop) and a press calls `request_focus` on the id the control closure returned [D127].
- Keyboard: none of its own; the control takes Tab [widget.text-field].
- Layer: `Base`; no overlay, no focus scope.

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: label, hint, error flag and error text (validation timing is the owner's [W docs/ui/components.md:451]).
- Exit retention: no.

### Accessibility

- Role `group`; name = the label; description = the hint; state `invalid` when the error is set, with the error text as a `static-text` child; the control inside keeps its own node, as the group's child [infra.accessibility], [W docs/ui/components.md:451]. A text field inside takes no error flag from the form field: the field draws the one ring, over the control's own border.

### Strings

None: label and hint text are app data.

### Gallery scenes

- `form-field-states`: normal stacked, error stacked, inline with a label at the left; settled.

### Painter primitives

- Existing: ring, text
- New: none

### Acceptance

- Pixel probes at scale 2: column gap 8 px; error ring 2 px in `--err` at the control rect.
- Conformance: n/a — no transition of its own.
- Semantic node: role group; name equals the label; invalid follows the error flag.

### Build

- Milestone: M6
- Depends on: widget.text-field, core.painter, infra.accessibility

## Open questions

- None.

## Notes for the planner

Wane P0. Structure only; the control inside carries the look.

---
id: overlay.toast-notifications
title: Toasts and notification history
area: overlay
status: in-progress
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/MainWindow.axaml
owner: toolkit-v1 M7
summary: Bottom-right toast stack (max 3, timed dismissal, loading and prompt toasts) and a 50-item notification history
---

## What it is

Toast stack and notification history, as Lunegit ships them. Toasts sit bottom-right over the content, at most three visible; info, success and warning toasts auto-dismiss, error, loading and prompt toasts do not; a click expands a toast to its log; the history keeps the last 50 items [L src/Lunegit.Desktop/MainWindow.axaml:272-296], [L src/Lunegit.Desktop/Services/NotificationService.cs:16-17,147-158], [L src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml:14-30].

## Needs

- Toasts and history (R-overlay-2).
- Timed dismissal needs app-supplied time (R-nfr-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/MainWindow.axaml`
- `src/Lunegit.Desktop/Services/NotificationService.cs`
- `src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml`

`crates/lune-ui-core/src/widgets/toast.rs` (`Toasts`, `ToastSpec`, `ToastKind`, `ToastAction`, `ToastId`, `Ui::toasts`, `Ui::toast_history`, `toast_size`) [D146]; tests in `crates/lune-ui-core/tests/widget_toast.rs`. Moved code: none (before M7 this repo had no toast or notification code).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/MainWindow.axaml:232,272-330,387-399`;
`src/Lunegit.Desktop/Services/NotificationService.cs:14-17,25-43,71,92-170,117-134,140-146,231-234`;
`src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml:14-30`;
`src/Lunegit.Desktop/Themes/DarkTheme.axaml:333-341`; `src/Lunegit.Infrastructure/Localization/en.json:8,390-392,402-405`;
`src/Lunegit.Desktop/Themes/Animations.axaml` and `AnimationsReduced.axaml` (no toast selector, `grep -i toast` empty).
Conflicts: none between sources (Wane has no toast entry). The layer order: Lunegit draws toasts above dialogs
[L src/Lunegit.Desktop/MainWindow.axaml:232,272]; the toolkit's top layer today is Modal [R crates/lune-ui-core/src/scene.rs:17-22]; a `Toast` layer above `Modal` is added in M4 ([D54], [core.ui]).

### Anatomy

- Stack: bottom-right, margin 16 on the right and bottom; items spaced 8 [L src/Lunegit.Desktop/MainWindow.axaml:273-281].
- Toast: min width 300; width follows expansion; padding 12; radius 8; fill `ToastBackground` #E6181818; border `ToastBorder` #33FFFFFF [L src/Lunegit.Desktop/MainWindow.axaml:285-291], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:333-334].
- Accent by type: info #61AFEF, success #4CAF50, warning #E2B93D, error #E06C75, loading #61AFEF [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:335-339].
- Message: wrapped text, base size, `ForegroundPrimary` [L src/Lunegit.Desktop/MainWindow.axaml:318-323].
- Cancel control: shown only on cancellable loading toasts [L src/Lunegit.Desktop/MainWindow.axaml:325-330].
- Action buttons on prompts: classes `toast-action`, `toast-primary` or `toast-secondary` [L src/Lunegit.Desktop/MainWindow.axaml:387-389].
- Log block (expanded): fill `ToastTerminalBackground` #FF0D0D0D, text `ToastTerminalForeground` #CCCCCC [L src/Lunegit.Desktop/MainWindow.axaml:399,441], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:340-341].
- History panel: header row with a clear-all button (14 px icon, 24 px button) and the header text; a scrolling list of items [L src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml:14-30].
- Built geometry [D146]: width 320 collapsed and 720 expanded (the shipped converter; the 300 minimum is under both), a 1 px border inside the radius 8 and 12 px of padding, so the content starts 13 px in. Row 0 is the 16 px kind glyph (a turning `Ui::spinner` in `toast_info` while loading), 8 px, the wrapped message vertically centred, then Cancel (8 px before it, only a cancellable loading toast) and Dismiss (4 px before it): 20 px buttons holding a 12 px glyph in `fg_secondary`, radius 4. Under it: the detail at 24 px in (11 px, `fg_secondary`, 4 px above), the bar of a loading toast (8 px above; `Ui::progress`, 4 px determinate and 3 px indeterminate: the progress component's own heights), the log block when expanded (8 px above, 200 high, radius 4, 8 px padding, mono 11 on 14 px lines, `toast_log_bg` and `toast_log_fg`, scrolled by `Ui::scroll_area` and following the tail; only the lines in view are shaped; it shows even with no lines, as the shipped markup does), and the actions (8 px above, right-aligned, 8 apart, secondary first and the primary last; padding 16 by 8). An action is `surface_input` for the primary one and a ghost for the rest, `surface_hover` on hover and `surface_active` pressed. The stack is 16 from the right and bottom edges, 8 between toasts, newest at the bottom.
- The history rows are the shipped panel's: 8 by 6 padding and 1 px margin, a 14 px kind glyph, 8 px, the wrapped message, the detail at 22 px in; the header is a 24 px row 5 px below the top with the 24 px clear-all button at its right, and the list starts 4 px under it.

### States

| state | look | cite |
|---|---|---|
| info | accent #61AFEF | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:335] |
| success | accent #4CAF50 | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:336] |
| warning | accent #E2B93D | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:337] |
| error | accent #E06C75; no auto-dismiss | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:338], [L src/Lunegit.Desktop/Services/NotificationService.cs:157] |
| loading | accent #61AFEF; progress determinate or indeterminate; cancel shown when cancellable | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:339], [L src/Lunegit.Desktop/Services/NotificationService.cs:231] |
| prompt | action buttons; never auto-dismissed | [L src/Lunegit.Desktop/MainWindow.axaml:387-389], [L src/Lunegit.Desktop/Services/NotificationService.cs:149-150] |
| collapsed | width from the converter, message only | [L src/Lunegit.Desktop/MainWindow.axaml:291] |
| expanded | width from the converter, log block shown; not auto-dismissed while open | [L src/Lunegit.Desktop/MainWindow.axaml:291,399], [L src/Lunegit.Desktop/Services/NotificationService.cs:166-167] |
| focus-visible on an action, Cancel or Dismiss | the button's own ring (the toolkit's `Ui::button` and `Ui::icon_button` look) | [D146] |
| leaving | fading out in place, no input, no node | [D146] |

### Transitions

Lunegit has no toast motion (no toast selector in Animations.axaml or AnimationsReduced.axaml) and removes at once [L src/Lunegit.Desktop/Services/NotificationService.cs:128-134]; the toolkit gives the toast D40's kinds at their defaults [D146], rows in `docs/motion.md`:

| change | kind | property | duration | easing | Reduced |
|---|---|---|---|---|---|
| appear | appear | opacity 0 to 1 and y 8 to 0 | base | quad-out | opacity only, at the mapped duration |
| leave | dismiss | opacity 1 to 0 | fast | quad-in | opacity only |
| neighbours | move | y of the toasts whose slot changed (an arrival or a removal) | fast | sine-in-out | snap |
| expand | expand | the card's height to show or hide the log; the text is wrapped once at the target width | base | sine-in-out | snap |

Off snaps all four. A toast's y is one transition retargeted from where it is: when its slot changes, the offset takes the difference and settles, so nothing jumps. A leaving toast keeps its slot until its fade ends. The archive happens when the toast starts to leave, once.

### Input and focus

- Pointer: a click on the toast toggles expansion [L src/Lunegit.Desktop/MainWindow.axaml:294]; action buttons on prompts [L src/Lunegit.Desktop/MainWindow.axaml:387-389]; cancel on cancellable loading toasts [L src/Lunegit.Desktop/MainWindow.axaml:325-330]; clear-all in the history [L src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml:14-21].
- Keyboard: only the buttons take focus: Tab reaches them (of the scope the app called `Ui::toasts` in), Enter and Space press the focused one. The body is not a tab stop; an access Default action toggles it and `Dismiss` dismisses it [D146].
- Tab stop: action and cancel buttons only, not the toast body [core.focus] (derived: the body is a click target, MainWindow.axaml:294).
- Layer: `Toast`, above `Modal` [L src/Lunegit.Desktop/MainWindow.axaml:232,272], [D54], [core.ui].
- Focus scope: none; toasts push no scope, so their buttons are tab stops of the scope below and a modal's Tab trap does not reach them [core.focus]; toasts do not take focus on appear [derived: no focus code found in the toast markup, MainWindow.axaml:272-399]. A press on a toast is never an outside press: a popover open under it stays for the press, which reaches the toast's button (the focus move then closes a popover that closes on focus loss) [D146].
- Dismiss: timed (see Acceptance); an expanded toast is not dismissed; prompts and errors are dismissed only by a button or the clear-all [L src/Lunegit.Desktop/Services/NotificationService.cs:147-170].
- Pointer capture while pressed: none.

### Retained state

- Transient: expansion toggle is retained per toast while it is visible [L src/Lunegit.Desktop/Services/NotificationService.cs:166-167].
- Persistent: the visible list (at most 3) and the history (at most 50), owned by the notification service [L src/Lunegit.Desktop/Services/NotificationService.cs:16-17,28]; here the app-owned `Toasts` model. The full history drops its oldest item (the shipped service removes index 0, the newest, which reads as a slip).
- App-owned: the dismiss timers. The toolkit does not read the clock; the app passes time (repo `CLAUDE.md`, Hard rules 2) [motion].
- Exit retention: no (immediate removal, NotificationService.cs:128-134).

### Accessibility

- Each toast: role `status`, live region polite (plan contract for toasts, [infra.accessibility]); an error toast is `alert`, assertive; name = the message; description = the detail. Actions: Default toggles the log, Expand or Collapse by state, Dismiss dismisses.
- Action and cancel buttons: `button` nodes named by their label [infra.accessibility].
- Loading toast: a `progress` node named by the message; a null progress value is indeterminate (no value) [L src/Lunegit.Desktop/Services/NotificationService.cs:231]. The turning glyph is a busy `progress` node with no name.
- History: `list` with `list-item` rows; clear-all is a `button` named "Clear all" [L src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml:14-21].
- Error toasts: `alert`, assertive [D146].

### Strings

- `luneui.toast-notifications.clear-all`: "Clear all" [L src/Lunegit.Infrastructure/Localization/en.json:392].
- `luneui.toast-notifications.history-header`: "NOTIFICATIONS" [L src/Lunegit.Infrastructure/Localization/en.json:390].
- `luneui.toast-notifications.dismiss`: "Dismiss", the name of the Dismiss button. Cancel reuses `luneui.dialog.cancel`.
- Toast titles, messages, details and action labels: None: the app passes them (`NotificationAction`, [L src/Lunegit.Desktop/Services/NotificationService.cs:92]).

### Gallery scenes

- `toast-notifications-stack` (new): an info toast with a detail, a warning and an error toast stacked, settled (the stack holds three, so a fourth kind would evict the first; timed toasts keep a frame due, so the capture reports `Repaint::At`).
- `toast-notifications-loading` (new): one loading toast at 40% with a cancel control; the glyph turns, so the capture is moving (`Off` gives the settled look).
- `toast-notifications-prompt` (new): one prompt toast with two actions and a detail, settled.
- `toast-notifications-expanded` (new): one error toast expanded to its log block, settled.
- `toast-notifications-history` (new): the history panel with three items and the clear-all control, settled.
- `toast-notifications-appearing` (new): a success toast half way through its arrival, the toast below it moving up.
- `toast-notifications-leaving` (new): a warning toast half way through its fade.

### Painter primitives

- Existing: fill, ring, rounded clip, text, icon, transforms (the arrival's rise and the fades), `Ui::progress` (determinate and indeterminate) and `Ui::spinner`.
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): info dismisses at 4 s, success at 3 s, warning at 6 s; error, loading and prompt never auto-dismiss; an expanded toast is not dismissed until collapsed; a fourth visible toast evicts the oldest non-loading, non-prompt toast; history is capped at 50. Source: [L src/Lunegit.Desktop/Services/NotificationService.cs:16-17,119-125,140-146,154-157,166-167]. None exists in the moved code.
- Pixel probes at scale 2: toast width 640 px (above the 600 px minimum), radius 16 px, padding 24 px inside the 2 px border, stack gap 16 px, bottom-right margin 32 px.
- Conformance: the four motions above in the three tiers (`tests/widget_toast.rs`).
- Semantic node: each toast `status` with polite live region (`alert` for an error); history `list`; actions named. Over a dialog and over a raised popover a toast's buttons work, and neither Tab nor Escape behaves differently for the dialog with a prompt toast up. In a disabled scope every node is disabled and no key reaches a binding.

### Build

- Milestone: M7 (step 5 built the model, `Ui::toasts` and `Ui::toast_history`).
- Depends on: core.ui, core.focus, infra.accessibility, widget.button, data.progress.

## Open questions

- Resolved [D146]: motion (D40's kinds at their defaults), keyboard (only the buttons), the error role (`alert`), the stacking order (newest at the bottom, as the service appends) and the indeterminate bar (`Ui::progress`).
- The log block's horizontal extent is the widest line in view, not the widest of a long log, to keep a frame's cost bounded.

## Notes for the planner

Lunegit's toast code is the only source; the step read it (2026-11-04) by line, not from a file-name survey.

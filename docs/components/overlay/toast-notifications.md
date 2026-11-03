---
id: overlay.toast-notifications
title: Toasts and notification history
area: overlay
status: planned
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/MainWindow.axaml
owner: —
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

Moved code: none. This repo has no toast or notification code (`grep -rn -i 'toast\|notification' crates apps`).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/MainWindow.axaml:232,272-330,387-399`;
`src/Lunegit.Desktop/Services/NotificationService.cs:14-17,25-43,71,92-170,117-134,140-146,231-234`;
`src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml:14-30`;
`src/Lunegit.Desktop/Themes/DarkTheme.axaml:333-341`; `src/Lunegit.Infrastructure/Localization/en.json:8,390-392,402-405`;
`src/Lunegit.Desktop/Themes/Animations.axaml` and `AnimationsReduced.axaml` (no toast selector, `grep -i toast` empty).
Conflicts: none between sources (Wane has no toast entry). The layer order: Lunegit draws toasts above dialogs
[L src/Lunegit.Desktop/MainWindow.axaml:232,272]; the toolkit's top layer is Modal [R crates/lune-ui-core/src/scene.rs:17-22]. Open question.

### Anatomy

- Stack: bottom-right, margin 16 on the right and bottom; items spaced 8 [L src/Lunegit.Desktop/MainWindow.axaml:273-281].
- Toast: min width 300; width follows expansion; padding 12; radius 8; fill `ToastBackground` #E6181818; border `ToastBorder` #33FFFFFF [L src/Lunegit.Desktop/MainWindow.axaml:285-291], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:333-334].
- Accent by type: info #61AFEF, success #4CAF50, warning #E2B93D, error #E06C75, loading #61AFEF [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:335-339].
- Message: wrapped text, base size, `ForegroundPrimary` [L src/Lunegit.Desktop/MainWindow.axaml:318-323].
- Cancel control: shown only on cancellable loading toasts [L src/Lunegit.Desktop/MainWindow.axaml:325-330].
- Action buttons on prompts: classes `toast-action`, `toast-primary` or `toast-secondary` [L src/Lunegit.Desktop/MainWindow.axaml:387-389].
- Log block (expanded): fill `ToastTerminalBackground` #FF0D0D0D, text `ToastTerminalForeground` #CCCCCC [L src/Lunegit.Desktop/MainWindow.axaml:399,441], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:340-341].
- History panel: header row with a clear-all button (14 px icon, 24 px button) and the header text; a scrolling list of items [L src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml:14-30].

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
| focus-visible on an action or cancel button | Lunegit button default; not specified for toasts | open question |

### Transitions

n/a — Lunegit has no toast motion (no toast selector in Animations.axaml or AnimationsReduced.axaml), and the moved code has none. Whether toasts appear or dismiss with motion is an Open question. Removal is immediate: `RemoveNotification` removes the item at once [L src/Lunegit.Desktop/Services/NotificationService.cs:128-134].

### Input and focus

- Pointer: a click on the toast toggles expansion [L src/Lunegit.Desktop/MainWindow.axaml:294]; action buttons on prompts [L src/Lunegit.Desktop/MainWindow.axaml:387-389]; cancel on cancellable loading toasts [L src/Lunegit.Desktop/MainWindow.axaml:325-330]; clear-all in the history [L src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml:14-21].
- Keyboard: none in the Lunegit sources read; open question.
- Tab stop: action and cancel buttons only, not the toast body [core.focus] (derived: the body is a click target, MainWindow.axaml:294).
- Layer: Popup [derived: drawn over content, MainWindow.axaml:272]. Lunegit draws toasts above dialogs [L src/Lunegit.Desktop/MainWindow.axaml:232,272]; open question.
- Focus scope: none; toasts do not take focus [derived: no focus code found in the toast markup, MainWindow.axaml:272-399].
- Dismiss: timed (see Acceptance); an expanded toast is not dismissed; prompts and errors are dismissed only by a button or the clear-all [L src/Lunegit.Desktop/Services/NotificationService.cs:147-170].
- Pointer capture while pressed: none.

### Retained state

- Transient: expansion toggle is retained per toast while it is visible [L src/Lunegit.Desktop/Services/NotificationService.cs:166-167].
- Persistent: the visible list (at most 3) and the history (at most 50), owned by the notification service [L src/Lunegit.Desktop/Services/NotificationService.cs:16-17,28].
- App-owned: the dismiss timers. The toolkit does not read the clock; the app passes time (repo `CLAUDE.md`, Hard rules 2) [motion].
- Exit retention: no (immediate removal, NotificationService.cs:128-134).

### Accessibility

- Each toast: role `status`, live region polite (plan contract for toasts, [infra.accessibility]); name = the title or message; description = the detail.
- Action and cancel buttons: `button` nodes named by their label [infra.accessibility].
- Loading toast: a `progress` node; a null progress value is indeterminate [L src/Lunegit.Desktop/Services/NotificationService.cs:231].
- History: `list` with `list-item` rows; clear-all is a `button` named "Clear all" [L src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml:14-21].
- Error toasts: `alert` or `status` is open (see Open questions).

### Strings

- `luneui.toast-notifications.clear-all`: "Clear all" [L src/Lunegit.Infrastructure/Localization/en.json:392].
- `luneui.toast-notifications.history-header`: "NOTIFICATIONS" [L src/Lunegit.Infrastructure/Localization/en.json:390].
- Toast titles, messages, details and action labels: None: the app passes them (`NotificationAction`, [L src/Lunegit.Desktop/Services/NotificationService.cs:92]).

### Gallery scenes

- `toast-notifications-stack` (new): info, success, warning and error toasts stacked, settled.
- `toast-notifications-loading` (new): one loading toast at 40% with a cancel control, settled.
- `toast-notifications-prompt` (new): one prompt toast with two actions, settled.
- `toast-notifications-expanded` (new): one expanded toast with a log block, settled.
- `toast-notifications-history` (new): the history panel with three items and the clear-all control, settled.

### Painter primitives

- Existing: backdrop, fill, ring, text, icon.
- New: none.
- The determinate progress bar is a `fill` of a width fraction; the indeterminate one has no source here (open question).

### Acceptance

- Behaviour (scripted input, injected time): info dismisses at 4 s, success at 3 s, warning at 6 s; error, loading and prompt never auto-dismiss; an expanded toast is not dismissed until collapsed; a fourth visible toast evicts the oldest non-loading, non-prompt toast; history is capped at 50. Source: [L src/Lunegit.Desktop/Services/NotificationService.cs:16-17,119-125,140-146,154-157,166-167]. None exists in the moved code.
- Pixel probes at scale 2: toast width 600 px minimum, radius 16 px, padding 24 px, stack gap 16 px, bottom-right margin 32 px.
- Conformance: none (no motion in the source).
- Semantic node: each toast `status` with polite live region; history `list`; actions named.

### Build

- Milestone: M7.
- Depends on: core.ui, core.focus, infra.accessibility, widget.button, data.progress.

## Open questions

- Toast layer above dialogs (Lunegit draws them over the dialog overlay, MainWindow.axaml:232,272) and the toolkit's top layer is Modal (scene.rs:17-22): open for M7.
- Toast appear and dismiss motion: none in Lunegit; open.
- Keyboard: no key handling found in the Lunegit toast markup; open.
- Role for error toasts: `alert` or `status` (plan says polite live region for all); open.
- Stacking order (newest at the bottom or top): the ItemsControl order is the collection order (MainWindow.axaml:273); the insert position was not read; open.
- Indeterminate loading progress look: no source in this area; open.

## Notes for the planner

Lunegit's toast code is the only source; the step read it (2026-11-04) by line, not from a file-name survey.

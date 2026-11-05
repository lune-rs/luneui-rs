---
id: layout.icon-rail
title: Icon rail
area: layout
status: in-progress
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/IconRailView.axaml
owner: toolkit-v1 M7
summary: Narrow icon column of 40 px toolbar buttons that switch the side panels and toggle the terminal, with a notification badge
---

## What it is

Lunegit's icon rail: a narrow column of 40 px icon buttons. Five sit at the top (repository, branches, commit, files, scratchpad), five at the bottom (settings, help, notifications, terminal, AI projects). Each button toggles one side panel by key, or the terminal; the notifications button carries an unread badge [L src/Lunegit.Desktop/Views/IconRailView.axaml:21-135].

## Needs

- Icon buttons from the Codicon set (R-core-6, R-widget-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/IconRailView.axaml`

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/Views/IconRailView.axaml:1-139`;
`src/Lunegit.Desktop/Themes/Animations.axaml:37-61` (`Button.toolbar-action` hover, press, transform);
`src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:160-163` (Reduced drops transforms);
`src/Lunegit.Infrastructure/Localization/en.json:73,200,225,318,343,391,773,839,865,931` (tooltip strings).
Conflicts: none. Wane and this repo have no icon rail.

### Anatomy

- Rail padding: left and right 4, top and bottom 10 [L src/Lunegit.Desktop/Views/IconRailView.axaml:21].
- Button: 40 × 40, transparent background, class `toolbar-action` [L src/Lunegit.Desktop/Views/IconRailView.axaml:24,31,85].
- Icon: 20 × 20, `ForegroundMuted` [L src/Lunegit.Desktop/Views/IconRailView.axaml:28,35,90].
- Top stack: spacing 4, five buttons in order: repository, branches, commit, files, scratchpad [L src/Lunegit.Desktop/Views/IconRailView.axaml:83-134].
- Bottom dock: settings, help, notifications, terminal, AI projects, docked bottom [L src/Lunegit.Desktop/Views/IconRailView.axaml:24-80].
- Commit dot: a `Border` shown when there are uncommitted changes [L src/Lunegit.Desktop/Views/IconRailView.axaml:109]; its size is not read (Open questions).
- Notification badge: a `Border` 16 high, min width 16, padding 2 0, margin 0 0 −4 −4 [L src/Lunegit.Desktop/Views/IconRailView.axaml:46-53]; hidden by default (`IsVisible="False"`), shown when there are unread items (binding in code-behind, not read).

### States

| state | look | cite |
|---|---|---|
| rest | transparent button, icon `ForegroundMuted` | [L src/Lunegit.Desktop/Views/IconRailView.axaml:24-28] |
| hover | background tween (Instant, linear) to the hover fill; the fill value is not read | [L src/Lunegit.Desktop/Themes/Animations.axaml:39-46] |
| pressed | scale 0.98 | [L src/Lunegit.Desktop/Themes/Animations.axaml:59-60] |
| panel active | not in the markup (the buttons toggle; no active class); open question | open question |
| commit with uncommitted changes | dot shown | [L src/Lunegit.Desktop/Views/IconRailView.axaml:109] |
| notifications with unread items | badge shown | [L src/Lunegit.Desktop/Views/IconRailView.axaml:46-53] |
| focus-visible | not in the markup; open question | open question |
| disabled | not in the markup; open question | open question |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| rest ↔ hover | background | hover | instant | linear | keep | [L src/Lunegit.Desktop/Themes/Animations.axaml:39-46] |
| press and release | render transform, scale 1 ↔ 0.98 | press | fast | quad-out | snap | [L src/Lunegit.Desktop/Themes/Animations.axaml:49-60], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:160-163] |

Never animated: the badge count and the commit dot (high-frequency content, [L .claude/docs/animation-rules.md:148]).

### Input and focus

- Pointer: a click toggles one side panel (`ToggleSidePanelCommand` with the panel key as the parameter) or the terminal (`ToggleTerminalPanelCommand`) [L src/Lunegit.Desktop/Views/IconRailView.axaml:26,33,41,67,77,87,95,103,121,129].
- Keyboard: none in the markup; open question.
- Tab stop: each button, top then bottom [core.focus] (derived: buttons in markup order).
- Layer: Base [R crates/lune-ui-core/src/scene.rs:17-22] [derived].
- Focus scope: none.
- Pointer capture while pressed: none in the markup.

### Retained state

- Transient: hover and press.
- Persistent: none.
- App-owned: the panel keys and toggle commands, `HasUncommittedChanges` (the commit dot), the unread count (the badge), the terminal open flag [L src/Lunegit.Desktop/Views/IconRailView.axaml:109,46].
- Exit retention: no.

### Accessibility

- Rail: role `toolbar`.
- Each button: `button` named by its tooltip (the English string) [infra.accessibility].
- Commit dot and badge: `status` text in the button name is not in any source; `role-gap`/name for the count is an open question.
- Live region: none; the badge count change is not announced in any source.

### Strings

- `luneui.icon-rail.repository`: "Repository" [L src/Lunegit.Infrastructure/Localization/en.json:225].
- `luneui.icon-rail.branches`: "Branches" [L src/Lunegit.Infrastructure/Localization/en.json:200].
- `luneui.icon-rail.commit`: "Commit" [L src/Lunegit.Infrastructure/Localization/en.json:318].
- `luneui.icon-rail.files`: "Files" [L src/Lunegit.Infrastructure/Localization/en.json:839].
- `luneui.icon-rail.scratchpad`: "Scratchpad" [L src/Lunegit.Infrastructure/Localization/en.json:865].
- `luneui.icon-rail.settings`: "Settings" [L src/Lunegit.Infrastructure/Localization/en.json:73].
- `luneui.icon-rail.help`: "Help" [L src/Lunegit.Infrastructure/Localization/en.json:343].
- `luneui.icon-rail.notifications`: "Notifications" [L src/Lunegit.Infrastructure/Localization/en.json:391].
- `luneui.icon-rail.terminal`: "Terminal" [L src/Lunegit.Infrastructure/Localization/en.json:773].
- `luneui.icon-rail.ai-projects`: "AI projects" [L src/Lunegit.Infrastructure/Localization/en.json:931].

### Gallery scenes

- `icon-rail-top` (new): five top buttons, settled.
- `icon-rail-bottom` (new): five bottom buttons with the notification badge shown, settled.
- `icon-rail-hover` (new): one button hovered, settled.
- `icon-rail-pressed` (new): one button pressed (scale 0.98), settled.
- `icon-rail-press-mid` (new): press at t = 90 ms (half of fast, 180 ms) [L src/Lunegit.Desktop/App.axaml:152].
- `icon-rail-commit-dot` (new): commit button with the uncommitted dot, settled.

### Painter primitives

- Existing: fill (hover fill), icon, text (badge count), xform (press scale). [R crates/lune-ui-core/src/paint.rs:215,171; crates/lune-ui-core/src/icon.rs]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): each button fires its toggle command once with its key; the dot and the badge follow their bindings. None exists.
- Pixel probes at scale 2: button 80 × 80 px; icon 40 px; rail padding 8 px sides and 20 px top and bottom; top stack gap 8 px.
- Conformance: press reaches scale 0.98 in 180 ms, quad-out, and back on release; Reduced snaps.
- Semantic node: `toolbar` with `button` children named by the strings above.

### Build

- Milestone: M7.
- Depends on: widget.button, widget.badge, layout.side-panel, layout.split-panes, core.focus, infra.accessibility, motion.transitions.

## Open questions

- Hover fill value and focus-visible look: the `toolbar-action` hover fill is not read; open.
- The commit dot's size and colour: not read (line 109-117); open.
- The badge count's accessible name: no source; open (role-gap if the count needs a node).
- Keyboard shortcuts for the rail buttons: none in the markup; open.
- Active-panel look: the markup has no active class; open (decide whether the toolkit adds one).

## Notes for the planner

Lunegit's rail is a switcher with no selection state of its own: the panels show and hide by key. The toolkit's version should decide whether the active panel is shown on the rail (open question).

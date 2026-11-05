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
- Tab stop: the rail once, on the active item (else the first enabled one); Up and Down move through the top group then the bottom, wrapping, Home and End go to the ends, a disabled item is skipped [core.focus] [D149] (Lunegit has a stop per button).
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
- Active item: the button reads `Pressed`. Commit dot and badge: the item's `note` is the button's description ("3 unread"), so the count has a name; the dot and badge draw no node of their own.
- Live region: none; the badge count change is not announced in any source.

### Strings

Dropped by [D149] (app text, hard rule 1): the keys below name an app's panels; the app passes each button's name.

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

- `icon-rail-top` (new): five top buttons, the first marked active, settled.
- `icon-rail-bottom` (new): five bottom buttons with the count badge shown on the third, settled.
- `icon-rail-hover` (new): one button hovered, its fill settled, the tooltip still pending.
- `icon-rail-pressed` (new): one button held down (scale 0.98), settled.
- `icon-rail-press-mid` (new): press at t = 90 ms (half of fast, 180 ms) [L src/Lunegit.Desktop/App.axaml:152].
- `icon-rail-commit-dot` (new): the third top button with the uncommitted dot, settled.

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

None. Closed by [D149]: the rail is a toolkit component (`Ui::icon_rail`, `RailItem`). Hover #10 and pressed #20 are the toolbar action's (`App.axaml:469-483`); focus-visible is the button's ring. The dot is 8 x 8, radius 4, `dirty_indicator` #D98A3D at (27, 7) from the button's top left (Lunegit margin 0,-3,-5,0 on a 20 px glyph panel); the badge is at least 16 wide and 16 high, radius 8, `count_badge` #E74856, 9 px bold `on_accent` text, right edge at x 34, top at y 10, "99+" above 99; both scale with the press and neither is animated; a count of 0 shows none. The count's name is the item's `note`. No keyboard shortcuts of its own: the app's `Binding`s. Active look (the toolkit's addition, Lunegit has none): `surface_selected` fill and `fg_primary` glyph, `Pressed` in the node. The rail draws no glass: the app's layer sits behind it. Each button has a tooltip of its name.

## Notes for the planner

Lunegit's rail is a switcher with no selection state of its own: the panels show and hide by key. The toolkit's version should decide whether the active panel is shown on the rail (open question).

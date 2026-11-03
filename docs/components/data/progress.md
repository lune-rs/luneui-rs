---
id: data.progress
title: Progress bar
area: data
status: planned
consumers: wane
source: —
owner: —
summary: 4 px track with an accent fill; warn and err only for meaning
---

## What it is

4 px track with an accent fill; warn and err only for meaning. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static. Indeterminate motion is not mocked.

## Needs

- Wane — Variants: default (accent); warn; err; with a head (name 13 left, RznMono value right) (R-data-2)
- Wane — States: a determinate fill only; the probe scene uses a pending row instead of an indeterminate bar. (R-data-2)
- Wane — Sizes: Track 4 high, radius 4, fill `--surface-overlay`; fill `--accent`; head 13/20 with 4 px below. (R-data-2)
- Wane — Behaviour: Static. Indeterminate motion is not mocked. (R-data-2)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Lunegit Desktop App/src/Lunegit.Desktop/App.axaml:253.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Wane `docs/ui/components.md:627-640`; Lunegit `src/Lunegit.Desktop/App.axaml:248-255` (ProgressBar foreground = `AccentPrimary`);
`src/Lunegit.Desktop/Views/StartupRepositoryLoadingView.axaml:40-43` (indeterminate bar, height 3); `crates/lune-ui-core/src/theme.rs:64` (`accent`).
Conflicts:
- Indeterminate height: Wane's track 4 [W docs/ui/components.md:627-640] vs Lunegit's indeterminate bar 3 [L src/Lunegit.Desktop/Views/StartupRepositoryLoadingView.axaml:42]. Determinate height is not set in Lunegit, so Wane's 4 is used for the determinate track; the indeterminate bar (not in Wane) takes Lunegit's 3.
- Accent value: the repo `accent` #767D85FF [R crates/lune-ui-core/src/theme.rs:64]; Lunegit's `AccentPrimary` key is referenced but its value was not read. The repo token is used; the values are an Open question.

### Anatomy

- Track: 4 high; radius 4; fill `--surface-overlay` [W docs/ui/components.md:627-640]. The repo has no `surface_overlay` token (`grep -n surface_overlay crates/lune-ui-core/src/theme.rs` empty): open question.
- Fill: `accent`, from the track's left, width = value fraction [W docs/ui/components.md:627-640], [R crates/lune-ui-core/src/theme.rs:64].
- Variants: warn, err (colours not in the sources read; Open questions).
- Head: name 13 at the left; value RznMono at the right; 13/20 with 4 px below [W docs/ui/components.md:627-640].
- Indeterminate: a moving 220 × 3 bar in Lunegit [L src/Lunegit.Desktop/Views/StartupRepositoryLoadingView.axaml:40-43].

### States

| state | look | cite |
|---|---|---|
| determinate | fill to the value fraction | [W docs/ui/components.md:627-640] |
| warn | warn fill (colour open) | [W docs/ui/components.md:627-640] |
| err | err fill (colour open) | [W docs/ui/components.md:627-640] |
| indeterminate | moving bar (Lunegit); not mocked in Wane | [L src/Lunegit.Desktop/Views/StartupRepositoryLoadingView.axaml:40-43] |
| disabled | not in the sources | open question |

### Transitions

n/a — the fill follows the value directly. A progress value updates continuously and is never animated
([L .claude/docs/animation-rules.md:148]). The indeterminate motion is not mocked and has no source for a motion kind (open question).

### Input and focus

- Pointer: none; static [W docs/ui/components.md:627-640].
- Keyboard: none.
- Tab stop: no.
- Layer: whatever layer the owner draws it on [derived: a bar].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: the value fraction, the variant (default, warn, err), the head text and value, the indeterminate flag [W docs/ui/components.md:627-640].
- Exit retention: no.

### Accessibility

- Role `progress`; value = number with min 0, max 100 when determinate; no value when indeterminate [infra.accessibility].
- Name = the head text; description = the variant meaning (err or warn) when set [infra.accessibility].
- Live region: none; a status change is announced by the owner.

### Strings

None: the head text is app text.

### Gallery scenes

- `progress-determinate` (new): fills at 40%, with a head, settled.
- `progress-variants` (new): default, warn and err bars, settled.
- `progress-pending` (new): the pending row the probe scene uses instead of an indeterminate bar, settled.

### Painter primitives

- Existing: fill (track and fill, radius 4 as a rounded rectangle), text (head). [R crates/lune-ui-core/src/paint.rs:215,274]
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): the fill width equals the value fraction at 0, 50 and 100%; a value outside 0–100 clamps. None exists.
- Pixel probes at scale 2: track 8 px high, radius 8 px, head 26 px line height, 8 px gap below the head.
- Conformance: none (no motion).
- Semantic node: `progress` with min 0 and max 100; no value when indeterminate.

### Build

- Milestone: M8.
- Depends on: core.painter, core.theme.

## Open questions

- The track colour `--surface-overlay`: no repo token; the value is open (a theme token decision).
- Warn and err colours: not in the sources read (the Lunegit toast accents at DarkTheme.axaml:337-338 are a candidate, not confirmed); open.
- Indeterminate motion: Lunegit has a moving bar and Wane does not mock it; the motion kind and duration are open.
- Indeterminate height 3 (Lunegit) versus determinate 4 (Wane): both kept as stated; open for the look owner.

## Notes for the planner

Wane priority P1 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.progress`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, pricing, sessions.

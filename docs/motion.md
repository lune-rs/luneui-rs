# Motion

The one motion standard for the toolkit: durations, easings, animation kinds, tiers, what never
animates, and the 60 fps contract. Catalogue entries name these tokens in their `### Transitions`
tables and never invent a number (`docs/components/README.md`, spec format SF5). The values are
Lunegit's; where a component has no Lunegit motion the row cites `[D40]`. Lunegit wins over Wane
where they differ, and the moved menu code is pixel-gated and wins over both [D50].

Pinned sources: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit
`.claude/docs/animation-rules.md:1-172`, `src/Lunegit.Desktop/App.axaml:147-155`,
`src/Lunegit.Desktop/Themes/Animations.axaml` (all selectors), `Themes/AnimationsReduced.axaml`
(all selectors), `Theming/AnimationApplicator.cs:17-59`, `Services/DialogService.cs:81-92`,
`.claude/docs/dialog-exit-timing.md`, `Controls/SpinBehavior.cs`,
`Controls/Terminal/TerminalTabDragController.cs:36-44`, `Views/TerminalPanelView.axaml.cs:413-440`;
Wane `docs/ui/mocks/css/tokens.css:115-119`, `docs/ui/mocks/css/components.css:360-363`,
`docs/ui/README.md:80-93`; this repo `crates/lune-ui-core/src/widgets/menu.rs:21-59`.

## Durations

The app passes time in each frame (hard rule 2); every value is evaluated in closed form at `t`, so
it does not depend on the frame rate [motion.transitions]. Five tokens, no intermediate values
[L .claude/docs/animation-rules.md:18-26].

| token | ms | Lunegit resource | Wane | use | cite |
|---|---|---|---|---|---|
| instant | 120 | `AnimDurationInstant` | `--dur-instant` | hover colour, tiny state flips | [L src/Lunegit.Desktop/App.axaml:151], [L .claude/docs/animation-rules.md:22], [W docs/ui/mocks/css/tokens.css:116] |
| fast | 180 | `AnimDurationFast` | `--dur-fast` | press, focus ring, small fades | [L src/Lunegit.Desktop/App.axaml:152], [L .claude/docs/animation-rules.md:23], [W docs/ui/mocks/css/tokens.css:117] |
| base | 250 | `AnimDurationBase` | `--dur-base` | default: most enter and exit, content fades, selection | [L src/Lunegit.Desktop/App.axaml:153], [L .claude/docs/animation-rules.md:24], [W docs/ui/mocks/css/tokens.css:118] |
| moderate | 350 | `AnimDurationModerate` | none | panels, dialogs, page and content swaps | [L src/Lunegit.Desktop/App.axaml:154], [L .claude/docs/animation-rules.md:25] |
| slow | 500 | `AnimDurationSlow` | none | full-screen or large reveals only | [L src/Lunegit.Desktop/App.axaml:155], [L .claude/docs/animation-rules.md:26] |

Not on the scale, by design: the flash highlight (3000 ms, `CubicEaseOut`, an attention cue)
[L .claude/docs/animation-rules.md:28], the busy-outline loops (2400 ms sweep, 1800 ms breath), and the
indeterminate progress loop (a 2400 ms trip of the segment under Full, the same 1800 ms breath of the fill
under Reduced, a static fill under Off [D118]), which are state indicators, not feedback on a user action
[L .claude/docs/animation-rules.md:30-38]. Nothing longer than `slow` animates except those and the spinner
[L .claude/docs/animation-rules.md:150].

Durations are theme tokens (`motion` tokens on `Theme`, M4); a component names the token, never the
milliseconds [L src/Lunegit.Desktop/App.axaml:147-150].

## Easings

Seven names, no others. The soft feel comes from the Quadratic and Sine families, not Cubic
[L .claude/docs/animation-rules.md:78].

| easing | curve | use | cite |
|---|---|---|---|
| linear | `t` | hover and colour, focus | [L .claude/docs/animation-rules.md:85] |
| quad-out | `1 - (1 - t)^2` | enter: fade, slide, scale in; press | [L .claude/docs/animation-rules.md:82] |
| quad-in | `t^2` | exit: eases away | [L .claude/docs/animation-rules.md:83] |
| sine-in-out | `-(cos(pi t) - 1) / 2` | move, resize, two-sided repositioning | [L .claude/docs/animation-rules.md:84] |
| back-out | `1 + c3 (t-1)^3 + c1 (t-1)^2` with c1 = 1.70158, c3 = c1 + 1 (slight overshoot) | emphasis only, rare, never on a routine action | [L .claude/docs/animation-rules.md:86], [derived: the standard back-out; Avalonia's `BackEaseOut` amplitude was not read] |
| cubic-out | cubic-bezier(0.215, 0.61, 0.355, 1); the moved menu open and Wane's `--ease-out` | [R crates/lune-ui-core/src/widgets/menu.rs:46-48], [W docs/ui/mocks/css/tokens.css:119] |
| ease-in | cubic-bezier(0.42, 0, 1, 1); the moved menu close | [R crates/lune-ui-core/src/widgets/menu.rs:52-56], [W docs/ui/mocks/css/components.css:361] |

The dialog and the Reduced dialog use `quad-out` in both directions, not `quad-in` on exit: the
exit is front-loaded so that the host, torn down after the exit retention (below), cuts an already
near-invisible tail [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:278-284],
[L src/Lunegit.Desktop/Themes/Animations.axaml:445-446]. The exit rows of the kinds table keep
`quad-in` as the default; a dialog-like surface cites this paragraph.

## Kinds

Every animation in a Transitions table has one of 15 kinds. The first nine are Lunegit's
interaction table; the last six are D40 additions.

| kind | property | default duration | default easing | magnitude | cite |
|---|---|---|---|---|---|
| hover | background, foreground, opacity | instant | linear | colour only; no lift or scale (Wane A11) | [L .claude/docs/animation-rules.md:102], [W docs/ui/README.md:92] |
| press | scale back to 1 on release | fast | quad-out | scale 0.98 | [L .claude/docs/animation-rules.md:95,103], [L src/Lunegit.Desktop/Themes/Animations.axaml:49-60] |
| focus | border or glow opacity | fast | linear | none | [L .claude/docs/animation-rules.md:104] |
| appear | opacity 0 to 1 and translateY 6-8 px to 0 | base | quad-out | slide 6-8 px | [L .claude/docs/animation-rules.md:94,105] |
| dismiss | opacity 1 to 0, optionally a small translate | fast or base | quad-in | slide 6-8 px at most | [L .claude/docs/animation-rules.md:94,106] |
| page-swap | crossfade, or fade and a small slide | moderate | quad-out (opacity), sine-in-out (the slide) | slide 6 px | [L .claude/docs/animation-rules.md:107], [L src/Lunegit.Desktop/Themes/Animations.axaml:297-304] |
| drawer | translateX and opacity | moderate | sine-in-out | slide 6 px | [L .claude/docs/animation-rules.md:108], [L src/Lunegit.Desktop/Themes/Animations.axaml:320-327] |
| pop | opacity and scale | base | quad-out | scale 0.96 to 1 | [L .claude/docs/animation-rules.md:96,109], [L src/Lunegit.Desktop/Themes/Animations.axaml:456-463] |
| selection | background opacity | fast | linear | colour only | [L .claude/docs/animation-rules.md:110] |
| move | position of one element between two stops | fast | sine-in-out | distance of the move | [D40], [derived: the move/resize row, animation-rules.md:84, with the 180 ms neighbour slide of `settle`] |
| expand | size of the container that expands or collapses (clip and height of the container only) | base | sine-in-out | container extent | [D40], [derived: two-sided resize, animation-rules.md:84, at the default token animation-rules.md:24] |
| value | a displayed quantity (fill extent, sweep angle, a number) towards a new target; retargets from the current value and velocity | fast | quad-out | target minus current | [D40], [derived: enter easing animation-rules.md:82 at the feedback token animation-rules.md:23] |
| spin | rotation of an indicator glyph, continuous | one revolution per 1200 ms, outside the scale | linear | 360 degrees per period | [D40], [L src/Lunegit.Desktop/Controls/SpinBehavior.cs:17,45-53] |
| scroll | scroll offset towards a target from wheel or trackpad input | fast | quad-out | input delta | [D40], [derived: enter easing animation-rules.md:82 at the feedback token animation-rules.md:23] |
| settle | neighbours slide into place after a drag reorder; the dragged item tracks the pointer 1:1 and never transitions | fast (180 ms) | sine-in-out | one slot | [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:44], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml.cs:413-431] |

`dismiss` takes `fast` for small surfaces and `base` for larger ones; a component says which
[L .claude/docs/animation-rules.md:106]. Magnitudes are small and soft: 6-8 px of travel, press 0.98,
pop 0.96 to 1 [L .claude/docs/animation-rules.md:88-96]. The popup scale in Lunegit's table
(0.96 to 1.0) is the kind `pop`; the moved menu is a mapped variant (below).

A transition that is interrupted retargets from the current value and velocity instead of queuing:
a user clicking quickly never waits on an animation [L .claude/docs/animation-rules.md:11],
[motion.transitions].

## M7 rows: toasts, menu rows, the tree's expand

Lunegit has no toast motion (no toast selector in `Animations.axaml`), so the toast rows are D40's
kinds at their defaults [D146]. The menu row hover and the tree's expand are answered here so the
catalogue tables cite one place.

| component | kind | property | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| toast appear | appear | opacity 0 to 1, y 8 to 0 | base | quad-out | fade | [D40], [D146] |
| toast leave | dismiss | opacity 1 to 0 | fast | quad-in | fade | [D40], [D146] |
| toast neighbours | move | y of the toasts above a leaving or arriving one | fast | sine-in-out | snap | [D40], [D146] |
| toast expand | expand | the toast's height to show its log; text wrapped once at the target width | base | sine-in-out | snap | [D40], [D146] |
| menu row hover | hover | the row's fill | instant | linear | keep | [D40], [D141] |
| tree expand | expand | the child block's clip height; one block in motion, another toggle snaps it | base | sine-in-out | snap | [D40], [D151] |

The expand duration is the kind's default, `base` (the entry's question on it is answered: no
separate token). Off snaps every row above.

## M8 rows: chart values, the table header, the board

Lunegit has no chart, table or board motion beyond the drag the tab strip already has, so the rows are D40's kinds
at their defaults [D162, D164, D165, D166, D168]. The charts' quantity is the normalised one (a bar segment's share of
the plot, a slice's share of the turn), so a scale change animates with no special case.

| component | kind | property | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| bar segment | value | a segment's extent as a share of the plot; `live: true` snaps | fast | quad-out | snap | [D40], [D164] |
| pie slice | value | a slice's sweep as a share of the turn; shares are divided by their sum each frame; from empty to data snaps; `live: true` snaps | fast | quad-out | snap | [D40], [D164] |
| table sorted header | selection | the sorted header's text colour and the arrow's opacity | fast | linear | keep | [D40], [D162] |
| table row hover and selection | hover, selection | the row fill, while the rows are still; rows never animate on a sort or a scroll | instant, fast | linear | keep | [D40], [D161] |
| board card dim | selection | the dragged card's source at `DIM_ALPHA` | fast | linear | fade | [D40], [D168] |
| board drop line | move | the line's y inside one column; across columns it snaps | fast | sine-in-out | snap | [D40], [D168] |
| board neighbours | move | the cards that make room or close a gap, one slot (`SLIDE`: the same token and easing as `settle`) | fast | sine-in-out | snap | [D40], [D168] |
| board landing | settle | the ghost into the card's slot after a release, wherever the model now puts it (accepted, refused, cancelled and keyboard moves are one path) | fast | sine-in-out | snap | [D40], [D168] |

The board's ghost tracks the pointer 1:1 and never transitions (`settle`). Off snaps every row above. The lane graph
never animates (a self-painting surface, below).

## M9 rows: the code block's button, the diff's hunk action

Lunegit animates neither the code block nor the diff beyond a button's hover, so the rows are D40's kinds at their defaults
[D186, D191]. Document text, the selection, lines and diff rows are not here: they never animate (below).

| component | kind | property | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| code block copy button | hover, press | the button fill, as the toolkit icon button | instant | linear | keep | [D40], [D186] |
| code block copy icon | appear | the copy glyph and the check mark cross-fade when a copy lands and when the feedback ends; the feedback's clock is the frame time the app passes | instant | linear | snap | [D40], [D186] |
| diff hunk action | hover, press | the action button's fill, as the toolkit icon button | instant | linear | keep | [D40], [D191] |

Off snaps every row above.

## Tiers

The app chooses the tier; the toolkit reads no OS setting (hard rule 2). Lunegit's three tiers are
Full, Reduced and None; this toolkit calls the third one Off [L src/Lunegit.Desktop/Theming/AnimationApplicator.cs:45-56].

- **Full**: everything in this document [L .claude/docs/animation-rules.md:156].
- **Reduced**: opacity and colour only. No transforms, slides or scales, no press-scale
  [L .claude/docs/animation-rules.md:157], [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:10-17]. A
  transform is replaced by an opacity fade (`fade`), a colour change keeps its tween (`keep`), a
  change with no opacity or colour part snaps (`snap`). Durations step down per the map below.
- **Off**: no transitions; every change snaps [L .claude/docs/animation-rules.md:158],
  [L src/Lunegit.Desktop/Theming/AnimationApplicator.cs:53-55].

Reduced duration map, read from the Reduced stylesheet against the Full one (the same selectors in
`AnimationsReduced.axaml` and `Animations.axaml`):

| Full token | Reduced token | evidence |
|---|---|---|
| instant | instant | hover tweens [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:24], [L src/Lunegit.Desktop/Themes/Animations.axaml:42] |
| fast | instant for exits and reveals, fast for selection | EULA exit Fast to Instant [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:333], [L src/Lunegit.Desktop/Themes/Animations.axaml:500]; scrollbar thumb Fast to Instant [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:444], [L src/Lunegit.Desktop/Themes/Animations.axaml:625]; list selection stays Fast [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:190], [L src/Lunegit.Desktop/Themes/Animations.axaml:276] |
| base | fast | menus, reveals, dialog enter and exit, tab crossfade [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:151,169,260,289], [L src/Lunegit.Desktop/Themes/Animations.axaml:219,252,388,445], [L .claude/docs/animation-rules.md:67-68] |
| moderate | fast | page swap and panels [L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:203,217], [L src/Lunegit.Desktop/Themes/Animations.axaml:300,323] |
| slow | base | [derived: Lunegit has no `slow` use in either stylesheet; the rule is "durations effectively halved", animation-rules.md:157] |

The map is the rule "one step down" (two for `moderate`), with the selection and the other
colour tweens that are already short left alone. A component's `reduced` column says `keep`,
`fade` or `snap`; the duration then follows this map.

**Dialog exit retention.** A dialog the app stops calling keeps drawing from retained state until
its exit transition ends: Full 180 ms, Reduced 120 ms, Off 0 ms, then it reports Idle. Lunegit
holds the dialog for exactly this long before tearing it down
[L src/Lunegit.Desktop/Services/DialogService.cs:87-92], and the caller's work waits behind it
[L .claude/docs/dialog-exit-timing.md:18-44]. The same retention applies to the popover, menu and
toast exits [core.ui].

## Never animate

From Lunegit's rules plus D40's performance rules:

- Virtualized list rows on scroll (commit list, file tree, diff rows): recycling plus per-row
  transitions is jank [L .claude/docs/animation-rules.md:145]. Rows do not animate during scroll
  [D40]. A deliberately hovered single row may tween its own background (the dwell means no row is
  built while scrolling) [L src/Lunegit.Desktop/Themes/Animations.axaml:597-605].
- Expand and collapse animate the container, not each row inside it [D40].
- The commit-graph canvas and any self-painting surface (the terminal): no transitions over it
  [L .claude/docs/animation-rules.md:146-147].
- High-frequency or continuously updating content: progress text, counters, anything that changes
  many times per second [L .claude/docs/animation-rules.md:148].
  A chart value escapes it with `live: true`: the app says the quantity changes often, and the bars and slices
  follow it directly, as the stat's `live` does [D126, D164].
- Document text, the selection, lines of a code view, diff rows, and the mode and wrap switches of a diff or a code
  view (M9): they change what the document is, not how it is shown [D186, D191].
- Layout properties on large or virtualized surfaces; prefer transforms
  [L .claude/docs/animation-rules.md:149].
- Anything longer than `slow` (500 ms), except the flash cue and the indicator loops
  [L .claude/docs/animation-rules.md:150].
- A determinate progress bar: its fill follows the value directly, however often the value changes.
  Only the indeterminate loop above moves, because nothing is known [D118]
  [L .claude/docs/animation-rules.md:148].

## Tab-crossfade exception

Switching tabs on a self-painting surface crossfades a wrapper around it: the outgoing wrapper fades
1 to 0 while the incoming one fades 0 to 1 over `base` (`sine-in-out`; Reduced: `fast`), and the
outgoing one is hidden only after its fade ends [L .claude/docs/animation-rules.md:40-43],
[L src/Lunegit.Desktop/Themes/Animations.axaml:216-222],
[L src/Lunegit.Desktop/Themes/AnimationsReduced.axaml:148-155]. It is the one carve-out to the
never-animate list for a self-painting surface, and it is bounded: nothing animates the surface
itself, steady state is the plain surface at opacity 1 or hidden, it runs only on an explicit
switch, never loops, never delays input (the active tab, focus and the notch snap; only pixels catch
up), and a second switch supersedes the fade instead of queuing
[L .claude/docs/animation-rules.md:45-65]. Off is a hard cut. It is not a licence to
transition any other self-painting surface [L .claude/docs/animation-rules.md:73-74].

## 60 fps contract

D41, in three parts [D41]:

1. **Conformance.** Every component transition passes the 60 Hz conformance harness in
   `cargo test`: `motion::conformance` samples the transition at 60 Hz and checks no jump above its
   bound, the exact end value, settled within duration plus one frame, and no NaN.
2. **Bench budget.** `gallery --bench` on every page with its animations running, at 1040x660 at
   2x offscreen on this Mac: p99 full frame time (CPU build, encode and GPU wait) at or below
   16.6 ms and no frame over 33 ms after 3 warm-up frames. One extra run at the full panel size
   (3456x2234) is report-only. The budget is enforced at milestone acceptance only, in the release profile, never
   in the per-commit gate; each of five runs is judged on its own and a page fails only when at
   least half its runs do, with the machine's load and the self-test's spread printed beside the
   verdict (D85). There is no
   memory target in v1.
3. **Idle when settled.** A settled page reports `Repaint::Idle`: zero idle frames. A running
   transition reports `Now` (or `At(t)`) until it settles [core.ui].

## Moved menu mapping

The moved glass menu keeps its pixel-gated values; its two constants map onto the tokens, so the
rest of the toolkit can speak in tokens without moving a pixel [R crates/lune-ui-core/src/widgets/menu.rs:21-22].

| moved name | value | token | easing | motion | cite |
|---|---|---|---|---|---|
| `MENU_IN` | 0.18 s | fast | cubic-out | opacity 0 to 1, 4 px lower to rest, scale 0.97 to 1, about the top-left corner | [R crates/lune-ui-core/src/widgets/menu.rs:21,45-49], [W docs/ui/mocks/css/components.css:360,362] |
| `MENU_OUT` | 0.12 s | instant | ease-in | opacity 1 to 0, 2 px lower, scale to 0.98 | [R crates/lune-ui-core/src/widgets/menu.rs:22,51-57], [W docs/ui/mocks/css/components.css:361,363] |

This is a pop (open) and a dismiss (close), but with its own magnitudes and curves, which is why
`cubic-out` and `ease-in` exist among the seven easings. Lunegit's own menu fades opacity only
over `base` (`quad-out`) because a recreated flyout cannot run a transform
[L src/Lunegit.Desktop/Themes/Animations.axaml:250-256], [L .claude/docs/animation-rules.md:130]; the
moved code wins [D50].

## Reading a Transitions table

`| change | property | kind | duration | easing | reduced | cite |`: the kind is one of the 15 above, the
duration one of the 5 tokens, the easing one of the 7 names; `reduced` is `keep`, `fade` or `snap`
per property. A row Lunegit does not have cites `[D40]`. After the table the entry names what never
animates from the list above.

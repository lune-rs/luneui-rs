---
id: text.diff-view
title: Diff view (unified, split, hunk actions)
area: text
status: in-progress
consumers: lunegit, wane
source: crates/lune-ui-core/src/widgets/diff_view.rs
owner: toolkit-v1 M9
summary: Unified and split diff over an app-supplied line model: line numbers, add and remove tints, hunk header rows with actions, row selection, intra-line marks
---

## What it is

A read-only diff of an app-supplied list of lines, shown unified (one column of lines, two line-number
columns) or split (old and new side by side), with hunk header rows that can carry action buttons,
row selection and copy, word wrap, and optional intra-line marks over changed spans. The toolkit never
runs a diff algorithm and never reads a repository (D13, D45): the app passes lines typed added, removed,
context or hunk, with their numbers. Lunegit's `DiffView` hosts a unified and a split sub-view; Wane
needs a before and after diff for what a compression stage changed.

## Needs

- Text view with per-line backgrounds (R-text-3, R-data-1).
- Keyboard bindings (R-infra-2).
- Virtualized rows (R-layout-3), scrolling and offset preservation (R-layout-2).
- Wane — Before / after diff view: "Requests 'View before / after diff' shows what a compression stage changed" (R-data-1).

## Existing code

None in `lune-ui`. Design sources (not code to move): Lunegit `src/Lunegit.Desktop/Views/DiffView.axaml` (210 lines, header and mode switch), `Views/UnifiedDiffView.axaml` (202), `Views/SplitDiffView.axaml` (314) and `.axaml.cs` (276, selection and scroll sync),
`Converters/DiffLineBackgroundConverter.cs`, `Converters/DiffLineForegroundConverter.cs`, `Converters/DiffLinePrefixConverter.cs`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/DiffView.axaml:1-210, Views/UnifiedDiffView.axaml:1-202, Views/SplitDiffView.axaml:1-314 (columns, rows, separator),
Views/SplitDiffView.axaml.cs:15-275, Views/FileViewerView.axaml:155-160,333-336, Converters/DiffLineBackgroundConverter.cs:14-36, Converters/DiffLineForegroundConverter.cs:14-30, Converters/DiffLinePrefixConverter.cs:11-26,
Themes/DarkTheme.axaml:12,17,27-36,48,106-110, src/Lunegit.Infrastructure/Localization/en.json:244-254; Wane docs/ui/components.md:725.
A grep for `intraline`, `intra-line`, `WordDiff`, `InlineChange` and `CharDiff` over `src` (instrument: `grep -rln -i`, 2026-11-04) finds no file: Lunegit marks no changed words inside a line. The contract's intra-line marks are therefore a toolkit addition (Open questions), not read from Lunegit.
No conflicts among sources.

### Anatomy

- Line model, supplied by the app: each line has a type (added, removed, context or hunk header), an old and a new line number (either may be empty), its text, and for hunk rows a hunk index [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:98-112,148-169]. The split form pairs a left and a right line per row [L src/Lunegit.Desktop/Views/SplitDiffView.axaml:92-100].
- Unified row: minimum height 20; columns 48 (old number), 48 (new number), a 10 px prefix column with margin 4 on both sides, and the text; numbers mono 13, right-aligned with margin right 8, in `fg_faint` ([D58]); text mono 13 [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:99-144].
- Split view: two panes of equal width with a 1 px separator between them (`*, 1, *`); each pane's row has a 48 px line-number column, a 10 px prefix and the text; minimum row height 20; each pane scrolls on its own and the second follows the first in the vertical direction [L src/Lunegit.Desktop/Views/SplitDiffView.axaml:45,92-117,179], [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:107-113].
- Line look: added rows `#1A73C991` fill and `#73C991` text with prefix "+"; removed rows `#1AE06C75` fill and `#E06C75` text with prefix "-"; context rows transparent, `fg_primary`, prefix a space; hunk rows no prefix, text `#61AFEF` [L src/Lunegit.Desktop/Converters/DiffLineBackgroundConverter.cs:21-26], [L src/Lunegit.Desktop/Converters/DiffLineForegroundConverter.cs:21-27], [L src/Lunegit.Desktop/Converters/DiffLinePrefixConverter.cs:18-24], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:106-110].
- Hunk header row: a 1 px `SurfaceSeparator` line, a centred pill (fill `surface_selected`, radius 10, padding 12 and 2, the hunk text in mono 9 px `fg_quaternary` ([D58]) [L src/Lunegit.Desktop/App.axaml:157]), the hunk actions, a second separator line; margin 2 above and below [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:148-192], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:10,30]. Built: the row keeps its 20 px pitch with the 18 px buttons 1 px clear, 2 px under Lunegit's 22 (D198).
- Hunk action button: 18 x 18, radius 5, fill `SurfaceOverlay`, 1 px `SurfaceBorder`, a 10 px icon (plus or minus), `fg_secondary`; a pointer over it: fill `surface_hover`, border `SurfaceDivider`, icon `fg_primary`; pressed: fill `SurfaceActive`, icon `ForegroundBright` [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:11-41,165-188], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:17,27,36]. They show only when the app says the diff is in a mode where actions apply, one action kind per file state (the app decides which kinds) [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:163-188].
- Row selection: a set of selected rows with an overlay `#33767D85` (the `accent` at alpha 0x33) over the row; click selects one row and sets the anchor, Meta-click toggles, Shift-click selects the range from the anchor, press and drag selects the range under the pointer; in the split form the selection lives in one pane and clears when the other pane is used [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:48], [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:133-215], [R crates/lune-ui-core/src/theme.rs:64].
- Copy: the selected lines, in order, joined by line breaks, go to the clipboard; a context menu offers the line actions the app defines and "Copy selected lines" with the shortcut Cmd+C [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:253-275], [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:57-77].
- Word wrap: on, long lines wrap and the horizontal bar is off; off, lines run on and the horizontal bar shows [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:49,143].
- Mode and options: the header (path, status badge, a "..." options button with unified, side-by-side and word-wrap rows, a close button) is composition, not the diff body: the file viewer hides the diff's own header and hosts its own options button [L src/Lunegit.Desktop/Views/DiffView.axaml:87-196], [L src/Lunegit.Desktop/Views/FileViewerView.axaml:155-160,333-336]. The toolkit supplies the body; the header is built from `widget.toolbar`, `overlay.popover` and `overlay.menu`.
- Intra-line marks: optional per-line spans the app supplies (start, end, added or removed), drawn as a stronger tint of the line's fill under the text; clamped like highlight spans. Not in Lunegit (Evidence); the tint value is open.
- Syntax colours inside diff text: through the same app-supplied spans as `text.code-editor-view` [D45], [text.code-editor-view]. Lunegit's diff rows colour a whole line by its type and apply no syntax colour [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:137-144].
- Shared text view: the rows are drawn as the code editor view's lines with per-line backgrounds (D42) [text.code-editor-view].

### States

| state | look | cite |
|---|---|---|
| context row | transparent, `fg_primary` | [L src/Lunegit.Desktop/Converters/DiffLineBackgroundConverter.cs:25], [L src/Lunegit.Desktop/Converters/DiffLineForegroundConverter.cs:26] |
| added row | fill `#1A73C991`, text `#73C991`, prefix "+" | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:106,108] |
| removed row | fill `#1AE06C75`, text `#E06C75`, prefix "-" | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:107,109] |
| hunk row | separators, pill, text `fg_quaternary` (the pill's, D198) | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:110], [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:148-192] |
| row selected | overlay `#33767D85` | [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:48] |
| row hover | none: the row style keeps the background transparent on hover and on selection | [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:81-96] |
| hunk action, rest | 18 x 18 button, `SurfaceOverlay` fill | [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:11-21] |
| hunk action, hover | fill `surface_hover`, icon `fg_primary` | [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:28-34] |
| hunk action, pressed | fill `SurfaceActive`, icon bright | [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:35-41] |
| hunk action, focus-visible | outline per the focus ring | [core.focus] |
| hunk action, disabled | opacity 0.4 | [derived: Lunegit's disabled convention, App.axaml:482] |
| word wrap on and off | wrapped lines, or a horizontal bar | [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:49,143] |
| empty | no rows; the app shows its empty state ("Select a file to view its diff") | [L src/Lunegit.Desktop/Views/CommitContentView.axaml:44-53], [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:44-45] |
| intra-line mark | stronger tint under the marked span | [derived: not in Lunegit; look open] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| hunk action hover | button fill and border | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102], [D40] |
| hunk action press | button fill | press | fast | quad-out | keep | [L .claude/docs/animation-rules.md:103], [D40] |

Rows never animate: the diff rows are virtualized, and recycled rows would tween spuriously [L .claude/docs/animation-rules.md:145]; selection, view-mode switches and word-wrap toggles replace the rows at once (Lunegit switches the two sub-views by visibility, no transition [L src/Lunegit.Desktop/Views/DiffView.axaml:203-206]). Lunegit's hunk buttons have no transition in either tier file; the hover and press rows are [D40] additions, and the 0.98 press scale of the `icon_button` host moves an 18 px edge by under 0.2 px (unseen).

### Input and focus

- Pointer: click, Meta-click, Shift-click and drag as in Anatomy; the hunk action buttons take their own presses, a press that starts on a button or a scroll bar is not a row press [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:133-136,180-189]; wheel and trackpad scroll; right press opens the context menu [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:57-77].
- Keys: the copy shortcut copies the selected lines [L src/Lunegit.Desktop/Views/UnifiedDiffView.axaml:70-71]; Up and Down move the selection and PageUp and PageDown scroll, as in `layout.virtualized-list` [layout.virtualized-list].
- Tab stops: the diff body is one stop; each visible hunk action is a stop in call order, a row in the list's margin is none, and a focus move brings its button into view (D200) [core.focus]. Layer: the host's. Context menu: `Popup` [core.ui]. No focus scope.

### Retained state

- Transient: hunk button hover and press, selection drag [core.ui].
- Persistent: the selected row keys and anchor, the scroll offsets of the unified view and of the split panes; the offset is kept when the app replaces the lines with the same document key, and a different key starts at the top [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:102-104], [layout.scroll-area]. A new key also ends what its rows held: a hunk button loses the focus (D72) even when the new document has one in its place; the focus on a pane stays (D202). Both split panes draw the new document's top in the frame of the switch, however close it lands after a scroll (D203).
- App-owned: the lines, the document key, the view mode (unified or split), word wrap, whether hunk actions show and of which kind, the intra-line spans.
- Exit retention: no.
- Selection is by row key: a row scrolled out of view stays selected [layout.virtualized-list].

### Accessibility

- Role `table` for the diff with `row` per line and `cell` for numbers and text; name = the file name the app passes; a row's description says added, removed or context; states selected per row [infra.accessibility].
- Hunk action: role `button`, name = the app's action label; default invokes it [infra.accessibility].
- The semantic tree holds only the built rows (virtualized) [layout.virtualized-list].

### Strings

- `luneui.diff-view.unified`: "Unified diff" [L src/Lunegit.Infrastructure/Localization/en.json:244].
- `luneui.diff-view.side-by-side`: "Side-by-side diff" [L src/Lunegit.Infrastructure/Localization/en.json:245].
- `luneui.diff-view.options`: "Diff options" [L src/Lunegit.Infrastructure/Localization/en.json:246].
- `luneui.diff-view.word-wrap`: "Word wrap" [L src/Lunegit.Infrastructure/Localization/en.json:247].
- `luneui.diff-view.copy-selected-lines`: "Copy selected lines" [L src/Lunegit.Infrastructure/Localization/en.json:252].
- `luneui.diff-view.pane-old`, `luneui.diff-view.pane-new`: "old" and "new", the suffix of a split pane's table name (`Changes (old)`); the toolkit's own, no Lunegit source (D199).

Hunk and line action labels (Lunegit: "Stage hunk", "Unstage hunk", "Stage line", "Unstage line", [L src/Lunegit.Infrastructure/Localization/en.json:248-251]) and the empty-state texts [L src/Lunegit.Infrastructure/Localization/en.json:253-254] are app strings.

### Gallery scenes

- `diff-view-unified`: a diff of two hunks with context, added and removed lines, settled.
- `diff-view-split`: the same diff in split form, settled.
- `diff-view-hunk-actions`: hunk rows with one action button each, one hovered, settled.
- `diff-view-hunk-pressed`: one action button pressed, settled.
- `diff-view-selection`: rows 4 to 7 selected by Shift-click, settled.
- `diff-view-wrapped`: word wrap on with a long line, settled.
- `diff-view-wide`: word wrap off with a long line and the horizontal bar, pointer over, settled.
- `diff-view-intraline`: a changed line with marked spans, settled.
- `diff-view-empty`: no rows (the empty state), settled.
- `diff-view-scrolled`: offset 4000 px, settled.
- `diff-view-hunk-hover-fade`: a hunk action entered, t = 60 ms (half of `instant`).
- `diff-view-hunk-press-fade`: a hunk action pressed, t = 90 ms (half of `fast`).
- `diff-view-hunk-focus`: Tab through the header's three stops and the body to the first hunk action: its ring, settled (D198).
- `diff-view-hunk-disabled`: the hunk actions in a disabled scope, settled.

### Painter primitives

- Existing: fill (row tints, selection overlay, separators), fill-radii (the pill, buttons), ring (button border), text, icon, clip-rect, xform.
- New: none.

### Acceptance

- Behaviour (scripted input): click selects one row; Meta-click toggles; Shift-click selects the range from the anchor; the copy shortcut puts the selected lines, in row order, in one clipboard request; in split form, a selection change in the other pane, by any input, clears the first pane's (D201) [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:144-173].
- Hunk actions: a press on a hunk button reports `(hunk_index, action)` and does not select a row; with actions off the buttons are absent.
- Preservation: replacing the lines with the same document key keeps the offset (clamped); a different key starts at the top [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:102-104].
- Synced split scroll: scrolling the left pane to 120 px puts the right at 120 and does not echo [L src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs:107-113].
- Spans: intra-line spans past the line end are clamped, empty or inverted skipped, never a panic [D45].
- Scale: a diff of 100,000 rows builds only the visible rows [D41].
- Pixel probes at scale 2: row slot at least 40 device px, number column 96 px, prefix column 20 px, split separator 2 px, hunk button 36 px with radius 10; added fill `#1A73C991` over the dark preset's surface.
- Conformance: the two transitions pass `motion::conformance`; Reduced keeps them, Off snaps [motion].
- Semantic node: role `table`, one `row` per built line, `selected` follows the key set; an injected `AccessAction` default on a hunk action invokes it.

### Build

- Milestone: M9. Built in M9 step 6: `Ui::diff_view`, `Ui::diff_selected_text`, `DiffLine`, `LineKind`, `DiffMark`, `SplitRow`, `DiffMode`, `HunkAction`, `DiffOptions`, `DiffResponse`, `pair_rows`, `widest_chars`, 14 scenes on the page `diff-view`.
- Depends on: text.code-editor-view, layout.virtualized-list, layout.scroll-area, widget.scrollbar, overlay.menu, widget.toolbar, platform.clipboard, core.ui, core.focus, infra.accessibility, motion.transitions

## Open questions

Answered in M9 (D15, D16):

- The intra-line marks: `DiffMark { start, end }` in chars of the line, clamped like a highlight span; the tint is the theme's `diff_added_mark` / `diff_removed_mark` (the fill's hue at alpha 0x40, a look deviation, D14). Word-level computation stays the app's.
- One row model for both forms: yes. `DiffLine` is the one model; split takes `&[SplitRow]` (index pairs into the lines) from the pure `pair_rows`, which the app calls when its lines change.
- Caps on very large diffs: none; 100,000 rows build only the window, `pair_rows` and `widest_chars` are O(lines) and the app calls them on change.

Built as the entry's contract, with these notes:

- The hunk text is `fg_quaternary` inside the pill, as Lunegit's template draws it; `DiffHunkForeground` (`link`) reaches only the normal-line text, which a hunk row hides (D198).
- A hunk row keeps the 20 px row; the 18 px buttons sit 1 px from its edges (Lunegit grows the row to 22).
- The hunk action is an `icon_button` host: its 0.98 press scale reaches what the host draws, not the ring the diff paints, and on 18 px moves an edge by under 0.2 px, so it cannot be seen.
- Number and prefix columns stay put while the text scrolls sideways (Lunegit's list moves them).
- A split pane's table node is named `<label> (old)` and `<label> (new)` from two toolkit strings, so a screen reader tells the two tables apart (D199); the unified pane keeps `label`. The diff's `context` is the pane's line index, `None` over an empty half. `Ui::diff_selected_text` gives the text Command+C writes, for the app's "Copy selected lines" row.

## Notes for the planner

Renamed from the Lunegit-area diff entry (D10, D42): the diff body is generic, git words (stage, unstage, staging mode) are the app's action labels. The stub's question "text view with overlays or custom drawn" is answered: Lunegit's diff rows are plain list rows with typed backgrounds, not an editor control; `LineBackgroundHighlighter` belongs to the editor panes (`text.code-editor-view`).

---
id: text.code-editor-view
title: Code viewer and editor with syntax highlighting
area: text
status: in-progress
consumers: lunegit, wane
source: crates/lune-ui-core/src/widgets/code_editor.rs
owner: toolkit-v1 M9
summary: Multi-line code view and editor over an app-supplied document: gutter, app-supplied highlight spans, line bands, synced scrolling, overlay anchors
---

## What it is

A scrollable monospace text view over a document the app supplies, with a line-number gutter, colours
from app-supplied highlight spans (D45), full-row line bands, optional editing (the editing engine,
`text.text-edit`) and anchors for overlays pinned to a line. Lunegit uses AvaloniaEdit's `TextEditor`
with TextMate highlighting in four places: a read-only file viewer, an editable note editor, and the
four panes of the conflict resolver (three read-only panes scrolling together plus an editable result).
This entry holds all of them; the conflict resolver's line bands and synced scroll fold in here (D42).
Parsing, tokenizing and the grammar registry stay in the app (D13, D45).

## Needs

- Multi-line text view and editor with highlighting (R-text-3).
- Selection, caret and clipboard (R-text-5) and IME (R-text-4) when it edits.
- Virtualized line rendering (R-layout-3) and scrolling (R-layout-2).
- Diff and conflict views need per-line backgrounds (R-data-1).

## Existing code

None in `lune-ui`. Design sources (not code to move): Lunegit `src/Lunegit.Desktop/Views/FileViewerView.axaml:333-346` (viewer), `Views/Scratchpad/ScratchpadNoteView.axaml:101-120` (editable),
`Views/ConflictResolverView.axaml:161-305` and `.axaml.cs:19-98` (panes, synced scroll), `Rendering/LineBackgroundHighlighter.cs` (bands), `Rendering/ConflictHunkStripOverlay.cs` (anchored strips),
`Markdown/TextMateEditorSupport.cs` (the grammar wiring that stays in the app).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/FileViewerView.axaml:270-346, Views/Scratchpad/ScratchpadNoteView.axaml:100-121, Views/ConflictResolverView.axaml:161-205,288-355,
Views/ConflictResolverView.axaml.cs:1-98, Rendering/LineBackgroundHighlighter.cs:1-60, Rendering/ConflictHunkStripOverlay.cs:1-75,300-310, Markdown/TextMateEditorSupport.cs:1-28,
ViewModels/Explorer/FileViewerViewModel.cs:78-90,560-570, Themes/DarkTheme.axaml:106-124; Wane docs/ui/components.md:724; this repo crates/lune-ui-core/src/theme.rs:11-45.
A grep for `FoldingManager`, `SearchPanel`, `InstallFolding`, `ColumnRuler`, `HighlightCurrentLine`, `ShowSpaces`, `ShowTabs` over `src/Lunegit.Desktop` finds nothing: Lunegit's editors have no folding, no in-editor search, no rulers and no whitespace marks.
Conflicts: none among sources; Wane needs an editable mono view for the Pricing "Edit as TOML" dialog with validation errors [W docs/ui/components.md:724], which this entry covers with the editable mode.

### Anatomy

- Text: mono face at `font_base` 13, transparent background, `fg_primary`; side panes of the conflict view use `fg_secondary` [L src/Lunegit.Desktop/Views/FileViewerView.axaml:342-345], [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml:195-198]. The view sits in an 8 px margin with no card, fill or border of its own: the framed look comes from the region behind it [L src/Lunegit.Desktop/Views/FileViewerView.axaml:333], [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadNoteView.axaml:101-106].
- Gutter: line numbers in `fg_faint` (white at alpha 0x44; the token is added by [D58]), shown in the viewer and in all four conflict panes, hidden in the note editor; the gutter's dotted separator is hidden in every pane [L src/Lunegit.Desktop/Views/FileViewerView.axaml:340-341], [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadNoteView.axaml:110], [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml.cs:45-59], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:12]. Gutter width follows the widest line number [derived: the numbers right-align to the widest; Lunegit's width is AvaloniaEdit's and is not read].
- Document: the app supplies the text and a line index; Lunegit caps a viewed text at 10,000,000 bytes and an image at 20,000,000 [L src/Lunegit.Desktop/ViewModels/Explorer/FileViewerViewModel.cs:80-84]. The view draws only the lines in the viewport [derived: R-layout-3; Lunegit's editor virtualizes its lines internally, not read].
- Highlight spans: per line, (start, end, colour) supplied by the app through a `Highlighter` trait with a plain default (no spans) [D13], [D45]. Lunegit tokenizes with TextMate and a Dark+ theme in the app layer and skips highlighting above 1,500,000 bytes, showing a warning [L src/Lunegit.Desktop/Markdown/TextMateEditorSupport.cs:15-19], [L src/Lunegit.Desktop/ViewModels/Explorer/FileViewerViewModel.cs:86-87,569], [L src/Lunegit.Desktop/Views/FileViewerView.axaml:309-313]. A span past the line end is clamped; an empty or inverted span is skipped; the view never panics on spans.
- Language by extension: the app maps a file extension to its highlighter; an unknown or empty extension means plain text [L src/Lunegit.Desktop/Markdown/TextMateEditorSupport.cs:21-27].
- Line bands: a full-row tint, as wide as the viewport, for each 1-based line number in a set; numbers below 1 or above the line count are skipped [L src/Lunegit.Desktop/Rendering/LineBackgroundHighlighter.cs:44-58]. Lunegit's tints: added `#1A73C991`, modified `#1AE2B93D`, renamed `#1A61AFEF`, conflicted `#1AF14C4C`; the diff add and remove backgrounds are `#1A73C991` and `#1AE06C75` [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:106-107,120-124].
- Synced panes: panes that share a font scroll together by pixel offset, equal offsets meaning aligned lines; a re-entrancy flag stops the echo; a hidden pane is skipped; the editable result pane scrolls on its own [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml.cs:19-23,38-42,69-86].
- Overlay anchors: an overlay (the conflict strips) pins to a line of the editable pane using the text view's visual-line geometry, so it matches the line bands; it is hidden when its line is scrolled out; the gaps around it are not hit-test-visible; it keeps a 16 px inset from the right edge [L src/Lunegit.Desktop/Rendering/ConflictHunkStripOverlay.cs:15-25,37,300-310]. The strip's look and buttons are the app's (`external`, [D42]); the view exposes `line_rect(line)` and the viewport.
- Read-only: no caret, as Lunegit suppresses it on the read-only panes [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml.cs:30-36]; a Tab stop, and a press focuses and selects, so the conflict panes stay selectable and text is copied through the engine (D181; Lunegit's panes take no focus) [text.text-edit].
- Wrap: off in the viewer, on in the note editor [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadNoteView.axaml:111].
- Notices above the text: a truncated-file banner (padding 12 and 6, `surface_input`, a 13 px warning icon and 11 px text) [L src/Lunegit.Desktop/Views/FileViewerView.axaml:319-330]; the app composes it with `widget.banner-empty-state`.

### States

| state | look | cite |
|---|---|---|
| rest, read-only | text, gutter, no caret | [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml.cs:30-36] |
| focused, editable | caret, text selectable | [text.text-edit] |
| selected | selection fill over the range | [text.text-edit] |
| line band | full-row tint on the banded lines | [L src/Lunegit.Desktop/Rendering/LineBackgroundHighlighter.cs:44-58] |
| highlight off | plain text with the large-file warning | [L src/Lunegit.Desktop/Views/FileViewerView.axaml:309-313] |
| truncated | banner above the text | [L src/Lunegit.Desktop/Views/FileViewerView.axaml:319-330] |
| wrapped | long lines wrap at the viewport | [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadNoteView.axaml:111] |
| empty | no lines; the app shows its empty state | [L src/Lunegit.Desktop/Views/FileViewerView.axaml:380-390] |
| hover, pressed | not applicable to the view (the text cursor shows over text) | [derived: Lunegit styles none] |
| focus-visible, editable | the caret is the focus mark; no outline on the view | [derived: Lunegit draws no outline on a TextEditor] |
| focus-visible, read-only | the ring, since there is no caret to show it | D181 |
| disabled | no caret, no selection, the keys taken and none run (D177); read-only is the usual inert state | [derived: no source] |

### Transitions

n/a — typing, caret movement, selection, scrolling and line bands are high-frequency content and never animate [L .claude/docs/animation-rules.md:145,148]; Lunegit styles no transition on its editors. An overlay anchor that comes into view appears without a tween; the overlay's own look is the app's.

### Input and focus

- Pointer: wheel and trackpad scroll (vertical, and horizontal when not wrapped) [layout.scroll-area]; click places the caret in the editable mode; drag selects; double-click selects a word [text.text-edit]. A press on an overlay anchor goes to the overlay first [core.ui].
- Keys, editable: every key of `text.text-edit`, run on the view. Enter inserts a line; Up and Down cross wrapped rows at the preferred x; PageUp and PageDown move the caret by a page of rows; caret motion reveals at once, in x and y.
- Keys, read-only: Select all and Copy reach the engine and nothing else; PageUp, PageDown, Home and End scroll, through the area; the arrows stay free, so a binding fires on them (D181) [layout.scroll-area].
- Tab stop: the view, in both modes. `tab_inserts` (default false): plain Tab inserts `\t` and Shift+Tab still leaves; `Ui::begin` reads the frame before's word from the view, so a view not drawn frees the key. Read-only never keeps it [core.focus].
- The caret starts at the document's top, not at its end as a field's does: a document opens there. A new `identity` is another document: caret, selection, undo, composition and pending paste go with the old one (D201). A pane of a `scroll_group` that opens one takes nothing from the group in that frame and takes the group to the top with it, whether the others open one too or not; a pane drawn for the first time joins the group where it stands (D203).
- Composition: on one line in place (`text.ime`). A selection on the line is replaced in the drawing; across lines the first line is cut at the selection and later lines stay as they are until the commit, a stated simplification. The spans of the composing line are asked for the shown text.
- Layer: the host's; overlay anchors above it on the same layer in call order [core.ui]. No focus scope.
- Word motions: the document is a `CodeDoc` (D179), and a word motion reads the lines either side of the caret's, one line further where the line past that edge holds text. It crosses one blank line as a `String` does; across two or more blank lines it stops at the blank line, a stated difference from `text.text-edit` on a `String` (D180).

### Retained state

- Transient: hover, the synced-scroll echo guard, anchor visibility [core.ui].
- Persistent: the scroll offsets, caret and selection, and (editable) the undo history, per view id, kept under `ui.keep_alive` for a hidden tab [core.ui], [text.text-edit].
- App-owned: the document text, the highlighter or the spans, the line band sets with their colours, read-only and wrap flags, the pane group that scrolls together.
- Exit retention: no.

### Accessibility

- Role `multiline-text`; read-only state in the viewer; name = the label the app passes (the file name); value = the text and the selection as char offsets [infra.accessibility].
- A document of up to 10,000,000 bytes cannot be re-exposed every frame: the node's `value` and `text` carry the lines in view joined by `\n`, and the selection clipped to them, in chars of that text; a set-value replaces the document under a paste's rules when editable (D185).

### Strings

- `luneui.code-editor.truncated`: "File truncated — showing partial content" [L src/Lunegit.Infrastructure/Localization/en.json:846].
- `luneui.code-editor.highlight-disabled`: "Syntax highlighting disabled for large files" [L src/Lunegit.Infrastructure/Localization/en.json:845].
- `luneui.code-editor.read-only`: "Read-only" [L src/Lunegit.Infrastructure/Localization/en.json:844].

### Gallery scenes

- `code-editor-view-readonly`: 40 lines of code, gutter, no caret, settled.
- `code-editor-view-highlighted`: the same with keyword, string and comment spans, settled.
- `code-editor-view-editable`: caret on line 3, settled.
- `code-editor-view-selection`: a selection across lines 3 to 5, settled.
- `code-editor-view-wrapped`: a long line wrapped, no gutter (the note editor), settled.
- `code-editor-view-bands`: lines 4 to 6 added, 9 modified, 12 conflicted, settled.
- `code-editor-view-bands-clamped`: band sets with lines 0 and 9999, clamped, settled.
- `code-editor-view-synced`: three read-only panes and an editable one, the first three at the same offset, settled.
- `code-editor-view-anchor`: two editable panes, an overlay pinned to line 7 of each: the left shows it, the right has scrolled it out of view, settled.
- `code-editor-view-truncated`: the truncated banner above the text, settled.
- `code-editor-view-highlight-off`: the large-file warning, plain text, settled.
- `code-editor-view-empty`: no lines, settled.
- `code-editor-view-bad-spans`: spans past the line end and inverted, clamped, settled.
- `code-editor-view-ime`: a Latin preedit on line 4 (D90), settled.
- `code-editor-view-focus`: read-only with keyboard focus: the ring, no caret, settled.

### Painter primitives

- Existing: fill (bands, selection), fill-over, text, icon, clip-rect, xform (scroll offset).
- New: none.

### Acceptance

- Behaviour (scripted input): wheel scrolls by the step and clamps; in the editable mode typing, caret keys, selection and undo follow `text.text-edit`; read-only mode ignores edits and shows no caret.
- Bands: a set containing 0, 3 and 10,000 on a 5-line document tints only line 3; an empty set draws nothing [L src/Lunegit.Desktop/Rendering/LineBackgroundHighlighter.cs:44-58].
- Spans: a span (3, 99) on a 10-character line draws 3 to 10; (5, 5) and (7, 2) draw nothing; spans for a missing line are ignored; none panics [D45].
- Synced scroll: scrolling pane A to 120 px puts panes B and C at 120; setting B does not re-trigger A (no echo); a hidden pane is skipped; the editable pane stays put [L src/Lunegit.Desktop/Views/ConflictResolverView.axaml.cs:69-86].
- Anchors: an anchor on line 7 sits at the line's rect, 16 px from the right edge; scrolled out of view it is hidden and not hit-testable [L src/Lunegit.Desktop/Rendering/ConflictHunkStripOverlay.cs:37,300-310].
- Scale: a document of 1,000,000 lines draws only the viewport's lines and its frame cost does not grow with the line count [D41].
- Pixel probes at scale 2: gutter numbers in `fg_faint`; a band row spans the viewport width; mono 13 px text.
- Conformance: none, no transitions.
- Semantic node: role `multiline-text`, read-only follows the mode, selection follows the engine; an injected `AccessAction` set-value changes the text when editable.

### Build

- Milestone: M9. Built in M9 step 4: `Ui::code_editor`, `Ui::code_line_rect`, `CodeOptions`, `LineBand`, 15 scenes on the page `code-editor-view`.
- Depends on: text.text-edit, layout.scroll-area, widget.scrollbar, layout.virtualized-list, text.shaping, platform.clipboard, core.ui, core.focus, infra.accessibility

## Open questions

Answered in M9 (D179 to D185, D194):

- Ranged text value: the node carries the lines in view (D185). The full-text query stays open for M10's bridge.
- Document representation: `CodeDoc`, one `String` and a line index (D179); at 11 MB and 1,000,000 lines an edit measures 0.9 ms.
- Folding and in-editor search: out of v1, as Lunegit has neither (D181).
- Gutter width and the selection fill: the numbers take `GUTTER_PAD` 4 and `GUTTER_GAP` 8 around the widest number, the text 2 from the column's edge; the selection is `text_selection`, with a 4 px sliver past a selected line's end (D195).
- A very long single line: shaping follows its length, so a multi-megabyte line is slow to first draw, then cached; no windowing inside a line in v1 (D194).

Built as the entry's contract, with these notes:

- `Ui::code_line_rect(key, line)` numbers lines from 1, as the gutter and `LineBand` do; `Highlighter::spans` and `CodeDoc::line` count from 0 (D195).
- The frame shapes the lines in view and nothing else: a highlighter is asked for those only, and a wrapped view also measures the `MARGIN` lines either side, undrawn, so a line that comes into view from above has already shifted the area.
- A reveal in a wrapped view measures the caret's line and what fills the view above it, then lands; a far line takes two frames at most.
- A read-only view follows no caret: a selection dragged past the edge does not scroll it.
- Word motions: see Input and focus.

## Notes for the planner

Likely the largest single piece (as the stub said). Build order: read-only viewer with spans and bands first; the editable mode rides on `text.text-edit`. The text-view is shared by `text.diff-view` (D42) for its per-line backgrounds.

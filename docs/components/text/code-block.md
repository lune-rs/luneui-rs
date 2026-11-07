---
id: text.code-block
title: Code block
area: text
status: done
consumers: lunegit, wane
source: crates/lune-ui-core/src/widgets/code_block.rs
owner: —
summary: Read-only mono block with optional app-supplied highlight spans, horizontal scroll, selectable text and a copy button with feedback
---

## What it is

A read-only block of monospace text on a faint fill: no wrap, horizontal scroll, text you can select
and copy, and a copy button in the top-right corner that copies the whole block. Colours come from
app-supplied highlight spans (D45); with none, every character is `fg_primary`. Lunegit draws it in
its Markdown preview (fenced code, with TextMate colours); Wane needs it for the Claude Code env block
and the data folder listing (key, operator and value in three text tones).

## Needs

- Multi-line text view with optional highlighting (R-text-3).
- Select and copy: the read-only text engine (R-text-5), the clipboard (R-platform-5).
- Horizontal overflow through the scroll area and scrollbar (R-layout-2, R-widget-2).

## Existing code

Built in M9 step 5: `crates/lune-ui-core/src/widgets/code_block.rs` (`Ui::code_block`, `code_block::height`, `CodeBlockOptions`, `CodeBlockResponse`), tests `crates/lune-ui-core/tests/widget_code_block.rs`. Design sources (not code to move): Lunegit `src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:68-80,176-187` (the block and its copy button),
`Markdown/TextMateInlineHighlighter.cs` (per-line colour segments), `Views/MarkdownPreviewView.axaml.cs:386-402` (copy); Wane `docs/ui/components.md:655-667`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:56-80,134-141,176-187, Views/MarkdownPreviewView.axaml.cs:386-402, Markdown/TextMateInlineHighlighter.cs:1-106,
Markdown/MarkdownBlocks.cs:19-20, Themes/DarkTheme.axaml:132-141, src/Lunegit.Infrastructure/Localization/en.json:857; Wane docs/ui/components.md:655-667; this repo crates/lune-ui-core/src/theme.rs:55-62.
Conflicts (Lunegit wins where both draw the block, PD6): (1) text size: Lunegit mono at `FontSizeS` 11 [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:68-72], [L src/Lunegit.Desktop/App.axaml:158]; Wane RznMono 13 on a 20 line [W docs/ui/components.md:660].
(2) padding: Lunegit 10 all round [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:177]; Wane 10 vertical and 12 horizontal [W docs/ui/components.md:660]. (3) border: Lunegit none; Wane 1 px `--border-subtle` [W docs/ui/components.md:660].
(4) colour: Lunegit TextMate colours; Wane none, three token tones [W docs/ui/components.md:658]. The fill agrees: `#0AFFFFFF` is the dark preset's `surface_input` [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:135], [R crates/lune-ui-core/src/theme.rs:59]; the radius agrees (6).
The block therefore takes text size, padding and an optional border as parameters whose defaults are Lunegit's; Wane's values are an app override (Open questions).

### Anatomy

- Shell: fill `surface_input`, radius 6, padding 10 [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:177], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:135]; an optional 1 px `border_subtle` ring (Wane) [W docs/ui/components.md:660].
- Text: mono face, 11 px, `fg_primary`, no wrap [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:68-72,179]; one line per source line; long lines scroll horizontally with the overlay scrollbar [W docs/ui/components.md:660,666], [layout.scroll-area], [widget.scrollbar].
- Text margin and bar (step 8, the director): the text's clip ends 4 px before the copy button's left edge, one straight right margin for every line (a text drawn over a button reads as neither, and the entry's remedy for the corner); the sideways extent follows, so scrolled fully right the last column ends at that edge. The overlay bar belongs to the whole box inside the ring: the pointer anywhere over the block, padding included, shows it, and it lies in the bottom padding at the box's edge, as Wane's `pre` scroller does (the default 10 px padding holds the 9 px track). In a one-line block scrolled fully right the thumb and the copy button overlap by about 4 px; the button is hit first and the thumb (at least 28 long) stays draggable left of it. [W docs/ui/mocks/css/components.css:642-649], [widget.scrollbar].
- Highlight spans: per line, a list of (start, end, colour or tone). Lunegit's segments are (text, optional hex colour) per line [L src/Lunegit.Desktop/Markdown/TextMateInlineHighlighter.cs:10,73-95]; the toolkit takes spans from the app and never tokenizes (D45) [D45]. Wane's three tones are key secondary, operator quaternary and value primary [W docs/ui/components.md:658].
- Bad spans: a span past the line end is clamped to it and an empty or inverted span is skipped, as Lunegit does when building segments [L src/Lunegit.Desktop/Markdown/TextMateInlineHighlighter.cs:78-80]; the block never panics on spans. A span list for lines that do not exist is ignored.
- Copy button: top-right of the block, no fill, padding 4, a 13 px copy icon in `fg_tertiary`; Wane places it 6 px from the top and right [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:73-80,181-184], [W docs/ui/components.md:660]. Built: 21 px square at the padding's corner, or 6 px inside the ring when `border` is on (D196).
- Copy: the whole block's text, not the selection, goes to the clipboard [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:386-402], [W docs/ui/components.md:661]. Selected text copies with the shortcut like any read-only text [text.text-edit].
- Copy feedback: Lunegit gives none (the handler only sets the text) [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:401]; Wane names "a short checkmark or toast", not mocked [W docs/ui/components.md:659]. The toolkit swaps the copy icon for a check mark for a period the app sets (Open questions) and announces "Copied" as a polite status.
- Large text: Lunegit's viewer skips highlighting above 1,500,000 bytes and warns [L src/Lunegit.Desktop/ViewModels/Explorer/FileViewerViewModel.cs:86-87], [L src/Lunegit.Desktop/Views/FileViewerView.axaml:309-313]; the block draws only what the app passes, so the cut-off is the app's.

### States

| state | look | cite |
|---|---|---|
| rest | fill `surface_input`, text `fg_primary` (or span colours), copy icon `fg_tertiary` | [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:73-80,177] |
| hover (copy button) | button fill `surface_hover` | [derived: Lunegit sets the button background transparent and no hover rule, MarkdownPreviewView.axaml:73-80; the toolkit button hover is used, widget.button] |
| pressed (copy button) | as the toolkit icon button | [widget.button] |
| focus-visible (copy button) | outline per the focus ring | [core.focus] |
| copied | copy icon replaced by a check mark | [W docs/ui/components.md:659] |
| text selected | selection fill over the range (read-only text engine) | [text.text-edit] |
| overflow | horizontal thumb on hover | [widget.scrollbar] |
| empty | an empty block at the padding height, copy button disabled | [derived: nothing to copy] |
| disabled | not applicable to a read-only block; the copy button is disabled only when empty | [derived: same] |

### Transitions

| change | property | kind | duration | easing | reduced | cite |
|---|---|---|---|---|---|---|
| copy button hover | button fill | hover | instant | linear | keep | [L .claude/docs/animation-rules.md:102] |
| copy icon to check mark and back | icon opacity (crossfade) | appear | instant | linear | keep | [D40], [L .claude/docs/animation-rules.md:105] |

Lunegit's copy button has no transition (neither tier file styles it). Never animated: the text, selection, scroll content [L .claude/docs/animation-rules.md:148].

### Input and focus

- Pointer: press and drag selects text; double-click selects a word; click on the copy button copies and shows the feedback [text.text-edit], [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:181-182]. Wheel and trackpad scroll horizontally when the block overflows [layout.scroll-area].
- Keys: select-all and copy on the shortcut modifier when the block has focus; Enter and Space on the focused copy button copy [core.focus], [text.text-edit].
- Tab stop: the block (to select by keyboard) and the copy button in call order. Layer: the host's. No focus scope.

### Retained state

- Transient: copy-button hover and press, the check-mark timer (from the frame time the app passes), selection while dragging [core.ui].
- Persistent: the selection and the horizontal scroll offset, kept under keep-alive [core.ui].
- App-owned: the text, the highlight spans, the feedback duration.
- Exit retention: no.

### Accessibility

- Role `multiline-text` with state read-only; name = the label the app passes (Wane: "Claude Code env block"); value = the text [infra.accessibility].
- Copy button: role `button`, name `luneui.code-block.copy`; action default copies. After a copy a node of role `status` with a polite live region says `luneui.code-block.copied` [infra.accessibility].

### Strings

- `luneui.code-block.copy`: "Copy code" [L src/Lunegit.Infrastructure/Localization/en.json:857].
- `luneui.code-block.copied`: "Copied" [derived: Lunegit gives no feedback text; Wane names a check mark or toast, components.md:659].

### Gallery scenes

- `code-block-rest`: five lines of plain text, settled.
- `code-block-highlighted`: the same lines with key, operator and value spans, settled.
- `code-block-overflow`: one line wider than the block, pointer over, thumb visible, settled.
- `code-block-hover-copy`: pointer over the copy button, settled.
- `code-block-focus-copy`: keyboard focus on the copy button, settled.
- `code-block-selection` (at 2x and 1x): two lines selected, one joined shape (D272), settled.
- `code-block-copied`: the check mark shown, settled.
- `code-block-empty`: empty text, copy disabled, settled.
- `code-block-bad-spans`: spans past the line end and an inverted span, clamped, settled.
- `code-block-copy-hover-fade`: t = 60 ms (half of `instant`).
- `code-block-copied-fade`: the icon crossfade, t = 60 ms.
- `code-block-wane`: Wane's look: 13 over 20, padding 12 by 10, the 1 px ring, an env block in the mock's three tones, settled.

### Painter primitives

- Existing: fill-radii, ring, text, icon, clip-rect, xform (scroll offset).
- New: none.

### Acceptance

- Behaviour (scripted input, injected time): click on the copy button emits one clipboard request with the whole text and shows the check mark for the set period, then restores the icon; selection plus the copy shortcut copies the selection only; a disabled (empty) copy button emits nothing [core.ui], [platform.clipboard].
- Spans: a span (3, 99) on a 10-character line draws 3 to 10; (5, 5) and (7, 2) draw nothing; spans for line 50 of a 5-line block are ignored; none panics [L src/Lunegit.Desktop/Markdown/TextMateInlineHighlighter.cs:78-80].
- Pixel probes at scale 2: radius 12, padding 20 device px, fill `surface_input`, copy icon 26 px; a 1 px ring (2 device px) only when the border is on.
- Conformance: the two transitions pass `motion::conformance`; Reduced keeps them, Off snaps [motion].
- Semantic node: role `multiline-text` read-only; the copy button's default action copies; the `status` node appears after the copy.
- Tests, bullet by bullet (`crates/lune-ui-core/tests/widget_code_block.rs`). Behaviour: `a_click_on_copy_writes_the_whole_text_and_the_check_shows_until_feedback_ms_exactly`, `a_second_copy_during_the_feedback_restarts_it`, `a_change_of_text_ends_the_feedback`, `enter_and_space_on_the_focused_button_copy`, `a_drag_selects_and_the_shortcut_copies_the_selection_not_the_whole`, `an_empty_block_disables_copy_and_emits_nothing`. Spans: `spans_clamp_empty_and_inverted_ones_draw_nothing_and_a_missing_line_is_ignored`, `spans_for_a_line_colour_that_line_only`. Pixel probes at scale 2: `at_scale_2_the_box_has_radius_12_padding_20_the_input_fill_and_a_26_px_copy_icon`, `a_ring_of_1_px_shows_only_when_the_border_is_on`, `the_roomy_look_is_13_over_20_with_the_padding_inside_the_border_and_the_button_at_6`. Conformance: `the_copy_hover_and_the_crossfade_conform_under_the_three_tiers`, `the_crossfade_runs_instant_in_full_and_snaps_off`. Node: `the_text_node_is_read_only_multiline_text_under_the_label`, `the_copy_button_is_a_named_button_whose_default_action_copies`, `the_status_node_is_polite_and_there_only_while_the_check_shows`. Wheel routing: `a_block_that_overflows_takes_the_horizontal_wheel_and_leaves_the_vertical_to_the_page`, `a_block_that_fits_gives_both_axes_to_the_page`.

### Build

- Milestone: M9. Built in M9 step 5: `Ui::code_block`, `code_block::height`, `CodeBlockOptions`, `CodeBlockResponse`, `COPY_FADE`, 12 scenes on the page `code-block`.
- Depends on: text.text-edit, platform.clipboard, layout.scroll-area, widget.scrollbar, widget.button, core.ui, core.focus, infra.accessibility, motion.transitions
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_code_block.rs::a_click_on_copy_writes_the_whole_text_and_the_check_shows_until_feedback_ms_exactly, crates/lune-ui-core/tests/widget_code_block.rs::an_empty_block_disables_copy_and_emits_nothing, crates/lune-ui-core/tests/widget_code_block.rs::a_drag_selects_and_the_shortcut_copies_the_selection_not_the_whole, crates/lune-ui-core/tests/widget_code_block.rs::spans_clamp_empty_and_inverted_ones_draw_nothing_and_a_missing_line_is_ignored, crates/lune-ui-core/tests/widget_code_block.rs::at_scale_2_the_box_has_radius_12_padding_20_the_input_fill_and_a_26_px_copy_icon, crates/lune-ui-core/tests/widget_code_block.rs::the_copy_hover_and_the_crossfade_conform_under_the_three_tiers, crates/lune-ui-core/tests/widget_code_block.rs::the_text_node_is_read_only_multiline_text_under_the_label, crates/lune-ui-core/tests/widget_code_block.rs::the_status_node_is_polite_and_there_only_while_the_check_shows`; page `code-block` (12 scenes); unverified here: the OS clipboard the copy reaches (`platform.clipboard`).

## Open questions

Answered in M9 (D10 of the plan, D196):

- Wane's size, padding and border against Lunegit's: parameters of `CodeBlockOptions`. The defaults are Lunegit's (`size` 11, `line` 14, `pad` (10, 10), no ring); Wane's block is `size: 13.0, line: 20.0, pad: (12.0, 10.0), border: true`, drawn by the scene `code-block-wane`.
- The check mark's time: `feedback_ms`, default 1,500. The check shows for exactly that long after the click (the frame at the end sees it gone); a second copy restarts it; a change of text ends it. The check mark is `fg_primary`.
- The Wane operator tone: the app's choice. The page colours the mock's env block with `fg_secondary` keys, `fg_quaternary` operators (a token since D58) and `fg_primary` values.

Built as the entry's contract, with these notes:

- The body is the code view's frame, read-only, without a gutter, scrolling only sideways: its vertical extent is its viewport's, so the vertical wheel always goes to the page around it and the horizontal one only when a line overflows (D178).
- `pad` is horizontal then vertical. With `border` the padding starts inside the ring, so the block's height is the lines, twice the padding and twice the ring; `code_block::height` returns it. An empty text is one empty line, so the copy button fits.
- `tab_stop: false` keeps the text off the Tab order; a press still focuses it. The copy button is always a stop, after the text.
- The keyboard-focus ring of the text goes round the whole block, not the padded text.
- A span list is by 0-based line, `(line, span)`; the highlighter given to the frame filters it, so lines that do not exist are never asked for.

## Notes for the planner

Named by Wane's UI mocks (P0, screens components, first-run, routes, settings, [W docs/ui/components.md:667]); Lunegit's Markdown preview is the second consumer (`text.markdown-preview` reuses this block for fenced code).

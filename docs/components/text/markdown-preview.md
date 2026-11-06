---
id: text.markdown-preview
title: Markdown preview
area: text
status: done
consumers: lunegit
source: crates/lune-ui-core/src/widgets/markdown.rs
owner: —
summary: Renders an app-supplied Markdown block model: headings, paragraphs with inline marks, lists with task boxes, quotes, tables, code, images; links raise an event
---

## What it is

A scrolling, read-only rendering of a Markdown document the app has already parsed into a block
model (D13): the toolkit never parses Markdown and never loads files or the network. It draws
headings, paragraphs with bold, italic, strike, inline code and links, bullet, numbered and task
lists, block quotes, GFM tables, fenced code (through `text.code-block`), standalone images, rules
and inert HTML. Clicking a link raises an event; the app opens it (D11). Lunegit builds the model
from Markdig into its own immutable block records and draws them in `MarkdownPreviewView`; it is
shown for the file viewer and for scratchpad notes.

## Needs

- Rich text layout (R-text-3, R-layout-1).
- Images: app-decoded RGBA drawn by the `image` primitive (R-core-9).
- Fenced code uses `text.code-block` (R-text-3); tables scroll horizontally (R-layout-2).

## Existing code

Built in M9 step 7: `crates/lune-ui-core/src/widgets/markdown.rs` (`Ui::markdown`, `MdBlock`, `MdInline`, `MdOptions`, `is_actionable`), tests `crates/lune-ui-core/tests/widget_markdown.rs`. Design sources (not code to move): Lunegit `src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml` (323 lines) and `.axaml.cs` (484), `Markdown/MarkdownBlocks.cs` (the block model, 97 lines),
`Markdown/MarkdownInlineRenderer.cs` (inline marks and links), `Markdown/MarkdownTableRenderer.cs`, `Markdown/IMarkdownPreviewHost.cs` (what the view asks of its host), `Markdown/MarkdownTaskToggler.cs` (app-side write-back).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:1-323, Views/MarkdownPreviewView.axaml.cs:386-484, Markdown/MarkdownBlocks.cs:1-97, Markdown/IMarkdownPreviewHost.cs:1-66,
Markdown/MarkdownInlineRenderer.cs:1-330, Markdown/MarkdownTableRenderer.cs:1-118, Markdown/MarkdownTaskToggler.cs:1-47, Converters/MarkdownHeadingFontSizeConverter.cs:13-35, Converters/MarkdownHeadingBrushConverter.cs:10-30,
Themes/DarkTheme.axaml:132-141, src/Lunegit.Infrastructure/Localization/en.json:857-860. Wane has no Markdown component. No conflicts among sources.

### Anatomy

- Document model, supplied by the app: blocks are heading (level 1 to 6, inlines), paragraph, code (text, optional language), mermaid source, display math, quote (blocks), list (ordered flag, items with a display marker, blocks and an optional task state), table (rows, column alignments), image (target, alt), rule and HTML (raw); inlines are text (bold, italic, strike), inline code, link (label, URL) and inline math [L src/Lunegit.Desktop/Markdown/MarkdownBlocks.cs:11-97]. The toolkit's model mirrors it.
- Layout: a scrolling column with 12 px margin and 8 px between blocks [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:134-140].
- Paragraph: 13 px, wrapped, `fg_primary` [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:57-61].
- Heading: bold, wrapped, margin 0, 8, 0, 4; H1 22, H2 18, H3 14, H4 to H6 13 (never below body); colour `fg_primary` for H1 to H4, `fg_secondary` for H5, `fg_tertiary` for H6 [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:62-67], [L src/Lunegit.Desktop/Converters/MarkdownHeadingFontSizeConverter.cs:17-26], [L src/Lunegit.Desktop/Converters/MarkdownHeadingBrushConverter.cs:10-23].
- Inline marks: bold weight, italic style, strikethrough [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:150-156]; inline code as a pill: mono, fill `#14FFFFFF`, radius 4, padding 5 horizontal, aligned to the text baseline [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:158-199], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:134].
- Links: colour `#61AFEF`, underlined only when actionable [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:242-247], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:140]. Actionable means an absolute http or https URL or a relative path; in-page anchors (`#...`) and other schemes (mailto, file) are inert [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:254-261].
- Rule: 1 px high, `#20FFFFFF`, margin 6 above and below [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:156-158], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:141].
- HTML block: never rendered as HTML; shown as inert mono text on the code fill with padding 8 [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:160-164], [L src/Lunegit.Desktop/Markdown/MarkdownBlocks.cs:81-82].
- Code block: `text.code-block` (fill `#0AFFFFFF`, radius 6, padding 10, copy button) [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:176-187], [text.code-block].
- Mermaid and display math: drawn as source in a code block with a one-line failure note ("Diagram could not be rendered", "Formula could not be rendered") unless the host supplies a rendered image for the block [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:194-234], [L src/Lunegit.Desktop/Markdown/IMarkdownPreviewHost.cs:37-49]. The toolkit renders neither (no diagram or maths engine, D13).
- Quote: a 3 px left bar `#61AFEF`, fill `#0AFFFFFF`, padding 10 horizontal and 6 vertical, radius 0, 4, 4, 0, blocks spaced 6 [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:236-247], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:136-137].
- List: items spaced 4; each item is a marker column (auto width, margin right 6, top-aligned) and the item's blocks spaced 4; the marker ("•" or "1.") is 13 px `fg_secondary` in the UI font, not mono, so its baseline matches the first text line [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:249-296].
- Task item: a 15 px check box replaces the marker, shown checked or not from the model; it is a display of the model, a click sends the intent to the host and the model value is re-asserted [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:53,121-130,260-262,285-288], [L src/Lunegit.Desktop/Markdown/IMarkdownPreviewHost.cs:58-65].
- Table: a 1 px `#20FFFFFF` border with radius 4, clipped; horizontal scroll only (a vertical wheel still scrolls the document); columns size to their widest cell (auto), cells padding 8 horizontal and 4 vertical, minimum column width 80, wrapped text; the header row fill `#10FFFFFF`; every row has a 1 px bottom rule; column alignment left, centre or right from the delimiter row; a trailing star column lets a narrow table's header fill and rules reach the right edge [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:81-96,306-318], [L src/Lunegit.Desktop/Markdown/MarkdownTableRenderer.cs:47-80], [L src/Lunegit.Desktop/Markdown/MarkdownBlocks.cs:52-72], [L src/Lunegit.Desktop/Themes/DarkTheme.axaml:138-139].
- Image: a block image shown uniformly scaled, at most 640 px wide, left-aligned; below it (or instead of it) a placeholder line in 11 px `fg_tertiary` [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:166-174]. An absolute http or https target is never loaded: the placeholder says "Remote image not loaded: <target>" [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:411-418]. Any other target asks the host for bytes; no bytes or a decode failure shows the placeholder with the alt text or the target [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:420-452], [L src/Lunegit.Desktop/Markdown/IMarkdownPreviewHost.cs:51-53]. The toolkit takes the decoded RGBA from the app (D11, `core.image`).
- Selection: text selects within one block; Lunegit uses one selectable text block per paragraph and heading, so a selection does not span blocks [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:145,152,162,179].
- Large documents: Lunegit's block list is not virtualized and hidden blocks stay attached [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:463-473]; the toolkit draws only blocks in view (D41).
- Styling values are Lunegit's dark preset resources, not `Theme` fields today (Open questions).

### States

| state | look | cite |
|---|---|---|
| document rest | blocks as above | [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:134-140] |
| link, actionable | `#61AFEF`, underlined, hand cursor on hover | [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:244-246,298-302] |
| link, inert | `#61AFEF`, no underline, no cursor change | [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:244-245,254-261] |
| task item, unchecked and checked | the 15 px check box from the model | [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:285-288] |
| task item, read-only host | the box renders correctly but a click writes nothing | [L src/Lunegit.Desktop/Markdown/IMarkdownPreviewHost.cs:58-61] |
| image, loaded | the image, uniformly scaled | [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:445-447] |
| image, remote blocked | placeholder "Remote image not loaded: <target>" | [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:413-417] |
| image, failed or unresolved | placeholder with the alt text or the target | [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:436-452] |
| mermaid, math, unrendered | source in a code block with a failure note | [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:194-234] |
| empty | no blocks (the app shows its empty state) | [L src/Lunegit.Desktop/Markdown/IMarkdownPreviewHost.cs:29-31] |
| hover, pressed, focus-visible | links and the task box: see Input and focus; blocks have none | [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:298-302] |
| disabled | not applicable | [derived: no source] |

### Transitions

n/a — the preview is document content: no change in it animates (a document reload replaces blocks at once) [L .claude/docs/animation-rules.md:148]. Lunegit's styles carry no transition for links, images or blocks. The task check box animates as `widget.checkbox` does, not here.

### Input and focus

- Pointer: wheel and trackpad scroll the document; a table scrolls horizontally on its own and passes the vertical wheel on [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml:309-313]; press on an actionable link raises `link_clicked(url)` and consumes the press so no selection drag starts; the hand cursor shows over an actionable link [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:284-302]. Press and drag elsewhere selects text inside one block [text.text-edit].
- Opening the link is the app's: an http(s) URL opens the browser, a relative path jumps to that repo file [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:10-14], [D11].
- Task box click: `task_toggle_requested(item)` to the host; a read-only host does nothing [L src/Lunegit.Desktop/Markdown/IMarkdownPreviewHost.cs:58-65]. The write-back refuses unless the source at the marker is exactly `[ ]`, `[x]` or `[X]` and matches the expected state, else it returns nothing and the host reloads [L src/Lunegit.Desktop/Markdown/MarkdownTaskToggler.cs:7-12,28-44]; that function is app code.
- Copy button on code blocks: see `text.code-block`.
- Keys: scrolling keys (PageUp, PageDown, Home, End) through the scroll area; select-all and copy on a focused block [layout.scroll-area], [text.text-edit]. Keyboard reach of links: none in Lunegit (they are inline runs hit-tested by position); see Open questions.
- Tab stops: the code blocks' copy buttons, task boxes and actionable links in the blocks in view; a keyboard focus move brings its target into view (D200). Layer: the host's. No focus scope.

### Retained state

- Transient: link hover, selection while dragging, image load state [core.ui].
- Persistent: the scroll offset of the document and of each table, kept under `ui.keep_alive` for a hidden tab (Lunegit keeps blocks attached across loads [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:463-473]) [core.ui], [layout.scroll-area]. A new `identity` is another document: the offset, the measured heights and everything its blocks kept (a selection, the focus, a table's or code block's sideways offset) go, and going back to a key finds none of it (D202).
- App-owned: the block model, images (decoded RGBA by target), whether task boxes are live, the link handler.
- Exit retention: no.

### Accessibility

- The document is a `group` with the file name as name; a heading is a `static-text` node that carries its level (`role-gap: heading level`, Open questions); paragraphs `static-text`; code `multiline-text` read-only; lists `list` and `list-item`; a task item `checkbox` with `checked`; a table `table` with `row`, `cell` and `column-header`; an image `image` with the alt text as name; an actionable link `link` with the URL as description; a quote `group` [infra.accessibility].
- Actions: link default raises the click; task box default requests the toggle; image: none.

### Strings

- The copy button of a fenced code block reuses the code block's copy key, defined once in [text.code-block]; this entry adds no key for it.
- `luneui.markdown.image-remote-blocked`: "Remote image not loaded" [L src/Lunegit.Infrastructure/Localization/en.json:858].
- `luneui.markdown.mermaid-failed`: "Diagram could not be rendered — showing source" [L src/Lunegit.Infrastructure/Localization/en.json:859].
- `luneui.markdown.math-failed`: "Formula could not be rendered — showing source" [L src/Lunegit.Infrastructure/Localization/en.json:860].

### Gallery scenes

- `markdown-headings`: H1 to H6, settled.
- `markdown-inline`: bold, italic, strike, inline code, an actionable link and an inert link in one paragraph, settled.
- `markdown-link-hover`: pointer over the actionable link (hand cursor requested), settled.
- `markdown-lists`: bullets, a numbered list, a nested list and task items checked and unchecked, settled.
- `markdown-quote`: a quote with a nested quote, settled.
- `markdown-table`: a table with left, centre and right columns, wider than the view, settled; and `markdown-table-narrow` narrower than the view.
- `markdown-code`: a fenced code block (see `code-block-highlighted`), settled.
- `markdown-image`: a resolved image wider than 640 px, settled.
- `markdown-image-remote`: the remote placeholder, settled.
- `markdown-image-failed`: an unresolved image with its alt text, settled.
- `markdown-mermaid-fallback`: a mermaid block as source with the failure note, settled.
- `markdown-html-inert`: an HTML block as inert text, settled.
- `markdown-rule`: a thematic break between two paragraphs, settled.
- `markdown-readonly-tasks`: task boxes on a read-only host, settled.
- `markdown-link-focus`: the first link with keyboard focus: its ring, settled (D197).

### Painter primitives

- Existing: fill-radii, ring (table border), fill (rules, bars, fills), text, icon, clip-rect, xform (scroll offset).
- New: image (the block image, drawn uniformly scaled; Lunegit shows a bitmap at `MarkdownPreviewView.axaml.cs:445` [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:445], and the viewer and the empty repository view draw images too [L src/Lunegit.Desktop/Views/FileViewerView.axaml:352]).

### Acceptance

- Link rules (known answers from `IsActionable`): `https://x.dev` is actionable; `docs/a.md` is actionable; `#section` is not; `mailto:a@b.c` is not; an empty URL is not [L src/Lunegit.Desktop/Markdown/MarkdownInlineRenderer.cs:254-261]. A press on an actionable link emits one `link_clicked` with that URL and no selection starts; a press elsewhere starts a selection.
- Heading sizes: levels 1 to 6 give 22, 18, 14, 13, 13, 13; colours primary, primary, primary, primary, secondary, tertiary [L src/Lunegit.Desktop/Converters/MarkdownHeadingFontSizeConverter.cs:17-26], [L src/Lunegit.Desktop/Converters/MarkdownHeadingBrushConverter.cs:17-23].
- Tables: a three-column table with alignments left, centre, right right-aligns the third column's cells; every column is at least 80 px wide; a table narrower than the view still has its header fill and rules to the right edge.
- Images: an `https://` target draws the remote placeholder and asks the host for nothing; a relative target asks the host; none or a failed decode draws the placeholder; the image is at most 640 px wide and keeps its aspect ratio [L src/Lunegit.Desktop/Views/MarkdownPreviewView.axaml.cs:411-452].
- Tasks (app-side reference): toggling `[ ]` at the marker offset of `- [ ] a` with the expected state unchecked gives `- [x] a`; an expected state that does not match, an offset out of range or a slice that is not `[ ]`, `[x]` or `[X]` gives nothing [L src/Lunegit.Desktop/Markdown/MarkdownTaskToggler.cs:19-44]. The widget never changes its model itself.
- HTML: the raw string is drawn as text, never interpreted.
- Pixel probes at scale 2: block gap 16 device px, margin 24, quote bar 6 px wide, rule 2 px high, table border 2 px.
- Conformance: none, no transitions.
- Semantic node: the document `group`, a `link` node per actionable link, a `checkbox` per task item; an injected `AccessAction` default on a link raises the click.
- Tests, bullet by bullet (`crates/lune-ui-core/tests/widget_markdown.rs`). Links: `is_actionable_gives_the_entrys_known_answers`, `a_press_on_an_actionable_link_raises_it_once_and_starts_no_selection`, `a_press_beside_the_link_selects_text_and_raises_nothing`, `an_inert_link_is_plain_coloured_text_with_no_region_node_or_cursor`, `enter_and_space_on_the_focused_link_raise_it`, `the_link_node_has_the_label_for_name_the_url_for_description_and_a_default_action`. Headings: `headings_take_their_size_and_colour_by_level_and_clamp` (22, 18, 14, 13, 13, 13; primary to tertiary), `a_heading_is_a_static_text_node_with_its_level`. Tables: `a_table_has_a_border_a_header_fill_a_rule_per_row_and_right_aligned_cells`, `a_column_is_as_wide_as_its_widest_cell_and_80_at_least`, `the_table_nodes_are_a_table_of_rows_with_column_headers_and_cells`. Images: `a_resolved_image_is_at_most_640_wide_and_keeps_its_aspect`, `an_unresolved_or_failed_image_shows_its_alt_or_its_target`, `a_remote_target_is_never_drawn_even_with_a_picture_and_says_so`. Tasks (the widget never changes its model; the toggle write-back is the app's): `a_task_box_shows_the_model_and_a_click_reports_its_id_when_live`, `task_boxes_on_a_read_only_host_report_nothing_and_still_draw`, `space_on_the_focused_task_box_reports_it`. HTML: `html_is_drawn_as_inert_text_on_the_code_fill`. Pixel probes at scale 2: `the_column_has_a_12_margin_and_8_between_blocks_at_scale_2` (24 and 16 device px), `a_quote_has_a_bar_3_wide_a_fill_and_nests`, `a_rule_is_1_high_in_surface_active_with_6_above_and_below`. Node: `the_document_is_a_group_named_by_its_label`, `lists_draw_markers_in_fg_secondary_and_nest_with_list_and_item_nodes`. Scale: `a_frame_costs_the_same_at_1_000_and_10_000_blocks_and_rests`, `only_the_blocks_in_view_are_built`.

### Build

- Milestone: M9. Built in M9 step 7: `Ui::markdown`, `MdBlock`, `MdInline`, `MdItem`, `MdTask`, `MdRow`, `MdAlign`, `MdImage`, `MdOptions`, `MdResponse`, `is_actionable`, 16 scenes on the page `markdown-preview`.
- Depends on: text.code-block, text.text-edit, layout.scroll-area, widget.scrollbar, widget.checkbox, core.image, core.ui, core.focus, infra.accessibility
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_markdown.rs::is_actionable_gives_the_entrys_known_answers, crates/lune-ui-core/tests/widget_markdown.rs::a_press_on_an_actionable_link_raises_it_once_and_starts_no_selection, crates/lune-ui-core/tests/widget_markdown.rs::headings_take_their_size_and_colour_by_level_and_clamp, crates/lune-ui-core/tests/widget_markdown.rs::a_table_has_a_border_a_header_fill_a_rule_per_row_and_right_aligned_cells, crates/lune-ui-core/tests/widget_markdown.rs::a_resolved_image_is_at_most_640_wide_and_keeps_its_aspect, crates/lune-ui-core/tests/widget_markdown.rs::a_remote_target_is_never_drawn_even_with_a_picture_and_says_so, crates/lune-ui-core/tests/widget_markdown.rs::a_task_box_shows_the_model_and_a_click_reports_its_id_when_live, crates/lune-ui-core/tests/widget_markdown.rs::html_is_drawn_as_inert_text_on_the_code_fill, crates/lune-ui-core/tests/widget_markdown.rs::the_column_has_a_12_margin_and_8_between_blocks_at_scale_2, crates/lune-ui-core/tests/widget_markdown.rs::the_link_node_has_the_label_for_name_the_url_for_description_and_a_default_action`; page `markdown-preview` (16 scenes); unverified here: the OS clipboard and the app's link handling.

## Open questions

Answered in M9 (D12 of the plan, D197):

- Keyboard reach of links: an actionable link is a Tab stop with `Role::Link`, Enter and Space raise it. One region per row box of the link, the first is the stop; a press on a link raises it once and starts no selection.
- `role-gap: heading level`: a heading is a `StaticText` node with `level` set, no new role.
- Selection across blocks: no. A selection stays inside one block, as Lunegit's.
- The Markdown colours: tokens. `link` (links, quote bar), `code_inline` (inline code pill), and the rest are existing tokens of equal value: `surface_input` (code, quote and HTML fill), `surface_hover` (table header), `surface_active` (rule, table border and row rules).
- A host-rendered Mermaid or maths image: the model's `rendered` field takes a picture the app drew. No engine ships.

Built as the entry's contract, with these notes:

- The model is borrowed data (`&[MdBlock]`); the toolkit draws it and interprets none of it. A level outside 1 to 6 clamps, a picture with no size is its placeholder, an `http` or `https` image target is never drawn, even with a picture supplied.
- The top-level blocks are the virtual rows: an estimate of 48 until built, then the measured height, and a block above the first in view that changes height moves the area by as much. A width change keeps the heights: what is built re-measures, so the reader's place stays (D201). The block that holds the focus stays built outside the window, off the Tab order, so the Page keys keep working after they scroll it away (D200).
- Inline code is a run padded with two U+2009 either side (in the UI face), which draws the pill's padding. The pads are left out of Copy and of the node by offset, so a U+2009 in the source survives. Underline, strike and the pill are fills from the text's row boxes. Their places (D198): the underline `(row height * 0.87).round()` below the row top, the strike `(row height * 0.6).round()`, the pill from 1 px under the row top to the row's bottom (Lunegit's zero bottom padding); a focused link's ring is `RingKind::Offset { radius: 3 }`.
- The marker column of a list is as wide as its widest marker, so the items' text lines up (Lunegit sizes each item's own).
- A task box is the checkbox's look at 15 px, drawn from the model with no transition of its own. Click or Space reports `task` when `tasks_live`; the box keeps the model's value until the app changes it. Off, it draws and reports nothing.
- A table's cells do not wrap: a column is its widest cell plus 16, 80 at least, and the table scrolls sideways alone; the header fill and the rules reach the table's width when it is narrower than the view.
- Mermaid and maths without a picture show the failure note above the source block, not inside it (the code block holds only code).
- The document is a `Group`; a quote a `Group`, a list a `List` with `ListItem`s, a table a `Table` with `Row`s and `Cell`s or `ColumnHeader`s.

## Notes for the planner

Replaces the stub's questions: Lunegit renders headings, paragraphs, lists (with GFM tasks), quotes, tables, images (local only), code with highlighting, and shows Mermaid and maths as source unless a host renders them. Parsing stays in the app (D13).

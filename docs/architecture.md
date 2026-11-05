# Architecture

State after M9 (text views): the toolkit is five crates. A pure core (`lune-ui-core`, no
dependencies) holds the painter, the display list, the per-window `Ui` frame, the motion system,
the string table, the semantic tree, widgets, the theme and the four seam types; three adapter
crates implement the seams on wgpu/glyphon, winit and the OS glass; the facade `lune-ui` re-exports
all of them under the module paths the toolkit has always had (D9, D24 to D38). The code moved from
Luneterm 296cfdc's `crates/lune-ui` in M1 (D10), was cut along its seams in M2, and gained the `Ui`,
motion, strings and accessibility data in M4 (D59 to D80), and in M5 gained the shared pieces every later component
reuses: layout helpers, the scroll area and virtualized list, popover and tooltip, the text editing engine with IME,
in-window drag and drop, the shortcut registry, seven painter primitives and the `ImageStore` seam (D91 to D109). M6 added the
controls and chrome (D113 to D136); M7 the overlays and containers: a sixth layer for an overlay above a dialog, the
flyout, toasts, the command palette, the page, panel, sidebar, side panel, icon rail, split panes, tree view and settings
page (D137 to D154). M8 added the data views: the table on the list frame, the bar and pie charts with value
transitions, the lane graph row and the kanban board with its drag across columns (D160 to D174). M9 added the text views: a
code editor over a line-indexed document (`CodeDoc`, one editing engine generic over the text it edits), the code block, the markdown
preview on virtual blocks and the diff view, unified and split on the list frame (D177 to D199).

## Workspace layout

| Path | What |
|---|---|
| `crates/lune-ui-core` | Pure core: painter, `Scene`, `Ui` frame, motion, strings, semantic tree, widgets, theme, `TextSystem`, `RawInput`/`FrameOutput`, `Backdrop`, fakes. No dependencies |
| `crates/lune-ui-wgpu` | wgpu renderer and glyphon text: one shared `Gpu` (which is also the `ImageStore`), per-target `FrameRenderer`, `WindowSurface`, dumps, `TextCache` |
| `crates/lune-ui-winit` | winit adapter: input translation, output application, glass window attributes |
| `crates/lune-ui-platform` | OS glass (`Material`) per platform; the only crate with `unsafe` |
| `crates/lune-ui` | Facade: `lib.rs` only, re-exports the four crates at the old paths |
| `apps/gallery` | A page registry (legacy Painter-API pages and `Ui` pages), a glass window, `--dump <dir>` (offscreen PNG frames on a headless device, no window), `--list`, `--bench` (frame-time probes). App code, outside `crates/` (D12): it bundles fonts |
| `tools/dump-compare` | Compares two dirs of PNG frame dumps pixel by pixel (the pixel gate's compare tool) |
| `scripts/` | Gates and generators, see "Boundary rules" and "Pixel gates" |
| `docs/components/` | Catalogue, one file per component; `docs/components/INDEX.md` is generated |
| `licenses/`, `THIRD_PARTY_NOTICES.md` | Licence texts and the per-asset notice rows |

Scripts: `scripts/check-boundaries.sh` (rules 1 to 12, see below; fixed-string exceptions in
`scripts/boundaries.allow`), `scripts/gen-component-index.sh` (catalogue index; `--check` is in the
gate), `scripts/pixel-gate.sh`, `scripts/pixel-gate-luneterm.sh`, `scripts/check-targets.sh`.

## Crates and modules

The app owns the winit event loop; no crate starts one or reads the clock or the environment (the
app passes time, sizes, fonts, theme, scale and hit values in), so frames are deterministic.

### `lune-ui-core` (`crates/lune-ui-core/src/`)

| Module (file) | Purpose | Key items |
|---|---|---|
| `color` (`color.rs`) | Straight-alpha colour and blend helpers | `Rgba = [u8; 4]`, `premul`, `over`, `with_alpha` |
| `theme` (`theme.rs`) | Theme tokens widgets read; the dark preset with its glass layers | `Theme`, `DARK`, `lunegit_dark::{THEME, WINDOW, PANEL, CARD, ...}` |
| `acrylic` (`acrylic.rs`) | Avalonia acrylic colour math and the noise opacity | `AcrylicColors`, `NOISE_OPACITY` |
| `paint` (`paint.rs`, `paint/path.rs`) | Immediate-mode painter: six layers, clip and rounded clip (`push_clip_rounded`), a transform stack (D66) and `push_translate`, the seven M5 primitives (image, rotate, dash, rounded clip, sector, path fill, arc; D105), hit regions generic over the app's hit type `H` and tagged with the overlay they belong to, an op recorder (`start_recording`, `take_recording`, replayed by exit painters), faded acrylic, `acrylic_radii` (one radius per corner) and `backdrop_with` (the glass's own layer, sigma and saturation; D137, D138), `stroke_sweep` and `stroke_comet` over `StrokeStyle.mode` (the busy cue's conic arc and dash, D122), `Painter::elide` (see `text`); builds a `Scene`; holds `&mut dyn TextSystem` | `Painter<H>`, `Rect`, `StrokeStyle`, `Xform`, `Ops` |
| `scene` (`scene.rs`) | The display list: plain data plus the draw-order contract | `Scene`, `Layer`, `LayerGeometry`, `Shape`, `Acrylic` (a shape and its opacity, D61), `StrokeVertex`, `TextItem`, `CustomItem`, `BackdropRequest` (rect, radius, alpha, `layer`, `sigma`, `saturation`), `ImageItem`, `RoundClip`, `LayerId` (Base, Page, Popup, Modal, Raised, Toast; D54, D137), `LAYER_COUNT` (6), `SCENE_LAYERS`, `Scene::split`, `FrameParams` |
| `text` (`text.rs`) | Text specs and the text seam, with the caret, hit, line and segment queries (D86, D94) | `TextSystem` (`prepare`, `measure`, `caret`, `hit`, `lines`, `segments`), `TextKey`, `Spec`, `Style`, `Font`, `CaretPos`, `LineSpan`, `Unit`, `elide` (D114: one line cut on a grapheme boundary, U+2026 in the last kept run's style, found by binary search over `caret`; a spec that fits comes back unchanged) |
| `input` (`input.rs`) | The input seam: what the app hands in, what the toolkit asks back | `RawInput`, `Event` (with `Access`, `ClipboardText`, `ClipboardUnavailable`), `Key`, `NamedKey`, `Modifiers`, `ImeEvent`, `FocusEvent`, `Keymap` (`MAC`, `PC`; D92), `FrameOutput` (with `repaint`, `clipboard`, `tree`, `focused`, `cursor`, `ime_area`, `fired`), `Repaint`, `ClipboardRequest`, `WindowCommand`, `CursorIcon` (with `ALL`), `printable` |
| `image` (`image.rs`) | The picture seam: the app decodes, registers by id, releases explicitly; fit geometry | `ImageStore`, `ImageData`, `ImageId`, `ImageError`, `Fit`, `ImageAlign`, `image_rect` |
| `layout` (`layout.rs`) | Pure stack, row, column and grid helpers: rects out, nothing drawn or stored (D108) | `stack`, `row`, `column`, `grid`, `Size`, `Align`, `Insets`, `Track` |
| `shortcuts` (`shortcuts.rs`) | The scoped chord registry, resolved by the frame (D93) | `Shortcuts`, `Binding`, `Chord`, `Scope`, `CommandId`, `Chord::hint` |
| `text_edit` (`text_edit.rs`) | The pure text editing engine: caret, selection, undo, clipboard commands, preedit; offsets in chars (D95) | `TextEdit` (`takes`), `EditCommand`, `EditOutcome`, `EditFlags`, `EditLayout` (`lines_near`), `SpecLayout`; since M9 the engine edits through `EditText` and `EditBuffer`, implemented for `str`, `String` and `CodeDoc` (D180) |
| `code_doc` (`code_doc.rs`) | The code editor's document (D179): one `String` and a line index, verbatim, with `\n`, `\r\n` and a lone `\r` ending a line; an edit costs O(log lines) | `CodeDoc` (`new`, `text`, `set_text`, `line_count`, `line`, `line_of`), implements `EditText` and `EditBuffer` |
| `drag` (`drag.rs`) | The in-window drag machines and their `Ui` glue (D89, D104) | `ReorderDrag`, `FreeDrag` (with `pressed`), `Metrics`, `Strip`, `Column`, `DropResult`, `FreeResult`, `paint_drop_line`, `edge_step` (the auto-scroll zone math, shared by the strip and the board; D168) |
| `backdrop` (`backdrop.rs`) | The material seam | `Backdrop`, `Material`, `BackdropKind`, `GlassLayer` |
| `window` (`window.rs`) | Edge resize, double click, caption buttons (no windowing type) | `ResizeEdge`, `edge_at`, `DoubleClick`, `caption_buttons` |
| `icon` (`icon.rs`) | Codicon codepoints as Nerd Font glyphs; no font file bundled | `glyph`, `ADD`, `GEAR`, `CHECK`, ... |
| `ui` (`ui.rs`, `ui/{access,animate,focus,id,keys,overlay,pointer,response,store}.rs`) | The per-window frame (see "The `Ui` frame" below) | `Ui`, `UiStore`, `FrameConfig`, `WidgetId`, `Response`, `Sense`, `OverlayOptions`, `OverlayFrame`, `OverlayPhase`, `Dismissal` |
| `access` (`access.rs`) | The semantic tree and the actions that come back; plain data the `Ui` fills and routes | `Node`, `SemanticTree`, `Role` (36), `State` (15), `Action` (10), `Value`, `AccessAction`; `Node.level` (a tree row's depth + 1) |
| `motion` (`motion.rs`, `motion/{easing,tween,transition,tokens,conformance}.rs`) | Closed-form motion: every value is a function of the app's `Time` and stored parameters; the 60 Hz conformance harness | `Time`, `Easing`, `Tween`, `Spring`, `Keyframes`, `Transition`, `Token`, `Kind`, `Motion`, `MotionTier`, `MOTION`, `conformance` |
| `strings` (`strings.rs`) | The string table: keys, `{n}` and `{n:spec}` placeholders, the toolkit's defaults | `StringTable`, `DEFAULTS`, `ArgFormat`, `DisplayFormat`, `format_template` |
| `widgets` (`widgets/<name>.rs`, `widgets/mod.rs`) | One file per control, generic over `H` for the Painter API; each has a `Look`, a `paint` function, the old function as a wrapper and a `Ui` method (D60) | `button`, `toggle`, `checkbox`, `label`, `combo`, `stepper`, `field` (text field with its variants), `text_area`, `input_box` (shared look), `focus_ring`, `popup_panel`, `scroll_area`, `scrollbar`, `virtual_list`, `popover`, `tooltip`; M6 added `badge`, `status_dot`, `verdict_row`, `progress`, `spinner`, `callout`, `stat`, `kv_list` (with `selectable`, the read-only selectable text of D125), `row`, `list_row`, `form_field`, `steps`, `nav_item` (`Ui::nav_list`), `radio_group`, `star_rating`, `slider`, `filter_bar`, `search_picker`, `tab_strip`, `toolbar`, `banner` (with the empty state) and `tone` (the one tone recipe, D129); `button` gained `Ghost`, `ButtonOptions` (`Ui::button_with`) and `IconButton` (`Ui::icon_button`, D116); M7 added `flyout` (`Ui::flyout`), `toast` (the `Toasts` model, `Ui::toasts`, `Ui::toast_history`), `command_palette` (`Ui::command_palette`), `page` and `panel` (`Ui::page`, `Ui::panel`: one closure called per slot), `sidebar`, `side_panel` and `icon_rail` (`Ui::sidebar`, `Ui::side_panel`, `Ui::icon_rail`), `split_panes` (`Ui::split_panes`, `Ui::split_state`), `tree_view` (`Ui::tree_view`, `Ui::tree_state`) and `settings_page` (`Ui::settings_page`); M8 added `table` (`Ui::table`, `Ui::table_state`, `Ui::table_selection`: the list frame with a header, sort requests and tones; `virtual_list` gained a crate-private `ListStyle`, D161), `bar_chart` (`Ui::bar_chart`), `pie_chart` (`Ui::pie_chart`), `lane_graph` (`Ui::lane_graph_row`, a stateless `paint`) and `kanban_board` (`Ui::kanban_board`, `landed_index`); M9 added `text_lines` (`HighlightSpan`, `Highlighter`, `PlainHighlighter`, the rich-line spec and `range_boxes` across runs), `code_editor` (`Ui::code_editor`, `Ui::code_line_rect`: lines on `RowHeights` and the scroll area, read-only or editable, wrapped or not, synced panes), `code_block` (`Ui::code_block`, `code_block::height`: the same frame read-only with a copy button), `markdown` (`Ui::markdown` over the `MdBlock` model: top-level blocks as virtual rows) and `diff_view` (`Ui::diff_view`, `pair_rows`, `widest_chars`: the list frame, twice for split) |
| `widgets::menu`, `dropdown`, `dialog` | The overlays: the old Painter functions and `Ui::menu`, `Ui::dropdown`, `Ui::dialog` on the overlay stack; M7 added `menu_with` (`MenuOptions`, `MenuPlace`: anchored menus and the node name; rows `MenuRow::Entry` and `Header`; one highlight), `dropdown_with` (`DropdownOptions`: glass, `max_rows`), `dialog_with` (`DialogOptions`: `DialogWidth::{Confirm, Form}`, scrim blur, disabled confirm) and `dialog::modal_host`, which the palette shares | `menu`, `MenuRow`, `MenuItem`, `menu_open_anim`; `dropdown`; `dialog`, `Dialog`, `DialogGlass`, `Cancel` |
| `chrome` (`chrome.rs`) | Tab chrome outline geometry | `outline`, `TabSpan` |
| `testing` (`testing/`) | Fakes and contract suites of the seams, public so apps can use them | `RecordingRenderer`, `FakeText`, `text_system_contract`, `FakeImages`, `image_store_contract`, `ScriptedInput`, `NullMaterial`, `backdrop_contract`, `UiHarness` |

`lib.rs` re-exports `Backdrop`, `BackdropKind`, `GlassLayer` and `theme::lunegit_dark`. Tests: unit
tests in the modules and the files under `tests/` (`window`, `painter`, `motion`, `strings`, `ui_*`,
one `widget_*` file per control, and `c4`, the cross-widget contract suite, with a section for the M6 set, one for the M7 set, one for the M8 set (the table, both charts, a lane graph row and the board in one frame) and one for the M9 set (the block, the editor, the document and the diff in one frame, unified and split): roles, names and states of every interactive widget in one frame, accessibility actions, one tab stop per composite, the disabled walk, idle at rest, and the overlays over a dialog); none needs a
GPU or a window.

### The `Ui` frame

A `Ui` is built per window and frame from the window's `UiStore` (the state that outlives a frame),
the frame's `RawInput`, a `FrameConfig` (theme, motion tier, string table, `ArgFormat`) and a
`TextSystem`. `Ui::finish` returns the `Scene` and a `FrameOutput` whose `repaint` says when to draw
again (`Idle`, `Now`, or `At(time)`), with the cursor, the input-method area, clipboard requests, semantic
tree, focused widget and the shortcut commands that fired. `FrameConfig` also carries the `Keymap` and the
`Shortcuts` table (D92, D93).

- Ids: `WidgetId` from app keys under scopes (`ui.id`, `ui.scope`); the same key path is the same id
  from frame to frame.
- Hits resolve against the previous frame's regions (the Painter's rule), with this frame's events in order.
  Regions from `ui.interact` and from raw `Painter::hit` calls share one layered list, so widgets of
  both APIs mix in a frame; `painter.hover` is the hovered id (D61). A press captures the pointer
  until release.
- State: `UiStore` keeps typed entries per `(WidgetId, TypeId)` in ordered maps (no random hasher,
  D62). Transient state lives while the widget is drawn; persistent state lives while the widget is
  drawn or a keep-alive scope on its chain is live (`keep_alive`, `mark_alive`, `end_keep_alive`,
  D63). Animated values (`animate_f32`, `animate_color`) are a retargetable `Transition` per id,
  resolved through the theme's motion tokens and the frame's tier.
- Overlays (`Ui::overlay`, used by `menu`, `dropdown`, `dialog`): a stack in open order, each on the
  layer it names, with its own focus scope and a phase (Opening, Open, Closing) whose progress is
  linear and mapped to the look by the overlay (D64). A modal overlay captures hover, press, scroll,
  keys and actions for everything below; Escape and an outside press dismiss only the top overlay
  that allows it, and a light-dismiss press is consumed (D77). The Ui never closes an overlay: the app
  stops calling it. Each called frame an overlay hands the Ui an exit painter built from owned inputs
  (with the app's content recorded as painter ops), and the Ui keeps drawing it through the close
  motion (D65).
- The `Raised` layer (D137): `Ui::overlay` chooses an overlay's layer once, when it opens or reopens, and holds it to
  its close. A `Popup` or `Modal` overlay that opens while a modal is active is promoted to `LayerId::Raised`
  (between `Modal` and `Toast`), so a popover, tooltip, combo list, search picker, menu or nested dialog over a dialog
  draws and takes input on top; no widget asks for it. The rule is one level deep: overlays stacked further share
  `Raised`, and a layer draws by kind, so a third level that overlaps its parent interleaves with it (fills under
  text). A toast is not an overlay record: `Ui::toasts` draws on `Toast` directly and a press on a `Toast` hit is
  never an outside press, moves no focus, and an accessibility action on one dismisses no overlay and takes no focus
  out of a modal (D146). `Ui::overlay` also pauses any enclosing `record`, so a dialog's exit replay does not
  draw the overlays opened in its body (D139).
- Disabled scope (D140): `Ui::node` inside a disabled scope (`with_enabled(false, ..)`) sets `State::Disabled` and
  clears `Focusable` and the actions, so a group node cannot be forgotten; a widget consumes its keys first and gates
  after (`Ui::activation_keys`), so a disabled widget keeps its keys from the shortcuts. Not every reader does yet:
  "Disabled keys" below is the one list.
  `testing::disabled_violations(tree, root)` walks a subtree for a node that broke the rule.
- Focus: scopes with a tab trap, initial focus, and restore on close (D72); keys are consumed by id
  (`consume_key`); a focused widget that is not registered for a frame loses focus unless a live
  keep-alive scope covers it.
- Roving focus (D113): `Sense.skip_tab` marks a focusable region that is not a tab stop; `Sense::CLICK_ROVE` is
  that and a click. A composite with one stop (the radio group, star rating, nav list, tab strip) gives
  `Ui::rove_stop(items, active)` its children, `(id, enabled)` in order, and the index of the current
  selection: the stop is the focused child, else the active child if enabled, else the first enabled one;
  that child registers with `Sense::CLICK_FOCUS` and the others with `CLICK_ROVE`. `Ui::rove_keys(items,
  RoveAxis, wrap)` moves focus with the arrows and returns the new index; `Ui::rove_focus(id)` focuses a
  child and keeps the ring. `tab_stops` also lists the focused child, so Tab from a child that stopped
  being the stop continues in call order, and `FocusState.roves` holds each composite's children so
  Tab skips the others. The stop is derived each frame; nothing stores a cursor.
- Focus input (D91): the frame keeps the keys (named and character, with modifiers and repeat), typed text,
  IME events and clipboard answers as one ordered list; `ui.focus_events(id)` hands the focused widget its
  entries in arrival order and `ui.consume_focus_event(i)` takes one, so a widget sees Backspace then
  "x" in the order they came, and what no widget took is left for the shortcuts. `consume_key` reads the
  same entries; `consume_key_while_pressed` serves a drag's Escape. `Response` also reports the press
  (`press_started`, `press_pos`, `press_count`, `press_modifiers`); `ui.set_cursor` and `ui.set_ime_area` fill the
  frame's output.
- Keymap (D92): `Keymap::MAC` and `Keymap::PC` say which modifier is primary, which moves by word and
  which way a word motion lands; the app picks one (the gallery's scripted scenes use `MAC`).
- Shortcut phases (D93): the preempt phase runs at begin over the top input-taking overlay's scope and
  the global scope, before Escape and Tab take the key; the bubble phase runs at finish over the focused
  widget's scope outwards, the overlays top-down down to the modal, then global, once every widget had its
  chance. One binding fires per press, in table order; a fired command makes the frame `Repaint::Now`
  and lands in `FrameOutput::fired`.
- Scrolling and lists (D96 to D100): `Ui::scroll_area` keeps `f64` offsets per id, wheel and trackpad
  motion, scroll-into-view (`Reveal::Center` or `Nearest`), follow-tail and the overlay scrollbar;
  `Ui::virtual_list` builds only the rows in view on an untranslated area, with keyed selection.
- Overlays above the page (D101, D102): the pure `place` computes a rect (side, alignment, gap, margin,
  flip, shift, overflow) for the popover, the tooltip, the dropdown and the menu; `Ui::popover` is a
  non-modal Popup overlay with light dismiss, `Ui::tooltip` shows after 800 ms of hover or on focus.
- Text editing (D87, D88, D103): `text_field_with` and `text_area` sync a `TextEdit` kept in the store,
  map the focus events to `EditCommand`s, and draw the selection, the preedit and the caret from the
  engine's display string; `text_edit` holds no frame state of its own.
- Drag and drop (D89, D104): `Ui::drag_gesture` and `free_drag_gesture` feed the machines from a `Response`;
  `drag_offset` and `drag_dim` animate the items; the app applies a `Commit` or a `Drop`.
- Accessibility: widgets emit `Node`s (`ui.node`, `ui.node_container`); the tree of the previous frame
  routes `AccessAction`s as pointer or key input (D71). `FrameOutput::tree` carries the tree out.
- Strings: `ui.tr(key)` and `ui.tr_fmt(key, args)` read the table, then the toolkit default, then the
  key (D70).

Frames stay deterministic: time, input, theme, tier and strings all come from the caller.

### Disabled keys

The one list of which key readers hold D140's rule (D159). STATE, `CLAUDE.md` and DECISIONS point here and restate
none of it. The rule: a widget that holds the key focus when its scope turns disabled (`with_enabled(false, ..)`) takes
every key it takes when enabled and acts on none, so none reaches a shortcut. Focus stays on it. A widget disabled before
it had focus gets no key, so a test that never focuses it proves nothing here.

How it was made, at 406f780. The readers are every call in `crates/lune-ui-core/src` of `consume_key`,
`consume_plain_key`, `consume_key_while_pressed`, `focus_events`, `activation_keys` and `rove_keys`; no other library crate
calls them, and Tab and Escape are the frame's, at begin, so they are not in the table. Each reader then ran one probe in a
scratch copy: focus it, disable it, press one of 27 keys with one binding on that key (14 named, 7 with Shift, a letter,
Command with A, C, V, X, Z), against the same run enabled. A key that fires nothing enabled is a key the reader takes;
a Shift form counts as its key below. Each run needs F5, which nothing takes, to fire: a `Global` binding is never
offered under a modal, so it cannot fail there, and the dialog and the palette run on `Scope::Overlay`.

Re-run for M8 (the same call patterns plus `consume_synthetic_key` and `consume_focus_event`, over `crates/lune-ui-core/src`
at the M8 tip, the search first run on a known-absent name to see it report nothing): against the M7 tip, `table.rs` (two
calls) and `kanban_board.rs` (four) are new, and `combo.rs` went from three calls to one (D160). `bar_chart.rs`, `pie_chart.rs`
and `lane_graph.rs` have none. The new rows below come from those calls, each with a test and a mutant, not from reading.

Re-run for M9 (the same call patterns over `crates/lune-ui-core/src` at the M9 tip, per-file counts by `rg -c` against a
`git archive` of the M8 tip 2474248; the search first run on a known-absent name, `consume_zzq_absent_key`, which printed 0
lines, and on `rove_keys`, which printed its files): `field.rs` went from 5 calls to 6 (`run_events`, D177) and `markdown.rs` from
none to 2 (`activation_keys` in the link regions, Enter and Space, and in the task box, Space). `code_editor.rs`, `code_block.rs`
and `diff_view.rs` call none: the editor and the block's text run on `begin_edit_with`, so on `run_events`; the copy buttons and
the hunk actions are hosts of `icon_button`; the diff's panes are `virtual_list`'s (`list_keys`). The rows below come from
`c4::m9`'s page with a `Global` binding on 12 keys, each stop focused, then disabled, with a mutant each. The widget files'
own disabled tests for these (`widget_markdown`, `widget_code_editor`, `widget_diff_view`) bind no shortcut on the key, so none of
them fails under any of those mutants: they hold the walk and the reports, not the keys.

A new reader, or a change to one, gets its row here. **Test** means a named test fails when that reader is made to leak
(a mutant of it, run at 406f780). **Probe** means only the scratch probe showed it: nothing in the repo holds it.

Holds, for the keys listed:

| Reader | Keys | Evidence |
|---|---|---|
| `button`, `icon_button`, `toolbar_label` | Enter, Space | Test: `widget_button::a_disabled_widget_keeps_the_keys_it_handles_from_the_shortcuts` |
| `checkbox`, `toggle` | Space | Test: the same |
| `radio_group` | Space, arrows, Home, End | Test: the same |
| Hosts of `Ui::button` or `icon_button`: `dialog`'s confirm and cancel, `toolbar_action`, the filter bar's clear, the tab strip's new tab | Enter, Space | Probe. `widget_dialog`'s `enter_and_space_on_a_disabled_confirm_...` binds `Global`, so it cannot fail (review F19) |
| `star_rating` | Enter, Space, arrows, Home, End | Probe; no test failed under a mutant of its keys or of its `rove_keys` |
| `nav_list`, so `sidebar` | Enter, Space, Up, Down, Home, End | Test: `widget_sidebar::a_focused_row_turned_disabled_keeps_its_keys_and_leaves_its_chords` (Enter, Space), `c4::m7::a_disabled_page_takes_every_key_a_focused_composite_takes_and_fires_no_global_binding` (the rest) |
| `icon_rail` | Enter, Space, Up, Down, Home, End | Test: `c4`, the same |
| `split_panes` | arrows, Home, End | Test: `c4`, the same, and `widget_split_panes::a_strip_that_was_focused_and_is_then_disabled_still_takes_its_keys_and_acts_on_none` |
| `tab_strip`, its own keys (see `scroll_area` for PageUp and PageDown) | Enter, Space, Left, Right, Home, End | Test: `widget_tab_strip::a_disabled_strip_activates_nothing_from_enter_or_space_and_still_does_after_enabling`, `a_disabled_strip_keeps_the_arrows_home_and_end_from_the_shortcuts` |
| `menu` | Enter, Space, Up, Down, Home, End | Test: `widget_menu::a_disabled_scope_takes_the_keys_picks_nothing_and_marks_every_node` |
| `flyout`, its rows (`menu_rows`) | Enter, Space, Up, Down, Home, End | Test: `widget_flyout::a_disabled_flyout_closes_on_escape_picks_nothing_and_marks_every_node` |
| `dropdown`, with and without `max_rows` | Enter, Up, Down | Test: `widget_dropdown::a_disabled_dropdown_takes_the_keys_picks_nothing_and_marks_every_node` |
| `tree_view` | Enter, Space, arrows, Home, End, PageUp, PageDown | Test: `widget_tree_view::a_disabled_tree_toggles_and_selects_nothing_takes_its_keys_and_passes_the_walk` |
| `toasts`, the action buttons | Enter, Space | Probe (`toast.rs:954`: no test failed under a mutant) |
| `toasts`, Cancel and Dismiss (`icon_button`, the `button` row's reader) | Enter, Space | Test: `widget_toast::the_toast_is_called_in_a_disabled_scope_and_every_node_says_so` holds Cancel; probe (Dismiss) |
| `settings_page`, the restart action | Enter, Space | Probe. `widget_settings_page::a_disabled_page_reports_nothing_and_leaves_the_keys_to_shortcuts` never focuses it |
| `command_palette`, its own keys | Up, Down, Enter | Test: `widget_command_palette::a_disabled_palette_takes_its_keys_activates_nothing_and_passes_the_walk` (`Scope::Overlay`) |
| `combo` | Enter, Space, Down | Test (D160): `widget_combo::a_combo_focused_then_disabled_keeps_its_keys_from_the_shortcuts`; a chord on them stays a shortcut, enabled and disabled: `a_chord_on_the_combos_keys_is_a_shortcut_and_opens_nothing` |
| `virtual_list`, so the list frame the table runs on | Up, Down, Home, End, PageUp, PageDown; Command+C with a selection | Test (D160): `widget_virtual_list::a_list_focused_then_disabled_takes_its_keys_and_changes_nothing`, `command_c_on_a_list_focused_then_disabled_copies_nothing_and_fires_no_binding`; chords: `a_chord_on_the_lists_keys_stays_a_shortcut` |
| `scroll_area`, a focused child the keys passed (including one in a disabled scope) | PageUp, PageDown, Home, End, in an area that overflows. Hosts: the page body and the settings content | Test (D160): `widget_scroll_area::a_focused_child_that_is_disabled_keeps_the_page_keys_from_the_shortcuts_and_scrolls_nothing`, `a_disabled_focused_child_leaves_f5_and_the_page_key_chords_to_the_shortcuts`, `nested_areas_take_a_page_key_of_a_disabled_child_once` |
| `table`, the header (`activation_keys` and `rove_keys`) | Left, Right, Home, End, Enter, Space | Test (M8): `widget_table::a_header_focused_then_disabled_takes_its_keys_and_changes_nothing`, `c4::m8::a_disabled_page_takes_every_key_a_focused_composite_takes_and_fires_no_global_binding` (header stop); mutants: the Enter and Space reader gated before it consumes and `rove_keys` skipped when disabled (both fail the widget test), the readers made `consume_key` (fails the chord test), `activation_keys` leaving a disabled key (fails the c4 test). Chords: `a_chord_on_the_tables_keys_stays_a_shortcut` |
| `table`, the body, through `virtual_list` (the row above) | Up, Down, Home, End, PageUp, PageDown | Test (M8): `widget_table::a_body_focused_then_disabled_takes_its_keys_and_changes_nothing`, the same c4 test (body stop); mutant: `virtual_list` consuming only when enabled fails the c4 test (`table body: ArrowUp reached ...`) |
| `kanban_board`, the cards (one roving stop) | Up, Down, Left, Right, Home, End, each also with Shift, Enter, Space | Test (M8): `widget_kanban_board::a_focused_card_turned_disabled_keeps_its_keys_and_acts_on_none`, the same c4 test (card stop). The arrow loop's own gate is masked by the `rove_keys` that follows it, which takes the same keys, so the mutant gates both: it fails both tests. Chords: `a_chord_on_a_card_stays_a_shortcut_enabled_and_disabled` |
| `kanban_board`, the column area, PageUp and PageDown (`scroll_area`) | PageUp, PageDown | Test (M8): `widget_kanban_board::page_keys_scroll_a_column_and_die_disabled` |
| Hosts of `Ui::icon_button`: the board's add column, add card and column actions | Enter, Space | Test (M8): `c4::m8::the_boards_buttons_focused_then_disabled_keep_enter_and_space_from_the_shortcuts`; mutant: `activation_keys` leaving a disabled key fails it |
| `text_field`, and the filter bar's box (`begin_edit`, M9 D177) | Enter, Space, Backspace, Delete, Left, Right, Home, End, a typed character, Command+A, V, Z; Command+C, X with a selection | Test (M9): `widget_text_field::disabled_keys::a_text_field_focused_then_disabled_takes_its_keys_and_changes_nothing`, `a_filter_bar_focused_then_disabled_...`, `a_key_the_engine_declines_still_fires_its_binding`, `a_clipboard_answer_in_a_disabled_frame_does_not_insert`; mutants: `run_events` gated on `focused` again, and acting while disabled (both fail the three box tests). `TextEdit::takes` is `apply(..).consumed` for every command and state (`text_edit::takes_is_what_apply_consumes_for_every_command_and_state`, mutant: Enter always taken) |
| `text_area` (`begin_edit`) | those, and Up, Down, PageUp, PageDown | Test (M9): `widget_text_field::disabled_keys::a_text_area_focused_then_disabled_takes_its_keys_and_changes_nothing`, the same mutants |
| `command_palette`, its search box (a filter bar) | Space, Backspace, Delete, Left, Right, Home, End, a typed character, Command+A, V, Z; Command+C, X with a selection (Up, Down, Enter by the palette's own keys, above) | Test (M9): `widget_command_palette::a_search_box_focused_then_disabled_takes_its_keys_and_changes_nothing` (`Scope::Overlay`); mutant: `run_events` gated on `focused` fails it |
| `selectable_text`, so `kv_list` values and the flyout's routes (each run), positioned and elided | Enter, Left, Right, Home, End, Command+A; Command+C with a selection. Elided text (`spec` differs from `full`): Command+A, C | Test (M9): `widget_kv_list::a_value_focused_then_disabled_takes_its_keys_and_changes_nothing`, `select_all_on_an_elided_value_focused_then_disabled_selects_nothing`; mutants: the whole-selection loop gated on `focused`, Select all acting while disabled, the positioned selection cleared while disabled (each fails one of the two) |
| `code_editor` editable, and the code block's text (read-only), through `begin_edit_with` (`run_events`) | Editable: Up, Down, Left, Right, Home, End, PageUp, PageDown, Enter, Command+A (typed text arrives as `Text`). Read-only: Command+A (the arrows are free, D181; its Page keys and Home and End are the area's, the `scroll_area` row) | Test (M9): `c4::m9::a_disabled_page_takes_every_key_a_focused_composite_takes_and_fires_no_global_binding` (the editor and block-text stops); mutant: a disabled box not consuming what `takes` says it takes leaks `code block text: Command+A`, `editor: Command+A` and the editor's named keys |
| `markdown`, the links (`activation_keys`) | Enter, Space | Test (M9): the same c4 test (link stop); mutant: the reader gated on `enabled` leaks `markdown link` |
| `markdown`, the task boxes (`activation_keys`) | Space | Test (M9): the same (task stop); mutant: the reader gated on `enabled` leaks `markdown task box` |
| Hosts of `Ui::icon_button`: the copy buttons of the code block and of markdown's fenced code, the diff's hunk actions | Enter, Space | Test (M9): the same (copy and hunk stops); mutant: `activation_keys` leaving a disabled key leaks `code block copy`, `diff hunk action`, `markdown link` and `markdown task box` |
| `diff_view`, the unified pane and each split pane, through `virtual_list` | Up, Down, Home, End, PageUp, PageDown | Test (M9): the same (body, left and right stops); mutant: `virtual_list` consuming only when enabled leaks all three. Command+C with a selection is the `virtual_list` row above, run on a list, not on a diff pane |

Does not hold: each key listed fired nothing enabled and fired its binding disabled, with focus kept. All by probe;
nothing acted. Each gates on `enabled` or a focus that is false in a disabled scope before it takes the key.

| Reader | Keys that reach the shortcut |
|---|---|
| `stepper` | Up, Down |
| `slider` | arrows |
| `search_picker`, the trigger | Enter, Space. An open picker closes when disabled and focus returns to the trigger, so this row is what is left of its box |

Not a leak: Escape in a pointer drag (`consume_key_while_pressed`: the splitter, `drag_gesture`) fired nothing enabled
and fired its binding once the scope turned disabled mid-drag (probe, splitter and tab strip). From reading: `Ui::interact`
reports no press in a disabled scope, so the drag has ended and Escape has nothing to cancel, which `drag.rs` leaves to
the shortcuts by design. A drag inside an overlay (a dialog body) is a different case, fixed in M8 (D175): `Ui::begin` used to turn
Escape into the overlay's dismissal and never queue it, so the drag could not take it. The three readers of
`consume_key_while_pressed` (`drag_gesture`, `free_drag_gesture`, the splitter) now call `Ui::note_live_gesture` while they drag,
and the next frame's Escape is queued for them while they hold the capture at the key (D176); once the drag ends the next Escape
dismisses the overlay.

One item disabled in a live group holds too, by probe: a nav row, a rail button, a menu row, a `toolbar_action`, the
dialog's disabled confirm. A disabled radio option takes its keys, and the arrows move on to a live sibling.

Not checked, from reading only:

- `Ui::empty_state`'s and `Ui::callout`'s actions (`Ui::button`), not run. `Ui::toast_history`'s clear (`icon_button`) holds Enter and Space by probe, with no test.
- The tab strip's menu (`menu`), not run.
- `free_drag_gesture`: no library caller, not run.
- Keys outside the 27, such as Alt word moves in a text field and Control chords. D157 leaves chords to the shortcuts where it applies.
- A closure host (`dialog` body, `popover`, `panel`, `page`, `side_panel`, `form_field`, `setting_row`, and a table's cell closure) reads no key: what it
  hosts has the row of that widget. The bar chart, pie chart and lane graph row read no key at all.

### Draw order

Inside one layer, back to front (the contract in `scene.rs`): the acrylic shapes, the under shapes, the
path fills (their own list, `LayerGeometry::fills`, drawn with the stroke pipeline so the order is
explicit, D105), the custom items in record order, the images in record order (`Layer::images`), the
text, the over shapes and the strokes. Layers run back to front, six of them (`Base`, `Page`, `Popup`, `Modal`,
`Raised`, `Toast`); those below `Scene::split` render into the offscreen scene a backdrop blurs. The split moves: it
is `SCENE_LAYERS` (2), or the backdrop request's own layer when that is higher, so a glass surface blurs everything
under it (a glass menu in a dialog blurs the dialog; D138). A frame has one blur: of two requests the lower layer
keeps it, and on one layer the last wins, so a glass menu over a dialog whose scrim is blurred has none of its own. Consequences a widget author meets: text draws above every fill
of its layer (a label shows through a fill drawn "over" it on the same layer), and a rounded clip
(`push_clip_rounded`) cuts shapes, strokes, fills and images but gives text and custom items only its
bounding rect.

### Motion

`docs/motion.md` holds the vocabulary: five duration tokens, seven easings, 15 kinds. A widget row
names a `Motion` (kind, token, easing, Reduced rule); `MotionTokens::resolve` turns it into a
duration under the frame's tier (`Full` runs it, `Reduced` maps the token or fades without
transform, `Off` snaps, D69). `Token::Zero` is a row that never runs, in any tier. A retarget while
in flight keeps position and velocity (D68). `conformance` samples a motion at 60 Hz and checks
monotone steps, the exact end and that it finishes within its duration plus one frame.

### `lune-ui-wgpu` (`crates/lune-ui-wgpu/src/`)

| Module (file) | Purpose | Key items |
|---|---|---|
| `gpu` (`gpu.rs`, `shader.wgsl`) | The GPU every target shares: device, queue, noise texture, per colour format the pipelines and the glyph atlas; headless-capable | `Gpu`, `instance`, `Uploaded`, `Globals` |
| `frame` (`frame.rs`) | One target's renderer: the scene, blur, target pass sequence over a core `Scene` | `FrameRenderer`, `FrameParams` |
| `images` (`images.rs`, `image.wgsl`) | The image pipeline and the picture store: premultiply on upload, nearest at an integer ratio without rotation, else linear; rotated text is a raster of the shaped text drawn through it (D105, D106) | `Gpu::register`, `Gpu::release` (the `ImageStore` impl) |
| `blur` (`blur.rs`, `blur.wgsl`) | Backdrop blur | `Blur` |
| `surface` (`surface.rs`) | A window's swapchain; no winit dependency | `WindowSurface`, `Acquired` |
| `dump` (`dump.rs`) | Offscreen targets and GPU readback to PNG; no screen capture | `target`, `read_rgba`, `write_png` |
| `custom` (`custom.rs`) | App-owned GPU drawing inside a painted layer (the hook for app-specific drawing, D8) | `CustomDraw`, `CustomFrame`, `CustomItem` |
| `text_cache` (`text_cache.rs`) | glyphon shaping behind the core's `TextSystem`, including the caret, hit, line and segment queries, which read one row list per shaped text (`unicode-segmentation`, D86; D112) | `TextCache` |
| `platform` (private; `platform/{windows,other}.rs`) | The Windows DX12 backend choice | `configure` |

Asset: `assets/acrylic-noise.png` (Avalonia's texture). Tests: `tests/text.rs` (text contract),
`tests/targets.rs` (two targets on one `Gpu`), `tests/images.rs` (the image contract on the `Gpu`),
`tests/primitives.rs` and `tests/text_field.rs` (pixel probes on real fonts).

### `lune-ui-winit` (`crates/lune-ui-winit/src/`)

| Module (file) | Purpose | Key items |
|---|---|---|
| `input` (`input.rs`) | winit events to `RawInput`; `FrameOutput` onto a window | `Translator`, `OutputState`, `cursor_icon` |
| `window` (`window.rs`) | Glass window attributes, `ResizeEdge` to winit's direction | `glass_attributes`, `resize_direction` |
| `platform` (private; `platform/{macos,windows,other}.rs`) | Per-platform window attributes | `glass` |

Tests: `tests/input.rs` (translator against `ScriptedInput`, key, button, cursor and edge mappings).

### `lune-ui-platform` (`crates/lune-ui-platform/src/`)

| Module (file) | Purpose | Key items |
|---|---|---|
| `material` (`material.rs`, `material/{macos,windows,other}.rs`) | OS backdrop per platform; the only `unsafe` is `material/macos.rs` | `apply`, `WindowBackdrop`, `Material` (re-export) |

### `lune-ui` (facade, `crates/lune-ui/src/lib.rs`)

One `pub use` per module keeps every `lune_ui::<module>` path. Three modules merge a core and an
adapter part: `text` (core text plus `TextCache`), `window` (core window helpers plus the winit
attributes), `input` (core model plus the winit translation). The facade also re-exports `testing`.
Boundary rule 11 keeps it free of items.

## Seams

Each seam is a core model (data or trait) that adapters implement, a fake in `lune_ui_core::testing`
and a contract test the real adapter passes too (R-nfr-8).

| Seam | Core model | Adapter | Fake | Contract test |
|---|---|---|---|---|
| Display list (renderer) | `scene::Scene` data and its draw-order contract; no renderer trait (D25) | `lune-ui-wgpu`: `Gpu::upload`, `FrameRenderer::draw` | `RecordingRenderer` | `records_the_documented_draw_order` (`testing/recording.rs`); two-target pixel equality in `lune-ui-wgpu/tests/targets.rs` |
| Text shaping | `text::TextSystem` (`prepare(spec, scale)`, `measure`, `caret`, `hit`, `lines`, `segments`) | `lune-ui-wgpu`: `TextCache` over glyphon | `FakeText` | `text_system_contract`, run in `lune-ui-wgpu/tests/text.rs` and on the fake |
| Pictures | `image::ImageStore` (`register`, `release`); the app decodes | `lune-ui-wgpu`: `Gpu` | `FakeImages` | `image_store_contract` (`testing/images.rs`), run on the fake and in `lune-ui-wgpu/tests/images.rs` |
| Input and output | `input::{RawInput, FrameOutput}` | `lune-ui-winit`: `Translator`, `OutputState` | `ScriptedInput` | `lune-ui-winit/tests/input.rs` (translator and scripted input agree) |
| OS glass | `backdrop::Backdrop` (`best_supported`, `apply`) | `lune-ui-platform`: `WindowBackdrop` over `apply` | `NullMaterial` | `backdrop_contract`, run on `NullMaterial`; OS glass is not run headless |
| Windowing | the core's window helpers and `FrameOutput::commands`; the app owns the event loop | `lune-ui-winit` | `ScriptedInput` | as input |

## Dependencies

Locked versions (`Cargo.lock`, unchanged since M1 and identical to Luneterm 296cfdc's; DECISIONS
D15 to D23 and D38 record each). Each crate's allowed set is enforced by boundary rule 9.

| Crate | Version | Used by | For |
|---|---|---|---|
| `lune-ui-core` | path | every other crate | none: it declares no dependency |
| `winit` | 0.30.13 | lune-ui-winit, lune-ui-platform, gallery | windowing |
| `wgpu` | 30.0.1 | lune-ui-wgpu, gallery | GPU renderer |
| `glyphon` | 0.12.0 (with `cosmic-text` 0.19.0) | lune-ui-wgpu | text shaping and rendering |
| `bytemuck` | 1.25.2 | lune-ui-wgpu | casting vertex data |
| `png` | 0.18.1 | lune-ui-wgpu, dump-compare | frame dumps, compare |
| `window-vibrancy` | 0.8.1 (macOS, Windows) | lune-ui-platform | OS glass |
| `objc2-app-kit` | 0.3.2 (macOS) | lune-ui-platform | NSView access for glass |
| `raw-window-handle` | 0.6.2 (macOS) | lune-ui-platform | native window handle |
| `pollster` | 0.4.0 | gallery; lune-ui-wgpu (dev) | block on async `Gpu::new` |

`window-vibrancy` also pulls `objc2` 0.5.3 and `objc2-app-kit` 0.2.2 beside `objc2` 0.6.5 and
`objc2-app-kit` 0.3.2. The facade `lune-ui` depends on the four crates only. Bundled assets and their
licences: `THIRD_PARTY_NOTICES.md`.

## Multi-window model

One `Gpu` and one `TextCache` serve the whole app; each window adds its own state:

- per app: `Gpu` (device, queue, noise texture, per-format pipelines and glyph atlas) and `TextCache`
  (text scale is a per-call argument, so windows at different scales keep their entries side by
  side instead of re-shaping everything on each scale change, D27);
- per window: a `WindowSurface` (swapchain), a `FrameRenderer` (everything a draw writes: globals,
  viewport, text renderers, blur textures, so two targets never share a write that lands at the next
  submit, D28), a `Painter` building a `Scene` and its hit list each frame, a `Translator`
  (`RawInput` in) and an `OutputState` (`FrameOutput` out);
- per headless target: a `FrameRenderer` and an offscreen texture from `dump::target`; no window.

Call `Gpu::end_frame` once per app frame, after every window drew; calling it per window only ages
text entries faster. `acquire` returns `Acquired::Redraw` instead of asking a window to redraw, and
the app calls `pre_present_notify` before `WindowSurface::present` (D29). Window-mode code (surfaces,
present, output commands, OS glass) is compile- and review-checked in M2; no gate opens a visible
window.

## Extension points

| Point | Mechanism |
|---|---|
| OS glass | A new `material` submodule (`crates/lune-ui-platform/src/material/`) behind `Backdrop` and `apply` |
| Another renderer | Consume `Scene` in the documented draw order; test the painter side with `RecordingRenderer` |
| Another text shaper | Implement `TextSystem`; pass `text_system_contract` |
| Another windowing library | Translate its events into `RawInput`, apply `FrameOutput`; script it with `ScriptedInput` |
| Widget | A file under `crates/lune-ui-core/src/widgets/`: a public `Look` with `of(theme, state)`, a `paint` function, the Painter-API function as a wrapper, a `Ui` method that interacts, animates through `Motion` rows and emits its node; a catalogue file; a gallery page; a `tests/widget_<name>.rs` with the rest-identity test (settled `Scene` equals the Painter function's) |
| Overlay | `Ui::overlay` with `OverlayOptions` (layer, modal, dismissal, anchor, initial focus, open and close `Motion`) and an exit painter; `Ui::menu` and `Ui::dialog` are the worked examples |
| Gallery page | A file under `apps/gallery/src/pages/` implementing `PageState`, its `UiScene`s (timed scripted input, then a capture frame, with the `Repaint` it must report) and one line in `apps/gallery/src/pages/mod.rs` |
| Test locale | A `StringTable` returned by `PageState::strings` (the gallery's `locale` page wraps every default in brackets) |
| Look | A `Theme` value handed to the `Painter`; `theme::DARK` and `lunegit_dark` are one |
| App-specific drawing | `CustomDraw`, one implementation per slot passed to `FrameRenderer::draw_with`; the gallery's `apps/gallery/src/custom.rs` is the worked example. The terminal-grid renderer built on it stays in Luneterm (D8) |
| Fonts | The app passes font bytes and family names to `TextCache::new` (or `with_locale`); the library bundles none and loads no system font (D5, D12, D78) |

## Gallery pages and bench

`apps/gallery` is app code (it reads the clock for the bench and bundles fonts). Every screen is a
`Page` in `apps/gallery/src/pages/mod.rs`: the legacy pages (`reference`, `effects`) hold the 15 fixed Painter-API
scenes the pixel gate compares and keep their drawing code; every other page draws through the `Ui`
(D73). A Ui page has a live state (window mode, the bench), scripted `UiScene`s (timed input, then a
capture frame, with the `Repaint` it must report: `Settled` or `Moving`) and optional bench steps.
`-states` scenes are specimen sheets painted through the widgets' public `Look` and `paint`
functions; mid-transition and interaction scenes are live scripted `Ui` frames. Pages: `button`,
`toggle`, `checkbox`, `label`, `stepper`, `combo`, `text_field`, `dialog`, `dropdown`, `menu`, `motion`,
`ui`, `focus`, `a11y`, `locale`, and from M5 `stack`, `image`, `shortcuts`, `primitives`, `scroll_area`,
`scrollbar`, `virtual_list`, `popover`, `tooltip`, `text_edit`, `ime` and `drag_drop`, and from M6 `badge`, `status_dot`,
`verdict_row`, `progress`, `spinner`, `callout`, `stat`, `kv_list`, `row`, `list_row`, `form_field`, `steps`,
`nav_item`, `radio_group`, `star_rating`, `slider`, `filter_bar`, `search_picker`, `tab_strip`, `toolbar` and `banner`, and from M7
`flyout`, `toast-notifications`, `command-palette`, `page`, `panel`, `sidebar`, `side-panel`, `icon-rail`, `split-panes`,
`tree-view` and `settings-page`, from M8 `table`, `bar-chart`, `pie-chart`, `kanban-board` and `lane-graph`, and from M9 `code-block`,
`code-editor-view`, `markdown-preview` and `diff-view`
(70 pages in all, 389 scene names; every page of the same pattern: a live state, scripted scenes, a bench timeline); `apps/gallery/src/pages/controls_ui.rs` is the shared composition of every M4 widget that the
core pages draw. Window mode builds a `Ui` per frame over the page's live state; dump and bench
drive the same state through a `Player` (a scripted input, the retained store and the page state). A scene's `Expect` is `Settled` (`Idle`),
`Moving` (`Now`) or, for a pending tooltip, `Waiting` (`At`). `Ev` scripts pointer, scroll, typed text, keys
with modifiers, IME preedit and commit, and the clipboard's answer; the window answers a clipboard
request from an in-process string until M10 (D107). The bundled fonts have no CJK glyph, so the IME scenes'
"かな" draws as boxes (D90). A shown overlay fades from the first frame that draws it, so a scene that wants
it visible needs a frame at the due time. Window mode and the bench draw the list page with 1,000,000 rows, the code editor page with a 1,000,000-line document, the table, lane graph and diff pages with 100,000 rows or lines, and the markdown page with a document of 2,800 blocks (400 sections).

`gallery --bench [--runs N] [--frames N] [--size WxH]` times offscreen frames: a timeline per page,
cost probes (text fades, a scaling and fading dialog over a blurred stripe page) and a 4 ms-sleep
self-test that calibrates the instrument. Warm-up 3 frames, 5 runs by default, at 2080 x 1320
(1040 x 660 at 2x). Each run is judged on its own: p99 at most 16.6 ms and no frame over 33 ms; a
page is `OVER` only when at least half its runs are, and a minority of over runs is printed beside
its `ok`. The self-test's spread (p99 and worst above its p50) and the load average are printed with
every verdict; nothing refuses a busy machine (D85). It is the milestone acceptance run, not part of
the per-commit gate.

## Boundary rules

`scripts/check-boundaries.sh` runs these (output `rule N: <path>:<line>: <text>`; exit 1 on an
offender, 2 when a scanned path or a tool is missing):

| Rule | Checks | Backs |
|---|---|---|
| 1 | No `crates/*/Cargo.toml` names an app | hard rule 1 |
| 2, 3 | No file under `crates/` names luneterm, or app or terminal words (exceptions: `scripts/boundaries.allow`) | hard rule 1 |
| 4 | No `crates/*/src` file reads the clock or the environment | hard rule 2 |
| 5 | Every manifest inherits the workspace licence | hard rule 7 |
| 6 | `unsafe` only under `lune-ui-platform/src/platform/` or `src/material/`; every other crate's `lib.rs` has `#![forbid(unsafe_code)]` | hard rule 3 |
| 7 | `lune-ui-core` declares no dependency of any kind (as `cargo metadata --no-deps` reports them) | crate rule |
| 8 | No third-party crate path or `extern crate` in a file under `lune-ui-core` | crate rule |
| 9 | Every crate dir is registered with its allowed dependency names and is the workspace member of that name; every dependency cargo reports is within its lists; an unregistered dir, a rename, a path dependency outside `crates/<name>`, or a root `package =`, `[patch]` or `[replace]` fails; adapters name no `lune-ui-*` crate but the core | crate rule (adapters depend on the core only) |
| 10 | No OS predicate in `cfg` outside `platform/` or `material/` dirs, except `#[cfg]` on `mod x;` or `use`; a wrapped statement is joined (strings and comments skipped), one open after 40 lines fails | hard rule 3 |
| 11 | `crates/lune-ui/src` holds only `lib.rs`, with no item definitions | crate rule (facade only) |
| 12 | Compiled code comes only from the scanned files: build targets are `src/lib.rs` and its modules (`tests/` files are not library code and are not scanned); no `path = "..."` attribute or `include!` in `crates/*/src` (comment lines and `let path =` bindings are not attributes) | rules 6, 10, 11 (what they scan is what is built) |

The script does not cover hard rules 4 (a new dependency needs a DECISIONS row; rule 9 forces a
script edit but does not check the row), 5, 6 (licence rows), 8, 9 or 10 (`gen-component-index.sh
--check` covers 10). Hard rule 3 reads: platform code lives in the adapter crate that implements the
seam, in `platform/` or `material/` files, so the Windows DX12 choice is in
`lune-ui-wgpu/src/platform/`, the winit attributes in `lune-ui-winit/src/platform/` and the glass in
`lune-ui-platform/src/material/` (D32).

## Known deviations

The two M1 deviations are resolved: the inline `cfg(target_os)` in `gpu.rs` and `window.rs` moved
into `platform/` files (step 8), and `Gpu::new` takes an optional compatible surface, so a headless
device needs no window (steps 3 and 4). Carried limits at the end of M5, none of them fixed here: an
overlay above a modal is unsupported, so a popover or tooltip inside a dialog draws under it; the
menu's node name; text cannot be cut by a rounded clip (it gets the bounding rect); text draws above
fills, so overlapping labels show through during a drag; and nothing here verifies real typing with an
input method, bidi caret placement or the Windows and Linux run time.

## API changes since M1

For M11's migration map (Luneterm's `lune-termview` uses `color`, `custom`, `paint`, `dump`; the
facade keeps those paths):

- `Painter::new` takes `&mut dyn TextSystem` (was `&mut TextCache` plus a global scale); the painter
  has a `scene` field (the display list) instead of GPU geometry;
- `FrameRenderer::new(gpu, format)` and `FrameRenderer::draw(gpu, text, scene, target, params)` /
  `draw_with(..., customs)` (a renderer per target, a scene instead of a painter);
- `Gpu::new(instance, compatible_surface)` (no window; `None` is headless), `Gpu::end_frame(text)`;
- `WindowSurface` and `Acquired` replace the surface held inside `Gpu`;
- `TextCache::new(fonts, ui_family, mono_family)` without a scale; `set_scale` is gone; scale is an
  argument of `TextSystem::prepare` and `measure`; new `TextKey`;
- `edge_at` returns the core's `ResizeEdge` (was winit's `ResizeDirection`); `resize_direction`
  maps it for winit;
- `BackdropRequest` (core) is what a glass menu records; the blur's own uniform is private;
- `theme::DARK`, a neutral alias of `lunegit_dark::THEME`;
- new: `RawInput`, `FrameOutput`, `Translator`, `OutputState`, `Backdrop::apply`, `WindowBackdrop`,
  the `testing` module.

Since M9 (D177 to D199; nothing public was removed or changed in signature):

- new widgets with a `Ui` method each: `code_editor` (`Ui::code_editor`, `code_line_rect`; `CodeOptions`, `LineBand`), `code_block`
  (`Ui::code_block`, `code_block::height`; `CodeBlockOptions`), `markdown` (`Ui::markdown`, `is_actionable`; `MdBlock`, `MdInline`,
  `MdItem`, `MdTask`, `MdRow`, `MdAlign`, `MdImage`, `MdOptions`, `MdResponse`) and `diff_view` (`Ui::diff_view`, `diff_selected_text`,
  `pair_rows`, `widest_chars`; `DiffLine`, `LineKind`, `DiffMark`, `SplitRow`, `DiffMode`, `HunkAction`, `DiffOptions`,
  `DiffResponse`); `widgets::text_lines` (`HighlightSpan`, `Highlighter`, `PlainHighlighter`); core gains `code_doc::CodeDoc`;
- `Style.italic` and `Style::italic()` (D189, additive to C2): hashed in the text key, mapped to the face's style in
  `lune-ui-wgpu`, ignored by `FakeText`; no italic face ships, so the shaper skews;
- `TextEdit` is generic over the text: `EditText` and `EditBuffer`, implemented for `str`, `String` and `CodeDoc`;
  `EditLayout::lines_near` (provided) and `TextEdit::takes(&cmd)` are new (D180, D177);
- text readers consume then gate (D177): `begin_edit` and `selectable_text` take their keys in a disabled scope and act on none,
  so `text_field`, the filter bar's and the palette's box, `text_area` and the selectable text hold D140's rule;
- a wheel delta routes per axis (D178, C4): the topmost scroll region live on that axis takes it (`HitRecord.scroll_axes`),
  so a sideways-only code block or table inside a document no longer swallows the vertical wheel, and a sideways swipe over a
  vertical composite reaches the area around it (the M8 limit is closed);
- the list frame gains `RowCtx.scroll_x`, `ListResponse.pressed` and, crate-private, `ListStyle.fill`, `identity` and `link`
  (the pixel scroll link of two panes, D184); `selectable_text` selects by position whenever the joined spec equals the full
  text, multi-run too (D187);
- six `Theme` tokens: `link`, `code_inline`, `diff_added`, `diff_removed`, `diff_added_mark`, `diff_removed_mark` (D190); `icon`
  gains `REMOVE`; `strings` gains the `luneui.code-block.*`, `luneui.code-editor.*`, `luneui.markdown.*` and
  `luneui.diff-view.*` keys, among them `pane-old` and `pane-new`, the suffix of a split diff pane's table name (D199);
- no scene that existed before M9 changed: the pixel gate against the M8 tip reads 332 identical and 57 new.

Since M8 (D160 to D176; nothing public was removed or changed in signature):

- new widgets with a `Ui` method each: `table` (`Ui::table`, `table_state`, `table_selection`; `columns`, `Cell`,
  `CellTone`, `TableColumn`, `ColumnWidth`, `SortKey`), `bar_chart` (`Ui::bar_chart`, `bar_chart::height`),
  `pie_chart` (`Ui::pie_chart`, `pie_chart::sweeps`), `lane_graph` (`Ui::lane_graph_row`, `lane_graph::paint`, `width`,
  `edge_points`) and `kanban_board` (`Ui::kanban_board`, `kanban_board::landed_index`);
- `drag` gains `edge_step` (the strip's zone math moved out, `ReorderDrag` calls it) and `FreeDrag::pressed`;
  `free_drag_gesture` reports a press and its release in one frame as a click; `Ui::drag_ghost` and `ListStyle` are
  crate-private;
- `scroll_area` reads only synthetic keys in its line loop (a real arrow on a focused area no longer scrolls, F18) and
  serves a focused child in a disabled scope; `virtual_list` and the combo consume then gate (D160);
- `icon` gains `ARROW_UP`, `ARROW_DOWN`, `ACCOUNT` and `NOTE`; `strings` gains `luneui.bar-chart.name`,
  `luneui.pie-chart.name` and four `luneui.kanban-board.*` keys; no `Theme` token was added;
- Escape cancels a live pointer drag before the overlay around it dismisses (D175): `UiStore.gesture_live`, set through the
  crate-private `Ui::note_live_gesture` by `drag_gesture`, `free_drag_gesture` and the splitter while they drag, read and
  cleared at each `begin`, and an Escape counts for it only while the capture holds at the key (D176);
- a composite that nests an area and knows its extent disables an axis with nothing to scroll (`Bars::unless_fits`, D176), so it
  registers no scroll sense and the delta reaches the area around it: the board and its columns, `virtual_list`, the table,
  `tree_view`, `tab_strip`, `text_area`, `toast_history`. Not these, and why, in D176: `page`, `settings_page` (measured extent,
  made to be the outermost area; nested, one that fits keeps the delta), the dropdown list, the search picker's rows, the palette's results, the toast log (overlays). The proof is
  `tests/c4.rs` `nested_wheel` and the board's tests in `widget_kanban_board`;
- no scene that existed before M8 changed: the pixel gate against the M7 tip reads 305 identical and 27 new.

Since M7 (D137 to D154):

- `LayerId::Raised` is a sixth layer between `Modal` and `Toast` (`Toast` is 5 now, `LAYER_COUNT` 6, `SCENE_LAYERS`
  unchanged); code that indexes `Scene::layers` by number moves with it. `Scene::split()` is the one place the
  renderer and the fake read the offscreen split; `BackdropRequest` gains `layer`, `sigma` and `saturation` (0 means
  the frame's) and `Painter::backdrop_with` sets them; `Painter::acrylic_radii` cuts one corner;
- `Node` gains `level` (every `Node` literal says `level: None`); `Ui::node` marks a disabled scope's nodes;
  `Ui::icon_button_sensed`; `testing::disabled_violations`;
- `MenuRow::Entry(MenuItem)` and `MenuRow::Header` join `MenuRow::Item` (kept); `Ui::menu_with`, `dropdown_with`,
  `dialog_with` and `Surface::Glass` on the popover; `Ui::menu` calls `menu_with`;
- new widgets with a `Ui` method each: `flyout`, `toasts` and `toast_history`, `command_palette`, `page`, `panel`,
  `sidebar`, `side_panel`, `icon_rail`, `split_panes`, `tree_view`, `settings_page`; `stat` and `kv_list` gain the
  measures the flyout needs;
- `Theme` gains `menu_item_pressed`, `menu_text_disabled`, `toast_bg`, `toast_border`, `toast_info`, `toast_success`,
  `toast_warning`, `toast_error`, `toast_log_bg`, `toast_log_fg`, `dirty_indicator`, `count_badge` and `status_error`;
  `strings` gains `luneui.menu.name`, the `luneui.toast-notifications.*`, `luneui.command-palette.*` and
  `luneui.settings-page.*` keys; `icon` gains `CHEVRON_RIGHT`, `WARNING`, `ERROR`, `CLEAR_ALL` and `CIRCLE_SLASH`;
- `FilterOptions` gains `clear_unfocusable`; `Ui::activation_keys` and `Ui::nav_list` leave chords to the shortcuts (D157, D158);
- no scene that existed before M7 changed: each step passed the pixel gate at zero delta against its base, and the
  new scenes are the only ones the visual batches (steps 13 and 14) moved.

Since M6 (D113 to D136):

- `Sense` gains `skip_tab` and the constant `CLICK_ROVE`; `Ui` gains `rove_stop`, `rove_keys`, `rove_focus` (with
  `RoveAxis`) and `snap_f32` (a pointer-driven value that must not animate); `tab_stops` lists the focused
  child of a roving group;
- `text::elide` and `Painter::elide`; `Painter` gains `stroke_sweep` and `stroke_comet`; `ScrollOptions`
  gains `wheel_x` (a vertical wheel scrolls sideways, the tab strip's);
- `Ui::button_with` (`ButtonOptions`: kind, small, icon), `ButtonKind::Ghost`, `Ui::icon_button`
  (`IconButton`); `FieldOptions.chrome: FieldChrome::{Box, Bare}` for the filter bar's field (D116);
- new widgets with a `Ui` method each: `badge`, `status_dot`, `verdict_row`, `progress`, `spinner`, `callout`,
  `stat`, `stat_strip`, `kv_list`, `selectable_text`, `setting_row`, `item_row`, `list_row` (inside `virtual_list`), `form_field`, `steps`,
  `nav_list`, `radio_group`, `star_rating`, `slider`, `filter_bar`, `search_picker`, `tab_strip`, `toolbar`,
  `toolbar_label`, `banner` and `empty_state`; `Tone` is the shared tone recipe; `Font::Mono` is the mono
  face the app supplies;
- `Theme` gains `window_fallback`, `fg_subtle` and `status_error_light` (22 tokens in the in-crate token
  test); `strings` gains the `luneui.tab-strip.*`, `luneui.verdict-row.*` and `luneui.stat.no-data` keys;
  `icon` gains `SYNC`, `STAR_FULL`, `STAR_EMPTY` and `CLOSE`;
- the radio joins `focus-ring-kinds` (the only intended change of an existing scene since M5).

Since M5 (D91 to D109):

- `Event`, `FrameConfig` and `FrameOutput` grow: `FocusEvent` and the ordered focus queue, `Keymap` and
  `Shortcuts` in `FrameConfig`, `cursor`, `ime_area` and `fired` in `FrameOutput`; `Response` gains the
  press details; `Xform` gains `dx` and `dy` (a translate);
- `Painter` gains `push_clip`/`pop_clip` (clips nest), `push_clip_rounded`, `push_translate`, `image`,
  `image_rotated`, `text_rotated`, `dash`, `arc`, `sector`, `fill_path`, and the text queries
  `caret`, `text_hit`, `text_lines`, `segments`; `Scene` layers gain `fills` and `images`;
- `TextSystem` gains `caret`, `hit`, `lines` and `segments` (a fake and contract cases come with them);
- `ImageStore`, `FakeImages` and `image_store_contract` are new, and `Gpu` implements the store;
- `Theme` gains `text_selection` and the scrollbar thumb fills; `MotionTokens` gains `tooltip_delay`;
- new core modules `layout`, `shortcuts`, `text_edit`, `drag` and the widgets `scroll_area`, `scrollbar`,
  `virtual_list`, `popover`, `tooltip`, `text_area`; `lune-ui-wgpu` takes `unicode-segmentation` as a
  direct dependency (already in the lock through cosmic-text; the core still has none);
- `Ui::text_field` keeps its signature; `Ui::text_field_with` and `Ui::text_area` are the full forms;
  `dropdown::panel_rect` and `menu_layout` call `place`.

Since M4:

- `Painter` gains a transform stack (`push_xform`, `pop_xform`; `set_xform` replaces it with zero or
  one entry; one entry is bit-identical to the old single transform, D66), hit tags, an op recorder
  and faded acrylic: `scene::Acrylic { shape, opacity }` replaces the bare acrylic shape in
  `LayerGeometry::acrylic` (opacity 1 is the old replace blend, D67);
- `LayerId::Toast` is a fifth layer above `Modal` (`LAYER_COUNT` 5, `SCENE_LAYERS` unchanged, D54);
- `Event` gains `Access`, `ClipboardText`, `ClipboardUnavailable` (it is `non_exhaustive`);
  `FrameOutput` gains `repaint`, `clipboard`, `tree`, `focused`; `CursorIcon::ALL` lists the 36 icons;
- `Theme` gains the D58 colour fields and `font_title`, `line_title`, and the theme carries the
  motion tokens (`Theme::motion`);
- `TextCache::new` loads only the fonts it is given and no system font; `TextCache::with_locale`
  takes the shaping locale; with no font every text is an empty layout of zero size (D78, D79);
- `Gpu::upload` and `Uploaded` are crate-private (the app draws through `FrameRenderer`);
  `Gpu::new_with_features` asks for optional device features and reports what the adapter granted;
- the old Painter-API widget functions keep their paths, signatures and pixels; each widget file now
  also has a `Look`, `paint` and a `Ui` method (D60);
- new modules `ui`, `access`, `motion`, `strings` (see above), re-exported by the facade;
- `Token::Zero` (outside `Token::ALL`) is a zero-duration row; the dropdown's snap
  open and close use it so an overlay with no motion reports `Idle` at once (D80).

## Provenance of reworded comments

On the move, 12 comment lines that named the source app's look were reworded so library source
carries no app word (rule 1, D14). No identifier or behaviour changed; the original wording and
what it pointed at (paths are the files' current homes):

| File | Original pointed at |
|---|---|
| `lune-ui-core/src/lib.rs` (doc) | "styled after Lunegit"; the original pointed at Luneterm's glass-styling doc (not in this repo) |
| `lune-ui-core/src/lib.rs` (doc) | the `lunegit_dark` preset; its values are copied from Lunegit's `DarkTheme.axaml` and `App.axaml` |
| `lune-ui-core/src/backdrop.rs` (`Backdrop` doc) | Lunegit's `TransparencyLevelHint` (Avalonia) |
| `lune-ui-core/src/acrylic.rs`, `lune-ui-wgpu/src/gpu.rs`, `lune-ui-wgpu/src/shader.wgsl`, `lune-ui-platform/src/material/macos.rs` | "same pixels as Lunegit's", "match Lunegit", "Lunegit's conic stop table", "what Avalonia (and so Lunegit) uses": the Avalonia look Lunegit uses |
| `lune-ui-core/src/chrome.rs` | Port of Lunegit `Controls/Terminal/TerminalChromeGeometry.cs`, `BuildLoopFigure` |
| `lune-ui-core/src/icon.rs` | "Lunegit uses the Codicons icon pack" |
| `lune-ui-core/src/widgets/dialog.rs`, `lune-ui-core/src/widgets/mod.rs` | the Lunegit dialog and Lunegit's Fluent theme overrides (`DarkTheme.axaml`) |

Source of truth for the original text: `git -C <Luneterm> show 296cfdc:crates/lune-ui/<file>`.

## Pixel gates

A drawing change must not move pixels unless the look change is intended (hard rule 8, D11). Dumps
are PNGs written by offscreen readback (no screen capture). The working tree dumps on a headless
device (no event loop, no window, D30); a base revision from before M2 step 4 still opens a hidden
winit window, so a gate against such a base needs a logged-in macOS GUI session. Each gate writes
only under `target/`, except that `scripts/pixel-gate.sh` extracts its base source under `$TMPDIR`
(outside the repo, so no cargo config in the repo tree reaches the base build; D16 of the M2 plan).

| Gate | Proves | Compare | Exit codes |
|---|---|---|---|
| `scripts/pixel-gate-luneterm.sh` | M1 only: this repo's `lune-ui` drew the same pixels as Luneterm's. Phases: lock parity (every registry package in this repo's `Cargo.lock` is in Luneterm's lock at the rev; `--parity-only` stops here and still applies), run A (Luneterm's own crate), run B (the scratch clone's `lune-ui` repointed at this repo's crate), compare with Luneterm's `dump_compare`. Since M2 step 2 run B cannot build against the new API, so a full run exits 2 by design (roadmap D4) | `18 files, 18 identical, 0 differ, max delta 0` | 0 pass; 1 parity mismatch or pixel difference; 2 usage or setup error |
| `scripts/pixel-gate.sh [--base <rev>] [--allow <id>...]` | A change since `<rev>` (default `HEAD`) leaves the gallery's reference scenes unchanged (the 15 legacy scenes stay on the Painter API, so the gate keeps proving the moved drawing code is untouched; scenes a later change adds are reported as new, D59). Dumps `git archive <rev>` and the working tree, compares with `dump-compare --allow-new`. `--allow <id>` names a scene whose frames may differ (reported only); an id not in `gallery --list` is an error | `15 files, 15 identical` | 0 pass; 1 delta or scene missing from the candidate; 2 usage or setup error (a base without `apps/gallery` included: nothing to compare is not a pass) |
| `scripts/check-targets.sh [--skip-linux]` | The workspace lints and builds beyond the host: Windows `clippy` (needs the `x86_64-pc-windows-msvc` target) and Linux fmt, clippy and tests in a `rust:1.92` container. Not a pixel gate; it covers the code the host `cfg` hides | `windows: passed`, `linux: passed` or `skipped (<reason>)` | 0 every half that ran passed; 1 a check failed; 2 usage |

Dump file names: the gallery writes `<id>@<scale>x.png` (logical 1040x660; `gallery --list` prints
the 15 legacy names first, then the Ui pages' scenes: 62 names after M4, 152 after M5, 231 after M6, 305 after M7, 332 after M8, 389 after M9); Luneterm's spike writes `NN-<name>@<scale>x.png` (18 scenes). `tools/dump-compare`
(`cargo run -p dump-compare -- <baseline> <candidate> [--max-delta N] [--allow-new]`) compares per
pixel, max channel delta including alpha; a file missing from the candidate, a file only in the
candidate (unless `--allow-new`), a size or colour-type mismatch or a delta above `N` (default 0)
exits 1; it prints `N files, M identical, K differ, max delta D`.

Re-baselining: a deliberate look change passes the gate with `--allow <id>` for the scenes it
moves, and records the reason in `docs/DECISIONS.md` in the same commit (hard rule 8). Dumps are not
committed (D11).

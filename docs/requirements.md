# Requirements

What luneui-rs must provide. Each requirement has an id `R-<area>-<n>` that component files in
`docs/components/` cite in their Needs section. Tags: **[existing]** the code exists today in
Luneterm's `crates/lune-ui` (from its module inventory, 2026-11-03); **[new]** no code anywhere;
**[open]** the requirement is real but its shape is not decided or not verified.

Nothing here is a design. Numbers are given only where Luneterm states one; the rest is "TBD by
planner".

## Consumers

- **Luneterm** (this author's terminal, `/Users/ijat/Projects/Luneterm`): its milestones M9 to M11
  (the app end to end, tabs and settings, finish) consume the toolkit. M8 (the C ABI) does not.
  Today only the `glass-ui` spike exercises the painter, frame and window APIs.
- **A possible Rust rebuild of Lunegit** (the Avalonia git client): about 75 views, custom controls
  and AvaloniaEdit-based editors. Not a commitment; its needs are rough stubs in the catalogue.
- **Others**: any app that wants the same glass look. The toolkit stays app-agnostic.

## Functional

### Core (painter, colour, theme)

- R-core-1 [existing] Immediate-mode painter with ordered layers; fill, rounded fill, ring, stroke
  and acrylic primitives; clip and transform (`paint.rs`).
- R-core-2 [existing] Hit regions carry an app-defined hit type; the painter answers hit and hover
  queries (`paint.rs`).
- R-core-3 [existing] Straight-alpha RGBA values with premultiply, blend-over and alpha helpers
  (`color.rs`).
- R-core-4 [existing] Look is a `Theme` value plus glass layer constants; the Lunegit dark preset is
  one such value (`theme.rs`). Other looks are further values, not code forks.
- R-core-5 [existing] Avalonia acrylic colour math with tests (`acrylic.rs`).
- R-core-6 [existing] Codicon glyph constants; the app supplies the icon font (`icon.rs`).
- R-core-7 [existing] Tab-outline loop geometry (`chrome.rs`).

### Text

- R-text-1 [existing] Shape and measure text with fonts the app registers; the toolkit bundles no
  font (`text.rs`, `TextCache::new`).
- R-text-2 [existing] Shaped-text cache keyed by content, with a scale setting (`text.rs`).
- R-text-3 [new] Multi-line text view and editor with syntax highlighting, for code, diffs and
  Markdown (Lunegit uses AvaloniaEdit + TextMate and Markdig).
- R-text-4 [open] IME composition input. Not covered by today's `lune-ui` (no module for it in the
  inventory); Luneterm M9 lists IME among its scope items.
- R-text-5 [open] Text selection, caret movement and clipboard in text fields. The existing `field`
  draws a placeholder and a caret when focused; selection behaviour is unverified.

### Render

- R-render-1 [existing] The scene, blur and target pass sequence over wgpu (`gpu.rs`, `frame.rs`).
- R-render-2 [existing] Backdrop blur pass (`blur.rs`).
- R-render-3 [existing] App-owned GPU drawing inside a layer through the `CustomDraw` hook; app
  specific surfaces (the terminal grid) draw through it, not in the toolkit (`custom.rs`, D8).
- R-render-4 [existing] Offscreen target, GPU readback and PNG writer, with no window or screen
  capture, so frames can be dumped for goldens (`dump.rs`).
- R-render-5 [existing] The Avalonia acrylic noise texture is bundled and licensed
  (`assets/acrylic-noise.png`, MIT).

### Platform (OS glass and window helpers)

- R-platform-1 [existing] macOS glass: NSVisualEffectView and NSGlassEffectView on the frame view
  (`material/macos.rs`).
- R-platform-2 [existing] Windows glass: DWM acrylic, mica and tabbed through `window-vibrancy`
  (`material/windows.rs`).
- R-platform-3 [existing] Linux and others: no OS glass; a glass `Material` returns an error and the
  app falls back to an opaque theme (`material/other.rs`).
- R-platform-4 [existing] Glass window attributes, edge resize hit test, double-click detection and
  caption button geometry for a custom caption (`window.rs`).
- R-platform-5 [open] OS clipboard, cursor icons, file dialogs and drag-and-drop between windows.
  None is in the inventory; whether the toolkit owns them or the app does is undecided.

### Widgets

- R-widget-1 [existing] Label, toggle, combo, stepper, text field, button and checkbox, drawn through
  the painter and generic over the app's hit type (`widgets/mod.rs`).
- R-widget-2 [new] The controls Lunegit uses that the toolkit lacks: scrollbar, tooltip, star rating,
  tab control, slider and others found by the planner (Lunegit styles several kinds; which exist as
  controls was not verified).
- R-widget-3 [open] Keyboard focus order and key handling inside widgets. Not in the inventory.

### Overlays

- R-overlay-1 [existing] Popup panel, modal dialog, dropdown (flips above its anchor near the window
  bottom) and glass context menu (`widgets/`).
- R-overlay-2 [new] Toasts, with a history list (Lunegit has both).
- R-overlay-3 [new] Tooltips and flyouts.

### Layout and data views

- R-layout-1 [new] Layout containers. No layout module exists in `lune-ui` today; widgets take a
  `Rect` the app computed.
- R-layout-2 [new] Resizable split panes and scrolling regions that preserve offset.
- R-layout-3 [new] Virtualized lists and tree views with tree guide lines, for long lists (Lunegit
  uses virtualizing panels in several views).
- R-data-1 [new] Domain views Lunegit draws itself: commit graph rows, diff views (unified and
  split) with hunk actions and conflict strips, pie chart, kanban board with drag-and-drop. Whether
  these are toolkit components or app code is for the planner.

### Motion

- R-motion-1 [existing] Cubic-bezier easing and menu open and close motion; the app passes the time
  in (`widgets/menu.rs`).
- R-motion-2 [new] General transitions and animation tiers, including a reduced-motion tier (Lunegit
  has `Animations.axaml` and `AnimationsReduced.axaml`).

### Infrastructure

- R-infra-1 [new] Localization: Lunegit loads `en.json` plus nine sample languages (de, es, fr, ja,
  ko, ms, pt-BR, ru, zh-CN). Today's `lune-ui` has none.
- R-infra-2 [new] Keyboard shortcut routing. Lunegit binds keys per view; a central registry was
  not located.
- R-infra-3 [new] Theme values loadable from JSON and re-appliable at run time (Lunegit's
  `ThemeApplicator` and sample themes).
- R-infra-4 [open] Accessibility (screen readers, focus indication, contrast). Not covered by
  today's `lune-ui`.
- R-infra-5 [new] In-window drag and drop (pointer capture and drag state) for tab reorder, kanban
  cards and the split diff in Lunegit. OS-level drag and drop is R-platform-5.

## Non-functional

- R-nfr-1 [existing] Deterministic frames: library `src` never reads the clock or the environment
  and never starts an event loop; the app passes time, sizes, fonts, theme and hit values in.
  Enforced by `scripts/check-boundaries.sh` rule 4.
- R-nfr-2 [existing] App-agnostic: no app, terminal or domain words in library source (boundary
  rules 1 to 3).
- R-nfr-3 [existing] `unsafe` only in platform modules, each block with a `// SAFETY:` comment
  (boundary rule 6, `CLAUDE.md` hard rule 3). Luneterm's three blocks are in `material/macos.rs`.
- R-nfr-4 [existing] Size: the release profile is LTO, one codegen unit, abort on panic, stripped;
  adding a dependency means measuring the release binary and noting the change. No numeric budget:
  TBD by planner.
- R-nfr-5 [open] Performance targets. The only number Luneterm states that touches this layer is
  the running-outline CPU budget of at most 3 percentage points (Luneterm `docs/lunegit-bug-ledger.md`
  row 16, roadmap M11); it is unmeasured. Frame-time and memory targets: TBD by planner.
- R-nfr-6 [open] HiDPI: the text cache has a scale setting and Luneterm's dump scenes carry a scale;
  the full scale contract (fractional scales, per-monitor change) is TBD by planner.
- R-nfr-7 [existing] Platforms: macOS and Windows get glass, Linux gets an opaque fallback
  (Luneterm decision 3). CI on all three OSes is not set up here yet (STATE.md).
- R-nfr-8 [new] Testability: each seam (text shaping, GPU, glass, windowing) gets a contract test
  and a fake (headless target, null material) once the extraction plan introduces it.

## Extraction

- R-ext-1 [new] The first move is behaviour-preserving: code moves from Luneterm's `lune-ui` as is;
  seams follow as separate gated steps.
- R-ext-2 [new] Pixel gate: the 18 scripted scenes of Luneterm's `glass-ui` spike, dumped before and
  after, compare at zero delta with its `dump_compare` tool, until a change is deliberate and
  re-baselined with its reason. The baseline form is undecided (STATE.md).
- R-ext-3 [new] Luneterm switches from its in-tree `lune-ui` to this repo's. How it depends on it
  before a remote exists is undecided and blocks Luneterm M9.
- R-ext-4 [new] Every bundled asset or code has its licence text in `licenses/` and a row in
  `THIRD_PARTY_NOTICES.md`: the Avalonia noise texture and colour formulas, any font the toolkit
  ships, any shader taken from elsewhere. The repo has neither file yet.
- R-ext-5 [new] Crate names and split are set by the extraction plan; `lune-termview` stays in
  Luneterm (D8).

## Out of scope

- The terminal grid and its renderer (`lune-termview`), the VT engine, PTY handling: Luneterm.
- Git logic, repository state, AI-session features: Lunegit.
- A web or CSS engine: styles are Rust tokens.

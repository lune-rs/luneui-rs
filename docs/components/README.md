# Component catalogue

One file per component the toolkit provides or must provide: what it is, what it needs, where the
code lives today, what is open. Requirements live in `../requirements.md`; this catalogue cites
them by id (`R-<area>-<n>`).

`INDEX.md` is **generated** by `scripts/gen-component-index.sh` from the front matter of the files
here. Never edit it by hand. The gate runs `scripts/gen-component-index.sh --check`, which fails
when `INDEX.md` is stale.

## Add a component

1. Copy `_template.md` to `docs/components/<area>/<slug>.md`.
2. Fill the front matter and the body sections.
3. Run `scripts/gen-component-index.sh`.
4. Commit the new file and the regenerated `INDEX.md` together.

Parallel agents only ever create or edit their own component files; the one shared file is the
generated `INDEX.md`, and a merge conflict there is resolved by re-running the generator.

## Id rule

Id = `<area>.<slug>`: lowercase letters and digits, single hyphens inside the slug, unique. The file
is `<area>/<slug>.md`, so the id and the path always agree (the generator checks it). No numeric
counters.

## Areas (closed list; extend it by editing this README and `AREAS` in the generator)

- `core`: colour, theme tokens, painter, icons, geometry shared by everything else
- `render`: GPU renderer, blur, frames, frame dump, the app-owned drawing hook
- `platform`: OS glass materials and window helpers (the only place for `unsafe`)
- `widget`: single controls (button, toggle, text field, ...)
- `overlay`: popups, dialogs, dropdowns, menus, toasts
- `layout`: containers, lists, trees, split panes, scrolling
- `data`: data-driven views (graphs, charts, boards, diff and code views)
- `text`: shaping, measuring, editing and viewing text
- `motion`: animation curves and timing
- `infra`: cross-cutting machinery (localization, shortcuts, theming reload, accessibility)
- `lunegit`: needs that are shaped like the Lunegit app and stay in it (`external` entries only)
- `luneterm`: the same for Luneterm (the terminal grid itself stays in Luneterm)

App areas hold app-shaped needs only. Since M3 they hold only `external` entries; a reusable shape
lives under a library area with a neutral id (DECISIONS D42). To generalise an app-area entry, move
it with `git mv` and change its id and area.

## Front matter

Between two `---` lines, one `key: value` per line, no nesting, every key present and non-empty:

| key | value |
| --- | --- |
| `id` | `<area>.<slug>` |
| `title` | short human title |
| `area` | one of the areas above, equal to the directory |
| `status` | see below |
| `consumers` | comma list: `luneterm`, `lunegit`, `wane`, `any` |
| `source` | existing code: a Luneterm-relative path for `in-luneterm`, `Lunegit: <path>` for a Lunegit stub, or `—`; for `done`, the repo paths (comma separated, from the workspace root) of the code, every one of which exists |
| `owner` | agent or task working on it, or `—` |
| `summary` | one line |

## Status

- `in-luneterm`: the code exists today in Luneterm's `lune-ui` (`source:` points at it) and moves in the extraction
- `planned`: a consumer has named the need; no code anywhere
- `stub`: rough placeholder, not analysed; a planner must look before anyone builds (none left since M3: an analysed stub becomes `planned`, DECISIONS D49)
- `in-progress`: someone owns it (`owner:`)
- `done`: lives in this repo and holds the done standard (DECISIONS D47) by something that runs here. Its
  `### Build` carries one line, `- Done (D47, M11): tests `<path>.rs::<fn>`, ...; page `<id>` or n/a;
  unverified here: <list or none>.`, which cites at least one test by file and function (a docs-only
  entry says `tests: n/a — <reason>` instead), has `owner: —`, and holds no "still needs" or "Open for
  `done`". A gallery test (`apps/gallery/src/catalogue.rs`) checks every one of these: a cited function that
  is not in its file fails the gate. What only a real OS shows stays in the entry as `unverified here`
  and is never ticked.
- `external`: needed by a consumer but stays in the consumer's repo

Class A and B (D234). Class A: every D47 item of the entry holds by something that runs here, so it goes
`done`. Class B: the entry's own text ties its done standard to a run that cannot happen here (an OS
material on real machines, a pixel gate on a presented frame); it stays `in-progress`, `owner:` names who
runs it, and the reason is under `## Open questions`. An audit may add a Class B entry only by quoting that
line of the entry; it may not remove one by rewording the line.

## Body sections

Five sections, in this order, with `## Spec` after `## Existing code` (DECISIONS D49):

1. `## What it is`
2. `## Needs` (bullets, each citing an `R-` id)
3. `## Existing code` (`path:line`, or "none")
4. `## Spec` with exactly these twelve `###` subsections in this order: `Evidence`, `Anatomy`, `States`,
   `Transitions`, `Input and focus`, `Retained state`, `Accessibility`, `Strings`, `Gallery scenes`,
   `Painter primitives`, `Acceptance`, `Build`. A subsection that does not apply reads
   `n/a — <reason>`; none is omitted. An entry may add `### Unverified` (what only a real OS shows) and
   `### Limits` (what the entry knowingly does not do, each with its DECISIONS row) after `Build`.
5. `## Open questions` (only what is still open, else `- None.`) and `## Notes for the planner`

An `external` entry (status `external`) has a short `## Spec` of three subsections only: `Evidence`,
`Reusable parts` (the library entry ids that cover the reusable pieces, with cites) and `Build`
(`- Milestone: external — stays in <app>`).

Rules for the spec subsections:

- **Evidence**: first line `Pinned: Lunegit <rev>, Wane <rev>, this repo <rev>`; then the files and line
  ranges read; then every conflict between sources and its winner. Precedence: this repo's moved
  code (pixel-gated), then Lunegit, then Wane; for a component only Wane draws, Wane.
- **Cites**: every claim in Anatomy, States, Transitions and Input and focus carries a tag:
  `[L path:n]` Lunegit, `[W path:n]` Wane, `[R path:n]` this repo, `[LT path:n]` Luneterm (`:n-m` and
  `:n,m` allowed), `[D<n>]` a DECISIONS row, `[derived: <from what>]` only when no source has the
  value, `[core.ui]`, `[core.focus]`, `[infra.accessibility]` or `[motion]` for toolkit-model
  statements, or `[<entry id>]` for another entry. A claim with no source and no derivation is an
  open question.
- **States**: table `| state | look | cite |`. **Transitions**: table
  `| change | property | kind | duration | easing | reduced | cite |` using the vocabulary of
  `../motion.md` (15 kinds, 5 durations, 7 easings; reduced is `keep`, `fade` or `snap`); a transition
  Lunegit lacks cites `[D40]`; name what never animates.
- **Input and focus**: pointer senses, keys, Tab stop, layer (`Base` < `Page` < `Popup` < `Modal`, and
  `Toast` above them by D54), and for overlays the focus scope, dismiss rules and pointer capture.
- **Retained state**: Transient, Persistent (owned by the container, survives `ui.keep_alive`),
  App-owned, Exit retention.
- **Accessibility**: role, name, description and value sources, states and actions from
  `infra.accessibility`'s enums; a role or action the enums lack is an open question `role-gap: ...`.
- **Strings**: keys `luneui.<slug>.<name>` with the English default and the Lunegit `en.json` line it
  comes from, or `None: <why>`.
- **Gallery scenes**: `- `<id>`: <setup>, t = <ms or settled>`; ids are lowercase-hyphen and unique
  across the catalogue except the existing scene ids, which are reused by name. Every state appears in
  a scene; every transition gets a mid-transition scene at half its Full duration.
- **Painter primitives**: `- Existing: <list>` and `- New: <list or none>` using the primitive names in
  `core.painter`; each new item cites the drawing that needs it and is added to `core.painter`'s
  aggregate table.
- **Acceptance**: behaviour tests with scripted input and injected time, pixel probes at scale 2, one
  conformance line per transition, the semantic-node test. Infrastructure entries with no UI name the
  existing tests by path.
- **Build**: `- Milestone: <M4..M11>` and `- Depends on: <entry ids>`.

State only what is verified; write unknowns as open questions.

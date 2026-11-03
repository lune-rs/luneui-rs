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
- `lunegit`: needs that are shaped like the Lunegit app, until a planner generalises them into a
  library area
- `luneterm`: the same for Luneterm (the terminal grid itself stays in Luneterm)

App areas hold app-shaped needs only. Once generalised, move the entry into a library area and
change its id (nothing else cites ids yet).

## Front matter

Between two `---` lines, one `key: value` per line, no nesting, every key present and non-empty:

| key | value |
| --- | --- |
| `id` | `<area>.<slug>` |
| `title` | short human title |
| `area` | one of the areas above, equal to the directory |
| `status` | see below |
| `consumers` | comma list: `luneterm`, `lunegit`, `any` |
| `source` | existing code: a Luneterm-relative path for `in-luneterm`, `Lunegit: <path>` for a Lunegit stub, or `—` |
| `owner` | agent or task working on it, or `—` |
| `summary` | one line |

## Status

- `in-luneterm`: the code exists today in Luneterm's `lune-ui` (`source:` points at it) and moves in the extraction
- `planned`: a consumer has named the need; no code anywhere
- `stub`: rough placeholder, not analysed; a planner must look before anyone builds
- `in-progress`: someone owns it (`owner:`)
- `done`: lives in this repo
- `external`: needed by a consumer but stays in the consumer's repo

## Body sections

What it is; Needs (bullets, each citing an `R-` id); Existing code (`path:line` or "none"); Open
questions; Notes for the planner. State only what is verified; write unknowns as open questions.

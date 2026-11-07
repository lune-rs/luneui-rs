---
id: infra.showcase
title: Gallery showcase
area: infra
status: planned
consumers: any
source: apps/gallery/src/showcase/mod.rs
owner: —
summary: The gallery's shell, one Ui page that hosts every other page behind a searchable sidebar; the live place to see and try the whole toolkit
---

## What it is

The gallery's window page (`apps/gallery/src/showcase`): a sidebar tree of groups, pages and scenes with
a search field, and the chosen page on a scrolling canvas beside it. It is an ordinary Ui page, so
its scenes run headless. A hosted page draws on a `Stage` (a rect of the window) and lays itself
out from its card; choosing a scene replays that scene's script in real time and leaves the page
live. Built in the gallery-showcase task; the global settings, the labs and the per-component
knobs follow in its later steps.

## Needs

None from a consumer: this is the gallery's own tool, not a toolkit component.

## Existing code

`apps/gallery/src/showcase/mod.rs` (the shell), `apps/gallery/src/showcase/nav.rs` (the groups, the
tree rows and the search).

## Spec

### Gallery scenes

- `showcase-home`: the shell on its first page, the Foundation group open.
- `showcase-search`: a search typed into the field narrows the tree.
- `showcase-table`: a search for the table, then a press on its row, which takes the content pane.

### Build

- Milestone: gallery-showcase, step 2 (the shell). Status is `planned` until the task closes (step 9).

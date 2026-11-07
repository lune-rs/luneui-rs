//! The showcase's sidebar model: which group each page sits in, the rows of the tree (all of
//! them, or those a search keeps) and what a row's key stands for.
//!
//! Pure: no `Ui`, no drawing. The groups follow the catalogue's areas (Foundation holds `core`
//! and `motion`, Platform and Infra holds both of those areas). Legacy pages are not listed (D8),
//! and the showcase does not list itself.

use std::collections::BTreeSet;

use crate::page::{Page, PageKind, UiScene};
use crate::pages::PAGES;

/// The id of the page that hosts the others.
pub(crate) const HOST: &str = "showcase";

/// One sidebar group: a heading and the pages under it, by page id, in the order shown.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Group {
    pub(crate) name: &'static str,
    pub(crate) pages: &'static [&'static str],
}

/// Every Ui page but the host sits in exactly one group (a test holds it).
pub(crate) const GROUPS: &[Group] = &[
    Group {
        name: "Foundation",
        pages: &["ui", "focus", "image", "primitives", "theme", "motion"],
    },
    Group {
        name: "Widgets",
        pages: &[
            "button",
            "toggle",
            "checkbox",
            "label",
            "stepper",
            "combo",
            "text-field",
            "scrollbar",
            "badge",
            "status-dot",
            "verdict-row",
            "spinner",
            "callout",
            "row",
            "list-row",
            "form-field",
            "steps",
            "nav-item",
            "radio-group",
            "star-rating",
            "slider",
            "filter-bar",
            "search-picker",
            "tab-strip",
            "toolbar",
            "banner",
        ],
    },
    Group {
        name: "Overlays",
        pages: &[
            "dialog",
            "dropdown",
            "menu",
            "popover",
            "tooltip",
            "flyout",
            "toast-notifications",
            "command-palette",
        ],
    },
    Group {
        name: "Layout",
        pages: &[
            "stack",
            "scroll-area",
            "virtual-list",
            "page",
            "panel",
            "sidebar",
            "side-panel",
            "icon-rail",
            "split-panes",
            "tree-view",
            "settings-page",
        ],
    },
    Group {
        name: "Data",
        pages: &[
            "progress",
            "stat",
            "kv-list",
            "table",
            "bar-chart",
            "pie-chart",
            "kanban-board",
            "lane-graph",
        ],
    },
    Group {
        name: "Text",
        pages: &[
            "text-edit",
            "ime",
            "code-block",
            "code-editor-view",
            "markdown-preview",
            "diff-view",
        ],
    },
    Group {
        name: "Platform and Infra",
        pages: &[
            "a11y",
            "locale",
            "shortcuts",
            "drag-and-drop",
            "tray",
            "file-drop",
            "window",
        ],
    },
];

/// The index in [`PAGES`] of the page with `id`.
pub(crate) fn index_of(id: &str) -> Option<usize> {
    PAGES.iter().position(|p| p.id == id)
}

/// The scenes of a Ui page; none for a legacy one.
pub(crate) fn scenes(page: &Page) -> &'static [UiScene] {
    match page.kind {
        PageKind::Ui(ui) => ui.scenes,
        PageKind::Legacy(_) => &[],
    }
}

/// The pages in the sidebar's order, as indices into [`PAGES`].
pub(crate) fn order() -> Vec<usize> {
    GROUPS
        .iter()
        .flat_map(|g| g.pages.iter())
        .filter_map(|id| index_of(id))
        .collect()
}

/// The group of the page at `page` in [`PAGES`].
pub(crate) fn group_of(page: usize) -> Option<usize> {
    let id = PAGES.get(page)?.id;
    GROUPS.iter().position(|g| g.pages.contains(&id))
}

/// The name a scene shows in the sidebar and the title: its id without the page's own name in
/// front, in words and capitalised (`table-rows` on the table page is "Rows"). Two scenes of one
/// page with the same id (the same look at two scales) say which scale in brackets.
pub(crate) fn scene_label(page: &Page, scene: usize) -> String {
    let list = scenes(page);
    let Some(s) = list.get(scene) else {
        return String::new();
    };
    let id = s.id;
    let stripped = id
        .strip_prefix(page.id)
        .and_then(|rest| rest.strip_prefix('-'))
        .unwrap_or(id);
    let words = if id == page.id { "default" } else { stripped };
    let mut text = words.replace('-', " ");
    if let Some(first) = text.get(..1) {
        text = first.to_uppercase() + &text[1..];
    }
    if list.iter().filter(|o| o.id == id).count() > 1 {
        text = format!("{text} ({}x)", s.scale);
    }
    text
}

/// What a row of the tree is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Group(usize),
    /// A page, by its index in [`PAGES`].
    Page(usize),
    /// A scene of a page: the page's index in [`PAGES`], the scene's in its list.
    Scene(usize, usize),
}

impl Target {
    /// The row key: unique across the tree, stable across a search.
    pub(crate) fn key(self) -> u64 {
        match self {
            Target::Group(g) => 1 + g as u64,
            Target::Page(p) => 100 + p as u64,
            Target::Scene(p, s) => 10_000 + p as u64 * 100 + s as u64,
        }
    }

    /// The target a row key stands for.
    pub(crate) fn of_key(key: u64) -> Option<Target> {
        match key {
            1..=99 => Some(Target::Group(usize::try_from(key - 1).ok()?)),
            100..=9_999 => Some(Target::Page(usize::try_from(key - 100).ok()?)),
            10_000.. => {
                let n = usize::try_from(key - 10_000).ok()?;
                Some(Target::Scene(n / 100, n % 100))
            }
            0 => None,
        }
    }
}

/// One row of the tree, in pre-order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    pub(crate) target: Target,
    pub(crate) depth: u16,
    pub(crate) text: String,
    pub(crate) has_children: bool,
}

fn matches(text: &str, query: &str) -> bool {
    text.to_lowercase().contains(query)
}

/// The rows a `query` keeps: with none, every group, page and scene; with one, the pages whose
/// title contains it or that have a scene whose id does (case does not matter), under their
/// groups, and of such a page the scenes that match (all of them when the title matched).
pub(crate) fn rows(query: &str) -> Vec<Row> {
    let query = query.trim().to_lowercase();
    let mut out = Vec::new();
    for (g, group) in GROUPS.iter().enumerate() {
        let start = out.len();
        out.push(Row {
            target: Target::Group(g),
            depth: 0,
            text: group.name.to_owned(),
            has_children: true,
        });
        for id in group.pages {
            let Some(p) = index_of(id) else { continue };
            let page = PAGES[p];
            let title_hit = query.is_empty() || matches(page.title, &query);
            let scenes: Vec<(usize, &UiScene)> = scenes(page)
                .iter()
                .enumerate()
                .filter(|(i, s)| {
                    title_hit || matches(s.id, &query) || matches(&scene_label(page, *i), &query)
                })
                .collect();
            if !title_hit && scenes.is_empty() {
                continue;
            }
            out.push(Row {
                target: Target::Page(p),
                depth: 1,
                text: page.title.to_owned(),
                has_children: !scenes.is_empty(),
            });
            out.extend(scenes.into_iter().map(|(s, _)| Row {
                target: Target::Scene(p, s),
                depth: 2,
                text: scene_label(page, s),
                has_children: false,
            }));
        }
        if out.len() == start + 1 {
            out.pop();
        }
    }
    out
}

/// What a search opens: every group and every page that holds a scene the query matched but
/// whose own title it did not (so the match shows).
pub(crate) fn expanded_for(query: &str) -> BTreeSet<u64> {
    let query = query.trim().to_lowercase();
    let mut open = BTreeSet::new();
    for row in rows(&query) {
        match row.target {
            Target::Group(_) => {
                open.insert(row.target.key());
            }
            Target::Page(p) if !query.is_empty() && !matches(PAGES[p].title, &query) => {
                open.insert(row.target.key());
            }
            _ => {}
        }
    }
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages(rows: &[Row]) -> Vec<&str> {
        rows.iter()
            .filter(|r| matches!(r.target, Target::Page(_)))
            .map(|r| r.text.as_str())
            .collect()
    }

    /// Every Ui page of the registry is in a group, once, bar the host; no group names a page
    /// that is not there or is a legacy one.
    #[test]
    fn every_ui_page_is_in_exactly_one_group() {
        let mut seen = BTreeSet::new();
        for g in GROUPS {
            for id in g.pages {
                let p = index_of(id).unwrap_or_else(|| panic!("group {}: no page {id}", g.name));
                assert!(
                    matches!(PAGES[p].kind, PageKind::Ui(_)),
                    "{id} is a legacy page"
                );
                assert!(seen.insert(*id), "{id} is in two groups");
            }
        }
        let missing: Vec<&str> = PAGES
            .iter()
            .filter(|p| matches!(p.kind, PageKind::Ui(_)) && p.id != HOST)
            .map(|p| p.id)
            .filter(|id| !seen.contains(id))
            .collect();
        assert!(missing.is_empty(), "pages in no group: {missing:?}");
    }

    /// Scene names read as words, without the page's own name or a scale suffix; a look that is
    /// listed at two scales says which.
    #[test]
    fn scene_labels_are_readable_and_distinct_within_a_page() {
        let table = PAGES[index_of("table").expect("a page")];
        let ids: Vec<&str> = scenes(table).iter().map(|s| s.id).collect();
        let rows = ids
            .iter()
            .position(|id| *id == "table-rows")
            .expect("a scene");
        assert_eq!(scene_label(table, rows), "Rows");
        for p in PAGES.iter().filter(|p| matches!(p.kind, PageKind::Ui(_))) {
            let labels: Vec<String> = (0..scenes(p).len()).map(|i| scene_label(p, i)).collect();
            let unique: BTreeSet<&String> = labels.iter().collect();
            assert_eq!(unique.len(), labels.len(), "{}: {labels:?}", p.id);
            assert!(
                labels.iter().all(|l| !l.contains('@') && !l.contains('-')),
                "{}: {labels:?}",
                p.id
            );
        }
    }

    #[test]
    fn row_keys_round_trip_and_do_not_collide() {
        let rows = rows("");
        let keys: BTreeSet<u64> = rows.iter().map(|r| r.target.key()).collect();
        assert_eq!(keys.len(), rows.len());
        for r in &rows {
            assert_eq!(Target::of_key(r.target.key()), Some(r.target), "{}", r.text);
        }
        assert_eq!(Target::of_key(0), None);
    }

    #[test]
    fn no_search_lists_every_page_and_scene_in_pre_order() {
        let all = rows("");
        let ui_pages = order().len();
        assert_eq!(pages(&all).len(), ui_pages);
        assert_eq!(all[0].text, "Foundation");
        assert_eq!(all[0].depth, 0);
        // Depths rise by at most one, as the tree needs.
        for w in all.windows(2) {
            assert!(
                w[1].depth <= w[0].depth + 1,
                "{} then {}",
                w[0].text,
                w[1].text
            );
        }
        let scene_rows = all
            .iter()
            .filter(|r| matches!(r.target, Target::Scene(..)))
            .count();
        let scenes_total: usize = PAGES.iter().map(|p| scenes(p).len()).sum::<usize>()
            - scenes(PAGES[index_of(HOST).unwrap_or(0)]).len();
        // Legacy pages' scenes are not listed; the host's are not either.
        assert!(scene_rows > 0 && scene_rows <= scenes_total);
    }

    #[test]
    fn a_search_matches_titles_and_scene_ids_without_case() {
        assert_eq!(pages(&rows("TAB STRIP")), ["Tab strip"]);
        assert_eq!(pages(&rows("tab strip")), ["Tab strip"]);
        // A scene id: the dialog page has a `dialog-form` scene.
        let hit = rows("dialog-form");
        assert!(pages(&hit).contains(&"Dialog"), "{:?}", pages(&hit));
        let scene_ids: Vec<&str> = hit
            .iter()
            .filter_map(|r| match r.target {
                Target::Scene(p, i) => Some(scenes(PAGES[p])[i].id),
                _ => None,
            })
            .collect();
        assert!(!scene_ids.is_empty());
        assert!(
            scene_ids.iter().all(|id| id.contains("dialog-form")),
            "{scene_ids:?}"
        );
        // A title match lists the page's scenes whole.
        let table = rows("table");
        let table_page = index_of("table").expect("a page");
        assert!(
            table
                .iter()
                .any(|r| r.target == Target::Scene(table_page, 0))
        );
        assert!(rows("zzzz-no-such").is_empty());
    }

    #[test]
    fn a_search_opens_what_matched_by_scene_and_clearing_it_opens_nothing_extra() {
        let open = expanded_for("dialog-form");
        let dialog = index_of("dialog").expect("a page");
        assert!(open.contains(&Target::Page(dialog).key()));
        assert!(open.contains(&Target::Group(2).key()));
        // No query: every group, no page (the groups keep their own state when it is cleared).
        let none = expanded_for("");
        assert!(
            none.iter()
                .all(|k| matches!(Target::of_key(*k), Some(Target::Group(_))))
        );
    }
}

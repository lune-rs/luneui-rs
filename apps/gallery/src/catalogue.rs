//! The catalogue check (M11, D236): the gallery and `docs/components` agree, and a `done` entry
//! carries the evidence its status claims. It reads the entries from `docs/components` under the
//! workspace root and the scenes from `PAGES`; nothing here draws.
//!
//! Each rule is a function over an [`Entry`] (or all of them) that returns its complaints, so a
//! test can hand it a small wrong entry and see it fire. The real tests run the rules over the
//! real catalogue.
//!
//! - every hyphenated backticked id under an entry's `### Gallery scenes`, and the first id of
//!   each bullet, is a scene or a page of the gallery; every scene is claimed;
//! - a non-external entry names a scene or says `n/a —`;
//! - the `in-progress` entries are exactly the two Class B ones (D234), each quoting its line;
//! - a `done` entry has no owner, a `source:` that exists, no open "still needs", and a
//!   `- Done (D47, M11):` line in `### Build` whose cited tests exist, unless it says
//!   `tests: n/a — <reason>`;
//! - every requirement id of `docs/requirements.md` is cited by an entry, bar the ones nothing
//!   in this repo can satisfy;
//! - every motion kind and easing is named where `motion-kinds` and `motion-easings` are claimed.
//!
//! A claim's `@Nx` suffix is dropped: the check reads scene ids, not scales, so an entry claiming
//! `ui-controls@3x` passes while only `ui-controls` exists.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use lune_ui::motion::Kind;

use crate::page::PageKind;
use crate::pages::PAGES;
use crate::pages::motion::{EASINGS, easing_name, kind_name};

/// Requirement ids no entry cites, with the reason, and the only ones allowed to go uncited.
const UNCITED: &[(&str, &str)] = &[
    ("R-ext-3", "an extraction rule, held by the pixel gates"),
    ("R-ext-5", "an extraction rule, held by the pixel gates"),
    ("R-nfr-2", "a non-functional rule about the whole repo"),
    ("R-nfr-4", "a non-functional rule about the whole repo"),
    ("R-nfr-7", "a non-functional rule about the whole repo"),
];

/// One catalogue file, split.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) path: String,
    pub(crate) status: String,
    pub(crate) owner: String,
    pub(crate) source: String,
    pub(crate) body: String,
}

impl Entry {
    /// Splits `text` at its front matter. `path` is only for messages.
    pub(crate) fn parse(path: &str, text: &str) -> Result<Entry, String> {
        let mut parts = text.splitn(3, "---\n");
        let (Some(""), Some(front), Some(body)) = (parts.next(), parts.next(), parts.next()) else {
            return Err(format!("{path}: no front matter"));
        };
        let key = |k: &str| {
            front
                .lines()
                .find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix(": ")))
                .map(|v| v.trim().to_owned())
                .ok_or_else(|| format!("{path}: no `{k}`"))
        };
        Ok(Entry {
            path: path.to_owned(),
            status: key("status")?,
            owner: key("owner")?,
            source: key("source")?,
            body: body.to_owned(),
        })
    }

    /// The text of `### <name>` up to the next heading.
    pub(crate) fn section(&self, name: &str) -> Option<&str> {
        let head = format!("### {name}\n");
        let at = self.body.find(&head)? + head.len();
        let rest = &self.body[at..];
        let end = rest
            .find("\n### ")
            .into_iter()
            .chain(rest.find("\n## "))
            .min()
            .unwrap_or(rest.len());
        Some(rest[..end].trim())
    }
}

/// Every backticked span of `text`.
fn spans(text: &str) -> Vec<&str> {
    text.split('`').skip(1).step_by(2).collect()
}

/// `id` or `id@2x`, lowercase letters, digits and single hyphens: the shape of a scene id.
fn scene_shaped(span: &str) -> Option<&str> {
    let id = span.split('@').next().unwrap_or(span);
    let rest = span.get(id.len()..).unwrap_or("");
    let suffix_ok = rest.is_empty()
        || (rest.starts_with('@')
            && rest.ends_with('x')
            && rest[1..rest.len() - 1]
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.'));
    let words_ok = !id.is_empty()
        && id.split('-').all(|w| {
            !w.is_empty()
                && w.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        });
    (suffix_ok && words_ok).then_some(id)
}

/// What a `### Gallery scenes` section claims: every hyphenated scene-shaped span, and the first
/// span of each bullet (a one-word scene id, `controls`, can only be told by its place).
pub(crate) fn claims(section: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for s in spans(section) {
        if let Some(id) = scene_shaped(s)
            && id.contains('-')
        {
            out.insert(id.to_owned());
        }
    }
    for line in section.lines() {
        if let Some(rest) = line.strip_prefix("- `")
            && let Some(first) = rest.split('`').next()
            && let Some(id) = scene_shaped(first)
        {
            out.insert(id.to_owned());
        }
    }
    out
}

/// What the gallery has: every scene id and every page id.
#[derive(Debug, Default)]
pub(crate) struct Known {
    pub(crate) scenes: BTreeSet<String>,
    pub(crate) pages: BTreeSet<String>,
}

impl Known {
    pub(crate) fn of_gallery() -> Known {
        let mut k = Known::default();
        for page in PAGES {
            k.pages.insert(page.id.to_owned());
            match page.kind {
                PageKind::Legacy(scenes) => {
                    k.scenes.extend(scenes.iter().map(|s| s.id.to_owned()));
                }
                PageKind::Ui(ui) => k.scenes.extend(ui.scenes.iter().map(|s| s.id.to_owned())),
            }
        }
        k
    }

    fn has(&self, id: &str) -> bool {
        self.scenes.contains(id) || self.pages.contains(id)
    }
}

/// The workspace root.
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every entry under `docs/components`, in path order.
pub(crate) fn load(root: &Path) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut dirs: Vec<_> = std::fs::read_dir(root.join("docs/components"))
        .expect("docs/components")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .expect("area dir")
            .map(|e| e.expect("file entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
            .collect();
        files.sort();
        for f in files {
            let text = std::fs::read_to_string(&f).expect("entry");
            // `/` separators on every host: the rules match names by `/` suffix.
            let name = f
                .strip_prefix(root)
                .unwrap_or(&f)
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.push(Entry::parse(&name, &text).unwrap_or_else(|e| panic!("{e}")));
        }
    }
    out
}

// ---- rules ----------------------------------------------------------------------------------

/// Scenes an entry claims that the gallery does not have.
pub(crate) fn unknown_scenes(e: &Entry, known: &Known) -> Vec<String> {
    let Some(sec) = e.section("Gallery scenes") else {
        return Vec::new();
    };
    claims(sec)
        .into_iter()
        .filter(|id| !known.has(id))
        .map(|id| format!("{}: claims `{id}`, which the gallery does not have", e.path))
        .collect()
}

/// A non-external entry names a scene (or a page) or says `n/a —`.
pub(crate) fn has_a_scene_or_na(e: &Entry, known: &Known) -> Vec<String> {
    if e.status == "external" {
        return Vec::new();
    }
    let Some(sec) = e.section("Gallery scenes") else {
        return vec![format!("{}: no `### Gallery scenes`", e.path)];
    };
    let names_one = claims(sec).iter().any(|id| known.has(id))
        || spans(sec)
            .into_iter()
            .filter_map(scene_shaped)
            .any(|id| known.has(id));
    if names_one || sec.starts_with("n/a —") {
        Vec::new()
    } else {
        vec![format!(
            "{}: Gallery scenes names no scene and is not `n/a —`",
            e.path
        )]
    }
}

/// Scenes and pages of the gallery no entry claims.
pub(crate) fn unclaimed(entries: &[Entry], known: &Known) -> Vec<String> {
    let mut claimed = BTreeSet::new();
    for e in entries {
        if let Some(sec) = e.section("Gallery scenes") {
            claimed.extend(claims(sec));
            claimed.extend(
                spans(sec)
                    .into_iter()
                    .filter_map(scene_shaped)
                    .map(str::to_owned),
            );
        }
    }
    known
        .scenes
        .iter()
        .filter(|id| !claimed.contains(*id))
        .map(|id| format!("scene `{id}` is claimed by no entry"))
        .collect()
}

/// The `<path>.rs::<fn>` citations in `line`.
fn cited_tests(line: &str) -> Vec<(String, String)> {
    spans(line)
        .into_iter()
        .flat_map(|s| s.split(", ").map(str::trim).collect::<Vec<_>>())
        .filter_map(|s| {
            let (file, name) = s.split_once(".rs::")?;
            Some((format!("{file}.rs"), name.to_owned()))
        })
        .collect()
}

/// What a `done` entry must hold; nothing for any other status.
pub(crate) fn done_complaints(e: &Entry, root: &Path) -> Vec<String> {
    if e.status != "done" {
        return Vec::new();
    }
    let mut out = Vec::new();
    let p = &e.path;
    if e.owner != "—" {
        out.push(format!("{p}: done, but owner is `{}`", e.owner));
    }
    let paths: Vec<&str> = e
        .source
        .split(',')
        .map(|s| s.trim().trim_matches('`'))
        .filter(|s| !s.is_empty())
        .collect();
    if paths.is_empty() || paths.contains(&"—") {
        out.push(format!("{p}: done, but source names no path"));
    }
    for s in paths.iter().filter(|s| **s != "—") {
        if !root.join(s).exists() {
            out.push(format!("{p}: source `{s}` does not exist"));
        }
    }
    for needle in ["still needs", "Open for `done`"] {
        if e.body.contains(needle) {
            out.push(format!("{p}: done, but the entry says \"{needle}\""));
        }
    }
    let Some(build) = e.section("Build") else {
        out.push(format!("{p}: done, but no `### Build`"));
        return out;
    };
    let Some(line) = build.lines().find(|l| l.starts_with("- Done (D47, M11):")) else {
        out.push(format!(
            "{p}: done, but Build has no `- Done (D47, M11):` line"
        ));
        return out;
    };
    // The page and unverified parts must be there; their content is the entry's to word (real
    // lines say `n/a (...)`, `progress-*`).
    for part in ["; page ", "; unverified here:"] {
        if !line.contains(part) {
            out.push(format!(
                "{p}: the Done line has no `{}` part",
                part.trim_start_matches("; ")
            ));
        }
    }
    if line.contains("tests: n/a —") {
        return out;
    }
    let tests = cited_tests(line);
    if tests.is_empty() {
        out.push(format!("{p}: the Done line cites no `<path>.rs::<fn>`"));
    }
    for (file, name) in tests {
        match std::fs::read_to_string(root.join(&file)) {
            Err(_) => out.push(format!("{p}: cited test file `{file}` does not exist")),
            Ok(text) => {
                if !text.contains(&format!("fn {name}(")) && !text.contains(&format!("fn {name}<"))
                {
                    out.push(format!("{p}: `{file}` has no `fn {name}`"));
                }
            }
        }
    }
    out
}

/// The entries that stay `in-progress` by their own text (Class B, D234): path suffix, who runs
/// what the entry names, and the line of the entry that ties its done standard to that run. An
/// audit may add one only by quoting such a line; it may not remove one by rewording it.
pub(crate) const CLASS_B: [(&str, &str, &str); 2] = [
    (
        "platform/material.md",
        "Luneterm M9 to M11 runs",
        "The done standard (D18) for this entry needs those runs",
    ),
    (
        "render/surface.md",
        "consumer's window run",
        "the pixel gate on a presented frame (open: the platform run is not in this milestone)",
    ),
];

/// The `in-progress` entries that are not Class B, and a Class B entry that is not `in-progress`,
/// has another owner, or no longer quotes its line.
pub(crate) fn class_b_complaints(entries: &[Entry]) -> Vec<String> {
    let mut out = Vec::new();
    for e in entries.iter().filter(|e| e.status == "in-progress") {
        let p = &e.path;
        let Some((_, owner, quote)) = CLASS_B.iter().find(|(suffix, _, _)| p.ends_with(suffix))
        else {
            out.push(format!("{p}: in-progress, but it is not a Class B entry"));
            continue;
        };
        if e.owner != *owner {
            out.push(format!("{p}: owner is `{}`, not `{owner}`", e.owner));
        }
        if !e.body.contains(quote) {
            out.push(format!("{p}: no longer quotes \"{quote}\""));
        }
    }
    for (suffix, _, _) in CLASS_B {
        if !entries
            .iter()
            .any(|e| e.path.ends_with(suffix) && e.status == "in-progress")
        {
            out.push(format!("{suffix}: Class B, but not an in-progress entry"));
        }
    }
    out
}

/// `R-<area>-<n>` ids in `text`.
pub(crate) fn requirement_ids(text: &str) -> BTreeSet<String> {
    let bytes = text.as_bytes();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while let Some(at) = text[i..].find("R-") {
        let start = i + at;
        i = start + 2;
        if start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
            continue;
        }
        let mut j = start + 2;
        while j < bytes.len() && bytes[j].is_ascii_lowercase() {
            j += 1;
        }
        if j == start + 2 || bytes.get(j) != Some(&b'-') {
            continue;
        }
        let digits = j + 1;
        let mut k = digits;
        while k < bytes.len() && bytes[k].is_ascii_digit() {
            k += 1;
        }
        if k > digits {
            out.insert(text[start..k].to_owned());
        }
    }
    out
}

/// Requirement ids of `requirements` that no entry cites and that are not in [`UNCITED`].
pub(crate) fn uncited_requirements(entries: &[Entry], requirements: &str) -> Vec<String> {
    let mut cited = BTreeSet::new();
    for e in entries {
        cited.extend(requirement_ids(&e.body));
    }
    requirement_ids(requirements)
        .into_iter()
        .filter(|id| !cited.contains(id) && !UNCITED.iter().any(|(u, _)| u == id))
        .map(|id| format!("{id} is cited by no entry"))
        .collect()
}

/// The words of `names` that the bullet claiming `scene` in the entries does not hold.
pub(crate) fn missing_from_scene(entries: &[Entry], scene: &str, names: &[&str]) -> Vec<String> {
    let needle = format!("- `{scene}`");
    let bullet = entries
        .iter()
        .filter_map(|e| e.section("Gallery scenes"))
        .flat_map(str::lines)
        .find(|l| l.starts_with(&needle));
    let Some(bullet) = bullet else {
        return vec![format!(
            "no entry's Gallery scenes holds a bullet for `{scene}`"
        )];
    };
    names
        .iter()
        .filter(|n| !bullet.contains(*n))
        .map(|n| format!("`{scene}` does not name `{n}`"))
        .collect()
}

// ---- the real catalogue ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(status: &str, owner: &str, source: &str, body: &str) -> Entry {
        let text = format!(
            "---\nid: x.y\nstatus: {status}\nowner: {owner}\nsource: {source}\n---\n{body}"
        );
        Entry::parse("x/y.md", &text).expect("parses")
    }

    fn known(scenes: &[&str]) -> Known {
        Known {
            scenes: scenes.iter().map(|s| (*s).to_owned()).collect(),
            pages: BTreeSet::new(),
        }
    }

    // -- the rules fire on a wrong entry --------------------------------------------------

    #[test]
    fn a_claimed_scene_the_gallery_lacks_is_reported() {
        let e = entry(
            "in-progress",
            "o",
            "—",
            "### Gallery scenes\n\n- `real-scene`: a\n- `no-such-scene`: b\n\n### Build\n",
        );
        let bad = unknown_scenes(&e, &known(&["real-scene"]));
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(bad[0].contains("no-such-scene"));
    }

    #[test]
    fn a_one_word_scene_id_at_a_bullets_start_is_checked() {
        let e = entry(
            "in-progress",
            "o",
            "—",
            "### Gallery scenes\n\n- `controls`: a\n",
        );
        assert_eq!(unknown_scenes(&e, &known(&["chrome"])).len(), 1);
        assert!(unknown_scenes(&e, &known(&["controls"])).is_empty());
    }

    #[test]
    fn a_scene_no_entry_claims_is_reported() {
        let e = entry(
            "in-progress",
            "o",
            "—",
            "### Gallery scenes\n\n- `a-b`: x\n",
        );
        let bad = unclaimed(&[e], &known(&["a-b", "c-d"]));
        assert_eq!(bad, ["scene `c-d` is claimed by no entry"]);
    }

    #[test]
    fn an_entry_with_neither_a_scene_nor_n_a_is_reported() {
        let none = entry(
            "in-progress",
            "o",
            "—",
            "### Gallery scenes\n\nSomething else.\n",
        );
        assert_eq!(has_a_scene_or_na(&none, &known(&["a-b"])).len(), 1);
        let na = entry(
            "in-progress",
            "o",
            "—",
            "### Gallery scenes\n\nn/a — no UI.\n",
        );
        assert!(has_a_scene_or_na(&na, &known(&["a-b"])).is_empty());
        let ext = entry(
            "external",
            "o",
            "—",
            "### Gallery scenes\n\nSomething else.\n",
        );
        assert!(has_a_scene_or_na(&ext, &known(&["a-b"])).is_empty());
    }

    const BUILD_OK: &str = "### Build\n\n- Milestone: M11\n- Done (D47, M11): tests `crates/lune-ui-core/tests/color.rs::premul_at_alpha_zero_is_all_zero`; page n/a; unverified here: none.\n";

    #[test]
    fn a_done_entry_that_holds_its_evidence_passes() {
        let e = entry("done", "—", "crates/lune-ui-core/src/color.rs", BUILD_OK);
        assert_eq!(done_complaints(&e, &root()), Vec::<String>::new());
    }

    #[test]
    fn a_done_entry_citing_an_absent_fn_is_reported() {
        let body = BUILD_OK.replace("premul_at_alpha_zero_is_all_zero", "no_such_fn");
        let e = entry("done", "—", "crates/lune-ui-core/src/color.rs", &body);
        let bad = done_complaints(&e, &root());
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(bad[0].contains("no `fn no_such_fn`"));
    }

    #[test]
    fn a_done_entry_citing_an_absent_file_is_reported() {
        let body = BUILD_OK.replace("tests/color.rs", "tests/nope.rs");
        let e = entry("done", "—", "crates/lune-ui-core/src/color.rs", &body);
        assert!(done_complaints(&e, &root())[0].contains("does not exist"));
    }

    #[test]
    fn a_done_entry_without_the_line_an_owner_or_a_source_is_reported() {
        let e = entry(
            "done",
            "o",
            "nope/none.rs",
            "### Build\n\n- Milestone: M11\n",
        );
        let bad = done_complaints(&e, &root());
        assert!(bad.iter().any(|b| b.contains("owner")), "{bad:?}");
        assert!(bad.iter().any(|b| b.contains("does not exist")), "{bad:?}");
        assert!(bad.iter().any(|b| b.contains("no `- Done")), "{bad:?}");
    }

    #[test]
    fn a_done_line_without_a_page_part_or_an_unverified_part_is_reported() {
        let no_page = BUILD_OK.replace("; page n/a", "");
        let e = entry("done", "—", "crates/lune-ui-core/src/color.rs", &no_page);
        let bad = done_complaints(&e, &root());
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(bad[0].contains("no `page ` part"), "{bad:?}");
        let no_unverified = BUILD_OK.replace("; unverified here: none", "");
        let e = entry(
            "done",
            "—",
            "crates/lune-ui-core/src/color.rs",
            &no_unverified,
        );
        let bad = done_complaints(&e, &root());
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(bad[0].contains("no `unverified here:` part"), "{bad:?}");
        // A docs-only line is held to the same two parts.
        let docs = "### Build\n\n- Done (D47, M11): tests: n/a — documentation only.\n";
        let e = entry("done", "—", "docs/architecture.md", docs);
        assert_eq!(done_complaints(&e, &root()).len(), 2);
    }

    #[test]
    fn a_done_entry_that_still_needs_something_is_reported() {
        let body = format!("- Done standard still needs: a test.\n\n{BUILD_OK}");
        let e = entry("done", "—", "crates/lune-ui-core/src/color.rs", &body);
        assert!(done_complaints(&e, &root())[0].contains("still needs"));
    }

    #[test]
    fn a_docs_only_done_entry_may_say_its_tests_are_n_a() {
        let body = "### Build\n\n- Done (D47, M11): tests: n/a — documentation only; page n/a; unverified here: none.\n";
        let e = entry("done", "—", "docs/architecture.md", body);
        assert_eq!(done_complaints(&e, &root()), Vec::<String>::new());
    }

    #[test]
    fn an_entry_that_is_not_done_is_not_held_to_the_done_rules() {
        let e = entry("in-progress", "o", "nope", "");
        assert!(done_complaints(&e, &root()).is_empty());
    }

    #[test]
    fn an_in_progress_entry_outside_class_b_is_reported() {
        let e = entry("in-progress", "o", "—", "");
        let bad = class_b_complaints(&[e]);
        assert!(
            bad.iter().any(|b| b.contains("not a Class B entry")),
            "{bad:?}"
        );
    }

    #[test]
    fn a_class_b_entry_that_lost_its_quote_its_owner_or_its_status_is_reported() {
        let make = |status: &str, owner: &str, body: &str| {
            let text = format!(
                "---\nid: platform.material\nstatus: {status}\nowner: {owner}\nsource: —\n---\n{body}"
            );
            Entry::parse("docs/components/platform/material.md", &text).expect("parses")
        };
        let surface = |status: &str| {
            let text = format!(
                "---\nid: render.surface\nstatus: {status}\nowner: consumer's window run\nsource: —\n---\n{}",
                CLASS_B[1].2
            );
            Entry::parse("docs/components/render/surface.md", &text).expect("parses")
        };
        let ok = make("in-progress", CLASS_B[0].1, CLASS_B[0].2);
        assert_eq!(
            class_b_complaints(&[ok, surface("in-progress")]),
            Vec::<String>::new()
        );
        let no_quote = make("in-progress", CLASS_B[0].1, "reworded");
        assert!(class_b_complaints(&[no_quote, surface("in-progress")])[0].contains("quotes"));
        let other_owner = make("in-progress", "someone", CLASS_B[0].2);
        assert!(class_b_complaints(&[other_owner, surface("in-progress")])[0].contains("owner"));
        let flipped = make("done", "—", CLASS_B[0].2);
        let bad = class_b_complaints(&[flipped, surface("in-progress")]);
        assert!(
            bad.iter().any(|b| b.contains("Class B, but not")),
            "{bad:?}"
        );
    }

    #[test]
    fn requirement_ids_are_read_and_an_uncited_one_is_reported() {
        let ids = requirement_ids("see R-core-4, (R-nfr-6) and R-<area>-<n>, not XR-core-9");
        assert_eq!(ids.into_iter().collect::<Vec<_>>(), ["R-core-4", "R-nfr-6"]);
        let e = entry("in-progress", "o", "—", "Cites R-core-4.");
        let bad = uncited_requirements(&[e], "R-core-4 and R-core-5 and R-nfr-2");
        assert_eq!(bad, ["R-core-5 is cited by no entry"]);
    }

    #[test]
    fn a_motion_name_missing_from_its_scene_bullet_is_reported() {
        let e = entry(
            "in-progress",
            "o",
            "—",
            "### Gallery scenes\n\n- `motion-kinds`: hover and press\n",
        );
        let bad = missing_from_scene(
            std::slice::from_ref(&e),
            "motion-kinds",
            &["hover", "focus"],
        );
        assert_eq!(bad, ["`motion-kinds` does not name `focus`"]);
        assert_eq!(
            missing_from_scene(&[e], "motion-nothing", &["hover"]).len(),
            1
        );
    }

    // -- the real catalogue -----------------------------------------------------------------

    fn all() -> (Vec<Entry>, Known) {
        (load(&root()), Known::of_gallery())
    }

    fn none(complaints: Vec<String>) {
        assert_eq!(complaints, Vec::<String>::new());
    }

    #[test]
    fn the_catalogue_loads_98_entries_of_which_four_are_external() {
        let (entries, _) = all();
        assert_eq!(entries.len(), 98);
        assert_eq!(entries.iter().filter(|e| e.status == "external").count(), 4);
    }

    #[test]
    fn every_scene_an_entry_claims_is_in_the_gallery() {
        let (entries, known) = all();
        none(
            entries
                .iter()
                .flat_map(|e| unknown_scenes(e, &known))
                .collect(),
        );
        // The instrument: the gallery list is not empty and holds a known id.
        assert!(known.scenes.contains("controls") && known.pages.contains("window"));
    }

    #[test]
    fn every_scene_in_the_gallery_is_claimed_by_an_entry() {
        let (entries, known) = all();
        none(unclaimed(&entries, &known));
    }

    #[test]
    fn every_entry_names_a_scene_or_says_n_a() {
        let (entries, known) = all();
        none(
            entries
                .iter()
                .flat_map(|e| has_a_scene_or_na(e, &known))
                .collect(),
        );
    }

    #[test]
    fn every_done_entry_holds_its_evidence() {
        let (entries, _) = all();
        let root = root();
        none(
            entries
                .iter()
                .flat_map(|e| done_complaints(e, &root))
                .collect(),
        );
    }

    #[test]
    fn the_in_progress_entries_are_exactly_the_two_class_b_ones() {
        let (entries, _) = all();
        none(class_b_complaints(&entries));
        let open: Vec<&str> = entries
            .iter()
            .filter(|e| e.status == "in-progress")
            .map(|e| e.path.as_str())
            .collect();
        assert_eq!(open.len(), 2, "{open:?}");
    }

    #[test]
    fn every_requirement_is_cited_by_an_entry_or_listed() {
        let (entries, _) = all();
        let req =
            std::fs::read_to_string(root().join("docs/requirements.md")).expect("requirements");
        assert!(
            requirement_ids(&req).len() >= 60,
            "the requirements were read"
        );
        none(uncited_requirements(&entries, &req));
        // The list cannot go stale: each listed id is still in the requirements and still cited
        // by no entry (cite one, and it leaves the list).
        let ids = requirement_ids(&req);
        let cited: BTreeSet<String> = entries
            .iter()
            .flat_map(|e| requirement_ids(&e.body))
            .collect();
        for (id, why) in UNCITED {
            assert!(ids.contains(*id) && !why.is_empty(), "{id}");
            assert!(
                !cited.contains(*id),
                "{id} is cited now: drop it from UNCITED"
            );
        }
    }

    #[test]
    fn every_motion_kind_and_easing_is_named_where_its_scene_is_claimed() {
        let (entries, _) = all();
        let kinds: Vec<&str> = Kind::ALL.into_iter().map(kind_name).collect();
        assert_eq!(kinds.len(), 15);
        none(missing_from_scene(&entries, "motion-kinds", &kinds));
        let easings: Vec<&str> = EASINGS.into_iter().map(easing_name).collect();
        assert_eq!(easings.len(), 7);
        none(missing_from_scene(&entries, "motion-easings", &easings));
    }
}

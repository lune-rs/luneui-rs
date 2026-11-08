//! Generates `docs/api/`, the Markdown reference of the public API of the five library crates,
//! from the JSON that rustdoc writes for them.
//!
//! `api-docs --json-dir <dir> --html-dir <dir> --out <dir>`
//!
//! `<dir>` of `--json-dir` holds `<lib>.json` for each crate, `--html-dir` holds
//! `<lib>/all.html`, and `--out` must not exist or be empty. `scripts/gen-api-docs.sh` makes the
//! inputs and decides what to do with the output.
//!
//! Exit 0 when every page was written and the completeness checks held; exit 2 otherwise, with
//! one line per finding on stderr and nothing written. The checks fail closed: a rustdoc JSON
//! format this tool does not read, a public item that no page shows, a page for an item rustdoc's
//! `all.html` does not list, or an `all.html` that lists one the pages do not show.

#![forbid(unsafe_code)]

mod allhtml;
mod docs;
mod plan;
mod render;
mod sig;
mod view;
mod world;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use world::{CRATES, Fail, Res, World};

struct Args {
    json: PathBuf,
    html: PathBuf,
    out: PathBuf,
}

fn parse_args() -> Res<Args> {
    let mut json = None;
    let mut html = None;
    let mut out = None;
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let slot = match flag.as_str() {
            "--json-dir" => &mut json,
            "--html-dir" => &mut html,
            "--out" => &mut out,
            other => return Err(Fail::one(format!("unknown argument `{other}`"))),
        };
        *slot = Some(PathBuf::from(
            it.next()
                .ok_or_else(|| Fail::one(format!("{flag} needs a value")))?,
        ));
    }
    match (json, html, out) {
        (Some(json), Some(html), Some(out)) => Ok(Args { json, html, out }),
        _ => Err(Fail::one(
            "usage: api-docs --json-dir <dir> --html-dir <dir> --out <dir>".to_string(),
        )),
    }
}

fn run() -> Res<()> {
    let args = parse_args()?;
    ensure_empty(&args.out)?;
    let world = World::load(&args.json)?;
    let plan = plan::build(&world)?;
    let mut problems = Vec::new();
    for (ci, (_, lib)) in CRATES.iter().enumerate() {
        let path = args.html.join(lib).join("all.html");
        let html = std::fs::read_to_string(&path)
            .map_err(|e| Fail::one(format!("cannot read {}: {e}", path.display())))?;
        let listing = allhtml::parse(&html, &path.display().to_string())?;
        let documented = plan.listed[ci]
            .iter()
            .map(|(k, p)| ((*k).to_string(), p.clone()))
            .collect();
        if let Err(Fail(p)) = allhtml::compare(lib, &listing, &documented) {
            problems.extend(p);
        }
    }
    let rendered = render::render(&world, &plan)?;
    problems.extend(rendered.problems);
    if !problems.is_empty() {
        return Err(Fail(problems));
    }
    for (rel, text) in &rendered.files {
        let path = args.out.join(rel);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| Fail::one(format!("cannot create {}: {e}", dir.display())))?;
        }
        std::fs::write(&path, text)
            .map_err(|e| Fail::one(format!("cannot write {}: {e}", path.display())))?;
    }
    let omitted: Vec<String> = rendered
        .counts
        .skipped
        .iter()
        .map(|((ci, class), n)| format!("{} {class} {n}", CRATES[*ci].1))
        .collect();
    println!(
        "api-docs: {} files; left out and counted: {}",
        rendered.files.len(),
        omitted.join(", ")
    );
    Ok(())
}

fn ensure_empty(dir: &Path) -> Res<()> {
    match std::fs::read_dir(dir) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                Err(Fail::one(format!("{} is not empty", dir.display())))
            } else {
                Ok(())
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Fail::one(format!("cannot read {}: {e}", dir.display()))),
    }
}

/// Prints the findings of a failed run and gives the exit status: 0, or 2 for any failure.
fn exit_status(result: &Res<()>) -> u8 {
    match result {
        Ok(()) => 0,
        Err(Fail(lines)) => {
            for l in lines {
                eprintln!("api-docs: {l}");
            }
            2
        }
    }
}

fn main() -> ExitCode {
    ExitCode::from(exit_status(&run()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn a_completeness_failure_exits_2() {
        let html: allhtml::Listing = [("struct", "a/B"), ("fn", "c")]
            .iter()
            .map(|(k, p)| (k.to_string(), p.to_string()))
            .collect();
        assert_eq!(exit_status(&allhtml::compare("c", &html, &html)), 0);
        // The all.html set with one entry removed and one invented.
        let mut mutated: BTreeSet<(String, String)> = html.clone();
        mutated.remove(&("fn".to_string(), "c".to_string()));
        mutated.insert(("enum".to_string(), "x/Y".to_string()));
        assert_eq!(exit_status(&allhtml::compare("c", &html, &mutated)), 2);
    }
}

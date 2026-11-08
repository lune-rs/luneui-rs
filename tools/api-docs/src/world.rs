//! Loading the rustdoc JSON of the five crates, and finding an item across them.

use std::collections::HashMap;
use std::path::Path;

use rustdoc_types::{Crate, FORMAT_VERSION, Id, Item, ItemKind};

/// A failure that stops the run with exit 2; each string is one finding.
#[derive(Debug)]
pub(crate) struct Fail(pub(crate) Vec<String>);

impl Fail {
    pub(crate) fn one(message: impl Into<String>) -> Fail {
        Fail(vec![message.into()])
    }
}

pub(crate) type Res<T> = Result<T, Fail>;

/// An item of one of the loaded crates: the crate's index in `World::crates` and the item's id there.
pub(crate) type ItemRef = (usize, Id);

/// The five library crates: package directory name, library name. The facade is last; it
/// re-exports the others, and its pages are tables of links.
pub(crate) const CRATES: [(&str, &str); 5] = [
    ("lune-ui-core", "lune_ui_core"),
    ("lune-ui-wgpu", "lune_ui_wgpu"),
    ("lune-ui-winit", "lune_ui_winit"),
    ("lune-ui-platform", "lune_ui_platform"),
    ("lune-ui", "lune_ui"),
];

/// Index of the facade in `CRATES`.
pub(crate) const FACADE: usize = 4;

pub(crate) struct Krate {
    /// Package name, also the directory under `docs/api/`.
    pub(crate) dir: &'static str,
    /// Library name, as rustdoc writes it (`lune_ui_core`).
    pub(crate) lib: &'static str,
    pub(crate) data: Crate,
    /// The crate's own items by canonical path and kind, to find what another crate re-exports.
    by_path: HashMap<(Vec<String>, ItemKind), Id>,
}

pub(crate) struct World {
    pub(crate) crates: Vec<Krate>,
}

impl World {
    /// Reads `<dir>/<lib>.json` for each crate. A file whose format version is not the one
    /// `rustdoc-types` reads is refused before it is deserialised.
    pub(crate) fn load(dir: &Path) -> Res<World> {
        let mut crates = Vec::new();
        for (name, lib) in CRATES {
            let path = dir.join(format!("{lib}.json"));
            let bytes = std::fs::read(&path)
                .map_err(|e| Fail::one(format!("cannot read {}: {e}", path.display())))?;
            let value: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| Fail::one(format!("{} is not JSON: {e}", path.display())))?;
            let found = value
                .get("format_version")
                .and_then(serde_json::Value::as_u64);
            if found != Some(u64::from(FORMAT_VERSION)) {
                return Err(Fail::one(format!(
                    "format version mismatch: {} is rustdoc JSON format {}, but the pinned \
                     `rustdoc-types` (tools/api-docs/Cargo.toml) reads format {FORMAT_VERSION}. \
                     Move the `rustdoc-types` pin to the release that reads this format when the \
                     toolchain moved, then run scripts/gen-api-docs.sh again",
                    path.display(),
                    found.map_or_else(|| "(missing)".to_string(), |v| v.to_string()),
                )));
            }
            let data: Crate = serde_json::from_value(value).map_err(|e| {
                Fail::one(format!(
                    "cannot read {} as rustdoc JSON: {e}",
                    path.display()
                ))
            })?;
            if data.includes_private {
                return Err(Fail::one(format!(
                    "{} was built with private items; the reference documents the public API only",
                    path.display()
                )));
            }
            let mut by_path = HashMap::new();
            for (id, summary) in &data.paths {
                if summary.crate_id == 0 {
                    by_path.insert((summary.path.clone(), summary.kind), *id);
                }
            }
            crates.push(Krate {
                dir: name,
                lib,
                data,
                by_path,
            });
        }
        Ok(World { crates })
    }

    /// The item behind a reference.
    pub(crate) fn item(&self, r: ItemRef) -> Res<&Item> {
        self.crates[r.0].data.index.get(&r.1).ok_or_else(|| {
            Fail::one(format!(
                "{}: item {} is not in the JSON index",
                self.crates[r.0].lib, r.1.0
            ))
        })
    }

    /// The item a crate's id names, as a reference into the loaded crates: an item of the crate
    /// itself, or one of another loaded crate found by its canonical path. `None` when the id
    /// names something outside the five crates.
    pub(crate) fn locate(&self, from: usize, id: Id) -> Option<ItemRef> {
        let data = &self.crates[from].data;
        if data.index.get(&id).is_some_and(|it| it.crate_id == 0) {
            return Some((from, id));
        }
        let summary = data.paths.get(&id)?;
        if summary.crate_id == 0 {
            return None;
        }
        let owner = &data.external_crates.get(&summary.crate_id)?.name;
        let ci = self.crates.iter().position(|k| k.lib == owner.as_str())?;
        let target = self.crates[ci]
            .by_path
            .get(&(summary.path.clone(), summary.kind))?;
        Some((ci, *target))
    }
}

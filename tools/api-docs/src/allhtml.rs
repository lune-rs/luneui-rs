//! The independent witness of completeness: rustdoc's own `all.html`, which lists every item the
//! HTML build documents. The set of (kind, path) it gives must equal the set this tool lists.

use std::collections::BTreeSet;

use crate::world::{Fail, Res};

/// A (rustdoc kind word, `/`-joined path) pair, for example `("struct", "backdrop/Material")`.
pub(crate) type Listing = BTreeSet<(String, String)>;

/// Reads the item links of an `all.html`: `<li><a href="mod/kind.Name.html">`.
pub(crate) fn parse(html: &str, origin: &str) -> Res<Listing> {
    const MARK: &str = "<li><a href=\"";
    let mut out = Listing::new();
    let mut rest = html;
    while let Some(i) = rest.find(MARK) {
        rest = &rest[i + MARK.len()..];
        let end = rest
            .find('"')
            .ok_or_else(|| Fail::one(format!("{origin}: an item link is not closed")))?;
        let href = &rest[..end];
        if href.starts_with('#') {
            continue;
        }
        let stem = href
            .strip_suffix(".html")
            .ok_or_else(|| Fail::one(format!("{origin}: link `{href}` does not end in .html")))?;
        let (dir, file) = stem.rsplit_once('/').map_or(("", stem), |(d, f)| (d, f));
        let (kind, name) = file.split_once('.').ok_or_else(|| {
            Fail::one(format!(
                "{origin}: link `{href}` is not of the form kind.Name.html"
            ))
        })?;
        let path = if dir.is_empty() {
            name.to_string()
        } else {
            format!("{dir}/{name}")
        };
        out.insert((kind.to_string(), path));
    }
    if out.is_empty() {
        return Err(Fail::one(format!(
            "{origin}: no item links found (is it an all.html?)"
        )));
    }
    Ok(out)
}

/// Fails, naming every difference in both directions, unless the two sets are equal.
pub(crate) fn compare(crate_name: &str, html: &Listing, documented: &Listing) -> Res<()> {
    let mut problems = Vec::new();
    for (kind, path) in html.difference(documented) {
        problems.push(format!(
            "{crate_name}: all.html lists {kind} {path}, which the reference does not document"
        ));
    }
    for (kind, path) in documented.difference(html) {
        problems.push(format!(
            "{crate_name}: the reference documents {kind} {path}, which all.html does not list"
        ));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(Fail(problems))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML: &str = "<ul class=\"all-items\"><li><a href=\"backdrop/struct.GlassLayer.html\">backdrop::GlassLayer</a></li>\
        <li><a href=\"fn.top.html\">top</a></li><li><a href=\"theme/near_black/constant.TINT.html\">x</a></li></ul>";

    fn set(items: &[(&str, &str)]) -> Listing {
        items
            .iter()
            .map(|(k, p)| (k.to_string(), p.to_string()))
            .collect()
    }

    #[test]
    fn links_become_kind_and_path() {
        let got = parse(HTML, "t").expect("parses");
        assert_eq!(
            got,
            set(&[
                ("struct", "backdrop/GlassLayer"),
                ("fn", "top"),
                ("constant", "theme/near_black/TINT"),
            ])
        );
    }

    #[test]
    fn an_empty_or_odd_file_is_refused() {
        assert!(parse("<html></html>", "t").is_err());
        assert!(parse("<li><a href=\"nope\">x</a>", "t").is_err());
    }

    #[test]
    fn a_mutated_set_fails_in_both_directions() {
        let html = parse(HTML, "t").expect("parses");
        assert!(compare("c", &html, &html).is_ok());
        let mut missing = html.clone();
        missing.remove(&("fn".to_string(), "top".to_string()));
        let err = compare("c", &html, &missing).expect_err("html has one more");
        assert_eq!(err.0.len(), 1);
        assert!(err.0[0].contains("all.html lists fn top"));
        let mut extra = html.clone();
        extra.insert(("enum".to_string(), "x/Y".to_string()));
        let err = compare("c", &html, &extra).expect_err("documented has one more");
        assert_eq!(err.0.len(), 1);
        assert!(err.0[0].contains("documents enum x/Y"));
    }
}

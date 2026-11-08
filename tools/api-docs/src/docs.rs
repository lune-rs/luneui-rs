//! Doc comments: kept verbatim, except that their headings sit below the reference's own and
//! their intra-doc links point at the pages that document the targets.

/// A fence line opens or closes a fenced code block: the fence character and its length.
fn fence(line: &str) -> Option<(char, usize)> {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() > 3 {
        return None;
    }
    let c = t.chars().next()?;
    if c != '`' && c != '~' {
        return None;
    }
    let n = t.chars().take_while(|x| *x == c).count();
    (n >= 3).then_some((c, n))
}

/// Splits `docs` into runs of lines that are inside a fenced code block (`true`) and outside it.
fn segments(docs: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut open: Option<(char, usize)> = None;
    for line in docs.split_inclusive('\n') {
        let f = fence(line.trim_end_matches('\n'));
        let in_code = match (open, f) {
            (None, Some(fc)) => {
                open = Some(fc);
                true
            }
            (Some((c, n)), Some((c2, n2))) if c == c2 && n2 >= n && is_bare_fence(line) => {
                open = None;
                true
            }
            (Some(_), _) => true,
            (None, None) => false,
        };
        match out.last_mut() {
            Some((k, s)) if *k == in_code => s.push_str(line),
            _ => out.push((in_code, line.to_string())),
        }
    }
    out
}

/// A closing fence has nothing after the fence characters but blanks.
fn is_bare_fence(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && (t.chars().all(|c| c == '`') || t.chars().all(|c| c == '~'))
}

/// Moves every heading (outside code) down `by` levels, to at most level 6.
pub(crate) fn demote(docs: &str, by: usize) -> String {
    let mut out = String::with_capacity(docs.len());
    for (code, text) in segments(docs) {
        if code {
            out.push_str(&text);
            continue;
        }
        for line in text.split_inclusive('\n') {
            let t = line.trim_start_matches(' ');
            let hashes = t.chars().take_while(|c| *c == '#').count();
            let heading = line.len() - t.len() <= 3
                && (1..=6).contains(&hashes)
                && t[hashes..]
                    .chars()
                    .next()
                    .is_none_or(|c| c == ' ' || c == '\n' || c == '\r');
            if heading {
                out.push_str(&line[..line.len() - t.len()]);
                out.push_str(&"#".repeat((hashes + by).min(6)));
                out.push_str(&t[hashes..]);
            } else {
                out.push_str(line);
            }
        }
    }
    out
}

/// The first paragraph of `docs` on one line, cut at its first sentence end when that is within
/// 200 characters; empty when there are no docs.
pub(crate) fn summary(docs: &str) -> String {
    let mut para = String::new();
    for line in docs.lines() {
        if line.trim().is_empty() {
            if para.is_empty() {
                continue;
            }
            break;
        }
        if fence(line).is_some() && para.is_empty() {
            break;
        }
        if !para.is_empty() {
            para.push(' ');
        }
        para.push_str(line.trim());
    }
    let cut = para.find(". ").map_or(para.len(), |i| i + 1);
    let mut s = para[..cut].to_string();
    if s.chars().count() > 200 {
        s = s.chars().take(200).collect::<String>() + "...";
    }
    s.replace('|', "\\|")
}

/// Rewrites intra-doc links outside code. `resolve` maps the key rustdoc recorded for a link (the
/// destination as written, or the link text for a shortcut link) to a URL; `None` leaves the
/// link as written.
pub(crate) fn rewrite_links(docs: &str, resolve: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(docs.len());
    for (code, text) in segments(docs) {
        if code {
            out.push_str(&text);
        } else {
            out.push_str(&rewrite_prose(&definitions(&text, resolve), resolve));
        }
    }
    out
}

/// `[label]: destination` lines whose destination resolves.
fn definitions(text: &str, resolve: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let t = line.trim_start_matches(' ');
        if line.len() - t.len() <= 3
            && t.starts_with('[')
            && let Some(close) = t.find("]:")
        {
            let rest = &t[close + 2..];
            let dest_start = rest.len() - rest.trim_start().len();
            let dest: &str = rest.split_whitespace().next().unwrap_or("");
            if !dest.is_empty()
                && let Some(url) = resolve(dest)
            {
                out.push_str(&line[..line.len() - t.len() + close + 2 + dest_start]);
                out.push_str(&url);
                out.push_str(&rest[dest_start + dest.len()..]);
                continue;
            }
        }
        out.push_str(line);
    }
    out
}

fn rewrite_prose(text: &str, resolve: &dyn Fn(&str) -> Option<String>) -> String {
    let b = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'`' => {
                let n = b[i..].iter().take_while(|c| **c == b'`').count();
                match find_run(b, i + n, n) {
                    Some(end) => {
                        out.push_str(&text[i..end]);
                        i = end;
                    }
                    None => {
                        out.push_str(&text[i..i + n]);
                        i += n;
                    }
                }
            }
            b'[' => match link_at(text, i, resolve) {
                Some((replacement, end)) => {
                    out.push_str(&replacement);
                    i = end;
                }
                None => {
                    out.push('[');
                    i += 1;
                }
            },
            _ => {
                // Copy a whole character.
                let ch_len = text[i..].chars().next().map_or(1, char::len_utf8);
                out.push_str(&text[i..i + ch_len]);
                i += ch_len;
            }
        }
    }
    out
}

/// The end of the next run of exactly `n` backticks at or after `from`.
fn find_run(b: &[u8], from: usize, n: usize) -> Option<usize> {
    let mut i = from;
    while i < b.len() {
        if b[i] == b'`' {
            let run = b[i..].iter().take_while(|c| **c == b'`').count();
            if run == n {
                return Some(i + run);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

/// The index of the `]` that closes the `[` at `open`, skipping code spans inside.
fn close_bracket(text: &str, open: usize) -> Option<usize> {
    let b = text.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'`' => {
                let n = b[i..].iter().take_while(|c| **c == b'`').count();
                i = find_run(b, i + n, n).unwrap_or(i + n);
                continue;
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'\n' if i + 1 < b.len() && b[i + 1] == b'\n' => return None,
            _ => {}
        }
        i += 1;
    }
    None
}

/// A link starting at the `[` at `open`: the text to write and the index after what it replaces.
fn link_at(
    text: &str,
    open: usize,
    resolve: &dyn Fn(&str) -> Option<String>,
) -> Option<(String, usize)> {
    let close = close_bracket(text, open)?;
    let label = &text[open + 1..close];
    let b = text.as_bytes();
    match b.get(close + 1) {
        Some(b'(') => {
            let end = close + 1 + text[close + 1..].find(')')?;
            let dest_full = &text[close + 2..end];
            let dest = dest_full.split_whitespace().next()?;
            let url = resolve(dest)?;
            let title = dest_full[dest_full.find(dest)? + dest.len()..].to_string();
            Some((format!("[{label}]({url}{title})"), end + 1))
        }
        Some(b'[') => {
            let end = close + 1 + text[close + 1..].find(']')?;
            let l2 = &text[close + 2..end];
            let key = if l2.is_empty() { label } else { l2 };
            let url = resolve(key)?;
            Some((format!("[{label}]({url})"), end + 1))
        }
        _ => {
            let url = resolve(label)?;
            Some((format!("[{label}]({url})"), close + 1))
        }
    }
}

/// Indents every non-empty line by `n` spaces (for docs inside a list item).
pub(crate) fn indent(text: &str, n: usize) -> String {
    let pad = " ".repeat(n);
    let mut out = String::with_capacity(text.len() + 16);
    for line in text.split_inclusive('\n') {
        if line.trim().is_empty() {
            out.push_str(line.trim_end_matches([' ', '\t']));
        } else {
            out.push_str(&pad);
            out.push_str(line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_move_down_but_code_stays() {
        let docs =
            "# One\ntext\n```\n# not a heading\n```\n## Two\n    # indented code\n####### seven\n";
        let got = demote(docs, 2);
        assert_eq!(
            got,
            "### One\ntext\n```\n# not a heading\n```\n#### Two\n    # indented code\n####### seven\n"
        );
        assert_eq!(demote("#### deep\n", 3), "###### deep\n");
    }

    #[test]
    fn a_longer_fence_is_closed_only_by_a_fence_as_long() {
        let docs = "````\n```\n# x\n```\n````\n# Real\n";
        assert_eq!(demote(docs, 1), "````\n```\n# x\n```\n````\n## Real\n");
    }

    fn resolver(key: &str) -> Option<String> {
        match key {
            "`Foo`" => Some("a.md#struct.Foo".into()),
            "Foo::bar" => Some("a.md#method.Foo.bar".into()),
            "crate::Foo" => Some("a.md#struct.Foo".into()),
            _ => None,
        }
    }

    #[test]
    fn links_are_rewritten_outside_code_only() {
        let docs = "See [`Foo`] and [the bar](Foo::bar), `[`Foo`]` stays.\n```\n[`Foo`]\n```\n[x]: crate::Foo\n[`Nope`] and [web](https://x.y)\n";
        let got = rewrite_links(docs, &resolver);
        assert_eq!(
            got,
            "See [`Foo`](a.md#struct.Foo) and [the bar](a.md#method.Foo.bar), `[`Foo`]` stays.\n```\n[`Foo`]\n```\n[x]: a.md#struct.Foo\n[`Nope`] and [web](https://x.y)\n"
        );
    }

    #[test]
    fn a_link_text_can_wrap_over_a_line() {
        let got = rewrite_links("a [`Foo`][] b [the\nbar](Foo::bar)\n", &|k| resolver(k));
        assert_eq!(
            got,
            "a [`Foo`](a.md#struct.Foo) b [the\nbar](a.md#method.Foo.bar)\n"
        );
    }

    #[test]
    fn summary_is_the_first_sentence() {
        assert_eq!(summary("First one. Second.\n\nOther para.\n"), "First one.");
        assert_eq!(summary("line a\nline b\n\nrest"), "line a line b");
        assert_eq!(summary(""), "");
    }

    #[test]
    fn indent_skips_blank_lines() {
        assert_eq!(indent("a\n\nb\n", 2), "  a\n\n  b\n");
    }
}

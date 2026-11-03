//! The text seam's fake and the contract every text system meets.

use crate::text::{Spec, Style, TextKey, TextSystem};

/// A deterministic [`TextSystem`] with no fonts, for tests.
///
/// - key: `TextKey(spec.cache_key(scale))`, so it matches any real system's identity rule;
/// - width: the sum over runs of char count × size × 0.5 (logical px);
/// - with `wrap: Some(w)`, `w > 0` and a width above `w`: `ceil(width / w)` lines and width `w`;
///   otherwise one line;
/// - height: lines × the first run's line height, or 17.0 with no runs (as the shaping cache
///   gives an empty spec).
///
/// Never panics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FakeText;

impl TextSystem for FakeText {
    fn prepare(&mut self, spec: &Spec, scale: f32) -> (TextKey, (f32, f32)) {
        let key = TextKey(spec.cache_key(scale));
        let mut width: f32 = spec
            .runs
            .iter()
            .map(|(text, style)| text.chars().count() as f32 * style.size * 0.5)
            .sum();
        let mut lines = 1.0;
        if let Some(w) = spec.wrap
            && w > 0.0
            && width > w
        {
            lines = (width / w).ceil();
            width = w;
        }
        let height = spec.runs.first().map_or(17.0, |(_, st)| lines * st.line);
        (key, (width, height))
    }
}

fn check(ok: bool, property: &str, detail: impl FnOnce() -> String) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(format!("{property}: {}", detail()))
    }
}

fn finite_size(property: &str, size: (f32, f32)) -> Result<(), String> {
    check(
        size.0.is_finite() && size.1.is_finite() && size.0 >= 0.0 && size.1 >= 0.0,
        property,
        || format!("size {size:?} is not finite and non-negative"),
    )
}

/// Checks the properties every [`TextSystem`] must have; the error names the first that fails.
///
/// - P1: preparing the same spec at the same scale twice gives the same key and size;
/// - P2: the same spec at scales 1 and 2 gives different keys;
/// - P3: different text gives different keys;
/// - P4: `measure` equals `prepare`'s size;
/// - P5: an empty spec (no runs) has width 0 and a finite height ≥ 0;
/// - P6: "aa bb cc dd ee ff gg" at `Style::ui(13.0, …)` wrapped at 40 is at most 41 wide;
/// - P7: every size is finite and ≥ 0.
pub fn text_system_contract(ts: &mut dyn TextSystem) -> Result<(), String> {
    let white = [255; 4];
    let samples = [
        Spec::plain("Hello", Style::ui(13.0, white)),
        Spec::plain("Hello world", Style::ui(15.0, white).bold()),
        Spec::plain("fn main() {}", Style::mono(12.0, 16.0, white)),
        Spec {
            runs: vec![
                ("Two ".to_owned(), Style::ui(13.0, white)),
                ("runs".to_owned(), Style::ui(13.0, white).medium()),
            ],
            wrap: None,
        },
        Spec::plain("A longer line that has to wrap", Style::ui(13.0, white)).wrapped(80.0),
    ];
    let mut sizes = Vec::new();

    for spec in &samples {
        let (k1, s1) = ts.prepare(spec, 1.0);
        let (k2, s2) = ts.prepare(spec, 1.0);
        check(k1 == k2 && s1 == s2, "P1 same spec and scale", || {
            format!("{k1:?} {s1:?} then {k2:?} {s2:?}")
        })?;
        sizes.push(s1);
    }

    for spec in &samples {
        let (k1, _) = ts.prepare(spec, 1.0);
        let (k2, s2) = ts.prepare(spec, 2.0);
        check(k1 != k2, "P2 scales 1 and 2", || format!("both {k1:?}"))?;
        sizes.push(s2);
    }

    let a = Spec::plain("alpha", Style::ui(13.0, white));
    let b = Spec::plain("bravo", Style::ui(13.0, white));
    let (ka, _) = ts.prepare(&a, 1.0);
    let (kb, _) = ts.prepare(&b, 1.0);
    check(ka != kb, "P3 different text", || format!("both {ka:?}"))?;

    for spec in &samples {
        for scale in [1.0, 2.0] {
            let (_, prepared) = ts.prepare(spec, scale);
            let measured = ts.measure(spec, scale);
            check(measured == prepared, "P4 measure equals prepare", || {
                format!("measure {measured:?}, prepare {prepared:?} at scale {scale}")
            })?;
        }
    }

    let empty = Spec {
        runs: Vec::new(),
        wrap: None,
    };
    let (_, (w, h)) = ts.prepare(&empty, 1.0);
    check(
        w == 0.0 && h.is_finite() && h >= 0.0,
        "P5 empty spec",
        || format!("size ({w}, {h})"),
    )?;
    sizes.push((w, h));

    let wrapped = Spec::plain("aa bb cc dd ee ff gg", Style::ui(13.0, white)).wrapped(40.0);
    for scale in [1.0, 2.0] {
        let (_, size) = ts.prepare(&wrapped, scale);
        check(size.0 <= 41.0, "P6 wrap at 40", || {
            format!("width {} at scale {scale}", size.0)
        })?;
        sizes.push(size);
    }

    for size in sizes {
        finite_size("P7 finite sizes", size)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_text_meets_the_contract() {
        assert_eq!(text_system_contract(&mut FakeText), Ok(()));
    }

    #[test]
    fn fake_text_sizes_follow_its_formula() {
        let style = Style::ui(10.0, [255; 4]);
        let mut t = FakeText;
        assert_eq!(t.prepare(&Spec::plain("abcd", style), 1.0).1, (20.0, 13.0));
        assert_eq!(
            t.prepare(&Spec::plain("abcd", style).wrapped(8.0), 1.0).1,
            (8.0, 39.0)
        );
        let empty = Spec {
            runs: Vec::new(),
            wrap: Some(0.0),
        };
        assert_eq!(t.prepare(&empty, 2.0).1, (0.0, 17.0));
    }
}

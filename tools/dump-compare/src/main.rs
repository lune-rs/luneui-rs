//! Compares two directories of PNG frame dumps pixel by pixel.
//!
//! `cargo run -p dump-compare -- <baseline-dir> <candidate-dir> [--max-delta N] [--allow-new]`
//!
//! Prints, per PNG, "identical" or the max per-channel delta (alpha included) and the
//! differing-pixel count, then `N files, M identical, K differ, max delta D`. Exits 1 on a file
//! missing from the candidate, a file only in the candidate (unless `--allow-new`, which prints
//! `<name>: NEW in candidate` and carries on), a size or colour-type mismatch, or any delta above
//! `N` (default 0).

use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

struct Image {
    info: (u32, u32, png::ColorType, png::BitDepth),
    bytes: Vec<u8>,
}

/// What a comparison printed and whether it passed. `lines` ends with the summary line.
struct Report {
    lines: Vec<String>,
    ok: bool,
}

fn load(path: &Path) -> Result<Image, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| format!("decode {}: {e}", path.display()))?;
    let mut bytes = vec![0; reader.output_buffer_size().ok_or("image too large")?];
    let frame = reader
        .next_frame(&mut bytes)
        .map_err(|e| format!("decode {}: {e}", path.display()))?;
    bytes.truncate(frame.buffer_size());
    Ok(Image {
        info: (frame.width, frame.height, frame.color_type, frame.bit_depth),
        bytes,
    })
}

fn names(dir: &Path) -> Result<BTreeSet<String>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".png"))
        .collect())
}

fn compare(base: &Path, cand: &Path, max_delta: u8, allow_new: bool) -> Result<Report, String> {
    let (a_names, b_names) = (names(base)?, names(cand)?);
    let mut lines = Vec::new();
    let mut ok = true;
    for n in a_names.difference(&b_names) {
        lines.push(format!("{n}: MISSING in candidate"));
        ok = false;
    }
    for n in b_names.difference(&a_names) {
        if allow_new {
            lines.push(format!("{n}: NEW in candidate"));
        } else {
            lines.push(format!("{n}: EXTRA in candidate"));
            ok = false;
        }
    }
    let (mut files, mut identical, mut differ, mut worst) = (0, 0, 0, 0_u8);
    for n in a_names.intersection(&b_names) {
        files += 1;
        let (a, b) = (load(&base.join(n))?, load(&cand.join(n))?);
        if a.info != b.info || a.bytes.len() != b.bytes.len() {
            lines.push(format!(
                "{n}: MISMATCH size/colour type {:?} vs {:?}",
                a.info, b.info
            ));
            ok = false;
            differ += 1;
            continue;
        }
        let channels = a.bytes.len() / (a.info.0 as usize * a.info.1 as usize).max(1);
        let (mut delta, mut pixels) = (0_u8, 0_usize);
        for (pa, pb) in a
            .bytes
            .chunks(channels.max(1))
            .zip(b.bytes.chunks(channels.max(1)))
        {
            let d = pa
                .iter()
                .zip(pb)
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            if d > 0 {
                pixels += 1;
                delta = delta.max(d);
            }
        }
        if delta == 0 {
            identical += 1;
            lines.push(format!("{n}: identical"));
        } else {
            differ += 1;
            worst = worst.max(delta);
            lines.push(format!("{n}: max delta {delta}, {pixels} px differ"));
            if delta > max_delta {
                ok = false;
            }
        }
    }
    lines.push(format!(
        "{files} files, {identical} identical, {differ} differ, max delta {worst}"
    ));
    Ok(Report { lines, ok })
}

fn run() -> Result<bool, String> {
    let mut args = std::env::args().skip(1);
    let (mut dirs, mut max_delta, mut allow_new) = (Vec::new(), 0_u8, false);
    while let Some(a) = args.next() {
        if a == "--max-delta" {
            max_delta = args
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or("--max-delta needs a number 0-255")?;
        } else if a == "--allow-new" {
            allow_new = true;
        } else {
            dirs.push(a);
        }
    }
    let [base, cand] = dirs.as_slice() else {
        return Err(
            "usage: dump-compare <baseline-dir> <candidate-dir> [--max-delta N] [--allow-new]"
                .into(),
        );
    };
    let report = compare(Path::new(base), Path::new(cand), max_delta, allow_new)?;
    for line in &report.lines {
        println!("{line}");
    }
    Ok(report.ok)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("dump-compare: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A fresh scratch dir with a baseline and a candidate subdir, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(test: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("dump-compare-test-{}-{test}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("base")).expect("create base dir");
            std::fs::create_dir_all(root.join("cand")).expect("create cand dir");
            Self(root)
        }
        fn base(&self) -> PathBuf {
            self.0.join("base")
        }
        fn cand(&self) -> PathBuf {
            self.0.join("cand")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Writes a w x h RGBA8 PNG of one colour, with `px0` replacing pixel 0 when given.
    fn write_png(dir: &Path, name: &str, w: u32, h: u32, px0: Option<[u8; 4]>) {
        let mut data = Vec::new();
        for _ in 0..w * h {
            data.extend_from_slice(&[10, 20, 30, 255]);
        }
        if let Some(p) = px0 {
            data[..4].copy_from_slice(&p);
        }
        let file = std::fs::File::create(dir.join(name)).expect("create png");
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().expect("png header");
        writer.write_image_data(&data).expect("png data");
    }

    fn summary(r: &Report) -> &str {
        r.lines.last().map_or("", String::as_str)
    }

    #[test]
    fn identical_pair_is_ok() {
        let s = Scratch::new("identical");
        write_png(&s.base(), "a.png", 2, 2, None);
        write_png(&s.cand(), "a.png", 2, 2, None);
        let r = compare(&s.base(), &s.cand(), 0, false).expect("compare");
        assert!(r.ok);
        assert_eq!(r.lines[0], "a.png: identical");
        assert_eq!(summary(&r), "1 files, 1 identical, 0 differ, max delta 0");
    }

    #[test]
    fn alpha_only_delta_fails_and_is_reported() {
        let s = Scratch::new("alpha");
        write_png(&s.base(), "a.png", 2, 2, None);
        write_png(&s.cand(), "a.png", 2, 2, Some([10, 20, 30, 252]));
        let r = compare(&s.base(), &s.cand(), 0, false).expect("compare");
        assert!(!r.ok);
        assert_eq!(r.lines[0], "a.png: max delta 3, 1 px differ");
        assert_eq!(summary(&r), "1 files, 0 identical, 1 differ, max delta 3");
    }

    #[test]
    fn delta_within_max_delta_is_ok() {
        let s = Scratch::new("tolerated");
        write_png(&s.base(), "a.png", 2, 2, None);
        write_png(&s.cand(), "a.png", 2, 2, Some([10, 20, 30, 252]));
        let r = compare(&s.base(), &s.cand(), 3, false).expect("compare");
        assert!(r.ok);
        assert_eq!(r.lines[0], "a.png: max delta 3, 1 px differ");
    }

    #[test]
    fn size_mismatch_fails() {
        let s = Scratch::new("size");
        write_png(&s.base(), "a.png", 2, 2, None);
        write_png(&s.cand(), "a.png", 3, 2, None);
        let r = compare(&s.base(), &s.cand(), 255, true).expect("compare");
        assert!(!r.ok);
        assert!(r.lines[0].starts_with("a.png: MISMATCH size/colour type"));
        assert_eq!(summary(&r), "1 files, 0 identical, 1 differ, max delta 0");
    }

    #[test]
    fn missing_in_candidate_fails_with_or_without_allow_new() {
        for allow_new in [false, true] {
            let s = Scratch::new(&format!("missing-{allow_new}"));
            write_png(&s.base(), "a.png", 2, 2, None);
            let r = compare(&s.base(), &s.cand(), 0, allow_new).expect("compare");
            assert!(!r.ok, "allow_new={allow_new}");
            assert_eq!(r.lines[0], "a.png: MISSING in candidate");
        }
    }

    #[test]
    fn extra_in_candidate_fails_unless_allow_new() {
        let s = Scratch::new("extra");
        write_png(&s.cand(), "a.png", 2, 2, None);
        let r = compare(&s.base(), &s.cand(), 0, false).expect("compare");
        assert!(!r.ok);
        assert_eq!(r.lines[0], "a.png: EXTRA in candidate");
        let r = compare(&s.base(), &s.cand(), 0, true).expect("compare");
        assert!(r.ok);
        assert_eq!(r.lines[0], "a.png: NEW in candidate");
        assert_eq!(summary(&r), "0 files, 0 identical, 0 differ, max delta 0");
    }

    #[test]
    fn empty_dirs_compare_zero_files_ok() {
        let s = Scratch::new("empty");
        let r = compare(&s.base(), &s.cand(), 0, false).expect("compare");
        assert!(r.ok);
        assert_eq!(r.lines, ["0 files, 0 identical, 0 differ, max delta 0"]);
    }
}

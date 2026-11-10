//! `GlyphRasterizer` against cosmic-text's own images and the stem-darkening geometry, on the
//! gallery's two text fonts.

use cosmic_text::fontdb::{Database, ID};
use cosmic_text::{CacheKey, CacheKeyFlags, FontSystem, SwashCache, SwashImage, Weight};
use lune_ui_core::text::GlyphParams;
use lune_ui_wgpu::raster::GlyphRasterizer;
use swash::scale::Source;

const SANS: &[u8] = include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf");
const MONO: &[u8] =
    include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf");
/// A codicon in the mono font's Nerd Font range.
const CODICON: char = '\u{ea60}';

struct Font {
    name: &'static str,
    fonts: FontSystem,
    id: ID,
}

fn font(name: &'static str, data: &[u8]) -> Font {
    let mut db = Database::new();
    db.load_font_data(data.to_vec());
    let id = db.faces().next().expect("the font loads").id;
    Font {
        name,
        fonts: FontSystem::new_with_locale_and_db("en-US".to_owned(), db),
        id,
    }
}

fn both() -> [Font; 2] {
    [font("RznSans", SANS), font("RobotoMono", MONO)]
}

impl Font {
    fn key(&mut self, ch: char, size: f32, x: f32, flags: CacheKeyFlags) -> CacheKey {
        let glyph = self
            .fonts
            .get_font(self.id, Weight::NORMAL)
            .expect("the face is in the system")
            .as_swash()
            .charmap()
            .map(ch);
        assert_ne!(glyph, 0, "{}: no glyph for {ch:?}", self.name);
        CacheKey::new(self.id, glyph, size, (x, 0.0), Weight::NORMAL, flags).0
    }

    fn image(&mut self, r: &mut GlyphRasterizer, key: CacheKey, p: GlyphParams) -> SwashImage {
        r.image(&mut self.fonts, key, p)
            .expect("the glyph rasterizes")
    }
}

fn place(img: &SwashImage) -> (i32, i32, u32, u32) {
    let p = img.placement;
    (p.left, p.top, p.width, p.height)
}

fn same(a: &SwashImage, b: &SwashImage) -> bool {
    place(a) == place(b) && a.content == b.content && a.data == b.data
}

/// Ink-weighted centre (x, y up) and the total coverage in pixels.
fn centroid(img: &SwashImage) -> (f64, f64, f64) {
    let (w, h) = (img.placement.width as usize, img.placement.height as usize);
    let (mut sx, mut sy, mut s) = (0.0, 0.0, 0.0);
    for r in 0..h {
        for c in 0..w {
            let a = f64::from(img.data[r * w + c]) / 255.0;
            sx += (f64::from(img.placement.left) + c as f64 + 0.5) * a;
            sy += (f64::from(img.placement.top) - r as f64 - 0.5) * a;
            s += a;
        }
    }
    (sx / s, sy / s, s)
}

fn row_sum(img: &SwashImage, row: usize) -> f64 {
    let w = img.placement.width as usize;
    img.data[row * w..(row + 1) * w]
        .iter()
        .map(|&a| f64::from(a) / 255.0)
        .sum()
}

const FLAGS: [CacheKeyFlags; 2] = [CacheKeyFlags::empty(), CacheKeyFlags::FAKE_ITALIC];

#[test]
fn t1_off_is_cosmic_texts_image() {
    let mut r = GlyphRasterizer::new();
    for mut f in both() {
        let mut chars = vec!['g', 'W'];
        if f.name == "RobotoMono" {
            chars.push(CODICON);
        }
        for ch in chars {
            for size in [13.0, 26.0, 100.0] {
                for x in [0.0, 0.3, 0.7] {
                    for flags in FLAGS {
                        let key = f.key(ch, size, x, flags);
                        let want = SwashCache::new()
                            .get_image_uncached(&mut f.fonts, key)
                            .expect("cosmic rasterizes");
                        let got = f.image(&mut r, key, GlyphParams::OFF);
                        assert!(same(&got, &want), "{} {ch:?} {size} x{x} {flags:?}", f.name);
                    }
                }
            }
        }
    }
}

#[test]
fn t2_zero_dilation_through_the_outline_path_is_cosmic_texts_unhinted_image() {
    let mut r = GlyphRasterizer::new();
    let params = GlyphParams::new(false, [0.0, 0.0]);
    assert_ne!(params, GlyphParams::OFF);
    for mut f in both() {
        for ch in ['l', 'g', 'W', 'a', 'I'] {
            for size in [13.0, 26.0, 100.0] {
                for x in [0.0, 0.3, 0.5, 0.7] {
                    for flags in FLAGS {
                        let key = f.key(ch, size, x, flags);
                        let mut unhinted = key;
                        unhinted.flags |= CacheKeyFlags::DISABLE_HINTING;
                        let want = SwashCache::new()
                            .get_image_uncached(&mut f.fonts, unhinted)
                            .expect("cosmic rasterizes");
                        assert!(matches!(want.source, Source::Outline));
                        let got = f.image(&mut r, key, params);
                        assert!(matches!(got.source, Source::Outline));
                        assert!(same(&got, &want), "{} {ch:?} {size} x{x} {flags:?}", f.name);
                    }
                }
            }
        }
    }
}

#[test]
fn t3_a_stem_gains_twice_the_dilation() {
    let mut r = GlyphRasterizer::new();
    for mut f in both() {
        for (size, d) in [(26.0, [0.3, 0.3]), (13.0, [0.1573, 0.196625])] {
            let key = f.key('I', size, 0.0, CacheKeyFlags::empty());
            let z = f.image(&mut r, key, GlyphParams::new(false, [0.0, 0.0]));
            let g = f.image(&mut r, key, GlyphParams::new(false, d));
            // The same glyph-space row in both images: half way up the undilated glyph.
            let ymid = f64::from(z.placement.top) - f64::from(z.placement.height) / 2.0;
            let rz = (f64::from(z.placement.top) - ymid - 0.5).round() as usize;
            let rg = (f64::from(g.placement.top) - ymid - 0.5).round() as usize;
            let gain = row_sum(&g, rg) - row_sum(&z, rz);
            let want = 2.0 * f64::from(d[0]);
            assert!(
                (gain - want).abs() <= 0.02,
                "{} I {size}: stem gained {gain:.4}, want {want:.4}",
                f.name
            );
        }
    }
}

#[test]
fn t4_a_dilated_glyph_stays_centred() {
    let mut r = GlyphRasterizer::new();
    for mut f in both() {
        for ch in ['o', 'H', 'O'] {
            for hinting in [true, false] {
                let key = f.key(ch, 26.0, 0.0, CacheKeyFlags::empty());
                let z = f.image(&mut r, key, GlyphParams::new(hinting, [0.0, 0.0]));
                let g = f.image(&mut r, key, GlyphParams::new(hinting, [0.3, 0.3]));
                let (cz, cg) = (centroid(&z), centroid(&g));
                let what = format!("{} {ch:?} hinting {hinting}", f.name);
                assert!(
                    (cg.0 - cz.0).abs() < 0.03,
                    "{what}: x moved {}",
                    cg.0 - cz.0
                );
                assert!(
                    (cg.1 - cz.1).abs() < 0.03,
                    "{what}: y moved {}",
                    cg.1 - cz.1
                );
                let (zl, zt, zw, zh) = place(&z);
                let (gl, gt, gw, gh) = place(&g);
                let (zr, zb) = (zl + zw as i32, zt - zh as i32);
                let (gr, gb) = (gl + gw as i32, gt - gh as i32);
                assert!(
                    gl <= zl && gt >= zt && gr >= zr && gb <= zb,
                    "{what}: box {:?} does not hold {:?}",
                    place(&g),
                    place(&z)
                );
            }
        }
    }
}

#[test]
fn t5_coverage_grows_with_the_dilation() {
    let mut r = GlyphRasterizer::new();
    for mut f in both() {
        let key = f.key('a', 26.0, 0.0, CacheKeyFlags::empty());
        let ink: Vec<f64> = [0.0, 0.15, 0.3]
            .into_iter()
            .map(|d| centroid(&f.image(&mut r, key, GlyphParams::new(false, [d, d]))).2)
            .collect();
        assert!(ink[0] < ink[1] && ink[1] < ink[2], "{}: {ink:?}", f.name);
    }
}

#[test]
fn t6_the_same_inputs_give_the_same_bytes() {
    let (mut one, mut two) = (GlyphRasterizer::new(), GlyphRasterizer::default());
    for mut f in both() {
        for ch in ['a', 'g', 'W'] {
            for params in [
                GlyphParams::OFF,
                GlyphParams::new(false, [0.3, 0.375]),
                GlyphParams::new(true, [0.1573, 0.196625]),
            ] {
                let key = f.key(ch, 13.0, 0.3, CacheKeyFlags::empty());
                let a = f.image(&mut one, key, params);
                let b = f.image(&mut one, key, params);
                let c = f.image(&mut two, key, params);
                assert!(same(&a, &b) && same(&a, &c), "{} {ch:?} {params:?}", f.name);
            }
        }
    }
}

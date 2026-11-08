//! Fakes of the toolkit's seams, for tests in this crate and in apps.
//!
//! - [`RecordingRenderer`]: the display-list seam's fake; records the draw calls a renderer
//!   issues for a [`Scene`](crate::scene::Scene), in the documented draw order.
//! - [`FakeText`]: the text seam's fake, deterministic sizes with no fonts; and
//!   [`text_system_contract`], the properties every [`TextSystem`](crate::text::TextSystem)
//!   must have.
//! - [`NullMaterial`]: the material seam's fake, a backdrop with no OS glass; and
//!   [`backdrop_contract`], the properties every [`Backdrop`](crate::backdrop::Backdrop) must have.
//! - [`FakeImages`]: the image seam's fake, a store that keeps sizes; and
//!   [`image_store_contract`], the properties every [`ImageStore`](crate::image::ImageStore) must have.
//! - [`ScriptedInput`]: the input seam's fake, a scripted window producing [`RawInput`](crate::input::RawInput)
//!   frames; and [`scripted_input_contract`], the properties its frames must have.
//! - [`UiHarness`]: a `Ui` test bench (store, fake text, default strings, scripted input); and
//!   [`disabled_violations`], the nodes under a root that a disabled scope left enabled.
//! - [`fill_shapes`] and [`fill_covers`]: the bounds of each path-filled shape of a colour, such
//!   as a text selection, and whether one covers a point.

mod backdrop;
mod images;
mod input;
mod recording;
mod text;
mod ui;

pub use backdrop::{NullMaterial, backdrop_contract};
pub use images::{FakeImages, image_store_contract};
pub use input::{ScriptedInput, scripted_input_contract};
pub use recording::{DrawCall, RecordedFrame, RecordingRenderer, flattened};
pub use text::{FakeText, SEGMENT_TABLE, SegmentCase, spec_line_contract, text_system_contract};
pub use ui::{HitRegion, UiHarness, disabled_violations, hit_regions};

/// The bounds of each filled shape of `color` (straight alpha, as a theme holds it) in
/// `geometry.fills`, in physical px, in drawn order: one per `Painter::fill_path`, `sector` or
/// `fill_joined` call, so one per joined text selection shape (`widgets::selection`). A shape's
/// bounds are its outline's, without the antialiasing fringe. Two shapes are told apart by their
/// layout in the list: a shape's interior triangles, then its fringe.
pub fn fill_shapes(
    geometry: &crate::scene::LayerGeometry,
    color: crate::color::Rgba,
) -> Vec<crate::paint::Rect> {
    let want = crate::color::premul(color);
    let tris = geometry
        .fills
        .chunks(3)
        .filter(|t| t.iter().all(|v| v.color == want));
    let mut out: Vec<[f32; 4]> = Vec::new();
    let mut in_fringe = true;
    for t in tris {
        let fringe = t.iter().any(|v| v.aa[0] != 0.0);
        if !fringe && in_fringe {
            out.push([f32::MAX, f32::MAX, f32::MIN, f32::MIN]);
        }
        in_fringe = fringe;
        let Some(b) = out.last_mut() else { continue };
        for v in t.iter().filter(|v| v.aa[0] == 0.0) {
            *b = [
                b[0].min(v.pos[0]),
                b[1].min(v.pos[1]),
                b[2].max(v.pos[0]),
                b[3].max(v.pos[1]),
            ];
        }
    }
    out.into_iter()
        .map(|b| crate::paint::Rect::new(b[0], b[1], b[2] - b[0], b[3] - b[1]))
        .collect()
}

/// Whether a path-filled shape of `color` (straight alpha) in `geometry.fills` covers the point
/// `at` (physical px) with its interior: the fringe does not count, and a point on an interior
/// edge does.
pub fn fill_covers(
    geometry: &crate::scene::LayerGeometry,
    color: crate::color::Rgba,
    at: [f32; 2],
) -> bool {
    let want = crate::color::premul(color);
    let side = |a: [f32; 2], b: [f32; 2]| {
        f64::from(b[0] - a[0]) * f64::from(at[1] - a[1])
            - f64::from(b[1] - a[1]) * f64::from(at[0] - a[0])
    };
    geometry
        .fills
        .chunks(3)
        .filter(|t| t.iter().all(|v| v.color == want && v.aa[0] == 0.0))
        .any(|t| {
            let (a, b, c) = (t[0].pos, t[1].pos, t[2].pos);
            let s = [side(a, b), side(b, c), side(c, a)];
            s.iter().all(|v| *v >= -1e-6) || s.iter().all(|v| *v <= 1e-6)
        })
}

//! The one selection routine (R18, D17): `selection::shapes` joins touching rows into one outline
//! with rounded outer and inner corners, and `selection::paint` fills it through
//! `Painter::fill_joined`, band by band under one fringe.

use lune_ui_core::paint::{Painter, Rect};
use lune_ui_core::scene::StrokeVertex;
use lune_ui_core::testing::FakeText;
use lune_ui_core::theme::DARK as THEME;
use lune_ui_core::widgets::selection::{self, Joined};

type P<'a> = Painter<'a, u8>;

const INK: [u8; 4] = [0x12, 0x34, 0x56, 0x66];
const R: f32 = 3.0;
const PAD: f32 = 2.0;

fn painter(text: &mut FakeText, scale: f32) -> P<'_> {
    Painter::new(
        scale,
        Rect::new(0.0, 0.0, 2000.0, 4000.0),
        text,
        None,
        THEME,
    )
}

/// The fills `rows` paints at scale 1.
fn painted(rows: &[Rect], open: (bool, bool)) -> Vec<StrokeVertex> {
    let mut text = FakeText;
    let mut p = painter(&mut text, 1.0);
    selection::paint(&mut p, rows, open, INK);
    p.scene.layers[0].geometry.fills.clone()
}

/// The triangles of `fills` with no fringe point: the interior.
fn interior(fills: &[StrokeVertex]) -> Vec<[[f32; 2]; 3]> {
    fills
        .chunks(3)
        .filter(|t| t.iter().all(|v| v.aa[0] == 0.0))
        .map(|t| [t[0].pos, t[1].pos, t[2].pos])
        .collect()
}

/// `[x0, y0, x1, y1]` of `pts`.
fn bounds(pts: impl IntoIterator<Item = [f32; 2]>) -> [f32; 4] {
    pts.into_iter().fold(
        [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
        |[a, b, c, d], [x, y]| [a.min(x), b.min(y), c.max(x), d.max(y)],
    )
}

fn area(p: &[[f32; 2]]) -> f64 {
    let n = p.len();
    (0..n)
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            f64::from(a[0]) * f64::from(b[1]) - f64::from(b[0]) * f64::from(a[1])
        })
        .sum::<f64>()
        / 2.0
}

fn tri_area(t: &[[f32; 2]; 3]) -> f64 {
    area(t).abs()
}

/// Whether `q` lies inside the loop `p` (even-odd), or within `eps` of an edge.
fn inside(p: &[[f32; 2]], q: [f32; 2], eps: f32) -> bool {
    let n = p.len();
    let mut odd = false;
    for i in 0..n {
        let (a, b) = (p[i], p[(i + 1) % n]);
        // Distance to the edge.
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 {
            (((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (cx, cy) = (a[0] + t * dx - q[0], a[1] + t * dy - q[1]);
        if (cx * cx + cy * cy).sqrt() <= eps {
            return true;
        }
        if (a[1] > q[1]) != (b[1] > q[1]) {
            let x = a[0] + (q[1] - a[1]) / (b[1] - a[1]) * dx;
            if q[0] < x {
                odd = !odd;
            }
        }
    }
    odd
}

/// The turn at each point of the loop `p`, signed so that the loop's own turn is positive.
fn turns(p: &[[f32; 2]]) -> Vec<f64> {
    let sign = area(p).signum();
    let n = p.len();
    (0..n)
        .map(|i| {
            let (a, b, c) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
            let cross = f64::from(b[0] - a[0]) * f64::from(c[1] - b[1])
                - f64::from(b[1] - a[1]) * f64::from(c[0] - b[0]);
            cross * sign
        })
        .collect()
}

/// Whether segments `a`-`b` and `c`-`d` cross at a point inside both.
fn cross_proper(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> bool {
    let o = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| {
        f64::from(q[0] - p[0]) * f64::from(r[1] - p[1])
            - f64::from(q[1] - p[1]) * f64::from(r[0] - p[0])
    };
    let (d1, d2) = (o(a, b, c), o(a, b, d));
    let (d3, d4) = (o(c, d, a), o(c, d, b));
    d1 * d2 < -1e-9 && d3 * d4 < -1e-9
}

/// No two edges of the loop `p` cross.
fn assert_simple(p: &[[f32; 2]], what: &str) {
    let n = p.len();
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (p[i], p[(i + 1) % n]);
            let (c, d) = (p[j], p[(j + 1) % n]);
            assert!(!cross_proper(a, b, c, d), "{what}: edges {i} and {j} cross");
        }
    }
}

/// The shapes of `rows` are the true picture: every loop is simple, and away from the corners
/// (farther than the radius from any row end's corner) a point is inside a shape exactly when it
/// is inside a padded row. An independent model, not the outline checked against itself.
fn assert_true_shape(rows: &[Rect], shapes: &[Joined]) {
    for (k, s) in shapes.iter().enumerate() {
        assert_simple(&s.outline, &format!("shape {k}"));
        for (i, part) in s.parts.iter().enumerate() {
            assert_simple(part, &format!("shape {k} part {i}"));
        }
    }
    let padded: Vec<Rect> = rows
        .iter()
        .map(|r| Rect::new(r.x - PAD, r.y, r.w + 2.0 * PAD, r.h))
        .collect();
    let near_corner = |q: [f32; 2]| {
        padded.iter().any(|r| {
            [r.x, r.right()].iter().any(|x| (q[0] - x).abs() <= R + 0.5)
                && [r.y, r.bottom()]
                    .iter()
                    .any(|y| (q[1] - y).abs() <= R + 0.5)
        })
    };
    let b = bounds(
        padded
            .iter()
            .flat_map(|r| [[r.x, r.y], [r.right(), r.bottom()]]),
    );
    // About 40 000 samples, never coarser than needed to pass between corners.
    let step = (((b[2] - b[0]) * (b[3] - b[1])) / 40_000.0)
        .sqrt()
        .clamp(0.75, 2.9);
    let mut seen = (0, 0);
    let mut y = b[1] - 2.0 + 0.13;
    while y <= b[3] + 2.0 {
        let mut x = b[0] - 2.0 + 0.07;
        while x <= b[2] + 2.0 {
            let q = [x, y];
            if !near_corner(q) {
                let in_rows = padded
                    .iter()
                    .any(|r| x > r.x && x < r.right() && y > r.y && y < r.bottom());
                let in_shape = shapes.iter().any(|s| inside(&s.outline, q, 0.0));
                assert_eq!(in_rows, in_shape, "at {q:?}");
                if in_rows {
                    seen.0 += 1;
                } else {
                    seen.1 += 1;
                }
            }
            x += step;
        }
        y += step;
    }
    assert!(
        seen.0 > 0 && seen.1 > 0,
        "the sampler saw both sides: {seen:?}"
    );
}

fn one(rows: &[Rect], open: (bool, bool)) -> Joined {
    let mut s = selection::shapes(rows, open, R, PAD);
    assert_eq!(s.len(), 1, "one shape");
    s.remove(0)
}

#[test]
fn one_row_is_the_row_and_its_padding_with_rounded_corners() {
    let row = Rect::new(100.0, 40.0, 60.0, 18.0);
    let fills = painted(&[row], (false, false));
    let tris = interior(&fills);
    assert!(!tris.is_empty(), "the instrument sees the interior");
    let b = bounds(tris.iter().flatten().copied());
    let want = [98.0, 40.0, 162.0, 58.0];
    for (got, want) in b.iter().zip(want) {
        assert!((got - want).abs() < 1e-3, "{b:?} vs {want:?}");
    }
    // Rounded: no point at the padded rect's corners.
    let s = one(&[row], (false, false));
    for c in [[98.0, 40.0], [162.0, 40.0], [162.0, 58.0], [98.0, 58.0]] {
        assert!(!s.outline.contains(&c), "{c:?} is cut");
    }
    // The colour is the ink, premultiplied.
    assert!(
        fills
            .iter()
            .all(|v| v.color == lune_ui_core::color::premul(INK))
    );
}

#[test]
fn two_rows_the_second_wider_are_one_shape_with_a_concave_step() {
    let rows = [
        Rect::new(100.0, 0.0, 40.0, 20.0),
        Rect::new(100.0, 20.0, 120.0, 20.0),
    ];
    let s = one(&rows, (false, false));
    assert_true_shape(&rows, std::slice::from_ref(&s));
    assert_eq!(s.parts.len(), 2, "a band per row");
    // A concave turn where the second row sticks out.
    let t = turns(&s.outline);
    let concave = s
        .outline
        .iter()
        .zip(&t)
        .filter(|(_, t)| **t < -1e-6)
        .map(|(q, _)| *q)
        .collect::<Vec<_>>();
    assert!(!concave.is_empty(), "a concave corner");
    assert!(
        concave
            .iter()
            .all(|q| q[0] >= 142.0 - 1e-3 && q[0] <= 142.0 + R && (q[1] - 20.0).abs() <= R),
        "the fillet sits in the step's corner: {concave:?}"
    );
    // The area is the padded rows' union within the corners' area.
    let union = 44.0 * 20.0 + 124.0 * 20.0;
    let corners = 5.0 * f64::from(R * R) * (1.0 - std::f64::consts::FRAC_PI_4);
    let got = area(&s.outline).abs();
    assert!((got - union).abs() <= corners + 0.5, "{got} vs {union}");
    // The parts tile the outline: same total area.
    let parts: f64 = s.parts.iter().map(|p| area(p).abs()).sum();
    assert!((parts - got).abs() < 0.5, "{parts} vs {got}");
    // One fringe loop: the fringe of the outline alone (6 vertices per outline point).
    let fills = painted(&rows, (false, false));
    let fringe = fills.len() - interior(&fills).len() * 3;
    let mut text = FakeText;
    let mut p = painter(&mut text, 1.0);
    p.fill_path(&s.outline, INK);
    let alone = p.scene.layers[0].geometry.fills.len()
        - interior(&p.scene.layers[0].geometry.fills).len() * 3;
    assert_eq!(fringe, alone, "the fringe is the outline's, once");
}

#[test]
fn a_ragged_stack_of_a_hundred_rows_is_linear_and_every_triangle_is_inside() {
    let rows: Vec<Rect> = (0..100)
        .map(|i| {
            let w = [300.0, 120.0, 4.0, 410.5, 410.0, 60.0, 0.5][i % 7];
            let x = if i % 11 == 0 { 30.0 } else { 0.0 };
            Rect::new(x, i as f32 * 18.0, w, 18.0)
        })
        .collect();
    let shapes = selection::shapes(&rows, (false, false), R, PAD);
    assert_true_shape(&rows, &shapes);
    let points: usize = shapes.iter().map(|s| s.outline.len()).sum();
    assert!(points <= 100 * 24, "{points} points for 100 rows");
    let fills = painted(&rows, (false, false));
    let tris = interior(&fills);
    assert!(tris.len() > 100, "the instrument sees triangles");
    for t in &tris {
        let c = [
            (t[0][0] + t[1][0] + t[2][0]) / 3.0,
            (t[0][1] + t[1][1] + t[2][1]) / 3.0,
        ];
        assert!(
            shapes.iter().any(|s| inside(&s.outline, c, 1e-3)),
            "triangle {t:?} leaves the outline"
        );
    }
    let covered: f64 = tris.iter().map(tri_area).sum();
    let want: f64 = shapes.iter().map(|s| area(&s.outline).abs()).sum();
    assert!((covered - want).abs() < 1.0, "{covered} vs {want}");
}

#[test]
fn rows_with_a_gap_are_two_shapes() {
    let rows = [
        Rect::new(0.0, 0.0, 50.0, 20.0),
        Rect::new(0.0, 30.0, 50.0, 20.0),
    ];
    assert_eq!(selection::shapes(&rows, (false, false), R, PAD).len(), 2);
    // Touching but sharing no width: two shapes as well.
    let rows = [
        Rect::new(100.0, 0.0, 50.0, 20.0),
        Rect::new(0.0, 20.0, 100.0, 20.0),
    ];
    assert_eq!(selection::shapes(&rows, (false, false), R, PAD).len(), 2);
}

#[test]
fn an_open_side_has_square_corners() {
    let rows = [
        Rect::new(10.0, 0.0, 50.0, 20.0),
        Rect::new(10.0, 20.0, 80.0, 20.0),
    ];
    let top = [[8.0, 0.0], [62.0, 0.0]];
    let bottom = [[8.0, 40.0], [92.0, 40.0]];
    let s = one(&rows, (true, false));
    assert!(top.iter().all(|c| s.outline.contains(c)), "flat top");
    assert!(
        !bottom.iter().any(|c| s.outline.contains(c)),
        "round bottom"
    );
    let s = one(&rows, (false, true));
    assert!(!top.iter().any(|c| s.outline.contains(c)), "round top");
    assert!(bottom.iter().all(|c| s.outline.contains(c)), "flat bottom");
}

#[test]
fn a_narrow_row_keeps_valid_corners() {
    // A blank line's sliver between two lines, and a one-row sliver.
    let rows = [
        Rect::new(0.0, 0.0, 80.0, 18.0),
        Rect::new(0.0, 18.0, 2.0, 18.0),
        Rect::new(0.0, 36.0, 40.0, 18.0),
    ];
    let s = one(&rows, (false, false));
    assert_true_shape(&rows, std::slice::from_ref(&s));
    let parts: f64 = s.parts.iter().map(|p| area(p).abs()).sum();
    assert!((parts - area(&s.outline).abs()).abs() < 0.5);
    let fills = painted(&rows, (false, false));
    let covered: f64 = interior(&fills).iter().map(tri_area).sum();
    assert!((covered - area(&s.outline).abs()).abs() < 0.5, "{covered}");
    let s = one(&[Rect::new(0.0, 0.0, 0.5, 18.0)], (false, false));
    assert!(area(&s.outline).abs() > 0.0);
}

#[test]
fn bad_rows_draw_nothing_non_finite() {
    let cases: Vec<Vec<Rect>> = vec![
        vec![Rect::new(f32::NAN, 0.0, 10.0, 10.0)],
        vec![Rect::new(0.0, 0.0, f32::INFINITY, 10.0)],
        vec![Rect::new(f32::MAX, 0.0, f32::MAX, 10.0)],
        vec![
            Rect::new(0.0, 0.0, 0.0, 10.0),
            Rect::new(0.0, 10.0, -5.0, 10.0),
        ],
        vec![
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(0.0, 10.0, 10.0, f32::NAN),
            Rect::new(0.0, 10.0, 10.0, 0.1),
            Rect::new(0.0, 10.1, 1e30, 10.0),
        ],
    ];
    for rows in cases {
        let fills = painted(&rows, (true, false));
        assert!(
            fills.iter().all(|v| v.pos.iter().all(|c| c.is_finite())),
            "{rows:?}"
        );
    }
    // A bad radius or padding counts as 0: a plain rect.
    let s = selection::shapes(
        &[Rect::new(0.0, 0.0, 10.0, 10.0)],
        (false, false),
        f32::NAN,
        -1.0,
    );
    assert_eq!(bounds(s[0].outline.iter().copied()), [0.0, 0.0, 10.0, 10.0]);
}

#[test]
fn every_scale_keeps_the_rows_joined_without_overlap() {
    // At 1.5x a fractional row edge is shared exactly: the parts tile the outline.
    for scale in [1.0, 1.5, 2.0] {
        let rows = [
            Rect::new(10.0, 3.3, 120.0, 17.6),
            Rect::new(4.0, 20.9, 60.0, 17.6),
            Rect::new(4.0, 38.5, 200.0, 17.6),
        ];
        let mut text = FakeText;
        let mut p = painter(&mut text, scale);
        selection::paint(&mut p, &rows, (false, false), INK);
        let tris = interior(&p.scene.layers[0].geometry.fills);
        let s = one(&rows, (false, false));
        assert_true_shape(&rows, std::slice::from_ref(&s));
        let want = area(&s.outline).abs() * f64::from(scale * scale);
        let covered: f64 = tris.iter().map(tri_area).sum();
        assert!((covered - want).abs() < 1.0, "{scale}: {covered} vs {want}");
    }
}

#[test]
fn fill_joined_records_and_replays_as_one_call() {
    let rows = [
        Rect::new(10.0, 0.0, 50.0, 20.0),
        Rect::new(10.0, 20.0, 80.0, 20.0),
    ];
    let mut text = FakeText;
    let mut p = painter(&mut text, 2.0);
    p.start_recording();
    selection::paint(&mut p, &rows, (false, false), INK);
    let ops = p.take_recording();
    assert_eq!(ops.len(), 1);
    let first = p.scene.layers[0].geometry.fills.clone();
    let mut text = FakeText;
    let mut q = painter(&mut text, 2.0);
    q.replay(&ops);
    assert_eq!(q.scene.layers[0].geometry.fills, first);
}

#[test]
fn fill_joined_with_a_point_not_finite_draws_nothing() {
    let mut text = FakeText;
    let mut p = painter(&mut text, 1.0);
    let part = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    p.fill_joined(
        &[part.clone(), vec![[0.0, 0.0], [f32::NAN, 1.0], [1.0, 1.0]]],
        &part,
        INK,
    );
    p.fill_joined(
        std::slice::from_ref(&part),
        &[[0.0, 0.0], [f32::INFINITY, 0.0], [1.0, 1.0]],
        INK,
    );
    assert!(p.scene.layers[0].geometry.fills.is_empty());
    p.fill_joined(std::slice::from_ref(&part), &part, INK);
    assert!(
        !p.scene.layers[0].geometry.fills.is_empty(),
        "the instrument"
    );
}

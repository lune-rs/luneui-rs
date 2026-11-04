//! Pure path geometry for the painter's vector primitives: dash splitting, arc flattening, ear
//! clipping and the antialiasing fringe. Plain `f32` points in, plain points and indices out; the
//! painter turns them into [`StrokeVertex`](crate::scene::StrokeVertex) records.

/// The most pieces one dashed line is split into; a finer pattern is drawn solid.
const MAX_DASHES: usize = 100_000;
/// The most points a filled polygon may have; ear clipping is quadratic in them.
pub(super) const MAX_FILL_POINTS: usize = 4096;
/// The most segments one arc is flattened into.
const MAX_ARC_SEGMENTS: usize = 4096;
/// The largest chord error of a flattened arc, in device px.
pub(super) const ARC_TOLERANCE: f32 = 0.25;

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
}

/// Twice the signed area of the loop `p` (the shoelace sum): positive when the loop turns the way
/// that makes a right-hand normal `(dy, -dx)` point outward.
pub(super) fn signed_area2(p: &[[f32; 2]]) -> f64 {
    let n = p.len();
    (0..n)
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            f64::from(a[0]) * f64::from(b[1]) - f64::from(b[0]) * f64::from(a[1])
        })
        .sum()
}

/// `pts` without consecutive duplicates, and without a closing point equal to the first.
pub(super) fn dedup_loop(pts: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut p: Vec<[f32; 2]> = Vec::with_capacity(pts.len());
    for &q in pts {
        if p.last()
            .is_none_or(|l| (l[0] - q[0]).abs() + (l[1] - q[1]).abs() > 1e-3)
        {
            p.push(q);
        }
    }
    while p.len() > 2 && {
        let (f, l) = (p[0], p[p.len() - 1]);
        (f[0] - l[0]).abs() + (f[1] - l[1]).abs() <= 1e-3
    } {
        p.pop();
    }
    p
}

/// Splits the polyline `pts` (closed: with the segment from the last point back to the first)
/// into the runs of a pattern that is `on` long, then `off` long, starting with `on` at the first
/// point. Each run is an open polyline with the path's corners inside it. `None` when the pattern
/// would make more than [`MAX_DASHES`] runs. Needs `on > 0`, `off > 0` and finite points.
pub(super) fn dash(
    pts: &[[f32; 2]],
    closed: bool,
    on: f32,
    off: f32,
) -> Option<Vec<Vec<[f32; 2]>>> {
    let mut path = dedup_loop(pts);
    if path.len() < 2 {
        return Some(Vec::new());
    }
    if closed {
        path.push(path[0]);
    }
    let total: f32 = path.windows(2).map(|w| dist(w[0], w[1])).sum();
    if !total.is_finite() || total / (on + off) > MAX_DASHES as f32 {
        return None;
    }
    let mut out: Vec<Vec<[f32; 2]>> = Vec::new();
    let mut piece = vec![path[0]];
    let (mut drawing, mut remaining) = (true, on);
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = dist(a, b);
        if len <= 1e-9 {
            continue;
        }
        let mut pos = 0.0_f32;
        while pos < len {
            let step = remaining.min(len - pos);
            pos += step;
            remaining -= step;
            let t = pos / len;
            let q = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            if drawing {
                piece.push(q);
            }
            if remaining <= 1e-6 {
                if drawing {
                    out.push(std::mem::take(&mut piece));
                } else {
                    piece = vec![q];
                }
                drawing = !drawing;
                remaining = if drawing { on } else { off };
            }
        }
    }
    if drawing && piece.len() >= 2 {
        out.push(piece);
    }
    Some(out)
}

/// Flattens the arc of `radius` about `center` (logical px) from angle `start` over `sweep`
/// (radians; 0 points right, a positive sweep turns clockwise on a y-down screen) so that no
/// chord is farther than [`ARC_TOLERANCE`] from the circle once drawn at `device_radius`.
/// Returns the points and whether they form a closed circle (`|sweep|` of 2π or more: the points
/// do not repeat the first). `None` for a radius or sweep that is not a positive finite radius
/// and a nonzero finite sweep.
pub(super) fn arc(
    center: [f32; 2],
    radius: f32,
    start: f32,
    sweep: f32,
    device_radius: f32,
) -> Option<(Vec<[f32; 2]>, bool)> {
    let ok = center.iter().all(|v| v.is_finite())
        && radius.is_finite()
        && radius > 0.0
        && start.is_finite()
        && sweep.is_finite()
        && sweep != 0.0
        && device_radius.is_finite()
        && device_radius > 0.0;
    if !ok {
        return None;
    }
    let tau = std::f64::consts::TAU;
    let full = f64::from(sweep).abs() >= tau - 1e-9;
    let span = if full {
        tau.copysign(f64::from(sweep))
    } else {
        f64::from(sweep)
    };
    let max_step = if device_radius > ARC_TOLERANCE {
        2.0 * (1.0 - f64::from(ARC_TOLERANCE) / f64::from(device_radius)).acos()
    } else {
        std::f64::consts::PI
    };
    let n = ((span.abs() / max_step).ceil() as usize)
        .max(if full { 3 } else { 1 })
        .min(MAX_ARC_SEGMENTS);
    let count = if full { n } else { n + 1 };
    let pts = (0..count)
        .map(|k| {
            let a = f64::from(start) + span * k as f64 / n as f64;
            [
                (f64::from(center[0]) + f64::from(radius) * a.cos()) as f32,
                (f64::from(center[1]) + f64::from(radius) * a.sin()) as f32,
            ]
        })
        .collect();
    Some((pts, full))
}

fn cross(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f64 {
    (f64::from(b[0]) - f64::from(a[0])) * (f64::from(c[1]) - f64::from(b[1]))
        - (f64::from(b[1]) - f64::from(a[1])) * (f64::from(c[0]) - f64::from(b[0]))
}

fn in_triangle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let side = |u: [f32; 2], v: [f32; 2]| {
        (f64::from(v[0]) - f64::from(u[0])) * (f64::from(p[1]) - f64::from(u[1]))
            - (f64::from(v[1]) - f64::from(u[1])) * (f64::from(p[0]) - f64::from(u[0]))
    };
    side(a, b) >= 0.0 && side(b, c) >= 0.0 && side(c, a) >= 0.0
}

/// Triangulates the simple polygon `p`, wound so that its [`signed_area2`] is positive, by ear
/// clipping. Returns index triples into `p`. A polygon the clipper cannot finish (self-crossing,
/// repeated points, more than [`MAX_FILL_POINTS`] points) yields the triangles found so far.
pub(super) fn ear_clip(p: &[[f32; 2]]) -> Vec<[usize; 3]> {
    let n = p.len();
    if !(3..=MAX_FILL_POINTS).contains(&n) {
        return Vec::new();
    }
    let mut idx: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity(n - 2);
    let (mut i, mut miss) = (0, 0);
    while idx.len() > 3 {
        let m = idx.len();
        i %= m;
        let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
        let ear = cross(p[a], p[b], p[c]) > 1e-9
            && idx.iter().all(|&q| {
                q == a
                    || q == b
                    || q == c
                    || p[q] == p[a]
                    || p[q] == p[b]
                    || p[q] == p[c]
                    || !in_triangle(p[q], p[a], p[b], p[c])
            });
        if ear {
            out.push([a, b, c]);
            idx.remove(i);
            miss = 0;
            i = i.saturating_sub(1);
        } else {
            i += 1;
            miss += 1;
            if miss >= m {
                // No ear anywhere: drop a collinear vertex, or give up.
                let flat = (0..m).find(|&k| {
                    cross(p[idx[(k + m - 1) % m]], p[idx[k]], p[idx[(k + 1) % m]]).abs() <= 1e-9
                });
                match flat {
                    Some(k) => {
                        idx.remove(k);
                        miss = 0;
                    }
                    None => return out,
                }
            }
        }
    }
    if idx.len() == 3 && cross(p[idx[0]], p[idx[1]], p[idx[2]]).abs() > 1e-9 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

/// The antialiasing fringe of the loop `b`: a one-pixel-wide band outside its edges, as a
/// triangle list of `(point, outer)` where `outer` marks the points `dist` away from the loop
/// (corners get a mitred offset, limited like a stroke's). The loop needs no particular winding.
pub(super) fn fringe(b: &[[f32; 2]], dist: f32) -> Vec<([f32; 2], bool)> {
    let n = b.len();
    if n < 3 {
        return Vec::new();
    }
    let sign = if signed_area2(b) >= 0.0 { 1.0 } else { -1.0 };
    // The outward unit normal of each edge k -> k + 1.
    let edge: Vec<[f32; 2]> = (0..n)
        .map(|k| {
            let (a, c) = (b[k], b[(k + 1) % n]);
            let (dx, dy) = (c[0] - a[0], c[1] - a[1]);
            let len = (dx * dx + dy * dy).sqrt();
            if len <= 1e-9 {
                [0.0, 0.0]
            } else {
                [sign * dy / len, -sign * dx / len]
            }
        })
        .collect();
    let outer: Vec<[f32; 2]> = (0..n)
        .map(|k| {
            let (e0, e1) = (edge[(k + n - 1) % n], edge[k]);
            let m = [e0[0] + e1[0], e0[1] + e1[1]];
            let len = (m[0] * m[0] + m[1] * m[1]).sqrt();
            let (v, miter) = if len <= 1e-6 {
                (e1, 1.0)
            } else {
                let v = [m[0] / len, m[1] / len];
                let cos = (v[0] * e1[0] + v[1] * e1[1]).max(0.3);
                (v, 1.0 / cos)
            };
            [b[k][0] + v[0] * dist * miter, b[k][1] + v[1] * dist * miter]
        })
        .collect();
    let mut out = Vec::with_capacity(n * 6);
    for k in 0..n {
        let j = (k + 1) % n;
        out.extend_from_slice(&[
            (b[k], false),
            (b[j], false),
            (outer[j], true),
            (b[k], false),
            (outer[j], true),
            (outer[k], true),
        ]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_area(p: &[[f32; 2]], t: [usize; 3]) -> f64 {
        cross(p[t[0]], p[t[1]], p[t[2]]) / 2.0
    }

    fn total(p: &[[f32; 2]], tris: &[[usize; 3]]) -> f64 {
        tris.iter().map(|&t| tri_area(p, t)).sum()
    }

    #[test]
    fn a_square_is_two_triangles() {
        let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let t = ear_clip(&sq);
        assert_eq!(t.len(), 2);
        assert!((total(&sq, &t) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn an_l_shape_keeps_its_area() {
        let l = [
            [0.0, 0.0],
            [20.0, 0.0],
            [20.0, 10.0],
            [10.0, 10.0],
            [10.0, 30.0],
            [0.0, 30.0],
        ];
        assert!(signed_area2(&l) > 0.0);
        let t = ear_clip(&l);
        assert_eq!(t.len(), 4);
        assert!((total(&l, &t) - 400.0).abs() < 1e-9);
    }

    #[test]
    fn a_collinear_vertex_is_absorbed() {
        let sq = [
            [0.0, 0.0],
            [5.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
        ];
        let t = ear_clip(&sq);
        assert!((total(&sq, &t) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn a_bow_tie_finishes_without_panicking() {
        let bow = [[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0]];
        let t = ear_clip(&bow);
        assert!(t.len() <= 2);
    }

    #[test]
    fn too_many_points_draw_no_triangles() {
        let p: Vec<[f32; 2]> = (0..=MAX_FILL_POINTS)
            .map(|i| {
                let a = i as f32 / MAX_FILL_POINTS as f32 * std::f32::consts::TAU;
                [a.cos() * 100.0, a.sin() * 100.0]
            })
            .collect();
        assert!(ear_clip(&p).is_empty());
    }

    #[test]
    fn dash_one_two_splits_every_third_unit() {
        let d = dash(&[[0.0, 0.0], [10.0, 0.0]], false, 1.0, 2.0).expect("few pieces");
        let xs: Vec<(f32, f32)> = d.iter().map(|p| (p[0][0], p[p.len() - 1][0])).collect();
        assert_eq!(xs.len(), 4);
        for (k, (a, b)) in xs.iter().enumerate() {
            assert!((a - 3.0 * k as f32).abs() < 1e-4, "{xs:?}");
            assert!((b - (3.0 * k as f32 + 1.0)).abs() < 1e-4, "{xs:?}");
        }
    }

    #[test]
    fn a_dash_run_keeps_the_corner_it_crosses() {
        let d = dash(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]], false, 6.0, 1.0).expect("few");
        assert_eq!(d[0], [[0.0, 0.0], [4.0, 0.0], [4.0, 2.0]]);
    }

    #[test]
    fn a_closed_dash_walks_the_closing_segment() {
        let sq = [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
        let d = dash(&sq, true, 2.0, 2.0).expect("few");
        let len: f32 = d.iter().map(|p| polyline(p)).sum();
        assert!((len - 8.0).abs() < 1e-3, "{len}");
        assert_eq!(d.len(), 4);
    }

    fn polyline(p: &[[f32; 2]]) -> f32 {
        p.windows(2).map(|w| dist(w[0], w[1])).sum()
    }

    #[test]
    fn a_pattern_finer_than_the_cap_is_refused() {
        assert!(dash(&[[0.0, 0.0], [1e6, 0.0]], false, 1.0, 1.0).is_none());
    }

    #[test]
    fn arc_chords_stay_within_a_quarter_pixel() {
        for device_radius in [0.1_f32, 1.0, 5.0, 40.0, 300.0, 5000.0] {
            let (pts, closed) =
                arc([0.0, 0.0], device_radius, 0.3, 4.0, device_radius).expect("arc");
            assert!(!closed);
            for w in pts.windows(2) {
                let (mx, my) = (
                    (f64::from(w[0][0]) + f64::from(w[1][0])) / 2.0,
                    (f64::from(w[0][1]) + f64::from(w[1][1])) / 2.0,
                );
                let off = (f64::from(device_radius) - (mx * mx + my * my).sqrt()).abs();
                assert!(
                    off <= f64::from(ARC_TOLERANCE) + 1e-2,
                    "r {device_radius}: {off}"
                );
            }
        }
    }

    #[test]
    fn a_full_sweep_is_a_closed_circle_without_a_repeated_point() {
        let (pts, closed) = arc([5.0, 5.0], 20.0, 0.0, 7.0, 20.0).expect("arc");
        assert!(closed);
        assert!(dist(pts[0], pts[pts.len() - 1]) > 0.1);
    }

    #[test]
    fn bad_arc_input_is_refused() {
        assert!(arc([0.0, 0.0], 0.0, 0.0, 1.0, 1.0).is_none());
        assert!(arc([0.0, 0.0], 5.0, f32::NAN, 1.0, 5.0).is_none());
        assert!(arc([0.0, 0.0], 5.0, 0.0, 0.0, 5.0).is_none());
        assert!(arc([f32::INFINITY, 0.0], 5.0, 0.0, 1.0, 5.0).is_none());
    }

    #[test]
    fn the_fringe_of_a_square_has_a_mitred_corner() {
        let sq = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        for pts in [sq.to_vec(), sq.iter().rev().copied().collect::<Vec<_>>()] {
            let f = fringe(&pts, 1.0);
            assert_eq!(f.len(), 24);
            let outer: Vec<[f32; 2]> = f.iter().filter(|v| v.1).map(|v| v.0).collect();
            let xs = outer.iter().map(|o| o[0]).fold(f32::MIN, f32::max);
            assert!((xs - 11.0).abs() < 1e-4, "{xs}");
            let ys = outer.iter().map(|o| o[1]).fold(f32::MIN, f32::max);
            assert!((ys - 11.0).abs() < 1e-4, "{ys}");
        }
    }
}

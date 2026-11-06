//! Pixel probes of the vector primitives at scale 2: dash, sector, path-fill, the rounded clip,
//! the backdrop at its own layer and the per-corner acrylic, drawn through the real renderer into an offscreen target and read back.
//!
//! GPU test: with no adapter it prints `skipped: no GPU adapter` and passes, unless
//! `LUNEUI_REQUIRE_GPU=1`, which makes a missing adapter a failure.

use lune_ui_core::paint::{LayerId, Painter, Rect, StrokeStyle};
use lune_ui_core::scene::Scene;
use lune_ui_core::text::{Spec, Style};
use lune_ui_core::theme::DARK;
use lune_ui_wgpu::dump;
use lune_ui_wgpu::frame::{FrameParams, FrameRenderer};
use lune_ui_wgpu::gpu::{self, Gpu};
use lune_ui_wgpu::text_cache::TextCache;

const SCALE: f32 = 2.0;
/// Device px.
const SIZE: (u32, u32) = (240, 160);
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn fonts() -> [&'static [u8]; 3] {
    [
        include_bytes!("../../../apps/gallery/assets/RobotoMonoNerdFontMono-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Regular.ttf"),
        include_bytes!("../../../apps/gallery/assets/RznSans-Medium.ttf"),
    ]
}

fn text_cache() -> TextCache {
    TextCache::new(&fonts(), "RznSans", "RobotoMono Nerd Font Mono")
}

fn headless_gpu() -> Option<Gpu> {
    match pollster::block_on(Gpu::new(gpu::instance(None), None)) {
        Ok(g) => Some(g),
        Err(e) => {
            let required = std::env::var("LUNEUI_REQUIRE_GPU").is_ok_and(|v| v == "1");
            assert!(!required, "no GPU adapter: {e}");
            println!("skipped: no GPU adapter");
            None
        }
    }
}

/// Paints with `f` into a fresh painter over the window (logical px) and returns the scene.
fn paint(text: &mut TextCache, f: impl FnOnce(&mut Painter<'_, u8>)) -> Scene {
    let win = Rect::new(0.0, 0.0, SIZE.0 as f32 / SCALE, SIZE.1 as f32 / SCALE);
    let mut p: Painter<'_, u8> = Painter::new(SCALE, win, text, None, DARK);
    f(&mut p);
    p.scene
}

fn render(gpu: &mut Gpu, text: &mut TextCache, scene: &Scene) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = FrameRenderer::new(gpu, format);
    let texture = dump::target(&gpu.device, format, SIZE);
    let view = texture.create_view(&Default::default());
    let params = FrameParams {
        size: SIZE,
        scale: SCALE,
        blur_sigma: 4.0,
        saturation: 1.0,
    };
    renderer
        .draw(gpu, text, scene, &view, params)
        .expect("draw");
    dump::read_rgba(&gpu.device, &gpu.queue, &texture).expect("readback")
}

fn px(img: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * SIZE.0 as usize + x) * 4;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

#[test]
fn a_dashed_line_has_its_runs_and_gaps() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    // Pattern 1 on, 2 off logical px at scale 2: 2 device px on, 4 off, 2 px thick (1 logical).
    let scene = paint(&mut text, |p| {
        p.dash(&[[0.0, 10.0], [50.0, 10.0]], false, 1.0, 1.0, 2.0, WHITE);
    });
    let img = render(&mut gpu, &mut text, &scene);
    // Instrument check: a pixel far from the line is empty, one on a run is white.
    assert_eq!(px(&img, 100, 100), [0, 0, 0, 0]);
    for row in [19, 20] {
        for k in 0..8 {
            let start = 6 * k;
            for dx in 0..2 {
                assert_eq!(
                    px(&img, start + dx, row),
                    WHITE,
                    "run {k} px {dx} row {row}"
                );
            }
            for dx in 2..6 {
                assert_eq!(px(&img, start + dx, row)[3], 0, "gap {k} px {dx} row {row}");
            }
        }
    }
    // Two device px thick: the rows beside the line are empty.
    assert_eq!(px(&img, 1, 17)[3], 0);
    assert_eq!(px(&img, 1, 22)[3], 0);
}

#[test]
fn a_sector_has_an_antialiased_edge_and_a_solid_inside() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    // Centre (60, 60) device, radius 40, from the right turning down a quarter.
    let scene = paint(&mut text, |p| {
        p.sector([30.0, 30.0], 20.0, 0.0, std::f32::consts::FRAC_PI_2, WHITE);
    });
    let img = render(&mut gpu, &mut text, &scene);
    // Instrument check: outside the sector is empty.
    assert_eq!(px(&img, 30, 30), [0, 0, 0, 0]);
    assert_eq!(px(&img, 150, 100), [0, 0, 0, 0]);
    // The radial edge along y = 60: the row above is the boundary pixel.
    let edge = px(&img, 80, 59);
    assert!(edge[3] > 0 && edge[3] < 255, "radial edge alpha {edge:?}");
    // The arc, at 45 degrees: pixel (88, 88) straddles the circle.
    let arc = px(&img, 88, 88);
    assert!(arc[3] > 0 && arc[3] < 255, "arc edge alpha {arc:?}");
    // Two px inside every edge it is full.
    for (x, y) in [(80, 62), (62, 80), (75, 75), (63, 63)] {
        assert_eq!(px(&img, x, y), WHITE, "inside ({x}, {y})");
    }
    // No seam at the centre: the pixels around the apex are filled where the sector is.
    for (x, y) in [(62, 62), (61, 64), (64, 61)] {
        assert_eq!(px(&img, x, y)[3], 255, "apex ({x}, {y})");
    }
}

#[test]
fn a_filled_rect_through_fill_path_draws_under_later_text() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let square = [[10.0, 10.0], [100.0, 10.0], [100.0, 60.0], [10.0, 60.0]];
    let dark = Spec::plain("Hello", Style::ui(24.0, [0, 0, 0, 255]));
    // The text call comes first and the fill second: the fill still goes below.
    let with_text = paint(&mut text, |p| {
        p.text(&dark, 20.0, 20.0);
        p.fill_path(&square, WHITE);
    });
    let only_fill = paint(&mut text, |p| p.fill_path(&square, WHITE));
    let a = render(&mut gpu, &mut text, &only_fill);
    let b = render(&mut gpu, &mut text, &with_text);
    // Instrument check: the fill is white inside and empty outside.
    assert_eq!(px(&a, 100, 60), WHITE);
    assert_eq!(px(&a, 5, 5)[3], 0);
    assert_eq!(px(&a, 215, 140)[3], 0);
    // An edge pixel is partial (the fringe), two px outside is empty.
    assert_eq!(px(&a, 18, 40)[3], 0, "outside left");
    assert_eq!(
        px(&a, 19, 40)[3],
        64,
        "boundary pixel left: {:?}",
        px(&a, 19, 40)
    );
    assert_eq!(px(&a, 20, 40), WHITE);
    // Inside the rect only the text darkens pixels.
    let darkest = |img: &[u8]| {
        let mut m = 255;
        for y in 24..120 {
            for x in 24..200 {
                m = m.min(px(img, x, y)[0]);
            }
        }
        m
    };
    assert_eq!(darkest(&a), 255);
    assert!(
        darkest(&b) < 60,
        "text pixels over the fill: {}",
        darkest(&b)
    );
}

#[test]
fn a_rounded_clip_cuts_the_corner_of_shapes_fills_and_strokes() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    // Clip rect (10, 10, 60, 40) logical = (20, 20, 120, 80) device, radius 8 = 16 device.
    let clip = Rect::new(10.0, 10.0, 60.0, 40.0);
    let all = Rect::new(0.0, 0.0, 120.0, 80.0);
    let scenes: [(&str, Scene); 3] = [
        (
            "shape",
            paint(&mut text, |p| {
                p.push_clip_rounded(clip, 8.0);
                p.fill(all, 0.0, WHITE);
            }),
        ),
        (
            "fill_path",
            paint(&mut text, |p| {
                p.push_clip_rounded(clip, 8.0);
                p.fill_path(
                    &[[0.0, 0.0], [120.0, 0.0], [120.0, 80.0], [0.0, 80.0]],
                    WHITE,
                );
            }),
        ),
        (
            "stroke",
            paint(&mut text, |p| {
                p.push_clip_rounded(clip, 8.0);
                // A thick horizontal bar over the whole clip.
                for y in 0..80 {
                    p.stroke(
                        &[[0.0, y as f32 + 0.5], [120.0, y as f32 + 0.5]],
                        false,
                        StrokeStyle::solid(WHITE, 1.0),
                    );
                }
            }),
        ),
    ];
    for (what, scene) in &scenes {
        let img = render(&mut gpu, &mut text, scene);
        // Instrument check: the middle of the clip is filled, outside it is empty.
        assert_eq!(px(&img, 80, 60), WHITE, "{what}: middle");
        assert_eq!(px(&img, 10, 60)[3], 0, "{what}: outside the clip rect");
        // The corner pixel is cut away; the pixel 16 device px in is filled.
        assert_eq!(px(&img, 20, 20)[3], 0, "{what}: corner");
        assert_eq!(px(&img, 139, 99)[3], 0, "{what}: opposite corner");
        assert_eq!(px(&img, 36, 36), WHITE, "{what}: 16 px in");
        // A pixel on the corner arc is partial: (24, 24) is at distance ~ 17.5 - ... from the
        // arc centre (36, 36): 16.97 vs radius 16.
        let arc = px(&img, 24, 24);
        assert!(arc[3] < 255, "{what}: the arc cuts (24, 24): {arc:?}");
        // Along the straight edge the clip does not round anything.
        assert_eq!(px(&img, 80, 21), WHITE, "{what}: straight edge");
    }
}

#[test]
fn a_radius_of_zero_takes_the_old_path() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let clip = Rect::new(10.0, 10.0, 60.0, 40.0);
    let plain = paint(&mut text, |p| {
        p.push_clip(clip);
        p.fill(Rect::new(0.0, 0.0, 120.0, 80.0), 0.0, WHITE);
    });
    let zero = paint(&mut text, |p| {
        p.push_clip_rounded(clip, 0.0);
        p.fill(Rect::new(0.0, 0.0, 120.0, 80.0), 0.0, WHITE);
    });
    assert_eq!(plain, zero);
    let img = render(&mut gpu, &mut text, &zero);
    assert_eq!(px(&img, 20, 20), WHITE);
}

/// How far `a` in a list of `(x, y)` samples is lit, as a share of them.
fn lit_share(img: &[u8], samples: &[(usize, usize)]) -> f32 {
    let lit = samples
        .iter()
        .filter(|(x, y)| px(img, *x, *y)[0] > 128)
        .count();
    lit as f32 / samples.len() as f32
}

/// A square loop, logical px, and a sample on each device px along its centre line.
const LOOP: [[f32; 2]; 4] = [[20.0, 10.0], [100.0, 10.0], [100.0, 70.0], [20.0, 70.0]];

fn loop_samples() -> Vec<(usize, usize)> {
    let (l, t, r, b) = (40, 20, 200, 140);
    let top = (l..r).map(|x| (x, t));
    let right = (t..b).map(|y| (r, y));
    let bottom = (l + 1..=r).rev().map(|x| (x, b));
    let left = (t + 1..=b).rev().map(|y| (l, y));
    top.chain(right).chain(bottom).chain(left).collect()
}

#[test]
fn a_comet_lights_its_fraction_of_the_loop_at_scale_2() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let scene = paint(&mut text, |p| {
        p.stroke_comet(&LOOP, WHITE, 2.0, 0.14, 0.3);
    });
    let img = render(&mut gpu, &mut text, &scene);
    let share = lit_share(&img, &loop_samples());
    assert!((share - 0.14).abs() < 0.01, "lit {share}");
}

#[test]
fn a_comet_travels_with_its_phase() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let samples = loop_samples();
    let first_lit = |gpu: &mut Gpu, text: &mut TextCache, phase: f32| {
        let scene = paint(text, |p| p.stroke_comet(&LOOP, WHITE, 2.0, 0.14, phase));
        let img = render(gpu, text, &scene);
        (0..samples.len())
            .find(|i| {
                let (x, y) = samples[*i];
                let prev = samples[(i + samples.len() - 1) % samples.len()];
                px(&img, x, y)[0] > 128 && px(&img, prev.0, prev.1)[0] <= 128
            })
            .expect("a dash")
    };
    let a = first_lit(&mut gpu, &mut text, 0.25);
    let b = first_lit(&mut gpu, &mut text, 0.5);
    let moved = (a + samples.len() - b) % samples.len();
    let share = moved as f32 / samples.len() as f32;
    assert!((share - 0.25).abs() < 0.01, "moved {share} of the loop");
}

#[test]
fn a_sweep_is_brightest_at_its_phase_around_a_translated_centre() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    // The loop drawn 10 px left of where it lands: the centre must move with it.
    let shifted: Vec<[f32; 2]> = LOOP.iter().map(|q| [q[0] - 10.0, q[1]]).collect();
    let centre = [50.0, 40.0];
    let at = |gpu: &mut Gpu, text: &mut TextCache, phase: f32| {
        let scene = paint(text, |p| {
            p.push_translate(10.0, 0.0);
            p.stroke_sweep(&shifted, WHITE, 2.0, phase, centre);
            p.pop_translate();
        });
        let img = render(gpu, text, &scene);
        // Device px: the top, right, bottom and left mid-points of the loop.
        [(120, 20), (200, 80), (120, 140), (40, 80)].map(|(x, y)| px(&img, x, y)[0])
    };
    let up = at(&mut gpu, &mut text, 0.0);
    assert!(up[0] > 200 && up[2] < 10, "{up:?}");
    let right = at(&mut gpu, &mut text, 0.25);
    assert!(right[1] > 200 && right[3] < 10, "{right:?}");
    let down = at(&mut gpu, &mut text, 0.5);
    assert!(down[2] > 200 && down[0] < 10, "{down:?}");
}

/// Logical px. Black on the left of `EDGE_X`, white on the right.
const EDGE_X: f32 = 60.0;
const BLACK: [u8; 4] = [0, 0, 0, 255];

fn hard_edge(p: &mut Painter<'_, u8>, layer: LayerId) {
    p.on(layer);
    let (w, h) = (p.window.w, p.window.h);
    p.fill(Rect::new(0.0, 0.0, EDGE_X, h), 0.0, [0, 0, 0, 255]);
    p.fill(
        Rect::new(EDGE_X, 0.0, w - EDGE_X, h),
        0.0,
        [255, 255, 255, 255],
    );
}

/// A glass region across the edge, device px of its middle row.
const GLASS: Rect = Rect::new(30.0, 20.0, 60.0, 40.0);
const GLASS_ROW: usize = 80;
/// Device px either side of the edge, inside the glass.
const LEFT_OF_EDGE: usize = 116;
const RIGHT_OF_EDGE: usize = 124;

#[test]
fn a_glass_panel_on_raised_blurs_the_modal_layer_under_it() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let scene = paint(&mut text, |p| {
        hard_edge(p, LayerId::Modal);
        p.on(LayerId::Raised);
        p.backdrop(GLASS, 0.0);
    });
    let img = render(&mut gpu, &mut text, &scene);
    let (left, right) = (
        px(&img, LEFT_OF_EDGE, GLASS_ROW),
        px(&img, RIGHT_OF_EDGE, GLASS_ROW),
    );
    assert!((30..226).contains(&left[0]), "blurred, not black: {left:?}");
    assert!(
        (30..226).contains(&right[0]),
        "blurred, not white: {right:?}"
    );
    // Outside the panel the edge stays sharp.
    assert_eq!(px(&img, LEFT_OF_EDGE, 20), BLACK);
    assert_eq!(px(&img, RIGHT_OF_EDGE, 20), WHITE);
}

#[test]
fn a_popup_backdrop_leaves_the_modal_layer_sharp_and_on_top() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let scene = paint(&mut text, |p| {
        p.on(LayerId::Page);
        p.fill(p.window, 0.0, [255, 0, 0, 255]);
        p.on(LayerId::Popup);
        p.backdrop(GLASS, 0.0);
        hard_edge(p, LayerId::Modal);
    });
    let img = render(&mut gpu, &mut text, &scene);
    assert_eq!(px(&img, LEFT_OF_EDGE, GLASS_ROW), BLACK);
    assert_eq!(px(&img, RIGHT_OF_EDGE, GLASS_ROW), WHITE);
}

/// The row of the glass across the edge, for a backdrop asked for by `f`.
fn blurred_row(sigma: f32, saturation: f32) -> Option<Vec<[u8; 4]>> {
    let mut gpu = headless_gpu()?;
    let mut text = text_cache();
    let scene = paint(&mut text, |p| {
        p.on(LayerId::Page);
        p.fill(
            Rect::new(0.0, 0.0, EDGE_X, p.window.h),
            0.0,
            [200, 40, 40, 255],
        );
        p.fill(
            Rect::new(EDGE_X, 0.0, p.window.w, p.window.h),
            0.0,
            [40, 40, 200, 255],
        );
        p.on(LayerId::Popup);
        p.backdrop_with(GLASS, 0.0, sigma, saturation);
    });
    let img = render(&mut gpu, &mut text, &scene);
    Some((70..170).map(|x| px(&img, x, GLASS_ROW)).collect())
}

#[test]
fn a_backdrop_with_its_own_sigma_and_saturation_differs_from_the_frames() {
    let Some(frame) = blurred_row(0.0, 0.0) else {
        return;
    };
    // `render` draws at sigma 4, saturation 1: asking for them by value is the frame's look.
    assert_eq!(blurred_row(4.0, 1.0), Some(frame.clone()));
    let wide = blurred_row(8.0, 0.0).expect("gpu");
    assert_ne!(wide, frame, "sigma 8 is not the frame's");
    // A wider blur has reached further from the edge.
    let reach = |row: &[[u8; 4]]| row.iter().filter(|c| c[0] > 60 && c[0] < 180).count();
    assert!(
        reach(&wide) > reach(&frame),
        "{} {}",
        reach(&wide),
        reach(&frame)
    );
    let vivid = blurred_row(0.0, 2.0).expect("gpu");
    assert_ne!(vivid, frame, "saturation 2 is not the frame's");
}

#[test]
fn acrylic_radii_cuts_only_the_corner_it_names() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let green = [0.0, 1.0, 0.0, 1.0];
    let red = [1.0, 0.0, 0.0, 1.0];
    let r = Rect::new(20.0, 20.0, 60.0, 40.0);
    let scene = paint(&mut text, |p| {
        p.acrylic_radii(r, [8.0, 0.0, 0.0, 0.0], green, red, false);
    });
    let img = render(&mut gpu, &mut text, &scene);
    let (l, t, right, b) = (40, 40, 159, 119);
    let green_px = [0, 255, 0, 255];
    assert_eq!(px(&img, l, t), [255, 0, 0, 255], "top-left is cut");
    assert_eq!(px(&img, right, t), green_px, "top-right is square");
    assert_eq!(px(&img, right, b), green_px, "bottom-right is square");
    assert_eq!(px(&img, l, b), green_px, "bottom-left is square");
    assert_eq!(px(&img, l + 30, t + 30), green_px, "inside");
}

/// Whether two opaque pixels agree to within one level a channel.
fn near(a: [u8; 4], b: [u8; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 1)
}

#[test]
fn a_lane_graph_dot_is_a_disc_of_diameter_10_over_a_line_of_another_colour() {
    use lune_ui_core::widgets::lane_graph::{LaneLine, LaneRow, paint as lane_row};
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let line = [200, 30, 30, 255];
    let dot = [30, 30, 200, 255];
    // Lane 0 at x = 10 + 20 = 30, the centre line at 17.5 + 22.5 = 40 logical: device (60, 80).
    let lanes = [LaneLine {
        lane: 0,
        color: line,
        above: true,
        below: true,
    }];
    let scene = paint(&mut text, |p| {
        lane_row(
            p,
            (10.0, 17.5),
            &LaneRow {
                dot_lane: 0,
                dot_color: dot,
                lanes: &lanes,
                edges: &[],
            },
        );
    });
    let img = render(&mut gpu, &mut text, &scene);
    // Instrument check: far from the row is empty, and the line away from the dot is the line's.
    assert_eq!(px(&img, 200, 140), [0, 0, 0, 0]);
    assert!(
        near(px(&img, 60, 66), line),
        "7 logical px above: {:?}",
        px(&img, 60, 66)
    );
    assert!(
        near(px(&img, 60, 94), line),
        "7 logical px below: {:?}",
        px(&img, 60, 94)
    );
    // The dot's colour at the centre, where the line passes through, and 4 logical px from it.
    assert!(near(px(&img, 60, 80), dot), "centre {:?}", px(&img, 60, 80));
    for (x, y) in [(68, 80), (52, 80), (60, 72), (60, 88)] {
        assert!(near(px(&img, x, y), dot), "({x}, {y}) {:?}", px(&img, x, y));
    }
    // Diameter 10 logical (20 device px) within 1 logical px, read along the centre row.
    let run = (0..SIZE.0 as usize)
        .filter(|&x| near(px(&img, x, 80), dot))
        .count();
    assert!((18..=22).contains(&run), "dot diameter {run} device px");
    let col = (0..SIZE.1 as usize)
        .filter(|&y| near(px(&img, 60, y), dot))
        .count();
    assert!((18..=22).contains(&col), "dot height {col} device px");
}

/// D13: an italic run draws different ink from the roman run of the same text. No italic face
/// ships, so the shaper skews the roman face; the instrument is the white fill under the text
/// (a blank image would be identical to itself, so each render must show ink first).
#[test]
fn an_italic_run_draws_different_ink_from_the_roman_run() {
    let Some(mut gpu) = headless_gpu() else {
        return;
    };
    let mut text = text_cache();
    let square = [[2.0, 2.0], [118.0, 2.0], [118.0, 78.0], [2.0, 78.0]];
    let roman = Style::ui(40.0, [0, 0, 0, 255]);
    let on = |text: &mut TextCache, style: Style| {
        paint(text, |p| {
            p.fill_path(&square, WHITE);
            p.text(&Spec::plain("Hill", style), 10.0, 10.0);
        })
    };
    let a_scene = on(&mut text, roman);
    let b_scene = on(&mut text, roman.italic());
    let a = render(&mut gpu, &mut text, &a_scene);
    let b = render(&mut gpu, &mut text, &b_scene);
    let ink = |img: &[u8]| {
        (0..SIZE.1 as usize)
            .flat_map(|y| (0..SIZE.0 as usize).map(move |x| (x, y)))
            .filter(|&(x, y)| px(img, x, y)[3] > 0 && px(img, x, y)[0] < 128)
            .count()
    };
    assert!(ink(&a) > 100, "the roman run draws ink: {}", ink(&a));
    assert!(ink(&b) > 100, "the italic run draws ink: {}", ink(&b));
    assert_ne!(a, b, "italic changes the picture");
}

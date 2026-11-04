//! Layout helpers: known answers from `layout.stack`'s Acceptance, tiling over many children,
//! gaps, alignment, overflow, grids and totality.

use lune_ui_core::layout::{Align, Axis, Child, Insets, Size, Track, column, grid, row, stack};
use lune_ui_core::paint::Rect;

const PARENT: Rect = Rect::new(10.0, 20.0, 300.0, 100.0);

fn fixed(main: f32, cross: f32) -> Child {
    Child {
        main: Size::Fixed(main),
        cross,
    }
}

fn fill(weight: f32) -> Child {
    Child {
        main: Size::Fill(weight),
        cross: 0.0,
    }
}

fn approx(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} vs {b}");
}

fn finite(r: &Rect) -> bool {
    [r.x, r.y, r.w, r.h].iter().all(|v| v.is_finite())
}

#[test]
fn a_weight_two_child_gets_twice_a_weight_one_childs_share() {
    let r = row(
        PARENT,
        &[fill(1.0), fill(2.0)],
        0.0,
        Align::Stretch,
        Insets::ZERO,
    );
    approx(r[0].w, 100.0);
    approx(r[1].w, 200.0);
    approx(r[1].w, 2.0 * r[0].w);
    approx(r[1].x, r[0].x + r[0].w);
}

#[test]
fn insets_shrink_the_parent_by_their_amounts() {
    let r = row(
        PARENT,
        &[fill(1.0)],
        0.0,
        Align::Stretch,
        Insets::new(8.0, 12.0, 8.0, 12.0),
    );
    assert_eq!(r, [Rect::new(18.0, 32.0, 284.0, 76.0)]);
}

#[test]
fn a_gap_goes_only_between_children() {
    let r = row(
        PARENT,
        &[fixed(20.0, 10.0), fixed(20.0, 10.0), fixed(20.0, 10.0)],
        8.0,
        Align::Start,
        Insets::ZERO,
    );
    assert_eq!(
        r.iter().map(|r| r.x).collect::<Vec<_>>(),
        [10.0, 38.0, 66.0]
    );
    let one = row(
        PARENT,
        &[fixed(20.0, 10.0)],
        8.0,
        Align::Start,
        Insets::ZERO,
    );
    assert_eq!(one, [Rect::new(10.0, 20.0, 20.0, 10.0)]);
    // The gap comes out of the leftover that fill children share.
    let r = row(
        PARENT,
        &[fixed(40.0, 10.0), fill(1.0), fill(1.0)],
        10.0,
        Align::Stretch,
        Insets::ZERO,
    );
    approx(r[1].w, 120.0);
    approx(r[2].w, 120.0);
    approx(r[2].x + r[2].w, 310.0);
}

#[test]
fn a_thousand_fill_children_tile_the_parent() {
    let kids: Vec<Child> = (0..1000)
        .map(|i| fill(1.0 + (i % 7) as f32 * 0.37))
        .collect();
    for gap in [0.0, 0.5] {
        let parent = Rect::new(3.3, 0.0, 1234.567, 40.0);
        let r = row(parent, &kids, gap, Align::Stretch, Insets::ZERO);
        approx(r[0].x, 3.3);
        let last = r.last().expect("rects");
        approx(last.x + last.w, parent.x + parent.w);
        for pair in r.windows(2) {
            approx(pair[1].x - (pair[0].x + pair[0].w), gap);
        }
    }
}

#[test]
fn fixed_and_hug_overflow_runs_past_the_parent_and_fills_get_nothing() {
    let kids = [
        fixed(200.0, 5.0),
        Child {
            main: Size::Hug(150.0),
            cross: 5.0,
        },
        fill(1.0),
    ];
    let r = row(PARENT, &kids, 0.0, Align::Start, Insets::ZERO);
    assert_eq!(r[0].w, 200.0);
    assert_eq!((r[1].x, r[1].w), (210.0, 150.0));
    assert_eq!((r[2].x, r[2].w), (360.0, 0.0));
    assert!(r[1].x + r[1].w > PARENT.x + PARENT.w);
}

#[test]
fn cross_axis_alignment() {
    let k = [fixed(30.0, 20.0)];
    let at = |a| row(PARENT, &k, 0.0, a, Insets::ZERO)[0];
    assert_eq!(at(Align::Start), Rect::new(10.0, 20.0, 30.0, 20.0));
    assert_eq!(at(Align::Center), Rect::new(10.0, 60.0, 30.0, 20.0));
    assert_eq!(at(Align::End), Rect::new(10.0, 100.0, 30.0, 20.0));
    assert_eq!(at(Align::Stretch), Rect::new(10.0, 20.0, 30.0, 100.0));
}

#[test]
fn a_column_runs_top_to_bottom() {
    let r = column(
        PARENT,
        &[fixed(30.0, 20.0), fill(1.0)],
        4.0,
        Align::Start,
        Insets::ZERO,
    );
    assert_eq!(r[0], Rect::new(10.0, 20.0, 20.0, 30.0));
    assert_eq!(r[1], Rect::new(10.0, 54.0, 0.0, 66.0));
    assert_eq!(
        stack(
            Axis::Vertical,
            PARENT,
            &[fixed(1.0, 1.0)],
            0.0,
            Align::End,
            Insets::ZERO
        )[0],
        Rect::new(309.0, 20.0, 1.0, 1.0)
    );
}

#[test]
fn a_grid_of_auto_star_auto_columns_and_two_rows() {
    let g = grid(
        PARENT,
        &[Track::Auto(40.0), Track::Star(1.0), Track::Auto(60.0)],
        &[Track::Fixed(30.0), Track::Star(1.0)],
        10.0,
        6.0,
        Insets::ZERO,
    );
    assert_eq!((g.columns(), g.rows()), (3, 2));
    assert_eq!(g.cell(0, 0), Rect::new(10.0, 20.0, 40.0, 30.0));
    assert_eq!(g.cell(1, 0), Rect::new(60.0, 20.0, 180.0, 30.0));
    assert_eq!(g.cell(2, 1), Rect::new(250.0, 56.0, 60.0, 64.0));
    let c = g.cell(1, 1);
    assert_eq!((c.x, c.y, c.w, c.h), (60.0, 56.0, 180.0, 64.0));
}

#[test]
fn a_cell_out_of_range_is_a_zero_rect() {
    let g = grid(
        PARENT,
        &[Track::Star(1.0)],
        &[Track::Star(1.0)],
        0.0,
        0.0,
        Insets::ZERO,
    );
    let zero = Rect::new(0.0, 0.0, 0.0, 0.0);
    assert_eq!(g.cell(1, 0), zero);
    assert_eq!(g.cell(0, 1), zero);
    assert_eq!(g.cell(usize::MAX, usize::MAX), zero);
    assert_eq!(g.cell(0, 0), PARENT);
}

#[test]
fn an_empty_list_gives_an_empty_result() {
    assert!(row(PARENT, &[], 4.0, Align::Center, Insets::ZERO).is_empty());
    let g = grid(PARENT, &[], &[], 4.0, 4.0, Insets::ZERO);
    assert_eq!((g.columns(), g.rows()), (0, 0));
    assert_eq!(g.cell(0, 0), Rect::new(0.0, 0.0, 0.0, 0.0));
}

#[test]
fn bad_numbers_count_as_zero_and_every_rect_is_finite() {
    let bad = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -5.0,
        0.0,
        f32::MAX,
        f32::MIN,
    ];
    for &b in &bad {
        let parents = [
            Rect::new(b, b, b, b),
            Rect::new(0.0, 0.0, b, 50.0),
            Rect::new(10.0, 10.0, 50.0, b),
        ];
        for parent in parents {
            let kids = [
                Child {
                    main: Size::Fixed(b),
                    cross: b,
                },
                Child {
                    main: Size::Hug(b),
                    cross: 3.0,
                },
                Child {
                    main: Size::Fill(b),
                    cross: b,
                },
                fill(1.0),
            ];
            for align in [Align::Start, Align::Center, Align::End, Align::Stretch] {
                for axis in [Axis::Horizontal, Axis::Vertical] {
                    let rs = stack(axis, parent, &kids, b, align, Insets::new(b, b, b, b));
                    assert_eq!(rs.len(), kids.len());
                    assert!(rs.iter().all(finite), "{b} {parent:?} {align:?} {rs:?}");
                }
            }
            let g = grid(
                parent,
                &[
                    Track::Fixed(b),
                    Track::Auto(b),
                    Track::Star(b),
                    Track::Star(1.0),
                ],
                &[Track::Star(b), Track::Fixed(b)],
                b,
                b,
                Insets::all(b),
            );
            for c in 0..4 {
                for r in 0..2 {
                    assert!(finite(&g.cell(c, r)), "{b} {parent:?}");
                }
            }
        }
    }
}

#[test]
fn bad_numbers_equal_zero_not_just_finite() {
    let with = |b: f32| {
        row(
            PARENT,
            &[
                Child {
                    main: Size::Fixed(b),
                    cross: b,
                },
                fixed(50.0, 10.0),
            ],
            b,
            Align::Center,
            Insets::all(b),
        )
    };
    let zero = with(0.0);
    assert_eq!(with(f32::NAN), zero);
    assert_eq!(with(-3.0), zero);
    assert_eq!(with(f32::INFINITY), zero);
}

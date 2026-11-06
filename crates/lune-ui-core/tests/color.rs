//! `premul`, `over` and `with_alpha` at alpha 0, 128 and 255, against answers worked out by hand
//! from the formulas and written as literals (never recomputed with the code under test).

use lune_ui_core::color::{over, premul, with_alpha};

const EPS: f32 = 1e-6;

fn close(got: [f32; 4], want: [f32; 4]) {
    for i in 0..4 {
        assert!(
            (got[i] - want[i]).abs() < EPS,
            "channel {i}: got {got:?}, want {want:?}"
        );
    }
}

#[test]
fn premul_at_alpha_zero_is_all_zero() {
    close(premul([255, 128, 10, 0]), [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn premul_at_alpha_255_is_straight_over_255() {
    // 255/255 = 1, 0/255 = 0, 128/255 = 0.50196078...
    close(premul([255, 0, 128, 255]), [1.0, 0.0, 0.501_960_8, 1.0]);
}

#[test]
fn premul_at_alpha_128_scales_colour_by_128_over_255() {
    // a = 128/255 = 0.50196078; r = 1 * a; g = 0; b = (128/255) * a = 0.25196463
    close(
        premul([255, 0, 128, 128]),
        [0.501_960_8, 0.0, 0.251_964_63, 0.501_960_8],
    );
}

#[test]
fn over_with_transparent_top_is_the_layer_below() {
    close(
        over([0.0, 0.0, 0.0, 0.0], [0.2, 0.4, 0.6, 1.0]),
        [0.2, 0.4, 0.6, 1.0],
    );
}

#[test]
fn over_with_opaque_top_is_the_top() {
    close(
        over([0.5, 0.25, 0.0, 1.0], [0.2, 0.4, 0.6, 1.0]),
        [0.5, 0.25, 0.0, 1.0],
    );
}

#[test]
fn over_with_half_top_keeps_half_of_below() {
    // k = 1 - 0.5; each channel is top + below * 0.5
    close(
        over([0.5, 0.25, 0.0, 0.5], [0.2, 0.4, 0.6, 1.0]),
        [0.6, 0.45, 0.3, 1.0],
    );
}

#[test]
fn over_at_alpha_128_of_a_premultiplied_top() {
    // top = premul([255, 0, 128, 128]); k = 1 - 128/255 = 0.49803922; below is opaque green.
    // r: 0.50196078, g: 0 + 1 * k = 0.49803922, b: 0.25196463, a: 0.50196078 + k = 1
    let top = premul([255, 0, 128, 128]);
    close(
        over(top, [0.0, 1.0, 0.0, 1.0]),
        [0.501_960_8, 0.498_039_22, 0.251_964_63, 1.0],
    );
}

#[test]
fn with_alpha_scales_only_alpha() {
    assert_eq!(with_alpha([10, 20, 30, 200], 0.0), [10, 20, 30, 0]);
    assert_eq!(with_alpha([10, 20, 30, 200], 0.5), [10, 20, 30, 100]);
    assert_eq!(with_alpha([10, 20, 30, 200], 1.0), [10, 20, 30, 200]);
}

#[test]
fn with_alpha_at_alpha_0_128_255_rounds_and_saturates() {
    // 0 * k = 0 for any k
    assert_eq!(with_alpha([1, 2, 3, 0], 0.7)[3], 0);
    // 255 * 0.5 = 127.5 rounds away from zero to 128
    assert_eq!(with_alpha([1, 2, 3, 255], 0.5)[3], 128);
    // 128 * 0.5 = 64
    assert_eq!(with_alpha([1, 2, 3, 128], 0.5)[3], 64);
    // 255 * 2 = 510 saturates at 255 (the `as u8` cast)
    assert_eq!(with_alpha([1, 2, 3, 255], 2.0)[3], 255);
    // 128 * 1 = 128
    assert_eq!(with_alpha([1, 2, 3, 128], 1.0)[3], 128);
}

//! Colour type and helpers shared by the toolkit and its apps.

/// RGBA, 8 bits per channel, straight (not premultiplied) alpha.
pub type Rgba = [u8; 4];

/// `c` with its colour channels multiplied by its alpha, every channel in 0 to 1.
pub fn premul(c: Rgba) -> [f32; 4] {
    let a = f32::from(c[3]) / 255.0;
    [
        f32::from(c[0]) / 255.0 * a,
        f32::from(c[1]) / 255.0 * a,
        f32::from(c[2]) / 255.0 * a,
        a,
    ]
}

/// `top` over `below`, both premultiplied (see [`premul`]).
pub fn over(top: [f32; 4], below: [f32; 4]) -> [f32; 4] {
    let k = 1.0 - top[3];
    [
        top[0] + below[0] * k,
        top[1] + below[1] * k,
        top[2] + below[2] * k,
        top[3] + below[3] * k,
    ]
}

/// `c` with its alpha scaled by `k` (clamped to 0 to 1), rounded to the nearest step.
pub fn with_alpha(c: Rgba, k: f32) -> Rgba {
    [
        c[0],
        c[1],
        c[2],
        (f32::from(c[3]) * k.clamp(0.0, 1.0)).round() as u8,
    ]
}

/// `c` with its alpha set to `a`.
pub fn with_alpha_byte(c: Rgba, a: u8) -> Rgba {
    [c[0], c[1], c[2], a]
}

/// `c` with alpha 0: the same hue, invisible, so a fade keeps its colour.
pub fn clear(c: Rgba) -> Rgba {
    with_alpha_byte(c, 0)
}

/// `a` to `b` by `t` (clamped to 0 to 1), channel by channel and alpha included; exactly `a` at 0
/// and `b` at 1.
pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| {
        let (x, y) = (f32::from(a[i]), f32::from(b[i]));
        (x + (y - x) * t).round().clamp(0.0, 255.0) as u8
    })
}

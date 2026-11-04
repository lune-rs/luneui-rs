// Images: one textured quad per instance, optionally rotated about its centre. All positions are
// physical pixels; the texture holds premultiplied RGBA, so the output is the texel scaled by the
// draw's alpha and its edge coverage.

struct Globals {
    size: vec2<f32>,
    scale: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct ImageIn {
    @location(0) rect: vec4<f32>,   // the picture's rect: x, y, w, h
    @location(1) clip: vec4<f32>,   // plain clip rect; zero width = none
    @location(2) round: vec4<f32>,  // rounded clip rect
    @location(3) params: vec4<f32>, // rotation (radians, clockwise), alpha, rounded clip radius, -
};

struct ImageOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) p: vec2<f32>,
    @location(1) @interpolate(flat) rect: vec4<f32>,
    @location(2) @interpolate(flat) clip: vec4<f32>,
    @location(3) @interpolate(flat) round: vec4<f32>,
    @location(4) @interpolate(flat) params: vec4<f32>,
};

fn to_ndc(p: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(p.x / g.size.x * 2.0 - 1.0, 1.0 - p.y / g.size.y * 2.0, 0.0, 1.0);
}

fn clipped(p: vec2<f32>, c: vec4<f32>) -> bool {
    return c.z > 0.0 && (p.x < c.x || p.y < c.y || p.x >= c.x + c.z || p.y >= c.y + c.w);
}

fn sd_round_rect(p: vec2<f32>, half: vec2<f32>, rad: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(rad);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - rad;
}

@vertex
fn vs_image(@builtin(vertex_index) vi: u32, s: ImageIn) -> ImageOut {
    let corner = vec2<f32>(f32(vi & 1u), f32((vi >> 1u) & 1u));
    let centre = s.rect.xy + s.rect.zw * 0.5;
    // One pixel beyond the rect on each side, for the antialiased edge.
    let off = (corner * 2.0 - vec2<f32>(1.0)) * (s.rect.zw * 0.5 + vec2<f32>(1.0));
    let cs = cos(s.params.x);
    let sn = sin(s.params.x);
    let p = centre + vec2<f32>(off.x * cs - off.y * sn, off.x * sn + off.y * cs);
    var o: ImageOut;
    o.pos = to_ndc(p);
    o.p = p;
    o.rect = s.rect;
    o.clip = s.clip;
    o.round = s.round;
    o.params = s.params;
    return o;
}

@fragment
fn fs_image(i: ImageOut) -> @location(0) vec4<f32> {
    if clipped(i.p, i.clip) { discard; }
    let centre = i.rect.xy + i.rect.zw * 0.5;
    let cs = cos(i.params.x);
    let sn = sin(i.params.x);
    let d = i.p - centre;
    // The point in the picture's own, unrotated frame.
    let q = vec2<f32>(d.x * cs + d.y * sn, -d.x * sn + d.y * cs);
    let half = i.rect.zw * 0.5;
    var cov = clamp(half.x + 0.5 - abs(q.x), 0.0, 1.0) * clamp(half.y + 0.5 - abs(q.y), 0.0, 1.0);
    if i.params.z > 0.0 {
        let rd = sd_round_rect(i.p - (i.round.xy + i.round.zw * 0.5), i.round.zw * 0.5, i.params.z);
        cov = cov * clamp(0.5 - rd, 0.0, 1.0);
    }
    let uv = clamp(q / i.rect.zw + vec2<f32>(0.5), vec2<f32>(0.0), vec2<f32>(1.0));
    return textureSampleLevel(tex, samp, uv, 0.0) * (i.params.y * cov);
}

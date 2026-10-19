//! name: Scrolling Gradient
//! description: Animated diagonal stripes blending the base color with a hue-shifted partner.
//! alpha: opaque
//! color: Base Color = #A855F7
//! p0: Scroll X = 0.25 (-2..2)
//! p1: Scroll Y = 0.1 (-2..2)
//! p2: Stripes = 6.0 (1..30)
//! p3: Hue Shift = 0.35 (0..1)

fn shade(in: ShaderInput) -> vec4<f32> {
    let uv = in.uv + vec2<f32>(p0(), p1()) * in.time;
    let wave = 0.5 + 0.5 * sin((uv.x + uv.y) * p2() * PI);
    let partner = hue_shift(material.color.rgb, p3() * 2.0 * PI);
    let color = mix(material.color.rgb, partner, wave);
    return vec4<f32>(color * mix(0.65, 1.0, lambert(in.normal)), 1.0);
}

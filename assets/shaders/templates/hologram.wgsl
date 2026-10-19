//! name: Hologram
//! description: Transparent sci-fi projection with moving scanlines, rim glow and flicker.
//! alpha: blend
//! color: Base Color = #2EF2FF
//! p0: Line Density = 40.0 (5..200)
//! p1: Scroll Speed = 1.0 (0..5)
//! p2: Flicker = 0.15 (0..1)
//! p3: Opacity = 0.8 (0..1)

fn shade(in: ShaderInput) -> vec4<f32> {
    let lines = sin(in.world_position.y * p0() - in.time * p1() * 6.0) * 0.5 + 0.5;
    let scan = smoothstep(0.3, 0.7, lines);
    let rim = fresnel(in.normal, in.view_dir, 2.0);
    let flicker = 1.0 - p2() * hash11(floor(in.time * 24.0));
    let alpha = clamp((0.25 + 0.75 * scan) * (0.35 + rim) * p3() * flicker, 0.0, 1.0);
    let color = material.color.rgb * (0.6 + 1.4 * rim) * flicker;
    return vec4<f32>(color, alpha);
}

//! name: Unlit Color
//! description: Flat base color with an optional touch of soft shading.
//! alpha: opaque
//! color: Base Color = #4FC3F7
//! p0: Shading = 0.25 (0..1)
//! p1: Brightness = 1.0 (0..4)
//
// Built-ins: in.uv, in.normal, in.world_position, in.view_dir, in.time, in.frag_coord,
// material.color, p0()..p3(), lambert(), fresnel(), noise2(), fbm2(), hue_shift(), PI, LIGHT_DIR.

fn shade(in: ShaderInput) -> vec4<f32> {
    let shading = mix(1.0, lambert(in.normal), p0());
    return vec4<f32>(material.color.rgb * shading * p1(), 1.0);
}

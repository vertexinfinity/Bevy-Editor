//! name: Fresnel Rim
//! description: Lit body with a pulsing glow around the silhouette.
//! alpha: opaque
//! color: Base Color = #3A7BFF
//! p0: Rim Power = 3.0 (0.5..8)
//! p1: Rim Intensity = 1.5 (0..4)
//! p2: Pulse Speed = 2.0 (0..10)
//! p3: Body Light = 0.6 (0..1)

fn shade(in: ShaderInput) -> vec4<f32> {
    let body = material.color.rgb * lambert(in.normal) * p3();
    let pulse = 0.75 + 0.25 * sin(in.time * p2());
    let rim = fresnel(in.normal, in.view_dir, p0()) * p1() * pulse;
    let rim_color = mix(material.color.rgb, vec3<f32>(1.0), 0.5);
    return vec4<f32>(body + rim_color * rim, 1.0);
}

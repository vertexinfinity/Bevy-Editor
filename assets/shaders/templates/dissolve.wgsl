//! name: Dissolve
//! description: Noise-driven dissolve with a glowing burn edge (animates with Speed > 0).
//! alpha: opaque
//! color: Base Color = #B8C2CC
//! p0: Amount = 0.45 (0..1)
//! p1: Edge Width = 0.06 (0..0.3)
//! p2: Noise Scale = 8.0 (1..30)
//! p3: Speed = 0.8 (0..4)

fn shade(in: ShaderInput) -> vec4<f32> {
    let n = fbm2(in.uv * p2());
    let amount = clamp(p0() + 0.35 * sin(in.time * p3()), 0.0, 1.0);
    if (n < amount) {
        discard;
    }
    var color = material.color.rgb * lambert(in.normal);
    let edge = 1.0 - smoothstep(amount, amount + p1(), n);
    let burn = mix(vec3<f32>(1.0, 0.35, 0.05), vec3<f32>(1.0, 0.9, 0.4), edge);
    color = mix(color, burn * 3.0, edge);
    return vec4<f32>(color, 1.0);
}

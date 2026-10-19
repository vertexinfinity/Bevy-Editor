//! name: Toon
//! description: Cel shading with hard light bands, a specular dot and a dark outline.
//! alpha: opaque
//! color: Base Color = #FF8A3D
//! p0: Bands = 3.0 (1..8)
//! p1: Outline = 0.35 (0..1)
//! p2: Specular Size = 0.3 (0..1)
//! p3: Ambient = 0.25 (0..1)

fn shade(in: ShaderInput) -> vec4<f32> {
    let n = normalize(in.normal);
    let ndl = max(dot(n, LIGHT_DIR), 0.0);
    let bands = max(floor(p0()), 1.0);
    let toon = floor(ndl * bands + 0.5) / bands;
    var color = material.color.rgb * (p3() + (1.0 - p3()) * toon);

    let h = normalize(LIGHT_DIR + in.view_dir);
    let spec = step(1.0 - p2() * 0.08, dot(n, h));
    color = color + vec3<f32>(spec * 0.6);

    let facing = max(dot(n, in.view_dir), 0.0);
    if (facing < p1() * 0.5) {
        color = color * 0.12;
    }
    return vec4<f32>(color, 1.0);
}

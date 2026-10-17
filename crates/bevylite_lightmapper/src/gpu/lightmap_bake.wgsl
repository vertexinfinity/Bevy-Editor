// Hardware-accelerated GPU lightmap baking compute shader (WGSL)
// Executes concurrent ray tracing across RTX/Vulkan/DX12 compute cores.

struct Vertex {
    position: vec3<f32>,
    _pad0: f32,
    normal: vec3<f32>,
    _pad1: f32,
    uv: vec2<f32>,
    _pad2: vec2<f32>,
}

struct Triangle {
    v0: Vertex,
    v1: Vertex,
    v2: Vertex,
    normal: vec3<f32>,
    mesh_index: u32,
}

struct Light {
    position_or_dir: vec3<f32>,
    light_type: u32, // 0 = Directional, 1 = Point
    color: vec3<f32>,
    intensity: f32,
    range: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

struct BakeParams {
    resolution: u32,
    triangle_count: u32,
    light_count: u32,
    samples_per_pixel: u32,
    sky_color: vec3<f32>,
    direct_multiplier: f32,
    ao_intensity: f32,
    normal_bias: f32,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<uniform> params: BakeParams;
@group(0) @binding(1) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(2) var<storage, read> lights: array<Light>;
@group(0) @binding(3) var<storage, read_write> output_pixels: array<vec4<f32>>;

// Fast Möller-Trumbore ray-triangle intersection
fn intersect_triangle(origin: vec3<f32>, dir: vec3<f32>, max_dist: f32, tri: Triangle) -> f32 {
    let p0 = tri.v0.position;
    let p1 = tri.v1.position;
    let p2 = tri.v2.position;

    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let h = cross(dir, edge2);
    let a = dot(edge1, h);

    if (abs(a) < 1e-7) {
        return -1.0;
    }

    let f = 1.0 / a;
    let s = origin - p0;
    let u = f * dot(s, h);
    if (u < 0.0 || u > 1.0) {
        return -1.0;
    }

    let q = cross(s, edge1);
    let v = f * dot(dir, q);
    if (v < 0.0 || u + v > 1.0) {
        return -1.0;
    }

    let t = f * dot(edge2, q);
    if (t > 1e-3 && t < max_dist) {
        return t;
    }
    return -1.0;
}

// Tests if ray is occluded by any triangle in the scene
fn is_occluded(origin: vec3<f32>, dir: vec3<f32>, max_dist: f32) -> bool {
    for (var i = 0u; i < params.triangle_count; i = i + 1u) {
        let t = intersect_triangle(origin, dir, max_dist, triangles[i]);
        if (t > 0.0) {
            return true;
        }
    }
    return false;
}

// Tests if UV point (u, v) is inside triangle UVs and returns barycentric coordinates
fn uv_to_barycentric(p: vec2<f32>, uv0: vec2<f32>, uv1: vec2<f32>, uv2: vec2<f32>) -> vec4<f32> {
    let v0 = uv2 - uv0;
    let v1 = uv1 - uv0;
    let v2 = p - uv0;

    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);

    let denom = dot00 * dot11 - dot01 * dot01;
    if (abs(denom) < 1e-8) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let inv = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv;
    let v = (dot00 * dot12 - dot01 * dot02) * inv;
    let w = 1.0 - u - v;

    if (u >= -0.01 && v >= -0.01 && w >= -0.01 && (u + v) <= 1.01) {
        return vec4<f32>(clamp(w, 0.0, 1.0), clamp(v, 0.0, 1.0), clamp(u, 0.0, 1.0), 1.0);
    }
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}

// Generate cosine-weighted hemisphere ray
fn sample_hemisphere_cosine(normal: vec3<f32>, u1: f32, u2: f32) -> vec3<f32> {
    let r = sqrt(u1);
    let theta = 6.2831853 * u2;

    let x = r * cos(theta);
    let y = r * sin(theta);
    let z = sqrt(max(0.0, 1.0 - u1));

    var up = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(normal.y) > 0.99) {
        up = vec3<f32>(0.0, 0.0, 1.0);
    }
    let tangent = normalize(cross(normal, up));
    let bitangent = cross(normal, tangent);

    return normalize(tangent * x + bitangent * y + normal * z);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    let res = params.resolution;

    if (x >= res || y >= res) {
        return;
    }

    let pixel_idx = y * res + x;
    let uv = (vec2<f32>(f32(x), f32(y)) + vec2<f32>(0.5, 0.5)) / f32(res);

    var found = false;
    var surface_pos = vec3<f32>(0.0);
    var surface_norm = vec3<f32>(0.0);

    // Find the triangle corresponding to this UV texel
    for (var i = 0u; i < params.triangle_count; i = i + 1u) {
        let tri = triangles[i];
        let bary = uv_to_barycentric(uv, tri.v0.uv, tri.v1.uv, tri.v2.uv);
        if (bary.w > 0.5) {
            surface_pos = tri.v0.position * bary.x + tri.v1.position * bary.y + tri.v2.position * bary.z;
            surface_norm = normalize(tri.v0.normal * bary.x + tri.v1.normal * bary.y + tri.v2.normal * bary.z);
            found = true;
            break;
        }
    }

    if (!found) {
        output_pixels[pixel_idx] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return;
    }

    let origin = surface_pos + surface_norm * params.normal_bias;
    var radiance = vec3<f32>(0.0);

    // 1. Direct Lighting (Shadow rays to all scene lights)
    for (var i = 0u; i < params.light_count; i = i + 1u) {
        let light = lights[i];
        if (light.light_type == 0u) {
            // Directional Light
            let light_dir = -normalize(light.position_or_dir);
            let n_dot_l = max(0.0, dot(surface_norm, light_dir));
            if (n_dot_l > 0.0) {
                if (!is_occluded(origin, light_dir, 1000.0)) {
                    radiance = radiance + light.color * (light.intensity * params.direct_multiplier * n_dot_l);
                }
            }
        } else {
            // Point Light
            let to_light = light.position_or_dir - origin;
            let dist = length(to_light);
            if (dist < light.range && dist > 1e-4) {
                let light_dir = to_light / dist;
                let n_dot_l = max(0.0, dot(surface_norm, light_dir));
                if (n_dot_l > 0.0) {
                    if (!is_occluded(origin, light_dir, dist - 0.01)) {
                        let atten = max(0.0, 1.0 - (dist / light.range) * (dist / light.range)) / (dist * dist + 1.0);
                        radiance = radiance + light.color * (light.intensity * atten * params.direct_multiplier * n_dot_l);
                    }
                }
            }
        }
    }

    // 2. Ambient Occlusion & Hemisphere Indirect Light
    var unoccluded = 0.0;
    let sample_count = max(1u, params.samples_per_pixel);

    for (var s = 0u; s < sample_count; s = s + 1u) {
        let u1 = (f32(s) + 0.5) / f32(sample_count);
        let u2 = fract(f32(s) * 0.6180339887 + f32(x) * 0.071 + f32(y) * 0.093);
        let ray_dir = sample_hemisphere_cosine(surface_norm, u1, u2);

        if (!is_occluded(origin, ray_dir, 40.0)) {
            unoccluded = unoccluded + 1.0;
        }
    }

    let ao = unoccluded / f32(sample_count);
    let sky = params.sky_color * (ao * params.ao_intensity);
    radiance = radiance + sky;

    output_pixels[pixel_idx] = vec4<f32>(radiance, 1.0);
}

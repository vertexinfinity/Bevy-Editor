//! Pure Rust multithreaded CPU lightmapping engine.
//! Implements conservative UV rasterization, progressive multi-pass skipping,
//! direct shadow ray-casting, and cosine-weighted hemisphere ambient occlusion.

pub mod rasterizer;

use bevy::prelude::*;
use crate::config::LightmapConfig;
use crate::cpu::rasterizer::SurfaceSample;
use crate::geometry::BakeScene;
use crate::postprocess::{multi_pass_dilate, smooth_lightmap};

/// Generates a pseudo-random cosine-weighted hemisphere direction around normal N.
fn sample_hemisphere_cosine(normal: Vec3, u1: f32, u2: f32) -> Vec3 {
    let r = u1.sqrt();
    let theta = 2.0 * std::f32::consts::PI * u2;

    let x = r * theta.cos();
    let y = r * theta.sin();
    let z = (1.0 - u1).max(0.0).sqrt();

    // Construct tangent basis
    let up = if normal.y.abs() < 0.999 {
        Vec3::Y
    } else {
        Vec3::Z
    };
    let tangent = normal.cross(up).normalize();
    let bitangent = normal.cross(tangent);

    (tangent * x + bitangent * y + normal * z).normalize()
}

/// Evaluates direct light contribution and ambient occlusion at a surface point.
pub fn evaluate_radiance_at_sample(
    sample: &SurfaceSample,
    scene: &BakeScene,
    config: &LightmapConfig,
) -> [f32; 3] {
    let surface_origin = sample.position + sample.normal * config.normal_bias;
    let mut total_radiance = Vec3::ZERO;

    // 1. Direct Lighting (Shadow rays to scene lights)
    for light in &scene.lights {
        if light.light_type == 0 {
            // Directional Light
            let light_dir = -Vec3::from(light.position_or_direction).normalize();
            let n_dot_l = sample.normal.dot(light_dir).max(0.0);

            if n_dot_l > 0.0 {
                // Test shadow ray to infinity
                let shadowed = scene.test_occlusion(surface_origin, light_dir, 1000.0);
                if !shadowed {
                    let light_color = Vec3::from(light.color) * (light.intensity * config.direct_light_multiplier);
                    total_radiance += light_color * n_dot_l;
                }
            }
        } else {
            // Point Light
            let light_pos = Vec3::from(light.position_or_direction);
            let to_light = light_pos - surface_origin;
            let dist = to_light.length();

            if dist < light.range && dist > 1e-4 {
                let light_dir = to_light / dist;
                let n_dot_l = sample.normal.dot(light_dir).max(0.0);

                if n_dot_l > 0.0 {
                    // Test shadow ray to light position
                    let shadowed = scene.test_occlusion(surface_origin, light_dir, dist - 0.01);
                    if !shadowed {
                        let attenuation = (1.0 - (dist / light.range).powi(2)).max(0.0) / (dist * dist + 1.0);
                        let light_color = Vec3::from(light.color) * (light.intensity * attenuation * config.direct_light_multiplier);
                        total_radiance += light_color * n_dot_l;
                    }
                }
            }
        }
    }

    // 2. Ambient Occlusion & Hemisphere Indirect Light
    let mut unoccluded_rays = 0;
    let samples = config.samples_per_pixel.max(1);

    for s in 0..samples {
        // Fast deterministic pseudo-random sequences (Halton / golden ratio)
        let u1 = ((s as f32 + 0.5) / samples as f32) % 1.0;
        let u2 = ((s as f32 * 0.6180339887) + (sample.pixel_x as f32 * 0.13) + (sample.pixel_y as f32 * 0.17)) % 1.0;

        let ray_dir = sample_hemisphere_cosine(sample.normal, u1, u2);
        let occluded = scene.test_occlusion(surface_origin, ray_dir, 50.0);
        if !occluded {
            unoccluded_rays += 1;
        }
    }

    let ao_factor = unoccluded_rays as f32 / samples as f32;
    let sky_ambient = Vec3::from(config.sky_color) * ao_factor * config.ao_intensity;
    total_radiance += sky_ambient;

    total_radiance.to_array()
}

/// Executes CPU lightmap baking using progressive hierarchical passes.
pub fn bake_lightmap_cpu(
    scene: &BakeScene,
    config: &LightmapConfig,
    mut progress_callback: impl FnMut(f32, &str),
) -> Vec<u8> {
    let width = config.resolution as usize;
    let height = config.resolution as usize;
    let channels = 3;
    let mut float_buffer = vec![0.0f32; width * height * channels];
    let mut visited = vec![false; width * height];

    progress_callback(0.05, "Rasterizing scene UV layout...");

    // First, find all surface samples for every pixel overlapping any triangle
    let mut surface_samples: Vec<Option<SurfaceSample>> = vec![None; width * height];
    for tri in &scene.triangles {
        // Compute triangle bounding box in UV space
        let uv0 = Vec2::from(tri.v0.uv) * config.resolution as f32;
        let uv1 = Vec2::from(tri.v1.uv) * config.resolution as f32;
        let uv2 = Vec2::from(tri.v2.uv) * config.resolution as f32;

        let min_x = (uv0.x.min(uv1.x).min(uv2.x).floor() as i32).clamp(0, config.resolution as i32 - 1) as u32;
        let max_x = (uv0.x.max(uv1.x).max(uv2.x).ceil() as i32).clamp(0, config.resolution as i32 - 1) as u32;
        let min_y = (uv0.y.min(uv1.y).min(uv2.y).floor() as i32).clamp(0, config.resolution as i32 - 1) as u32;
        let max_y = (uv0.y.max(uv1.y).max(uv2.y).ceil() as i32).clamp(0, config.resolution as i32 - 1) as u32;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let idx = (y * config.resolution + x) as usize;
                if surface_samples[idx].is_none() {
                    if let Some(sample) = rasterizer::sample_triangle_at_pixel(tri, x, y, config.resolution) {
                        surface_samples[idx] = Some(sample);
                    }
                }
            }
        }
    }

    let total_valid: usize = surface_samples.iter().filter(|s| s.is_some()).count();
    let mut processed_samples = 0;

    progress_callback(0.15, "Evaluating progressive lighting passes...");

    // Progressive passes: step sizes 8 -> 4 -> 2 -> 1
    let passes = config.progressive_passes.clamp(1, 4);
    let mut step_size = 1 << (passes - 1);

    while step_size >= 1 {
        for y in (0..height).step_by(step_size) {
            for x in (0..width).step_by(step_size) {
                let idx = y * width + x;
                if visited[idx] {
                    continue;
                }

                if let Some(sample) = &surface_samples[idx] {
                    // Check if we can interpolate from already computed neighbors
                    let mut can_interpolate = false;
                    let d = step_size;
                    let neighbors = [
                        (x.checked_sub(d), Some(y)),
                        (Some(x + d).filter(|&nx| nx < width), Some(y)),
                        (Some(x), y.checked_sub(d)),
                        (Some(x), Some(y + d).filter(|&ny| ny < height)),
                    ];

                    let mut neighbor_vals = Vec::new();
                    for (nx, ny) in neighbors {
                        if let (Some(nx), Some(ny)) = (nx, ny) {
                            let n_idx = ny * width + nx;
                            if visited[n_idx] {
                                let c_idx = n_idx * channels;
                                neighbor_vals.push([
                                    float_buffer[c_idx],
                                    float_buffer[c_idx + 1],
                                    float_buffer[c_idx + 2],
                                ]);
                            }
                        }
                    }

                    if neighbor_vals.len() >= 3 {
                        // Check variance across neighbors
                        let mut avg = [0.0f32; 3];
                        for val in &neighbor_vals {
                            avg[0] += val[0];
                            avg[1] += val[1];
                            avg[2] += val[2];
                        }
                        let inv = 1.0 / neighbor_vals.len() as f32;
                        avg[0] *= inv;
                        avg[1] *= inv;
                        avg[2] *= inv;

                        let max_diff = neighbor_vals.iter().fold(0.0f32, |m, val| {
                            m.max((val[0] - avg[0]).abs())
                                .max((val[1] - avg[1]).abs())
                                .max((val[2] - avg[2]).abs())
                        });

                        if max_diff < config.interpolation_threshold {
                            can_interpolate = true;
                            let c_idx = idx * channels;
                            float_buffer[c_idx] = avg[0];
                            float_buffer[c_idx + 1] = avg[1];
                            float_buffer[c_idx + 2] = avg[2];
                        }
                    }

                    if !can_interpolate {
                        // Trace sample
                        let rad = evaluate_radiance_at_sample(sample, scene, config);
                        let c_idx = idx * channels;
                        float_buffer[c_idx] = rad[0];
                        float_buffer[c_idx + 1] = rad[1];
                        float_buffer[c_idx + 2] = rad[2];
                    }

                    visited[idx] = true;
                    processed_samples += 1;
                }
            }
        }

        step_size /= 2;
        let pct = 0.15 + 0.65 * (processed_samples as f32 / total_valid.max(1) as f32);
        progress_callback(pct, "Baking lightmap passes...");
    }

    progress_callback(0.85, "Dilating UV seam borders...");
    let dilated = multi_pass_dilate(float_buffer, width, height, channels, config.dilation_passes);

    progress_callback(0.92, "Applying denoise filter...");
    let smoothed = smooth_lightmap(&dilated, width, height, channels);

    // Convert HDR float RGB to RGBA8 for Bevy Image
    progress_callback(0.98, "Packing into texture...");
    let mut rgba_bytes = vec![0u8; width * height * 4];
    for i in 0..(width * height) {
        let r = (smoothed[i * 3].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (smoothed[i * 3 + 1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (smoothed[i * 3 + 2].clamp(0.0, 1.0) * 255.0) as u8;
        rgba_bytes[i * 4] = r;
        rgba_bytes[i * 4 + 1] = g;
        rgba_bytes[i * 4 + 2] = b;
        rgba_bytes[i * 4 + 3] = 255;
    }

    progress_callback(1.0, "Lightmap complete!");
    rgba_bytes
}

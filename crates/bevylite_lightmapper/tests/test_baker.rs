//! Tests for CPU and GPU lightmapping algorithms, ray-triangle intersection,
//! conservative UV clipping, dilation, and end-to-end scene baking.

use bevy::prelude::*;
use bevylite_lightmapper::config::{LightmapBackend, LightmapConfig};
use bevylite_lightmapper::cpu::bake_lightmap_cpu;
use bevylite_lightmapper::cpu::rasterizer::{clip_triangle_to_pixel, to_barycentric};
use bevylite_lightmapper::geometry::{BakeScene, GpuLight, LightmapTriangle, LightmapVertex, SceneMeshTarget};
use bevylite_lightmapper::postprocess::{dilate_lightmap, smooth_lightmap};

#[test]
fn test_ray_triangle_intersection() {
    let v0 = LightmapVertex {
        position: [-1.0, 0.0, -1.0],
        _pad0: 0.0,
        normal: [0.0, 1.0, 0.0],
        _pad1: 0.0,
        uv: [0.0, 0.0],
        _pad2: [0.0, 0.0],
    };
    let v1 = LightmapVertex {
        position: [1.0, 0.0, -1.0],
        _pad0: 0.0,
        normal: [0.0, 1.0, 0.0],
        _pad1: 0.0,
        uv: [1.0, 0.0],
        _pad2: [0.0, 0.0],
    };
    let v2 = LightmapVertex {
        position: [0.0, 0.0, 1.0],
        _pad0: 0.0,
        normal: [0.0, 1.0, 0.0],
        _pad1: 0.0,
        uv: [0.5, 1.0],
        _pad2: [0.0, 0.0],
    };
    let tri = LightmapTriangle::new(v0, v1, v2, 0);

    // Ray shooting down from (0, 5, 0)
    let hit = tri.intersect_ray(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y, 10.0);
    assert!(hit.is_some(), "Ray should hit horizontal triangle");
    let dist = hit.unwrap();
    assert!((dist - 5.0).abs() < 1e-4, "Hit distance should be 5.0, got {dist}");

    // Ray missing triangle (shooting at x = 10.0)
    let miss = tri.intersect_ray(Vec3::new(10.0, 5.0, 0.0), Vec3::NEG_Y, 10.0);
    assert!(miss.is_none(), "Ray outside triangle should miss");

    // Ray pointing away from triangle (shooting up +Y)
    let away = tri.intersect_ray(Vec3::new(0.0, 5.0, 0.0), Vec3::Y, 10.0);
    assert!(away.is_none(), "Ray pointing opposite direction should not hit");
}

#[test]
fn test_conservative_clipping_and_barycentric() {
    let tri_uv = [
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(0.0, 10.0),
    ];

    // Clip against pixel [1, 2] x [1, 2]
    let clipped = clip_triangle_to_pixel(&tri_uv, Vec2::new(1.0, 1.0), Vec2::new(2.0, 2.0));
    assert!(clipped.len() >= 3, "Clipped polygon should have at least 3 vertices");

    // Test barycentric coordinates of a point inside the triangle
    let bary = to_barycentric(tri_uv[0], tri_uv[1], tri_uv[2], Vec2::new(1.0, 1.0));
    assert!(bary.is_some(), "Barycentric calculation should succeed for valid triangle");
    let w = bary.unwrap();
    assert!(w.x >= 0.0 && w.y >= 0.0 && w.z >= 0.0, "Barycentric coordinates must be non-negative");
    assert!((w.x + w.y + w.z - 1.0).abs() < 1e-5, "Barycentric coordinates must sum to 1.0");
}

#[test]
fn test_postprocess_dilation_and_smoothing() {
    // 3x3 image with single pixel in center
    let width = 3;
    let height = 3;
    let channels = 3;
    let mut img = vec![0.0f32; width * height * channels];

    // Center pixel (1, 1) has white color
    let center_idx = (1 * width + 1) * channels;
    img[center_idx] = 1.0;
    img[center_idx + 1] = 1.0;
    img[center_idx + 2] = 1.0;

    // After dilation, neighboring pixels should be populated
    let dilated = dilate_lightmap(&img, width, height, channels);
    let top_idx = (0 * width + 1) * channels;
    assert!(dilated[top_idx] > 0.0, "Dilated top neighbor should receive color");

    // Smoothing should preserve non-zero pixels
    let smoothed = smooth_lightmap(&dilated, width, height, channels);
    assert!(smoothed[center_idx] > 0.0, "Smoothed center should remain non-zero");
}

#[test]
fn test_end_to_end_cpu_lightmap_baking() {
    // Build a test scene with a ground plane (2 triangles) and a sun light
    let mut scene = BakeScene::default();

    // Ground Quad
    let v0 = LightmapVertex {
        position: [-5.0, 0.0, -5.0],
        _pad0: 0.0,
        normal: [0.0, 1.0, 0.0],
        _pad1: 0.0,
        uv: [0.0, 0.0],
        _pad2: [0.0, 0.0],
    };
    let v1 = LightmapVertex {
        position: [5.0, 0.0, -5.0],
        _pad0: 0.0,
        normal: [0.0, 1.0, 0.0],
        _pad1: 0.0,
        uv: [1.0, 0.0],
        _pad2: [0.0, 0.0],
    };
    let v2 = LightmapVertex {
        position: [5.0, 0.0, 5.0],
        _pad0: 0.0,
        normal: [0.0, 1.0, 0.0],
        _pad1: 0.0,
        uv: [1.0, 1.0],
        _pad2: [0.0, 0.0],
    };
    let v3 = LightmapVertex {
        position: [-5.0, 0.0, 5.0],
        _pad0: 0.0,
        normal: [0.0, 1.0, 0.0],
        _pad1: 0.0,
        uv: [0.0, 1.0],
        _pad2: [0.0, 0.0],
    };

    scene.triangles.push(LightmapTriangle::new(v0, v1, v2, 0));
    scene.triangles.push(LightmapTriangle::new(v0, v2, v3, 0));

    // Sun Light shining diagonally downwards
    scene.lights.push(GpuLight {
        position_or_direction: [-0.5, -1.0, -0.5],
        light_type: 0, // Directional
        color: [1.0, 0.95, 0.9],
        intensity: 1.5,
        range: 0.0,
        _pad: [0.0; 3],
    });

    scene.meshes.push(SceneMeshTarget {
        entity: Entity::from_bits(1),
        name: "GroundPlane".to_string(),
        triangle_start: 0,
        triangle_count: 2,
        uv_min: Vec2::ZERO,
        uv_max: Vec2::ONE,
        uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
    });

    let config = LightmapConfig {
        backend: LightmapBackend::Cpu,
        resolution: 32, // fast 32x32 for test
        samples_per_pixel: 16,
        progressive_passes: 2,
        interpolation_threshold: 0.05,
        dilation_passes: 1,
        sky_color: [0.2, 0.25, 0.3],
        direct_light_multiplier: 1.0,
        ao_intensity: 1.0,
        normal_bias: 0.01,
    };

    let mut progress_reports = Vec::new();
    let baked_rgba = bake_lightmap_cpu(&scene, &config, |pct, msg| {
        progress_reports.push((pct, msg.to_string()));
    });

    assert_eq!(baked_rgba.len(), 32 * 32 * 4, "Baked image should be 32x32 RGBA8 (4096 bytes)");
    assert!(!progress_reports.is_empty(), "Progress reports should be called during baking");

    // Check that baked texels are not completely black (should have received direct sun + ambient sky light)
    let non_zero_pixels = baked_rgba
        .chunks_exact(4)
        .filter(|rgba| rgba[0] > 0 || rgba[1] > 0 || rgba[2] > 0)
        .count();

    assert!(
        non_zero_pixels > 0,
        "Baked lightmap should have illuminated non-zero pixels, found {non_zero_pixels}"
    );

    println!(
        "Successfully baked 32x32 test lightmap! Valid non-zero texels: {} / {}",
        non_zero_pixels,
        32 * 32
    );
}

#[test]
fn test_gpu_struct_alignments() {
    // Assert exact memory sizes expected by the GPU shader
    assert_eq!(std::mem::size_of::<LightmapVertex>(), 48, "LightmapVertex must be 48 bytes");
    assert_eq!(std::mem::size_of::<LightmapTriangle>(), 160, "LightmapTriangle must be 160 bytes");
    assert_eq!(std::mem::size_of::<GpuLight>(), 48, "GpuLight must be 48 bytes");

    // Also assert alignments
    assert_eq!(std::mem::align_of::<LightmapVertex>(), 4);
    assert_eq!(std::mem::align_of::<LightmapTriangle>(), 4);
    assert_eq!(std::mem::align_of::<GpuLight>(), 4);

    // Verify byte layout offsets of GpuLight match WGSL Light struct
    // Offset 0: position_or_direction (12 bytes)
    // Offset 12: light_type (4 bytes)
    // Offset 16: color (12 bytes)
    // Offset 28: intensity (4 bytes)
    // Offset 32: range (4 bytes)
    // Offset 36..48: _pad (12 bytes)
    let light = GpuLight {
        position_or_direction: [1.0, 2.0, 3.0],
        light_type: 1,
        color: [0.5, 0.6, 0.7],
        intensity: 2.5,
        range: 15.0,
        _pad: [0.0; 3],
    };
    let bytes = bytemuck::bytes_of(&light);
    assert_eq!(bytes.len(), 48);

    // Verify light_type is at byte 12
    let light_type_bytes = &bytes[12..16];
    assert_eq!(u32::from_ne_bytes(light_type_bytes.try_into().unwrap()), 1);

    // Verify intensity is at byte 28
    let intensity_bytes = &bytes[28..32];
    assert_eq!(f32::from_ne_bytes(intensity_bytes.try_into().unwrap()), 2.5);

    // Verify range is at byte 32
    let range_bytes = &bytes[32..36];
    assert_eq!(f32::from_ne_bytes(range_bytes.try_into().unwrap()), 15.0);
}


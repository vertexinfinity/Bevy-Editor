//! Extracts scene mesh triangles, UV coordinates, and lights from Bevy ECS into BakeScene.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};

use crate::geometry::{BakeScene, GpuLight, LightmapTriangle, LightmapVertex, SceneMeshTarget};

/// Extracts all static meshes and scene lights into a BakeScene structure.
pub fn extract_bake_scene(
    mesh_query: &Query<
        (Entity, &Mesh3d, &GlobalTransform, Option<&Name>),
        With<MeshMaterial3d<StandardMaterial>>,
    >,
    meshes: &Assets<Mesh>,
    dir_light_query: &Query<(&DirectionalLight, &GlobalTransform)>,
    point_light_query: &Query<(&PointLight, &GlobalTransform)>,
) -> BakeScene {
    let mut scene = BakeScene::default();

    // 1. Extract Lights
    for (dir_light, transform) in dir_light_query.iter() {
        let forward = transform.forward();
        let color = dir_light.color.to_linear().to_vec3().to_array();
        scene.lights.push(GpuLight {
            position_or_direction: forward.to_array(),
            light_type: 0, // Directional
            color,
            intensity: (dir_light.illuminance / 10000.0).clamp(0.1, 5.0),
            range: 0.0,
            _pad: [0.0; 3],
        });
    }

    for (point_light, transform) in point_light_query.iter() {
        let pos = transform.translation();
        let color = point_light.color.to_linear().to_vec3().to_array();
        scene.lights.push(GpuLight {
            position_or_direction: pos.to_array(),
            light_type: 1, // Point
            color,
            intensity: (point_light.intensity / 50000.0).clamp(0.1, 10.0),
            range: point_light.range.max(1.0),
            _pad: [0.0; 3],
        });
    }

    // Default fallback sun if no lights exist
    if scene.lights.is_empty() {
        scene.lights.push(GpuLight {
            position_or_direction: [-0.4, -0.8, -0.4],
            light_type: 0,
            color: [1.0, 0.95, 0.85],
            intensity: 1.2,
            range: 0.0,
            _pad: [0.0; 3],
        });
    }

    // 2. Extract Meshes & Triangles (skip unnamed internal gizmo meshes if named scene entities exist)
    let has_named_meshes = mesh_query.iter().any(|(_, _, _, name)| name.is_some());
    let candidates: Vec<_> = mesh_query
        .iter()
        .filter(|(_, mesh_handle, _, name)| {
            (!has_named_meshes || name.is_some()) && meshes.contains(&mesh_handle.0)
        })
        .collect();

    let total_meshes = candidates.len().max(1);
    let cols = (total_meshes as f32).sqrt().ceil().max(1.0) as u32;
    let rows = ((total_meshes as u32 + cols - 1) / cols).max(1);

    for (entity, mesh_handle, global_transform, name) in candidates {
        let Some(mesh) = meshes.get(&mesh_handle.0) else {
            continue;
        };

        let positions: &[[f32; 3]] = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(p)) => p.as_slice(),
            _ => continue,
        };

        // Prefer secondary lightmap UV (ATTRIBUTE_UV_1), fallback to primary UV (ATTRIBUTE_UV_0)
        let uvs: Vec<[f32; 2]> = if let Some(VertexAttributeValues::Float32x2(uv1)) = mesh.attribute(Mesh::ATTRIBUTE_UV_1) {
            uv1.clone()
        } else if let Some(VertexAttributeValues::Float32x2(uv0)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            uv0.clone()
        } else {
            default_uvs(positions.len())
        };

        // Extract Normals
        let normals: Vec<[f32; 3]> = if let Some(VertexAttributeValues::Float32x3(norm)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            norm.clone()
        } else {
            vec![[0.0, 1.0, 0.0]; positions.len()]
        };

        let model_matrix = global_transform.to_matrix();
        let normal_matrix = Mat3::from_mat4(model_matrix).inverse().transpose();

        let mesh_idx = scene.meshes.len() as u32;
        let tri_start = scene.triangles.len();

        let uv_rect = if total_meshes == 1 {
            Rect::new(0.0, 0.0, 1.0, 1.0)
        } else {
            let col = mesh_idx % cols;
            let row = mesh_idx / cols;
            let pad_x = 0.02 / cols as f32;
            let pad_y = 0.02 / rows as f32;
            let min_x = col as f32 / cols as f32 + pad_x;
            let min_y = row as f32 / rows as f32 + pad_y;
            let max_x = (col + 1) as f32 / cols as f32 - pad_x;
            let max_y = (row + 1) as f32 / rows as f32 - pad_y;
            Rect::new(min_x, min_y, max_x, max_y)
        };

        let mut uv_min = Vec2::splat(1.0);
        let mut uv_max = Vec2::splat(0.0);

        // Helper to transform vertex to world space and atlas UV space
        let transform_vertex = |idx: usize| -> LightmapVertex {
            let local_p = Vec3::from(positions[idx]);
            let world_p = model_matrix.transform_point3(local_p);

            let local_n = Vec3::from(normals[idx]);
            let world_n = (normal_matrix * local_n).normalize_or_zero();

            let raw_uv = Vec2::from(uvs[idx]).clamp(Vec2::ZERO, Vec2::ONE);
            let atlas_uv = uv_rect.min + raw_uv * (uv_rect.max - uv_rect.min);
            LightmapVertex {
                position: world_p.to_array(),
                _pad0: 0.0,
                normal: world_n.to_array(),
                _pad1: 0.0,
                uv: atlas_uv.to_array(),
                _pad2: [0.0; 2],
            }
        };

        // Process triangles (indexed or unindexed)
        if let Some(indices) = mesh.indices() {
            let idx_vec: Vec<usize> = match indices {
                Indices::U16(u16_vec) => u16_vec.iter().map(|&i| i as usize).collect(),
                Indices::U32(u32_vec) => u32_vec.iter().map(|&i| i as usize).collect(),
            };

            for chunk in idx_vec.chunks_exact(3) {
                let v0 = transform_vertex(chunk[0]);
                let v1 = transform_vertex(chunk[1]);
                let v2 = transform_vertex(chunk[2]);

                uv_min = uv_min.min(Vec2::from(v0.uv)).min(Vec2::from(v1.uv)).min(Vec2::from(v2.uv));
                uv_max = uv_max.max(Vec2::from(v0.uv)).max(Vec2::from(v1.uv)).max(Vec2::from(v2.uv));

                scene.triangles.push(LightmapTriangle::new(v0, v1, v2, mesh_idx));
            }
        } else {
            for i in (0..positions.len()).step_by(3) {
                if i + 2 < positions.len() {
                    let v0 = transform_vertex(i);
                    let v1 = transform_vertex(i + 1);
                    let v2 = transform_vertex(i + 2);

                    uv_min = uv_min.min(Vec2::from(v0.uv)).min(Vec2::from(v1.uv)).min(Vec2::from(v2.uv));
                    uv_max = uv_max.max(Vec2::from(v0.uv)).max(Vec2::from(v1.uv)).max(Vec2::from(v2.uv));

                    scene.triangles.push(LightmapTriangle::new(v0, v1, v2, mesh_idx));
                }
            }
        }

        let tri_count = scene.triangles.len() - tri_start;
        let label = name.map(|n| n.as_str()).unwrap_or("Mesh").to_string();

        scene.meshes.push(SceneMeshTarget {
            entity,
            name: label,
            triangle_start: tri_start,
            triangle_count: tri_count,
            uv_min,
            uv_max,
            uv_rect,
        });
    }

    scene
}

fn default_uvs(len: usize) -> Vec<[f32; 2]> {
    vec![[0.0, 0.0]; len]
}

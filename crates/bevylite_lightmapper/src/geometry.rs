//! Geometry extraction, ray intersection, and scene light representations.

use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

/// A single vertex representation for lightmapping in world space.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct LightmapVertex {
    pub position: [f32; 3],
    pub _pad0: f32,
    pub normal: [f32; 3],
    pub _pad1: f32,
    pub uv: [f32; 2],
    pub _pad2: [f32; 2],
}

/// A triangle in world space with associated lightmap UV coordinates.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct LightmapTriangle {
    pub v0: LightmapVertex,
    pub v1: LightmapVertex,
    pub v2: LightmapVertex,
    pub normal: [f32; 3],
    pub mesh_index: u32,
}

impl LightmapTriangle {
    pub fn new(v0: LightmapVertex, v1: LightmapVertex, v2: LightmapVertex, mesh_index: u32) -> Self {
        let p0 = Vec3::from(v0.position);
        let p1 = Vec3::from(v1.position);
        let p2 = Vec3::from(v2.position);
        let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero().to_array();
        Self {
            v0,
            v1,
            v2,
            normal,
            mesh_index,
        }
    }

    /// Fast Möller-Trumbore ray-triangle intersection test.
    /// Returns distance `t` along ray if intersection occurs, else `None`.
    #[inline]
    pub fn intersect_ray(&self, ray_origin: Vec3, ray_dir: Vec3, max_dist: f32) -> Option<f32> {
        let p0 = Vec3::from(self.v0.position);
        let p1 = Vec3::from(self.v1.position);
        let p2 = Vec3::from(self.v2.position);

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let h = ray_dir.cross(edge2);
        let a = edge1.dot(h);

        if a.abs() < 1e-7 {
            return None; // Ray is parallel to triangle
        }

        let f = 1.0 / a;
        let s = ray_origin - p0;
        let u = f * s.dot(h);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * ray_dir.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);
        if t > 1e-4 && t < max_dist {
            Some(t)
        } else {
            None
        }
    }
}

/// Representation of a light in the scene for lightmap baking.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct GpuLight {
    pub position_or_direction: [f32; 3],
    pub light_type: u32, // 0 = Directional, 1 = Point
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub _pad: [f32; 3],
}

/// Extracted scene mesh metadata for associating baked lightmaps back to entities.
#[derive(Clone, Debug)]
pub struct SceneMeshTarget {
    pub entity: Entity,
    pub name: String,
    pub triangle_start: usize,
    pub triangle_count: usize,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub uv_rect: Rect,
}

/// Complete scene geometry ready for lightmapping.
#[derive(Default, Clone, Debug)]
pub struct BakeScene {
    pub triangles: Vec<LightmapTriangle>,
    pub meshes: Vec<SceneMeshTarget>,
    pub lights: Vec<GpuLight>,
}

impl BakeScene {
    /// Tests if a ray intersects any triangle in the scene.
    pub fn test_occlusion(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> bool {
        for tri in &self.triangles {
            if tri.intersect_ray(origin, dir, max_dist).is_some() {
                return true;
            }
        }
        false
    }
}

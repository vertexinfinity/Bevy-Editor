//! Conservative UV triangle rasterizer and progressive neighbor interpolator
//! translated from Andreas Mantler's ands/lightmapper (`lightmapper.h`).

use bevy::prelude::*;
use crate::geometry::LightmapTriangle;

/// Sutherland-Hodgman convex polygon clipping.
/// Clips a polygon against an edge (defined by points a and b).
fn clip_edge(poly: &[Vec2], a: Vec2, b: Vec2) -> Vec<Vec2> {
    let mut out = Vec::new();
    if poly.is_empty() {
        return out;
    }

    let is_inside = |p: Vec2| -> bool {
        (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x) >= 0.0
    };

    let intersection = |p1: Vec2, p2: Vec2| -> Vec2 {
        let dc = Vec2::new(a.x - b.x, a.y - b.y);
        let dp = Vec2::new(p1.x - p2.x, p1.y - p2.y);
        let n1 = a.x * b.y - a.y * b.x;
        let n2 = p1.x * p2.y - p1.y * p2.x;
        let denom = 1.0 / (dc.x * dp.y - dc.y * dp.x);
        Vec2::new(
            (n1 * dp.x - n2 * dc.x) * denom,
            (n1 * dp.y - n2 * dc.y) * denom,
        )
    };

    let mut s = *poly.last().unwrap();
    for &p in poly {
        if is_inside(p) {
            if is_inside(s) {
                out.push(p);
            } else {
                out.push(intersection(s, p));
                out.push(p);
            }
        } else if is_inside(s) {
            out.push(intersection(s, p));
        }
        s = p;
    }

    out
}

/// Sutherland-Hodgman convex polygon clipping against a rectangular pixel [min, max].
pub fn clip_triangle_to_pixel(tri: &[Vec2; 3], pixel_min: Vec2, pixel_max: Vec2) -> Vec<Vec2> {
    let corners = [
        Vec2::new(pixel_min.x, pixel_min.y),
        Vec2::new(pixel_max.x, pixel_min.y),
        Vec2::new(pixel_max.x, pixel_max.y),
        Vec2::new(pixel_min.x, pixel_max.y),
    ];

    let mut poly = tri.to_vec();
    // Clip against the 4 edges of the pixel rectangle
    poly = clip_edge(&poly, corners[0], corners[1]);
    poly = clip_edge(&poly, corners[1], corners[2]);
    poly = clip_edge(&poly, corners[2], corners[3]);
    poly = clip_edge(&poly, corners[3], corners[0]);

    poly
}

/// Calculates barycentric coordinates `(u, v, w)` of point `p` within triangle `(p0, p1, p2)`.
/// Translated from `lm_toBarycentric`.
pub fn to_barycentric(p0: Vec2, p1: Vec2, p2: Vec2, p: Vec2) -> Option<Vec3> {
    let v0 = p2 - p0;
    let v1 = p1 - p0;
    let v2 = p - p0;

    let dot00 = v0.dot(v0);
    let dot01 = v0.dot(v1);
    let dot02 = v0.dot(v2);
    let dot11 = v1.dot(v1);
    let dot12 = v1.dot(v2);

    let denom = dot00 * dot11 - dot01 * dot01;
    if denom.abs() < 1e-8 {
        return None;
    }

    let inv = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv;
    let v = (dot00 * dot12 - dot01 * dot02) * inv;
    let w = 1.0 - u - v;

    Some(Vec3::new(w, v, u)) // weights for p0, p1, p2
}

/// A surface sample point to be evaluated.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceSample {
    pub position: Vec3,
    pub normal: Vec3,
    pub pixel_x: u32,
    pub pixel_y: u32,
}

/// Tests if pixel (x, y) overlaps triangle UV and returns the 3D surface sample (position & normal).
pub fn sample_triangle_at_pixel(
    tri: &LightmapTriangle,
    x: u32,
    y: u32,
    resolution: u32,
) -> Option<SurfaceSample> {
    let uv0 = Vec2::from(tri.v0.uv) * resolution as f32;
    let uv1 = Vec2::from(tri.v1.uv) * resolution as f32;
    let uv2 = Vec2::from(tri.v2.uv) * resolution as f32;

    let p_min = Vec2::new(x as f32, y as f32);
    let p_max = Vec2::new((x + 1) as f32, (y + 1) as f32);

    // Conservative clipping
    let clipped = clip_triangle_to_pixel(&[uv0, uv1, uv2], p_min, p_max);
    if clipped.len() < 3 {
        return None;
    }

    // Calculate centroid of the clipped polygon
    let mut centroid = Vec2::ZERO;
    for p in &clipped {
        centroid += *p;
    }
    centroid /= clipped.len() as f32;

    // Calculate barycentric coordinates for the centroid
    let bary = to_barycentric(uv0, uv1, uv2, centroid)?;

    let pos0 = Vec3::from(tri.v0.position);
    let pos1 = Vec3::from(tri.v1.position);
    let pos2 = Vec3::from(tri.v2.position);
    let position = pos0 * bary.x + pos1 * bary.y + pos2 * bary.z;

    let norm0 = Vec3::from(tri.v0.normal);
    let norm1 = Vec3::from(tri.v1.normal);
    let norm2 = Vec3::from(tri.v2.normal);
    let normal = (norm0 * bary.x + norm1 * bary.y + norm2 * bary.z).normalize_or_zero();

    Some(SurfaceSample {
        position,
        normal,
        pixel_x: x,
        pixel_y: y,
    })
}

//! Static level for "Target Range": an open-air shooting range with three lanes.
//!
//! Shared with the Bevylite editor, which includes this file via `#[path]`, so it only
//! depends on the Bevy prelude. Gameplay is attached by `main.rs`.
use bevy::prelude::*;

/// Handles to the spawned level, plus static data the gameplay needs.
pub struct LevelEntities {
    /// Every top-level entity of the level.
    pub all: Vec<Entity>,
    /// The player's camera (eye height 1.7 at the start position).
    pub camera: Entity,
    /// Where targets may pop up (target centers).
    pub target_spots: Vec<Vec3>,
    /// Solid boxes the player and bullets collide with, as (center, half extents).
    pub colliders: Vec<(Vec3, Vec3)>,
    /// Walkable area on the XZ plane (min, max): the inner faces of the perimeter walls.
    pub bounds: (Vec2, Vec2),
}

/// Half width of the range (inner face of the side walls).
const HALF_WIDTH: f32 = 10.0;
/// Z of the inner face of the back wall (downrange).
const BACK_Z: f32 = -16.0;
/// Z of the inner face of the front wall (behind the player).
const FRONT_Z: f32 = 9.0;
/// Lane centers on the X axis.
const LANES: [f32; 3] = [-6.0, 0.0, 6.0];

/// Spawns the range: floor, walls, lane dividers, crates, lights and the camera.
pub fn spawn_level(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> LevelEntities {
    let mut all = Vec::new();
    let mut colliders = Vec::new();
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mut mat = |color: Color, roughness: f32, emissive: LinearRgba| {
        materials.add(StandardMaterial { base_color: color, perceptual_roughness: roughness, emissive, ..default() })
    };
    let sand = mat(Color::srgb(0.76, 0.66, 0.50), 0.95, LinearRgba::BLACK);
    let lane_floor = mat(Color::srgb(0.62, 0.53, 0.40), 0.95, LinearRgba::BLACK);
    let concrete = mat(Color::srgb(0.58, 0.60, 0.62), 0.85, LinearRgba::BLACK);
    let berm = mat(Color::srgb(0.36, 0.30, 0.24), 1.0, LinearRgba::BLACK);
    let slate = mat(Color::srgb(0.20, 0.25, 0.30), 0.7, LinearRgba::BLACK);
    let teal = mat(Color::srgb(0.10, 0.55, 0.58), 0.5, LinearRgba::BLACK);
    let wood = mat(Color::srgb(0.55, 0.36, 0.18), 0.8, LinearRgba::BLACK);
    let wood_dark = mat(Color::srgb(0.36, 0.22, 0.10), 0.8, LinearRgba::BLACK);
    let stripe = mat(Color::srgb(0.95, 0.78, 0.15), 0.6, LinearRgba::rgb(0.25, 0.18, 0.0));
    let lamp = mat(Color::srgb(1.0, 0.9, 0.7), 0.3, LinearRgba::rgb(6.0, 4.5, 2.5));

    // Spawns one named box; `solid` boxes are also returned as colliders.
    let mut block = |commands: &mut Commands, name: &str, material: &Handle<StandardMaterial>, center: Vec3, size: Vec3, solid: bool| {
        let id = commands
            .spawn((Name::new(name.to_string()), Mesh3d(cube.clone()), MeshMaterial3d(material.clone()), Transform::from_translation(center).with_scale(size)))
            .id();
        all.push(id);
        if solid {
            colliders.push((center, size * 0.5));
        }
    };

    // Ground, lane strips and the firing line.
    let depth = FRONT_Z - BACK_Z;
    let mid_z = (FRONT_Z + BACK_Z) * 0.5;
    block(commands, "Floor - Sand", &sand, Vec3::new(0.0, -0.25, mid_z), Vec3::new(HALF_WIDTH * 2.0 + 2.0, 0.5, depth + 2.0), false);
    for (index, x) in LANES.iter().enumerate() {
        block(commands, &format!("Lane {} - Floor", index + 1), &lane_floor, Vec3::new(*x, 0.005, -9.5), Vec3::new(5.0, 0.01, 12.0), false);
    }
    block(commands, "Firing Line", &stripe, Vec3::new(0.0, 0.012, -3.2), Vec3::new(HALF_WIDTH * 2.0, 0.01, 0.25), false);

    // Perimeter: low side walls, a front wall and a tall earth berm downrange.
    for (side, label) in [(-1.0, "West"), (1.0, "East")] {
        block(commands, &format!("Wall - {label}"), &concrete, Vec3::new(side * (HALF_WIDTH + 0.25), 1.5, mid_z), Vec3::new(0.5, 3.0, depth), true);
        block(commands, &format!("Wall Cap - {label}"), &teal, Vec3::new(side * (HALF_WIDTH + 0.25), 3.05, mid_z), Vec3::new(0.7, 0.1, depth), false);
    }
    block(commands, "Wall - Front", &concrete, Vec3::new(0.0, 1.5, FRONT_Z + 0.25), Vec3::new(HALF_WIDTH * 2.0 + 1.0, 3.0, 0.5), true);
    block(commands, "Back Wall - Berm", &berm, Vec3::new(0.0, 2.5, BACK_Z - 0.5), Vec3::new(HALF_WIDTH * 2.0 + 1.0, 5.0, 1.0), true);
    block(commands, "Back Wall - Trim", &teal, Vec3::new(0.0, 5.05, BACK_Z - 0.5), Vec3::new(HALF_WIDTH * 2.0 + 1.2, 0.1, 1.2), false);

    // Lane dividers between the three lanes, with a teal cap.
    for (index, x) in [-3.0_f32, 3.0].iter().enumerate() {
        block(commands, &format!("Lane Divider {}", index + 1), &slate, Vec3::new(*x, 0.55, -9.6), Vec3::new(0.3, 1.1, 11.8), true);
        block(commands, &format!("Lane Divider {} - Cap", index + 1), &teal, Vec3::new(*x, 1.13, -9.6), Vec3::new(0.4, 0.06, 11.8), false);
    }

    // Lane number boards on the berm.
    for (index, x) in LANES.iter().enumerate() {
        block(commands, &format!("Lane {} - Board", index + 1), &slate, Vec3::new(*x, 4.2, BACK_Z + 0.06), Vec3::new(1.2, 0.6, 0.1), false);
        for pip in 0..=index {
            let offset = (pip as f32 - index as f32 * 0.5) * 0.3;
            block(commands, &format!("Lane {} - Marker {}", index + 1, pip + 1), &stripe, Vec3::new(*x + offset, 4.2, BACK_Z + 0.13), Vec3::new(0.15, 0.3, 0.05), false);
        }
    }

    // Cover crates around the firing line (some stacked); all solid.
    let crates: [(&str, Vec3, f32, bool); 8] = [
        ("Crate - West Pair A", Vec3::new(-8.6, 0.0, -1.2), 1.2, false),
        ("Crate - West Pair B", Vec3::new(-8.6, 1.2, -1.2), 0.9, true),
        ("Crate - West Single", Vec3::new(-7.2, 0.0, -0.8), 1.0, true),
        ("Crate - Center Left", Vec3::new(-3.6, 0.0, 2.2), 1.1, false),
        ("Crate - Center Right", Vec3::new(3.8, 0.0, 0.8), 1.2, true),
        ("Crate - Center Right Top", Vec3::new(3.8, 1.2, 0.8), 0.8, false),
        ("Crate - East Corner", Vec3::new(8.4, 0.0, 6.8), 1.4, true),
        ("Crate - West Corner", Vec3::new(-8.3, 0.0, 6.9), 1.2, false),
    ];
    for (name, base, size, dark) in crates {
        let center = base + Vec3::Y * size * 0.5;
        block(commands, name, if dark { &wood_dark } else { &wood }, center, Vec3::splat(size), true);
    }

    // Lamp posts behind the firing line.
    for (side, label) in [(-1.0, "West"), (1.0, "East")] {
        let x = side * 9.3;
        block(commands, &format!("Lamp Post - {label}"), &slate, Vec3::new(x, 2.0, -4.0), Vec3::new(0.2, 4.0, 0.2), false);
        block(commands, &format!("Lamp Head - {label}"), &lamp, Vec3::new(x - side * 0.4, 4.0, -4.0), Vec3::new(0.9, 0.15, 0.4), false);
    }

    // Lighting: a low afternoon sun plus two warm range lamps.
    let sun = commands
        .spawn((
            Name::new("Sun - Afternoon"),
            DirectionalLight { illuminance: 9000.0, color: Color::srgb(1.0, 0.95, 0.85), shadow_maps_enabled: true, ..default() },
            Transform::from_xyz(-10.0, 16.0, 8.0).looking_at(Vec3::new(0.0, 0.0, -4.0), Vec3::Y),
        ))
        .id();
    all.push(sun);
    for (side, label) in [(-1.0, "West"), (1.0, "East")] {
        let light = commands
            .spawn((
                Name::new(format!("Range Lamp - {label}")),
                PointLight { color: Color::srgb(1.0, 0.78, 0.5), intensity: 400_000.0, range: 20.0, ..default() },
                Transform::from_xyz(side * 8.6, 3.8, -4.0),
            ))
            .id();
        all.push(light);
    }

    // Target spots: two depths per lane at alternating heights, plus high ones on the berm.
    let mut target_spots = Vec::new();
    for (lane, x) in LANES.iter().enumerate() {
        for (row, z) in [-13.5_f32, -8.5].iter().enumerate() {
            let y = if (lane + row) % 2 == 0 { 1.7 } else { 2.3 };
            target_spots.push(Vec3::new(*x, y, *z));
        }
        target_spots.push(Vec3::new(*x, 3.2, BACK_Z + 0.6));
    }

    let camera = commands
        .spawn((Name::new("Player Camera"), Camera3d::default(), Transform::from_xyz(0.0, 1.7, 6.0).looking_at(Vec3::new(0.0, 1.7, -10.0), Vec3::Y)))
        .id();
    all.push(camera);

    let bounds = (Vec2::new(-HALF_WIDTH, BACK_Z), Vec2::new(HALF_WIDTH, FRONT_Z));
    LevelEntities { all, camera, target_spots, colliders, bounds }
}

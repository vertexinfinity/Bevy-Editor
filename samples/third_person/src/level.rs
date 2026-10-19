//! Static level for Coin Courtyard: a walled stone courtyard with raised platforms, trees and
//! pillars, lit by a low warm sun.
//!
//! This file is shared with the Bevylite editor, which includes it via `#[path]`. It only uses
//! the Bevy prelude and spawns plain meshes, lights, the player body and the camera; gameplay
//! components are attached by the sample's `main.rs`.
use bevy::prelude::*;

/// Half the width of the walkable courtyard (the inner faces of the walls).
pub const COURTYARD_HALF: f32 = 18.0;
/// Capsule radius of the player body.
pub const PLAYER_RADIUS: f32 = 0.4;
/// Distance from the player's center to its feet.
pub const PLAYER_HALF_HEIGHT: f32 = 0.9;
/// Where the player starts each round.
pub const PLAYER_START: Vec3 = Vec3::new(0.0, PLAYER_HALF_HEIGHT, 10.0);

/// A solid box the player can stand on or bump into.
#[derive(Clone, Copy, Debug)]
pub struct Platform {
    pub center: Vec3,
    pub half_size: Vec3,
}

/// Entities and layout data produced by [`spawn_level`].
pub struct LevelEntities {
    /// Every top-level entity spawned by the level.
    pub all: Vec<Entity>,
    pub player: Entity,
    pub camera: Entity,
    /// Where coins are placed each round (coin centers).
    pub coin_spots: Vec<Vec3>,
    /// Boxes the player collides with (platforms, stairs and pillar bases).
    pub platforms: Vec<Platform>,
}

/// Platform layout: (center, size). Tops are low enough to hop between.
const PLATFORMS: [(Vec3, Vec3); 7] = [
    // North-east stair: three steps climbing to a high deck.
    (Vec3::new(7.0, 0.4, -2.0), Vec3::new(3.0, 0.8, 3.0)),
    (Vec3::new(10.0, 0.8, -5.0), Vec3::new(3.0, 1.6, 3.0)),
    (Vec3::new(13.0, 1.2, -9.5), Vec3::new(4.0, 2.4, 5.0)),
    // West terrace.
    (Vec3::new(-10.0, 0.5, 2.0), Vec3::new(5.0, 1.0, 4.0)),
    (Vec3::new(-12.5, 1.0, -4.0), Vec3::new(4.0, 2.0, 4.0)),
    // Central fountain plinth.
    (Vec3::new(0.0, 0.35, -4.0), Vec3::new(4.0, 0.7, 4.0)),
    // South-east crate.
    (Vec3::new(10.0, 0.5, 10.0), Vec3::new(2.5, 1.0, 2.5)),
];

/// Coin positions: ground coins first, then coins resting above platform tops.
const COIN_SPOTS: [Vec3; 12] = [
    Vec3::new(-6.0, 1.0, 12.0),
    Vec3::new(6.0, 1.0, 14.0),
    Vec3::new(-15.0, 1.0, 14.0),
    Vec3::new(15.0, 1.0, 3.0),
    Vec3::new(-4.0, 1.0, -13.0),
    Vec3::new(4.0, 1.0, -15.0),
    Vec3::new(-15.0, 1.0, -14.0),
    Vec3::new(0.0, 1.7, -4.0),
    Vec3::new(10.0, 2.6, -5.0),
    Vec3::new(13.0, 3.4, -10.5),
    Vec3::new(-12.5, 3.0, -4.0),
    Vec3::new(10.0, 2.0, 10.0),
];

fn material(materials: &mut Assets<StandardMaterial>, color: Color, roughness: f32) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial { base_color: color, perceptual_roughness: roughness, ..default() })
}

/// Spawns the courtyard, lights, player body and camera.
pub fn spawn_level(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> LevelEntities {
    let mut all = Vec::new();

    // Palette: warm sandstone, mossy grass, terracotta trims and deep green foliage.
    let grass = material(materials, Color::srgb(0.36, 0.52, 0.28), 0.95);
    let paving = material(materials, Color::srgb(0.78, 0.70, 0.56), 0.9);
    let wall = material(materials, Color::srgb(0.70, 0.60, 0.48), 0.9);
    let trim = material(materials, Color::srgb(0.72, 0.38, 0.26), 0.8);
    let stone = material(materials, Color::srgb(0.62, 0.58, 0.54), 0.85);
    let bark = material(materials, Color::srgb(0.40, 0.27, 0.17), 0.9);
    let leaves = material(materials, Color::srgb(0.20, 0.45, 0.24), 0.8);
    let water = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.55, 0.75),
        perceptual_roughness: 0.15,
        metallic: 0.1,
        ..default()
    });

    // Ground: grass with a paved cross through the middle.
    let size = COURTYARD_HALF * 2.0;
    all.push(commands.spawn((Name::new("Ground"), Mesh3d(meshes.add(Plane3d::default().mesh().size(size + 4.0, size + 4.0))), MeshMaterial3d(grass), Transform::default())).id());
    let path_mesh = meshes.add(Cuboid::new(4.0, 0.02, size));
    for (name, rotation) in [("Path North-South", 0.0), ("Path East-West", std::f32::consts::FRAC_PI_2)] {
        all.push(
            commands
                .spawn((Name::new(name), Mesh3d(path_mesh.clone()), MeshMaterial3d(paving.clone()), Transform::from_xyz(0.0, 0.01, 0.0).with_rotation(Quat::from_rotation_y(rotation))))
                .id(),
        );
    }

    // Walls with a terracotta cap along the top.
    let wall_height = 2.6;
    let wall_long = meshes.add(Cuboid::new(size + 2.0, wall_height, 1.0));
    let cap_long = meshes.add(Cuboid::new(size + 2.4, 0.25, 1.3));
    let offset = COURTYARD_HALF + 0.5;
    for (name, pos, rot) in [
        ("Wall North", Vec3::new(0.0, 0.0, -offset), 0.0),
        ("Wall South", Vec3::new(0.0, 0.0, offset), 0.0),
        ("Wall East", Vec3::new(offset, 0.0, 0.0), std::f32::consts::FRAC_PI_2),
        ("Wall West", Vec3::new(-offset, 0.0, 0.0), std::f32::consts::FRAC_PI_2),
    ] {
        let rotation = Quat::from_rotation_y(rot);
        let wall_entity = commands
            .spawn((Name::new(name), Mesh3d(wall_long.clone()), MeshMaterial3d(wall.clone()), Transform::from_translation(pos + Vec3::Y * wall_height * 0.5).with_rotation(rotation)))
            .id();
        commands.spawn((Name::new("Cap"), Mesh3d(cap_long.clone()), MeshMaterial3d(trim.clone()), Transform::from_xyz(0.0, wall_height * 0.5 + 0.12, 0.0), ChildOf(wall_entity)));
        all.push(wall_entity);
    }

    // Platforms: stone blocks with a terracotta band just below the top edge.
    let mut platforms = Vec::new();
    for (center, size) in PLATFORMS {
        platforms.push(Platform { center, half_size: size * 0.5 });
    }
    for (i, platform) in platforms.iter().enumerate() {
        let size = platform.half_size * 2.0;
        let block = commands
            .spawn((Name::new(format!("Platform {}", i + 1)), Mesh3d(meshes.add(Cuboid::from_size(size))), MeshMaterial3d(stone.clone()), Transform::from_translation(platform.center)))
            .id();
        commands.spawn((
            Name::new("Lip"),
            Mesh3d(meshes.add(Cuboid::new(size.x + 0.1, 0.12, size.z + 0.1))),
            MeshMaterial3d(trim.clone()),
            Transform::from_xyz(0.0, size.y * 0.5 - 0.2, 0.0),
            ChildOf(block),
        ));
        all.push(block);
    }

    // Fountain bowl on the central plinth.
    let fountain = commands
        .spawn((Name::new("Fountain Water"), Mesh3d(meshes.add(Cylinder::new(1.5, 0.08))), MeshMaterial3d(water), Transform::from_xyz(0.0, 0.72, -4.0)))
        .id();
    all.push(fountain);

    // Pillars in the corners of the paths; their footprint also blocks the player.
    let pillar_mesh = meshes.add(Cylinder::new(0.45, 3.6));
    let pillar_cap = meshes.add(Cuboid::new(1.2, 0.3, 1.2));
    for (i, (x, z)) in [(-4.5, 6.0), (4.5, 6.0), (-4.5, -12.0), (4.5, -12.0)].into_iter().enumerate() {
        let pillar = commands
            .spawn((Name::new(format!("Pillar {}", i + 1)), Mesh3d(pillar_mesh.clone()), MeshMaterial3d(wall.clone()), Transform::from_xyz(x, 1.8, z)))
            .id();
        commands.spawn((Name::new("Pillar Cap"), Mesh3d(pillar_cap.clone()), MeshMaterial3d(trim.clone()), Transform::from_xyz(0.0, 1.85, 0.0), ChildOf(pillar)));
        all.push(pillar);
        platforms.push(Platform { center: Vec3::new(x, 1.8, z), half_size: Vec3::new(0.45, 1.8, 0.45) });
    }

    // Trees: a trunk with two stacked foliage spheres.
    let trunk_mesh = meshes.add(Cylinder::new(0.25, 2.0));
    let crown_mesh = meshes.add(Sphere::new(1.3).mesh().ico(3).unwrap());
    let top_mesh = meshes.add(Sphere::new(0.85).mesh().ico(3).unwrap());
    for (i, (x, z)) in [(-15.0, 9.0), (15.5, 15.0), (-8.0, -15.5), (15.0, -15.5), (-15.5, -9.5), (2.0, 15.5)].into_iter().enumerate() {
        let tree = commands
            .spawn((Name::new(format!("Tree {}", i + 1)), Mesh3d(trunk_mesh.clone()), MeshMaterial3d(bark.clone()), Transform::from_xyz(x, 1.0, z)))
            .id();
        commands.spawn((Name::new("Crown"), Mesh3d(crown_mesh.clone()), MeshMaterial3d(leaves.clone()), Transform::from_xyz(0.0, 1.6, 0.0), ChildOf(tree)));
        commands.spawn((Name::new("Crown Top"), Mesh3d(top_mesh.clone()), MeshMaterial3d(leaves.clone()), Transform::from_xyz(0.2, 2.6, 0.1), ChildOf(tree)));
        all.push(tree);
        platforms.push(Platform { center: Vec3::new(x, 1.0, z), half_size: Vec3::new(0.3, 1.0, 0.3) });
    }

    // Sun: low and warm so platforms and trees cast long shadows.
    all.push(
        commands
            .spawn((
                Name::new("Sun"),
                DirectionalLight { color: Color::srgb(1.0, 0.94, 0.82), illuminance: 9_000.0, shadow_maps_enabled: true, ..default() },
                Transform::from_xyz(12.0, 20.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
                bevy::light::CascadeShadowConfigBuilder { maximum_distance: 70.0, first_cascade_far_bound: 15.0, ..default() }.build(),
            ))
            .id(),
    );
    // Soft fill light glowing over the fountain.
    all.push(
        commands
            .spawn((Name::new("Fountain Glow"), PointLight { color: Color::srgb(0.6, 0.8, 1.0), intensity: 60_000.0, range: 14.0, ..default() }, Transform::from_xyz(0.0, 3.0, -4.0)))
            .id(),
    );

    // Player body: a blue capsule with a dark visor showing which way it faces (-Z).
    let player = commands
        .spawn((
            Name::new("Player"),
            Mesh3d(meshes.add(Capsule3d::new(PLAYER_RADIUS, PLAYER_HALF_HEIGHT * 2.0 - PLAYER_RADIUS * 2.0))),
            MeshMaterial3d(material(materials, Color::srgb(0.22, 0.48, 0.92), 0.5)),
            Transform::from_translation(PLAYER_START),
        ))
        .id();
    commands.spawn((
        Name::new("Visor"),
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.18, 0.2))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.08, 0.1, 0.14), perceptual_roughness: 0.2, metallic: 0.6, ..default() })),
        Transform::from_xyz(0.0, 0.45, -0.34),
        ChildOf(player),
    ));
    all.push(player);

    let camera = commands
        .spawn((Name::new("Camera"), Camera3d::default(), Transform::from_xyz(0.0, 6.0, 20.0).looking_at(PLAYER_START, Vec3::Y)))
        .id();
    all.push(camera);

    LevelEntities { all, player, camera, coin_spots: COIN_SPOTS.to_vec(), platforms }
}

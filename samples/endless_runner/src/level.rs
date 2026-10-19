//! Static layout for Neon Runner: a three-lane track, side pylons, arches and lighting.
//!
//! Shared with the Bevylite editor (included by path), so it only depends on Bevy.
//! The game scrolls the `track_tiles` and `decor` toward the camera to fake endless motion.
use bevy::prelude::*;

/// Lane centers on the X axis, left to right.
pub const LANES: [f32; 3] = [-2.6, 0.0, 2.6];
/// Length of one track tile along Z.
pub const TILE_LENGTH: f32 = 12.0;
/// Number of tiles laid end to end; the track is recycled as it scrolls.
pub const TILE_COUNT: usize = 12;

pub struct LevelEntities {
    pub all: Vec<Entity>,
    pub player: Entity,
    pub camera: Entity,
    /// Track tiles that scroll and wrap around.
    pub track_tiles: Vec<Entity>,
    /// Side decoration that scrolls with the track.
    pub decor: Vec<Entity>,
}

pub fn spawn_level(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> LevelEntities {
    let mut all = Vec::new();
    let mut track_tiles = Vec::new();
    let mut decor = Vec::new();

    let asphalt = materials.add(StandardMaterial { base_color: Color::srgb(0.07, 0.08, 0.12), perceptual_roughness: 0.6, ..default() });
    let asphalt_alt = materials.add(StandardMaterial { base_color: Color::srgb(0.09, 0.1, 0.15), perceptual_roughness: 0.6, ..default() });
    let lane_line = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.9, 1.0),
        emissive: LinearRgba::rgb(0.6, 3.0, 4.0),
        ..default()
    });
    let edge = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.25, 0.75),
        emissive: LinearRgba::rgb(4.0, 0.6, 2.6),
        ..default()
    });
    let pylon = materials.add(StandardMaterial { base_color: Color::srgb(0.16, 0.17, 0.24), metallic: 0.6, perceptual_roughness: 0.35, ..default() });
    let ground = materials.add(StandardMaterial { base_color: Color::srgb(0.03, 0.03, 0.06), perceptual_roughness: 0.9, ..default() });

    let tile_mesh = meshes.add(Cuboid::new(8.4, 0.2, TILE_LENGTH));
    let line_mesh = meshes.add(Cuboid::new(0.08, 0.02, TILE_LENGTH * 0.45));
    let edge_mesh = meshes.add(Cuboid::new(0.18, 0.18, TILE_LENGTH));
    let pylon_mesh = meshes.add(Cuboid::new(0.6, 6.0, 0.6));
    let beam_mesh = meshes.add(Cuboid::new(10.0, 0.35, 0.35));

    // Dark ground far below the track so the horizon is not empty.
    all.push(
        commands
            .spawn((
                Name::new("Ground"),
                Mesh3d(meshes.add(Plane3d::default().mesh().size(400.0, 400.0))),
                MeshMaterial3d(ground),
                Transform::from_xyz(0.0, -1.5, -80.0),
            ))
            .id(),
    );

    for i in 0..TILE_COUNT {
        let z = -(i as f32) * TILE_LENGTH;
        let tile = commands
            .spawn((
                Name::new(format!("Track Tile {i}")),
                Mesh3d(tile_mesh.clone()),
                MeshMaterial3d(if i % 2 == 0 { asphalt.clone() } else { asphalt_alt.clone() }),
                Transform::from_xyz(0.0, -0.1, z),
            ))
            .id();
        for x in [-1.3, 1.3] {
            commands.spawn((
                Name::new("Lane Line"),
                Mesh3d(line_mesh.clone()),
                MeshMaterial3d(lane_line.clone()),
                Transform::from_xyz(x, 0.11, 0.0),
                ChildOf(tile),
            ));
        }
        for x in [-4.2, 4.2] {
            commands.spawn((
                Name::new("Track Edge"),
                Mesh3d(edge_mesh.clone()),
                MeshMaterial3d(edge.clone()),
                Transform::from_xyz(x, 0.1, 0.0),
                ChildOf(tile),
            ));
        }
        all.push(tile);
        track_tiles.push(tile);

        // Every third tile gets a neon arch made of two pylons and a beam.
        if i % 3 == 0 {
            let arch = commands.spawn((Name::new(format!("Arch {i}")), Transform::from_xyz(0.0, 0.0, z), Visibility::default())).id();
            for x in [-5.0, 5.0] {
                commands.spawn((
                    Name::new("Pylon"),
                    Mesh3d(pylon_mesh.clone()),
                    MeshMaterial3d(pylon.clone()),
                    Transform::from_xyz(x, 3.0, 0.0),
                    ChildOf(arch),
                ));
            }
            commands.spawn((
                Name::new("Arch Beam"),
                Mesh3d(beam_mesh.clone()),
                MeshMaterial3d(edge.clone()),
                Transform::from_xyz(0.0, 6.0, 0.0),
                ChildOf(arch),
            ));
            commands.spawn((
                Name::new("Arch Light"),
                PointLight { color: Color::srgb(1.0, 0.35, 0.8), intensity: 300_000.0, range: 16.0, ..default() },
                Transform::from_xyz(0.0, 5.0, 0.0),
                ChildOf(arch),
            ));
            all.push(arch);
            decor.push(arch);
        }
    }

    let sun = commands
        .spawn((
            Name::new("Moon Light"),
            DirectionalLight { color: Color::srgb(0.65, 0.75, 1.0), illuminance: 6_000.0, shadow_maps_enabled: true, ..default() },
            Transform::from_xyz(-6.0, 12.0, 6.0).looking_at(Vec3::new(0.0, 0.0, -10.0), Vec3::Y),
        ))
        .id();
    all.push(sun);

    let player = commands
        .spawn((
            Name::new("Runner"),
            Mesh3d(meshes.add(Capsule3d::new(0.35, 0.9))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.95, 1.0),
                emissive: LinearRgba::rgb(0.2, 0.8, 1.2),
                metallic: 0.2,
                perceptual_roughness: 0.3,
                ..default()
            })),
            Transform::from_xyz(LANES[1], 0.8, 0.0),
        ))
        .id();
    all.push(player);

    let camera = commands
        .spawn((
            Name::new("Chase Camera"),
            Camera3d::default(),
            Transform::from_xyz(0.0, 3.6, 7.5).looking_at(Vec3::new(0.0, 1.0, -8.0), Vec3::Y),
        ))
        .id();
    all.push(camera);

    LevelEntities { all, player, camera, track_tiles, decor }
}

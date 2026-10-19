//! Static level for Arena Survivor: arena floor, walls, pillars, corner pylons, lights,
//! the player's hover-tank body and the camera.
//!
//! The Bevylite editor includes this file into its own crate via `#[path]`, so it only uses
//! the Bevy prelude and carries no gameplay components. `main.rs` attaches those at Startup.
use bevy::prelude::*;

/// Handles to the entities gameplay code needs, plus layout data for spawning and collision.
pub struct LevelEntities {
    /// Every top-level entity spawned by [`spawn_level`].
    pub all: Vec<Entity>,
    /// The hover-tank root (body mesh); move this entity to move the player.
    pub player: Entity,
    /// The turret child of the player; rotate it around Y to aim.
    pub turret: Entity,
    pub camera: Entity,
    /// Distance from the arena center to the inside face of the walls.
    pub arena_half_size: f32,
    /// Points just inside the walls where enemies enter.
    pub spawn_points: Vec<Vec3>,
    /// Round obstacles on the ground plane as `(center XZ, radius)`.
    pub obstacles: Vec<(Vec2, f32)>,
    /// Height of the player's hover body above the floor.
    pub player_hover_height: f32,
    /// Camera offset from its focus point: `(distance, pitch in radians)`.
    pub camera_distance: f32,
    pub camera_pitch: f32,
}

const PLAYER_HOVER_HEIGHT: f32 = 0.55;
const CAMERA_DISTANCE: f32 = 30.0;
const CAMERA_PITCH_DEGREES: f32 = 55.0;

const HALF: f32 = 18.0;
const WALL_THICKNESS: f32 = 0.8;

fn mat(materials: &mut Assets<StandardMaterial>, color: Color, metallic: f32, roughness: f32) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial { base_color: color, metallic, perceptual_roughness: roughness, ..default() })
}

fn glow(materials: &mut Assets<StandardMaterial>, color: Color, emissive: LinearRgba) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial { base_color: color, emissive, ..default() })
}

/// Spawns the whole static arena and returns the entities gameplay needs.
pub fn spawn_level(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> LevelEntities {
    let mut all = Vec::new();

    // Palette: deep navy floor, steel walls, cyan grid and pylons, warm amber accents.
    let floor_dark = mat(materials, Color::srgb(0.07, 0.09, 0.13), 0.2, 0.85);
    let floor_tile = mat(materials, Color::srgb(0.10, 0.13, 0.19), 0.2, 0.75);
    let grid_line = glow(materials, Color::srgb(0.1, 0.35, 0.45), LinearRgba::rgb(0.0, 0.25, 0.35));
    let steel = mat(materials, Color::srgb(0.30, 0.34, 0.42), 0.7, 0.4);
    let wall_trim = glow(materials, Color::srgb(0.2, 0.8, 1.0), LinearRgba::rgb(0.0, 1.6, 2.4));
    let pillar_mat = mat(materials, Color::srgb(0.22, 0.25, 0.32), 0.6, 0.5);
    let amber = glow(materials, Color::srgb(1.0, 0.6, 0.15), LinearRgba::rgb(2.4, 1.1, 0.15));
    let pylon_core = glow(materials, Color::srgb(0.3, 0.9, 1.0), LinearRgba::rgb(0.5, 4.0, 6.0));
    let hull = mat(materials, Color::srgb(0.85, 0.88, 0.92), 0.5, 0.35);
    let hull_accent = glow(materials, Color::srgb(0.2, 0.9, 1.0), LinearRgba::rgb(0.0, 1.8, 2.6));
    let turret_mat = mat(materials, Color::srgb(0.25, 0.28, 0.35), 0.8, 0.3);

    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Floor: one dark base slab with a checker of slightly lighter tiles and thin glowing lines.
    let floor = commands
        .spawn((
            Name::new("Arena Floor"),
            Mesh3d(cube.clone()),
            MeshMaterial3d(floor_dark),
            Transform::from_xyz(0.0, -0.25, 0.0).with_scale(Vec3::new(HALF * 2.0 + WALL_THICKNESS * 2.0, 0.5, HALF * 2.0 + WALL_THICKNESS * 2.0)),
        ))
        .id();
    all.push(floor);

    let tiles = commands.spawn((Name::new("Floor Tiles"), Transform::default(), Visibility::Inherited)).id();
    all.push(tiles);
    let tile = 4.0;
    let count = (HALF * 2.0 / tile) as i32;
    for x in 0..count {
        for z in 0..count {
            if (x + z) % 2 == 0 {
                let center = Vec3::new(-HALF + tile * (x as f32 + 0.5), 0.005, -HALF + tile * (z as f32 + 0.5));
                commands.spawn((
                    Name::new(format!("Tile {x}-{z}")),
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(floor_tile.clone()),
                    Transform::from_translation(center).with_scale(Vec3::new(tile, 0.01, tile)),
                    ChildOf(tiles),
                ));
            }
        }
    }
    let grid = commands.spawn((Name::new("Floor Grid"), Transform::default(), Visibility::Inherited)).id();
    all.push(grid);
    for i in 0..=count {
        let offset = -HALF + tile * i as f32;
        commands.spawn((
            Name::new(format!("Grid Line X {i}")),
            Mesh3d(cube.clone()),
            MeshMaterial3d(grid_line.clone()),
            Transform::from_xyz(offset, 0.015, 0.0).with_scale(Vec3::new(0.06, 0.01, HALF * 2.0)),
            ChildOf(grid),
        ));
        commands.spawn((
            Name::new(format!("Grid Line Z {i}")),
            Mesh3d(cube.clone()),
            MeshMaterial3d(grid_line.clone()),
            Transform::from_xyz(0.0, 0.015, offset).with_scale(Vec3::new(HALF * 2.0, 0.01, 0.06)),
            ChildOf(grid),
        ));
    }

    // Low perimeter walls with a glowing trim strip on top.
    let walls = commands.spawn((Name::new("Perimeter Walls"), Transform::default(), Visibility::Inherited)).id();
    all.push(walls);
    let wall_len = HALF * 2.0 + WALL_THICKNESS * 2.0;
    let edge = HALF + WALL_THICKNESS * 0.5;
    for (name, position, size) in [
        ("Wall North", Vec3::new(0.0, 0.5, -edge), Vec3::new(wall_len, 1.0, WALL_THICKNESS)),
        ("Wall South", Vec3::new(0.0, 0.5, edge), Vec3::new(wall_len, 1.0, WALL_THICKNESS)),
        ("Wall West", Vec3::new(-edge, 0.5, 0.0), Vec3::new(WALL_THICKNESS, 1.0, wall_len)),
        ("Wall East", Vec3::new(edge, 0.5, 0.0), Vec3::new(WALL_THICKNESS, 1.0, wall_len)),
    ] {
        let wall = commands
            .spawn((Name::new(name), Mesh3d(cube.clone()), MeshMaterial3d(steel.clone()), Transform::from_translation(position).with_scale(size), ChildOf(walls)))
            .id();
        // Trim lives in the wall's unit space: a thin slab on top.
        commands.spawn((
            Name::new(format!("{name} Trim")),
            Mesh3d(cube.clone()),
            MeshMaterial3d(wall_trim.clone()),
            Transform::from_xyz(0.0, 0.52, 0.0).with_scale(Vec3::new(1.0, 0.06, 0.4)),
            ChildOf(wall),
        ));
    }

    // Pillars: steel cylinders with an amber band. They block movement and bullets.
    let pillar_mesh = meshes.add(Cylinder::new(1.0, 2.4));
    let band_mesh = meshes.add(Cylinder::new(1.06, 0.18));
    let pillars = commands.spawn((Name::new("Pillars"), Transform::default(), Visibility::Inherited)).id();
    all.push(pillars);
    let mut obstacles = Vec::new();
    for (i, (x, z)) in [(-8.0, -6.0), (8.0, -6.0), (-8.0, 6.0), (8.0, 6.0), (0.0, -11.0), (0.0, 11.0)].into_iter().enumerate() {
        let pillar = commands
            .spawn((
                Name::new(format!("Pillar {}", i + 1)),
                Mesh3d(pillar_mesh.clone()),
                MeshMaterial3d(pillar_mat.clone()),
                Transform::from_xyz(x, 1.2, z),
                ChildOf(pillars),
            ))
            .id();
        commands.spawn((Name::new("Amber Band"), Mesh3d(band_mesh.clone()), MeshMaterial3d(amber.clone()), Transform::from_xyz(0.0, 0.5, 0.0), ChildOf(pillar)));
        obstacles.push((Vec2::new(x, z), 1.0));
    }

    // Corner pylons: glowing cores on steel bases, each with a cyan point light.
    let pylon_base = meshes.add(Cylinder::new(0.7, 0.6));
    let pylon_mesh = meshes.add(Cylinder::new(0.28, 3.2));
    let pylons = commands.spawn((Name::new("Corner Pylons"), Transform::default(), Visibility::Inherited)).id();
    all.push(pylons);
    let corner = HALF - 1.2;
    for (label, x, z) in [("NW", -corner, -corner), ("NE", corner, -corner), ("SW", -corner, corner), ("SE", corner, corner)] {
        let base = commands
            .spawn((Name::new(format!("Pylon {label}")), Mesh3d(pylon_base.clone()), MeshMaterial3d(steel.clone()), Transform::from_xyz(x, 0.3, z), ChildOf(pylons)))
            .id();
        commands.spawn((Name::new("Pylon Core"), Mesh3d(pylon_mesh.clone()), MeshMaterial3d(pylon_core.clone()), Transform::from_xyz(0.0, 1.9, 0.0), ChildOf(base)));
        commands.spawn((
            Name::new("Pylon Light"),
            PointLight { color: Color::srgb(0.3, 0.85, 1.0), intensity: 400_000.0, range: 14.0, ..default() },
            Transform::from_xyz(0.0, 3.0, 0.0),
            ChildOf(base),
        ));
        obstacles.push((Vec2::new(x, z), 0.7));
    }

    let sun = commands
        .spawn((
            Name::new("Sun"),
            DirectionalLight { illuminance: 6_000.0, color: Color::srgb(0.85, 0.9, 1.0), shadow_maps_enabled: true, ..default() },
            Transform::from_xyz(10.0, 20.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();
    all.push(sun);

    // Player hover tank: a white hull with cyan skirt lights and a rotating turret.
    let player = commands
        .spawn((
            Name::new("Player"),
            Mesh3d(meshes.add(Cuboid::new(1.3, 0.45, 1.5))),
            MeshMaterial3d(hull),
            Transform::from_xyz(0.0, PLAYER_HOVER_HEIGHT, 0.0),
        ))
        .id();
    all.push(player);
    commands.spawn((
        Name::new("Hover Skirt"),
        Mesh3d(meshes.add(Cuboid::new(1.45, 0.08, 1.65))),
        MeshMaterial3d(hull_accent.clone()),
        Transform::from_xyz(0.0, -0.22, 0.0),
        ChildOf(player),
    ));
    let turret = commands
        .spawn((
            Name::new("Turret"),
            Mesh3d(meshes.add(Cylinder::new(0.42, 0.32))),
            MeshMaterial3d(turret_mat.clone()),
            Transform::from_xyz(0.0, 0.36, 0.0),
            ChildOf(player),
        ))
        .id();
    // The barrel points down local -Z, matching `Transform::forward`.
    commands.spawn((
        Name::new("Barrel"),
        Mesh3d(meshes.add(Cuboid::new(0.16, 0.16, 0.9))),
        MeshMaterial3d(turret_mat),
        Transform::from_xyz(0.0, 0.02, -0.6),
        ChildOf(turret),
    ));
    commands.spawn((
        Name::new("Muzzle Light"),
        Mesh3d(meshes.add(Cuboid::new(0.18, 0.18, 0.06))),
        MeshMaterial3d(hull_accent),
        Transform::from_xyz(0.0, 0.02, -1.05),
        ChildOf(turret),
    ));

    // Angled top-down camera (55 degree pitch) looking at the arena center.
    let pitch = CAMERA_PITCH_DEGREES.to_radians();
    let distance = CAMERA_DISTANCE;
    let camera = commands
        .spawn((
            Name::new("Camera"),
            Camera3d::default(),
            Transform::from_xyz(0.0, distance * pitch.sin(), distance * pitch.cos()).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();
    all.push(camera);

    // Enemy entry points: evenly spaced along each wall, just inside it.
    let mut spawn_points = Vec::new();
    let inset = HALF - 1.0;
    for i in 0..4 {
        let t = -HALF * 0.75 + i as f32 * HALF * 0.5;
        spawn_points.push(Vec3::new(t, 0.0, -inset));
        spawn_points.push(Vec3::new(t, 0.0, inset));
        spawn_points.push(Vec3::new(-inset, 0.0, t));
        spawn_points.push(Vec3::new(inset, 0.0, t));
    }

    LevelEntities {
        all,
        player,
        turret,
        camera,
        arena_half_size: HALF,
        spawn_points,
        obstacles,
        player_hover_height: PLAYER_HOVER_HEIGHT,
        camera_distance: distance,
        camera_pitch: pitch,
    }
}

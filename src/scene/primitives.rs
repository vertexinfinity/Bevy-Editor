//! Helper systems to spawn 3D primitive shapes and starter objects.

use bevy::prelude::*;

/// Spawns a starter scene with ground, lighting, and sample props.
pub fn spawn_starter_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Directional Sun Light
    commands.spawn((
        Name::new("Sun Light"),
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground Plane
    commands.spawn((
        Name::new("Ground Plane"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.22, 0.25),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::IDENTITY,
    ));

    // Center Cube
    commands.spawn((
        Name::new("Cube"),
        Mesh3d(meshes.add(Cuboid::new(1.5, 1.5, 1.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.6, 0.9),
            perceptual_roughness: 0.3,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.75, 0.0),
    ));

    // Sphere
    commands.spawn((
        Name::new("Sphere"),
        Mesh3d(meshes.add(Sphere::new(0.8))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.3, 0.3),
            perceptual_roughness: 0.2,
            ..default()
        })),
        Transform::from_xyz(3.0, 0.8, 0.0),
    ));
}

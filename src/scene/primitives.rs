//! Helper systems to spawn 3D primitive shapes and starter objects.

use bevy::prelude::*;
use crate::project::scaffold::ProjectTemplateKind;
use crate::scene::components::*;
use crate::scene::SceneEntity;

/// Direct starter scene spawner used by system or procedural callers.
pub fn spawn_starter_scene_direct(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Directional Sun Light
    commands.spawn((
        SceneEntity,
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
        SceneEntity,
        Name::new("Ground Plane"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.22, 0.25),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::IDENTITY,
        bevy::picking::Pickable::IGNORE,
    ));

    // Center Cube
    commands.spawn((
        SceneEntity,
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
        SceneEntity,
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

/// Spawns a starter scene with ground, lighting, and sample props.
pub fn spawn_starter_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_starter_scene_direct(&mut commands, &mut meshes, &mut materials);
}

/// Spawns the rich starter scene tailored to the selected project template.
pub fn spawn_template_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    template: ProjectTemplateKind,
) {
    match template {
        ProjectTemplateKind::FirstPerson => {
            // 1. Sun Light
            commands.spawn((
                SceneEntity,
                Name::new("Sun Light"),
                DirectionalLight {
                    illuminance: 14_000.0,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_xyz(8.0, 16.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));

            // 2. Ground Arena
            commands.spawn((
                SceneEntity,
                Name::new("Ground Arena"),
                Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.18, 0.20, 0.24),
                    perceptual_roughness: 0.85,
                    ..default()
                })),
                Transform::IDENTITY,
                bevy::picking::Pickable::IGNORE,
            ));

            // 3. Obstacle crates / Cover geometry
            let crates = [
                ("Cover Crate A", Vec3::new(3.5, 1.0, -5.0), Vec3::new(2.0, 2.0, 2.0), Color::srgb(0.9, 0.5, 0.1)),
                ("Cover Pillar B", Vec3::new(-4.5, 2.0, -6.0), Vec3::new(1.8, 4.0, 1.8), Color::srgb(0.1, 0.7, 0.9)),
                ("Barrier Wall C", Vec3::new(0.0, 1.0, -11.0), Vec3::new(6.0, 2.0, 1.2), Color::srgb(0.6, 0.2, 0.8)),
                ("Tactical Block D", Vec3::new(-6.0, 1.25, 4.0), Vec3::new(2.5, 2.5, 2.5), Color::srgb(0.1, 0.8, 0.4)),
                ("Low Platform E", Vec3::new(6.0, 0.5, 4.0), Vec3::new(4.0, 1.0, 3.0), Color::srgb(0.95, 0.8, 0.2)),
            ];

            for (name, pos, size, color) in crates {
                commands.spawn((
                    SceneEntity,
                    Name::new(name),
                    Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        perceptual_roughness: 0.4,
                        ..default()
                    })),
                    Transform::from_translation(pos),
                ));
            }

            // 4. Target Core (Spinning Floating Sphere)
            commands.spawn((
                SceneEntity,
                Name::new("Target Core"),
                SpinAnimation {
                    speed: 2.0,
                    axis: Vec3::Y,
                },
                Mesh3d(meshes.add(Sphere::new(0.9))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.95, 0.2, 0.3),
                    emissive: LinearRgba::rgb(0.8, 0.1, 0.2),
                    metallic: 0.7,
                    perceptual_roughness: 0.2,
                    ..default()
                })),
                Transform::from_xyz(0.0, 2.2, -16.0),
            ));

            // 5. Dedicated First-Person Camera
            commands.spawn((
                SceneEntity,
                Name::new("FPS Camera"),
                Camera3d::default(),
                FpsController {
                    speed: 10.0,
                    sensitivity: 0.003,
                    pitch: 0.0,
                    yaw: 0.0,
                },
                ScriptComponent {
                    script_file: "scripts/fps_controller.lua".to_string(),
                    ..default()
                },
                Transform::from_xyz(0.0, 1.8, 5.0).looking_at(Vec3::new(0.0, 1.8, 0.0), Vec3::Y),
            ));
        }

        ProjectTemplateKind::Action3D => {
            // 1. Sun Light
            commands.spawn((
                SceneEntity,
                Name::new("Sun Light"),
                DirectionalLight {
                    illuminance: 12_000.0,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_xyz(6.0, 12.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));

            // 2. Ground Arena
            commands.spawn((
                SceneEntity,
                Name::new("Ground Arena"),
                Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.22, 0.26),
                    perceptual_roughness: 0.8,
                    ..default()
                })),
                Transform::IDENTITY,
                bevy::picking::Pickable::IGNORE,
            ));

            // 3. Playable Character Hero
            commands.spawn((
                SceneEntity,
                Name::new("Player Hero"),
                PlayerController {
                    speed: 8.5,
                    jump_force: 10.0,
                },
                GameplayTag {
                    tag: "Player".to_string(),
                },
                ScriptComponent {
                    script_file: "scripts/player.lua".to_string(),
                    ..default()
                },
                Mesh3d(meshes.add(Capsule3d::new(0.4, 1.2))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.75, 0.95),
                    perceptual_roughness: 0.3,
                    metallic: 0.1,
                    ..default()
                })),
                Transform::from_xyz(0.0, 1.0, 0.0),
            ));

            // 4. Smooth Follow Camera
            commands.spawn((
                SceneEntity,
                Name::new("Follow Camera"),
                Camera3d::default(),
                FollowCamera {
                    offset: Vec3::new(0.0, 4.0, 8.0),
                    smooth_speed: 10.0,
                },
                Transform::from_xyz(0.0, 4.0, 8.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
            ));

            // 5. Spinning Collectible Gems
            for i in 0..6 {
                let angle = (i as f32) * std::f32::consts::TAU / 6.0;
                let radius = 6.5;
                commands.spawn((
                    SceneEntity,
                    Name::new(format!("Collectible Gem {}", i + 1)),
                    SpinAnimation {
                        speed: 1.8,
                        axis: Vec3::Y,
                    },
                    Mesh3d(meshes.add(Cuboid::new(0.8, 0.8, 0.8))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.95, 0.75, 0.2),
                        metallic: 0.8,
                        perceptual_roughness: 0.2,
                        ..default()
                    })),
                    Transform::from_xyz(angle.cos() * radius, 1.2, angle.sin() * radius),
                ));
            }
        }

        ProjectTemplateKind::OrbitViewer => {
            // 1. Key Light & Fill Light
            commands.spawn((
                SceneEntity,
                Name::new("Key Light"),
                DirectionalLight {
                    illuminance: 15_000.0,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_xyz(5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));

            commands.spawn((
                SceneEntity,
                Name::new("Fill Light"),
                PointLight {
                    intensity: 6_000.0,
                    color: Color::srgb(0.7, 0.85, 1.0),
                    ..default()
                },
                Transform::from_xyz(-5.0, 4.0, -5.0),
            ));

            // 2. Showcase Pedestal
            commands.spawn((
                SceneEntity,
                Name::new("Showcase Pedestal"),
                Mesh3d(meshes.add(Cylinder::new(2.5, 0.3))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.15, 0.16, 0.18),
                    perceptual_roughness: 0.6,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.15, 0.0),
                bevy::picking::Pickable::IGNORE,
            ));

            // 3. Showcase Centerpiece
            commands.spawn((
                SceneEntity,
                Name::new("Showcase Centerpiece"),
                SpinAnimation {
                    speed: 0.6,
                    axis: Vec3::Y,
                },
                Mesh3d(meshes.add(Sphere::new(1.1))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.9, 0.7, 0.2),
                    metallic: 0.9,
                    perceptual_roughness: 0.15,
                    ..default()
                })),
                Transform::from_xyz(0.0, 1.5, 0.0),
            ));

            // 4. Orbit Turntable Camera
            commands.spawn((
                SceneEntity,
                Name::new("Orbit Camera"),
                Camera3d::default(),
                OrbitController {
                    distance: 8.5,
                    yaw: 0.6,
                    pitch: 0.35,
                    sensitivity: 0.004,
                },
                Transform::from_xyz(0.0, 3.0, 8.5).looking_at(Vec3::new(0.0, 1.3, 0.0), Vec3::Y),
            ));
        }

        ProjectTemplateKind::Strategy => {
            // 1. Sun Light
            commands.spawn((
                SceneEntity,
                Name::new("Sun Light"),
                DirectionalLight {
                    illuminance: 12_000.0,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));

            // 2. Terrain Grid
            commands.spawn((
                SceneEntity,
                Name::new("Battlefield Grid"),
                Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.16, 0.22, 0.18),
                    perceptual_roughness: 0.9,
                    ..default()
                })),
                Transform::IDENTITY,
                bevy::picking::Pickable::IGNORE,
            ));

            // 3. Base Structures
            commands.spawn((
                SceneEntity,
                Name::new("Command Center"),
                Mesh3d(meshes.add(Cuboid::new(3.5, 2.0, 3.5))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.5, 0.9),
                    perceptual_roughness: 0.3,
                    ..default()
                })),
                Transform::from_xyz(0.0, 1.0, 0.0),
            ));

            commands.spawn((
                SceneEntity,
                Name::new("Power Relay"),
                Mesh3d(meshes.add(Cuboid::new(1.8, 3.0, 1.8))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.9, 0.6, 0.1),
                    perceptual_roughness: 0.4,
                    ..default()
                })),
                Transform::from_xyz(5.0, 1.5, 3.0),
            ));

            commands.spawn((
                SceneEntity,
                Name::new("Destination Beacon"),
                SpinAnimation {
                    speed: 2.0,
                    axis: Vec3::Y,
                },
                Mesh3d(meshes.add(Cylinder::new(0.25, 4.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.1, 0.95, 0.5),
                    emissive: LinearRgba::rgb(0.1, 0.8, 0.4),
                    ..default()
                })),
                Transform::from_xyz(0.0, 2.0, -10.0),
            ));

            // 4. RTS Strategy Camera
            commands.spawn((
                SceneEntity,
                Name::new("RTS Camera"),
                Camera3d::default(),
                RtsController {
                    speed: 22.0,
                },
                Transform::from_xyz(0.0, 24.0, 24.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));
        }

        ProjectTemplateKind::Blank => {
            spawn_starter_scene_direct(commands, meshes, materials);
        }
    }
}

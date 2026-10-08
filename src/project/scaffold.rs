//! Standalone Project Scaffolding Engine.
//!
//! Generates clean, minimal Bevy game projects that build and run independently
//! without any editor dependencies or compilation errors.

use std::fs;
use std::path::{Path, PathBuf};
use bevy::prelude::*;

pub struct ProjectScaffolder;

impl ProjectScaffolder {
    /// Generates a standalone, clean Bevy game project at `target_dir`.
    pub fn create_project(target_dir: &Path, project_name: &str) -> Result<PathBuf, String> {
        let clean_name = project_name.trim().replace(' ', "_").to_lowercase();
        let project_root = target_dir.join(&clean_name);

        fs::create_dir_all(project_root.join("src"))
            .map_err(|e| format!("Failed to create project src dir: {e}"))?;
        fs::create_dir_all(project_root.join("assets"))
            .map_err(|e| format!("Failed to create project assets dir: {e}"))?;
        fs::create_dir_all(project_root.join("scripts"))
            .map_err(|e| format!("Failed to create project scripts dir: {e}"))?;

        // 1. Cargo.toml
        // Note: [workspace] with empty members ensures Cargo treats this project as its own root workspace,
        // preventing "current package believes it's in a workspace when it's not" errors if placed inside any parent folder with a Cargo.toml.
        let cargo_toml = format!(
            r#"[package]
name = "{clean_name}"
version = "0.1.0"
edition = "2024"

[workspace]

[dependencies]
bevy = "0.15"

[profile.dev]
opt-level = 1

[profile.release]
opt-level = 3
lto = "thin"
"#
        );
        fs::write(project_root.join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("Failed to write Cargo.toml: {e}"))?;

        // 2. src/main.rs (Clean, zero-error standalone Bevy game)
        let main_rs = format!(
            r#"//! Standalone Bevy Game created by Bevy Editor.

use bevy::prelude::*;

fn main() {{
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {{
            primary_window: Some(Window {{
                title: "{project_name}".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }}),
            ..default()
        }}))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, rotate_demo_object)
        .run();
}}

#[derive(Component)]
struct DemoSpinner;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {{
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.0, 7.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // Sun light
    commands.spawn((
        DirectionalLight {{
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        }},
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(15.0, 15.0))),
        MeshMaterial3d(materials.add(StandardMaterial {{
            base_color: Color::srgb(0.2, 0.22, 0.26),
            perceptual_roughness: 0.8,
            ..default()
        }})),
        Transform::IDENTITY,
    ));

    // Centered interactive object
    commands.spawn((
        DemoSpinner,
        Mesh3d(meshes.add(Cuboid::new(1.4, 1.4, 1.4))),
        MeshMaterial3d(materials.add(StandardMaterial {{
            base_color: Color::srgb(0.25, 0.65, 0.95),
            perceptual_roughness: 0.3,
            ..default()
        }})),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
}}

fn rotate_demo_object(time: Res<Time>, mut query: Query<&mut Transform, With<DemoSpinner>>) {{
    for mut transform in &mut query {{
        transform.rotate_y(1.2 * time.delta_secs());
    }}
}}
"#
        );
        fs::write(project_root.join("src/main.rs"), main_rs)
            .map_err(|e| format!("Failed to write src/main.rs: {e}"))?;

        // 3. assets/README.md & scripts/demo.lua
        let script_demo = r#"-- Demo Script
function on_update(entity, delta_time)
    -- live script behavior
end
"#;
        let _ = fs::write(project_root.join("scripts/demo.lua"), script_demo);

        let gitignore = "/target\nCargo.lock\n";
        let _ = fs::write(project_root.join(".gitignore"), gitignore);

        info!("Created clean standalone Bevy project at: {:?}", project_root);
        Ok(project_root)
    }
}

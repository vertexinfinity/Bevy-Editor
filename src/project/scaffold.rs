//! Standalone Project Scaffolding Engine.
//!
//! Generates clean, minimal Bevy game projects that build and run independently
//! without any editor dependencies or compilation errors.

use std::fs;
use std::path::{Path, PathBuf};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectTemplateKind {
    Blank,
    #[default]
    Action3D,
    FirstPerson,
    OrbitViewer,
    Strategy,
}

impl ProjectTemplateKind {
    pub fn from_id(id: &str) -> Self {
        match id {
            "3d_game" | "action3d" => Self::Action3D,
            "first_person" | "fps" => Self::FirstPerson,
            "third_person" | "orbit_viewer" | "orbital" => Self::OrbitViewer,
            "strategy" | "rts" => Self::Strategy,
            _ => Self::Blank,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Blank => "Blank Canvas",
            Self::Action3D => "3D Action / Character Controller",
            Self::FirstPerson => "First Person / FPS Controller",
            Self::OrbitViewer => "Orbit 3D Scene / Model Viewer",
            Self::Strategy => "Top-Down / Strategy (RTS)",
        }
    }
}

pub struct ProjectScaffolder;

impl ProjectScaffolder {
    /// Generates a standalone, clean Bevy game project at `target_dir` with the default 3D template.
    pub fn create_project(target_dir: &Path, project_name: &str) -> Result<PathBuf, String> {
        Self::create_project_with_template(target_dir, project_name, ProjectTemplateKind::Action3D)
    }

    /// Generates a standalone, clean Bevy game project at `target_dir` with a chosen starter template.
    pub fn create_project_with_template(
        target_dir: &Path,
        project_name: &str,
        template: ProjectTemplateKind,
    ) -> Result<PathBuf, String> {
        let clean_name = project_name.trim().replace(' ', "_").to_lowercase();
        let project_root = target_dir.join(&clean_name);

        fs::create_dir_all(project_root.join("src"))
            .map_err(|e| format!("Failed to create project src dir: {e}"))?;
        fs::create_dir_all(project_root.join("assets").join("scenes"))
            .map_err(|e| format!("Failed to create project assets/scenes dir: {e}"))?;
        fs::create_dir_all(project_root.join("scripts"))
            .map_err(|e| format!("Failed to create project scripts dir: {e}"))?;

        // 1. Cargo.toml
        // Compatible with Bevy git main (0.20-dev) used by the editor, with serialization enabled.
        let cargo_toml = format!(
            r#"[package]
name = "{clean_name}"
version = "0.1.0"
edition = "2024"

[workspace]

[dependencies]
bevy = {{ git = "https://github.com/bevyengine/bevy", branch = "main", default-features = false, features = [
    "3d",
    "serialize",
    "png",
    "jpeg",
    "tonemapping_luts",
] }}

[profile.dev]
opt-level = 1

[profile.release]
opt-level = 3
lto = "thin"
"#
        );
        fs::write(project_root.join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("Failed to write Cargo.toml: {e}"))?;

        // 2. src/main.rs tailored to the selected template
        let main_rs = Self::generate_main_rs(project_name, template);
        fs::write(project_root.join("src/main.rs"), main_rs)
            .map_err(|e| format!("Failed to write src/main.rs: {e}"))?;

        // 3. assets/scenes/default_scene.bsn
        let default_scene_ron = Self::generate_starter_scene_bsn(template);
        let scene_path = project_root.join("assets/scenes/default_scene.bsn");
        let _ = fs::write(scene_path, default_scene_ron);

        // 4. scripts/demo.lua & .gitignore
        let script_demo = r#"-- Demo Script for Bevy Editor Project
function on_update(entity, delta_time)
    -- live script behavior
end
"#;
        let _ = fs::write(project_root.join("scripts/demo.lua"), script_demo);

        let gitignore = "/target\nCargo.lock\n";
        let _ = fs::write(project_root.join(".gitignore"), gitignore);

        info!("Created standalone Bevy project [{}] at: {:?}", template.display_name(), project_root);
        Ok(project_root)
    }

    fn generate_starter_scene_bsn(_template: ProjectTemplateKind) -> String {
        // Minimal valid DynamicWorld scene file in modern RON format
        r#"(
  resources: {},
  entities: {
    "1v0": (
      components: {
        "bevy_camera::visibility::InheritedVisibility": (true),
        "bevy_camera::visibility::Visibility": Inherited,
        "bevy_ecs::name::Name": "Sun Light",
        "bevy_light::directional_light::DirectionalLight": (
          illuminance: 12000.0,
          shadow_maps_enabled: true,
        ),
        "bevy_transform::components::global_transform::GlobalTransform": ((1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 4.0, 10.0, 4.0)),
        "bevy_transform::components::transform::Transform": (
          translation: (4.0, 10.0, 4.0),
          rotation: (-0.38268343, 0.0, 0.0, 0.9238795),
          scale: (1.0, 1.0, 1.0),
        ),
      },
    ),
  },
)
"#.to_string()
    }

    fn generate_main_rs(project_name: &str, template: ProjectTemplateKind) -> String {
        match template {
            ProjectTemplateKind::Action3D => format!(
                r#"//! Standalone 3D Action Game created by Bevy Editor.

use bevy::prelude::*;
use bevy::world_serialization::DynamicWorldRoot;

fn main() {{
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {{
            primary_window: Some(Window {{
                title: "{project_name} - 3D Action".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }}),
            ..default()
        }}))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (player_movement, follow_camera, rotate_spinning_props))
        .run();
}}

#[derive(Component)]
struct Player {{
    speed: f32,
}}

#[derive(Component)]
struct FollowCamera {{
    offset: Vec3,
}}

#[derive(Component)]
struct SpinProp;

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {{
    // 1. Sun light
    commands.spawn((
        DirectionalLight {{
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        }},
        Transform::from_xyz(6.0, 12.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 2. Playable Character
    commands.spawn((
        Player {{ speed: 8.0 }},
        Mesh3d(meshes.add(Capsule3d::new(0.4, 1.2))),
        MeshMaterial3d(materials.add(StandardMaterial {{
            base_color: Color::srgb(0.2, 0.75, 0.95),
            perceptual_roughness: 0.3,
            ..default()
        }})),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));

    // 3. Smooth Third-Person Follow Camera
    commands.spawn((
        Camera3d::default(),
        FollowCamera {{
            offset: Vec3::new(0.0, 4.0, 8.0),
        }},
        Transform::from_xyz(0.0, 4.0, 8.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // 4. Ground Arena
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(materials.add(StandardMaterial {{
            base_color: Color::srgb(0.18, 0.20, 0.24),
            perceptual_roughness: 0.85,
            ..default()
        }})),
        Transform::IDENTITY,
    ));

    // 5. Interactive Spinning Collectibles
    for i in 0..6 {{
        let angle = (i as f32) * std::f32::consts::TAU / 6.0;
        let radius = 6.0;
        commands.spawn((
            SpinProp,
            Mesh3d(meshes.add(Cuboid::new(0.8, 0.8, 0.8))),
            MeshMaterial3d(materials.add(StandardMaterial {{
                base_color: Color::srgb(0.95, 0.75, 0.2),
                metallic: 0.6,
                perceptual_roughness: 0.2,
                ..default()
            }})),
            Transform::from_xyz(angle.cos() * radius, 1.2, angle.sin() * radius),
        ));
    }}

    // 6. Load saved scene from Bevy Editor (overrides or augments environment)
    commands.spawn(DynamicWorldRoot(asset_server.load("scenes/default_scene.bsn")));
}}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Player)>,
) {{
    for (mut transform, player) in &mut query {{
        let mut move_dir = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {{
            move_dir.z -= 1.0;
        }}
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {{
            move_dir.z += 1.0;
        }}
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {{
            move_dir.x -= 1.0;
        }}
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {{
            move_dir.x += 1.0;
        }}

        if move_dir.length_squared() > 0.001 {{
            move_dir = move_dir.normalize();
            transform.translation += move_dir * player.speed * time.delta_secs();
            let target_rotation = Quat::from_rotation_y(move_dir.x.atan2(move_dir.z) + std::f32::consts::PI);
            transform.rotation = transform.rotation.slerp(target_rotation, 10.0 * time.delta_secs());
        }}
    }}
}}

fn follow_camera(
    player_query: Query<&Transform, (With<Player>, Without<FollowCamera>)>,
    mut camera_query: Query<(&mut Transform, &FollowCamera), (With<Camera3d>, Without<Player>)>,
    time: Res<Time>,
) {{
    if let Ok(player_trans) = player_query.single() {{
        for (mut cam_trans, follow) in &mut camera_query {{
            let desired_pos = player_trans.translation + follow.offset;
            cam_trans.translation = cam_trans.translation.lerp(desired_pos, 5.0 * time.delta_secs());
            cam_trans.look_at(player_trans.translation + Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
        }}
    }}
}}

fn rotate_spinning_props(time: Res<Time>, mut query: Query<&mut Transform, With<SpinProp>>) {{
    for mut transform in &mut query {{
        transform.rotate_y(1.5 * time.delta_secs());
    }}
}}
"#
            ),

            ProjectTemplateKind::FirstPerson => format!(
                r#"//! Standalone FPS Game created by Bevy Editor.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::world_serialization::DynamicWorldRoot;

fn main() {{
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {{
            primary_window: Some(Window {{
                title: "{project_name} - First Person".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }}),
            ..default()
        }}))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (fps_movement, fps_mouse_look, cursor_grab_system))
        .run();
}}

#[derive(Component)]
struct FpsController {{
    pitch: f32,
    yaw: f32,
}}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {{
    // 1. Sun light
    commands.spawn((
        DirectionalLight {{
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        }},
        Transform::from_xyz(5.0, 12.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 2. FPS Player + Camera
    commands.spawn((
        Camera3d::default(),
        FpsController {{ pitch: 0.0, yaw: 0.0 }},
        Transform::from_xyz(0.0, 1.7, 5.0).looking_at(Vec3::new(0.0, 1.7, 0.0), Vec3::Y),
    ));

    // 3. Ground arena
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {{
            base_color: Color::srgb(0.22, 0.24, 0.28),
            perceptual_roughness: 0.9,
            ..default()
        }})),
        Transform::IDENTITY,
    ));

    // 4. Obstacles / Arena Pillars
    for x in [-8.0, 8.0] {{
        for z in [-8.0, 8.0] {{
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(2.0, 6.0, 2.0))),
                MeshMaterial3d(materials.add(StandardMaterial {{
                    base_color: Color::srgb(0.3, 0.65, 0.9),
                    perceptual_roughness: 0.4,
                    ..default()
                }})),
                Transform::from_xyz(x, 3.0, z),
            ));
        }}
    }}

    // 5. Crosshair HUD
    commands
        .spawn(Node {{
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        }})
        .with_children(|parent| {{
            parent.spawn((
                Node {{
                    width: Val::Px(6.0),
                    height: Val::Px(6.0),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                }},
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            ));
        }});

    // 6. Load saved scene
    commands.spawn(DynamicWorldRoot(asset_server.load("scenes/default_scene.bsn")));
}}

fn fps_mouse_look(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: Query<(&mut Transform, &mut FpsController), With<Camera3d>>,
) {{
    for (mut transform, mut controller) in &mut query {{
        let delta = mouse_motion.delta;
        if delta != Vec2::ZERO {{
            controller.yaw -= delta.x * 0.003;
            controller.pitch = (controller.pitch - delta.y * 0.003).clamp(-1.45, 1.45);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
        }}
    }}
}}

fn fps_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<FpsController>>,
) {{
    for mut transform in &mut query {{
        let speed = if keyboard.pressed(KeyCode::ShiftLeft) {{ 12.0 }} else {{ 6.0 }};
        let mut forward = transform.forward().as_vec3();
        forward.y = 0.0;
        if forward.length_squared() > 0.001 {{
            forward = forward.normalize();
        }}
        let mut right = transform.right().as_vec3();
        right.y = 0.0;
        if right.length_squared() > 0.001 {{
            right = right.normalize();
        }}

        let mut move_vec = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) {{ move_vec += forward; }}
        if keyboard.pressed(KeyCode::KeyS) {{ move_vec -= forward; }}
        if keyboard.pressed(KeyCode::KeyA) {{ move_vec -= right; }}
        if keyboard.pressed(KeyCode::KeyD) {{ move_vec += right; }}

        if move_vec.length_squared() > 0.001 {{
            transform.translation += move_vec.normalize() * speed * time.delta_secs();
        }}
    }}
}}

fn cursor_grab_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window>,
) {{
    for mut window in &mut window_query {{
        if mouse_button.just_pressed(MouseButton::Left) {{
            window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        }}
        if keyboard.just_pressed(KeyCode::Escape) {{
            window.cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
            window.cursor_options.visible = true;
        }}
    }}
}}
"#
            ),

            ProjectTemplateKind::OrbitViewer => format!(
                r#"//! Standalone 3D Model & Scene Viewer created by Bevy Editor.

use bevy::input::mouse::{{AccumulatedMouseMotion, AccumulatedMouseScroll}};
use bevy::prelude::*;
use bevy::world_serialization::DynamicWorldRoot;

fn main() {{
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {{
            primary_window: Some(Window {{
                title: "{project_name} - 3D Orbit Viewer".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }}),
            ..default()
        }}))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, orbit_camera_controls)
        .run();
}}

#[derive(Component)]
struct OrbitCamera {{
    distance: f32,
    yaw: f32,
    pitch: f32,
    target: Vec3,
}}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {{
    // 1. Studio Key & Fill lights
    commands.spawn((
        DirectionalLight {{
            illuminance: 14_000.0,
            shadow_maps_enabled: true,
            ..default()
        }},
        Transform::from_xyz(6.0, 10.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {{
            illuminance: 5_000.0,
            shadow_maps_enabled: false,
            ..default()
        }},
        Transform::from_xyz(-6.0, 6.0, -6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 2. Orbit Camera
    commands.spawn((
        Camera3d::default(),
        OrbitCamera {{
            distance: 10.0,
            yaw: 0.6,
            pitch: 0.35,
            target: Vec3::new(0.0, 1.0, 0.0),
        }},
        Transform::IDENTITY,
    ));

    // 3. Studio Turntable Platform
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(6.0, 0.2))),
        MeshMaterial3d(materials.add(StandardMaterial {{
            base_color: Color::srgb(0.2, 0.22, 0.25),
            perceptual_roughness: 0.5,
            metallic: 0.2,
            ..default()
        }})),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // 4. Center showcase mesh
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(0.6, 1.4))),
        MeshMaterial3d(materials.add(StandardMaterial {{
            base_color: Color::srgb(0.25, 0.75, 0.95),
            metallic: 0.85,
            perceptual_roughness: 0.15,
            ..default()
        }})),
        Transform::from_xyz(0.0, 1.6, 0.0),
    ));

    // 5. Load default scene
    commands.spawn(DynamicWorldRoot(asset_server.load("scenes/default_scene.bsn")));
}}

fn orbit_camera_controls(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut query: Query<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
) {{
    for (mut transform, mut orbit) in &mut query {{
        if mouse_buttons.pressed(MouseButton::Left) {{
            let delta = mouse_motion.delta;
            orbit.yaw -= delta.x * 0.005;
            orbit.pitch = (orbit.pitch + delta.y * 0.005).clamp(-1.4, 1.4);
        }}

        if mouse_buttons.pressed(MouseButton::Right) || mouse_buttons.pressed(MouseButton::Middle) {{
            let delta = mouse_motion.delta;
            let right = transform.right().as_vec3();
            let up = transform.up().as_vec3();
            orbit.target += (-right * delta.x + up * delta.y) * 0.008 * (orbit.distance / 10.0);
        }}

        let scroll = mouse_scroll.delta.y;
        if scroll.abs() > 0.001 {{
            orbit.distance = (orbit.distance - scroll * 1.2).clamp(2.0, 60.0);
        }}

        let rot = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
        let cam_pos = orbit.target + rot * Vec3::new(0.0, 0.0, orbit.distance);
        transform.translation = cam_pos;
        transform.look_at(orbit.target, Vec3::Y);
    }}
}}
"#
            ),

            ProjectTemplateKind::Strategy => format!(
                r#"//! Standalone Top-Down / RTS Strategy Game created by Bevy Editor.

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::world_serialization::DynamicWorldRoot;

fn main() {{
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {{
            primary_window: Some(Window {{
                title: "{project_name} - Strategy / RTS".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }}),
            ..default()
        }}))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (rts_camera_pan, rts_click_target))
        .run();
}}

#[derive(Component)]
struct RtsCamera {{
    speed: f32,
    zoom: f32,
}}

#[derive(Component)]
struct DestinationBeacon;

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {{
    // 1. Sun light
    commands.spawn((
        DirectionalLight {{
            illuminance: 14_000.0,
            shadow_maps_enabled: true,
            ..default()
        }},
        Transform::from_xyz(8.0, 16.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 2. RTS Angled Camera
    commands.spawn((
        Camera3d::default(),
        RtsCamera {{ speed: 20.0, zoom: 18.0 }},
        Transform::from_xyz(0.0, 18.0, 14.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 3. Terrain Grid
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
        MeshMaterial3d(materials.add(StandardMaterial {{
            base_color: Color::srgb(0.2, 0.28, 0.22),
            perceptual_roughness: 0.9,
            ..default()
        }})),
        Transform::IDENTITY,
    ));

    // 4. Sample Strategic Base Building Props
    for &(x, z, h, color) in &[
        (-4.0, -4.0, 3.0, Color::srgb(0.2, 0.6, 0.9)),
        (4.0, -4.0, 2.0, Color::srgb(0.9, 0.5, 0.2)),
        (0.0, -6.0, 4.0, Color::srgb(0.3, 0.8, 0.4)),
    ] {{
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.5, h, 2.5))),
            MeshMaterial3d(materials.add(StandardMaterial {{
                base_color: color,
                perceptual_roughness: 0.4,
                ..default()
            }})),
            Transform::from_xyz(x, h / 2.0, z),
        ));
    }}

    // 5. Load default scene
    commands.spawn(DynamicWorldRoot(asset_server.load("scenes/default_scene.bsn")));
}}

fn rts_camera_pan(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    scroll: Res<AccumulatedMouseScroll>,
    mut query: Query<(&mut Transform, &mut RtsCamera)>,
) {{
    for (mut transform, mut rts) in &mut query {{
        let mut pan = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {{
            pan.z -= 1.0;
        }}
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {{
            pan.z += 1.0;
        }}
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {{
            pan.x -= 1.0;
        }}
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {{
            pan.x += 1.0;
        }}

        if pan.length_squared() > 0.001 {{
            transform.translation += pan.normalize() * rts.speed * time.delta_secs();
        }}

        let scroll_y = scroll.delta.y;
        if scroll_y.abs() > 0.001 {{
            rts.zoom = (rts.zoom - scroll_y * 2.0).clamp(6.0, 40.0);
            transform.translation.y = rts.zoom;
        }}
    }}
}}

fn rts_click_target(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut beacon_query: Query<(Entity, &mut Transform), With<DestinationBeacon>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {{
    if mouse_buttons.just_pressed(MouseButton::Right) {{
        if let Ok((_entity, mut trans)) = beacon_query.single_mut() {{
            trans.translation = Vec3::new(0.0, 0.1, 0.0);
        }} else {{
            commands.spawn((
                DestinationBeacon,
                Mesh3d(meshes.add(Cylinder::new(1.0, 0.1))),
                MeshMaterial3d(materials.add(StandardMaterial {{
                    base_color: Color::srgb(0.2, 0.9, 0.5),
                    emissive: LinearRgba::rgb(0.2, 1.0, 0.5),
                    ..default()
                }})),
                Transform::from_xyz(0.0, 0.1, 0.0),
            ));
        }}
    }}
}}
"#
            ),

            ProjectTemplateKind::Blank => format!(
                r#"//! Standalone Bevy Game created by Bevy Editor.

use bevy::prelude::*;
use bevy::world_serialization::DynamicWorldRoot;

fn main() {{
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {{
            primary_window: Some(Window {{
                title: "{project_name}".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }}),
            ..default()
        }}))
        .add_systems(Startup, setup_scene)
        .run();
}}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {{
    // Default Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // Directional Sunlight Fallback
    commands.spawn((
        DirectionalLight {{
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        }},
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Load default scene configured in Bevy Editor
    commands.spawn(DynamicWorldRoot(asset_server.load("scenes/default_scene.bsn")));
}}
"#
            ),
        }
    }
}

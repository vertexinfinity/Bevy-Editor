//! Authorable components that can be attached to entities in the editor.
//!
//! Includes extensible script public property serialization & object reference assignment.

use std::collections::HashMap;
use bevy::prelude::*;
use crate::core::play_mode::PlayState;

/// Types of public script properties that can be inspected and serialized.
#[derive(Reflect, Clone, Debug, PartialEq)]
pub enum ScriptPropertyVal {
    Float(f32),
    Int(i64),
    Bool(bool),
    String(String),
    Vec3(Vec3),
}

/// Custom script component allowing user scripts (Lua, Rhai, or custom behavior).
///
/// Features:
/// - Public property exposure and serialization to scene files
/// - Object/entity assignment slots (linking other scene entities or target scripts)
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct ScriptComponent {
    pub script_file: String,
    pub enabled: bool,
    /// Exposed public properties edited in inspector and serialized to scene
    pub properties: HashMap<String, ScriptPropertyVal>,
    /// Assigned entity references (e.g. target_actor, trigger_volume, camera_focus)
    pub entity_refs: HashMap<String, Entity>,
}

impl Default for ScriptComponent {
    fn default() -> Self {
        let mut props = HashMap::new();
        props.insert("move_speed".to_string(), ScriptPropertyVal::Float(5.0));
        props.insert("jump_force".to_string(), ScriptPropertyVal::Float(10.0));
        props.insert("is_active".to_string(), ScriptPropertyVal::Bool(true));
        props.insert("actor_name".to_string(), ScriptPropertyVal::String("PlayerHero".to_string()));

        Self {
            script_file: "scripts/player.lua".to_string(),
            enabled: true,
            properties: props,
            entity_refs: HashMap::new(),
        }
    }
}

/// Dynamic velocity for physics or character movement.
#[derive(Component, Reflect, Default, Clone, Debug)]
#[reflect(Component, Default)]
pub struct Velocity {
    pub linear: Vec3,
}

/// Continuous rotation animation around an axis.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct SpinAnimation {
    pub speed: f32,
    pub axis: Vec3,
}

impl Default for SpinAnimation {
    fn default() -> Self {
        Self {
            speed: 1.5,
            axis: Vec3::Y,
        }
    }
}

/// Gameplay tag or identifier.
#[derive(Component, Reflect, Default, Clone, Debug)]
#[reflect(Component, Default)]
pub struct GameplayTag {
    pub tag: String,
}

/// Interactive action assigned to a UI button, triggered on click during Play mode.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub enum UiButtonClickAction {
    LogMessage(String),
    ToggleLight,
    SpawnCube,
    PrintStats,
    CustomAction(String),
}

impl Default for UiButtonClickAction {
    fn default() -> Self {
        Self::LogMessage("Button Clicked!".to_string())
    }
}

impl UiButtonClickAction {
    pub fn execute(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        lights: &mut Query<&mut Visibility, With<PointLight>>,
    ) {
        match self {
            Self::LogMessage(msg) => {
                info!("[UI Action] {}", msg);
            }
            Self::ToggleLight => {
                for mut vis in lights.iter_mut() {
                    *vis = match *vis {
                        Visibility::Visible | Visibility::Inherited => Visibility::Hidden,
                        Visibility::Hidden => Visibility::Visible,
                    };
                }
                info!("[UI Action] Toggled scene lights!");
            }
            Self::SpawnCube => {
                let mesh = meshes.add(Cuboid::new(0.5, 0.5, 0.5));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.9, 0.8, 0.2),
                    ..default()
                });
                commands.spawn((
                    crate::scene::SceneEntity,
                    Name::new("Spawned Dynamic Cube"),
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 3.0, 0.0),
                    Velocity { linear: Vec3::new(0.0, -1.0, 0.0) },
                ));
                info!("[UI Action] Spawned dynamic cube!");
            }
            Self::PrintStats => {
                info!("[UI Action] Simulation running at regular tick.");
            }
            Self::CustomAction(name) => {
                info!("[UI Action] Triggered custom callback: {}", name);
            }
        }
    }
}

/// Playable character controller for in-viewport simulation.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct PlayerController {
    pub speed: f32,
    pub jump_force: f32,
}

impl Default for PlayerController {
    fn default() -> Self {
        Self {
            speed: 8.0,
            jump_force: 6.0,
        }
    }
}

/// Smooth follow camera for 3D action / character controllers.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct FollowCamera {
    pub offset: Vec3,
    pub smooth_speed: f32,
}

impl Default for FollowCamera {
    fn default() -> Self {
        Self {
            offset: Vec3::new(0.0, 4.0, 8.0),
            smooth_speed: 10.0,
        }
    }
}

/// First-person camera controller with mouse look and WASD movement.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct FpsController {
    pub speed: f32,
    pub sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for FpsController {
    fn default() -> Self {
        Self {
            speed: 9.0,
            sensitivity: 0.003,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

/// Turntable orbit controller for 3D model viewers and showcase scenes.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct OrbitController {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub sensitivity: f32,
}

impl Default for OrbitController {
    fn default() -> Self {
        Self {
            distance: 10.0,
            yaw: 0.6,
            pitch: 0.35,
            sensitivity: 0.004,
        }
    }
}

/// Strategy / RTS camera controller with WASD/edge panning and zoom.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct RtsController {
    pub speed: f32,
}

impl Default for RtsController {
    fn default() -> Self {
        Self {
            speed: 15.0,
        }
    }
}

pub struct AuthorableComponentsPlugin;

impl Plugin for AuthorableComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ScriptPropertyVal>()
            .register_type::<ScriptComponent>()
            .register_type::<Velocity>()
            .register_type::<SpinAnimation>()
            .register_type::<GameplayTag>()
            .register_type::<UiButtonClickAction>()
            .register_type::<PlayerController>()
            .register_type::<FollowCamera>()
            .register_type::<FpsController>()
            .register_type::<OrbitController>()
            .register_type::<RtsController>()
            .add_systems(
                Update,
                (
                    animate_spinning_objects,
                    simulate_velocities,
                    handle_ui_button_interactions,
                    simulate_player_movement,
                    simulate_follow_camera,
                    simulate_fps_controller,
                    simulate_orbit_camera,
                    simulate_rts_camera,
                ),
            );
    }
}

fn animate_spinning_objects(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    mut query: Query<(&SpinAnimation, &mut Transform)>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    for (spin, mut transform) in &mut query {
        if spin.axis.length_squared() > 0.001 {
            let dir = spin.axis.normalize();
            transform.rotate_axis(Dir3::new_unchecked(dir), spin.speed * time.delta_secs());
        }
    }
}

/// Simulates linear velocity when in Play mode.
fn simulate_velocities(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    mut query: Query<(&Velocity, &mut Transform)>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    for (vel, mut transform) in &mut query {
        transform.translation += vel.linear * time.delta_secs();
    }
}

/// Handles interactive UI button clicks during Play mode.
fn handle_ui_button_interactions(
    play_state: Option<Res<PlayState>>,
    interaction_query: Query<&UiButtonClickAction, Added<bevy::ui::Pressed>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<&mut Visibility, With<PointLight>>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    for action in &interaction_query {
        action.execute(&mut commands, &mut meshes, &mut materials, &mut lights);
    }
}

/// Simulates player character movement (WASD + Space jump) in the viewport during Play mode.
fn simulate_player_movement(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, Option<&PlayerController>, Option<&Name>, Option<&GameplayTag>, Option<&ScriptComponent>), Without<Camera>>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    let dt = time.delta_secs();
    for (mut transform, player_ctrl, name, tag, script) in &mut query {
        let is_player = player_ctrl.is_some()
            || name.map_or(false, |n| n.as_str().to_lowercase().contains("player") || n.as_str().to_lowercase().contains("hero"))
            || tag.map_or(false, |t| t.tag.to_lowercase().contains("player"));

        if !is_player {
            continue;
        }

        let speed = script
            .and_then(|s| s.properties.get("move_speed"))
            .and_then(|val| match val {
                ScriptPropertyVal::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or_else(|| player_ctrl.map_or(8.0, |p| p.speed));
        let mut move_dir = Vec3::ZERO;

        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            move_dir.z -= 1.0;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            move_dir.z += 1.0;
        }
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            move_dir.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            move_dir.x += 1.0;
        }

        if move_dir.length_squared() > 0.001 {
            move_dir = move_dir.normalize();
            transform.translation += move_dir * speed * dt;

            // Orient towards movement direction
            let target_rot = Quat::from_rotation_y(move_dir.x.atan2(move_dir.z));
            transform.rotation = transform.rotation.slerp(target_rot, (15.0 * dt).min(1.0));
        }

        // Jump
        if keys.just_pressed(KeyCode::Space) {
            transform.translation.y += 1.5;
        }
    }
}

/// Smoothly follows the active player entity during in-viewport Play mode.
fn simulate_follow_camera(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    player_query: Query<&Transform, (Or<(With<PlayerController>, With<GameplayTag>)>, Without<Camera>)>,
    mut camera_query: Query<(&mut Transform, Option<&FollowCamera>), (With<Camera>, Without<PlayerController>)>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    let Some(player_trans) = player_query.iter().next() else {
        return;
    };

    let dt = time.delta_secs();
    let target_pos = player_trans.translation;

    for (mut cam_trans, follow_cam) in &mut camera_query {
        let (offset, smooth_speed) = follow_cam.map_or((Vec3::new(0.0, 4.0, 8.0), 10.0), |f| (f.offset, f.smooth_speed));
        let desired_pos = target_pos + offset;
        cam_trans.translation = cam_trans.translation.lerp(desired_pos, (smooth_speed * dt).min(1.0));
        cam_trans.look_at(target_pos + Vec3::Y * 0.8, Vec3::Y);
    }
}

/// Simulates first-person camera movement (WASD) and mouse look (pitch/yaw) during in-viewport Play mode.
fn simulate_fps_controller(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mut query: Query<(&mut Transform, &mut FpsController, Option<&ScriptComponent>), (With<Camera>, Without<PlayerController>, Without<FollowCamera>)>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    let dt = time.delta_secs();

    for (mut trans, mut fps, script) in &mut query {
        let speed = script
            .and_then(|s| s.properties.get("move_speed"))
            .and_then(|val| match val {
                ScriptPropertyVal::Float(f) => Some(*f),
                _ => None,
            })
            .unwrap_or(fps.speed);

        // Mouse look: dragging either mouse button rotates camera view
        if mouse_buttons.pressed(MouseButton::Right) || mouse_buttons.pressed(MouseButton::Left) {
            fps.yaw -= mouse_motion.delta.x * fps.sensitivity;
            fps.pitch = (fps.pitch - mouse_motion.delta.y * fps.sensitivity).clamp(-1.45, 1.45);
        }

        trans.rotation = Quat::from_euler(EulerRot::YXZ, fps.yaw, fps.pitch, 0.0);

        let forward = trans.forward().as_vec3();
        let right = trans.right().as_vec3();
        let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        let mut move_dir = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            move_dir += forward_flat;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            move_dir -= forward_flat;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            move_dir += right_flat;
        }
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            move_dir -= right_flat;
        }

        if move_dir.length_squared() > 0.001 {
            trans.translation += move_dir.normalize() * speed * dt;
        }
    }
}

/// Simulates turntable orbit camera around scene center during in-viewport Play mode.
fn simulate_orbit_camera(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mut camera_query: Query<(&mut Transform, &mut OrbitController), With<Camera>>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    for (mut trans, mut orbit) in &mut camera_query {
        if mouse_buttons.pressed(MouseButton::Left) || mouse_buttons.pressed(MouseButton::Right) {
            orbit.yaw -= mouse_motion.delta.x * orbit.sensitivity;
            orbit.pitch = (orbit.pitch - mouse_motion.delta.y * orbit.sensitivity).clamp(-1.4, 1.4);
        } else {
            // Gentle continuous turntable spin when not dragging
            orbit.yaw += 0.3 * time.delta_secs();
        }

        let rot = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
        trans.translation = rot * Vec3::new(0.0, 0.0, orbit.distance);
        trans.look_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
    }
}

/// Simulates RTS / Strategy camera panning (WASD / arrows) and zoom during in-viewport Play mode.
fn simulate_rts_camera(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    mut camera_query: Query<(&mut Transform, Option<&RtsController>), With<Camera>>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    let dt = time.delta_secs();
    for (mut cam_trans, rts_ctrl) in &mut camera_query {
        if rts_ctrl.is_none() {
            continue;
        }
        let speed = rts_ctrl.map_or(15.0, |r| r.speed);
        let mut pan = Vec3::ZERO;

        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            pan.z -= 1.0;
            pan.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            pan.z += 1.0;
            pan.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            pan.x -= 1.0;
            pan.z += 1.0;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            pan.x += 1.0;
            pan.z -= 1.0;
        }

        if pan.length_squared() > 0.001 {
            cam_trans.translation += pan.normalize() * speed * dt;
        }

        if mouse_scroll.delta.y.abs() > 0.01 {
            cam_trans.translation.y = (cam_trans.translation.y - mouse_scroll.delta.y * 2.0).clamp(5.0, 40.0);
        }
    }
}

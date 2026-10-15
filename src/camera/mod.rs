//! Viewport and industry-standard fly-through camera setup.
//!
//! Behavior matches standard 3D software (Unreal, Unity, Blender):
//! - Hold RMB (Right Mouse Button) to look around with mouse and enable flight mode.
//! - While holding RMB:
//!   - W/S: Move forward / backward
//!   - A/D: Strafe left / right
//!   - E/Q (or Space/C): Fly up / down
//!   - Shift: Boost movement speed
//!   - Mouse Scroll: Adjust flight speed up or down
//! - When RMB is released:
//!   - Cursor is unlocked and visible
//!   - Keys do not move the camera (safe for UI and shortcuts)

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, Window},
};
use crate::core::play_mode::PlayState;
use crate::scene::EditorInternal;

/// Marker for the editor's primary viewport camera.
#[derive(Component)]
pub struct EditorCamera;

/// Component holding the state and configuration of the editor fly camera.
#[derive(Component)]
pub struct FlyCamera {
    pub base_speed: f32,
    pub boost_multiplier: f32,
    pub sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub is_active: bool,
}

impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            base_speed: 12.0,
            boost_multiplier: 2.5,
            sensitivity: 0.003,
            yaw: 0.0,
            pitch: 0.0,
            is_active: false,
        }
    }
}

pub struct EditorCameraPlugin;

impl Plugin for EditorCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_editor_camera)
            .add_systems(Update, screenshot_on_f12)
            .add_observer(|c: On<bevy::picking::events::PointerClick>| {
                info!("[debug] PointerClick on entity {:?} button {:?}", c.entity, c.button);
            })
            .add_systems(
                Update,
                (update_fly_camera, handle_camera_focus_shortcut)
                    .run_if(in_state(crate::ui::EditorAppState::InEditor)),
            );
    }
}

/// Press F12 to save `screenshots/shot_<n>.png` (used for remote UI debugging).
fn screenshot_on_f12(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut counter: Local<u32>,
) {
    if keys.just_pressed(KeyCode::F12) {
        let _ = std::fs::create_dir_all("screenshots");
        let path = format!("screenshots/shot_{}.png", *counter);
        *counter += 1;
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
    }
}

/// Focus viewport camera on selected entity when `F` is pressed.
fn handle_camera_focus_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    play_state: Option<Res<PlayState>>,
    selection: Res<bevy::inspector::InspectorSelection>,
    transforms: Query<&GlobalTransform>,
    mut camera_query: Query<(&mut Transform, &mut FlyCamera), With<EditorCamera>>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Edit {
            return;
        }
    }

    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }

    let Some(selected_entity) = selection.0 else {
        return;
    };

    let Ok(target_global) = transforms.get(selected_entity) else {
        return;
    };

    let Ok((mut cam_transform, mut fly_cam)) = camera_query.single_mut() else {
        return;
    };

    let target_pos = target_global.translation();
    let offset = Vec3::new(0.0, 3.5, 7.0);
    let new_cam_pos = target_pos + offset;

    cam_transform.translation = new_cam_pos;
    cam_transform.look_at(target_pos, Vec3::Y);

    let dir = (target_pos - new_cam_pos).normalize();
    fly_cam.pitch = dir.y.asin().clamp(-1.4, 1.4);
    fly_cam.yaw = (-dir.x).atan2(-dir.z);

    info!("Focused camera on selected entity: {:?}", selected_entity);
}

fn setup_editor_camera(mut commands: Commands) {
    let initial_pos = Vec3::new(0.0, 5.0, 10.0);
    let target = Vec3::new(0.0, 1.0, 0.0);
    let dir = (target - initial_pos).normalize();
    let pitch = dir.y.asin();
    let yaw = (-dir.x).atan2(-dir.z);

    commands.spawn((
        EditorCamera,
        FlyCamera {
            yaw,
            pitch,
            ..default()
        },
        EditorInternal,
        Name::new("Editor Viewport Camera"),
        Camera3d::default(),
        bevy::picking::mesh_picking::MeshPickingCamera,
        bevy::gizmos::transform_gizmo::TransformGizmoCamera,
        Transform::from_translation(initial_pos).looking_at(target, Vec3::Y),
    ));
}

fn update_fly_camera(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut windows: Query<(&Window, &mut CursorOptions)>,
    mut camera_query: Query<(&mut Transform, &mut FlyCamera), With<EditorCamera>>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Edit {
            return;
        }
    }

    let Ok((mut transform, mut fly_cam)) = camera_query.single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    // 1. Right Mouse Button toggles flight mode
    if mouse_button.just_pressed(MouseButton::Right) {
        fly_cam.is_active = true;
        for (window, mut cursor_options) in &mut windows {
            if window.focused {
                cursor_options.grab_mode = CursorGrabMode::Locked;
                cursor_options.visible = false;
            }
        }
    }

    if mouse_button.just_released(MouseButton::Right) {
        fly_cam.is_active = false;
        for (_, mut cursor_options) in &mut windows {
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
    }

    // Scroll wheel adjusts flight base speed smoothly
    if mouse_scroll.delta.y.abs() > 0.01 {
        let factor = 1.15_f32.powf(mouse_scroll.delta.y);
        fly_cam.base_speed = (fly_cam.base_speed * factor).clamp(1.0, 200.0);
    }

    // Only process look and movement when RMB is held
    if !fly_cam.is_active {
        return;
    }

    // 2. Mouse look (Yaw & Pitch)
    if mouse_motion.delta != Vec2::ZERO {
        fly_cam.yaw -= mouse_motion.delta.x * fly_cam.sensitivity;
        fly_cam.pitch -= mouse_motion.delta.y * fly_cam.sensitivity;

        // Clamp pitch to avoid gimbal flip (-89 deg to +89 deg)
        let max_pitch = 89.0_f32.to_radians();
        fly_cam.pitch = fly_cam.pitch.clamp(-max_pitch, max_pitch);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, fly_cam.yaw, fly_cam.pitch, 0.0);
    }

    // 3. Movement (WASD + Q/E / Space/C)
    let mut move_dir = Vec3::ZERO;
    let forward = *transform.forward();
    let right = *transform.right();
    let up = Vec3::Y;

    if keys.pressed(KeyCode::KeyW) {
        move_dir += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        move_dir -= forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        move_dir += right;
    }
    if keys.pressed(KeyCode::KeyA) {
        move_dir -= right;
    }
    if keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::Space) {
        move_dir += up;
    }
    if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::KeyC) {
        move_dir -= up;
    }

    if move_dir != Vec3::ZERO {
        let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            fly_cam.base_speed * fly_cam.boost_multiplier
        } else {
            fly_cam.base_speed
        };
        transform.translation += move_dir.normalize() * speed * dt;
    }
}

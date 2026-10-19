//! Third-person orbit camera: right mouse drags to orbit, the wheel zooms, and the focus point
//! smoothly follows the player.
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    prelude::*,
};
use sample_kit::GameState;

use crate::{Autopilot, level::COURTYARD_HALF, player::Player};

const MIN_PITCH: f32 = -1.35;
const MAX_PITCH: f32 = -0.08;
const MIN_DISTANCE: f32 = 3.5;
const MAX_DISTANCE: f32 = 16.0;
const MENU_PITCH: f32 = -0.62;
const MENU_DISTANCE: f32 = 34.0;
/// Height above the player's center the camera looks at.
const FOCUS_HEIGHT: f32 = 0.9;

/// Orbit parameters around the followed player.
#[derive(Component)]
pub struct OrbitCamera {
    /// Rotation around the vertical axis; 0 puts the camera on the player's +Z side.
    pub yaw: f32,
    /// Negative values look down at the player.
    pub pitch: f32,
    pub distance: f32,
    /// Smoothed point the camera looks at.
    focus: Vec3,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: -0.42, distance: 9.0, focus: Vec3::ZERO }
    }
}

impl OrbitCamera {
    /// Puts the camera behind a player standing at `at`, facing -Z.
    pub fn reset(&mut self, at: Vec3) {
        *self = Self { focus: at + Vec3::Y * FOCUS_HEIGHT, ..default() };
    }
}

/// Applies orbit/zoom input, follows the player and places the camera.
pub fn orbit_camera(
    time: Res<Time>,
    state: Res<State<GameState>>,
    autopilot: Res<Autopilot>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    players: Query<&Transform, (With<Player>, Without<OrbitCamera>)>,
    mut cameras: Query<(&mut Transform, &mut OrbitCamera)>,
) {
    let Ok((mut transform, mut orbit)) = cameras.single_mut() else { return };
    let Ok(player) = players.single() else { return };
    let dt = time.delta_secs();
    let playing = *state.get() == GameState::Playing;

    if playing && dt > 0.0 {
        if mouse.pressed(MouseButton::Right) {
            orbit.yaw -= motion.delta.x * 0.005;
            orbit.pitch = (orbit.pitch - motion.delta.y * 0.004).clamp(MIN_PITCH, MAX_PITCH);
        }
        let lines = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
        };
        orbit.distance = (orbit.distance * (1.0 - lines * 0.1)).clamp(MIN_DISTANCE, MAX_DISTANCE);
        if autopilot.0 {
            // Test runs: drift slowly so the shot shows the courtyard from varying angles.
            orbit.yaw += dt * 0.15;
        }
    }

    if *state.get() == GameState::Menu {
        // Title screen: a slow, high overview orbit of the whole courtyard.
        let rotation = Quat::from_euler(EulerRot::YXZ, time.elapsed_secs() * 0.1 + 0.6, MENU_PITCH, 0.0);
        let center = Vec3::new(0.0, 1.0, -1.0);
        *transform = Transform::from_translation(center + rotation * Vec3::Z * MENU_DISTANCE).looking_at(center, Vec3::Y);
        return;
    }

    let target = player.translation + Vec3::Y * FOCUS_HEIGHT;
    if orbit.focus == Vec3::ZERO {
        orbit.focus = target;
    }
    let blend = 1.0 - (-8.0 * dt).exp();
    orbit.focus = orbit.focus.lerp(target, blend);

    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    let mut position = orbit.focus + rotation * Vec3::Z * orbit.distance;
    // Keep the camera inside the walls so they never block the view from behind.
    let limit = COURTYARD_HALF - 0.3;
    position.x = position.x.clamp(-limit, limit);
    position.z = position.z.clamp(-limit, limit);
    position.y = position.y.max(0.4);
    *transform = Transform::from_translation(position).looking_at(orbit.focus, Vec3::Y);
}

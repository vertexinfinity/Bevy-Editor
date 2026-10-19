//! Player movement: camera-relative walking, sprinting, jumping and simple box collision.
use bevy::prelude::*;

use crate::{
    Autopilot, Coin, Colliders,
    camera::OrbitCamera,
    level::{COURTYARD_HALF, PLAYER_HALF_HEIGHT, PLAYER_RADIUS},
};

const WALK_SPEED: f32 = 6.0;
const SPRINT_SPEED: f32 = 10.0;
const JUMP_SPEED: f32 = 9.0;
const GRAVITY: f32 = 24.0;
/// How far below a box top the feet may be and still count as standing on it.
const STEP_TOLERANCE: f32 = 0.1;

/// Gameplay state of the player body.
#[derive(Component, Default)]
pub struct Player {
    pub velocity: Vec3,
    pub grounded: bool,
    /// Seconds the autopilot has been pushing without moving (it jumps when stuck).
    stuck: f32,
}

/// Reads input (or the autopilot), integrates velocity and resolves collisions.
pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    autopilot: Res<Autopilot>,
    colliders: Res<Colliders>,
    cameras: Query<&OrbitCamera>,
    coins: Query<&Transform, (With<Coin>, Without<Player>)>,
    mut players: Query<(&mut Transform, &mut Player)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let Ok((mut transform, mut player)) = players.single_mut() else { return };
    let yaw = cameras.single().map(|c| c.yaw).unwrap_or(0.0);
    let start = transform.translation;

    // Desired horizontal direction and jump request.
    let (direction, sprint, jump) = if autopilot.0 {
        autopilot_input(start, &player, coins.iter().map(|t| t.translation))
    } else {
        let mut input = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            input.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            input.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            input.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            input.x -= 1.0;
        }
        let basis = Quat::from_rotation_y(yaw);
        let world = basis * Vec3::new(input.x, 0.0, -input.y);
        (world.normalize_or_zero(), keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight), keys.just_pressed(KeyCode::Space))
    };

    // Horizontal velocity eases toward the target so starts and stops feel soft.
    let speed = if sprint { SPRINT_SPEED } else { WALK_SPEED };
    let target = direction * speed;
    let blend = 1.0 - (-14.0 * dt).exp();
    player.velocity.x += (target.x - player.velocity.x) * blend;
    player.velocity.z += (target.z - player.velocity.z) * blend;
    if jump && player.grounded {
        player.velocity.y = JUMP_SPEED;
        player.grounded = false;
    }
    player.velocity.y -= GRAVITY * dt;

    let mut pos = start + player.velocity * dt;

    // Vertical: land on the ground or on any box whose top we were above last frame.
    let prev_feet = start.y - PLAYER_HALF_HEIGHT;
    let mut support = 0.0_f32;
    for b in &colliders.0 {
        let top = b.center.y + b.half_size.y;
        let over = (pos.x - b.center.x).abs() < b.half_size.x + PLAYER_RADIUS * 0.5 && (pos.z - b.center.z).abs() < b.half_size.z + PLAYER_RADIUS * 0.5;
        if over && prev_feet >= top - STEP_TOLERANCE {
            support = support.max(top);
        }
    }
    if pos.y - PLAYER_HALF_HEIGHT <= support && player.velocity.y <= 0.0 {
        pos.y = support + PLAYER_HALF_HEIGHT;
        player.velocity.y = 0.0;
        player.grounded = true;
    } else {
        player.grounded = false;
    }

    // Horizontal: push the capsule's circle out of boxes it is beside (not on top of).
    let feet = pos.y - PLAYER_HALF_HEIGHT;
    for b in &colliders.0 {
        let top = b.center.y + b.half_size.y;
        let bottom = b.center.y - b.half_size.y;
        if feet >= top - STEP_TOLERANCE || pos.y + PLAYER_HALF_HEIGHT <= bottom {
            continue;
        }
        let local = Vec2::new(pos.x - b.center.x, pos.z - b.center.z);
        let half = Vec2::new(b.half_size.x, b.half_size.z);
        let closest = local.clamp(-half, half);
        let offset = local - closest;
        let distance = offset.length();
        if distance > 0.0 && distance < PLAYER_RADIUS {
            let push = offset / distance * (PLAYER_RADIUS - distance);
            pos.x += push.x;
            pos.z += push.y;
        } else if distance == 0.0 {
            // Center inside the box: exit along the shallowest axis.
            let depth = half - local.abs();
            if depth.x < depth.y {
                pos.x = b.center.x + local.x.signum() * (half.x + PLAYER_RADIUS);
            } else {
                pos.z = b.center.z + local.y.signum() * (half.y + PLAYER_RADIUS);
            }
        }
    }

    // Courtyard walls.
    let limit = COURTYARD_HALF - PLAYER_RADIUS;
    pos.x = pos.x.clamp(-limit, limit);
    pos.z = pos.z.clamp(-limit, limit);

    // Track lack of progress so the autopilot can hop over obstacles.
    let moved = Vec2::new(pos.x - start.x, pos.z - start.z).length() / dt;
    player.stuck = if direction != Vec3::ZERO && moved < speed * 0.3 { player.stuck + dt } else { 0.0 };

    transform.translation = pos;
    if direction != Vec3::ZERO {
        let facing = Quat::from_rotation_y(f32::atan2(-direction.x, -direction.z));
        transform.rotation = transform.rotation.slerp(facing, 1.0 - (-12.0 * dt).exp());
    }
}

/// Test-only steering: run to the nearest coin reachable with one jump from the current height,
/// jumping when it is up high or the way is blocked. It does not plan routes up the stairs.
fn autopilot_input(position: Vec3, player: &Player, coins: impl Iterator<Item = Vec3>) -> (Vec3, bool, bool) {
    let nearest = |a: &Vec3, b: &Vec3| a.distance_squared(position).total_cmp(&b.distance_squared(position));
    let coins: Vec<Vec3> = coins.collect();
    let reachable = coins.iter().filter(|c| c.y - position.y < 1.4).min_by(|a, b| nearest(a, b));
    let Some(target) = reachable.or_else(|| coins.iter().min_by(|a, b| nearest(a, b))).copied() else {
        return (Vec3::ZERO, false, false);
    };
    let flat = Vec3::new(target.x - position.x, 0.0, target.z - position.z);
    let elevated = target.y - position.y > 0.8 && flat.length() < 3.5;
    (flat.normalize_or_zero(), flat.length() > 4.0, player.grounded && (elevated || player.stuck > 0.15))
}

//! Neon Runner: a three-lane endless runner built on the shared Bevylite sample kit.
//!
//! The runner stays near the origin while the track, arches, obstacles and coins scroll
//! toward the camera. Speed ramps up over time; hitting an obstacle ends the run.
mod level;

use bevy::prelude::*;
use level::{LANES, TILE_COUNT, TILE_LENGTH};
use sample_kit::{BestScore, GameState, Hud, RoundResult, RoundScoped, SampleInfo, SampleKitPlugin, end_round};

const START_SPEED: f32 = 14.0;
const MAX_SPEED: f32 = 38.0;
const ACCELERATION: f32 = 0.45;
const GRAVITY: f32 = 28.0;
const JUMP_VELOCITY: f32 = 10.5;
const SPAWN_Z: f32 = -130.0;
const DESPAWN_Z: f32 = 12.0;
const PLAYER_GROUND_Y: f32 = 0.8;

fn main() {
    sample_kit::sample_app("Neon Runner — Bevylite Sample")
        .add_plugins(SampleKitPlugin(SampleInfo {
            title: "NEON RUNNER",
            tagline: "Dodge the barriers, grab the coins, survive the speed.",
            controls: &["A / D or Left / Right   Switch lane", "Space / W / Up   Jump", "Esc   Pause"],
            accent: Color::srgb(1.0, 0.35, 0.8),
            grab_cursor: false,
        }))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.01, 0.05)))
        .insert_resource(GlobalAmbientLight { color: Color::srgb(0.6, 0.6, 1.0), brightness: 120.0, ..default() })
        .init_resource::<Run>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::Playing), start_run)
        .add_systems(
            Update,
            (autopilot, steer, scroll_world, spawn_hazards, collect_and_collide, spin_coins, update_hud)
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, idle_scroll.run_if(not(in_state(GameState::Playing))))
        .run();
}

/// State of the current run.
#[derive(Resource, Default)]
struct Run {
    speed: f32,
    distance: f32,
    coins: u32,
    lane: usize,
    vertical_velocity: f32,
    next_spawn: f32,
    pattern: u32,
}

#[derive(Component)]
struct Runner;

/// Anything that scrolls toward the camera and wraps around (track tiles and arches).
#[derive(Component)]
struct Looping;

#[derive(Component)]
enum Hazard {
    /// Low barrier: jump over it.
    Barrier,
    /// Full-height wall: change lane.
    Wall,
}

#[derive(Component)]
struct Coin;

/// Meshes and materials reused by everything spawned during a run.
#[derive(Resource)]
struct RunAssets {
    barrier: (Handle<Mesh>, Handle<StandardMaterial>),
    wall: (Handle<Mesh>, Handle<StandardMaterial>),
    coin: (Handle<Mesh>, Handle<StandardMaterial>),
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let level = level::spawn_level(&mut commands, &mut meshes, &mut materials);
    info!("Neon Runner level: {} entities, chase camera {:?}", level.all.len(), level.camera);
    commands.entity(level.player).insert(Runner);
    for entity in level.track_tiles.iter().chain(&level.decor) {
        commands.entity(*entity).insert(Looping);
    }
    commands.insert_resource(RunAssets {
        barrier: (
            meshes.add(Cuboid::new(2.0, 0.8, 0.5)),
            materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.75, 0.1), emissive: LinearRgba::rgb(2.0, 1.2, 0.1), ..default() }),
        ),
        wall: (
            meshes.add(Cuboid::new(2.1, 3.0, 0.8)),
            materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.15, 0.2), emissive: LinearRgba::rgb(1.6, 0.1, 0.2), ..default() }),
        ),
        coin: (
            meshes.add(Cylinder::new(0.38, 0.08)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.85, 0.2),
                emissive: LinearRgba::rgb(2.5, 1.8, 0.2),
                metallic: 0.9,
                perceptual_roughness: 0.25,
                ..default()
            }),
        ),
    });
}

fn start_run(mut run: ResMut<Run>, mut runner: Query<&mut Transform, With<Runner>>, mut hud: ResMut<Hud>) {
    *run = Run { speed: START_SPEED, lane: 1, next_spawn: 25.0, ..default() };
    hud.banner = None;
    if let Ok(mut transform) = runner.single_mut() {
        transform.translation = Vec3::new(LANES[1], PLAYER_GROUND_Y, 0.0);
        transform.rotation = Quat::IDENTITY;
    }
}

fn steer(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut run: ResMut<Run>, mut runner: Query<&mut Transform, With<Runner>>) {
    let Ok(mut transform) = runner.single_mut() else { return };
    if keys.any_just_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) && run.lane > 0 {
        run.lane -= 1;
    }
    if keys.any_just_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) && run.lane < LANES.len() - 1 {
        run.lane += 1;
    }
    let grounded = transform.translation.y <= PLAYER_GROUND_Y + 0.001;
    if grounded && keys.any_just_pressed([KeyCode::Space, KeyCode::KeyW, KeyCode::ArrowUp]) {
        run.vertical_velocity = JUMP_VELOCITY;
    }
    let dt = time.delta_secs();
    let target_x = LANES[run.lane];
    transform.translation.x += (target_x - transform.translation.x) * (dt * 14.0).min(1.0);
    run.vertical_velocity -= GRAVITY * dt;
    transform.translation.y = (transform.translation.y + run.vertical_velocity * dt).max(PLAYER_GROUND_Y);
    if transform.translation.y <= PLAYER_GROUND_Y {
        run.vertical_velocity = 0.0;
    }
    // Lean into lane changes for a bit of life.
    let lean = (target_x - transform.translation.x) * -0.25;
    transform.rotation = Quat::from_rotation_z(lean);
}

fn scroll(dz: f32, looping: &mut Query<&mut Transform, (With<Looping>, Without<Runner>)>) {
    let span = TILE_COUNT as f32 * TILE_LENGTH;
    for mut transform in looping.iter_mut() {
        transform.translation.z += dz;
        if transform.translation.z > DESPAWN_Z + TILE_LENGTH {
            transform.translation.z -= span;
        }
    }
}

/// Keeps the track gently moving behind the menu and game-over screens.
fn idle_scroll(time: Res<Time<Real>>, mut looping: Query<&mut Transform, (With<Looping>, Without<Runner>)>) {
    scroll(time.delta_secs() * 6.0, &mut looping);
}

#[allow(clippy::type_complexity)]
fn scroll_world(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut looping: Query<&mut Transform, (With<Looping>, Without<Runner>)>,
    mut movers: Query<(Entity, &mut Transform), (Or<(With<Hazard>, With<Coin>)>, Without<Looping>, Without<Runner>)>,
) {
    let dt = time.delta_secs();
    run.speed = (run.speed + ACCELERATION * dt).min(MAX_SPEED);
    let dz = run.speed * dt;
    run.distance += dz;
    scroll(dz, &mut looping);
    for (entity, mut transform) in &mut movers {
        transform.translation.z += dz;
        if transform.translation.z > DESPAWN_Z {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawns a row of hazards and coins every so often, always leaving one lane open.
fn spawn_hazards(mut commands: Commands, mut run: ResMut<Run>, assets: Res<RunAssets>) {
    if run.distance < run.next_spawn {
        return;
    }
    run.pattern = run.pattern.wrapping_mul(1_103_515_245).wrapping_add(12_345 + run.distance as u32);
    let roll = run.pattern >> 8;
    let open_lane = (roll % 3) as usize;
    let gap = 16.0 + 10.0 * (1.0 - (run.speed - START_SPEED) / (MAX_SPEED - START_SPEED));
    run.next_spawn = run.distance + gap;
    for (lane, x) in LANES.iter().enumerate() {
        if lane == open_lane {
            // Coins trail through the open lane.
            for i in 0..4 {
                commands.spawn((
                    Name::new("Coin"),
                    Coin,
                    RoundScoped,
                    Mesh3d(assets.coin.0.clone()),
                    MeshMaterial3d(assets.coin.1.clone()),
                    Transform::from_xyz(*x, 1.0, SPAWN_Z - i as f32 * 2.2).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ));
            }
            continue;
        }
        let wall = (roll >> (lane * 3)) % 3 == 0;
        let (mesh, material, y, hazard) = if wall {
            (&assets.wall.0, &assets.wall.1, 1.5, Hazard::Wall)
        } else {
            (&assets.barrier.0, &assets.barrier.1, 0.4, Hazard::Barrier)
        };
        commands.spawn((
            Name::new(if wall { "Wall" } else { "Barrier" }),
            hazard,
            RoundScoped,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(*x, y, SPAWN_Z),
        ));
    }
}

fn collect_and_collide(
    mut commands: Commands,
    mut run: ResMut<Run>,
    mut best: ResMut<BestScore>,
    runner: Query<&Transform, With<Runner>>,
    hazards: Query<(&Transform, &Hazard), Without<Runner>>,
    coins: Query<(Entity, &Transform), (With<Coin>, Without<Runner>)>,
) {
    let Ok(player) = runner.single() else { return };
    let p = player.translation;
    for (entity, coin) in &coins {
        if coin.translation.distance(p) < 1.1 {
            run.coins += 1;
            commands.entity(entity).despawn();
        }
    }
    for (transform, hazard) in &hazards {
        let t = transform.translation;
        let overlap_x = (t.x - p.x).abs() < 1.2;
        let overlap_z = (t.z - p.z).abs() < 0.55;
        let feet = p.y - 0.8;
        let clears = matches!(hazard, Hazard::Barrier) && feet > 0.75;
        if overlap_x && overlap_z && !clears {
            let score = score(&run);
            let record = best.0.is_none_or(|b| score > b);
            if record {
                best.0 = Some(score);
            }
            let what = match hazard {
                Hazard::Barrier => "Tripped on a barrier",
                Hazard::Wall => "Crashed into a wall",
            };
            end_round(
                RoundResult {
                    won: false,
                    headline: what.to_string(),
                    details: vec![
                        format!("Distance {} m    Coins {}", run.distance as i64, run.coins),
                        format!("Score {score}{}", if record { "   (new best!)" } else { "" }),
                        format!("Top speed {:.0} m/s", run.speed),
                    ],
                },
                &mut commands,
            );
            return;
        }
    }
}

fn score(run: &Run) -> i64 {
    run.distance as i64 + run.coins as i64 * 25
}

fn spin_coins(time: Res<Time>, mut coins: Query<&mut Transform, With<Coin>>) {
    for mut transform in &mut coins {
        transform.rotate_y(time.delta_secs() * 4.0);
    }
}

fn update_hud(run: Res<Run>, best: Res<BestScore>, mut hud: ResMut<Hud>) {
    hud.lines = vec![
        format!("Distance  {} m", run.distance as i64),
        format!("Coins     {}", run.coins),
        format!("Speed     {:.0} m/s", run.speed),
        format!("Score     {}{}", score(&run), best.0.map(|b| format!("   (best {b})")).unwrap_or_default()),
    ];
    hud.banner = (run.distance < 18.0).then(|| "GO!".to_string());
}

/// Test hook: with `SAMPLE_AUTOPLAY=1`, dodge whatever is ahead in the current lane.
fn autopilot(
    mut run: ResMut<Run>,
    hazards: Query<(&Transform, &Hazard)>,
    runner: Query<&Transform, With<Runner>>,
    mut enabled: Local<Option<bool>>,
) {
    if !*enabled.get_or_insert_with(|| std::env::var("SAMPLE_AUTOPLAY").is_ok_and(|v| v == "1")) {
        return;
    }
    let Ok(player) = runner.single() else { return };
    let lookahead = 6.0 + run.speed * 0.35;
    let blocked = |lane: usize, kind: Option<bool>| {
        hazards.iter().any(|(t, h)| {
            (t.translation.x - LANES[lane]).abs() < 0.5
                && t.translation.z < player.translation.z
                && t.translation.z > player.translation.z - lookahead
                && kind.is_none_or(|wall| matches!(h, Hazard::Wall) == wall)
        })
    };
    if blocked(run.lane, Some(true)) {
        if let Some(free) = (0..LANES.len()).filter(|l| !blocked(*l, None)).min_by_key(|l| l.abs_diff(run.lane)) {
            run.lane = free;
        }
    } else if blocked(run.lane, Some(false)) && player.translation.y <= PLAYER_GROUND_Y + 0.001 {
        let near = hazards.iter().any(|(t, h)| {
            matches!(h, Hazard::Barrier)
                && (t.translation.x - LANES[run.lane]).abs() < 0.5
                && t.translation.z > player.translation.z - run.speed * 0.32
                && t.translation.z < player.translation.z
        });
        if near {
            run.vertical_velocity = JUMP_VELOCITY;
        }
    }
}

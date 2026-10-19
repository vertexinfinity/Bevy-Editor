//! Coin Courtyard: a third-person collect-a-thon built on the shared sample kit.
//!
//! The static courtyard lives in `level.rs` (shared with the Bevylite editor); this file wires
//! up the round flow: coins, timer, HUD and the win/lose result.
mod camera;
mod level;
mod player;

use bevy::prelude::*;
use sample_kit::{BestScore, GameState, Hud, RoundResult, RoundScoped, SampleInfo, SampleKitPlugin, end_round, sample_app};

use crate::{
    camera::OrbitCamera,
    level::{LevelEntities, PLAYER_START, Platform, spawn_level},
    player::Player,
};

const TITLE: &str = "Coin Courtyard";
/// Seconds available to collect every coin.
const ROUND_SECONDS: f32 = 90.0;
/// How close the player's center must get to a coin to pick it up.
const PICKUP_RADIUS: f32 = 1.1;
const GOLD: Color = Color::srgb(1.0, 0.78, 0.22);

fn main() {
    sample_app(TITLE)
        .add_plugins(SampleKitPlugin(SampleInfo {
            title: TITLE,
            tagline: "Grab all 12 gold coins before the clock runs out",
            controls: &["WASD  Move (camera relative)", "Shift  Sprint", "Space  Jump", "Right mouse  Orbit camera", "Mouse wheel  Zoom", "Esc  Pause"],
            accent: GOLD,
            grab_cursor: false,
        }))
        .insert_resource(ClearColor(Color::srgb(0.55, 0.72, 0.88)))
        .insert_resource(GlobalAmbientLight { color: Color::srgb(0.85, 0.88, 1.0), brightness: 450.0, ..default() })
        .insert_resource(Autopilot(std::env::var("SAMPLE_AUTOPLAY").is_ok_and(|v| v == "1")))
        .init_resource::<Round>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::Playing), start_round)
        .add_systems(Update, animate_coins)
        .add_systems(Update, camera::orbit_camera.after(player::move_player))
        .add_systems(Update, (player::move_player, collect_coins, tick_round, update_hud).chain().run_if(in_state(GameState::Playing)))
        .run();
}

/// When set, the player steers itself toward coins (`SAMPLE_AUTOPLAY=1` test runs).
#[derive(Resource)]
pub struct Autopilot(pub bool);

/// Solid boxes from the level, used for player collision.
#[derive(Resource)]
pub struct Colliders(pub Vec<Platform>);

/// Shared coin assets and the spots coins are placed at each round.
#[derive(Resource)]
struct CoinAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    spots: Vec<Vec3>,
}

/// A collectible coin that spins and bobs around its rest position.
#[derive(Component)]
pub struct Coin {
    rest: Vec3,
    phase: f32,
}

/// Per-round progress.
#[derive(Resource, Default)]
struct Round {
    time_left: f32,
    elapsed: f32,
    collected: usize,
    total: usize,
    /// Counts down after the last coin so the "All coins!" banner is visible before the result.
    finish_delay: Option<f32>,
}

/// Spawns the level and attaches gameplay components to its player and camera.
fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let LevelEntities { all, player, camera, coin_spots, platforms } = spawn_level(&mut commands, &mut meshes, &mut materials);
    info!("Coin Courtyard: {} level entities, {} colliders, {} coin spots", all.len(), platforms.len(), coin_spots.len());

    commands.entity(player).insert(Player::default());
    commands.entity(camera).insert(OrbitCamera::default());
    commands.insert_resource(Colliders(platforms));
    commands.insert_resource(CoinAssets {
        mesh: meshes.add(Cylinder::new(0.38, 0.08)),
        material: materials.add(StandardMaterial {
            base_color: GOLD,
            emissive: LinearRgba::rgb(0.9, 0.55, 0.05),
            metallic: 0.9,
            perceptual_roughness: 0.25,
            ..default()
        }),
        spots: coin_spots,
    });
}

/// Resets the player, camera and timer and spawns a fresh set of coins.
fn start_round(
    mut commands: Commands,
    assets: Res<CoinAssets>,
    mut round: ResMut<Round>,
    mut players: Query<(&mut Transform, &mut Player)>,
    mut cameras: Query<&mut OrbitCamera>,
) {
    *round = Round { time_left: ROUND_SECONDS, total: assets.spots.len(), ..default() };
    for (mut transform, mut player) in &mut players {
        *transform = Transform::from_translation(PLAYER_START);
        *player = Player::default();
    }
    for mut orbit in &mut cameras {
        orbit.reset(PLAYER_START);
    }
    for (i, spot) in assets.spots.iter().enumerate() {
        commands.spawn((
            Name::new(format!("Coin {}", i + 1)),
            Coin { rest: *spot, phase: i as f32 * 0.7 },
            RoundScoped,
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            Transform::from_translation(*spot),
        ));
    }
}

/// Spins coins around the vertical axis and bobs them up and down.
fn animate_coins(time: Res<Time>, mut coins: Query<(&Coin, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (coin, mut transform) in &mut coins {
        transform.translation = coin.rest + Vec3::Y * ((t * 2.2 + coin.phase).sin() * 0.15);
        transform.rotation = Quat::from_rotation_y(t * 2.5 + coin.phase) * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    }
}

/// Picks up coins the player touches.
fn collect_coins(mut commands: Commands, mut round: ResMut<Round>, players: Query<&Transform, With<Player>>, coins: Query<(Entity, &Transform), With<Coin>>) {
    let Ok(player) = players.single() else { return };
    for (entity, coin) in &coins {
        if coin.translation.distance(player.translation) < PICKUP_RADIUS {
            commands.entity(entity).despawn();
            round.collected += 1;
        }
    }
    if round.collected >= round.total && round.finish_delay.is_none() {
        round.finish_delay = Some(1.2);
    }
}

/// Runs the countdown and ends the round on a win or when time runs out.
fn tick_round(mut commands: Commands, time: Res<Time>, mut round: ResMut<Round>, mut best: ResMut<BestScore>) {
    let dt = time.delta_secs();
    if let Some(delay) = round.finish_delay.as_mut() {
        // The clock stops once every coin is collected.
        *delay -= dt;
        if *delay <= 0.0 {
            round.finish_delay = Some(f32::INFINITY);
            end_round(win_result(round.elapsed, &mut best), &mut commands);
        }
        return;
    }
    round.elapsed += dt;
    round.time_left = (round.time_left - dt).max(0.0);
    if round.time_left <= 0.0 {
        let mut details = vec![format!("Collected {} / {} coins", round.collected, round.total)];
        if let Some(best) = best.0 {
            details.push(format!("Best clear time: {best}s"));
        }
        end_round(RoundResult { won: false, headline: "Time's up!".into(), details }, &mut commands);
    }
}

/// Builds the win result. `BestScore` holds the fastest clear in whole seconds (lower is better).
fn win_result(elapsed: f32, best: &mut BestScore) -> RoundResult {
    let seconds = elapsed.ceil() as i64;
    let mut details = vec![format!("Cleared the courtyard in {elapsed:.1}s")];
    match best.0 {
        Some(previous) if previous <= seconds => details.push(format!("Best clear time: {previous}s")),
        previous => {
            best.0 = Some(seconds);
            details.push(if previous.is_some() { format!("New best time: {seconds}s!") } else { format!("Best clear time: {seconds}s") });
        }
    }
    RoundResult { won: true, headline: "All coins collected!".into(), details }
}

/// Writes the HUD lines and the banner for the current moment of the round.
fn update_hud(mut hud: ResMut<Hud>, round: Res<Round>) {
    hud.lines = vec![
        format!("Coins {} / {}", round.collected, round.total),
        format!("Time {}s", round.time_left.ceil() as i32),
        "Shift sprint  |  Space jump  |  Right mouse orbit".into(),
    ];
    hud.banner = if round.finish_delay.is_some() {
        Some("All coins!".into())
    } else if round.elapsed < 2.5 {
        Some(format!("Collect all {} coins!", round.total))
    } else if round.time_left <= 10.0 && round.time_left > 7.0 {
        Some("10 seconds left!".into())
    } else if round.total - round.collected == 1 {
        Some("One coin left!".into())
    } else {
        None
    };
}

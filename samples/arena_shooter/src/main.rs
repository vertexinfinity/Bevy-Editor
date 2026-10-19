//! Arena Survivor: a top-down twin-stick shooter built on the Bevylite sample kit.
//!
//! The static arena comes from [`level::spawn_level`] (shared with the editor). This file adds
//! the gameplay: hover-tank movement with a dash, mouse aiming, bullets, enemy waves, contact
//! damage, health pickups and the HUD. `SAMPLE_AUTOPLAY=1` swaps player input for a small
//! autopilot so automated screenshots show real combat.
mod level;

use bevy::{prelude::*, window::PrimaryWindow};
use sample_kit::{BestScore, GameState, Hud, RoundResult, RoundScoped, SampleInfo, SampleKitPlugin, end_round, sample_app};

const ACCENT: Color = Color::srgb(0.3, 0.85, 1.0);
const MAX_HP: i32 = 100;
const PLAYER_SPEED: f32 = 9.0;
const PLAYER_RADIUS: f32 = 0.8;
const DASH_SPEED: f32 = 28.0;
const DASH_TIME: f32 = 0.18;
const DASH_COOLDOWN: f32 = 1.5;
const FIRE_INTERVAL: f32 = 1.0 / 8.0;
const BULLET_SPEED: f32 = 30.0;
const BULLET_LIFE: f32 = 1.1;
const INVULNERABLE_TIME: f32 = 1.0;
const PICKUP_HEAL: i32 = 25;
/// Height of the turret muzzle; mouse aim is projected onto this horizontal plane.
const AIM_HEIGHT: f32 = 0.95;

fn main() {
    let autoplay = std::env::var("SAMPLE_AUTOPLAY").is_ok_and(|v| v == "1");
    sample_app("Arena Survivor")
        .add_plugins(SampleKitPlugin(SampleInfo {
            title: "Arena Survivor",
            tagline: "Hold the arena against endless waves",
            controls: &["WASD  Move", "Mouse  Aim", "Left mouse  Fire", "Space  Dash", "Esc  Pause"],
            accent: ACCENT,
            grab_cursor: false,
        }))
        .insert_resource(ClearColor(Color::srgb(0.02, 0.025, 0.05)))
        .insert_resource(GlobalAmbientLight { color: Color::srgb(0.6, 0.7, 1.0), brightness: 220.0, ..default() })
        .insert_resource(Autopilot(autoplay))
        .insert_resource(Rng(0x9E37_79B9_7F4A_7C15))
        .init_resource::<Round>()
        .init_resource::<PlayerInput>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::Playing), start_round)
        .add_systems(
            Update,
            (
                read_input.run_if(|a: Res<Autopilot>| !a.0),
                autopilot.run_if(|a: Res<Autopilot>| a.0),
                move_player,
                fire,
                move_bullets,
                run_waves,
                move_enemies,
                bullet_hits,
                enemy_contact,
                pickups,
                sparks,
                enemy_flash,
                write_hud,
            )
                .chain()
                .run_if(in_state(GameState::Playing))
                .run_if(not_paused),
        )
        .add_systems(PostUpdate, follow_camera.before(TransformSystems::Propagate))
        .run();
}

// ---------------------------------------------------------------------------------------------
// Resources and components
// ---------------------------------------------------------------------------------------------

/// Level layout data kept from [`level::LevelEntities`].
#[derive(Resource)]
struct Arena {
    half: f32,
    hover_height: f32,
    /// Camera offset from the player, from the level's distance and pitch.
    camera_offset: Vec3,
    spawn_points: Vec<Vec3>,
    obstacles: Vec<(Vec2, f32)>,
}

/// Meshes and materials for everything spawned during a round.
#[derive(Resource)]
struct Kit {
    bullet_mesh: Handle<Mesh>,
    bullet_mat: Handle<StandardMaterial>,
    runner_mesh: Handle<Mesh>,
    runner_mat: Handle<StandardMaterial>,
    brute_mesh: Handle<Mesh>,
    brute_mat: Handle<StandardMaterial>,
    hit_mat: Handle<StandardMaterial>,
    pickup_mesh: Handle<Mesh>,
    pickup_mat: Handle<StandardMaterial>,
    spark_mesh: Handle<Mesh>,
    spark_mat: Handle<StandardMaterial>,
}

/// Per-round player and wave state, reset in [`start_round`].
#[derive(Resource, Default)]
struct Round {
    wave: u32,
    score: i64,
    kills: u32,
    hp: i32,
    over: bool,
    invulnerable: f32,
    dash_cooldown: f32,
    dash_time: f32,
    dash_dir: Vec3,
    fire_cooldown: f32,
    /// Firing stays off until the mouse is released once, so the menu click never shoots.
    fire_armed: bool,
    /// Horizontal aim direction of the turret.
    aim: Vec3,
    banner_time: f32,
    to_spawn: u32,
    spawn_timer: f32,
    next_wave_timer: f32,
}

/// One frame of player intent, written by the keyboard/mouse or by the autopilot.
#[derive(Resource, Default)]
struct PlayerInput {
    /// x = right, y = up the screen (world -Z).
    movement: Vec2,
    aim_point: Option<Vec3>,
    fire: bool,
    dash: bool,
}

/// Whether the test autopilot drives the player (`SAMPLE_AUTOPLAY=1`).
#[derive(Resource)]
struct Autopilot(bool);

/// Tiny xorshift generator, so the sample needs no extra crates.
#[derive(Resource)]
struct Rng(u64);

impl Rng {
    fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Turret;

#[derive(Clone, Copy, PartialEq)]
enum EnemyKind {
    Runner,
    Brute,
}

impl EnemyKind {
    fn speed(self) -> f32 {
        match self {
            EnemyKind::Runner => 6.0,
            EnemyKind::Brute => 2.8,
        }
    }
    fn radius(self) -> f32 {
        match self {
            EnemyKind::Runner => 0.45,
            EnemyKind::Brute => 0.85,
        }
    }
    fn damage(self) -> i32 {
        match self {
            EnemyKind::Runner => 10,
            EnemyKind::Brute => 20,
        }
    }
    fn score(self) -> i64 {
        match self {
            EnemyKind::Runner => 100,
            EnemyKind::Brute => 250,
        }
    }
    fn height(self) -> f32 {
        match self {
            EnemyKind::Runner => 0.45,
            EnemyKind::Brute => 0.75,
        }
    }
}

#[derive(Component)]
struct Enemy {
    kind: EnemyKind,
    hp: i32,
    speed: f32,
    /// Seconds left of the white hit flash.
    flash: f32,
}

#[derive(Component)]
struct Bullet {
    velocity: Vec3,
    life: f32,
}

#[derive(Component)]
struct Pickup {
    age: f32,
}

/// Short-lived burst shown where an enemy dies.
#[derive(Component)]
struct Spark {
    life: f32,
}

fn not_paused(time: Res<Time<Virtual>>) -> bool {
    !time.is_paused()
}

// ---------------------------------------------------------------------------------------------
// Setup and round reset
// ---------------------------------------------------------------------------------------------

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let level = level::spawn_level(&mut commands, &mut meshes, &mut materials);
    commands.entity(level.player).insert(Player);
    commands.entity(level.turret).insert(Turret);
    info!("Arena Survivor: level spawned ({} top-level entities, camera {})", level.all.len(), level.camera);
    commands.insert_resource(Arena {
        half: level.arena_half_size,
        hover_height: level.player_hover_height,
        camera_offset: Vec3::new(0.0, level.camera_pitch.sin(), level.camera_pitch.cos()) * level.camera_distance,
        spawn_points: level.spawn_points,
        obstacles: level.obstacles,
    });

    let glow = |materials: &mut Assets<StandardMaterial>, color: Color, emissive: LinearRgba| materials.add(StandardMaterial { base_color: color, emissive, ..default() });
    commands.insert_resource(Kit {
        bullet_mesh: meshes.add(Sphere::new(0.16)),
        bullet_mat: glow(&mut materials, Color::srgb(1.0, 0.9, 0.5), LinearRgba::rgb(8.0, 5.0, 1.0)),
        runner_mesh: meshes.add(Sphere::new(EnemyKind::Runner.radius())),
        runner_mat: glow(&mut materials, Color::srgb(1.0, 0.3, 0.2), LinearRgba::rgb(1.8, 0.25, 0.1)),
        brute_mesh: meshes.add(Cuboid::from_length(EnemyKind::Brute.radius() * 1.6)),
        brute_mat: glow(&mut materials, Color::srgb(0.6, 0.2, 0.9), LinearRgba::rgb(0.9, 0.15, 1.6)),
        hit_mat: glow(&mut materials, Color::WHITE, LinearRgba::rgb(4.0, 4.0, 4.0)),
        pickup_mesh: meshes.add(Cuboid::new(0.7, 0.22, 0.22)),
        pickup_mat: glow(&mut materials, Color::srgb(0.3, 1.0, 0.45), LinearRgba::rgb(0.3, 3.0, 0.8)),
        spark_mesh: meshes.add(Sphere::new(0.5)),
        spark_mat: glow(&mut materials, Color::srgb(1.0, 0.6, 0.2), LinearRgba::rgb(6.0, 2.5, 0.5)),
    });
}

/// Resets the player and wave state at the start of every round.
fn start_round(
    arena: Res<Arena>,
    mut round: ResMut<Round>,
    mut hud: ResMut<Hud>,
    mut player: Single<(&mut Transform, &mut Visibility), With<Player>>,
) {
    *round = Round { hp: MAX_HP, aim: Vec3::NEG_Z, next_wave_timer: 0.6, ..default() };
    hud.banner = None;
    player.0.translation = Vec3::new(0.0, arena.hover_height, 0.0);
    player.0.rotation = Quat::IDENTITY;
    *player.1 = Visibility::Inherited;
}

// ---------------------------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------------------------

/// Reads WASD, Space and the mouse; the cursor is projected onto the aim plane.
fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut input: ResMut<PlayerInput>,
    mut round: ResMut<Round>,
) {
    let axis = |pos: KeyCode, neg: KeyCode| keys.pressed(pos) as i32 as f32 - keys.pressed(neg) as i32 as f32;
    input.movement = Vec2::new(axis(KeyCode::KeyD, KeyCode::KeyA), axis(KeyCode::KeyW, KeyCode::KeyS)).normalize_or_zero();
    input.dash = keys.just_pressed(KeyCode::Space);
    if !mouse.pressed(MouseButton::Left) {
        round.fire_armed = true;
    }
    input.fire = round.fire_armed && mouse.pressed(MouseButton::Left);

    let (camera, camera_transform) = *camera;
    input.aim_point = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .and_then(|ray| ray.plane_intersection_point(Vec3::Y * AIM_HEIGHT, InfinitePlane3d::new(Vec3::Y)));
}

/// Test-only driver: circle-strafes around the center, sidesteps close enemies and fires at
/// the nearest one.
fn autopilot(
    time: Res<Time>,
    mut input: ResMut<PlayerInput>,
    player: Single<&Transform, With<Player>>,
    enemies: Query<&Transform, With<Enemy>>,
) {
    let position = player.translation.with_y(0.0);
    let t = time.elapsed_secs() * 0.6;
    let mut direction = Vec3::new(t.cos() * 7.0, 0.0, t.sin() * 7.0) - position;
    let nearest = enemies.iter().map(|e| e.translation).min_by(|a, b| a.distance_squared(position).total_cmp(&b.distance_squared(position)));
    let mut danger = false;
    if let Some(enemy) = nearest {
        let away = position - enemy.with_y(0.0);
        if away.length() < 4.0 {
            direction += away.normalize_or_zero() * 6.0;
            danger = away.length() < 2.2;
        }
    }
    let direction = direction.normalize_or_zero();
    input.movement = Vec2::new(direction.x, -direction.z);
    // Hold fire until targets are in range so enemies are visible on screen for a moment.
    let target = nearest.filter(|e| e.with_y(0.0).distance(position) < 11.0);
    input.aim_point = target;
    input.fire = target.is_some();
    input.dash = danger;
}

// ---------------------------------------------------------------------------------------------
// Player
// ---------------------------------------------------------------------------------------------

/// Keeps a circle of `radius` outside every obstacle and inside the walls.
fn resolve_collisions(position: &mut Vec3, radius: f32, arena: &Arena) {
    for (center, obstacle_radius) in &arena.obstacles {
        let offset = Vec2::new(position.x, position.z) - *center;
        let min = obstacle_radius + radius;
        if offset.length_squared() < min * min {
            let pushed = *center + offset.normalize_or(Vec2::X) * min;
            position.x = pushed.x;
            position.z = pushed.y;
        }
    }
    let limit = arena.half - radius;
    position.x = position.x.clamp(-limit, limit);
    position.z = position.z.clamp(-limit, limit);
}

/// Yaw that points `Transform::forward` (-Z) along a horizontal direction.
fn yaw_towards(direction: Vec3) -> Quat {
    Quat::from_rotation_y(f32::atan2(-direction.x, -direction.z))
}

/// Moves the hover tank, handles the dash, turns the hull and turret, and flashes while hurt.
fn move_player(
    time: Res<Time>,
    input: Res<PlayerInput>,
    arena: Res<Arena>,
    mut round: ResMut<Round>,
    mut player: Single<(&mut Transform, &mut Visibility), (With<Player>, Without<Turret>)>,
    mut turret: Single<&mut Transform, (With<Turret>, Without<Player>)>,
) {
    let dt = time.delta_secs();
    let (transform, visibility) = &mut *player;
    let movement = Vec3::new(input.movement.x, 0.0, -input.movement.y);

    round.dash_cooldown = (round.dash_cooldown - dt).max(0.0);
    if input.dash && round.dash_cooldown <= 0.0 {
        round.dash_time = DASH_TIME;
        round.dash_cooldown = DASH_COOLDOWN;
        round.dash_dir = if movement != Vec3::ZERO { movement } else { round.aim };
    }
    let velocity = if round.dash_time > 0.0 {
        round.dash_time -= dt;
        round.dash_dir * DASH_SPEED
    } else {
        movement * PLAYER_SPEED
    };
    transform.translation += velocity * dt;
    resolve_collisions(&mut transform.translation, PLAYER_RADIUS, &arena);
    transform.translation.y = arena.hover_height + (time.elapsed_secs() * 3.0).sin() * 0.05;

    // The hull eases toward the travel direction; the turret tracks the aim point in world space.
    if movement != Vec3::ZERO {
        let target = yaw_towards(movement);
        transform.rotation = transform.rotation.slerp(target, (dt * 10.0).min(1.0));
    }
    if let Some(point) = input.aim_point {
        let direction = (point - transform.translation).with_y(0.0);
        if direction.length_squared() > 0.01 {
            round.aim = direction.normalize();
        }
    }
    turret.rotation = transform.rotation.inverse() * yaw_towards(round.aim);

    round.invulnerable = (round.invulnerable - dt).max(0.0);
    let hidden = round.invulnerable > 0.0 && (round.invulnerable * 12.0) as i32 % 2 == 0;
    visibility.set_if_neq(if hidden { Visibility::Hidden } else { Visibility::Inherited });
}

/// Spawns bullets from the muzzle at the fixed fire rate while firing is held.
fn fire(time: Res<Time>, input: Res<PlayerInput>, kit: Res<Kit>, mut round: ResMut<Round>, player: Single<&Transform, With<Player>>, mut commands: Commands) {
    round.fire_cooldown = (round.fire_cooldown - time.delta_secs()).max(0.0);
    if !input.fire || round.fire_cooldown > 0.0 {
        return;
    }
    round.fire_cooldown = FIRE_INTERVAL;
    let muzzle = player.translation.with_y(AIM_HEIGHT) + round.aim * 1.2;
    commands.spawn((
        Name::new("Bullet"),
        Bullet { velocity: round.aim * BULLET_SPEED, life: BULLET_LIFE },
        Mesh3d(kit.bullet_mesh.clone()),
        MeshMaterial3d(kit.bullet_mat.clone()),
        Transform::from_translation(muzzle),
        RoundScoped,
    ));
}

/// Moves bullets and removes them when they expire, leave the arena or hit a pillar.
fn move_bullets(time: Res<Time>, arena: Res<Arena>, mut bullets: Query<(Entity, &mut Bullet, &mut Transform)>, mut commands: Commands) {
    let dt = time.delta_secs();
    for (entity, mut bullet, mut transform) in &mut bullets {
        transform.translation += bullet.velocity * dt;
        bullet.life -= dt;
        let p = transform.translation;
        let flat = Vec2::new(p.x, p.z);
        let blocked = arena.obstacles.iter().any(|(center, radius)| flat.distance_squared(*center) < radius * radius);
        if bullet.life <= 0.0 || blocked || p.x.abs() > arena.half || p.z.abs() > arena.half {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Enemies and waves
// ---------------------------------------------------------------------------------------------

/// Starts the next wave once the arena is clear, and trickles the wave's enemies in.
fn run_waves(
    time: Res<Time>,
    arena: Res<Arena>,
    kit: Res<Kit>,
    mut rng: ResMut<Rng>,
    mut round: ResMut<Round>,
    mut hud: ResMut<Hud>,
    enemies: Query<(), With<Enemy>>,
    player: Single<&Transform, With<Player>>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    round.banner_time = (round.banner_time - dt).max(0.0);
    hud.banner = (round.banner_time > 0.0).then(|| format!("Wave {}", round.wave));

    if round.to_spawn > 0 {
        round.spawn_timer -= dt;
        if round.spawn_timer <= 0.0 {
            round.spawn_timer = 0.2;
            round.to_spawn -= 1;
            spawn_enemy(&mut commands, &kit, &arena, &mut rng, round.wave, player.translation);
        }
    } else if enemies.is_empty() {
        if round.next_wave_timer > 0.0 {
            round.next_wave_timer -= dt;
            if round.next_wave_timer <= 0.0 {
                round.wave += 1;
                round.to_spawn = 4 + 2 * round.wave;
                round.spawn_timer = 0.0;
                round.banner_time = 2.0;
            }
        } else {
            // Wave cleared: award a bonus and pause briefly before the next one.
            round.score += 100 * round.wave as i64;
            round.next_wave_timer = 1.5;
        }
    }
}

/// Spawns one enemy at a random entry point away from the player.
fn spawn_enemy(commands: &mut Commands, kit: &Kit, arena: &Arena, rng: &mut Rng, wave: u32, player: Vec3) {
    let mut point = arena.spawn_points[0];
    for _ in 0..6 {
        point = arena.spawn_points[(rng.next_f32() * arena.spawn_points.len() as f32) as usize % arena.spawn_points.len()];
        if point.distance(player.with_y(0.0)) > 8.0 {
            break;
        }
    }
    let brute_chance = if wave >= 2 { (0.1 + 0.06 * wave as f32).min(0.45) } else { 0.0 };
    let kind = if rng.next_f32() < brute_chance { EnemyKind::Brute } else { EnemyKind::Runner };
    let (mesh, material, hp) = match kind {
        EnemyKind::Runner => (kit.runner_mesh.clone(), kit.runner_mat.clone(), 1),
        EnemyKind::Brute => (kit.brute_mesh.clone(), kit.brute_mat.clone(), 3),
    };
    let jitter = Vec3::new(rng.next_f32() - 0.5, 0.0, rng.next_f32() - 0.5) * 2.0;
    let speed = kind.speed() * (1.0 + 0.04 * wave.min(10) as f32) * (0.9 + rng.next_f32() * 0.2);
    commands.spawn((
        Name::new(if kind == EnemyKind::Runner { "Runner" } else { "Brute" }),
        Enemy { kind, hp, speed, flash: 0.0 },
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation((point + jitter).with_y(kind.height())),
        RoundScoped,
    ));
}

/// Enemies chase the player, keep a little apart from each other and slide around pillars.
fn move_enemies(time: Res<Time>, arena: Res<Arena>, player: Single<&Transform, (With<Player>, Without<Enemy>)>, mut enemies: Query<(&Enemy, &mut Transform)>) {
    let dt = time.delta_secs();
    let target = player.translation.with_y(0.0);
    let others: Vec<(Vec3, f32)> = enemies.iter().map(|(e, t)| (t.translation.with_y(0.0), e.kind.radius())).collect();
    for (enemy, mut transform) in &mut enemies {
        let position = transform.translation.with_y(0.0);
        let mut steer = (target - position).normalize_or_zero();
        for (other, radius) in &others {
            let away = position - *other;
            let min = radius + enemy.kind.radius() + 0.2;
            let distance = away.length();
            if distance > 0.001 && distance < min {
                steer += away / distance * (min - distance) * 2.0;
            }
        }
        transform.translation += steer.normalize_or_zero() * enemy.speed * dt;
        resolve_collisions(&mut transform.translation, enemy.kind.radius(), &arena);
        transform.translation.y = enemy.kind.height();
        if enemy.kind == EnemyKind::Brute {
            transform.rotate_y(dt * 1.5);
        }
    }
}

/// Bullet-vs-enemy sphere checks; kills award score and sometimes drop a health pickup.
fn bullet_hits(
    kit: Res<Kit>,
    mut rng: ResMut<Rng>,
    mut round: ResMut<Round>,
    bullets: Query<(Entity, &Transform), With<Bullet>>,
    mut enemies: Query<(Entity, &mut Enemy, &Transform), Without<Bullet>>,
    mut commands: Commands,
) {
    for (bullet, bullet_transform) in &bullets {
        let b = bullet_transform.translation.with_y(0.0);
        let hit = enemies.iter_mut().find(|(_, enemy, transform)| enemy.hp > 0 && transform.translation.with_y(0.0).distance(b) < enemy.kind.radius() + 0.2);
        let Some((entity, mut enemy, transform)) = hit else { continue };
        commands.entity(bullet).despawn();
        enemy.hp -= 1;
        enemy.flash = 0.08;
        if enemy.hp > 0 {
            continue;
        }
        commands.entity(entity).despawn();
        round.kills += 1;
        round.score += enemy.kind.score();
        let position = transform.translation;
        commands.spawn((Name::new("Spark"), Spark { life: 0.25 }, Mesh3d(kit.spark_mesh.clone()), MeshMaterial3d(kit.spark_mat.clone()), Transform::from_translation(position), RoundScoped));
        let drop_chance = if enemy.kind == EnemyKind::Brute { 0.3 } else { 0.08 };
        if rng.next_f32() < drop_chance {
            spawn_pickup(&mut commands, &kit, position.with_y(0.6));
        }
    }
}

/// A green cross that heals the player on contact.
fn spawn_pickup(commands: &mut Commands, kit: &Kit, position: Vec3) {
    let pickup = commands
        .spawn((Name::new("Health Pickup"), Pickup { age: 0.0 }, Mesh3d(kit.pickup_mesh.clone()), MeshMaterial3d(kit.pickup_mat.clone()), Transform::from_translation(position), RoundScoped))
        .id();
    commands.spawn((
        Name::new("Cross Bar"),
        Mesh3d(kit.pickup_mesh.clone()),
        MeshMaterial3d(kit.pickup_mat.clone()),
        Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ChildOf(pickup),
    ));
}

/// Enemies touching the player deal damage, then the player is briefly invulnerable.
fn enemy_contact(
    mut round: ResMut<Round>,
    mut best: ResMut<BestScore>,
    player: Single<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<(&Enemy, &mut Transform)>,
    mut commands: Commands,
) {
    if round.over {
        return;
    }
    let position = player.translation.with_y(0.0);
    for (enemy, mut transform) in &mut enemies {
        let offset = transform.translation.with_y(0.0) - position;
        if offset.length() > enemy.kind.radius() + PLAYER_RADIUS {
            continue;
        }
        // Push the attacker back so it does not sit inside the player.
        transform.translation += offset.normalize_or(Vec3::X) * 1.5;
        if round.invulnerable > 0.0 || round.dash_time > 0.0 {
            continue;
        }
        round.hp = (round.hp - enemy.kind.damage()).max(0);
        round.invulnerable = INVULNERABLE_TIME;
    }
    if round.hp > 0 {
        return;
    }
    round.over = true;
    let previous = best.0;
    let new_best = previous.is_none_or(|b| round.score > b);
    if new_best {
        best.0 = Some(round.score);
    }
    let mut details = vec![format!("Reached wave {}", round.wave), format!("Kills: {}", round.kills), format!("Score: {}", round.score)];
    details.push(if new_best { "New best score!".to_string() } else { format!("Best: {}", previous.unwrap_or(0)) });
    end_round(RoundResult { won: false, headline: "Overrun".to_string(), details }, &mut commands);
}

/// Spins and bobs pickups, heals on contact and expires them after a while.
fn pickups(time: Res<Time>, mut round: ResMut<Round>, player: Single<&Transform, (With<Player>, Without<Pickup>)>, mut pickups: Query<(Entity, &mut Pickup, &mut Transform)>, mut commands: Commands) {
    let dt = time.delta_secs();
    for (entity, mut pickup, mut transform) in &mut pickups {
        pickup.age += dt;
        transform.rotate_y(dt * 2.0);
        transform.translation.y = 0.6 + (pickup.age * 3.0).sin() * 0.12;
        let close = transform.translation.with_y(0.0).distance(player.translation.with_y(0.0)) < PLAYER_RADIUS + 0.6;
        if close && round.hp < MAX_HP {
            round.hp = (round.hp + PICKUP_HEAL).min(MAX_HP);
            commands.entity(entity).despawn();
        } else if pickup.age > 12.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Shrinks and removes death bursts.
fn sparks(time: Res<Time>, mut sparks: Query<(Entity, &mut Spark, &mut Transform)>, mut commands: Commands) {
    for (entity, mut spark, mut transform) in &mut sparks {
        spark.life -= time.delta_secs();
        if spark.life <= 0.0 {
            commands.entity(entity).despawn();
        } else {
            transform.scale = Vec3::splat(spark.life * 6.0);
        }
    }
}

/// Swaps an enemy's material to white for a moment after it is hit.
fn enemy_flash(time: Res<Time>, kit: Res<Kit>, mut enemies: Query<(&mut Enemy, &mut MeshMaterial3d<StandardMaterial>)>) {
    for (mut enemy, mut material) in &mut enemies {
        enemy.flash = (enemy.flash - time.delta_secs()).max(0.0);
        let wanted = if enemy.flash > 0.0 {
            &kit.hit_mat
        } else if enemy.kind == EnemyKind::Brute {
            &kit.brute_mat
        } else {
            &kit.runner_mat
        };
        if material.0 != *wanted {
            material.0 = wanted.clone();
        }
    }
}

fn write_hud(round: Res<Round>, mut hud: ResMut<Hud>) {
    let dash = if round.dash_cooldown > 0.0 { format!("Dash {:.1}s", round.dash_cooldown) } else { "Dash ready".to_string() };
    hud.lines = vec![format!("Wave {}", round.wave.max(1)), format!("Score {}", round.score), format!("HP {} / {}", round.hp, MAX_HP), dash];
}

// ---------------------------------------------------------------------------------------------
// Camera
// ---------------------------------------------------------------------------------------------

/// Smoothly follows the player at the level's fixed pitch (runs in every state).
fn follow_camera(
    time: Res<Time<Real>>,
    arena: Res<Arena>,
    player: Single<&Transform, (With<Player>, Without<Camera3d>)>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
) {
    let offset = arena.camera_offset;
    let target = player.translation.with_y(0.0) + offset;
    let t = (time.delta_secs() * 5.0).min(1.0);
    camera.translation = camera.translation.lerp(target, t);
    camera.rotation = Transform::from_translation(offset).looking_at(Vec3::ZERO, Vec3::Y).rotation;
}

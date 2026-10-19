//! Target Range: a first-person shooting gallery built on `sample_kit`.
//!
//! The static range lives in `level.rs` (shared with the editor); this file adds the FPS
//! controller, hitscan shooting, pop-up targets, the HUD and an autopilot for test runs.
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};
use sample_kit::{BestScore, GameState, Hud, RoundResult, RoundScoped, SampleInfo, SampleKitPlugin, end_round};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

mod level;

const ROUND_SECONDS: f32 = 60.0;
const MAGAZINE: u32 = 12;
const RELOAD_SECONDS: f32 = 1.2;
const ACTIVE_TARGETS: usize = 4;
const TARGET_RADIUS: f32 = 0.45;
const EYE_HEIGHT: f32 = 1.7;
const PLAYER_RADIUS: f32 = 0.35;
const WALK_SPEED: f32 = 5.0;
const SPRINT_SPEED: f32 = 8.5;
const JUMP_SPEED: f32 = 7.0;
const GRAVITY: f32 = 20.0;
const MOUSE_SENSITIVITY: f32 = 0.0022;
const ACCENT: Color = Color::srgb(1.0, 0.42, 0.30);

fn main() {
    let mut app = sample_kit::sample_app("Target Range");
    app.add_plugins(SampleKitPlugin(SampleInfo {
        title: "Target Range",
        tagline: "60 seconds, 12 rounds a mag. Pop every target you can.",
        controls: &["Mouse  Look", "WASD  Move    Shift  Sprint    Space  Jump", "Left click  Fire    R  Reload", "Moving targets are worth +50", "Esc  Pause"],
        accent: ACCENT,
        grab_cursor: true,
    }))
    .insert_resource(ClearColor(Color::srgb(0.55, 0.72, 0.88)))
    .insert_resource(GlobalAmbientLight { color: Color::srgb(0.85, 0.88, 1.0), brightness: 350.0, ..default() })
    .insert_resource(Autoplay(std::env::var("SAMPLE_AUTOPLAY").is_ok_and(|v| v == "1")))
    .init_resource::<Round>()
    .add_systems(Startup, setup)
    .add_systems(PostStartup, capture_start_pose)
    .add_systems(OnEnter(GameState::Playing), start_round)
    .add_systems(
        Update,
        (look, move_player, shoot, animate_targets, respawn_targets, update_particles, apply_view, tick_round)
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(Update, update_crosshair)
    .run();
}

/// True when `SAMPLE_AUTOPLAY=1`: the autopilot aims and fires instead of the mouse.
#[derive(Resource)]
struct Autoplay(bool);

/// Level data and shared assets used by the gameplay systems.
#[derive(Resource)]
struct Range {
    camera: Entity,
    gun: Entity,
    muzzle: Entity,
    spots: Vec<Vec3>,
    colliders: Vec<(Vec3, Vec3)>,
    bounds: (Vec2, Vec2),
    start: Transform,
    disc: Handle<Mesh>,
    ring: Handle<Mesh>,
    bullseye: Handle<Mesh>,
    spark: Handle<Mesh>,
    red: Handle<StandardMaterial>,
    white: Handle<StandardMaterial>,
    dust: Handle<StandardMaterial>,
}

/// FPS controller state, stored on the camera (the camera position is the eye).
#[derive(Component, Default)]
struct Player {
    yaw: f32,
    pitch: f32,
    velocity_y: f32,
    grounded: bool,
    /// Extra pitch from recoil, decays back to zero.
    recoil: f32,
}

/// Per-round score, ammo and timers.
#[derive(Resource)]
struct Round {
    score: i64,
    hits: u32,
    shots: u32,
    time_left: f32,
    ammo: u32,
    reload: Option<f32>,
    cooldown: f32,
    muzzle_flash: f32,
    hit_marker: f32,
    popup: Option<(String, f32)>,
    respawns: Vec<f32>,
    autopilot_timer: f32,
    autopilot_fire: bool,
    rng: u64,
}

impl Default for Round {
    fn default() -> Self {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x9E37_79B9);
        Self {
            score: 0,
            hits: 0,
            shots: 0,
            time_left: ROUND_SECONDS,
            ammo: MAGAZINE,
            reload: None,
            // Short grace period so the click that started the round never fires.
            cooldown: 0.25,
            muzzle_flash: 0.0,
            hit_marker: 0.0,
            popup: None,
            respawns: Vec::new(),
            autopilot_timer: 0.0,
            autopilot_fire: false,
            rng: seed | 1,
        }
    }
}

impl Round {
    /// Xorshift random number in `0.0..1.0`.
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// A pop-up target at one of the level's spots.
#[derive(Component)]
struct Target {
    spot: usize,
    moving: bool,
    phase: f32,
    age: f32,
}

/// Short-lived hit spark or dust puff.
#[derive(Component)]
struct Particle {
    velocity: Vec3,
    life: f32,
}

/// One bar of the centered crosshair.
#[derive(Component)]
struct CrosshairPart;

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let level = level::spawn_level(&mut commands, &mut meshes, &mut materials);
    info!("Target Range: {} level entities, {} target spots, {} colliders", level.all.len(), level.target_spots.len(), level.colliders.len());

    // A simple view-model pistol and a muzzle light, parented to the camera.
    let gun_metal = materials.add(StandardMaterial { base_color: Color::srgb(0.12, 0.13, 0.15), metallic: 0.6, perceptual_roughness: 0.4, ..default() });
    let gun_grip = materials.add(StandardMaterial { base_color: Color::srgb(0.35, 0.22, 0.12), perceptual_roughness: 0.8, ..default() });
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let gun = commands
        .spawn((Name::new("View Model - Pistol"), Transform::from_xyz(0.22, -0.2, -0.62), Visibility::Inherited, ChildOf(level.camera)))
        .id();
    commands.spawn((Name::new("Pistol Slide"), Mesh3d(cube.clone()), MeshMaterial3d(gun_metal), Transform::from_scale(Vec3::new(0.05, 0.06, 0.28)), ChildOf(gun)));
    commands.spawn((
        Name::new("Pistol Grip"),
        Mesh3d(cube),
        MeshMaterial3d(gun_grip),
        Transform::from_xyz(0.0, -0.08, 0.09).with_rotation(Quat::from_rotation_x(-0.25)).with_scale(Vec3::new(0.045, 0.13, 0.06)),
        ChildOf(gun),
    ));
    let muzzle = commands
        .spawn((
            Name::new("Muzzle Flash"),
            PointLight { color: Color::srgb(1.0, 0.75, 0.4), intensity: 0.0, range: 8.0, ..default() },
            Transform::from_xyz(0.22, -0.15, -1.0),
            ChildOf(level.camera),
        ))
        .id();
    commands.entity(level.camera).insert(Player::default());

    let material = |materials: &mut Assets<StandardMaterial>, color: Color, emissive: LinearRgba| {
        materials.add(StandardMaterial { base_color: color, emissive, perceptual_roughness: 0.5, ..default() })
    };
    commands.insert_resource(Range {
        camera: level.camera,
        gun,
        muzzle,
        spots: level.target_spots,
        colliders: level.colliders,
        bounds: level.bounds,
        start: Transform::from_xyz(0.0, EYE_HEIGHT, 6.0),
        disc: meshes.add(Cylinder::new(TARGET_RADIUS, 0.06)),
        ring: meshes.add(Cylinder::new(0.31, 0.07)),
        bullseye: meshes.add(Cylinder::new(0.15, 0.08)),
        spark: meshes.add(Sphere::new(0.05)),
        red: material(&mut materials, Color::srgb(0.85, 0.08, 0.06), LinearRgba::rgb(0.25, 0.0, 0.0)),
        white: material(&mut materials, Color::srgb(0.96, 0.95, 0.92), LinearRgba::rgb(0.08, 0.08, 0.08)),
        dust: material(&mut materials, Color::srgb(0.70, 0.62, 0.50), LinearRgba::BLACK),
    });
}

/// Remembers where the level placed the camera, so every round starts there.
fn capture_start_pose(mut range: ResMut<Range>, transforms: Query<&Transform>) {
    if let Ok(transform) = transforms.get(range.camera) {
        range.start = *transform;
    }
}

/// Resets the player, score and targets, and spawns the crosshair.
fn start_round(mut commands: Commands, range: Res<Range>, mut round: ResMut<Round>, mut hud: ResMut<Hud>, mut cameras: Query<(&mut Transform, &mut Player)>) {
    let rng = round.rng;
    *round = Round { rng, ..default() };
    hud.banner = None;
    if let Ok((mut transform, mut player)) = cameras.get_mut(range.camera) {
        let (yaw, pitch, _) = range.start.rotation.to_euler(EulerRot::YXZ);
        *transform = range.start;
        *player = Player { yaw, pitch, grounded: true, ..default() };
    }
    let mut used = Vec::new();
    for _ in 0..ACTIVE_TARGETS {
        spawn_target(&mut commands, &range, &mut round, &mut used);
    }
    spawn_crosshair(&mut commands);
}

/// Spawns a red/white disc target at a random spot not in `used`.
fn spawn_target(commands: &mut Commands, range: &Range, round: &mut Round, used: &mut Vec<usize>) {
    let free: Vec<usize> = (0..range.spots.len()).filter(|i| !used.contains(i)).collect();
    if free.is_empty() {
        return;
    }
    let spot = free[((round.random() * free.len() as f32) as usize).min(free.len() - 1)];
    used.push(spot);
    let moving = round.random() < 0.4;
    let target = Target { spot, moving, phase: round.random() * TAU, age: 0.0 };
    // The cylinders' axis is turned toward the player (along Z); the target pops up from scale 0.
    commands
        .spawn((
            Name::new(if moving { "Target - Moving" } else { "Target" }),
            target,
            RoundScoped,
            Transform::from_translation(range.spots[spot]).with_rotation(Quat::from_rotation_x(FRAC_PI_2)).with_scale(Vec3::ZERO),
            Visibility::Inherited,
        ))
        .with_children(|parent| {
            parent.spawn((Mesh3d(range.disc.clone()), MeshMaterial3d(range.red.clone())));
            parent.spawn((Mesh3d(range.ring.clone()), MeshMaterial3d(range.white.clone())));
            parent.spawn((Mesh3d(range.bullseye.clone()), MeshMaterial3d(range.red.clone())));
        });
}

/// Centered crosshair: four short bars and a dot.
fn spawn_crosshair(commands: &mut Commands) {
    let root = commands
        .spawn((
            Name::new("Crosshair"),
            RoundScoped,
            Node { position_type: PositionType::Absolute, left: percent(50), top: percent(50), width: px(0), height: px(0), ..default() },
        ))
        .id();
    // (left, top, width, height) in pixels relative to the screen center.
    let bars = [(-13.0, -1.0, 8.0, 2.0), (5.0, -1.0, 8.0, 2.0), (-1.0, -13.0, 2.0, 8.0), (-1.0, 5.0, 2.0, 8.0), (-1.5, -1.5, 3.0, 3.0)];
    for (left, top, width, height) in bars {
        commands.spawn((
            CrosshairPart,
            Node { position_type: PositionType::Absolute, left: px(left), top: px(top), width: px(width), height: px(height), ..default() },
            BackgroundColor(Color::WHITE),
            ChildOf(root),
        ));
    }
}

/// Mouse look (or the autopilot's aim). Ignored while paused.
fn look(
    time: Res<Time<Virtual>>,
    mouse: Res<AccumulatedMouseMotion>,
    autoplay: Res<Autoplay>,
    range: Res<Range>,
    mut round: ResMut<Round>,
    mut players: Query<(&Transform, &mut Player)>,
    targets: Query<(&Transform, &Target)>,
) {
    if time.is_paused() {
        return;
    }
    let Ok((transform, mut player)) = players.get_mut(range.camera) else { return };
    if !autoplay.0 {
        player.yaw -= mouse.delta.x * MOUSE_SENSITIVITY;
        player.pitch = (player.pitch - mouse.delta.y * MOUSE_SENSITIVITY).clamp(-1.45, 1.45);
        return;
    }
    // Autopilot: turn toward the target closest to the current aim, fire every 0.4 s once on it.
    let eye = transform.translation;
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.0) * Vec3::NEG_Z;
    let best = targets
        .iter()
        .filter(|(_, target)| target.age > 0.2)
        .map(|(t, _)| (t.translation - eye).normalize())
        .max_by(|a, b| a.dot(forward).total_cmp(&b.dot(forward)));
    round.autopilot_timer += time.delta_secs();
    round.autopilot_fire = false;
    let Some(dir) = best else { return };
    let yaw = (-dir.x).atan2(-dir.z);
    let pitch = dir.y.asin();
    let mut yaw_error = (yaw - player.yaw) % TAU;
    if yaw_error > PI {
        yaw_error -= TAU;
    } else if yaw_error < -PI {
        yaw_error += TAU;
    }
    let blend = (time.delta_secs() * 9.0).min(1.0);
    player.yaw += yaw_error * blend;
    player.pitch += (pitch - player.pitch) * blend;
    if forward.dot(dir) > 0.9996 && round.autopilot_timer >= 0.4 {
        round.autopilot_timer = 0.0;
        round.autopilot_fire = true;
    }
}

/// WASD movement with sprint, jump, gravity, arena bounds and crate collisions.
fn move_player(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, range: Res<Range>, mut players: Query<(&mut Transform, &mut Player)>) {
    let dt = time.delta_secs();
    if dt == 0.0 {
        return;
    }
    let Ok((mut transform, mut player)) = players.get_mut(range.camera) else { return };
    let mut input = Vec2::ZERO;
    for (key, dir) in [(KeyCode::KeyW, Vec2::Y), (KeyCode::KeyS, Vec2::NEG_Y), (KeyCode::KeyA, Vec2::NEG_X), (KeyCode::KeyD, Vec2::X)] {
        if keys.pressed(key) {
            input += dir;
        }
    }
    let speed = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) { SPRINT_SPEED } else { WALK_SPEED };
    let (sin, cos) = player.yaw.sin_cos();
    let forward = Vec3::new(-sin, 0.0, -cos);
    let right = Vec3::new(cos, 0.0, -sin);
    let step = (forward * input.y + right * input.x).normalize_or_zero() * speed * dt;

    let mut position = transform.translation + step;
    let feet = position.y - EYE_HEIGHT;
    // Push out of boxes taller than our feet; stand on the ones we are above.
    let mut ground: f32 = 0.0;
    for (center, half) in &range.colliders {
        let top = center.y + half.y;
        let closest = Vec2::new(position.x.clamp(center.x - half.x, center.x + half.x), position.z.clamp(center.z - half.z, center.z + half.z));
        let offset = Vec2::new(position.x, position.z) - closest;
        if offset.length_squared() >= PLAYER_RADIUS * PLAYER_RADIUS {
            continue;
        }
        if top <= feet + 0.05 {
            ground = ground.max(top);
        } else if center.y - half.y < feet + EYE_HEIGHT {
            let push = if offset.length_squared() > 1e-6 {
                offset.normalize() * (PLAYER_RADIUS - offset.length())
            } else {
                -Vec2::new(step.x, step.z).normalize_or_zero() * PLAYER_RADIUS
            };
            position.x += push.x;
            position.z += push.y;
        }
    }
    let (min, max) = range.bounds;
    position.x = position.x.clamp(min.x + PLAYER_RADIUS, max.x - PLAYER_RADIUS);
    position.z = position.z.clamp(min.y + PLAYER_RADIUS, max.y - PLAYER_RADIUS);

    if player.grounded && keys.just_pressed(KeyCode::Space) {
        player.velocity_y = JUMP_SPEED;
        player.grounded = false;
    }
    player.velocity_y -= GRAVITY * dt;
    let mut feet = feet + player.velocity_y * dt;
    if feet <= ground {
        feet = ground;
        player.velocity_y = 0.0;
        player.grounded = true;
    } else {
        player.grounded = false;
    }
    position.y = feet + EYE_HEIGHT;
    transform.translation = position;
}

/// Ray vs axis-aligned box; returns the entry distance.
fn ray_box(origin: Vec3, dir: Vec3, center: Vec3, half: Vec3) -> Option<f32> {
    let inv = dir.recip();
    let t1 = (center - half - origin) * inv;
    let t2 = (center + half - origin) * inv;
    let near = t1.min(t2).max_element().max(0.0);
    let far = t1.max(t2).min_element();
    (far >= near).then_some(near)
}

/// Ray vs sphere; returns the entry distance.
fn ray_sphere(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let to_center = center - origin;
    let along = to_center.dot(dir);
    let miss = to_center.length_squared() - along * along;
    if along < 0.0 || miss > radius * radius {
        return None;
    }
    Some(along - (radius * radius - miss).sqrt())
}

/// Firing, reloading and hitscan hit detection.
fn shoot(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    autoplay: Res<Autoplay>,
    range: Res<Range>,
    mut round: ResMut<Round>,
    mut players: Query<(&Transform, &mut Player)>,
    targets: Query<(Entity, &Transform, &Target)>,
) {
    if time.is_paused() {
        return;
    }
    let dt = time.delta_secs();
    round.cooldown -= dt;
    if let Some(left) = round.reload.as_mut() {
        *left -= dt;
        if *left <= 0.0 {
            round.reload = None;
            round.ammo = MAGAZINE;
        }
    }
    if keys.just_pressed(KeyCode::KeyR) && round.reload.is_none() && round.ammo < MAGAZINE {
        round.reload = Some(RELOAD_SECONDS);
    }
    let trigger = if autoplay.0 { round.autopilot_fire } else { mouse.just_pressed(MouseButton::Left) };
    if !trigger || round.cooldown > 0.0 || round.reload.is_some() {
        return;
    }
    let Ok((transform, mut player)) = players.get_mut(range.camera) else { return };
    round.ammo -= 1;
    round.shots += 1;
    round.cooldown = 0.12;
    round.muzzle_flash = 0.06;
    player.recoil += 0.035;
    if round.ammo == 0 {
        // Auto-reload on an empty magazine.
        round.reload = Some(RELOAD_SECONDS);
    }

    let origin = transform.translation;
    let dir = transform.forward().as_vec3();
    // Nearest solid surface: level boxes or the floor.
    let mut wall = range.colliders.iter().filter_map(|(c, h)| ray_box(origin, dir, *c, *h)).fold(f32::INFINITY, f32::min);
    if dir.y < 0.0 {
        wall = wall.min(-origin.y / dir.y);
    }
    let hit = targets
        .iter()
        .filter_map(|(entity, t, target)| ray_sphere(origin, dir, t.translation, TARGET_RADIUS * t.scale.x).map(|d| (d, entity, target.moving)))
        .filter(|(d, ..)| *d < wall)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    if let Some((distance, entity, moving)) = hit {
        let points = if moving { 150 } else { 100 };
        round.score += points;
        round.hits += 1;
        round.hit_marker = 0.15;
        round.popup = Some((format!("+{points}"), 0.6));
        round.respawns.push(0.6);
        commands.entity(entity).despawn();
        let point = origin + dir * distance;
        for i in 0..14 {
            let material = if i % 2 == 0 { range.red.clone() } else { range.white.clone() };
            spawn_particle(&mut commands, &range, &mut round, point, material, 4.0);
        }
    } else if wall.is_finite() {
        let point = origin + dir * (wall - 0.02);
        for _ in 0..5 {
            spawn_particle(&mut commands, &range, &mut round, point, range.dust.clone(), 1.5);
        }
    }
}

/// Spawns one spark flying out from `at`.
fn spawn_particle(commands: &mut Commands, range: &Range, round: &mut Round, at: Vec3, material: Handle<StandardMaterial>, speed: f32) {
    let velocity = Vec3::new(round.random() - 0.5, round.random() * 0.8 + 0.2, round.random() - 0.5).normalize() * speed * (0.5 + round.random());
    commands.spawn((
        Name::new("Particle"),
        Particle { velocity, life: 0.45 },
        RoundScoped,
        Mesh3d(range.spark.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(at),
    ));
}

/// Pop-up scale animation and side-to-side sliding.
fn animate_targets(time: Res<Time>, range: Res<Range>, mut targets: Query<(&mut Transform, &mut Target)>) {
    for (mut transform, mut target) in &mut targets {
        target.age += time.delta_secs();
        let pop = (target.age / 0.25).min(1.0);
        transform.scale = Vec3::splat(1.0 - (1.0 - pop).powi(3));
        if target.moving {
            let base = range.spots[target.spot];
            transform.translation.x = base.x + (target.phase + target.age * 1.7).sin() * 1.6;
        }
    }
}

/// Brings a new target up a moment after one is hit, at a free spot.
fn respawn_targets(mut commands: Commands, time: Res<Time>, range: Res<Range>, mut round: ResMut<Round>, targets: Query<&Target>) {
    let dt = time.delta_secs();
    let mut ready = 0;
    round.respawns.retain_mut(|timer| {
        *timer -= dt;
        if *timer <= 0.0 {
            ready += 1;
        }
        *timer > 0.0
    });
    let mut used: Vec<usize> = targets.iter().map(|t| t.spot).collect();
    for _ in 0..ready {
        spawn_target(&mut commands, &range, &mut round, &mut used);
    }
}

/// Moves sparks under gravity and shrinks them out.
fn update_particles(mut commands: Commands, time: Res<Time>, mut particles: Query<(Entity, &mut Transform, &mut Particle)>) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut particle) in &mut particles {
        particle.life -= dt;
        if particle.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        particle.velocity.y -= 9.0 * dt;
        transform.translation += particle.velocity * dt;
        transform.scale = Vec3::splat(particle.life / 0.45);
    }
}

/// Applies yaw/pitch plus recoil to the camera, and drives the gun kick and muzzle light.
fn apply_view(
    time: Res<Time>,
    range: Res<Range>,
    mut round: ResMut<Round>,
    mut players: Query<&mut Player>,
    mut transforms: Query<&mut Transform>,
    mut lights: Query<&mut PointLight>,
) {
    let dt = time.delta_secs();
    let Ok(mut player) = players.get_mut(range.camera) else { return };
    player.recoil *= (-dt * 12.0).exp();
    round.muzzle_flash = (round.muzzle_flash - dt).max(0.0);
    let pitch = (player.pitch + player.recoil).clamp(-1.5, 1.5);
    if let Ok(mut camera) = transforms.get_mut(range.camera) {
        camera.rotation = Quat::from_euler(EulerRot::YXZ, player.yaw, pitch, 0.0);
    }
    if let Ok(mut gun) = transforms.get_mut(range.gun) {
        gun.translation.z = -0.62 + player.recoil * 2.5;
        gun.rotation = Quat::from_rotation_x(player.recoil * 4.0);
    }
    if let Ok(mut light) = lights.get_mut(range.muzzle) {
        light.intensity = if round.muzzle_flash > 0.0 { 80_000.0 } else { 0.0 };
    }
}

/// Counts down the round, writes the HUD and ends the round at zero.
fn tick_round(mut commands: Commands, time: Res<Time>, mut round: ResMut<Round>, mut hud: ResMut<Hud>, mut best: ResMut<BestScore>) {
    let dt = time.delta_secs();
    round.time_left -= dt;
    round.hit_marker = (round.hit_marker - dt).max(0.0);
    if let Some((_, left)) = round.popup.as_mut() {
        *left -= dt;
    }
    if round.popup.as_ref().is_some_and(|(_, left)| *left <= 0.0) {
        round.popup = None;
    }
    hud.banner = round.popup.as_ref().map(|(text, _)| text.clone());
    let accuracy = (round.shots > 0).then(|| round.hits as f32 / round.shots as f32 * 100.0);
    hud.lines = vec![
        format!("Score {}", round.score),
        format!("Time {}s", round.time_left.max(0.0).ceil() as i32),
        match round.reload {
            Some(_) => "Reloading...".to_string(),
            None => format!("Ammo {} / {MAGAZINE}", round.ammo),
        },
        match accuracy {
            Some(a) => format!("Accuracy {a:.0}%"),
            None => "Accuracy --".to_string(),
        },
    ];
    if round.time_left > 0.0 {
        return;
    }
    let new_best = best.0.is_none_or(|b| round.score > b);
    if new_best {
        best.0 = Some(round.score);
    }
    hud.banner = None;
    end_round(
        RoundResult {
            won: new_best,
            headline: if new_best { "New best!".to_string() } else { "Time's up".to_string() },
            details: vec![
                format!("Score  {}", round.score),
                format!("Hits  {} of {} shots", round.hits, round.shots),
                format!("Accuracy  {:.0}%", accuracy.unwrap_or(0.0)),
                format!("Best  {}", best.0.unwrap_or(0)),
            ],
        },
        &mut commands,
    );
}

/// Shows the crosshair only while actually playing, and tints it red on a hit.
fn update_crosshair(
    state: Res<State<GameState>>,
    time: Res<Time<Virtual>>,
    round: Res<Round>,
    mut parts: Query<(&mut BackgroundColor, &mut Visibility), With<CrosshairPart>>,
) {
    let visible = *state.get() == GameState::Playing && !time.is_paused();
    let color = if round.hit_marker > 0.0 { Color::srgb(1.0, 0.25, 0.2) } else { Color::srgba(1.0, 1.0, 1.0, 0.9) };
    for (mut background, mut visibility) in &mut parts {
        background.0 = color;
        visibility.set_if_neq(if visible { Visibility::Inherited } else { Visibility::Hidden });
    }
}

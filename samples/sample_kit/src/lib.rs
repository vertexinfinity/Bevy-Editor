//! Shared game flow for the Bevylite samples: title menu, HUD, pause, game over and restart.
//!
//! A sample adds [`SampleKitPlugin`], spawns its per-round entities with [`RoundScoped`] in
//! `OnEnter(GameState::Playing)`, writes its HUD lines into [`Hud`], and calls
//! [`end_round`] when the player wins or loses. Everything else (menus, Enter to start or
//! restart, Esc to pause, cleanup between rounds) is handled here.
//!
//! Test hooks (used by the editor's verification scripts):
//! - `SAMPLE_AUTOPLAY=1` starts a round automatically after one second.
//! - `SAMPLE_SCREENSHOT=<path>` saves a screenshot after `SAMPLE_SCREENSHOT_DELAY` seconds
//!   (default 4) and then exits.
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::{CursorGrabMode, CursorOptions},
};

/// Top-level flow shared by every sample.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
    GameOver,
}

/// Static text shown on the title screen.
#[derive(Clone)]
pub struct SampleInfo {
    pub title: &'static str,
    pub tagline: &'static str,
    /// One control per line, e.g. `"WASD  Move"`.
    pub controls: &'static [&'static str],
    /// Accent color for titles and highlights.
    pub accent: Color,
    /// Lock and hide the cursor while playing (first-person games).
    pub grab_cursor: bool,
}

pub struct SampleKitPlugin(pub SampleInfo);

/// Lines drawn in the top-left HUD panel while playing.
#[derive(Resource, Default)]
pub struct Hud {
    pub lines: Vec<String>,
    /// Large centered banner (e.g. "Wave 3"); cleared by the game.
    pub banner: Option<String>,
}

/// Result of the last round, shown on the game-over screen.
#[derive(Resource, Default, Clone)]
pub struct RoundResult {
    pub won: bool,
    pub headline: String,
    pub details: Vec<String>,
}

/// Best score per sample for this session (games decide what "score" means).
#[derive(Resource, Default)]
pub struct BestScore(pub Option<i64>);

/// Entities despawned when a round ends, so the next round starts clean.
#[derive(Component, Default)]
pub struct RoundScoped;

#[derive(Resource, Clone)]
struct Info(SampleInfo);

#[derive(Component)]
struct MenuRoot;
#[derive(Component)]
struct GameOverRoot;
#[derive(Component)]
struct HudText;
#[derive(Component)]
struct BannerText;
#[derive(Component)]
struct PausedText;

#[derive(Resource)]
struct TestHooks {
    autoplay: bool,
    screenshot: Option<String>,
    delay: f32,
    taken: bool,
}

impl Plugin for SampleKitPlugin {
    fn build(&self, app: &mut App) {
        let screenshot = std::env::var("SAMPLE_SCREENSHOT").ok();
        let delay = std::env::var("SAMPLE_SCREENSHOT_DELAY").ok().and_then(|d| d.parse().ok()).unwrap_or(4.0);
        app.init_state::<GameState>()
            .insert_resource(Info(self.0.clone()))
            .init_resource::<Hud>()
            .init_resource::<RoundResult>()
            .init_resource::<BestScore>()
            .insert_resource(TestHooks {
                autoplay: std::env::var("SAMPLE_AUTOPLAY").is_ok_and(|v| v == "1"),
                screenshot,
                delay,
                taken: false,
            })
            .add_systems(Startup, spawn_hud)
            .add_systems(OnEnter(GameState::Menu), spawn_menu)
            .add_systems(OnExit(GameState::Menu), despawn::<MenuRoot>)
            .add_systems(OnEnter(GameState::GameOver), spawn_game_over)
            .add_systems(OnExit(GameState::GameOver), (despawn::<GameOverRoot>, despawn::<RoundScoped>))
            .add_systems(OnEnter(GameState::Playing), (grab_cursor(true), unpause))
            .add_systems(OnExit(GameState::Playing), grab_cursor(false))
            .add_systems(Update, (start_or_restart, toggle_pause, update_hud, test_hooks));
    }
}

/// Ends the current round and shows the result screen.
pub fn end_round(result: RoundResult, commands: &mut Commands) {
    commands.insert_resource(result);
    commands.set_state(GameState::GameOver);
}

/// Helper for building UI text with the kit's styling.
pub fn text(value: impl Into<String>, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (Text::new(value), TextFont { font_size: bevy::text::FontSize::Px(size), ..default() }, TextColor(color))
}

const PANEL: Color = Color::srgba(0.03, 0.04, 0.07, 0.82);
const MUTED: Color = Color::srgb(0.72, 0.76, 0.84);

fn despawn<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn overlay(commands: &mut Commands, marker: impl Bundle) -> Entity {
    commands
        .spawn((
            marker,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            GlobalZIndex(10),
        ))
        .id()
}

fn panel(commands: &mut Commands, parent: Entity) -> Entity {
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(10),
                padding: UiRect::axes(px(48), px(32)),
                border_radius: BorderRadius::all(px(14)),
                ..default()
            },
            BackgroundColor(PANEL),
            ChildOf(parent),
        ))
        .id()
}

fn spawn_menu(mut commands: Commands, info: Res<Info>, best: Res<BestScore>) {
    let root = overlay(&mut commands, MenuRoot);
    let card = panel(&mut commands, root);
    commands.spawn((text(info.0.title, 54.0, info.0.accent), ChildOf(card)));
    commands.spawn((text(info.0.tagline, 20.0, MUTED), ChildOf(card)));
    commands.spawn((Node { height: px(12), ..default() }, ChildOf(card)));
    for line in info.0.controls {
        commands.spawn((text(*line, 18.0, Color::WHITE), ChildOf(card)));
    }
    if let Some(best) = best.0 {
        commands.spawn((text(format!("Best: {best}"), 18.0, info.0.accent), ChildOf(card)));
    }
    commands.spawn((Node { height: px(12), ..default() }, ChildOf(card)));
    commands.spawn((text("Press Enter or click to play", 24.0, info.0.accent), ChildOf(card)));
}

fn spawn_game_over(mut commands: Commands, info: Res<Info>, result: Res<RoundResult>) {
    let root = overlay(&mut commands, GameOverRoot);
    let card = panel(&mut commands, root);
    let color = if result.won { Color::srgb(0.35, 0.95, 0.55) } else { Color::srgb(1.0, 0.42, 0.38) };
    commands.spawn((text(result.headline.clone(), 48.0, color), ChildOf(card)));
    for line in &result.details {
        commands.spawn((text(line.clone(), 20.0, Color::WHITE), ChildOf(card)));
    }
    commands.spawn((Node { height: px(12), ..default() }, ChildOf(card)));
    commands.spawn((text("Enter: play again    M: menu", 22.0, info.0.accent), ChildOf(card)));
}

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        HudText,
        Node {
            position_type: PositionType::Absolute,
            left: px(16),
            top: px(16),
            padding: UiRect::axes(px(14), px(10)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(PANEL),
        text("", 20.0, Color::WHITE),
        Visibility::Hidden,
    ));
    commands.spawn((
        BannerText,
        Node { position_type: PositionType::Absolute, top: percent(18), width: percent(100), justify_content: JustifyContent::Center, ..default() },
        TextLayout::justify(Justify::Center),
        text("", 44.0, Color::WHITE),
    ));
    commands.spawn((
        PausedText,
        Node { position_type: PositionType::Absolute, top: percent(45), width: percent(100), ..default() },
        TextLayout::justify(Justify::Center),
        text("Paused - Esc to resume", 36.0, Color::WHITE),
        Visibility::Hidden,
        GlobalZIndex(20),
    ));
}

fn update_hud(
    hud: Res<Hud>,
    state: Res<State<GameState>>,
    time: Res<Time<Virtual>>,
    mut texts: Query<(&mut Text, &mut Visibility), (With<HudText>, Without<BannerText>, Without<PausedText>)>,
    mut banners: Query<&mut Text, (With<BannerText>, Without<HudText>)>,
    mut paused: Query<&mut Visibility, (With<PausedText>, Without<HudText>)>,
) {
    let playing = *state.get() == GameState::Playing;
    for (mut text, mut visibility) in &mut texts {
        let joined = hud.lines.join("\n");
        if text.0 != joined {
            text.0 = joined;
        }
        visibility.set_if_neq(if playing && !hud.lines.is_empty() { Visibility::Inherited } else { Visibility::Hidden });
    }
    for mut text in &mut banners {
        let banner = if playing { hud.banner.clone().unwrap_or_default() } else { String::new() };
        if text.0 != banner {
            text.0 = banner;
        }
    }
    for mut visibility in &mut paused {
        visibility.set_if_neq(if playing && time.is_paused() { Visibility::Inherited } else { Visibility::Hidden });
    }
}

fn start_or_restart(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
) {
    let confirm = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter);
    match state.get() {
        GameState::Menu if confirm || mouse.just_pressed(MouseButton::Left) => next.set(GameState::Playing),
        GameState::GameOver if confirm => next.set(GameState::Playing),
        GameState::GameOver if keys.just_pressed(KeyCode::KeyM) => next.set(GameState::Menu),
        _ => {}
    }
}

fn toggle_pause(keys: Res<ButtonInput<KeyCode>>, state: Res<State<GameState>>, mut time: ResMut<Time<Virtual>>, info: Res<Info>, mut cursors: Query<&mut CursorOptions>) {
    if *state.get() != GameState::Playing || !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if time.is_paused() {
        time.unpause();
    } else {
        time.pause();
    }
    if info.0.grab_cursor {
        for mut cursor in &mut cursors {
            set_grab(&mut cursor, !time.is_paused());
        }
    }
}

fn unpause(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

fn set_grab(cursor: &mut CursorOptions, grab: bool) {
    cursor.grab_mode = if grab { CursorGrabMode::Locked } else { CursorGrabMode::None };
    cursor.visible = !grab;
}

fn grab_cursor(grab: bool) -> impl Fn(Res<Info>, Res<TestHooks>, Query<&mut CursorOptions>) {
    move |info: Res<Info>, hooks: Res<TestHooks>, mut cursors: Query<&mut CursorOptions>| {
        // Automated runs must not capture the developer's mouse.
        if !info.0.grab_cursor || hooks.autoplay {
            return;
        }
        for mut cursor in &mut cursors {
            set_grab(&mut cursor, grab);
        }
    }
}

fn test_hooks(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut hooks: ResMut<TestHooks>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    let elapsed = time.elapsed_secs();
    if hooks.autoplay && *state.get() == GameState::Menu && elapsed > 1.0 {
        next.set(GameState::Playing);
    }
    let Some(path) = hooks.screenshot.clone() else { return };
    if !hooks.taken && elapsed > hooks.delay {
        hooks.taken = true;
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
    if hooks.taken && elapsed > hooks.delay + 1.5 {
        exit.write(AppExit::Success);
    }
}

/// Standard window and plugin setup shared by the samples.
pub fn sample_app(title: &str) -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window { title: title.to_string(), resolution: (1280, 720).into(), ..default() }),
        ..default()
    }));
    app
}

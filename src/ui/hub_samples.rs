//! "Sample Games" section of the Project Hub: open a bundled game's level in the editor or
//! play it in its own window.
use bevy::{
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};

use super::hub::{ActiveProject, EditorAppState, ProjectRegistry};
use crate::project::sample_levels::{SAMPLE_GAMES, SampleGame};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

/// Label on a sample's Play button, updated while the game is launching.
#[derive(Component)]
struct PlayLabel(&'static str);

pub fn spawn_samples_section(commands: &mut Commands, hub_root: Entity) {
    let section = commands
        .spawn((
            EditorInternal,
            Node { width: percent(100.0), max_width: px(1120), flex_direction: FlexDirection::Column, row_gap: px(12), flex_shrink: 0.0, ..default() },
            ChildOf(hub_root),
        ))
        .id();
    let header = commands
        .spawn((EditorInternal, Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Baseline, column_gap: px(12), ..default() }, ChildOf(section)))
        .id();
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Sample Games"),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SECTION), ..default() },
        TextColor(TEXT_PRIMARY),
        ChildOf(header),
    ));
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Complete little games with menus, HUD and gameplay. Open one to edit its level, or play it."),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default() },
        TextColor(TEXT_MUTED),
        ChildOf(header),
    ));
    let grid = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100.0),
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                column_gap: px(8),
                row_gap: px(8),
                ..default()
            },
            ChildOf(section),
        ))
        .id();
    for sample in SAMPLE_GAMES {
        spawn_sample_card(commands, grid, sample);
    }
}

fn hover_colors(entity: &mut EntityCommands, normal: Color, hover: Color) {
    entity
        .observe(move |t: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(t.entity) { *bg = BackgroundColor(hover); }
        })
        .observe(move |t: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(t.entity) { *bg = BackgroundColor(normal); }
        });
}

fn spawn_sample_card(commands: &mut Commands, parent: Entity, sample: &'static SampleGame) {
    let mut card = commands.spawn((
        EditorInternal,
        Node {
            padding: UiRect::axes(px(14), px(12)),
            border: UiRect::all(px(1)),
            border_radius: radius_card(),
            column_gap: px(12),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(BG_CARD),
        BorderColor::all(BORDER_SUBTLE),
        ChildOf(parent),
    ));
    hover_colors(&mut card, BG_CARD, BG_HOVER);
    card.observe(move |click: On<PointerClick>, mut commands: Commands| {
        if click.button == PointerButton::Primary {
            commands.queue(move |world: &mut World| open_sample(world, sample));
        }
    });
    let card = card.id();

    let icon = commands
        .spawn((
            EditorInternal,
            Pickable::IGNORE,
            Node { width: px(44), height: px(44), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border_radius: radius_button(), flex_shrink: 0.0, ..default() },
            BackgroundColor(BG_INPUT),
            ChildOf(card),
        ))
        .id();
    commands.spawn((EditorInternal, Pickable::IGNORE, Text::new(sample.icon), TextFont { font_size: bevy::text::FontSize::Px(20.0), ..default() }, ChildOf(icon)));

    let text = commands
        .spawn((EditorInternal, Pickable::IGNORE, Node { flex_direction: FlexDirection::Column, row_gap: px(3), flex_grow: 1.0, min_width: px(0), ..default() }, ChildOf(card)))
        .id();
    let title_row = commands
        .spawn((EditorInternal, Pickable::IGNORE, Node { column_gap: px(8), align_items: AlignItems::Center, ..default() }, ChildOf(text)))
        .id();
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(sample.title),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL), weight: FontWeight::SEMIBOLD, ..default() },
        TextColor(TEXT_PRIMARY),
        ChildOf(title_row),
    ));
    let badge = commands
        .spawn((EditorInternal, Pickable::IGNORE, Node { padding: UiRect::axes(px(6), px(1)), border_radius: radius_pill(), ..default() }, BackgroundColor(ACCENT_CYAN_DIM), ChildOf(title_row)))
        .id();
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(sample.genre),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL - 1.0), ..default() },
        TextColor(ACCENT_CYAN),
        ChildOf(badge),
    ));
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(sample.description),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default() },
        TextColor(TEXT_MUTED),
        ChildOf(text),
    ));

    let buttons = commands
        .spawn((EditorInternal, Node { flex_direction: FlexDirection::Column, row_gap: px(6), flex_shrink: 0.0, ..default() }, ChildOf(card)))
        .id();
    let mut open = commands.spawn((
        EditorInternal,
        Node { padding: UiRect::axes(px(10), px(4)), border_radius: radius_button(), justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(ACCENT_CYAN_DIM),
        ChildOf(buttons),
    ));
    hover_colors(&mut open, ACCENT_CYAN_DIM, BG_HOVER);
    let open = open.id();
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Open in Editor ➜"),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default() },
        TextColor(ACCENT_CYAN),
        ChildOf(open),
    ));
    let mut play = commands.spawn((
        EditorInternal,
        Node { padding: UiRect::axes(px(10), px(4)), border_radius: radius_button(), justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(BG_INPUT),
        ChildOf(buttons),
    ));
    hover_colors(&mut play, BG_INPUT, BG_HOVER);
    play.observe(move |mut click: On<PointerClick>, mut labels: Query<(&PlayLabel, &mut Text)>| {
        if click.button != PointerButton::Primary {
            return;
        }
        click.propagate(false);
        let status = match sample.launch() {
            Ok(_) => {
                info!("Launching sample '{}' (cargo run -p {}); the first build compiles Bevy and can take a few minutes", sample.title, sample.crate_name);
                "Launching…"
            }
            Err(error) => {
                warn!("Could not start cargo for sample '{}': {error}", sample.title);
                "cargo not found"
            }
        };
        for (label, mut text) in &mut labels {
            if label.0 == sample.crate_name {
                text.0 = status.to_string();
            }
        }
    });
    let play = play.id();
    commands.spawn((
        EditorInternal,
        PlayLabel(sample.crate_name),
        Pickable::IGNORE,
        Text::new("▶ Play"),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default() },
        TextColor(ACCENT_EMERALD),
        ChildOf(play),
    ));
}

/// Opens a sample as the active project and loads its level into the editor.
pub fn open_sample(world: &mut World, sample: &'static SampleGame) {
    let dir = sample.dir();
    world.resource_mut::<ProjectRegistry>().add_or_update(&dir, sample.title, &sample.registry_label());
    world.resource_mut::<ActiveProject>().set_project_dir(dir);
    load_sample_level(world, sample);
    world.resource_mut::<NextState<EditorAppState>>().set(EditorAppState::InEditor);
}

/// Replaces the authoring scene with the sample's level.
pub fn load_sample_level(world: &mut World, sample: &'static SampleGame) {
    let existing: Vec<Entity> = world.query_filtered::<Entity, With<crate::scene::SceneEntity>>().iter(world).collect();
    for entity in existing {
        if let Ok(e) = world.get_entity_mut(entity) {
            e.despawn();
        }
    }
    world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
        world.resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
            let mut commands = world.commands();
            sample.spawn_level(&mut commands, &mut meshes, &mut materials);
        });
    });
    world.flush();
    if let Some(mut selection) = world.get_resource_mut::<bevy::inspector::InspectorSelection>() {
        selection.0 = None;
    }
    info!("Opened sample '{}' from {:?}", sample.title, sample.dir());
}

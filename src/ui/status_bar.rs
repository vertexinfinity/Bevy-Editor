//! Bottom status bar showing scene info, export feedback, and active object context.

use bevy::{inspector::InspectorSelection, prelude::*};
use crate::export::ExportStatus;
use crate::scene::EditorInternal;

#[derive(Component)]
pub struct StatusText;

#[derive(Component)]
pub struct ActiveContextChip;

pub fn spawn_status_bar(mut commands: Commands) {
    let bar_root = commands
        .spawn((
            EditorInternal,
            Name::new("BottomStatusBar"),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                right: px(0),
                height: px(28),
                border: UiRect::top(px(1)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(12), px(0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.12)),
            BorderColor::all(Color::srgb(0.2, 0.2, 0.22)),
        ))
        .id();

    // Left status text
    commands.spawn((
        EditorInternal,
        StatusText,
        Text::new("Ready | Fly Cam: WASD + Hold Right Mouse | Q/E: Up/Down"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.65, 0.65, 0.7)),
        ChildOf(bar_root),
    ));

    // Right container: Active context badge + Engine info
    let right_container = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                padding: UiRect::right(px(12)),
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    // Active Context Badge (bottom right)
    let chip = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(2)),
                border_radius: BorderRadius::all(px(3)),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.16, 0.18, 0.22)),
            BorderColor::all(Color::srgb(0.25, 0.3, 0.38)),
            ChildOf(right_container),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        ActiveContextChip,
        Text::new("No Selection"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.75, 1.0)),
        ChildOf(chip),
    ));

    // Right engine info text
    commands.spawn((
        EditorInternal,
        Text::new("Bevy 0.20-dev"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.45, 0.45, 0.5)),
        ChildOf(right_container),
    ));
}

pub fn update_status_bar(
    export_status: Res<ExportStatus>,
    selection: Res<InspectorSelection>,
    name_query: Query<Option<&Name>>,
    mut text_query: Query<&mut Text, (With<StatusText>, Without<ActiveContextChip>)>,
    mut chip_query: Query<&mut Text, (With<ActiveContextChip>, Without<StatusText>)>,
) {
    if export_status.is_changed() {
        if let Ok(mut text) = text_query.single_mut() {
            match export_status.as_ref() {
                ExportStatus::Idle => {
                    text.0 = "Ready | Fly Cam: WASD + Hold Right Mouse | Q/E: Up/Down".to_string();
                }
                ExportStatus::InProgress { target, progress } => {
                    text.0 = format!("Exporting {target}: {progress}");
                }
                ExportStatus::Success { target, message, .. } => {
                    text.0 = format!("Success [{target}]: {message}");
                }
                ExportStatus::Failed { target, error } => {
                    text.0 = format!("Error [{target}]: {error}");
                }
            }
        }
    }

    if selection.is_changed() {
        if let Ok(mut chip) = chip_query.single_mut() {
            if let Some(entity) = selection.0 {
                let name = name_query
                    .get(entity)
                    .ok()
                    .flatten()
                    .map(|n| n.as_str())
                    .unwrap_or("Object");
                chip.0 = format!("Active: {name} ({:?})", entity);
            } else {
                chip.0 = "No Selection".to_string();
            }
        }
    }
}

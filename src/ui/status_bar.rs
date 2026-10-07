//! Bottom status bar showing scene info, export feedback, and active object context.

use bevy::{inspector::InspectorSelection, prelude::*};
use crate::export::ExportStatus;
use crate::scene::EditorInternal;
use crate::ui::theme::*;

#[derive(Component)]
pub struct StatusText;

#[derive(Component)]
pub struct ActiveContextChip;

#[derive(Component)]
pub struct FpsCounterLabel;

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
            BackgroundColor(BG_SHELL),
            BorderColor::all(BORDER_SUBTLE),
        ))
        .id();

    // Left container: Status dot + helper text
    let left_container = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    // Cyan Status Indicator Dot
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Node {
            width: px(7),
            height: px(7),
            border_radius: radius_pill(),
            ..default()
        },
        BackgroundColor(ACCENT_CYAN),
        ChildOf(left_container),
    ));

    // Left status text
    commands.spawn((
        EditorInternal,
        StatusText,
        Text::new("Ready | Fly Cam: WASD + Hold RMB | Speed: Scroll Wheel"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(left_container),
    ));

    // Right container: FPS badge + Active context badge + Engine info
    let right_container = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                padding: UiRect::right(px(8)),
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    // FPS diagnostic chip
    let fps_chip = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border_radius: radius_button(),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(right_container),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        FpsCounterLabel,
        Text::new("60 FPS"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(fps_chip),
    ));

    // Active Context Badge (bottom right)
    let chip = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(2)),
                border_radius: radius_button(),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
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
        TextColor(ACCENT_CYAN),
        ChildOf(chip),
    ));

    // Right engine info text
    commands.spawn((
        EditorInternal,
        Text::new("Bevy 0.20-dev | Rust 2024"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(right_container),
    ));
}

pub fn update_status_bar(
    time: Res<Time>,
    export_status: Res<ExportStatus>,
    selection: Res<InspectorSelection>,
    name_query: Query<Option<&Name>>,
    mut text_query: Query<&mut Text, (With<StatusText>, Without<ActiveContextChip>, Without<FpsCounterLabel>)>,
    mut chip_query: Query<&mut Text, (With<ActiveContextChip>, Without<StatusText>, Without<FpsCounterLabel>)>,
    mut fps_query: Query<&mut Text, (With<FpsCounterLabel>, Without<StatusText>, Without<ActiveContextChip>)>,
) {
    // Smooth FPS calculation
    if let Ok(mut fps_text) = fps_query.single_mut() {
        let delta = time.delta_secs();
        if delta > 0.0001 {
            let fps = (1.0 / delta).round() as u32;
            fps_text.0 = format!("{fps} FPS");
        }
    }

    if export_status.is_changed() {
        if let Ok(mut text) = text_query.single_mut() {
            match export_status.as_ref() {
                ExportStatus::Idle => {
                    text.0 = "Ready | Fly Cam: WASD + Hold RMB | Speed: Scroll Wheel".to_string();
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
                chip.0 = format!("Active: {name} (#{entity})");
            } else {
                chip.0 = "No Selection".to_string();
            }
        }
    }
}

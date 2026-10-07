//! Bottom status bar showing scene info and export feedback.

use bevy::prelude::*;
use crate::export::ExportStatus;
use crate::scene::EditorInternal;

#[derive(Component)]
pub struct StatusText;

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

    // Right engine info text
    commands.spawn((
        EditorInternal,
        Text::new("Bevy 0.20-dev"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.5, 0.55)),
        ChildOf(bar_root),
    ));
}

pub fn update_status_bar(
    export_status: Res<ExportStatus>,
    mut query: Query<&mut Text, With<StatusText>>,
) {
    if !export_status.is_changed() {
        return;
    }

    if let Ok(mut text) = query.single_mut() {
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

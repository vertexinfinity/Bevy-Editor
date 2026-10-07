//! Project Explorer / Asset Browser panel for browsing and spawning assets.

use std::fs;
use std::path::PathBuf;
use bevy::{
    ecs::message::{Message, MessageReader, MessageWriter},
    gltf::GltfAssetLabel,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
    world_serialization::{DynamicWorldRoot, WorldAssetRoot},
};
use crate::export::ExportStatus;
use crate::scene::EditorInternal;

/// Marker component for an asset row in the Project Explorer.
#[derive(Component, Clone)]
pub struct AssetItem {
    pub file_path: PathBuf,
    pub file_name: String,
    pub is_dir: bool,
}

#[derive(Message, Clone)]
pub struct SpawnAssetMessage(pub AssetItem);

pub fn spawn_asset_browser(mut commands: Commands) {
    let browser_root = commands
        .spawn((
            EditorInternal,
            Name::new("ProjectExplorer"),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                bottom: px(28), // above bottom status bar
                width: px(340),
                height: px(260),
                border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.14)),
            BorderColor::all(Color::srgb(0.22, 0.22, 0.26)),
        ))
        .id();

    // Explorer Header
    let header = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(28),
                padding: UiRect::axes(px(8), px(4)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.16, 0.16, 0.18)),
            BorderColor::all(Color::srgb(0.22, 0.22, 0.26)),
            ChildOf(browser_root),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("📁 Project Explorer (assets/)"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
        ChildOf(header),
    ));

    // Scrollable File List Container
    let list_container = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(6)),
                row_gap: px(4),
                overflow: Overflow::clip_y(),
                ..default()
            },
            ChildOf(browser_root),
        ))
        .id();

    // Populate file list from assets/
    populate_asset_list(&mut commands, list_container);
}

fn populate_asset_list(commands: &mut Commands, parent: Entity) {
    let assets_dir = PathBuf::from("assets");
    let mut entries = Vec::new();

    if let Ok(read_dir) = fs::read_dir(&assets_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();
            entries.push((path, name, is_dir));
        }
    }

    if entries.is_empty() {
        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new("No files in assets/ - Drag .glb/.gltf here"),
            TextFont {
                font_size: bevy::text::FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::srgb(0.55, 0.55, 0.6)),
            ChildOf(parent),
        ));
        return;
    }

    for (path, name, is_dir) in entries {
        let icon = if is_dir {
            "📁"
        } else if name.ends_with(".glb") || name.ends_with(".gltf") {
            "📦"
        } else if name.ends_with(".ron") || name.ends_with(".bsn") {
            "🗺️"
        } else if name.ends_with(".png") || name.ends_with(".jpg") {
            "🖼️"
        } else {
            "📄"
        };

        let label = format!("{icon} {name}");
        let row = commands
            .spawn((
                EditorInternal,
                AssetItem {
                    file_path: path.clone(),
                    file_name: name.clone(),
                    is_dir,
                },
                Node {
                    padding: UiRect::axes(px(6), px(3)),
                    border_radius: BorderRadius::all(px(3)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                ChildOf(parent),
            ))
            .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(Color::srgb(0.26, 0.32, 0.44));
                }
            })
            .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(Color::srgb(0.18, 0.18, 0.22));
                }
            })
            .observe(|click: On<PointerClick>, query: Query<&AssetItem>, mut writer: MessageWriter<SpawnAssetMessage>| {
                if click.button == PointerButton::Primary {
                    if let Ok(item) = query.get(click.entity) {
                        writer.write(SpawnAssetMessage(item.clone()));
                    }
                }
            })
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(label),
            TextFont {
                font_size: bevy::text::FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.85)),
            ChildOf(row),
        ));
    }
}

/// Handles spawn messages when an asset item is clicked.
pub fn handle_spawn_asset_messages(
    mut messages: MessageReader<SpawnAssetMessage>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut export_status: ResMut<ExportStatus>,
) {
    for msg in messages.read() {
        let item = &msg.0;
        if item.is_dir {
            continue;
        }

        let ext = item.file_path.extension().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
        let path_str = item.file_path.to_string_lossy().to_string();

        match ext.as_str() {
            "glb" | "gltf" => {
                commands.spawn((
                    crate::scene::SceneEntity,
                    Name::new(item.file_name.clone()),
                    WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(path_str))),
                    Transform::IDENTITY,
                ));
                *export_status = ExportStatus::InProgress {
                    target: "Explorer".to_string(),
                    progress: format!("Spawned model: {}", item.file_name),
                };
            }
            "ron" | "bsn" => {
                commands.spawn((
                    crate::scene::SceneEntity,
                    Name::new(format!("Scene: {}", item.file_name)),
                    DynamicWorldRoot(asset_server.load(path_str)),
                    Transform::IDENTITY,
                ));
                *export_status = ExportStatus::InProgress {
                    target: "Explorer".to_string(),
                    progress: format!("Loaded scene: {}", item.file_name),
                };
            }
            _ => {}
        }
    }
}

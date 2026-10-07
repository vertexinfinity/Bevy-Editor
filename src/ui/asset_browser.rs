//! Project Explorer / Asset Browser dockable panel.

use std::fs;
use std::path::{Path, PathBuf};
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

/// Marker component for the Project Explorer root dock container.
#[derive(Component)]
pub struct ProjectExplorerDockRoot;

/// Marker component for the container listing all files.
#[derive(Component)]
pub struct AssetListContainer;

/// Marker component for the collapse button text.
#[derive(Component)]
pub struct ExplorerCollapseLabel;

/// Tracks hovered directory target for drag-and-drop file copy operations.
#[derive(Resource, Debug, Clone)]
pub struct HoveredAssetDirectory(pub PathBuf);

impl Default for HoveredAssetDirectory {
    fn default() -> Self {
        Self(PathBuf::from("assets/models"))
    }
}

/// Message to toggle Project Explorer dock collapse state.
#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleExplorerDockMessage;

/// Message to request spawning an asset into the 3D scene.
#[derive(Message, Clone)]
pub struct SpawnAssetMessage(pub AssetItem);

/// Message to request refreshing the file list.
#[derive(Message, Clone, Copy, Debug)]
pub struct RefreshAssetBrowserMessage;

/// State for the Project Explorer dock.
#[derive(Resource, Debug, Clone)]
pub struct AssetBrowserState {
    pub is_collapsed: bool,
    pub current_dir: PathBuf,
}

impl Default for AssetBrowserState {
    fn default() -> Self {
        Self {
            is_collapsed: false,
            current_dir: PathBuf::from("assets"),
        }
    }
}

pub fn spawn_asset_browser(mut commands: Commands) {
    let browser_root = commands
        .spawn((
            EditorInternal,
            ProjectExplorerDockRoot,
            Name::new("ProjectExplorerDock"),
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                bottom: px(36), // above bottom status bar
                width: px(340),
                max_width: percent(45.0),
                height: px(260),
                max_height: percent(50.0),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.15)),
            BorderColor::all(Color::srgb(0.24, 0.24, 0.28)),
        ))
        .id();

    // Explorer Header (Draggable / Dockable styling)
    let header = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(30),
                padding: UiRect::axes(px(8), px(4)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.16, 0.16, 0.20)),
            BorderColor::all(Color::srgb(0.24, 0.24, 0.28)),
            ChildOf(browser_root),
        ))
        .id();

    let header_left = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            ChildOf(header),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("📁 Project Explorer"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.95)),
        ChildOf(header_left),
    ));

    // Header Right controls (Refresh & Dock Collapse)
    let header_right = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
            ChildOf(header),
        ))
        .id();

    // Refresh Button
    let refresh_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.38)),
            ChildOf(header_right),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.28, 0.34, 0.46));
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.2, 0.2, 0.25));
            }
        })
        .observe(|click: On<PointerClick>, mut writer: MessageWriter<RefreshAssetBrowserMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(RefreshAssetBrowserMessage);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("🔄"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
        ChildOf(refresh_btn),
    ));

    // Collapse / Dock Toggle Button
    let collapse_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.38)),
            ChildOf(header_right),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.28, 0.34, 0.46));
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.2, 0.2, 0.25));
            }
        })
        .observe(|click: On<PointerClick>, mut writer: MessageWriter<ToggleExplorerDockMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(ToggleExplorerDockMessage);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        ExplorerCollapseLabel,
        Pickable::IGNORE,
        Text::new("▼"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
        ChildOf(collapse_btn),
    ));

    // Scrollable File List Container
    let list_container = commands
        .spawn((
            EditorInternal,
            AssetListContainer,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(6)),
                row_gap: px(4),
                overflow: Overflow::clip_y(),
                flex_grow: 1.0,
                ..default()
            },
            ChildOf(browser_root),
        ))
        .id();

    // Populate initial file list from assets/
    populate_asset_list(&mut commands, list_container, Path::new("assets"));
}

fn populate_asset_list(commands: &mut Commands, parent: Entity, current_dir: &Path) {
    let mut entries = Vec::new();

    if let Ok(read_dir) = fs::read_dir(current_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();
            entries.push((path, name, is_dir));
        }
    }

    // Sort: directories first, then alphabetical
    entries.sort_by(|a, b| match (a.2, b.2) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
    });

    if entries.is_empty() {
        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new("No files in assets/ - Drag .glb/.gltf here to import"),
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
        let row_path = path.clone();
        let hover_dir_path = if is_dir { path.clone() } else { PathBuf::from("assets/models") };

        let row = commands
            .spawn((
                EditorInternal,
                AssetItem {
                    file_path: row_path,
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
            .observe(move |trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>, mut hovered_dir: ResMut<HoveredAssetDirectory>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(Color::srgb(0.26, 0.32, 0.44));
                }
                hovered_dir.0 = hover_dir_path.clone();
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

/// Handles toggling Project Explorer dock collapse and refresh requests.
pub fn handle_explorer_dock_and_refresh(
    mut toggle_msgs: MessageReader<ToggleExplorerDockMessage>,
    mut refresh_msgs: MessageReader<RefreshAssetBrowserMessage>,
    mut state: ResMut<AssetBrowserState>,
    mut dock_query: Query<&mut Node, With<ProjectExplorerDockRoot>>,
    mut list_query: Query<&mut Node, (With<AssetListContainer>, Without<ProjectExplorerDockRoot>)>,
    mut collapse_label_query: Query<&mut Text, With<ExplorerCollapseLabel>>,
    list_container_query: Query<Entity, With<AssetListContainer>>,
    children_query: Query<&Children>,
    mut commands: Commands,
) {
    let mut should_refresh = false;

    for _ in toggle_msgs.read() {
        state.is_collapsed = !state.is_collapsed;
    }

    for _ in refresh_msgs.read() {
        should_refresh = true;
    }

    if state.is_changed() {
        if let Ok(mut dock_node) = dock_query.single_mut() {
            if state.is_collapsed {
                dock_node.height = px(30);
            } else {
                dock_node.height = px(260);
            }
        }

        if let Ok(mut list_node) = list_query.single_mut() {
            if state.is_collapsed {
                list_node.display = Display::None;
            } else {
                list_node.display = Display::Flex;
            }
        }

        if let Ok(mut label) = collapse_label_query.single_mut() {
            label.0 = if state.is_collapsed { "▲".to_string() } else { "▼".to_string() };
        }
    }

    if should_refresh {
        if let Ok(list_entity) = list_container_query.single() {
            // Despawn old child rows
            if let Ok(children) = children_query.get(list_entity) {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }
            populate_asset_list(&mut commands, list_entity, &state.current_dir);
        }
    }
}

/// Handles spawn messages when an asset item is clicked in the Project Explorer.
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

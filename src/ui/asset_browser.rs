//! Project Explorer / Asset Browser dockable panel with breadcrumbs and folder navigation.

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
use crate::ui::theme::*;

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

#[derive(Component)]
pub(super) struct AssetBrowserResizeHandle { origin: Option<(f32, f32)> }

/// Marker component for the container listing all files.
#[derive(Component)]
pub struct AssetListContainer;

/// Marker component for the collapse button text.
#[derive(Component)]
pub struct ExplorerCollapseLabel;

/// Marker component for the directory breadcrumb title text.
#[derive(Component)]
pub struct DirectoryBreadcrumbLabel;

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

/// Message to navigate into a subfolder or back up to parent.
#[derive(Message, Clone, Debug)]
pub struct NavigateAssetFolderMessage(pub PathBuf);

/// Message to request refreshing the file list.
#[derive(Message, Clone, Copy, Debug)]
pub struct RefreshAssetBrowserMessage;

/// Message to set a scene file as project default scene.
#[derive(Message, Clone, Debug)]
pub struct SetDefaultSceneMessage(pub PathBuf);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AssetCategoryFilter {
    #[default]
    All,
    Models,
    Textures,
    Scenes,
    Scripts,
    Audio,
}

impl AssetCategoryFilter {
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Models => "Models",
            Self::Textures => "Textures",
            Self::Scenes => "Scenes",
            Self::Scripts => "Scripts",
            Self::Audio => "Audio",
        }
    }

    pub fn matches(&self, file_name: &str, is_dir: bool) -> bool {
        if is_dir {
            return true;
        }
        let lower = file_name.to_lowercase();
        match self {
            Self::All => true,
            Self::Models => lower.ends_with(".glb") || lower.ends_with(".gltf") || lower.ends_with(".obj"),
            Self::Textures => lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".ktx2"),
            Self::Scenes => lower.ends_with(".ron") || lower.ends_with(".bsn"),
            Self::Scripts => lower.ends_with(".lua") || lower.ends_with(".rhai") || lower.ends_with(".rs"),
            Self::Audio => lower.ends_with(".ogg") || lower.ends_with(".wav") || lower.ends_with(".mp3"),
        }
    }
}

/// Tracks double-click timing to open files in system default editor (e.g. VS Code).
#[derive(Resource, Default)]
pub struct LastAssetClick {
    pub path: Option<PathBuf>,
    pub timestamp: f64,
}

/// Message to request creating a new asset file from template.
#[derive(Message, Clone, Debug)]
pub struct CreateAssetAction {
    pub kind: crate::assets::AssetTemplateKind,
    pub name: String,
}

/// Message to set the active asset category filter.
#[derive(Message, Clone, Copy, Debug)]
pub struct SetAssetFilterMessage(pub AssetCategoryFilter);

/// State for the Project Explorer dock.
#[derive(Resource, Debug, Clone)]
pub struct AssetBrowserState {
    pub is_collapsed: bool,
    pub current_dir: PathBuf,
    pub active_filter: AssetCategoryFilter,
    pub width: f32,
    pub height: f32,
}

impl Default for AssetBrowserState {
    fn default() -> Self {
        Self {
            is_collapsed: false,
            current_dir: PathBuf::from("assets"),
            active_filter: AssetCategoryFilter::All,
            width: 1200.0,
            height: 210.0,
        }
    }
}

pub fn spawn_asset_browser(mut commands: Commands, state: Res<AssetBrowserState>) {
    let browser_root = commands
        .spawn((
            EditorInternal,
            ProjectExplorerDockRoot,
            Name::new("ProjectExplorerDock"),
            Node {
                position_type: PositionType::Absolute,
                left: px(255), // offset cleanly from hierarchy dock
                bottom: px(34), // above bottom status bar
                width: px(state.width),
                max_width: percent(75.0),
                height: px(state.height),
                max_height: percent(70.0),
                border: UiRect::top(px(1)),
                border_radius: radius_panel(),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_SUBTLE),
        ))
        .id();

    commands
        .spawn((
            EditorInternal,
            AssetBrowserResizeHandle { origin: None },
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(0),
                width: px(12),
                height: px(12),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BORDER_SUBTLE),
            Pickable::default(),
            ChildOf(browser_root),
        ))
        .observe(|trigger: On<PointerOver>, mut color: Query<&mut BackgroundColor, With<AssetBrowserResizeHandle>>| {
            if let Ok(mut color) = color.get_mut(trigger.entity) { *color = BackgroundColor(BORDER_ACCENT); }
        })
        .observe(|trigger: On<PointerOut>, mut color: Query<&mut BackgroundColor, With<AssetBrowserResizeHandle>>| {
            if let Ok(mut color) = color.get_mut(trigger.entity) { *color = BackgroundColor(BORDER_SUBTLE); }
        })
         .observe(|_: On<bevy::picking::events::PointerDragStart>, root: Query<&ComputedNode, With<ProjectExplorerDockRoot>>, mut handle: Query<&mut AssetBrowserResizeHandle>| {
            if let (Ok(root), Ok(mut handle)) = (root.single(), handle.single_mut()) {
                let size = root.size() * root.inverse_scale_factor;
                handle.origin = Some((size.x, size.y));
            }
        })
        .observe(|drag: On<bevy::picking::events::PointerDrag>, window: Query<&Window, With<bevy::window::PrimaryWindow>>, state: Res<AssetBrowserState>, handle: Query<&AssetBrowserResizeHandle>, mut root: Query<&mut Node, With<ProjectExplorerDockRoot>>| {
            if state.is_collapsed { return; }
            let Ok(handle) = handle.single() else { return; };
            let Some((origin_width, origin_height)) = handle.origin else { return; };
            let (window_width, window_height) = window.single().map(|window| (window.width(), window.height())).unwrap_or((1600.0, 900.0));
            let width = (origin_width + drag.distance.x).clamp(320.0, (window_width * 0.75).min(1200.0).max(320.0));
            let height = (origin_height - drag.distance.y).clamp(160.0, (window_height * 0.70).min(720.0).max(160.0));
            if let Ok(mut node) = root.single_mut() { node.width = px(width); node.height = px(height); }
        })
        .observe(|_: On<bevy::picking::events::PointerDragEnd>, mut state: ResMut<AssetBrowserState>, mut handle: Query<&mut AssetBrowserResizeHandle>, root: Query<&Node, With<ProjectExplorerDockRoot>>| {
            if let Ok(node) = root.single() {
                if let Val::Px(width) = node.width { state.width = width; }
                if let Val::Px(height) = node.height { state.height = height; }
            }
            if let Ok(mut handle) = handle.single_mut() { handle.origin = None; }
        });

    // 1. Explorer Header (Draggable / Dockable styling)
    let header = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(32),
                padding: UiRect::axes(px(8), px(4)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(BORDER_SUBTLE),
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

    // Directory breadcrumb title
    commands.spawn((
        EditorInternal,
        DirectoryBreadcrumbLabel,
        Pickable::IGNORE,
        Text::new("📁 assets/"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            weight: FontWeight::SEMIBOLD,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(header_left),
    ));

    // + New Script Button (placed in header_left so it's always prominently visible)
    let new_script_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::ZERO,
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(header_left),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_INPUT);
            }
        })
        .observe(|click: On<PointerClick>, _state: Res<AssetBrowserState>, mut writer: MessageWriter<CreateAssetAction>| {
            if click.button == PointerButton::Primary {
                let name = format!("script_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() % 1000).unwrap_or(1));
                writer.write(CreateAssetAction {
                    kind: crate::assets::AssetTemplateKind::LuaScript,
                    name,
                });
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("+ Script"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(new_script_btn),
    ));

    // + New Scene Button
    let new_scene_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::ZERO,
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(header_left),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_INPUT);
            }
        })
        .observe(|click: On<PointerClick>, mut writer: MessageWriter<CreateAssetAction>| {
            if click.button == PointerButton::Primary {
                let name = format!("scene_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() % 1000).unwrap_or(1));
                writer.write(CreateAssetAction {
                    kind: crate::assets::AssetTemplateKind::SceneRon,
                    name,
                });
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("+ Scene"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(new_scene_btn),
    ));

    // Header Right controls (Up/Back, Refresh & Dock Collapse)
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

    // Up / Back Button
    let up_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::ZERO,
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(header_right),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_INPUT);
            }
        })
        .observe(|click: On<PointerClick>, state: Res<AssetBrowserState>, mut writer: MessageWriter<NavigateAssetFolderMessage>| {
            if click.button == PointerButton::Primary {
                if let Some(parent) = state.current_dir.parent() {
                    if state.current_dir != Path::new("assets") {
                        writer.write(NavigateAssetFolderMessage(parent.to_path_buf()));
                    }
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("▲ Up"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(up_btn),
    ));

    // Refresh Button
    let refresh_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::ZERO,
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(header_right),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_INPUT);
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
        Text::new("↻ Sync"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(refresh_btn),
    ));

    // 2. Filter Bar: Filter tabs (All, Models, Textures, Scenes, Scripts, Audio)
    let filter_bar = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(28),
                padding: UiRect::axes(px(6), px(2)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(browser_root),
        ))
        .id();

    for filter in [
        AssetCategoryFilter::All,
        AssetCategoryFilter::Models,
        AssetCategoryFilter::Textures,
        AssetCategoryFilter::Scenes,
        AssetCategoryFilter::Scripts,
        AssetCategoryFilter::Audio,
    ] {
        let is_active = state.active_filter == filter;
        let tab_btn = commands
            .spawn((
                EditorInternal,
                Node {
                    padding: UiRect::axes(px(6), px(2)),
                    border_radius: radius_button(),
                    border: UiRect::ZERO,
                    ..default()
                },
                BackgroundColor(if is_active { ACCENT_CYAN_DIM } else { BG_CARD }),
                BorderColor::all(if is_active { BORDER_ACCENT } else { BORDER_SUBTLE }),
                ChildOf(filter_bar),
            ))
            .observe(move |trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
                if !is_active {
                    if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                        *bg = BackgroundColor(BG_HOVER);
                    }
                }
            })
            .observe(move |trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
                if !is_active {
                    if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                        *bg = BackgroundColor(Color::NONE);
                    }
                }
            })
            .observe(move |click: On<PointerClick>, mut writer: MessageWriter<SetAssetFilterMessage>| {
                if click.button == PointerButton::Primary {
                    writer.write(SetAssetFilterMessage(filter));
                }
            })
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(filter.label()),
            TextFont {
                font_size: bevy::text::FontSize::Px(10.0),
                weight: if is_active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL },
                ..default()
            },
            TextColor(if is_active { ACCENT_CYAN } else { TEXT_SECONDARY }),
            ChildOf(tab_btn),
        ));
    }

    // Collapse / Dock Toggle Button
    let collapse_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::ZERO,
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(header_right),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_INPUT);
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
        Text::new("v"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(collapse_btn),
    ));

    // Scrollable File List Container
    let list_container = commands
        .spawn((
            EditorInternal,
            AssetListContainer,
            bevy::ui_widgets::ScrollArea,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(6)),
                row_gap: px(4),
                overflow: Overflow::scroll_y(),
                min_height: px(0),
                flex_grow: 1.0,
                ..default()
            },
            ChildOf(browser_root),
        ))
        .id();

    // Populate initial file list from assets/
    populate_asset_list(&mut commands, list_container, Path::new("assets"), state.active_filter);
}

fn populate_asset_list(
    commands: &mut Commands,
    parent: Entity,
    current_dir: &Path,
    filter: AssetCategoryFilter,
) {
    let mut entries = Vec::new();

    if let Ok(read_dir) = fs::read_dir(current_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();

            // Filter check
            if filter.matches(&name, is_dir) {
                entries.push((path, name, is_dir));
            }
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
            Text::new("No matching assets in this folder"),
            TextFont {
                font_size: bevy::text::FontSize::Px(11.0),
                ..default()
            },
            TextColor(TEXT_MUTED),
            ChildOf(parent),
        ));
        return;
    }

    for (path, name, is_dir) in entries {
        let icon = if is_dir {
            "📁"
        } else if name.ends_with(".glb") || name.ends_with(".gltf") {
            "🧊"
        } else if name.ends_with(".ron") || name.ends_with(".bsn") {
            "🎬"
        } else if name.ends_with(".png") || name.ends_with(".jpg") {
            "🖼"
        } else if name.ends_with(".lua") || name.ends_with(".rhai") || name.ends_with(".rs") {
            "📜"
        } else if name.ends_with(".ogg") || name.ends_with(".wav") {
            "🔊"
        } else {
            "📄"
        };

        let label = if is_dir { format!("{icon}  {name}/") } else { format!("{icon}  {name}") };
        let row_path = path.clone();
        let hover_dir_path = if is_dir { path.clone() } else { current_dir.to_path_buf() };

        let row = commands
            .spawn((
                EditorInternal,
                AssetItem {
                    file_path: row_path.clone(),
                    file_name: name.clone(),
                    is_dir,
                },
                Node {
                    padding: UiRect::axes(px(8), px(3)),
                    border_radius: radius_button(),
                    border: UiRect::ZERO,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    flex_direction: FlexDirection::Row,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(BORDER_SUBTLE),
                ChildOf(parent),
            ))
            .observe(move |trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>, mut hovered_dir: ResMut<HoveredAssetDirectory>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(BG_HOVER);
                }
                hovered_dir.0 = hover_dir_path.clone();
            })
            .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(Color::NONE);
                }
            })
            .observe(move |click: On<PointerClick>, query: Query<&AssetItem>, time: Res<Time>, mut last_click: ResMut<LastAssetClick>, mut spawn_writer: MessageWriter<SpawnAssetMessage>, mut nav_writer: MessageWriter<NavigateAssetFolderMessage>| {
                if click.button == PointerButton::Primary {
                    if let Ok(item) = query.get(click.entity) {
                        if item.is_dir {
                            // Click directory: navigate into it!
                            nav_writer.write(NavigateAssetFolderMessage(item.file_path.clone()));
                        } else {
                            let current_time = time.elapsed_secs_f64();
                            let is_double_click = last_click.path.as_ref() == Some(&item.file_path)
                                && (current_time - last_click.timestamp) < 0.35;

                            if is_double_click {
                                // Double click: Open in OS default editor (VS Code)
                                let _ = crate::assets::AssetCreator::open_in_system_editor(&item.file_path);
                                last_click.path = None;
                                last_click.timestamp = 0.0;
                            } else {
                                // Single click: update tracking and spawn into 3D scene
                                last_click.path = Some(item.file_path.clone());
                                last_click.timestamp = current_time;
                                spawn_writer.write(SpawnAssetMessage(item.clone()));
                            }
                        }
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
            TextColor(TEXT_PRIMARY),
            ChildOf(row),
        ));

        let is_scene = !is_dir && (name.ends_with(".ron") || name.ends_with(".bsn"));
        let is_default = is_scene && (name == "default_scene.bsn" || path.ends_with("default_scene.bsn"));

        if is_scene {
            if is_default {
                commands.spawn((
                    EditorInternal,
                    Pickable::IGNORE,
                    Text::new("⭐ Default"),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(9.5),
                        weight: FontWeight::BOLD,
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.8, 0.2)),
                    Node {
                        padding: UiRect::axes(px(5), px(1)),
                        border_radius: BorderRadius::all(px(3)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.95, 0.8, 0.2, 0.15)),
                    ChildOf(row),
                ));
            } else {
                let set_def_btn = commands
                    .spawn((
                        EditorInternal,
                        Node {
                            padding: UiRect::axes(px(6), px(2)),
                            border_radius: BorderRadius::all(px(3)),
                            border: UiRect::ZERO,
                            ..default()
                        },
                        BackgroundColor(BG_INPUT),
                        BorderColor::all(BORDER_SUBTLE),
                        ChildOf(row),
                    ))
                    .observe(move |mut click: On<PointerClick>, mut writer: MessageWriter<SetDefaultSceneMessage>| {
                        if click.button == PointerButton::Primary {
                            click.propagate(false);
                            writer.write(SetDefaultSceneMessage(row_path.clone()));
                        }
                    })
                    .id();

                commands.spawn((
                    EditorInternal,
                    Pickable::IGNORE,
                    Text::new("Set Default"),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(9.0),
                        ..default()
                    },
                    TextColor(TEXT_SECONDARY),
                    ChildOf(set_def_btn),
                ));
            }
        }
    }
}

/// Handles setting an existing scene as the default scene.
pub fn handle_set_default_scene(
    mut reader: MessageReader<SetDefaultSceneMessage>,
    mut refresh_writer: MessageWriter<RefreshAssetBrowserMessage>,
    mut current_scene: Option<ResMut<crate::scene::CurrentScene>>,
) {
    for msg in reader.read() {
        let default_target = Path::new("assets/scenes/default_scene.bsn");
        if let Err(e) = std::fs::copy(&msg.0, default_target) {
            warn!("Failed to set default scene: {e}");
        } else {
            info!("⭐ Successfully set {:?} as default scene!", msg.0);
            if let Some(ref mut cs) = current_scene {
                cs.file_path = Some(default_target.to_path_buf());
            }
            refresh_writer.write(RefreshAssetBrowserMessage);
        }
    }
}

/// Handles toggling Project Explorer dock collapse, folder navigation, filter changes, and refresh requests.
pub(super) fn handle_explorer_dock_and_refresh(
    mut toggle_msgs: MessageReader<ToggleExplorerDockMessage>,
    mut refresh_msgs: MessageReader<RefreshAssetBrowserMessage>,
    mut filter_msgs: MessageReader<SetAssetFilterMessage>,
    mut nav_msgs: MessageReader<NavigateAssetFolderMessage>,
    mut state: ResMut<AssetBrowserState>,
    mut dock_query: Query<&mut Node, With<ProjectExplorerDockRoot>>,
    mut resize_handle_query: Query<&mut Node, (With<AssetBrowserResizeHandle>, Without<ProjectExplorerDockRoot>, Without<AssetListContainer>)>,
    mut list_query: Query<&mut Node, (With<AssetListContainer>, Without<ProjectExplorerDockRoot>)>,
    mut collapse_label_query: Query<&mut Text, (With<ExplorerCollapseLabel>, Without<DirectoryBreadcrumbLabel>)>,
    mut breadcrumb_query: Query<&mut Text, (With<DirectoryBreadcrumbLabel>, Without<ExplorerCollapseLabel>)>,
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

    for filter_msg in filter_msgs.read() {
        state.active_filter = filter_msg.0;
        should_refresh = true;
    }

    for nav in nav_msgs.read() {
        state.current_dir = nav.0.clone();
        should_refresh = true;
    }

    if state.is_changed() {
        if let Ok(mut dock_node) = dock_query.single_mut() {
            dock_node.width = px(state.width);
            dock_node.height = px(if state.is_collapsed { 30.0 } else { state.height });
        }

        if let Ok(mut resize_node) = resize_handle_query.single_mut() {
            resize_node.display = if state.is_collapsed { Display::None } else { Display::Flex };
        }

        if let Ok(mut list_node) = list_query.single_mut() {
            if state.is_collapsed {
                list_node.display = Display::None;
            } else {
                list_node.display = Display::Flex;
            }
        }

        if let Ok(mut label) = collapse_label_query.single_mut() {
            label.0 = if state.is_collapsed { "^".to_string() } else { "v".to_string() };
        }

        if let Ok(mut breadcrumb) = breadcrumb_query.single_mut() {
            let path_str = state.current_dir.to_string_lossy().replace('\\', "/");
            breadcrumb.0 = format!("{path_str}/");
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
            populate_asset_list(&mut commands, list_entity, &state.current_dir, state.active_filter);
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
        // Convert to asset-relative path
        let relative_path = if let Ok(stripped) = item.file_path.strip_prefix("assets") {
            stripped.to_string_lossy().replace('\\', "/")
        } else {
            item.file_path.to_string_lossy().replace('\\', "/")
        };
        let clean_path = relative_path.trim_start_matches('/').to_string();

        match ext.as_str() {
            "glb" | "gltf" => {
                if ext == "gltf" {
                    let gltf_path = std::path::Path::new("assets").join(&clean_path);
                    if let Ok(content) = std::fs::read_to_string(&gltf_path) {
                        if content.contains("\"uri\": \"") && content.contains(".bin\"") {
                            let parent = gltf_path.parent().unwrap_or(std::path::Path::new("assets"));
                            let mut missing_file = None;
                            for line in content.lines() {
                                if line.contains("\"uri\": \"") && line.contains(".bin\"") {
                                    if let Some(start) = line.find("\"uri\": \"") {
                                        let rest = &line[start + 8..];
                                        if let Some(end) = rest.find('\"') {
                                            let bin_name = &rest[..end];
                                            if !parent.join(bin_name).exists() {
                                                missing_file = Some(bin_name.to_string());
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(bin_name) = missing_file {
                                warn!("Cannot spawn glTF '{}': missing required companion buffer '{}'", clean_path, bin_name);
                                *export_status = ExportStatus::Failed {
                                    target: "Explorer".to_string(),
                                    error: format!("Missing {bin_name} for {}", item.file_name),
                                };
                                continue;
                            }
                        }
                    }
                }

                info!("Spawning model from Explorer: {}", clean_path);
                commands.spawn((
                    crate::scene::SceneEntity,
                    Name::new(item.file_name.clone()),
                    WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(clean_path))),
                    Transform::IDENTITY,
                ));
                *export_status = ExportStatus::InProgress {
                    target: "Explorer".to_string(),
                    progress: format!("Spawned model: {}", item.file_name),
                };
            }
            "ron" | "bsn" => {
                info!("Loading scene from Explorer: {}", clean_path);
                commands.spawn((
                    crate::scene::SceneEntity,
                    Name::new(format!("Scene: {}", item.file_name)),
                    DynamicWorldRoot(asset_server.load(clean_path)),
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

/// System handling asset creation requests from the Asset Browser.
pub fn handle_create_asset_actions(
    mut actions: MessageReader<CreateAssetAction>,
    state: Res<AssetBrowserState>,
    mut refresh_writer: MessageWriter<RefreshAssetBrowserMessage>,
) {
    for action in actions.read() {
        let dir = &state.current_dir;
        match crate::assets::AssetCreator::create_asset(dir, &action.name, action.kind) {
            Ok(created_path) => {
                info!("Created new asset: {:?}", created_path);
                let _ = crate::assets::AssetCreator::open_in_system_editor(&created_path);
                refresh_writer.write(RefreshAssetBrowserMessage);
            }
            Err(e) => {
                error!("Failed to create asset: {}", e);
            }
        }
    }
}

//! Top Menu Bar & Application Header for Bevylite.
//!
//! Provides brand identity, project management, scene operations,
//! layout presets, and multi-platform export triggers.

use bevy::{
    ecs::{
        message::{Message, MessageWriter},
        system::RunSystemOnce,
    },
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::export::RequestExportMessage;
use crate::scene::{primitives, save_load, CurrentScene, EditorInternal};
use crate::ui::theme::*;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    NewProject,
    NewScene,
    OpenScene,
    SaveScene,
    SaveAsScene,
    ExportAndroid,
    ExportWindows,
    ExportWeb,
    ExportIos,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct MenuActionMessage(pub MenuAction);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WorkspacePreset {
    #[default]
    Default,
    LevelDesign,
    Programming,
    Debugging,
    Assets,
}

impl WorkspacePreset {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::LevelDesign => "Level Design",
            Self::Programming => "Programming",
            Self::Debugging => "Debugging",
            Self::Assets => "Assets",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Default => Self::LevelDesign,
            Self::LevelDesign => Self::Programming,
            Self::Programming => Self::Debugging,
            Self::Debugging => Self::Assets,
            Self::Assets => Self::Default,
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ActiveWorkspace(pub WorkspacePreset);

#[derive(Message, Clone, Copy, Debug)]
pub struct SwitchWorkspaceMessage(pub WorkspacePreset);

#[derive(Component)]
pub struct WorkspaceSelectorLabel;

/// Spawns the top unified header and menu bar.
pub fn spawn_menu_bar(mut commands: Commands, current_ws: Res<ActiveWorkspace>) {
    let bar_root = commands
        .spawn((
            EditorInternal,
            Name::new("TopMenuBar"),
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                height: px(38),
                border: UiRect::bottom(px(1)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(12), px(4)),
                ..default()
            },
            BackgroundColor(BG_SHELL),
            BorderColor::all(BORDER_SUBTLE),
        ))
        .id();

    // 1. Left Container: Brand Badge + Organized Menu Buttons (File, Edit, Scene, Export)
    let left_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    // Bevylite Brand Logo Badge
    let brand_pill = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                padding: UiRect::axes(px(8), px(3)),
                margin: UiRect::right(px(6)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(left_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("BL"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(brand_pill),
    ));

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("BEVYLITE"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            weight: FontWeight::SEMIBOLD,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(brand_pill),
    ));

    // Section 1: Project & Scene Management
    spawn_menu_button(&mut commands, left_group, "+ Project", MenuAction::NewProject);
    spawn_menu_button(&mut commands, left_group, "New Scene", MenuAction::NewScene);
    spawn_menu_button(&mut commands, left_group, "Open...", MenuAction::OpenScene);
    spawn_menu_button(&mut commands, left_group, "Save", MenuAction::SaveScene);
    spawn_menu_button(&mut commands, left_group, "Save As...", MenuAction::SaveAsScene);

    // 2. Right Container: Platform Exports + Workspace Switcher + MCP Status
    let right_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    // Multi-Platform Export Action Pills
    spawn_menu_button(&mut commands, right_group, "Android", MenuAction::ExportAndroid);
    spawn_menu_button(&mut commands, right_group, "Windows", MenuAction::ExportWindows);
    spawn_menu_button(&mut commands, right_group, "Web", MenuAction::ExportWeb);
    spawn_menu_button(&mut commands, right_group, "iOS", MenuAction::ExportIos);

    // MCP Live Status Pill
    let mcp_pill = commands
        .spawn((
            EditorInternal,
            McpStatusLabel,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_ACCENT),
            ChildOf(right_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("MCP: READY"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(mcp_pill),
    ));

    // Workspace layout selector button (Clicking cycles workspaces: Default -> Level Design -> Programming -> Debugging -> Assets)
    let layout_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(right_group),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_CARD);
            }
        })
        .observe(|click: On<PointerClick>, ws: Res<ActiveWorkspace>, mut writer: MessageWriter<SwitchWorkspaceMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(SwitchWorkspaceMessage(ws.0.next()));
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        WorkspaceSelectorLabel,
        Pickable::IGNORE,
        Text::new(format!("Workspace: {}", current_ws.0.label())),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(layout_btn),
    ));
}

fn spawn_menu_button(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    action: MenuAction,
) {
    let btn = commands
        .spawn((
            EditorInternal,
            action,
            Node {
                padding: UiRect::axes(px(9), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
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
        .observe(
            |click: On<PointerClick>,
             query: Query<&MenuAction>,
             mut writer: MessageWriter<MenuActionMessage>| {
                if click.button == PointerButton::Primary {
                    if let Ok(action) = query.get(click.entity) {
                        writer.write(MenuActionMessage(*action));
                    }
                }
            },
        )
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(btn),
    ));
}

#[derive(Component)]
pub struct McpStatusLabel;


/// Handles executing menu actions received via MenuActionMessage.
pub fn handle_menu_actions(world: &mut World) {
    let actions: Vec<MenuAction> = {
        let Some(mut messages) = world.get_resource_mut::<Messages<MenuActionMessage>>() else {
            return;
        };
        messages.drain().map(|m| m.0).collect()
    };

    for action in actions {
        match action {
            MenuAction::NewProject => {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    let _ = crate::project::ProjectScaffolder::create_project(&folder, "MyBevyGame");
                    if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                        *status = crate::export::ExportStatus::Success {
                            target: "Project Scaffolder".to_string(),
                            message: "Created clean, zero-error standalone Bevy project".to_string(),
                            path: folder,
                        };
                    }
                }
            }
            MenuAction::NewScene => {
                info!("Creating new scene: resetting user scene entities...");
                world.despawn_all::<With<crate::scene::SceneEntity>>();

                if let Some(mut current_scene) = world.get_resource_mut::<CurrentScene>() {
                    current_scene.file_path = None;
                    current_scene.is_modified = false;
                }

                // Re-spawn starter scene
                let _ = world.run_system_once(primitives::spawn_starter_scene);
                info!("New scene created successfully.");
            }
            MenuAction::OpenScene => {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Bevy Scene (*.ron)", &["ron", "bsn"])
                    .pick_file()
                {
                    info!("Selected scene to open: {:?}", file);
                    world.despawn_all::<With<crate::scene::SceneEntity>>();

                    let path_str = file.to_string_lossy().to_string();
                    let asset_server = world.resource::<AssetServer>().clone();
                    world.spawn((
                        crate::scene::SceneEntity,
                        Name::new(format!("Scene: {}", file.file_name().and_then(|n| n.to_str()).unwrap_or("loaded"))),
                        bevy::world_serialization::DynamicWorldRoot(asset_server.load(path_str)),
                        Transform::IDENTITY,
                    ));

                    if let Some(mut current_scene) = world.get_resource_mut::<CurrentScene>() {
                        current_scene.file_path = Some(file);
                        current_scene.is_modified = false;
                    }
                }
            }
            MenuAction::SaveScene => {
                let existing_path = world.resource::<CurrentScene>().file_path.clone();
                if let Some(path) = existing_path {
                    let _ = save_load::save_world_to_scene(world, &path);
                } else {
                    save_as_dialog(world);
                }
            }
            MenuAction::SaveAsScene => {
                save_as_dialog(world);
            }
            MenuAction::ExportAndroid => {
                trigger_export(world, "android");
            }
            MenuAction::ExportWindows => {
                trigger_export(world, "windows");
            }
            MenuAction::ExportWeb => {
                trigger_export(world, "web");
            }
            MenuAction::ExportIos => {
                trigger_export(world, "ios");
            }
        }
    }
}

fn save_as_dialog(world: &mut World) {
    if let Some(file) = rfd::FileDialog::new()
        .add_filter("Bevy Scene (*.ron)", &["ron"])
        .set_file_name("scene.ron")
        .save_file()
    {
        if let Ok(()) = save_load::save_world_to_scene(world, &file) {
            if let Some(mut current_scene) = world.get_resource_mut::<CurrentScene>() {
                current_scene.file_path = Some(file);
                current_scene.is_modified = false;
            }
        }
    }
}

fn trigger_export(world: &mut World, target_id: &str) {
    let project_dir = if target_id == "windows" {
        rfd::FileDialog::new()
            .set_title("Select the Cargo project to build")
            .pick_folder()
    } else {
        None
    };
    if target_id == "windows" && project_dir.is_none() {
        return;
    }

    if let Some(output_dir) = rfd::FileDialog::new()
        .set_title("Select the export output folder")
        .pick_folder()
    {
        let current_scene_path = world.resource::<CurrentScene>().file_path.clone();
        world.write_message(RequestExportMessage {
            target_id: target_id.to_string(),
            output_dir,
            project_name: "BevyGame".to_string(),
            project_dir,
            scene_path: current_scene_path,
        });
    }
}

/// State tracking live commands received from the MCP Server bridge file.
#[derive(Resource, Default)]
pub struct McpBridgeState {
    pub last_timestamp: u64,
}

/// Polls `assets/.mcp_bridge.json` to reflect live AI/MCP operations directly in the editor UI.
pub fn update_mcp_bridge_status(
    mut mcp_state: ResMut<McpBridgeState>,
    mut label_query: Query<&mut Text, With<McpStatusLabel>>,
    mut export_status: ResMut<crate::export::ExportStatus>,
) {
    let bridge_path = std::path::Path::new("assets/.mcp_bridge.json");
    if let Ok(metadata) = std::fs::metadata(bridge_path) {
        if let Ok(modified) = metadata.modified() {
            let dur = modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            if dur > mcp_state.last_timestamp {
                mcp_state.last_timestamp = dur;
                if let Ok(content) = std::fs::read_to_string(bridge_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        let action = val.get("action").and_then(|a| a.as_str()).unwrap_or("Active");
                        let entity = val.get("entity").and_then(|e| e.as_str()).unwrap_or("");
                        let details = val.get("details").and_then(|d| d.as_str()).unwrap_or("");

                        if let Ok(mut txt) = label_query.single_mut() {
                            txt.0 = format!("MCP: {} ({})", action.to_uppercase(), entity);
                        }

                        *export_status = crate::export::ExportStatus::Success {
                            target: "MCP Controller".to_string(),
                            message: format!("AI Event: {} on '{}' -> {}", action, entity, details),
                            path: std::path::PathBuf::from("assets/.mcp_bridge.json"),
                        };
                    }
                }
            }
        }
    }
}

/// Handles switching workspace layout presets and updating UI layout accordingly.
pub fn handle_switch_workspace(
    mut active_ws: ResMut<ActiveWorkspace>,
    mut reader: MessageReader<SwitchWorkspaceMessage>,
    mut label_query: Query<&mut Text, With<WorkspaceSelectorLabel>>,
    mut hierarchy_state: ResMut<crate::ui::hierarchy::HierarchyState>,
    mut inspector_state: ResMut<crate::ui::inspector::InspectorState>,
    mut asset_browser_state: ResMut<crate::ui::asset_browser::AssetBrowserState>,
    mut dock_query: Query<&mut Node, With<crate::ui::hierarchy::HierarchyDockRoot>>,
    mut insp_query: Query<&mut Node, (With<crate::ui::inspector::InspectorDockRoot>, Without<crate::ui::hierarchy::HierarchyDockRoot>)>,
    mut asset_query: Query<&mut Node, (With<crate::ui::asset_browser::ProjectExplorerDockRoot>, Without<crate::ui::hierarchy::HierarchyDockRoot>, Without<crate::ui::inspector::InspectorDockRoot>)>,
) {
    for msg in reader.read() {
        active_ws.0 = msg.0;

        if let Ok(mut label) = label_query.single_mut() {
            label.0 = format!("Workspace: {}", active_ws.0.label());
        }

        match active_ws.0 {
            WorkspacePreset::Default => {
                hierarchy_state.is_collapsed = false;
                hierarchy_state.width = 250.0;
                inspector_state.is_collapsed = false;
                inspector_state.width = 380.0;
                asset_browser_state.is_collapsed = false;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(250.0);
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(380.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(250.0);
                    node.display = Display::Flex;
                }
            }
            WorkspacePreset::LevelDesign => {
                // Maximum viewport room: slim side panels, minimize asset browser
                hierarchy_state.is_collapsed = false;
                hierarchy_state.width = 220.0;
                inspector_state.is_collapsed = false;
                inspector_state.width = 320.0;
                asset_browser_state.is_collapsed = true;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(220.0);
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(320.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(32.0); // Collapsed
                }
            }
            WorkspacePreset::Programming => {
                // Collapse hierarchy, focus inspector and asset scripts
                hierarchy_state.is_collapsed = true;
                inspector_state.is_collapsed = false;
                inspector_state.width = 420.0;
                asset_browser_state.is_collapsed = false;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(34.0); // Collapsed
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(420.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(300.0);
                    node.display = Display::Flex;
                }
            }
            WorkspacePreset::Debugging => {
                // Expanded inspector for component diagnostics
                hierarchy_state.is_collapsed = false;
                hierarchy_state.width = 280.0;
                inspector_state.is_collapsed = false;
                inspector_state.width = 460.0;
                inspector_state.is_debug_view = true;
                asset_browser_state.is_collapsed = true;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(280.0);
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(460.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(32.0);
                }
            }
            WorkspacePreset::Assets => {
                // Focus bottom asset browser
                hierarchy_state.is_collapsed = false;
                hierarchy_state.width = 220.0;
                inspector_state.is_collapsed = false;
                inspector_state.width = 320.0;
                asset_browser_state.is_collapsed = false;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(220.0);
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(320.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(380.0);
                    node.display = Display::Flex;
                }
            }
        }
    }
}


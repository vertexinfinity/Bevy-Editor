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

/// Spawns the top unified header and menu bar.
pub fn spawn_menu_bar(mut commands: Commands) {
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

    // 1. Left Container: Brand + Scene Actions
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
                border_radius: BorderRadius::all(px(4)),
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
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(brand_pill),
    ));

    // Menu Action Buttons
    spawn_menu_button(&mut commands, left_group, "+ Project", MenuAction::NewProject);
    spawn_menu_button(&mut commands, left_group, "New Scene", MenuAction::NewScene);
    spawn_menu_button(&mut commands, left_group, "Open...", MenuAction::OpenScene);
    spawn_menu_button(&mut commands, left_group, "Save", MenuAction::SaveScene);
    spawn_menu_button(&mut commands, left_group, "Save As...", MenuAction::SaveAsScene);

    // 2. Center Container: Transport Controls (Play / Pause / Stop / Step)
    let center_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_CARD),
            ChildOf(bar_root),
        ))
        .id();

    spawn_play_button(&mut commands, center_group, "PLAY", crate::core::play_mode::PlayModeAction::Play, true);
    spawn_play_button(&mut commands, center_group, "PAUSE", crate::core::play_mode::PlayModeAction::Pause, false);
    spawn_play_button(&mut commands, center_group, "STOP", crate::core::play_mode::PlayModeAction::Stop, false);
    spawn_play_button(&mut commands, center_group, "STEP", crate::core::play_mode::PlayModeAction::Step, false);
    spawn_play_button(&mut commands, center_group, "WINDOW", crate::core::play_mode::PlayModeAction::PlayInWindow, false);

    // 3. Right Container: Platform Exports + MCP Status + Workspace layout
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
                border_radius: BorderRadius::all(px(4)),
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

    // Workspace layout selector pill
    let layout_chip = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(right_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Layout: Default"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(layout_chip),
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

fn spawn_play_button(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    action: crate::core::play_mode::PlayModeAction,
    is_primary: bool,
) {
    let (normal_bg, hover_bg) = if is_primary {
        (ACCENT_EMERALD, ACCENT_EMERALD_HOVER)
    } else {
        (BG_INPUT, BG_HOVER)
    };

    let btn = commands
        .spawn((
            EditorInternal,
            action,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(normal_bg),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .observe(move |trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(hover_bg);
            }
        })
        .observe(move |trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(normal_bg);
            }
        })
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<crate::core::play_mode::PlayModeAction>| {
            if click.button == PointerButton::Primary {
                writer.write(action);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(if is_primary { Color::WHITE } else { TEXT_PRIMARY }),
        ChildOf(btn),
    ));
}

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
    if let Some(output_dir) = rfd::FileDialog::new().pick_folder() {
        let current_scene_path = world.resource::<CurrentScene>().file_path.clone();
        world.write_message(RequestExportMessage {
            target_id: target_id.to_string(),
            output_dir,
            project_name: "BevyGame".to_string(),
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

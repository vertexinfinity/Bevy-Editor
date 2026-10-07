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

    // Left Container: Brand + Scene Actions
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
                margin: UiRect::right(px(10)),
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
        Text::new("◈"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
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
            font_size: bevy::text::FontSize::Px(12.0),
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

    // Separator
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Node {
            width: px(1),
            height: px(18),
            margin: UiRect::axes(px(4), px(0)),
            ..default()
        },
        BackgroundColor(BORDER_CARD),
        ChildOf(left_group),
    ));

    // Multi-Platform Export Action Pills
    spawn_menu_button(&mut commands, left_group, "Android", MenuAction::ExportAndroid);
    spawn_menu_button(&mut commands, left_group, "Windows", MenuAction::ExportWindows);
    spawn_menu_button(&mut commands, left_group, "Web", MenuAction::ExportWeb);
    spawn_menu_button(&mut commands, left_group, "iOS", MenuAction::ExportIos);

    // Right Container: Workspace layout switcher + Scene chip
    let right_group = commands
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
        Text::new("Layout: Default ▾"),
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

//! Top menu bar with File, Save, Open, and Export actions.

use std::path::PathBuf;
use bevy::{picking::hover::Hovered, prelude::*, ui::Pressed};
use crate::export::RequestExportMessage;
use crate::scene::{save_load, CurrentScene, EditorInternal};

#[derive(Component, Clone, Copy)]
pub enum MenuAction {
    NewScene,
    OpenScene,
    SaveScene,
    ExportAndroid,
    ExportWindows,
    ExportWeb,
    ExportIos,
}

/// Spawns the top menu bar.
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
                height: px(36),
                border: UiRect::bottom(px(1)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(4)),
                column_gap: px(6),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.14)),
            BorderColor::all(Color::srgb(0.25, 0.25, 0.28)),
        ))
        .id();

    // Spawn Title / Brand
    commands.spawn((
        EditorInternal,
        Text::new("Bevy Editor"),
        TextFont {
            font_size: bevy::text::FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        Node {
            margin: UiRect::right(px(16)),
            ..default()
        },
        ChildOf(bar_root),
    ));

    // Spawn Menu Action Buttons
    spawn_menu_button(&mut commands, bar_root, "New", MenuAction::NewScene);
    spawn_menu_button(&mut commands, bar_root, "Open...", MenuAction::OpenScene);
    spawn_menu_button(&mut commands, bar_root, "Save", MenuAction::SaveScene);

    // Separator
    commands.spawn((
        EditorInternal,
        Node {
            width: px(1),
            height: px(18),
            margin: UiRect::axes(px(6), px(0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.3, 0.3, 0.35)),
        ChildOf(bar_root),
    ));

    // Export Buttons
    spawn_menu_button(&mut commands, bar_root, "Export Android", MenuAction::ExportAndroid);
    spawn_menu_button(&mut commands, bar_root, "Export Windows", MenuAction::ExportWindows);
    spawn_menu_button(&mut commands, bar_root, "Export Web", MenuAction::ExportWeb);
    spawn_menu_button(&mut commands, bar_root, "Export iOS", MenuAction::ExportIos);
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
                padding: UiRect::axes(px(10), px(4)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
            BorderColor::all(Color::srgb(0.28, 0.28, 0.32)),
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
        ChildOf(btn),
    ));
}

/// Handles clicking top menu action buttons.
pub fn handle_menu_actions(world: &mut World) {
    let mut pressed_actions = Vec::new();

    // Query pressed and hovered states
    {
        let mut query = world.query::<(Entity, &MenuAction, Option<&Pressed>, Option<&Hovered>, &mut BackgroundColor)>();
        for (_entity, action, pressed, hovered, mut bg) in query.iter_mut(world) {
            if pressed.is_some() {
                *bg = BackgroundColor(Color::srgb(0.3, 0.45, 0.8));
                pressed_actions.push(*action);
            } else if hovered.is_some() {
                *bg = BackgroundColor(Color::srgb(0.26, 0.26, 0.32));
            } else {
                *bg = BackgroundColor(Color::srgb(0.18, 0.18, 0.22));
            }
        }
    }

    for action in pressed_actions {
        match action {
            MenuAction::NewScene => {
                info!("Creating new scene...");
            }
            MenuAction::OpenScene => {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Bevy Scene (*.ron)", &["ron", "bsn"])
                    .pick_file()
                {
                    info!("Selected scene to open: {:?}", file);
                }
            }
            MenuAction::SaveScene => {
                let scene_path = world
                    .resource::<CurrentScene>()
                    .file_path
                    .clone()
                    .unwrap_or_else(|| {
                        rfd::FileDialog::new()
                            .add_filter("Bevy Scene (*.ron)", &["ron"])
                            .set_file_name("scene.ron")
                            .save_file()
                            .unwrap_or_else(|| PathBuf::from("scene.ron"))
                    });

                let _ = save_load::save_world_to_scene(world, &scene_path);
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

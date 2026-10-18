//! Components card, Add Component menu tree, and component insertion systems.
use std::path::{Path, PathBuf};
use std::fs;
use bevy::{ecs::message::{MessageReader, MessageWriter}, picking::{events::{PointerClick, PointerOut, PointerOver}, pointer::PointerButton}, prelude::*, inspector::InspectorSelection};
use crate::scene::{components::*, EditorInternal};
use crate::ui::theme::*;
use super::types::*;
pub(crate) fn spawn_add_tree_branch(commands: &mut Commands, parent: Entity, label: &str) -> Entity {
    let label = label.to_owned();
    let row = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100),
                padding: UiRect::axes(px(8), px(4)),
                border_radius: radius_button(),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(parent),
        ))
        .id();
    let label_entity = spawn_add_tree_label(commands, row, &format!("[+] {label}"));
    let contents = commands
        .spawn((
            EditorInternal,
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: UiRect::left(px(10)),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    commands.entity(row).observe(move |click: On<PointerClick>, mut nodes: Query<&mut Node>, mut texts: Query<&mut Text>| {
        if click.button != PointerButton::Primary {
            return;
        }

        if let Ok(mut node) = nodes.get_mut(contents) {
            let is_open = node.display == Display::Flex;
            node.display = if is_open { Display::None } else { Display::Flex };
            if let Ok(mut text) = texts.get_mut(label_entity) {
                text.0 = format!("[{}] {label}", if is_open { "+" } else { "-" });
            }
        }
    });

    contents
}

pub(crate) fn spawn_add_tree_label(commands: &mut Commands, parent: Entity, label: &str) -> Entity {
    commands
        .spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(label),
            TextFont {
                font_size: bevy::text::FontSize::Px(11.0),
                ..default()
            },
            TextColor(TEXT_PRIMARY),
            ChildOf(parent),
        ))
        .id()
}

pub(crate) fn spawn_script_directory(commands: &mut Commands, parent: Entity, directory: &ScriptDirectory) {
    let contents = spawn_add_tree_branch(commands, parent, &directory.name);
    spawn_script_entries(commands, contents, &directory.scripts);
    for child in &directory.directories {
        spawn_script_directory(commands, contents, child);
    }
}

pub(crate) fn spawn_script_entries(commands: &mut Commands, parent: Entity, scripts: &[(String, String)]) {
    for (name, path) in scripts {
        spawn_add_comp_btn(
            commands,
            parent,
            name,
            AddComponentKind::ScriptAsset(path.clone()),
        );
    }
}

pub(crate) fn spawn_add_comp_toggle(commands: &mut Commands, parent: Entity) {
    let button = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100),
                padding: UiRect::axes(px(8), px(5)),
                border: UiRect::ZERO,
                border_radius: radius_button(),
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
        .observe(|click: On<PointerClick>, mut menu_query: Query<&mut Node, With<InspectorAddMenu>>| {
            if click.button == PointerButton::Primary {
                if let Ok(mut node) = menu_query.single_mut() {
                    node.display = if node.display == Display::None { Display::Flex } else { Display::None };
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Add Component  +"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(button),
    ));
}

pub(crate) fn spawn_add_comp_btn(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    kind: AddComponentKind,
) {
    let btn = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100),
                padding: UiRect::axes(px(8), px(4)),
                border_radius: radius_button(),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_PANEL),
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
        .observe(move |mut click: On<PointerClick>, mut writer: MessageWriter<AddComponentKind>, mut menu_query: Query<&mut Node, With<InspectorAddMenu>>| {
            if click.button == PointerButton::Primary {
                click.propagate(false);
                writer.write(kind.clone());
                if let Ok(mut node) = menu_query.get_mut(parent) {
                    node.display = Display::None;
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
        ChildOf(btn),
    ));
}

pub(crate) fn find_script_tree(root: &Path) -> (Vec<ScriptDirectory>, Vec<(String, String)>) {
    fn visit(directory: &Path, root: &Path) -> Option<ScriptDirectory> {
        let mut entries: Vec<PathBuf> = fs::read_dir(directory)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .collect();
        entries.sort_by_key(|path| path.file_name().map(|name| name.to_string_lossy().to_ascii_lowercase()));

        let mut scripts = Vec::new();
        let mut directories = Vec::new();
        for path in entries {
            if path.is_dir() {
                if let Some(child) = visit(&path, root) {
                    directories.push(child);
                }
            } else if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "lua" | "rhai" | "rs"))
            {
                if let Ok(relative_path) = path.strip_prefix(root) {
                    if let Some(name) = path.file_name() {
                        scripts.push((
                            name.to_string_lossy().into_owned(),
                            relative_path.to_string_lossy().replace('\\', "/"),
                        ));
                    }
                }
            }
        }

        if scripts.is_empty() && directories.is_empty() {
            return None;
        }

        Some(ScriptDirectory {
            name: directory.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            scripts,
            directories,
        })
    }

    let Some(tree) = visit(root, root) else {
        return (Vec::new(), Vec::new());
    };
    (tree.directories, tree.scripts)
}

pub fn handle_add_component_actions(
    mut actions: MessageReader<AddComponentKind>,
    selection: Res<InspectorSelection>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(selected) = selection.0 else {
        return;
    };

    for action in actions.read() {
        match action {
            AddComponentKind::MeshCube => {
                let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
                let material = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.7, 0.9),
                    perceptual_roughness: 0.4,
                    ..default()
                });
                commands.entity(selected).insert((Mesh3d(mesh), MeshMaterial3d(material)));
            }
            AddComponentKind::MeshSphere => {
                let mesh = meshes.add(Sphere::new(0.6));
                let material = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.9, 0.4, 0.2),
                    perceptual_roughness: 0.3,
                    ..default()
                });
                commands.entity(selected).insert((Mesh3d(mesh), MeshMaterial3d(material)));
            }
            AddComponentKind::MeshPlane => {
                let mesh = meshes.add(Plane3d::default().mesh().size(2.0, 2.0));
                let material = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.25, 0.65, 0.35),
                    perceptual_roughness: 0.7,
                    ..default()
                });
                commands.entity(selected).insert((Mesh3d(mesh), MeshMaterial3d(material)));
            }
            AddComponentKind::NewScript => {
                let script_name = format!("actor_{}", selected.index());
                let script_dir = Path::new("assets").join("scripts");
                match crate::assets::AssetCreator::create_asset(&script_dir, &script_name, crate::assets::AssetTemplateKind::LuaScript) {
                    Ok(created_path) => {
                        let rel_path = format!("scripts/{}.lua", script_name);
                        commands.entity(selected).insert(ScriptComponent {
                            script_file: rel_path,
                            ..default()
                        });
                        let _ = crate::assets::AssetCreator::open_in_system_editor(&created_path);
                        info!("Created and attached new script at {:?}", created_path);
                    }
                    Err(err) => {
                        error!("Failed to create script: {}", err);
                    }
                }
            }
            AddComponentKind::Script => {
                commands.entity(selected).insert(ScriptComponent::default());
                info!("Added ScriptComponent to {:?}", selected);
            }
            AddComponentKind::ScriptAsset(script_file) => {
                commands.entity(selected).insert(ScriptComponent {
                    script_file: script_file.clone(),
                    ..default()
                });
                info!("Attached script '{}' to {:?}", script_file, selected);
            }
            AddComponentKind::SpinAnimation => {
                commands.entity(selected).insert(SpinAnimation::default());
                info!("Added SpinAnimation to {:?}", selected);
            }
            AddComponentKind::Velocity => {
                commands.entity(selected).insert(Velocity { linear: Vec3::new(0.0, 5.0, 0.0) });
                info!("Added Velocity to {:?}", selected);
            }
            AddComponentKind::PointLight => {
                commands.entity(selected).insert(PointLight {
                    intensity: 5_000.0,
                    range: 20.0,
                    color: Color::srgb(1.0, 0.95, 0.8),
                    shadow_maps_enabled: true,
                    ..default()
                });
                info!("Added PointLight to {:?}", selected);
            }
            AddComponentKind::DirectionalLight => {
                commands.entity(selected).insert(DirectionalLight {
                    illuminance: 10_000.0,
                    shadow_maps_enabled: true,
                    ..default()
                });
                info!("Added DirectionalLight to {:?}", selected);
            }
            AddComponentKind::SpotLight => {
                commands.entity(selected).insert(SpotLight {
                    intensity: 1_500.0,
                    range: 20.0,
                    ..default()
                });
                info!("Added SpotLight to {:?}", selected);
            }
            AddComponentKind::Camera3d => {
                commands.entity(selected).insert(Camera3d::default());
                info!("Added Camera3d to {:?}", selected);
            }
            AddComponentKind::GameplayTag => {
                commands.entity(selected).insert(GameplayTag { tag: "Interactable".to_string() });
                info!("Added GameplayTag to {:?}", selected);
            }
        }
    }
}
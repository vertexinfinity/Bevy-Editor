//! Scene Hierarchy Panel for Bevylite.
//!
//! Provides a clean tree-structured entity list, click-to-select,
//! entity creation primitives (Cube, Sphere, Light), and collapsible docking.

use bevy::{
    ecs::message::{Message, MessageReader, MessageWriter},
    inspector::InspectorSelection,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::scene::{EditorInternal, SceneEntity};
use crate::ui::theme::*;

/// State for the Hierarchy panel.
#[derive(Resource, Debug, Clone)]
pub struct HierarchyState {
    pub is_collapsed: bool,
    pub width: f32,
    #[allow(dead_code)]
    pub filter: String,
}

impl Default for HierarchyState {
    fn default() -> Self {
        Self {
            is_collapsed: false,
            width: 250.0,
            filter: String::new(),
        }
    }
}

// Markers for UI elements
#[derive(Component)]
pub struct HierarchyDockRoot;

#[derive(Component)]
pub struct HierarchyTitleGroup;

#[derive(Component)]
pub struct HierarchyActionsRow;

#[derive(Component)]
pub struct HierarchyListContainer;

#[derive(Component)]
pub struct HierarchyCountLabel;

#[derive(Component)]
pub struct HierarchyCollapseBtnLabel;

#[derive(Component)]
#[allow(dead_code)]
pub struct HierarchyEntityRow(pub Entity);

// Messages
#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleHierarchyDockMessage;

#[derive(Message, Clone, Copy, Debug)]
pub enum SpawnPrimitiveKind {
    Cube,
    Sphere,
    Light,
    Empty,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct SpawnPrimitiveMessage(pub SpawnPrimitiveKind);

/// Spawns the Hierarchy dock panel on the left.
pub fn spawn_hierarchy_panel(mut commands: Commands) {
    let dock_root = commands
        .spawn((
            EditorInternal,
            HierarchyDockRoot,
            Name::new("HierarchyDock"),
            Node {
                position_type: PositionType::Absolute,
                top: px(38),
                left: px(0),
                bottom: px(28),
                width: px(250),
                border: UiRect::right(px(1)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_SUBTLE),
        ))
        .id();

    // 1. Header Bar
    let header = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(34),
                border: UiRect::bottom(px(1)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(10), px(4)),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(dock_root),
        ))
        .id();

    // Left: Title + Entity Count
    let title_group = commands
        .spawn((
            EditorInternal,
            HierarchyTitleGroup,
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
        Text::new("Hierarchy"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(title_group),
    ));

    commands.spawn((
        EditorInternal,
        HierarchyCountLabel,
        Pickable::IGNORE,
        Text::new("0"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        Node {
            padding: UiRect::axes(px(5), px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(ACCENT_CYAN_DIM),
        ChildOf(title_group),
    ));

    // Right: Collapse & Drag-Resize toggle button
    let collapse_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(header),
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
        .observe(|drag: On<bevy::picking::events::PointerDrag>, mut state: ResMut<HierarchyState>, mut dock_query: Query<&mut Node, With<HierarchyDockRoot>>| {
            if !state.is_collapsed {
                let new_width = (state.width + drag.distance.x * 0.05).clamp(180.0, 500.0);
                state.width = new_width;
                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(new_width);
                }
            }
        })
        .observe(|click: On<PointerClick>, mut writer: MessageWriter<ToggleHierarchyDockMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(ToggleHierarchyDockMessage);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        HierarchyCollapseBtnLabel,
        Pickable::IGNORE,
        Text::new("<"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(collapse_btn),
    ));

    // 2. Action Bar: Spawn Primitives Quick Buttons
    let actions_row = commands
        .spawn((
            EditorInternal,
            HierarchyActionsRow,
            Node {
                height: px(30),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(2)),
                column_gap: px(4),
                border: UiRect::bottom(px(1)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(dock_root),
        ))
        .id();

    spawn_primitive_btn(&mut commands, actions_row, "+Cube", SpawnPrimitiveKind::Cube);
    spawn_primitive_btn(&mut commands, actions_row, "+Sphere", SpawnPrimitiveKind::Sphere);
    spawn_primitive_btn(&mut commands, actions_row, "+Light", SpawnPrimitiveKind::Light);
    spawn_primitive_btn(&mut commands, actions_row, "+Empty", SpawnPrimitiveKind::Empty);

    // 3. Entity List Container
    commands.spawn((
        EditorInternal,
        HierarchyListContainer,
        Node {
            flex_grow: 1.0,
            width: percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(px(4), px(6)),
            row_gap: px(2),
            overflow: Overflow::clip_y(),
            ..default()
        },
        ChildOf(dock_root),
    ));
}

fn spawn_primitive_btn(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    kind: SpawnPrimitiveKind,
) {
    let btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
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
                *bg = BackgroundColor(BG_CARD);
            }
        })
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<SpawnPrimitiveMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(SpawnPrimitiveMessage(kind));
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(btn),
    ));
}

/// Updates the hierarchy tree view to reflect all active `SceneEntity` components.
pub fn update_hierarchy_list(
    mut commands: Commands,
    entities_query: Query<(Entity, Option<&Name>, Option<&DirectionalLight>, Option<&Mesh3d>, Option<&Visibility>), With<SceneEntity>>,
    selection: Res<InspectorSelection>,
    mut count_label_query: Query<&mut Text, With<HierarchyCountLabel>>,
    list_container_query: Query<Entity, With<HierarchyListContainer>>,
    existing_rows: Query<Entity, With<HierarchyEntityRow>>,
) {
    // Only rebuild if entity counts or selection change
    let count = entities_query.iter().count();
    if let Ok(mut count_text) = count_label_query.single_mut() {
        count_text.0 = format!("{}", count);
    }

    let Ok(container_ent) = list_container_query.single() else {
        return;
    };

    // Check if we need to refresh list
    if !selection.is_changed() && existing_rows.iter().count() == count {
        return;
    }

    // Clear existing rows
    for row in &existing_rows {
        commands.entity(row).despawn();
    }

    // Spawn new rows
    for (entity, name, is_light, is_mesh, vis) in &entities_query {
        let is_selected = selection.0 == Some(entity);
        let is_hidden = vis == Some(&Visibility::Hidden);
        let name_str = name.map(|n| n.as_str()).unwrap_or("Entity");

        let type_tag = if is_light.is_some() {
            "L"
        } else if is_mesh.is_some() {
            "M"
        } else {
            "•"
        };

        let row_bg = if is_selected {
            ACCENT_CYAN_DIM
        } else {
            BG_CARD
        };

        let row_border = if is_selected {
            BORDER_ACCENT
        } else {
            BORDER_SUBTLE
        };

        let text_color = if is_selected {
            ACCENT_CYAN
        } else if is_hidden {
            TEXT_MUTED
        } else {
            TEXT_PRIMARY
        };

        let row = commands
            .spawn((
                EditorInternal,
                HierarchyEntityRow(entity),
                Node {
                    width: percent(100.0),
                    height: px(26),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::axes(px(8), px(2)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(row_bg),
                BorderColor::all(row_border),
                ChildOf(container_ent),
            ))
            .observe(move |trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
                if !is_selected {
                    if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                        *bg = BackgroundColor(BG_HOVER);
                    }
                }
            })
            .observe(move |trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
                if !is_selected {
                    if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                        *bg = BackgroundColor(BG_CARD);
                    }
                }
            })
            .observe(move |click: On<PointerClick>, mut sel: ResMut<InspectorSelection>| {
                if click.button == PointerButton::Primary {
                    sel.0 = Some(entity);
                    info!("Hierarchy selected entity: {:?}", entity);
                }
            })
            .id();

        // Left: Type Tag + Name
        let label_group = commands
            .spawn((
                EditorInternal,
                Pickable::IGNORE,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(6),
                    ..default()
                },
                ChildOf(row),
            ))
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(type_tag),
            TextFont {
                font_size: bevy::text::FontSize::Px(10.0),
                ..default()
            },
            TextColor(ACCENT_CYAN),
            ChildOf(label_group),
        ));

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(if is_hidden { format!("{name_str} (Hidden)") } else { name_str.to_string() }),
            TextFont {
                font_size: bevy::text::FontSize::Px(11.5),
                ..default()
            },
            TextColor(text_color),
            ChildOf(label_group),
        ));

        // Right Group: Visibility Eye Toggle + Entity ID badge
        let right_group = commands
            .spawn((
                EditorInternal,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(6),
                    ..default()
                },
                ChildOf(row),
            ))
            .id();

        // Visibility Toggle Button
        let vis_btn = commands
            .spawn((
                EditorInternal,
                Node {
                    padding: UiRect::axes(px(4), px(1)),
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BackgroundColor(if is_hidden { BG_INPUT } else { ACCENT_CYAN_DIM }),
                ChildOf(right_group),
            ))
            .observe(move |mut click: On<PointerClick>, mut vis_query: Query<&mut Visibility>, mut commands: Commands| {
                if click.button == PointerButton::Primary {
                    click.propagate(false);
                    if let Ok(mut v) = vis_query.get_mut(entity) {
                        *v = if *v == Visibility::Hidden { Visibility::Inherited } else { Visibility::Hidden };
                    } else {
                        commands.entity(entity).insert(Visibility::Hidden);
                    }
                }
            })
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(if is_hidden { "OFF" } else { "ON" }),
            TextFont {
                font_size: bevy::text::FontSize::Px(9.0),
                ..default()
            },
            TextColor(if is_hidden { TEXT_MUTED } else { ACCENT_CYAN }),
            ChildOf(vis_btn),
        ));

        // Entity ID badge
        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(format!("#{}", entity.index())),
            TextFont {
                font_size: bevy::text::FontSize::Px(9.5),
                ..default()
            },
            TextColor(TEXT_MUTED),
            ChildOf(right_group),
        ));
    }
}

/// Handles spawning primitive shapes from the action bar.
pub fn handle_spawn_primitives(
    world: &mut World,
) {
    let requests: Vec<SpawnPrimitiveKind> = {
        let Some(mut messages) = world.get_resource_mut::<Messages<SpawnPrimitiveMessage>>() else {
            return;
        };
        messages.drain().map(|m| m.0).collect()
    };

    for kind in requests {
        match kind {
            SpawnPrimitiveKind::Cube => {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                let mesh_handle = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
                let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                let mat_handle = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.7, 0.9),
                    perceptual_roughness: 0.4,
                    ..default()
                });

                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Cube"),
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        Transform::from_xyz(0.0, 0.5, 0.0),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned primitive Cube: {:?}", entity);
            }
            SpawnPrimitiveKind::Sphere => {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                let mesh_handle = meshes.add(Sphere::new(0.6));
                let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                let mat_handle = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.9, 0.4, 0.2),
                    perceptual_roughness: 0.3,
                    ..default()
                });

                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Sphere"),
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        Transform::from_xyz(0.0, 0.6, 0.0),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned primitive Sphere: {:?}", entity);
            }
            SpawnPrimitiveKind::Light => {
                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Point Light"),
                        PointLight {
                            intensity: 1_000_000.0,
                            range: 20.0,
                            shadow_maps_enabled: true,
                            ..default()
                        },
                        Transform::from_xyz(0.0, 3.0, 0.0),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned Point Light: {:?}", entity);
            }
            SpawnPrimitiveKind::Empty => {
                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Empty Entity"),
                        Transform::IDENTITY,
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned Empty Entity: {:?}", entity);
            }
        }
    }
}

/// Handles collapsing and expanding the hierarchy dock.
pub fn handle_hierarchy_collapse(
    mut state: ResMut<HierarchyState>,
    mut reader: MessageReader<ToggleHierarchyDockMessage>,
    mut dock_query: Query<&mut Node, With<HierarchyDockRoot>>,
    mut title_query: Query<&mut Node, (With<HierarchyTitleGroup>, Without<HierarchyDockRoot>, Without<HierarchyActionsRow>, Without<HierarchyListContainer>)>,
    mut actions_query: Query<&mut Node, (With<HierarchyActionsRow>, Without<HierarchyDockRoot>, Without<HierarchyTitleGroup>, Without<HierarchyListContainer>)>,
    mut list_query: Query<&mut Node, (With<HierarchyListContainer>, Without<HierarchyDockRoot>, Without<HierarchyTitleGroup>, Without<HierarchyActionsRow>)>,
    mut label_query: Query<&mut Text, With<HierarchyCollapseBtnLabel>>,
) {
    for _ in reader.read() {
        state.is_collapsed = !state.is_collapsed;

        if let Ok(mut node) = dock_query.single_mut() {
            if state.is_collapsed {
                node.width = px(34);
            } else {
                node.width = px(state.width);
            }
        }

        if let Ok(mut title_node) = title_query.single_mut() {
            title_node.display = if state.is_collapsed { Display::None } else { Display::Flex };
        }

        if let Ok(mut actions_node) = actions_query.single_mut() {
            actions_node.display = if state.is_collapsed { Display::None } else { Display::Flex };
        }

        if let Ok(mut list_node) = list_query.single_mut() {
            list_node.display = if state.is_collapsed { Display::None } else { Display::Flex };
        }

        if let Ok(mut label) = label_query.single_mut() {
            label.0 = if state.is_collapsed { ">" } else { "<" }.to_string();
        }
    }
}

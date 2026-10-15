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
    /// True only while the user has clicked into the search box.
    pub search_focused: bool,
    pub search_hovered: bool,
}

impl Default for HierarchyState {
    fn default() -> Self {
        Self {
            is_collapsed: false,
            width: 250.0,
            filter: String::new(),
            search_focused: false,
            search_hovered: false,
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
pub struct HierarchyAddMenu;

#[derive(Component)]
pub struct HierarchyListContainer;

#[derive(Component)]
pub struct HierarchyCountLabel;

#[derive(Component)]
pub struct HierarchyCollapseBtnLabel;

#[derive(Component)]
pub struct HierarchySearchText;

#[derive(Component)]
#[allow(dead_code)]
pub struct HierarchyEntityRow(pub Entity);

// Messages
#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleHierarchyDockMessage;

#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnPrimitiveKind {
    // 3D Primitives
    Cube,
    Sphere,
    Cylinder,
    Plane,
    Capsule,
    Torus,
    // Cameras
    Camera3d,
    // Light Sources
    Light,
    PointLight,
    DirectionalLight,
    SpotLight,
    // UI Elements
    UiButton,
    UiText,
    UiPanel,
    // Hierarchy Management
    Empty,
    Delete,
    Duplicate,
    Unparent,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct SpawnPrimitiveMessage(pub SpawnPrimitiveKind, pub Option<Entity>);

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

    // 2. Search / Filter Input Row
    let search_row = commands
        .spawn((
            EditorInternal,
            HierarchyActionsRow,
            Node {
                height: px(30),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(2)),
                column_gap: px(6),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(dock_root),
        ))
        .observe(|_: On<PointerOver>, mut s: ResMut<HierarchyState>| s.search_hovered = true)
        .observe(|_: On<PointerOut>, mut s: ResMut<HierarchyState>| s.search_hovered = false)
        .id();

    // Search Icon badge
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("🔍"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(search_row),
    ));

    // Search input placeholder / prompt
    commands.spawn((
        EditorInternal,
        HierarchySearchText,
        Pickable::IGNORE,
        Text::new("Search Hierarchy..."),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(search_row),
    ));

    // 3. Action Bar
    let actions_row = commands
        .spawn((
            EditorInternal,
            HierarchyActionsRow,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(8), px(2)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(dock_root),
        ))
        .id();

    let action_buttons = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(26),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
            ChildOf(actions_row),
        ))
        .id();

    let add_menu = commands
        .spawn((
            EditorInternal,
            HierarchyAddMenu,
            GlobalZIndex(150),
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: UiRect::all(px(4)),
                border: UiRect::all(px(1)),
                border_radius: radius_card(),
                max_height: px(380),
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(actions_row),
        ))
        .id();

    spawn_hierarchy_add_toggle(&mut commands, action_buttons, add_menu);
    spawn_primitive_btn(&mut commands, action_buttons, "Dup", SpawnPrimitiveKind::Duplicate);
    spawn_primitive_btn(&mut commands, action_buttons, "Del", SpawnPrimitiveKind::Delete);
    spawn_primitive_btn(&mut commands, action_buttons, "Unparent", SpawnPrimitiveKind::Unparent);

    // 3D Objects
    spawn_hierarchy_menu_section(&mut commands, add_menu, "3D PRIMITIVES");
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Cube", SpawnPrimitiveKind::Cube);
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Sphere", SpawnPrimitiveKind::Sphere);
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Cylinder", SpawnPrimitiveKind::Cylinder);
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Plane Ground", SpawnPrimitiveKind::Plane);
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Capsule", SpawnPrimitiveKind::Capsule);
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Torus", SpawnPrimitiveKind::Torus);

    // Cameras
    spawn_hierarchy_menu_section(&mut commands, add_menu, "CAMERAS");
    spawn_hierarchy_menu_item(&mut commands, add_menu, "📷 3D Camera", SpawnPrimitiveKind::Camera3d);

    // Light Sources
    spawn_hierarchy_menu_section(&mut commands, add_menu, "LIGHT SOURCES");
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Sun (Directional)", SpawnPrimitiveKind::DirectionalLight);
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Point Light", SpawnPrimitiveKind::PointLight);
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Spot Light", SpawnPrimitiveKind::SpotLight);

    // UI Elements
    spawn_hierarchy_menu_section(&mut commands, add_menu, "UI ELEMENTS");
    spawn_hierarchy_menu_item(&mut commands, add_menu, "UI Button", SpawnPrimitiveKind::UiButton);
    spawn_hierarchy_menu_item(&mut commands, add_menu, "UI Text Label", SpawnPrimitiveKind::UiText);
    spawn_hierarchy_menu_item(&mut commands, add_menu, "UI Panel Container", SpawnPrimitiveKind::UiPanel);

    // General
    spawn_hierarchy_menu_section(&mut commands, add_menu, "GENERAL");
    spawn_hierarchy_menu_item(&mut commands, add_menu, "Empty Entity", SpawnPrimitiveKind::Empty);

    // 4. Entity List Container
    commands.spawn((
        EditorInternal,
        HierarchyListContainer,
        Node {
            flex_grow: 1.0,
            width: percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(px(6), px(6)),
            row_gap: px(3),
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
                border_radius: radius_button(),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
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
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<SpawnPrimitiveMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(SpawnPrimitiveMessage(kind, None));
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

fn spawn_hierarchy_add_toggle(commands: &mut Commands, parent: Entity, menu: Entity) {
    let button = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
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
        .observe(move |click: On<PointerClick>, mut menu_query: Query<&mut Node, With<HierarchyAddMenu>>| {
            if click.button == PointerButton::Primary {
                if let Ok(mut node) = menu_query.get_mut(menu) {
                    node.display = if node.display == Display::None { Display::Flex } else { Display::None };
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("+ Add"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(button),
    ));
}

fn spawn_hierarchy_menu_item(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    kind: SpawnPrimitiveKind,
) {
    let button = commands
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
                *bg = BackgroundColor(BG_PANEL);
            }
        })
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<SpawnPrimitiveMessage>, mut menu_query: Query<&mut Node, With<HierarchyAddMenu>>| {
            if click.button == PointerButton::Primary {
                writer.write(SpawnPrimitiveMessage(kind, None));
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
            font_size: bevy::text::FontSize::Px(10.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(button),
    ));
}

fn spawn_hierarchy_menu_section(commands: &mut Commands, parent: Entity, label: &str) {
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(9.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(TEXT_MUTED),
        Node {
            margin: UiRect::axes(px(6), px(3)),
            ..default()
        },
        ChildOf(parent),
    ));
}


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HierarchyCategory {
    Environment,
    Gameplay,
    Props,
}

impl HierarchyCategory {
    fn label(&self) -> &'static str {
        match self {
            Self::Environment => "▾ Environment",
            Self::Gameplay => "▾ Gameplay",
            Self::Props => "▾ Props",
        }
    }
}

/// Updates the hierarchy tree view to reflect all active `SceneEntity` components
/// grouped neatly into categories (Environment, Gameplay, Props).
pub fn update_hierarchy_list(
    mut commands: Commands,
    entities_query: Query<(
        Entity,
        Option<&Name>,
        Option<&DirectionalLight>,
        Option<&PointLight>,
        Option<&Mesh3d>,
        Option<&crate::scene::components::ScriptComponent>,
        Option<&crate::scene::components::GameplayTag>,
        Option<&Visibility>,
        Option<&ChildOf>,
        Option<&Camera3d>,
    ), With<SceneEntity>>,
    scene_entities: Query<(), With<SceneEntity>>,
    selection: Res<InspectorSelection>,
    state: Res<HierarchyState>,
    mut count_label_query: Query<&mut Text, (With<HierarchyCountLabel>, Without<HierarchySearchText>)>,
    list_container_query: Query<Entity, With<HierarchyListContainer>>,
    existing_rows: Query<Entity, With<HierarchyEntityRow>>,
    _visibility_query: Query<&Visibility>,
    vis_changed: Query<(), (With<SceneEntity>, Changed<Visibility>)>,
    added: Query<(), Added<SceneEntity>>,
    mut removed: RemovedComponents<SceneEntity>,
    mut first: Local<bool>,
) {
    let count = entities_query.iter().count();
    if let Ok(mut count_text) = count_label_query.single_mut() {
        count_text.0 = format!("{}", count);
    }

    let Ok(container_ent) = list_container_query.single() else {
        return;
    };

    let removed_any = removed.read().count() > 0;
    if *first
        && !selection.is_changed()
        && !state.is_changed()
        && vis_changed.is_empty()
        && added.is_empty()
        && !removed_any
    {
        return;
    }
    *first = true;

    // Clear existing rows and section headers
    for row in &existing_rows {
        commands.entity(row).despawn();
    }

    // Classify entities into categories - only show root level entities to prevent lag
    let mut items: Vec<(HierarchyCategory, Entity, String, &'static str, bool, bool)> = Vec::new();

    for (entity, name, dir_light, point_light, mesh, script, gameplay_tag, vis, child_of, camera) in &entities_query {
        // Skip child entities if parent is also a SceneEntity (keep collapsed to root)
        if let Some(child) = child_of {
            if scene_entities.contains(child.parent()) {
                continue;
            }
        }

        let is_hidden = vis == Some(&Visibility::Hidden);
        let name_str = name.map(|n| n.as_str()).unwrap_or("Entity").to_string();
        let is_selected = selection.0 == Some(entity);

        let (category, icon) = if camera.is_some() || name_str.to_lowercase().contains("camera") {
            (HierarchyCategory::Gameplay, "📷")
        } else if dir_light.is_some() || name_str.contains("Ground") || name_str.contains("Sky") {
            (HierarchyCategory::Environment, "💡")
        } else if point_light.is_some() {
            (HierarchyCategory::Environment, "💡")
        } else if script.is_some() || gameplay_tag.is_some() || name_str.to_lowercase().contains("player") {
            (HierarchyCategory::Gameplay, "⚡")
        } else if mesh.is_some() {
            (HierarchyCategory::Props, "⬡")
        } else {
            (HierarchyCategory::Props, "📦")
        };

        let filter_query = state.filter.trim().to_lowercase();
        if !filter_query.is_empty() && !name_str.to_lowercase().contains(&filter_query) {
            continue;
        }

        items.push((category, entity, name_str, icon, is_hidden, is_selected));
    }

    // Sort by category first, then entity name
    items.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| a.2.to_lowercase().cmp(&b.2.to_lowercase()))
    });

    let mut current_category: Option<HierarchyCategory> = None;

    for (category, entity, name_str, icon, is_hidden, is_selected) in items {
        // Render Category Header if changed
        if current_category != Some(category) {
            current_category = Some(category);

            let header_row = commands
                .spawn((
                    EditorInternal,
                    HierarchyEntityRow(entity), // marker so it gets despawned on refresh
                    Node {
                        width: percent(100.0),
                        height: px(22),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(px(4), px(2)),
                        margin: UiRect::top(px(4)),
                        ..default()
                    },
                    ChildOf(container_ent),
                ))
                .id();

            commands.spawn((
                EditorInternal,
                Pickable::IGNORE,
                Text::new(category.label()),
                TextFont {
                    font_size: bevy::text::FontSize::Px(10.5),
                    weight: FontWeight::SEMIBOLD,
                    ..default()
                },
                TextColor(TEXT_SECONDARY),
                ChildOf(header_row),
            ));
        }

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
                    padding: UiRect::axes(px(10), px(2)),
                    margin: UiRect::left(px(6)), // Indent under category
                    border: UiRect::all(px(1)),
                    border_radius: radius_button(),
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

        // Left: Category Icon + Name
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
            Text::new(icon),
            TextFont {
                font_size: bevy::text::FontSize::Px(11.0),
                ..default()
            },
            TextColor(if is_selected { ACCENT_CYAN } else { TEXT_SECONDARY }),
            ChildOf(label_group),
        ));

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(if is_hidden { format!("{name_str} (Hidden)") } else { name_str }),
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
            .observe(move |mut click: On<PointerClick>,
                           mut vis_query: Query<&mut Visibility>,
                           children_query: Query<&Children>,
                           mut commands: Commands| {
                if click.button == PointerButton::Primary {
                    click.propagate(false);
                    let new_vis = if let Ok(v) = vis_query.get(entity) {
                        if *v == Visibility::Hidden { Visibility::Inherited } else { Visibility::Hidden }
                    } else {
                        Visibility::Hidden
                    };

                    // Apply to entity
                    if let Ok(mut v) = vis_query.get_mut(entity) {
                        *v = new_vis;
                    } else {
                        commands.entity(entity).insert(new_vis);
                    }

                    // Recursively apply to children
                    let mut stack = vec![entity];
                    while let Some(current) = stack.pop() {
                        if let Ok(children) = children_query.get(current) {
                            for child in children.iter() {
                                if let Ok(mut v) = vis_query.get_mut(child) {
                                    *v = new_vis;
                                }
                                stack.push(child);
                            }
                        }
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

        let delete_btn = commands
            .spawn((
                EditorInternal,
                Node {
                    padding: UiRect::axes(px(4), px(1)),
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BackgroundColor(BG_INPUT),
                ChildOf(right_group),
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
            .observe(move |mut click: On<PointerClick>, mut writer: MessageWriter<SpawnPrimitiveMessage>| {
                if click.button == PointerButton::Primary {
                    click.propagate(false);
                    writer.write(SpawnPrimitiveMessage(SpawnPrimitiveKind::Delete, Some(entity)));
                }
            })
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new("X"),
            TextFont {
                font_size: bevy::text::FontSize::Px(9.0),
                ..default()
            },
            TextColor(TEXT_MUTED),
            ChildOf(delete_btn),
        ));
    }
}

/// Handles spawning primitive shapes from the action bar.
pub fn handle_spawn_primitives(
    world: &mut World,
) {
    let requests: Vec<(SpawnPrimitiveKind, Option<Entity>)> = {
        let Some(mut messages) = world.get_resource_mut::<Messages<SpawnPrimitiveMessage>>() else {
            return;
        };
        messages.drain().map(|m| (m.0, m.1)).collect()
    };

    for (kind, target) in requests {
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
            SpawnPrimitiveKind::Cylinder => {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                let mesh_handle = meshes.add(Cylinder::new(0.5, 1.5));
                let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                let mat_handle = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.8, 0.5),
                    perceptual_roughness: 0.35,
                    ..default()
                });

                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Cylinder"),
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        Transform::from_xyz(0.0, 0.75, 0.0),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned primitive Cylinder: {:?}", entity);
            }
            SpawnPrimitiveKind::Plane => {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                let mesh_handle = meshes.add(Plane3d::default().mesh().size(10.0, 10.0));
                let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                let mat_handle = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.3, 0.35, 0.4),
                    perceptual_roughness: 0.8,
                    ..default()
                });

                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Plane Ground"),
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        Transform::IDENTITY,
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned primitive Plane: {:?}", entity);
            }
            SpawnPrimitiveKind::Capsule => {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                let mesh_handle = meshes.add(Capsule3d::default());
                let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                let mat_handle = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.8, 0.3, 0.7),
                    perceptual_roughness: 0.4,
                    ..default()
                });

                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Capsule"),
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        Transform::from_xyz(0.0, 1.0, 0.0),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned primitive Capsule: {:?}", entity);
            }
            SpawnPrimitiveKind::Torus => {
                let mut meshes = world.resource_mut::<Assets<Mesh>>();
                let mesh_handle = meshes.add(Torus::default());
                let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                let mat_handle = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.9, 0.7, 0.1),
                    perceptual_roughness: 0.3,
                    ..default()
                });

                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Torus"),
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        Transform::from_xyz(0.0, 0.5, 0.0),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned primitive Torus: {:?}", entity);
            }
            SpawnPrimitiveKind::Light | SpawnPrimitiveKind::PointLight => {
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
            SpawnPrimitiveKind::DirectionalLight => {
                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Sun (Directional Light)"),
                        DirectionalLight {
                            illuminance: 10_000.0,
                            shadow_maps_enabled: true,
                            ..default()
                        },
                        Transform::from_xyz(5.0, 10.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned Sun (Directional Light): {:?}", entity);
            }
            SpawnPrimitiveKind::SpotLight => {
                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("Spot Light"),
                        SpotLight {
                            intensity: 2_000_000.0,
                            range: 25.0,
                            inner_angle: 0.5,
                            outer_angle: 0.8,
                            shadow_maps_enabled: true,
                            ..default()
                        },
                        Transform::from_xyz(0.0, 6.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned Spot Light: {:?}", entity);
            }
            SpawnPrimitiveKind::UiButton => {
                let btn_entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("UI Button"),
                        crate::scene::components::UiButtonClickAction::default(),
                        Node {
                            width: px(130),
                            height: px(38),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: BorderRadius::all(px(5)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.15, 0.45, 0.75)),
                    ))
                    .id();

                world.spawn((
                    Pickable::IGNORE,
                    Text::new("Interactive Button"),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    ChildOf(btn_entity),
                ));

                world.resource_mut::<InspectorSelection>().0 = Some(btn_entity);
                info!("Spawned UI Button: {:?}", btn_entity);
            }
            SpawnPrimitiveKind::UiText => {
                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("UI Text Label"),
                        Text::new("Hello Bevy World"),
                        TextFont {
                            font_size: bevy::text::FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Node {
                            margin: UiRect::all(px(10)),
                            ..default()
                        },
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned UI Text: {:?}", entity);
            }
            SpawnPrimitiveKind::UiPanel => {
                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("UI Panel"),
                        Node {
                            width: px(220),
                            height: px(140),
                            padding: UiRect::all(px(10)),
                            border_radius: BorderRadius::all(px(6)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.12, 0.18, 0.85)),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned UI Panel: {:?}", entity);
            }
            SpawnPrimitiveKind::Camera3d => {
                let entity = world
                    .spawn((
                        SceneEntity,
                        Name::new("3D Camera"),
                        Camera3d::default(),
                        Camera {
                            // Keep inactive in editor so viewport stays driven by editor fly camera
                            is_active: false,
                            ..default()
                        },
                        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
                    ))
                    .id();

                world.resource_mut::<InspectorSelection>().0 = Some(entity);
                info!("Spawned primitive 3D Camera: {:?}", entity);
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
            SpawnPrimitiveKind::Delete => {
                if let Some(e) = target.or(world.resource::<InspectorSelection>().0) {
                    if let Ok(ent) = world.get_entity_mut(e) {
                        ent.despawn();
                    }
                    if world.resource::<InspectorSelection>().0 == Some(e) {
                        world.resource_mut::<InspectorSelection>().0 = None;
                    }
                }
            }
            SpawnPrimitiveKind::Duplicate => {
                if let Some(e) = world.resource::<InspectorSelection>().0 {
                    let new_e = world.entity_mut(e).clone_and_spawn();
                    world.resource_mut::<InspectorSelection>().0 = Some(new_e);
                }
            }
            SpawnPrimitiveKind::Unparent => {
                if let Some(e) = world.resource::<InspectorSelection>().0 {
                    world.entity_mut(e).remove::<ChildOf>();
                }
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

/// Handles keyboard input when searching in the hierarchy.
pub fn handle_hierarchy_search_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut char_events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut state: ResMut<HierarchyState>,
    mut search_query: Query<&mut Text, With<HierarchySearchText>>,
) {
    // Focus handling: click inside search row = focus, click anywhere else = blur
    if mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right) {
        let want = state.search_hovered && mouse.just_pressed(MouseButton::Left);
        if state.search_focused != want {
            state.search_focused = want;
        }
    }
    if state.search_focused && keyboard_input.just_pressed(KeyCode::Enter) {
        state.search_focused = false;
    }

    if state.is_collapsed || !state.search_focused {
        char_events.clear();
        return;
    }

    let mut changed = false;

    // Check backspace
    if keyboard_input.just_pressed(KeyCode::Backspace) {
        if !state.filter.is_empty() {
            state.filter.pop();
            changed = true;
        }
    }

    // Check escape to clear filter
    if keyboard_input.just_pressed(KeyCode::Escape) && !state.filter.is_empty() {
        state.filter.clear();
        changed = true;
    }

    // Process character typing
    for ev in char_events.read() {
        if ev.state.is_pressed() {
            if let bevy::input::keyboard::Key::Character(ref ch) = ev.logical_key {
                if ch.len() == 1 && ch.chars().next().map_or(false, |c| !c.is_control()) {
                    state.filter.push_str(ch);
                    changed = true;
                }
            }
        }
    }

    if changed {
        if let Ok(mut search_text) = search_query.single_mut() {
            if state.filter.is_empty() {
                search_text.0 = "Search Hierarchy...".to_string();
            } else {
                search_text.0 = state.filter.clone();
            }
        }
    }
}


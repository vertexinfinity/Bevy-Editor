//! Inspector Dock Root, Header Draggable Title Bar, Clean & Feathers Debug View Spawning.
use std::path::Path;
use bevy::{ecs::message::{MessageReader, MessageWriter}, feathers::controls::{FeathersPane, FeathersSplitPane, FeathersSplitPaneHandle}, inspector::{details_panel::details_panel, entity_tree::{entity_tree_panel, InspectorUi}}, picking::{events::{PointerClick, PointerOut, PointerOver}, pointer::PointerButton}, prelude::*, scene::bsn, ui_widgets::split_pane_self_update};
use crate::scene::EditorInternal;
use crate::ui::theme::*;
use super::types::*;
use super::components_card::*;
use super::transform_card::*;
use super::material_card::*;
use super::light_card::*;
use super::active_scripts_card::*;
pub fn spawn_inspector_panel(mut commands: Commands, extensions: Res<super::extensions::InspectorSectionRegistry>) {
    let dock_root = commands
        .spawn((
            EditorInternal,
            InspectorDockRoot,
            Name::new("InspectorDock"),
            Node {
                position_type: PositionType::Absolute,
                top: px(36),
                right: px(0),
                bottom: px(28),
                width: px(380),
                max_width: percent(45.0),
                border: UiRect::left(px(1)),
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
            InspectorResizeHandle { origin_width: None },
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                bottom: px(0),
                width: px(6),
                ..default()
            },
            BackgroundColor(BORDER_SUBTLE),
            ChildOf(dock_root),
        ))
        .observe(|_: On<PointerOver>, mut color: Query<&mut BackgroundColor, With<InspectorResizeHandle>>| {
            if let Ok(mut color) = color.single_mut() { *color = BackgroundColor(BORDER_ACCENT); }
        })
        .observe(|_: On<PointerOut>, mut color: Query<&mut BackgroundColor, With<InspectorResizeHandle>>| {
            if let Ok(mut color) = color.single_mut() { *color = BackgroundColor(BORDER_SUBTLE); }
        })
        .observe(|_: On<bevy::picking::events::PointerDragStart>, state: Res<InspectorState>, mut handle: Query<&mut InspectorResizeHandle>| {
            if let Ok(mut handle) = handle.single_mut() { handle.origin_width = Some(state.width); }
        })
        .observe(|drag: On<bevy::picking::events::PointerDrag>, window: Query<&Window, With<bevy::window::PrimaryWindow>>, state: Res<InspectorState>, handle: Query<&InspectorResizeHandle>, mut dock: Query<&mut Node, With<InspectorDockRoot>>| {
            if state.is_collapsed { return; }
            let Ok(handle) = handle.single() else { return; };
            let Some(origin) = handle.origin_width else { return; };
            let direction = if state.dock_zone == PanelDockZone::Right { -1.0 } else { 1.0 };
            let max_width = window.single().map(|window| (window.width() * 0.45).min(640.0)).unwrap_or(640.0);
            let width = (origin + drag.distance.x * direction).clamp(260.0, max_width.max(260.0));
            if let Ok(mut node) = dock.single_mut() { node.width = px(width); }
        })
        .observe(|_: On<bevy::picking::events::PointerDragEnd>, mut state: ResMut<InspectorState>, mut handle: Query<&mut InspectorResizeHandle>, dock: Query<&Node, With<InspectorDockRoot>>| {
            if let Ok(node) = dock.single() { if let Val::Px(width) = node.width { state.width = width; } }
            if let Ok(mut handle) = handle.single_mut() { handle.origin_width = None; }
        });

    // 1. Inspector Header (Draggable title bar for docking / moving)
    let header = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(34),
                border: UiRect::bottom(px(1)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(8), px(4)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(dock_root),
        ))
        .observe(|_drag: On<bevy::picking::events::PointerDrag>,
                  window: Query<&Window, With<bevy::window::PrimaryWindow>>,
                  mut state: ResMut<InspectorState>,
                  mut dock_query: Query<(&mut Node, &mut BorderColor), With<InspectorDockRoot>>| {
            state.is_dragging = true;
            if let Ok(win) = window.single() {
                if let Some(cursor) = win.cursor_position() {
                    let win_width = win.width();
                    if let Ok((mut node, mut border)) = dock_query.single_mut() {
                        *border = BorderColor::all(BORDER_ACCENT);
                        if cursor.x < win_width * 0.35 {
                            // Snap to Left Dock Zone
                            node.left = px(0);
                            node.right = Val::Auto;
                            node.border = UiRect::right(px(1));
                            state.dock_zone = PanelDockZone::Left;
                        } else if cursor.x > win_width * 0.65 {
                            // Snap to Right Dock Zone
                            node.left = Val::Auto;
                            node.right = px(0);
                            node.border = UiRect::left(px(1));
                            state.dock_zone = PanelDockZone::Right;
                        } else {
                            // Free floating
                            let half_w = state.width * 0.5;
                            node.left = px((cursor.x - half_w).clamp(10.0, win_width - state.width - 10.0));
                            node.top = px((cursor.y - 15.0).clamp(40.0, win.height() - 200.0));
                            node.right = Val::Auto;
                            node.border = UiRect::all(px(1));
                            state.dock_zone = PanelDockZone::Floating;
                        }
                    }
                }
            }
        })
        .observe(|_drag_end: On<bevy::picking::events::PointerDragEnd>,
                  mut state: ResMut<InspectorState>,
                  mut dock_query: Query<(&mut Node, &mut BorderColor), With<InspectorDockRoot>>,
                  mut dock_label_query: Query<&mut Text, With<DockToggleLabel>>| {
            state.is_dragging = false;
            if let Ok((mut node, mut border)) = dock_query.single_mut() {
                *border = BorderColor::all(BORDER_SUBTLE);
                match state.dock_zone {
                    PanelDockZone::Right => {
                        node.left = Val::Auto;
                        node.right = px(0);
                        node.top = px(36);
                        node.bottom = px(28);
                        node.border = UiRect::left(px(1));
                        node.border_radius = BorderRadius::ZERO;
                    }
                    PanelDockZone::Left => {
                        node.left = px(0);
                        node.right = Val::Auto;
                        node.top = px(36);
                        node.bottom = px(28);
                        node.border = UiRect::right(px(1));
                        node.border_radius = BorderRadius::ZERO;
                    }
                    PanelDockZone::Floating => {
                        node.bottom = px(28);
                        node.border = UiRect::all(px(1));
                        node.border_radius = BorderRadius::all(px(6));
                    }
                }
            }
            if let Ok(mut label) = dock_label_query.single_mut() {
                crate::ui::set_text(&mut label, match state.dock_zone {
                    PanelDockZone::Right => "Dock Right".to_string(),
                    PanelDockZone::Left => "Dock Left".to_string(),
                    PanelDockZone::Floating => "Floating".to_string(),
                });
            }
            info!("Inspector docked to {:?}", state.dock_zone);
        })
        .id();

    // Header Title
    commands.spawn((
        EditorInternal,
        InspectorHeaderTitle,
        Pickable::IGNORE,
        Text::new("Inspector"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(header),
    ));

    // Header Action Buttons (Mode toggle & Dock collapse)
    let header_btns = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(header),
        ))
        .id();

    // Mode Toggle Button (Standard vs Debug)
    let mode_btn = commands
        .spawn((
            EditorInternal,
            InspectorModeBtn,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::ZERO,
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(header_btns),
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
        .observe(|click: On<PointerClick>, mut writer: MessageWriter<ToggleDebugModeMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(ToggleDebugModeMessage);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        ModeToggleLabel,
        Pickable::IGNORE,
        Text::new("Debug View: OFF"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(mode_btn),
    ));

    // Dock Drag Handle & Toggle Button
    let dock_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::ZERO,
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(header_btns),
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
        .observe(|_drag: On<bevy::picking::events::PointerDrag>,
                  window: Query<&Window, With<bevy::window::PrimaryWindow>>,
                  mut state: ResMut<InspectorState>,
                  mut dock_query: Query<(&mut Node, &mut BorderColor), With<InspectorDockRoot>>| {
            state.is_dragging = true;
            if let Ok(win) = window.single() {
                if let Some(cursor) = win.cursor_position() {
                    let win_width = win.width();
                    if let Ok((mut node, mut border)) = dock_query.single_mut() {
                        *border = BorderColor::all(BORDER_ACCENT);
                        if cursor.x < win_width * 0.35 {
                            // Snap to Left Dock Zone
                            node.left = px(0);
                            node.right = Val::Auto;
                            node.border = UiRect::right(px(1));
                            state.dock_zone = PanelDockZone::Left;
                        } else if cursor.x > win_width * 0.65 {
                            // Snap to Right Dock Zone
                            node.left = Val::Auto;
                            node.right = px(0);
                            node.border = UiRect::left(px(1));
                            state.dock_zone = PanelDockZone::Right;
                        } else {
                            // Free floating
                            let half_w = state.width * 0.5;
                            node.left = px((cursor.x - half_w).clamp(10.0, win_width - state.width - 10.0));
                            node.top = px((cursor.y - 15.0).clamp(40.0, win.height() - 200.0));
                            node.right = Val::Auto;
                            node.border = UiRect::all(px(1));
                            state.dock_zone = PanelDockZone::Floating;
                        }
                    }
                }
            }
        })
        .observe(|_drag_end: On<bevy::picking::events::PointerDragEnd>,
                  mut state: ResMut<InspectorState>,
                  mut dock_query: Query<(&mut Node, &mut BorderColor), With<InspectorDockRoot>>,
                  mut dock_label_query: Query<&mut Text, With<DockToggleLabel>>| {
            state.is_dragging = false;
            if let Ok((mut node, mut border)) = dock_query.single_mut() {
                *border = BorderColor::all(BORDER_SUBTLE);
                match state.dock_zone {
                    PanelDockZone::Right => {
                        node.left = Val::Auto;
                        node.right = px(0);
                        node.top = px(36);
                        node.bottom = px(28);
                        node.border = UiRect::left(px(1));
                        node.border_radius = BorderRadius::ZERO;
                    }
                    PanelDockZone::Left => {
                        node.left = px(0);
                        node.right = Val::Auto;
                        node.top = px(36);
                        node.bottom = px(28);
                        node.border = UiRect::right(px(1));
                        node.border_radius = BorderRadius::ZERO;
                    }
                    PanelDockZone::Floating => {
                        node.bottom = px(28);
                        node.border = UiRect::all(px(1));
                        node.border_radius = BorderRadius::all(px(6));
                    }
                }
            }
            if let Ok(mut label) = dock_label_query.single_mut() {
                crate::ui::set_text(&mut label, match state.dock_zone {
                    PanelDockZone::Right => "? Dock Right".to_string(),
                    PanelDockZone::Left => "? Dock Left".to_string(),
                    PanelDockZone::Floating => "? Floating".to_string(),
                });
            }
            info!("Inspector docked to {:?}", state.dock_zone);
        })
        .observe(|click: On<PointerClick>, mut writer: MessageWriter<ToggleDockMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(ToggleDockMessage);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        DockToggleLabel,
        Pickable::IGNORE,
        Text::new("? Dock Right"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(dock_btn),
    ));

    // 2. Clean Transform View Root (Default View)
    let clean_root = commands
        .spawn((
            EditorInternal,
            CleanViewRoot,
            bevy::ui_widgets::ScrollArea,
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(0),
                flex_basis: px(0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(10)),
                row_gap: px(0),
                overflow: Overflow::scroll_y(),
                display: Display::Flex,
                ..default()
            },
            ChildOf(dock_root),
        ))
        .id();

    spawn_clean_transform_panel(&mut commands, clean_root);
    extensions.spawn_sections(&mut commands, clean_root);

    // 3. Debug View Root (Full Feathers Reflection Inspector)
    let debug_root = commands
        .spawn((
            EditorInternal,
            DebugViewRoot,
            InspectorUi,
            Node {
                width: percent(100),
                height: percent(100),
                display: Display::None, // Hidden by default
                ..default()
            },
            ChildOf(dock_root),
        ))
        .id();

    // The Feathers reflection panels are spawned on demand by `manage_debug_panels`:
    // built while hidden they cost ~1,000 entities and rebuild on every selection.
    let _ = debug_root;
}


fn spawn_clean_transform_panel(commands: &mut Commands, parent: Entity) {
    // A. Selected Entity & Coordinate Space Header
    let ent_header = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(8),
                flex_shrink: 0.0,
                padding: UiRect::all(px(12)),
                border: UiRect::ZERO,
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        SelectedEntityLabel,
        Pickable::IGNORE,
        Text::new("Select an entity"),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(ent_header),
    ));

    // Coordinate Mode Button (Local vs Global)
    let coord_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::ZERO,
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(ent_header),
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
        .observe(|click: On<PointerClick>, mut writer: MessageWriter<ToggleCoordModeMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(ToggleCoordModeMessage);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        CoordModeLabel,
        Pickable::IGNORE,
        Text::new("Space: Local"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(coord_btn),
    ));

    // B. Transform Card
    let transform_card = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                padding: UiRect::all(SPACE_COMPACT),
                row_gap: SPACE_COMPACT,
                border: UiRect::ZERO,
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_CARD),
            ChildOf(parent),
        ))
        .id();

    let tf_header = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(transform_card),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Transform"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(tf_header),
    ));

    // Reset All Transform Button
    let reset_all_btn = commands
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
            ChildOf(tf_header),
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
        .observe(|click: On<PointerClick>, mut writer: MessageWriter<TransformNudgeAction>| {
            if click.button == PointerButton::Primary {
                writer.write(TransformNudgeAction::ResetPosition);
                writer.write(TransformNudgeAction::ResetRotation);
                writer.write(TransformNudgeAction::ResetScale);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Reset"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(reset_all_btn),
    ));

    spawn_transform_value_row(commands, transform_card, "Position", TransformProperty::Position);
    spawn_transform_value_row(commands, transform_card, "Rotation", TransformProperty::Rotation);
    spawn_transform_value_row(commands, transform_card, "Scale", TransformProperty::Scale);

    // C. Attached Game Components Card
    let comp_card = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                padding: UiRect::all(SPACE_COMPACT),
                row_gap: px(6),
                border: UiRect::ZERO,
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_CARD),
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Components"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(comp_card),
    ));

    commands.spawn((
        EditorInternal,
        ComponentsBadgesLabel,
        Pickable::IGNORE,
        Text::new("None"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(comp_card),
    ));

    // D. Script Properties Card (Standardized CardBuilder shell)
    let (script_card, _header_actions, script_body) = crate::ui::panel::CardBuilder::new("Script Component")
        .icon("⌘")
        .collapsible(true)
        .spawn_shell(commands, parent);

    commands.entity(script_card).insert(ScriptCardRoot);

    commands.entity(script_body).insert(super::script_card::ScriptFieldsRoot);

    commands.spawn((
        EditorInternal,
        ScriptPropsLabel,
        Pickable::IGNORE,
        Text::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(script_body),
    ));

    // E. Light & Shadows Card
    spawn_light_card(commands, parent);

    // F. PBR Material Card
    spawn_material_card(commands, parent);

    // G. Active Scripts Monitor Card
    super::relations_card::spawn_relations_card(commands, parent);
    spawn_active_scripts_card(commands, parent);

    // G. Add Component Section
    let add_comp_card = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                padding: UiRect::all(SPACE_COMPACT),
                row_gap: SPACE_COMPACT,
                border: UiRect::ZERO,
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_CARD),
            ChildOf(parent),
        ))
        .id();

    spawn_add_comp_toggle(commands, add_comp_card);

    let add_menu = commands
        .spawn((
            EditorInternal,
            InspectorAddMenu,
            Node {
                display: Display::None,
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                padding: UiRect::all(px(4)),
                border: UiRect::ZERO,
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(add_comp_card),
        ))
        .id();

    let bevy_section = spawn_add_tree_branch(commands, add_menu, "Bevy");
    spawn_add_comp_btn(commands, bevy_section, "Mesh: Cube", AddComponentKind::MeshCube);
    spawn_add_comp_btn(commands, bevy_section, "Mesh: Sphere", AddComponentKind::MeshSphere);
    spawn_add_comp_btn(commands, bevy_section, "Mesh: Plane", AddComponentKind::MeshPlane);
    spawn_add_comp_btn(commands, bevy_section, "Point Light", AddComponentKind::PointLight);
    spawn_add_comp_btn(commands, bevy_section, "Directional Light", AddComponentKind::DirectionalLight);
    spawn_add_comp_btn(commands, bevy_section, "Spot Light", AddComponentKind::SpotLight);
    spawn_add_comp_btn(commands, bevy_section, "3D Camera", AddComponentKind::Camera3d);

    let user_section = spawn_add_tree_branch(commands, add_menu, "User");
    let user_components = spawn_add_tree_branch(commands, user_section, "User Components");
    spawn_add_comp_btn(commands, user_components, "+ New Script...", AddComponentKind::NewScript);
    spawn_add_comp_btn(commands, user_components, "Script Component (default)", AddComponentKind::Script);
    spawn_add_comp_btn(commands, user_components, "Spin Animation", AddComponentKind::SpinAnimation);
    spawn_add_comp_btn(commands, user_components, "Velocity", AddComponentKind::Velocity);
    spawn_add_comp_btn(commands, user_components, "Gameplay Tag", AddComponentKind::GameplayTag);

    let (script_directories, root_scripts) = find_script_tree(Path::new("assets"));
    if root_scripts.is_empty() && script_directories.is_empty() {
        spawn_add_tree_label(commands, user_section, "No scripts found under assets/");
    } else {
        spawn_script_entries(commands, user_section, &root_scripts);
        for directory in &script_directories {
            spawn_script_directory(commands, user_section, directory);
        }
    }
}

pub fn handle_inspector_toggle_messages(
    mut debug_msgs: MessageReader<ToggleDebugModeMessage>,
    mut dock_msgs: MessageReader<ToggleDockMessage>,
    mut coord_msgs: MessageReader<ToggleCoordModeMessage>,
    mut state: ResMut<InspectorState>,
    mut dock_query: Query<&mut Node, With<InspectorDockRoot>>,
    mut resize_handle_query: Query<&mut Node, (With<InspectorResizeHandle>, Without<InspectorDockRoot>, Without<CleanViewRoot>, Without<DebugViewRoot>, Without<InspectorHeaderTitle>, Without<InspectorModeBtn>)>,
    mut clean_query: Query<&mut Node, (With<CleanViewRoot>, Without<InspectorDockRoot>, Without<DebugViewRoot>, Without<InspectorHeaderTitle>, Without<InspectorModeBtn>)>,
    mut debug_query: Query<&mut Node, (With<DebugViewRoot>, Without<InspectorDockRoot>, Without<CleanViewRoot>, Without<InspectorHeaderTitle>, Without<InspectorModeBtn>)>,
    mut title_query: Query<&mut Node, (With<InspectorHeaderTitle>, Without<InspectorDockRoot>, Without<CleanViewRoot>, Without<DebugViewRoot>, Without<InspectorModeBtn>)>,
    mut mode_btn_query: Query<&mut Node, (With<InspectorModeBtn>, Without<InspectorDockRoot>, Without<CleanViewRoot>, Without<DebugViewRoot>, Without<InspectorHeaderTitle>)>,
    mut mode_label_query: Query<&mut Text, (With<ModeToggleLabel>, Without<DockToggleLabel>, Without<CoordModeLabel>)>,
    mut dock_label_query: Query<&mut Text, (With<DockToggleLabel>, Without<ModeToggleLabel>, Without<CoordModeLabel>)>,
    mut coord_label_query: Query<&mut Text, (With<CoordModeLabel>, Without<ModeToggleLabel>, Without<DockToggleLabel>)>,
) {
    for _ in debug_msgs.read() {
        state.is_debug_view = !state.is_debug_view;
    }
    for _ in dock_msgs.read() {
        state.is_collapsed = !state.is_collapsed;
    }
    for _ in coord_msgs.read() {
        state.is_global_coords = !state.is_global_coords;
    }

    if state.is_changed() {
        if let Ok(mut handle_node) = resize_handle_query.single_mut() {
            crate::ui::set_display(&mut handle_node, if state.is_collapsed { Display::None } else { Display::Flex });
            match state.dock_zone {
                PanelDockZone::Right => { handle_node.left = px(0); handle_node.right = Val::Auto; }
                PanelDockZone::Left => { handle_node.left = Val::Auto; handle_node.right = px(0); }
                PanelDockZone::Floating => { handle_node.left = px(0); handle_node.right = Val::Auto; }
            }
        }
        // Update dock root width and position
        if let Ok(mut dock_node) = dock_query.single_mut() {
            if state.is_collapsed {
                dock_node.width = px(34);
            } else {
                dock_node.width = px(state.width);
            }

            match state.dock_zone {
                PanelDockZone::Right => {
                    dock_node.left = Val::Auto;
                    dock_node.right = px(0);
                    dock_node.border = UiRect::left(px(1));
                }
                PanelDockZone::Left => {
                    dock_node.left = px(0);
                    dock_node.right = Val::Auto;
                    dock_node.border = UiRect::right(px(1));
                }
                PanelDockZone::Floating => {}
            }
        }

        // Toggle header title and mode button visibility
        if let Ok(mut title_node) = title_query.single_mut() {
            crate::ui::set_display(&mut title_node, if state.is_collapsed { Display::None } else { Display::Flex });
        }
        if let Ok(mut mode_btn_node) = mode_btn_query.single_mut() {
            crate::ui::set_display(&mut mode_btn_node, if state.is_collapsed { Display::None } else { Display::Flex });
        }

        // Update clean view vs debug view visibility
        if let Ok(mut clean_node) = clean_query.single_mut() {
            if state.is_collapsed || state.is_debug_view {
                crate::ui::set_display(&mut clean_node, Display::None);
            } else {
                crate::ui::set_display(&mut clean_node, Display::Flex);
            }
        }

        if let Ok(mut debug_node) = debug_query.single_mut() {
            if state.is_collapsed || !state.is_debug_view {
                crate::ui::set_display(&mut debug_node, Display::None);
            } else {
                crate::ui::set_display(&mut debug_node, Display::Flex);
            }
        }

        // Update mode label
        if let Ok(mut mode_txt) = mode_label_query.single_mut() {
            crate::ui::set_text(&mut mode_txt, if state.is_debug_view {
                "Debug View: ON".to_string()
            } else {
                "Debug View: OFF".to_string()
            });
        }

        // Update dock label
        if let Ok(mut dock_txt) = dock_label_query.single_mut() {
            crate::ui::set_text(&mut dock_txt, if state.is_collapsed {
                match state.dock_zone {
                    PanelDockZone::Right => "<".to_string(),
                    PanelDockZone::Left => ">".to_string(),
                    PanelDockZone::Floating => "[F]".to_string(),
                }
            } else {
                match state.dock_zone {
                    PanelDockZone::Right => "Dock Right".to_string(),
                    PanelDockZone::Left => "Dock Left".to_string(),
                    PanelDockZone::Floating => "Floating".to_string(),
                }
            });
        }

        // Update coordinate space label
        if let Ok(mut coord_txt) = coord_label_query.single_mut() {
            crate::ui::set_text(&mut coord_txt, if state.is_global_coords {
                "Space: Global".to_string()
            } else {
                "Space: Local".to_string()
            });
        }
    }
}

/// Spawns the Feathers debug inspector only while Debug View is on, and frees it after.
pub fn manage_debug_panels(mut commands: Commands, state: Res<InspectorState>, roots: Query<(Entity, Option<&Children>), With<DebugViewRoot>>) {
    if !state.is_changed() {
        return;
    }
    let Ok((debug_root, children)) = roots.single() else { return };
    let has_panels = children.is_some_and(|c| !c.is_empty());
    if state.is_debug_view && !has_panels {
        // Feathers split pane: entity tree on the left, component details on the right
        let feathers_scene = bsn! {
            @FeathersSplitPane
            Node {
                width: percent(100),
                height: percent(100),
            }
            on(split_pane_self_update)
            Children [
                @FeathersPane { @size: 1.0, @min_size: 140.0 }
                Children [
                    @entity_tree_panel()
                    Node { width: Val::Auto }
                ]
                --
                @FeathersSplitPaneHandle
                --
                @FeathersPane { @size: 1.5, @min_size: 180.0 }
                Children [
                    @details_panel()
                    Node { width: Val::Auto }
                ]
            ]
        };
        commands.spawn_scene(feathers_scene).insert((
            EditorInternal,
            ChildOf(debug_root),
        ));
    } else if !state.is_debug_view && has_panels {
        for child in children.into_iter().flatten() {
            commands.entity(*child).despawn();
        }
    }
}

//! Streamlined Entity Inspector & Transform Panel with Debug View and Docking.
//!
//! Features:
//! - Default Clean View: Focused Transform panel (Position, Euler Rotation in degrees, Scale)
//! - Coordinate space toggle: Local vs Global coordinates
//! - Dockable / Collapsible panel (minimizes to side to expand 3D viewport)
//! - Debug View toggle: switches to full Feathers reflection inspector (all components/internals)
//! - Direct entity selection via 3D viewport clicking or entity quick-chips
//! - Interactive Transform nudging (X/Y/Z translation, rotation, scale)
//! - Add Component action buttons: ScriptComponent, SpinAnimation, Velocity, PointLight, GameplayTag

use std::{fs, path::{Path, PathBuf}};
use bevy::{
    ecs::message::{Message, MessageReader, MessageWriter},
    feathers::controls::{FeathersPane, FeathersSplitPane, FeathersSplitPaneHandle},
    inspector::{
        details_panel::details_panel,
        entity_tree::{entity_tree_panel, InspectorUi},
        InspectorSelection,
    },
    math::EulerRot,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
    scene::bsn,
    ui_widgets::split_pane_self_update,
};
use crate::scene::{components::*, EditorInternal};
use crate::ui::theme::*;

/// Docking zones for panels (Left, Right, or Floating).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelDockZone {
    #[default]
    Right,
    Left,
    Floating,
}

/// State managing the inspector's display mode, coordinate frame, and docking.
#[derive(Resource, Debug, Clone)]
pub struct InspectorState {
    pub is_debug_view: bool,
    pub is_global_coords: bool,
    pub is_collapsed: bool,
    pub dock_zone: PanelDockZone,
    pub width: f32,
    #[allow(dead_code)]
    pub is_dragging: bool,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            is_debug_view: false, // Default is clean Transform view
            is_global_coords: false, // Default is Local coordinates
            is_collapsed: false, // Default is expanded dock
            dock_zone: PanelDockZone::Right,
            width: 380.0,
            is_dragging: false,
        }
    }
}

// Markers for UI elements
#[derive(Component)]
pub struct InspectorDockRoot;

#[derive(Component)]
pub struct CleanViewRoot;

#[derive(Component)]
pub struct DebugViewRoot;

#[derive(Component)]
pub struct SelectedEntityLabel;

#[derive(Component)]
pub struct CoordModeLabel;

#[derive(Component)]
pub struct ModeToggleLabel;

#[derive(Component)]
pub struct DockToggleLabel;

#[derive(Component)]
pub struct InspectorHeaderTitle;

#[derive(Component)]
pub struct InspectorModeBtn;

#[derive(Component)]
pub struct ComponentsBadgesLabel;

#[derive(Component)]
pub struct TransformAxisValue {
    property: TransformProperty,
    axis: TransformAxis,
}

#[derive(Component)]
pub struct InspectorAddMenu;

#[derive(Component)]
pub struct ScriptPropsLabel;

// Messages for Inspector Actions
#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleDebugModeMessage;

#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleDockMessage;

#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleCoordModeMessage;

#[derive(Message, Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct SelectEntityMessage(pub Entity);

#[derive(Message, Clone, Copy, Debug)]
pub enum TransformNudgeAction {
    AdjustAxis {
        property: TransformProperty,
        axis: TransformAxis,
        delta: f32,
    },
    TranslateX(f32),
    TranslateY(f32),
    TranslateZ(f32),
    RotateX(f32),
    RotateY(f32),
    #[allow(dead_code)]
    RotateZ(f32),
    ScaleUniform(f32),
    ResetPosition,
    ResetRotation,
    ResetScale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformProperty {
    Position,
    Rotation,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformAxis {
    X,
    Y,
    Z,
}

#[derive(Message, Clone, Debug)]
pub enum AddComponentKind {
    Script,
    ScriptAsset(String),
    MeshCube,
    MeshSphere,
    MeshPlane,
    SpinAnimation,
    Velocity,
    PointLight,
    DirectionalLight,
    SpotLight,
    Camera3d,
    GameplayTag,
}

struct ScriptDirectory {
    name: String,
    scripts: Vec<(String, String)>,
    directories: Vec<ScriptDirectory>,
}

#[derive(Message, Clone, Debug)]
pub enum ScriptPropAction {
    NudgeFloat { key: String, delta: f32 },
    ToggleBool { key: String },
}

/// Spawns the entire inspector dock including clean and debug views.
pub fn spawn_inspector_panel(mut commands: Commands) {
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
            BackgroundColor(BG_CARD),
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
                label.0 = match state.dock_zone {
                    PanelDockZone::Right => "Dock Right".to_string(),
                    PanelDockZone::Left => "Dock Left".to_string(),
                    PanelDockZone::Floating => "Floating".to_string(),
                };
            }
            info!("Inspector docked to {:?}", state.dock_zone);
        })
        .id();

    // Header Title
    commands.spawn((
        EditorInternal,
        InspectorHeaderTitle,
        Pickable::IGNORE,
        Text::new("Inspector ⠿"),
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
                border: UiRect::all(px(1)),
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
                border: UiRect::all(px(1)),
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
                label.0 = match state.dock_zone {
                    PanelDockZone::Right => "⇲ Dock Right".to_string(),
                    PanelDockZone::Left => "⇱ Dock Left".to_string(),
                    PanelDockZone::Floating => "⧉ Floating".to_string(),
                };
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
        Text::new("⇲ Dock Right"),
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
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(10)),
                row_gap: px(10),
                overflow: Overflow::clip_y(),
                display: Display::Flex,
                ..default()
            },
            ChildOf(dock_root),
        ))
        .id();

    spawn_clean_transform_panel(&mut commands, clean_root);

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

    // Spawn Feathers split pane inside debug root
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
}

fn spawn_clean_transform_panel(commands: &mut Commands, parent: Entity) {
    // A. Selected Entity & Coordinate Space Header
    let ent_header = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(6)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        SelectedEntityLabel,
        Pickable::IGNORE,
        Text::new("Selected: None (Click object in scene)"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(ent_header),
    ));

    // Coordinate Mode Button (Local vs Global)
    let coord_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
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
                padding: UiRect::all(SPACE_COMPACT),
                row_gap: SPACE_COMPACT,
                border: UiRect::all(px(1)),
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_CARD),
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
                border: UiRect::all(px(1)),
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
        Text::new("↺ Reset"),
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
                padding: UiRect::all(SPACE_COMPACT),
                row_gap: px(6),
                border: UiRect::all(px(1)),
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_CARD),
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

    // D. Script Properties Card (Interactive Game Script Tuning)
    let script_card = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(SPACE_COMPACT),
                row_gap: px(6),
                border: UiRect::all(px(1)),
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_CARD),
            ChildOf(parent),
        ))
        .id();

    let script_header = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(script_card),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Script Properties 📜"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(script_header),
    ));

    let script_nudges = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                ..default()
            },
            ChildOf(script_header),
        ))
        .id();

    spawn_script_prop_btn(commands, script_nudges, "Speed -1", ScriptPropAction::NudgeFloat { key: "move_speed".to_string(), delta: -1.0 });
    spawn_script_prop_btn(commands, script_nudges, "Speed +1", ScriptPropAction::NudgeFloat { key: "move_speed".to_string(), delta: 1.0 });
    spawn_script_prop_btn(commands, script_nudges, "Jump +2", ScriptPropAction::NudgeFloat { key: "jump_force".to_string(), delta: 2.0 });
    spawn_script_prop_btn(commands, script_nudges, "Toggle Active", ScriptPropAction::ToggleBool { key: "is_active".to_string() });

    commands.spawn((
        EditorInternal,
        ScriptPropsLabel,
        Pickable::IGNORE,
        Text::new("No ScriptComponent on selected entity"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(script_card),
    ));

    // E. Add Component Section
    let add_comp_card = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(SPACE_COMPACT),
                row_gap: SPACE_COMPACT,
                border: UiRect::all(px(1)),
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_CARD),
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
                border: UiRect::all(px(1)),
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

fn spawn_add_tree_branch(commands: &mut Commands, parent: Entity, label: &str) -> Entity {
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

fn spawn_add_tree_label(commands: &mut Commands, parent: Entity, label: &str) -> Entity {
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

fn spawn_script_directory(commands: &mut Commands, parent: Entity, directory: &ScriptDirectory) {
    let contents = spawn_add_tree_branch(commands, parent, &directory.name);
    spawn_script_entries(commands, contents, &directory.scripts);
    for child in &directory.directories {
        spawn_script_directory(commands, contents, child);
    }
}

fn spawn_script_entries(commands: &mut Commands, parent: Entity, scripts: &[(String, String)]) {
    for (name, path) in scripts {
        spawn_add_comp_btn(
            commands,
            parent,
            name,
            AddComponentKind::ScriptAsset(path.clone()),
        );
    }
}

fn spawn_transform_value_row(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    property: TransformProperty,
) {
    let row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(row),
    ));

    let axes = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(row),
        ))
        .id();

    for (axis, label) in [(TransformAxis::X, "X"), (TransformAxis::Y, "Y"), (TransformAxis::Z, "Z")] {
        let axis_cell = commands
            .spawn((
                EditorInternal,
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(3),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(axes),
            ))
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(label),
            TextFont {
                font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
                ..default()
            },
            TextColor(TEXT_MUTED),
            ChildOf(axis_cell),
        ));

        spawn_transform_axis_widget(commands, axis_cell, property, axis);
    }
}

fn spawn_transform_axis_widget(
    commands: &mut Commands,
    parent: Entity,
    property: TransformProperty,
    axis: TransformAxis,
) {
    let step = match property {
        TransformProperty::Position | TransformProperty::Scale => 0.1,
        TransformProperty::Rotation => 1.0,
    };

    let widget = commands
        .spawn((
            EditorInternal,
            Node {
                position_type: PositionType::Relative,
                width: px(78),
                height: px(24),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    let value_field = commands
        .spawn((
            EditorInternal,
            Node {
                width: px(50),
                height: px(22),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(widget),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        TransformAxisValue { property, axis },
        Pickable::IGNORE,
        Text::new("0.00"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(value_field),
    ));

    let left_arrow = spawn_transform_arrow(commands, widget, "‹", property, axis, -step, true);
    let right_arrow = spawn_transform_arrow(commands, widget, "›", property, axis, step, false);

    commands.entity(widget)
        .observe(move |_over: On<PointerOver>, mut nodes: Query<&mut Node>| {
            for arrow in [left_arrow, right_arrow] {
                if let Ok(mut node) = nodes.get_mut(arrow) {
                    node.display = Display::Flex;
                }
            }
        })
        .observe(move |_out: On<PointerOut>, mut nodes: Query<&mut Node>| {
            for arrow in [left_arrow, right_arrow] {
                if let Ok(mut node) = nodes.get_mut(arrow) {
                    node.display = Display::None;
                }
            }
        })
        .observe(move |drag: On<bevy::picking::events::PointerDrag>, mut writer: MessageWriter<TransformNudgeAction>| {
            if drag.button == PointerButton::Primary && drag.delta.x != 0.0 {
                writer.write(TransformNudgeAction::AdjustAxis {
                    property,
                    axis,
                    delta: drag.delta.x * step * 0.1,
                });
            }
        });
}

fn spawn_transform_arrow(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    property: TransformProperty,
    axis: TransformAxis,
    delta: f32,
    is_left: bool,
) -> Entity {
    let mut node = Node {
        display: Display::None,
        position_type: PositionType::Absolute,
        top: px(1),
        width: px(16),
        height: px(22),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    if is_left {
        node.left = px(0);
    } else {
        node.right = px(0);
    }

    let arrow = commands
        .spawn((
            EditorInternal,
            node,
            BackgroundColor(BG_HOVER),
            ChildOf(parent),
        ))
        .observe(move |mut click: On<PointerClick>, mut writer: MessageWriter<TransformNudgeAction>| {
            if click.button == PointerButton::Primary {
                click.propagate(false);
                writer.write(TransformNudgeAction::AdjustAxis { property, axis, delta });
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(arrow),
    ));

    arrow
}

fn spawn_add_comp_toggle(commands: &mut Commands, parent: Entity) {
    let button = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100),
                padding: UiRect::axes(px(8), px(5)),
                border: UiRect::all(px(1)),
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

fn spawn_add_comp_btn(
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

fn find_script_tree(root: &Path) -> (Vec<ScriptDirectory>, Vec<(String, String)>) {
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

fn spawn_script_prop_btn(commands: &mut Commands, parent: Entity, label: &str, action: ScriptPropAction) {
    let btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
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
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<ScriptPropAction>| {
            if click.button == PointerButton::Primary {
                writer.write(action.clone());
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
        TextColor(ACCENT_CYAN),
        ChildOf(btn),
    ));
}

/// Global observer: clicks on non-editor entities in the 3D scene select them in the inspector.
pub fn select_entity_on_viewport_click(
    click: On<PointerClick>,
    scene_entities: Query<(), With<crate::scene::SceneEntity>>,
    parent_query: Query<&ChildOf>,
    editor_internal: Query<(), With<EditorInternal>>,
    gizmo_marker: Query<(), With<bevy::gizmos::transform_gizmo::TransformGizmoMeshMarker>>,
    gizmo_root: Query<(), With<bevy::gizmos::transform_gizmo::TransformGizmoRoot>>,
    gizmo_state: Option<Res<bevy::gizmos::transform_gizmo::TransformGizmoState>>,
    mut selection: ResMut<InspectorSelection>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    // Do not select editor UI elements or transform gizmo handles
    if editor_internal.contains(click.entity)
        || gizmo_marker.contains(click.entity)
        || gizmo_root.contains(click.entity)
    {
        return;
    }

    // If transform gizmo is actively being dragged or was just dragged, do not change selection
    if let Some(state) = gizmo_state {
        if state.active || state.axis.is_some() {
            return;
        }
    }

    // Check if clicked entity is a child of gizmo root
    let mut curr_ancestor = click.entity;
    while let Ok(child_of) = parent_query.get(curr_ancestor) {
        let parent = child_of.parent();
        if gizmo_root.contains(parent) || editor_internal.contains(parent) {
            return;
        }
        curr_ancestor = parent;
    }

    // Resolve clicked entity or its ancestor marked with SceneEntity
    // Climb to the TOPMOST SceneEntity ancestor (glTF descendants are all tagged SceneEntity)
    let mut current = click.entity;
    let mut top: Option<Entity> = None;
    loop {
        if scene_entities.contains(current) {
            top = Some(current);
        }
        if let Ok(child_of) = parent_query.get(current) {
            current = child_of.parent();
        } else {
            break;
        }
    }
    if let Some(root) = top {
        info!("Selected 3D scene object: {:?}", root);
        selection.0 = Some(root);
        return;
    }

    // No fallback: bubbled clicks reach the Window entity and must never become the selection.
}

/// Updates inspector state toggles (Debug view mode, Dock collapse/expand, Local/Global coords).
pub fn handle_inspector_toggle_messages(
    mut debug_msgs: MessageReader<ToggleDebugModeMessage>,
    mut dock_msgs: MessageReader<ToggleDockMessage>,
    mut coord_msgs: MessageReader<ToggleCoordModeMessage>,
    mut state: ResMut<InspectorState>,
    mut dock_query: Query<&mut Node, With<InspectorDockRoot>>,
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
            title_node.display = if state.is_collapsed { Display::None } else { Display::Flex };
        }
        if let Ok(mut mode_btn_node) = mode_btn_query.single_mut() {
            mode_btn_node.display = if state.is_collapsed { Display::None } else { Display::Flex };
        }

        // Update clean view vs debug view visibility
        if let Ok(mut clean_node) = clean_query.single_mut() {
            if state.is_collapsed || state.is_debug_view {
                clean_node.display = Display::None;
            } else {
                clean_node.display = Display::Flex;
            }
        }

        if let Ok(mut debug_node) = debug_query.single_mut() {
            if state.is_collapsed || !state.is_debug_view {
                debug_node.display = Display::None;
            } else {
                debug_node.display = Display::Flex;
            }
        }

        // Update mode label
        if let Ok(mut mode_txt) = mode_label_query.single_mut() {
            mode_txt.0 = if state.is_debug_view {
                "Debug View: ON".to_string()
            } else {
                "Debug View: OFF".to_string()
            };
        }

        // Update dock label
        if let Ok(mut dock_txt) = dock_label_query.single_mut() {
            dock_txt.0 = if state.is_collapsed {
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
            };
        }

        // Update coordinate space label
        if let Ok(mut coord_txt) = coord_label_query.single_mut() {
            coord_txt.0 = if state.is_global_coords {
                "Space: Global".to_string()
            } else {
                "Space: Local".to_string()
            };
        }
    }
}

/// Executes nudge actions on the selected entity.
pub fn handle_transform_nudges(
    mut nudges: MessageReader<TransformNudgeAction>,
    selection: Res<InspectorSelection>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(selected) = selection.0 else {
        return;
    };

    let Ok(mut transform) = transforms.get_mut(selected) else {
        return;
    };

    for nudge in nudges.read() {
        match *nudge {
            TransformNudgeAction::AdjustAxis { property, axis, delta } => match property {
                TransformProperty::Position => match axis {
                    TransformAxis::X => transform.translation.x += delta,
                    TransformAxis::Y => transform.translation.y += delta,
                    TransformAxis::Z => transform.translation.z += delta,
                },
                TransformProperty::Rotation => match axis {
                    TransformAxis::X => transform.rotate_local_x(delta.to_radians()),
                    TransformAxis::Y => transform.rotate_local_y(delta.to_radians()),
                    TransformAxis::Z => transform.rotate_local_z(delta.to_radians()),
                },
                TransformProperty::Scale => match axis {
                    TransformAxis::X => transform.scale.x += delta,
                    TransformAxis::Y => transform.scale.y += delta,
                    TransformAxis::Z => transform.scale.z += delta,
                },
            },
            TransformNudgeAction::TranslateX(dx) => transform.translation.x += dx,
            TransformNudgeAction::TranslateY(dy) => transform.translation.y += dy,
            TransformNudgeAction::TranslateZ(dz) => transform.translation.z += dz,
            TransformNudgeAction::RotateX(deg) => transform.rotate_local_x(deg.to_radians()),
            TransformNudgeAction::RotateY(deg) => transform.rotate_local_y(deg.to_radians()),
            TransformNudgeAction::RotateZ(deg) => transform.rotate_local_z(deg.to_radians()),
            TransformNudgeAction::ScaleUniform(s) => transform.scale *= s,
            TransformNudgeAction::ResetPosition => transform.translation = Vec3::ZERO,
            TransformNudgeAction::ResetRotation => transform.rotation = Quat::IDENTITY,
            TransformNudgeAction::ResetScale => transform.scale = Vec3::ONE,
        }
    }
}

/// Adds requested components to the selected entity.
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

/// Handles live interactive tuning of ScriptComponent properties in inspector.
pub fn handle_script_prop_actions(
    mut actions: MessageReader<ScriptPropAction>,
    selection: Res<InspectorSelection>,
    mut scripts: Query<&mut ScriptComponent>,
) {
    let Some(selected) = selection.0 else { return };
    let Ok(mut script) = scripts.get_mut(selected) else { return };

    for action in actions.read() {
        match action {
            ScriptPropAction::NudgeFloat { key, delta } => {
                if let Some(prop) = script.properties.get_mut(key) {
                    if let ScriptPropertyVal::Float(val) = prop {
                        *val += delta;
                        info!("Updated script property {} to {:.2}", key, *val);
                    }
                }
            }
            ScriptPropAction::ToggleBool { key } => {
                if let Some(prop) = script.properties.get_mut(key) {
                    if let ScriptPropertyVal::Bool(val) = prop {
                        *val = !*val;
                        info!("Toggled script property {} to {}", key, *val);
                    }
                }
            }
        }
    }
}

/// Updates the clean transform inspector labels based on the currently selected entity.
fn set_transform_axis_values(
    axis_labels: &mut Query<(&TransformAxisValue, &mut Text), With<TransformAxisValue>>,
    position: Vec3,
    rotation: Vec3,
    scale: Vec3,
) {
    for (axis_value, mut text) in axis_labels.iter_mut() {
        let vector = match axis_value.property {
            TransformProperty::Position => position,
            TransformProperty::Rotation => rotation,
            TransformProperty::Scale => scale,
        };
        let value = match axis_value.axis {
            TransformAxis::X => vector.x,
            TransformAxis::Y => vector.y,
            TransformAxis::Z => vector.z,
        };
        text.0 = if axis_value.property == TransformProperty::Rotation {
            format!("{value:.1}°")
        } else {
            format!("{value:.2}")
        };
    }
}

pub fn update_clean_transform_labels(
    selection: Res<InspectorSelection>,
    state: Res<InspectorState>,
    entities: Query<(
        Option<&Name>,
        Option<&Transform>,
        Option<&GlobalTransform>,
        Option<&ScriptComponent>,
        Option<&SpinAnimation>,
        Option<&Velocity>,
        Option<&GameplayTag>,
        Option<&Mesh3d>,
        Option<&DirectionalLight>,
        Option<&PointLight>,
        Option<&Visibility>,
    )>,
    mut name_label: Query<&mut Text, (With<SelectedEntityLabel>, Without<TransformAxisValue>, Without<ComponentsBadgesLabel>, Without<ScriptPropsLabel>)>,
    mut axis_labels: Query<(&TransformAxisValue, &mut Text), With<TransformAxisValue>>,
    mut badges_label: Query<&mut Text, (With<ComponentsBadgesLabel>, Without<SelectedEntityLabel>, Without<TransformAxisValue>, Without<ScriptPropsLabel>)>,
    mut script_label: Query<&mut Text, (With<ScriptPropsLabel>, Without<SelectedEntityLabel>, Without<TransformAxisValue>, Without<ComponentsBadgesLabel>)>,
) {
    let Ok(mut name_txt) = name_label.single_mut() else { return };
    let Ok(mut badges_txt) = badges_label.single_mut() else { return };
    let Ok(mut script_txt) = script_label.single_mut() else { return };

    let Some(selected) = selection.0 else {
        name_txt.0 = "Selected: None (Click object in scene)".to_string();
        set_transform_axis_values(&mut axis_labels, Vec3::ZERO, Vec3::ZERO, Vec3::ONE);
        badges_txt.0 = "None".to_string();
        script_txt.0 = "No entity selected".to_string();
        return;
    };

    let Ok((name, transform, global_transform, script, spin, vel, tag, mesh, dir_light, pt_light, vis)) = entities.get(selected) else {
        name_txt.0 = format!("Selected: {:?} (Despawned)", selected);
        set_transform_axis_values(&mut axis_labels, Vec3::ZERO, Vec3::ZERO, Vec3::ONE);
        script_txt.0 = "Despawned".to_string();
        return;
    };

    let is_hidden = vis == Some(&Visibility::Hidden);
    let type_desc = if mesh.is_some() {
        "Mesh Prop"
    } else if dir_light.is_some() || pt_light.is_some() {
        "Light"
    } else {
        "Entity"
    };

    let base_name = if let Some(n) = name {
        n.as_str().to_string()
    } else {
        format!("{type_desc} #{}", selected.index())
    };

    name_txt.0 = if is_hidden {
        format!("Selected: {base_name} [Hidden]")
    } else {
        format!("Selected: {base_name}")
    };

    // Compute display position, rotation, scale
    let (pos, rot, scale) = if state.is_global_coords {
        if let Some(gt) = global_transform {
            let t = gt.compute_transform();
            let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);
            (t.translation, Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()), t.scale)
        } else {
            (Vec3::ZERO, Vec3::ZERO, Vec3::ONE)
        }
    } else if let Some(t) = transform {
        let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);
        (t.translation, Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()), t.scale)
    } else {
        (Vec3::ZERO, Vec3::ZERO, Vec3::ONE)
    };

    set_transform_axis_values(&mut axis_labels, pos, rot, scale);

    // Collect badges
    let mut badges = Vec::new();
    if transform.is_some() { badges.push("[Transform]"); }
    if mesh.is_some() { badges.push("[Mesh3d]"); }
    if dir_light.is_some() { badges.push("[DirectionalLight]"); }
    if pt_light.is_some() { badges.push("[PointLight]"); }
    if script.is_some() { badges.push("[Script]"); }
    if spin.is_some() { badges.push("[SpinAnimation]"); }
    if vel.is_some() { badges.push("[Velocity]"); }
    if tag.is_some() { badges.push("[GameplayTag]"); }

    badges_txt.0 = if badges.is_empty() {
        "None".to_string()
    } else {
        badges.join(" ")
    };

    // Display ScriptComponent properties if present
    if let Some(script_comp) = script {
        let mut lines = Vec::new();
        lines.push(format!("File: {}", script_comp.script_file));
        for (k, v) in &script_comp.properties {
            match v {
                ScriptPropertyVal::Float(f) => lines.push(format!("• {}: {:.2}", k, f)),
                ScriptPropertyVal::Int(i) => lines.push(format!("• {}: {}", k, i)),
                ScriptPropertyVal::Bool(b) => lines.push(format!("• {}: {}", k, b)),
                ScriptPropertyVal::String(s) => lines.push(format!("• {}: \"{}\"", k, s)),
                ScriptPropertyVal::Vec3(v) => lines.push(format!("• {}: ({:.1}, {:.1}, {:.1})", k, v.x, v.y, v.z)),
            }
        }
        script_txt.0 = lines.join("\n");
    } else {
        script_txt.0 = "No ScriptComponent attached".to_string();
    }
}

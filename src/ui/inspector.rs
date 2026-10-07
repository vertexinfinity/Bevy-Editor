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
pub struct PositionDisplayLabel;

#[derive(Component)]
pub struct RotationDisplayLabel;

#[derive(Component)]
pub struct ScaleDisplayLabel;

#[derive(Component)]
pub struct ComponentsBadgesLabel;

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

#[derive(Message, Clone, Copy, Debug)]
pub enum AddComponentKind {
    Script,
    SpinAnimation,
    Velocity,
    PointLight,
    GameplayTag,
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

    // 1. Position row & controls
    let pos_row = commands
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
        PositionDisplayLabel,
        Pickable::IGNORE,
        Text::new("Position: X: 0.00   Y: 0.00   Z: 0.00"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(pos_row),
    ));

    let pos_actions = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                ..default()
            },
            ChildOf(pos_row),
        ))
        .id();

    spawn_nudge_btn(commands, pos_actions, "-X", TransformNudgeAction::TranslateX(-0.5));
    spawn_nudge_btn(commands, pos_actions, "+X", TransformNudgeAction::TranslateX(0.5));
    spawn_nudge_btn(commands, pos_actions, "-Y", TransformNudgeAction::TranslateY(-0.5));
    spawn_nudge_btn(commands, pos_actions, "+Y", TransformNudgeAction::TranslateY(0.5));
    spawn_nudge_btn(commands, pos_actions, "-Z", TransformNudgeAction::TranslateZ(-0.5));
    spawn_nudge_btn(commands, pos_actions, "+Z", TransformNudgeAction::TranslateZ(0.5));
    spawn_nudge_btn(commands, pos_actions, "↺", TransformNudgeAction::ResetPosition);

    // 2. Rotation row & controls
    let rot_row = commands
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
        RotationDisplayLabel,
        Pickable::IGNORE,
        Text::new("Rotation: X: 0.0°   Y: 0.0°   Z: 0.0°"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(rot_row),
    ));

    let rot_actions = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                ..default()
            },
            ChildOf(rot_row),
        ))
        .id();

    spawn_nudge_btn(commands, rot_actions, "-Y", TransformNudgeAction::RotateY(-15.0));
    spawn_nudge_btn(commands, rot_actions, "+Y", TransformNudgeAction::RotateY(15.0));
    spawn_nudge_btn(commands, rot_actions, "-X", TransformNudgeAction::RotateX(-15.0));
    spawn_nudge_btn(commands, rot_actions, "+X", TransformNudgeAction::RotateX(15.0));
    spawn_nudge_btn(commands, rot_actions, "↺", TransformNudgeAction::ResetRotation);

    // 3. Scale row & controls
    let scale_row = commands
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
        ScaleDisplayLabel,
        Pickable::IGNORE,
        Text::new("Scale: X: 1.00   Y: 1.00   Z: 1.00"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(scale_row),
    ));

    let scale_actions = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                ..default()
            },
            ChildOf(scale_row),
        ))
        .id();

    spawn_nudge_btn(commands, scale_actions, "-0.1", TransformNudgeAction::ScaleUniform(0.9));
    spawn_nudge_btn(commands, scale_actions, "+0.1", TransformNudgeAction::ScaleUniform(1.1));
    spawn_nudge_btn(commands, scale_actions, "↺", TransformNudgeAction::ResetScale);

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

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("+ Add Component"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(add_comp_card),
    ));

    let add_btns_grid = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                row_gap: px(6),
                column_gap: px(6),
                ..default()
            },
            ChildOf(add_comp_card),
        ))
        .id();

    spawn_add_comp_btn(commands, add_btns_grid, "+ Mesh", AddComponentKind::Script);
    spawn_add_comp_btn(commands, add_btns_grid, "+ Light", AddComponentKind::PointLight);
    spawn_add_comp_btn(commands, add_btns_grid, "+ Script", AddComponentKind::Script);
    spawn_add_comp_btn(commands, add_btns_grid, "+ Spin Anim", AddComponentKind::SpinAnimation);
    spawn_add_comp_btn(commands, add_btns_grid, "+ Velocity", AddComponentKind::Velocity);
    spawn_add_comp_btn(commands, add_btns_grid, "+ Gameplay Tag", AddComponentKind::GameplayTag);
}

fn spawn_nudge_btn(commands: &mut Commands, parent: Entity, label: &str, action: TransformNudgeAction) {
    let btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
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
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<TransformNudgeAction>| {
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
            font_size: bevy::text::FontSize::Px(10.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(btn),
    ));
}

fn spawn_add_comp_btn(commands: &mut Commands, parent: Entity, label: &str, kind: AddComponentKind) {
    let btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(4)),
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
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<AddComponentKind>| {
            if click.button == PointerButton::Primary {
                writer.write(kind);
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
) {
    let Some(selected) = selection.0 else {
        return;
    };

    for action in actions.read() {
        match action {
            AddComponentKind::Script => {
                commands.entity(selected).insert(ScriptComponent::default());
                info!("Added ScriptComponent to {:?}", selected);
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
    mut name_label: Query<&mut Text, (With<SelectedEntityLabel>, Without<PositionDisplayLabel>, Without<RotationDisplayLabel>, Without<ScaleDisplayLabel>, Without<ComponentsBadgesLabel>, Without<ScriptPropsLabel>)>,
    mut pos_label: Query<&mut Text, (With<PositionDisplayLabel>, Without<SelectedEntityLabel>, Without<RotationDisplayLabel>, Without<ScaleDisplayLabel>, Without<ComponentsBadgesLabel>, Without<ScriptPropsLabel>)>,
    mut rot_label: Query<&mut Text, (With<RotationDisplayLabel>, Without<SelectedEntityLabel>, Without<PositionDisplayLabel>, Without<ScaleDisplayLabel>, Without<ComponentsBadgesLabel>, Without<ScriptPropsLabel>)>,
    mut scale_label: Query<&mut Text, (With<ScaleDisplayLabel>, Without<SelectedEntityLabel>, Without<PositionDisplayLabel>, Without<RotationDisplayLabel>, Without<ComponentsBadgesLabel>, Without<ScriptPropsLabel>)>,
    mut badges_label: Query<&mut Text, (With<ComponentsBadgesLabel>, Without<SelectedEntityLabel>, Without<PositionDisplayLabel>, Without<RotationDisplayLabel>, Without<ScaleDisplayLabel>, Without<ScriptPropsLabel>)>,
    mut script_label: Query<&mut Text, (With<ScriptPropsLabel>, Without<SelectedEntityLabel>, Without<PositionDisplayLabel>, Without<RotationDisplayLabel>, Without<ScaleDisplayLabel>, Without<ComponentsBadgesLabel>)>,
) {
    let Ok(mut name_txt) = name_label.single_mut() else { return };
    let Ok(mut pos_txt) = pos_label.single_mut() else { return };
    let Ok(mut rot_txt) = rot_label.single_mut() else { return };
    let Ok(mut scale_txt) = scale_label.single_mut() else { return };
    let Ok(mut badges_txt) = badges_label.single_mut() else { return };
    let Ok(mut script_txt) = script_label.single_mut() else { return };

    let Some(selected) = selection.0 else {
        name_txt.0 = "Selected: None (Click object in scene)".to_string();
        pos_txt.0 = "Position: X: 0.00   Y: 0.00   Z: 0.00".to_string();
        rot_txt.0 = "Rotation: X: 0.0°   Y: 0.0°   Z: 0.0°".to_string();
        scale_txt.0 = "Scale: X: 1.00   Y: 1.00   Z: 1.00".to_string();
        badges_txt.0 = "None".to_string();
        script_txt.0 = "No entity selected".to_string();
        return;
    };

    let Ok((name, transform, global_transform, script, spin, vel, tag, mesh, dir_light, pt_light, vis)) = entities.get(selected) else {
        name_txt.0 = format!("Selected: {:?} (Despawned)", selected);
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

    pos_txt.0 = format!("Position: X: {:.2}   Y: {:.2}   Z: {:.2}", pos.x, pos.y, pos.z);
    rot_txt.0 = format!("Rotation: X: {:.1}°   Y: {:.1}°   Z: {:.1}°", rot.x, rot.y, rot.z);
    scale_txt.0 = format!("Scale: X: {:.2}   Y: {:.2}   Z: {:.2}", scale.x, scale.y, scale.z);

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

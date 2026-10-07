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

/// State managing the inspector's display mode, coordinate frame, and docking.
#[derive(Resource, Debug, Clone)]
pub struct InspectorState {
    pub is_debug_view: bool,
    pub is_global_coords: bool,
    pub is_collapsed: bool,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            is_debug_view: false, // Default is clean Transform view
            is_global_coords: false, // Default is Local coordinates
            is_collapsed: false, // Default is expanded dock
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
pub struct PositionDisplayLabel;

#[derive(Component)]
pub struct RotationDisplayLabel;

#[derive(Component)]
pub struct ScaleDisplayLabel;

#[derive(Component)]
pub struct ComponentsBadgesLabel;

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
                width: px(400),
                border: UiRect::left(px(1)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.15)),
            BorderColor::all(Color::srgb(0.24, 0.24, 0.28)),
        ))
        .id();

    // 1. Inspector Header (Always visible, contains Dock & Mode toggles)
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
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.18)),
            BorderColor::all(Color::srgb(0.24, 0.24, 0.28)),
            ChildOf(dock_root),
        ))
        .id();

    // Header Title
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Inspector"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.95)),
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
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.38)),
            ChildOf(header_btns),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.28, 0.34, 0.46));
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.2, 0.2, 0.25));
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
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
        ChildOf(mode_btn),
    ));

    // Dock Collapse / Expand Button
    let dock_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.38)),
            ChildOf(header_btns),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.28, 0.34, 0.46));
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.2, 0.2, 0.25));
            }
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
        Text::new("Dock ▶"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
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
        FeathersSplitPane
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
            BackgroundColor(Color::srgb(0.16, 0.16, 0.2)),
            BorderColor::all(Color::srgb(0.26, 0.26, 0.32)),
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
        TextColor(Color::srgb(0.9, 0.85, 0.4)),
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
            BackgroundColor(Color::srgb(0.22, 0.22, 0.28)),
            BorderColor::all(Color::srgb(0.35, 0.35, 0.45)),
            ChildOf(ent_header),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.32, 0.38, 0.52));
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.22, 0.22, 0.28));
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
        TextColor(Color::srgb(0.7, 0.85, 1.0)),
        ChildOf(coord_btn),
    ));

    // B. Transform Card
    let transform_card = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(10)),
                row_gap: px(8),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.14, 0.17)),
            BorderColor::all(Color::srgb(0.25, 0.25, 0.3)),
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("📐 Transform"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
        ChildOf(transform_card),
    ));

    // 1. Position row & nudges
    commands.spawn((
        EditorInternal,
        PositionDisplayLabel,
        Pickable::IGNORE,
        Text::new("Position: X: 0.00   Y: 0.00   Z: 0.00"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.75, 0.75, 0.8)),
        ChildOf(transform_card),
    ));

    let pos_nudges = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                ..default()
            },
            ChildOf(transform_card),
        ))
        .id();

    spawn_nudge_btn(commands, pos_nudges, "-X", TransformNudgeAction::TranslateX(-0.5));
    spawn_nudge_btn(commands, pos_nudges, "+X", TransformNudgeAction::TranslateX(0.5));
    spawn_nudge_btn(commands, pos_nudges, "-Y", TransformNudgeAction::TranslateY(-0.5));
    spawn_nudge_btn(commands, pos_nudges, "+Y", TransformNudgeAction::TranslateY(0.5));
    spawn_nudge_btn(commands, pos_nudges, "-Z", TransformNudgeAction::TranslateZ(-0.5));
    spawn_nudge_btn(commands, pos_nudges, "+Z", TransformNudgeAction::TranslateZ(0.5));

    // 2. Rotation row & nudges
    commands.spawn((
        EditorInternal,
        RotationDisplayLabel,
        Pickable::IGNORE,
        Text::new("Rotation: X: 0.0°   Y: 0.0°   Z: 0.0°"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.75, 0.75, 0.8)),
        ChildOf(transform_card),
    ));

    let rot_nudges = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                ..default()
            },
            ChildOf(transform_card),
        ))
        .id();

    spawn_nudge_btn(commands, rot_nudges, "-Y 15°", TransformNudgeAction::RotateY(-15.0));
    spawn_nudge_btn(commands, rot_nudges, "+Y 15°", TransformNudgeAction::RotateY(15.0));
    spawn_nudge_btn(commands, rot_nudges, "-X 15°", TransformNudgeAction::RotateX(-15.0));
    spawn_nudge_btn(commands, rot_nudges, "+X 15°", TransformNudgeAction::RotateX(15.0));

    // 3. Scale row & nudges
    commands.spawn((
        EditorInternal,
        ScaleDisplayLabel,
        Pickable::IGNORE,
        Text::new("Scale: X: 1.00   Y: 1.00   Z: 1.00"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.75, 0.75, 0.8)),
        ChildOf(transform_card),
    ));

    let scale_nudges = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                ..default()
            },
            ChildOf(transform_card),
        ))
        .id();

    spawn_nudge_btn(commands, scale_nudges, "-0.1", TransformNudgeAction::ScaleUniform(0.9));
    spawn_nudge_btn(commands, scale_nudges, "+0.1", TransformNudgeAction::ScaleUniform(1.1));
    spawn_nudge_btn(commands, scale_nudges, "Reset 1.0", TransformNudgeAction::ResetScale);

    // C. Attached Components Card
    let comp_card = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(10)),
                row_gap: px(6),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.14, 0.17)),
            BorderColor::all(Color::srgb(0.25, 0.25, 0.3)),
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("📦 Attached Components:"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
        ChildOf(comp_card),
    ));

    commands.spawn((
        EditorInternal,
        ComponentsBadgesLabel,
        Pickable::IGNORE,
        Text::new("None"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.65, 0.8, 0.9)),
        ChildOf(comp_card),
    ));

    // D. Add Component Section
    let add_comp_card = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(10)),
                row_gap: px(8),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.14, 0.17)),
            BorderColor::all(Color::srgb(0.25, 0.25, 0.3)),
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("➕ Add Component:"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
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

    spawn_add_comp_btn(commands, add_btns_grid, "+ Script (Lua)", AddComponentKind::Script);
    spawn_add_comp_btn(commands, add_btns_grid, "+ Spin Animation", AddComponentKind::SpinAnimation);
    spawn_add_comp_btn(commands, add_btns_grid, "+ Velocity", AddComponentKind::Velocity);
    spawn_add_comp_btn(commands, add_btns_grid, "+ Point Light", AddComponentKind::PointLight);
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
            BackgroundColor(Color::srgb(0.2, 0.2, 0.24)),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.36)),
            ChildOf(parent),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.3, 0.36, 0.48));
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.2, 0.2, 0.24));
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
        TextColor(Color::srgb(0.85, 0.85, 0.9)),
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
            BackgroundColor(Color::srgb(0.18, 0.24, 0.32)),
            BorderColor::all(Color::srgb(0.28, 0.38, 0.5)),
            ChildOf(parent),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.25, 0.35, 0.55));
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.18, 0.24, 0.32));
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
        TextColor(Color::srgb(0.9, 0.95, 1.0)),
        ChildOf(btn),
    ));
}

/// Global observer: clicks on non-editor entities in the 3D scene select them in the inspector.
pub fn select_entity_on_viewport_click(
    click: On<PointerClick>,
    non_editor: Query<(), Without<EditorInternal>>,
    mut selection: ResMut<InspectorSelection>,
) {
    if click.button == PointerButton::Primary && non_editor.contains(click.entity) {
        selection.0 = Some(click.entity);
    }
}

/// Updates inspector state toggles (Debug view mode, Dock collapse/expand, Local/Global coords).
pub fn handle_inspector_toggle_messages(
    mut debug_msgs: MessageReader<ToggleDebugModeMessage>,
    mut dock_msgs: MessageReader<ToggleDockMessage>,
    mut coord_msgs: MessageReader<ToggleCoordModeMessage>,
    mut state: ResMut<InspectorState>,
    mut dock_query: Query<&mut Node, With<InspectorDockRoot>>,
    mut clean_query: Query<&mut Node, (With<CleanViewRoot>, Without<InspectorDockRoot>, Without<DebugViewRoot>)>,
    mut debug_query: Query<&mut Node, (With<DebugViewRoot>, Without<InspectorDockRoot>, Without<CleanViewRoot>)>,
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
        // Update dock root width
        if let Ok(mut dock_node) = dock_query.single_mut() {
            if state.is_collapsed {
                dock_node.width = px(40);
            } else {
                dock_node.width = px(400);
            }
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
                "◀ Dock".to_string()
            } else {
                "Dock ▶".to_string()
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
    )>,
    mut name_label: Query<&mut Text, (With<SelectedEntityLabel>, Without<PositionDisplayLabel>, Without<RotationDisplayLabel>, Without<ScaleDisplayLabel>, Without<ComponentsBadgesLabel>)>,
    mut pos_label: Query<&mut Text, (With<PositionDisplayLabel>, Without<SelectedEntityLabel>, Without<RotationDisplayLabel>, Without<ScaleDisplayLabel>, Without<ComponentsBadgesLabel>)>,
    mut rot_label: Query<&mut Text, (With<RotationDisplayLabel>, Without<SelectedEntityLabel>, Without<PositionDisplayLabel>, Without<ScaleDisplayLabel>, Without<ComponentsBadgesLabel>)>,
    mut scale_label: Query<&mut Text, (With<ScaleDisplayLabel>, Without<SelectedEntityLabel>, Without<PositionDisplayLabel>, Without<RotationDisplayLabel>, Without<ComponentsBadgesLabel>)>,
    mut badges_label: Query<&mut Text, (With<ComponentsBadgesLabel>, Without<SelectedEntityLabel>, Without<PositionDisplayLabel>, Without<RotationDisplayLabel>, Without<ScaleDisplayLabel>)>,
) {
    let Ok(mut name_txt) = name_label.single_mut() else { return };
    let Ok(mut pos_txt) = pos_label.single_mut() else { return };
    let Ok(mut rot_txt) = rot_label.single_mut() else { return };
    let Ok(mut scale_txt) = scale_label.single_mut() else { return };
    let Ok(mut badges_txt) = badges_label.single_mut() else { return };

    let Some(selected) = selection.0 else {
        name_txt.0 = "Selected: None (Click object in scene)".to_string();
        pos_txt.0 = "Position: X: 0.00   Y: 0.00   Z: 0.00".to_string();
        rot_txt.0 = "Rotation: X: 0.0°   Y: 0.0°   Z: 0.0°".to_string();
        scale_txt.0 = "Scale: X: 1.00   Y: 1.00   Z: 1.00".to_string();
        badges_txt.0 = "None".to_string();
        return;
    };

    let Ok((name, transform, global_transform, script, spin, vel, tag, mesh, dir_light, pt_light)) = entities.get(selected) else {
        name_txt.0 = format!("Selected: {:?} (Despawned)", selected);
        return;
    };

    let name_str = name.map(|n| n.as_str()).unwrap_or("Entity");
    name_txt.0 = format!("Selected: {} ({:?})", name_str, selected);

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
}

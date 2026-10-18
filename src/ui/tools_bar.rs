//! Viewport Floating Tool Selector Bar (Select, Move, Rotate, Scale).
//!
//! Spawns a sleek vertical floating HUD docked on the viewport upper-left.

use bevy::{
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::core::tools::{ActiveTool, ToolRegistry};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

#[derive(Component)]
pub struct ToolBtnMarker(pub &'static str);

#[derive(Component)]
pub struct ViewportTabMarker(pub &'static str);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportChipKind {
    Gizmos,
    Bounds,
    Grid,
    Wireframe,
}

#[derive(Component)]
pub struct ViewportChipLabel(pub ViewportChipKind);

pub fn spawn_tools_bar(mut commands: Commands, registry: Res<ToolRegistry>) {
    // 1. Sleek Scene / Game Tab Bar at upper left of viewport
    let tabs_root = commands
        .spawn((
            EditorInternal,
            Name::new("ViewportTabBar"),
            Node {
                position_type: PositionType::Absolute,
                top: px(44),
                left: px(256), // offset from hierarchy dock
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(4), px(2)),
                column_gap: px(4),
                border: UiRect::ZERO,
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_SHELL),
            BorderColor::all(BORDER_SUBTLE),
        ))
        .id();

    // Scene Tab (Active by default)
    let scene_tab = commands
        .spawn((
            EditorInternal,
            ViewportTabMarker("Scene"),
            Node {
                padding: UiRect::axes(px(10), px(3)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_ACCENT),
            ChildOf(tabs_root),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Scene"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            weight: FontWeight::SEMIBOLD,
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(scene_tab),
    ));

    // Game Tab
    let game_tab = commands
        .spawn((
            EditorInternal,
            ViewportTabMarker("Game"),
            Node {
                padding: UiRect::axes(px(10), px(3)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_SHELL),
            ChildOf(tabs_root),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Game"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(game_tab),
    ));

    // Viewport Quick Toggles (Gizmos, Bounds, Grid)
    let toggles = [
        (ViewportChipKind::Gizmos, "Gizmos: On", true),
        (ViewportChipKind::Bounds, "Bounds: On", true),
        (ViewportChipKind::Grid, "Grid: On", true),
        (ViewportChipKind::Wireframe, "Wireframe: Off", false),
    ];

    for (kind, label, active) in toggles {
        let chip = commands
            .spawn((
                EditorInternal,
                kind,
                Node {
                    padding: UiRect::axes(px(6), px(2)),
                    border_radius: radius_button(),
                    border: UiRect::ZERO,
                    margin: UiRect::left(px(4)),
                    ..default()
                },
                BackgroundColor(if active { ACCENT_CYAN_DIM } else { BG_INPUT }),
                BorderColor::all(if active { BORDER_ACCENT } else { BORDER_SUBTLE }),
                ChildOf(tabs_root),
            ))
            .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(BG_HOVER);
                }
            })
            .observe(move |trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>, settings: Res<crate::core::selection::ViewportGizmoSettings>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    let active = match kind {
                        ViewportChipKind::Gizmos => settings.show_gizmos,
                        ViewportChipKind::Bounds => settings.show_bounds,
                        ViewportChipKind::Grid => settings.show_grid,
                        ViewportChipKind::Wireframe => settings.show_wireframe,
                    };
                    *bg = BackgroundColor(if active { ACCENT_CYAN_DIM } else { BG_INPUT });
                }
            })
            .observe(move |click: On<PointerClick>, mut settings: ResMut<crate::core::selection::ViewportGizmoSettings>| {
                if click.button == PointerButton::Primary {
                    match kind {
                        ViewportChipKind::Gizmos => settings.show_gizmos = !settings.show_gizmos,
                        ViewportChipKind::Bounds => settings.show_bounds = !settings.show_bounds,
                        ViewportChipKind::Grid => settings.show_grid = !settings.show_grid,
                        ViewportChipKind::Wireframe => settings.show_wireframe = !settings.show_wireframe,
                    }
                }
            })
            .id();

        commands.spawn((
            EditorInternal,
            ViewportChipLabel(kind),
            Pickable::IGNORE,
            Text::new(label),
            TextFont {
                font_size: bevy::text::FontSize::Px(9.5),
                ..default()
            },
            TextColor(if active { ACCENT_CYAN } else { TEXT_SECONDARY }),
            ChildOf(chip),
        ));
    }

    // 2. Vertical Transform Tools Bar (Select, Move, Rotate, Scale)
    let bar_root = commands
        .spawn((
            EditorInternal,
            Name::new("ViewportToolsBar"),
            Node {
                position_type: PositionType::Absolute,
                top: px(82),
                left: px(256), // Cleanly offset from hierarchy panel
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(SPACE_MICRO),
                row_gap: SPACE_MICRO,
                border: UiRect::ZERO,
                border_radius: radius_panel(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
        ))
        .id();

    for tool in registry.list() {
        let id = tool.id();
        let icon = tool.icon();

        let btn = commands
            .spawn((
                EditorInternal,
                ToolBtnMarker(id),
                Node {
                    width: px(30),
                    height: px(30),
                    border: UiRect::ZERO,
                    border_radius: radius_button(),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(BG_INPUT),
                BorderColor::all(BORDER_SUBTLE),
                ChildOf(bar_root),
            ))
            .observe(move |trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(BG_HOVER);
                }
            })
            .observe(move |trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>, active: Res<ActiveTool>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    let col = if active.current_id == id {
                        ACCENT_CYAN_DIM
                    } else {
                        BG_INPUT
                    };
                    *bg = BackgroundColor(col);
                }
            })
            .observe(move |click: On<PointerClick>, mut active: ResMut<ActiveTool>| {
                if click.button == PointerButton::Primary {
                    active.current_id = id;
                    info!("Selected tool: {}", id);
                }
            })
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(icon),
            TextFont {
                font_size: bevy::text::FontSize::Px(13.0),
                ..default()
            },
            TextColor(TEXT_PRIMARY),
            ChildOf(btn),
        ));
    }
}

/// Updates background color & border of tool buttons based on active tool.
pub fn update_active_tool_highlight(
    active_tool: Res<ActiveTool>,
    mut query: Query<(&ToolBtnMarker, &mut BackgroundColor, &mut BorderColor)>,
) {
    if !active_tool.is_changed() {
        return;
    }

    for (marker, mut bg, mut border) in &mut query {
        if marker.0 == active_tool.current_id {
            *bg = BackgroundColor(ACCENT_CYAN_DIM);
            *border = BorderColor::all(BORDER_ACCENT);
        } else {
            *bg = BackgroundColor(BG_INPUT);
            *border = BorderColor::all(BORDER_SUBTLE);
        }
    }
}

/// Keeps viewport overlay chips synchronized with `ViewportGizmoSettings`.
pub fn update_viewport_chips(
    settings: Res<crate::core::selection::ViewportGizmoSettings>,
    mut chips: Query<(&ViewportChipKind, &mut BackgroundColor, &mut BorderColor)>,
    mut labels: Query<(&ViewportChipLabel, &mut Text, &mut TextColor)>,
) {
    if !settings.is_changed() {
        return;
    }

    for (kind, mut bg, mut border) in &mut chips {
        let active = match kind {
            ViewportChipKind::Gizmos => settings.show_gizmos,
            ViewportChipKind::Bounds => settings.show_bounds,
            ViewportChipKind::Grid => settings.show_grid,
                        ViewportChipKind::Wireframe => settings.show_wireframe,
        };
        *bg = BackgroundColor(if active { ACCENT_CYAN_DIM } else { BG_INPUT });
        *border = BorderColor::all(if active { BORDER_ACCENT } else { BORDER_SUBTLE });
    }

    for (label_marker, mut text, mut color) in &mut labels {
        let (name, active) = match label_marker.0 {
            ViewportChipKind::Gizmos => ("Gizmos", settings.show_gizmos),
            ViewportChipKind::Bounds => ("Bounds", settings.show_bounds),
            ViewportChipKind::Grid => ("Grid", settings.show_grid),
            ViewportChipKind::Wireframe => ("Wireframe", settings.show_wireframe),
        };
        text.0 = format!("{}: {}", name, if active { "On" } else { "Off" });
        *color = TextColor(if active { ACCENT_CYAN } else { TEXT_MUTED });
    }
}

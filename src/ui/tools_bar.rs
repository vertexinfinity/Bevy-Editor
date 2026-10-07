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

pub fn spawn_tools_bar(mut commands: Commands, registry: Res<ToolRegistry>) {
    let bar_root = commands
        .spawn((
            EditorInternal,
            Name::new("ViewportToolsBar"),
            Node {
                position_type: PositionType::Absolute,
                top: px(48),
                left: px(262), // Cleanly offset from hierarchy panel
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(4)),
                row_gap: px(4),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
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
                    width: px(32),
                    height: px(32),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4)),
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

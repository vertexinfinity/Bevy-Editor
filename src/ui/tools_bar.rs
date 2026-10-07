//! Viewport Tool Selector Bar (Select, Move, Rotate, Scale).
//!
//! Spawns floating vertical tool switcher on the upper-left of the viewport.

use bevy::{
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::core::tools::{ActiveTool, ToolRegistry};
use crate::scene::EditorInternal;

#[derive(Component)]
pub struct ToolBtnMarker(pub &'static str);

pub fn spawn_tools_bar(mut commands: Commands, registry: Res<ToolRegistry>) {
    let bar_root = commands
        .spawn((
            EditorInternal,
            Name::new("ViewportToolsBar"),
            Node {
                position_type: PositionType::Absolute,
                top: px(46),
                left: px(12),
                flex_direction: FlexDirection::Row,
                padding: UiRect::all(px(3)),
                column_gap: px(3),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.16)),
            BorderColor::all(Color::srgb(0.25, 0.25, 0.32)),
        ))
        .id();

    for tool in registry.list() {
        let id = tool.id();
        let label = format!("{} {}", tool.icon(), tool.name());

        let btn = commands
            .spawn((
                EditorInternal,
                ToolBtnMarker(id),
                Node {
                    padding: UiRect::axes(px(8), px(4)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(3)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                BorderColor::all(Color::srgb(0.28, 0.28, 0.35)),
                ChildOf(bar_root),
            ))
            .observe(move |trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(Color::srgb(0.28, 0.34, 0.48));
                }
            })
            .observe(move |trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>, active: Res<ActiveTool>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    let col = if active.current_id == id {
                        Color::srgb(0.25, 0.38, 0.6)
                    } else {
                        Color::srgb(0.18, 0.18, 0.22)
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
            Text::new(label),
            TextFont {
                font_size: bevy::text::FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::srgb(0.85, 0.85, 0.9)),
            ChildOf(btn),
        ));
    }
}

/// Updates background color of tool buttons based on active tool.
pub fn update_active_tool_highlight(
    active_tool: Res<ActiveTool>,
    mut query: Query<(&ToolBtnMarker, &mut BackgroundColor)>,
) {
    if !active_tool.is_changed() {
        return;
    }

    for (marker, mut bg) in &mut query {
        if marker.0 == active_tool.current_id {
            *bg = BackgroundColor(Color::srgb(0.25, 0.4, 0.65));
        } else {
            *bg = BackgroundColor(Color::srgb(0.18, 0.18, 0.22));
        }
    }
}

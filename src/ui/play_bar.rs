//! Play-In-Editor (PIE) Control Bar.
//!
//! Spawns centered Play (▶), Pause (⏸), Stop (⏹), and Step (⏭) controls
//! at the top of the viewport.

use bevy::{
    ecs::message::MessageWriter,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::core::play_mode::PlayModeAction;
use crate::scene::EditorInternal;

pub fn spawn_play_bar(mut commands: Commands) {
    let bar_root = commands
        .spawn((
            EditorInternal,
            Name::new("PlayControlBar"),
            Node {
                position_type: PositionType::Absolute,
                top: px(4),
                left: Val::Percent(42.0),
                height: px(28),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(6), px(2)),
                column_gap: px(4),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.14, 0.18)),
            BorderColor::all(Color::srgb(0.28, 0.28, 0.35)),
        ))
        .id();

    // Play Button (In-Viewport)
    spawn_play_button(&mut commands, bar_root, "▶ Play", PlayModeAction::Play, true);
    // Pop-out Window Button (Dedicated Window)
    spawn_play_button(&mut commands, bar_root, "⧉ Window", PlayModeAction::PlayInWindow, false);
    // Pause Button
    spawn_play_button(&mut commands, bar_root, "⏸ Pause", PlayModeAction::Pause, false);
    // Stop Button
    spawn_play_button(&mut commands, bar_root, "⏹ Stop", PlayModeAction::Stop, false);
    // Step Button
    spawn_play_button(&mut commands, bar_root, "⏭", PlayModeAction::Step, false);
}

fn spawn_play_button(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    action: PlayModeAction,
    is_play: bool,
) {
    let normal_bg = if is_play {
        Color::srgb(0.15, 0.35, 0.2)
    } else {
        Color::srgb(0.2, 0.2, 0.25)
    };

    let hover_bg = if is_play {
        Color::srgb(0.22, 0.48, 0.28)
    } else {
        Color::srgb(0.28, 0.28, 0.36)
    };

    let btn = commands
        .spawn((
            EditorInternal,
            action,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(normal_bg),
            BorderColor::all(Color::srgb(0.3, 0.3, 0.38)),
            ChildOf(parent),
        ))
        .observe(move |trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(hover_bg);
            }
        })
        .observe(move |trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(normal_bg);
            }
        })
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<PlayModeAction>| {
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
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.95, 0.9)),
        ChildOf(btn),
    ));
}

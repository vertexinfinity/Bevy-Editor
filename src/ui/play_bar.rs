//! Play-In-Editor (PIE) Control Bar for Bevylite.
//!
//! Spawns centered Play (▶), Pause (⏸), Stop (⏹), and Pop-out Window (⧉) controls
//! cleanly nested at the top toolbar center.

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
use crate::ui::theme::*;

/// Embeds the centered Play-In-Editor (PIE) controls inside a parent toolbar/menu bar.
pub fn spawn_play_controls(commands: &mut Commands, parent: Entity) -> Entity {
    let bar_root = commands
        .spawn((
            EditorInternal,
            Name::new("PlayControlBar"),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(4), px(2)),
                column_gap: px(4),
                border: UiRect::all(px(1)),
                border_radius: radius_card(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_CARD),
            ChildOf(parent),
        ))
        .id();

    // Play Button (In-Viewport) - Emerald Accent
    spawn_play_button(commands, bar_root, "▶ Play", PlayModeAction::Play, PlayBtnType::Play);
    // Pause Button
    spawn_play_button(commands, bar_root, "⏸", PlayModeAction::Pause, PlayBtnType::Normal);
    // Stop Button
    spawn_play_button(commands, bar_root, "⏹", PlayModeAction::Stop, PlayBtnType::Stop);
    // Step Button
    spawn_play_button(commands, bar_root, "⏭", PlayModeAction::Step, PlayBtnType::Normal);
    // Pop-out Window Button
    spawn_play_button(commands, bar_root, "⧉ Window", PlayModeAction::PlayInWindow, PlayBtnType::Normal);

    bar_root
}

pub fn spawn_play_bar(mut commands: Commands) {
    // Kept for backward compatibility if invoked independently
    let root = commands
        .spawn((
            EditorInternal,
            Node {
                position_type: PositionType::Absolute,
                top: px(4),
                left: Val::Percent(44.0),
                ..default()
            },
        ))
        .id();
    spawn_play_controls(&mut commands, root);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayBtnType {
    Play,
    Stop,
    Normal,
}

fn spawn_play_button(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    action: PlayModeAction,
    btn_type: PlayBtnType,
) {
    let (normal_bg, hover_bg, border_col) = match btn_type {
        PlayBtnType::Play => (ACCENT_EMERALD, ACCENT_EMERALD_HOVER, BORDER_SUBTLE),
        PlayBtnType::Stop => (BG_INPUT, ACCENT_CRIMSON, BORDER_SUBTLE),
        PlayBtnType::Normal => (BG_INPUT, BG_HOVER, BORDER_SUBTLE),
    };

    let btn = commands
        .spawn((
            EditorInternal,
            action,
            Node {
                height: HEIGHT_BTN_STANDARD,
                padding: UiRect::axes(px(10), px(2)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(normal_bg),
            BorderColor::all(border_col),
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
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(if btn_type == PlayBtnType::Play { Color::WHITE } else { TEXT_PRIMARY }),
        ChildOf(btn),
    ));
}

//! Active Scripts monitor panel listing all running scene scripts, parameters, and quick entity selection.

use bevy::{
    ecs::message::{MessageReader, MessageWriter},
    inspector::InspectorSelection,
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};
use crate::scene::{components::ScriptComponent, EditorInternal};
use crate::ui::panel::CardBuilder;
use crate::ui::theme::*;
use super::types::*;

#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleScriptEnabledAction(pub Entity);

pub(crate) fn spawn_active_scripts_card(commands: &mut Commands, parent: Entity) {
    let (card, _actions, body) = CardBuilder::new("Active Scripts Monitor")
        .icon("⚡")
        .collapsible(true)
        .spawn_shell(commands, parent);

    commands.entity(card).insert(ActiveScriptsCardRoot);

    commands.spawn((
        EditorInternal,
        ActiveScriptsContent,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            width: percent(100.0),
            ..default()
        },
        ChildOf(body),
    ));
}

/// System that continuously updates the Active Scripts card with all scene scripts.
pub fn update_active_scripts_panel(
    scripts_query: Query<(Entity, Option<&Name>, &ScriptComponent)>,
    content_query: Query<Entity, With<ActiveScriptsContent>>,
    mut commands: Commands,
    children_query: Query<&Children>,
) {
    let Ok(content_entity) = content_query.single() else { return };

    // Despawn old child rows
    if let Ok(children) = children_query.get(content_entity) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let mut scripts_list: Vec<(Entity, String, String, bool)> = Vec::new();
    for (entity, name, script) in &scripts_query {
        let ent_name = name.map(|n| n.as_str().to_string()).unwrap_or_else(|| format!("Entity #{}", entity.index()));
        scripts_list.push((entity, ent_name, script.script_file.clone(), script.enabled));
    }

    if scripts_list.is_empty() {
        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new("No active scripts attached to scene entities."),
            TextFont {
                font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
                ..default()
            },
            TextColor(TEXT_MUTED),
            ChildOf(content_entity),
        ));
        return;
    }

    for (entity, ent_name, script_path, is_enabled) in scripts_list {
        let row = commands
            .spawn((
                EditorInternal,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::axes(px(6), px(3)),
                    border: UiRect::all(px(1)),
                    border_radius: radius_button(),
                    ..default()
                },
                BackgroundColor(BG_INPUT),
                BorderColor::all(BORDER_SUBTLE),
                ChildOf(content_entity),
            ))
            .id();

        // Left info
        let info_col = commands
            .spawn((
                EditorInternal,
                Pickable::IGNORE,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(1),
                    ..default()
                },
                ChildOf(row),
            ))
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(format!("📜 {ent_name}")),
            TextFont {
                font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
                weight: FontWeight::MEDIUM,
                ..default()
            },
            TextColor(TEXT_PRIMARY),
            ChildOf(info_col),
        ));

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(script_path),
            TextFont {
                font_size: bevy::text::FontSize::Px(9.0),
                ..default()
            },
            TextColor(TEXT_MUTED),
            ChildOf(info_col),
        ));

        // Right actions
        let actions_col = commands
            .spawn((
                EditorInternal,
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(4),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(row),
            ))
            .id();

        // Status badge / Toggle button
        let toggle_btn = commands
            .spawn((
                EditorInternal,
                Node {
                    padding: UiRect::axes(px(5), px(2)),
                    border_radius: BorderRadius::all(px(3)),
                    ..default()
                },
                BackgroundColor(if is_enabled { ACCENT_EMERALD } else { ACCENT_CRIMSON }),
                ChildOf(actions_col),
            ))
            .observe(move |click: On<PointerClick>, mut writer: MessageWriter<ToggleScriptEnabledAction>| {
                if click.button == PointerButton::Primary {
                    writer.write(ToggleScriptEnabledAction(entity));
                }
            })
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(if is_enabled { "🟢 Active" } else { "🔴 Paused" }),
            TextFont {
                font_size: bevy::text::FontSize::Px(9.0),
                weight: FontWeight::SEMIBOLD,
                ..default()
            },
            TextColor(if is_enabled { ACCENT_EMERALD } else { Color::WHITE }),
            ChildOf(toggle_btn),
        ));

        // Select button
        let sel_btn = commands
            .spawn((
                EditorInternal,
                Node {
                    padding: UiRect::axes(px(6), px(2)),
                    border: UiRect::all(px(1)),
                    border_radius: radius_button(),
                    ..default()
                },
                BackgroundColor(BG_CARD),
                BorderColor::all(BORDER_SUBTLE),
                ChildOf(actions_col),
            ))
            .observe(move |click: On<PointerClick>, mut selection: ResMut<InspectorSelection>| {
                if click.button == PointerButton::Primary {
                    selection.0 = Some(entity);
                    info!("Selected script host entity {:?}", entity);
                }
            })
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new("Inspect"),
            TextFont {
                font_size: bevy::text::FontSize::Px(9.5),
                ..default()
            },
            TextColor(ACCENT_CYAN),
            ChildOf(sel_btn),
        ));
    }
}

pub fn handle_toggle_script_enabled(
    mut reader: MessageReader<ToggleScriptEnabledAction>,
    mut scripts: Query<&mut ScriptComponent>,
) {
    for action in reader.read() {
        if let Ok(mut script) = scripts.get_mut(action.0) {
            script.enabled = !script.enabled;
            info!("Toggled script enabled state on entity {:?} to {}", action.0, script.enabled);
        }
    }
}

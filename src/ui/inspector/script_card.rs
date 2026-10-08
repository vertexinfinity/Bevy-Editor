//! Script properties display, tuning buttons, and script mutation systems.
use bevy::{ecs::message::{MessageReader, MessageWriter}, picking::{events::{PointerClick, PointerOut, PointerOver}, pointer::PointerButton}, prelude::*, inspector::InspectorSelection};
use crate::scene::{components::*, EditorInternal};
use crate::ui::theme::*;
use super::types::*;
pub(crate) fn spawn_script_prop_btn(commands: &mut Commands, parent: Entity, label: &str, action: ScriptPropAction) {
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
//! Script properties display, tuning buttons, and script mutation systems.
use super::types::*;
use crate::scene::{EditorInternal, components::*};
use bevy::{ecs::message::MessageReader, inspector::InspectorSelection, prelude::*};

#[derive(Component)]
pub struct ScriptFieldsRoot;
#[derive(Component)]
pub(crate) struct ScriptFieldGroup;

/// Rebuild only when the selected script's schema changes; value edits keep fields stable.
pub(crate) fn rebuild_script_fields(
    selection: Res<InspectorSelection>,
    scripts: Query<&ScriptComponent>,
    roots: Query<(Entity, Option<&Children>), With<ScriptFieldsRoot>>,
    groups: Query<(), With<ScriptFieldGroup>>,
    mut previous: Local<Option<(Entity, Option<Entity>, String)>>,
    mut commands: Commands,
) {
    let Ok((body, children)) = roots.single() else {
        return;
    };
    let script = selection.0.and_then(|entity| scripts.get(entity).ok());
    let mut keys = script
        .map(|s| s.properties.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    keys.sort();
    let signature = script
        .map(|script| {
            format!(
                "{}{:?}{:?}",
                script.script_file,
                script.inspector_sections,
                keys.iter()
                    .map(|key| (key, std::mem::discriminant(&script.properties[key])))
                    .collect::<Vec<_>>()
            )
        })
        .unwrap_or_default();
    let next = (body, selection.0, signature);
    if previous.as_ref() == Some(&next) {
        return;
    }
    *previous = Some(next);
    if let Some(children) = children {
        for child in children.iter().filter(|child| groups.contains(*child)) {
            commands.entity(child).despawn();
        }
    }
    let Some(script) = script else {
        return;
    };
    let group = commands
        .spawn((
            EditorInternal,
            ScriptFieldGroup,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            },
            ChildOf(body),
        ))
        .id();
    let mut rendered = std::collections::HashSet::new();
    for section in &script.inspector_sections {
        let (_, _, section_body) =
            crate::ui::panel::CardBuilder::new(&section.title).spawn_shell(&mut commands, group);
        for key in &section.property_keys {
            if !rendered.insert(key.clone()) {
                continue;
            }
            if let Some(value) = script.properties.get(key) {
                spawn_script_field(&mut commands, section_body, key, value);
            }
        }
    }
    for key in keys {
        if !rendered.contains(&key) {
            spawn_script_field(&mut commands, group, &key, &script.properties[&key]);
        }
    }
    super::property_editor::spawn_property_help(&mut commands, group);
}
fn spawn_script_field(
    commands: &mut Commands,
    parent: Entity,
    key: &str,
    value: &ScriptPropertyVal,
) {
    use super::property_editor::{PropertyPath, spawn_property_field};
    let label = key.replace('_', " ");
    if matches!(value, ScriptPropertyVal::Vec3(_)) {
        for (axis, name) in ["X", "Y", "Z"].iter().enumerate() {
            spawn_property_field(
                commands,
                parent,
                &format!("{label} {name}"),
                PropertyPath::Script(key.into(), Some(axis)),
            );
        }
    } else {
        spawn_property_field(
            commands,
            parent,
            &label,
            PropertyPath::Script(key.into(), None),
        );
    }
}

/// Formats a script's declarative inspector sections. The schema contains only data,
/// keeping script runtimes independent from Bevy UI while allowing scripts/plugins to
/// group exposed properties into useful cards/sections.
pub fn format_script_inspector(script: &ScriptComponent) -> String {
    let mut output = vec![format!("File: {}", script.script_file)];
    let mut rendered = std::collections::HashSet::new();
    for section in &script.inspector_sections {
        let mut rows = Vec::new();
        for key in &section.property_keys {
            if let Some(value) = script.properties.get(key) {
                rendered.insert(key.as_str());
                rows.push(format_script_property(key, value));
            }
        }
        if !rows.is_empty() {
            output.push(format!("{}\n{}", section.title, rows.join("\n")));
        }
    }
    let mut ungrouped = script
        .properties
        .iter()
        .filter(|(key, _)| !rendered.contains(key.as_str()))
        .map(|(key, value)| format_script_property(key, value))
        .collect::<Vec<_>>();
    ungrouped.sort();
    if !ungrouped.is_empty() {
        output.push(format!("Properties\n{}", ungrouped.join("\n")));
    }
    let mut references = script.entity_refs.iter().collect::<Vec<_>>();
    references.sort_by(|a, b| a.0.cmp(b.0));
    if !references.is_empty() {
        output.push(format!(
            "Entity References\n{}",
            references
                .iter()
                .map(|(key, entity)| format!("• {key}: {entity:?}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    output.join("\n\n")
}

fn format_script_property(key: &str, value: &ScriptPropertyVal) -> String {
    match value {
        ScriptPropertyVal::Float(value) => format!("• {key}: {value:.2}"),
        ScriptPropertyVal::Int(value) => format!("• {key}: {value}"),
        ScriptPropertyVal::Bool(value) => format!("• {key}: {value}"),
        ScriptPropertyVal::String(value) => format!("• {key}: \"{value}\""),
        ScriptPropertyVal::Vec3(value) => {
            format!("• {key}: ({:.2}, {:.2}, {:.2})", value.x, value.y, value.z)
        }
    }
}
pub fn handle_script_prop_actions(
    mut actions: MessageReader<ScriptPropAction>,
    selection: Res<InspectorSelection>,
    mut scripts: Query<&mut ScriptComponent>,
) {
    if actions.is_empty() {
        return;
    }
    let Some(selected) = selection.0 else { return };
    let Ok(mut script) = scripts.get_mut(selected) else {
        return;
    };

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

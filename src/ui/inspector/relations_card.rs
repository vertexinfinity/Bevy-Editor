//! Navigable ECS relationships for the selected scene entity.
use bevy::{inspector::InspectorSelection, picking::{events::PointerClick, pointer::PointerButton}, prelude::*};
use crate::scene::{components::ScriptComponent, EditorInternal, SceneEntity};
use crate::ui::{panel::CardBuilder, theme::*};

#[derive(Component)]
pub struct RelationsContent;

pub fn spawn_relations_card(commands: &mut Commands, parent: Entity) {
    let (_, _, body) = CardBuilder::new("Relations").spawn_shell(commands, parent);
    commands.entity(body).insert(RelationsContent);
}

pub fn update_relations_card(
    selection: Res<InspectorSelection>,
    entities: Query<(Entity, Option<&Name>, Option<&ChildOf>, Option<&ScriptComponent>), With<SceneEntity>>,
    names: Query<&Name>,
    content: Query<(Entity, Option<&Children>), With<RelationsContent>>,
    mut previous: Local<Option<(Entity, Option<Entity>, Vec<(String, Option<Entity>)>)>>,
    mut commands: Commands,
) {
    let Ok((body, children)) = content.single() else { return; };
    let mut links = Vec::new();
    let describe = |relation: &str, target: Entity| {
        let name = names.get(target).map(|name| name.as_str().to_string()).unwrap_or_else(|_| format!("{target:?}"));
        (format!("{relation}  ·  {name}"), Some(target))
    };
    if let Some(selected) = selection.0 {
        if let Ok((_, _, parent, script)) = entities.get(selected) {
            if let Some(parent) = parent { links.push(describe("Parent", parent.parent())); }
            if let Some(script) = script {
                for (key, target) in &script.entity_refs { links.push(describe(key, *target)); }
            }
            for (entity, _, parent, script) in &entities {
                if parent.is_some_and(|parent| parent.parent() == selected) { links.push(describe("Child", entity)); }
                if let Some(script) = script {
                    for (key, target) in &script.entity_refs {
                        if *target == selected { links.push(describe(&format!("Referenced by ({key})"), entity)); }
                    }
                }
            }
        }
    }
    links.sort_by(|a, b| a.0.cmp(&b.0));
    if links.is_empty() { links.push((if selection.0.is_some() { "No entity relationships" } else { "Select an entity to inspect its relationships" }.to_string(), None)); }
    if previous.as_ref().is_some_and(|old| *old == (body, selection.0, links.clone())) { return; }
    *previous = Some((body, selection.0, links.clone()));
    if let Some(children) = children { for child in children.iter() { commands.entity(child).despawn(); } }
    for (label, target) in links {
        let row = commands.spawn((EditorInternal, Node {
            padding: UiRect::all(px(8)), min_width: px(0), border_radius: radius_button(), ..default()
        }, BackgroundColor(BG_INPUT), ChildOf(body))).id();
        if let Some(target) = target {
            commands.entity(row).observe(move |mut click: On<PointerClick>, mut selection: ResMut<InspectorSelection>, entities: Query<Entity>| {
                if click.button == PointerButton::Primary && entities.contains(target) { click.propagate(false); selection.0 = Some(target); }
            });
        }
        commands.spawn((EditorInternal, Pickable::IGNORE, Text::new(label), TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default()
        }, TextColor(if target.is_some() { TEXT_PRIMARY } else { TEXT_MUTED }), ChildOf(row)));
    }
}

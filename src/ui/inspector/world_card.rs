//! World / Environment card, shown in the Inspector when nothing is selected.
//!
//! Rows are generated from the reflected, annotated [`WorldEnvironment`] component, so new
//! settings appear here (and over RPC) just by adding a field.
use bevy::{inspector::InspectorSelection, prelude::*};

use super::reflect_card::spawn_component_section;
use crate::core::reflect_fields;
use crate::scene::environment::WorldSettingsEntity;

#[derive(Component)]
pub struct WorldCardBody;

pub fn spawn_world_card(commands: &mut Commands, body: Entity) {
    commands.entity(body).insert(WorldCardBody);
}

/// Shows the card when the selection is empty and builds its rows once.
pub fn update_world_card(world: &mut World, mut built: Local<bool>) {
    let Some(body) = world.query_filtered::<Entity, With<WorldCardBody>>().iter(world).next() else {
        *built = false;
        return;
    };
    let Some(card) = world.get::<ChildOf>(body).map(|c| c.parent()) else { return };
    let visible = world.resource::<InspectorSelection>().0.is_none();
    if let Some(mut node) = world.get_mut::<Node>(card) {
        let display = if visible { Display::Flex } else { Display::None };
        if node.display != display {
            crate::ui::set_display(&mut node, display);
        }
    }
    if *built || !visible {
        return;
    }
    let Some(settings) = world.query_filtered::<Entity, With<WorldSettingsEntity>>().iter(world).next() else { return };
    let Some(meta) = reflect_fields::describe_entity(world, settings)
        .into_iter()
        .find(|c| c.type_path.ends_with("WorldEnvironment"))
    else {
        return;
    };
    *built = true;
    let mut commands = world.commands();
    spawn_component_section(&mut commands, body, settings, &meta, false);
    world.flush();
}

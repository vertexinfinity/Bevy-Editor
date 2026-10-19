//! "Mesh Rendering" card: per-object shadow, wireframe and bounds toggles.
//!
//! Bevy expresses these as marker components (`NotShadowCaster`, `NotShadowReceiver`,
//! `Wireframe`, `ShowAabbGizmo`), so each switch inserts or removes one with undo.
use bevy::{
    inspector::InspectorSelection,
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};

use super::camera_card::{has_component, toggle_row};
use crate::core::play_mode::PlayState;
use crate::core::reflect_fields::{self, ToggleReflectComponent};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

/// (label, marker component, `true` when the marker means the feature is *off*).
pub const OBJECT_TOGGLES: &[(&str, &str, bool)] = &[
    ("Cast Shadows", "NotShadowCaster", true),
    ("Receive Shadows", "NotShadowReceiver", true),
    ("Wireframe", "Wireframe", false),
    ("Show Bounds", "ShowAabbGizmo", false),
];

#[derive(Component)]
pub struct ObjectCardRoot;

#[derive(Component)]
struct ObjectChip(&'static str, bool);

pub fn spawn_object_card(commands: &mut Commands, body: Entity) {
    commands.entity(body).insert(ObjectCardRoot);
    for (label, type_path, inverted) in OBJECT_TOGGLES {
        let chip = toggle_row(commands, body, label, "");
        commands.entity(chip).insert(ObjectChip(type_path, *inverted)).observe(move |click: On<PointerClick>, mut commands: Commands| {
            if click.button != PointerButton::Primary {
                return;
            }
            commands.queue(move |world: &mut World| {
                if *world.resource::<PlayState>() != PlayState::Edit {
                    return;
                }
                let Some(entity) = world.resource::<InspectorSelection>().0 else { return };
                if let Err(error) = set_object_flag(world, entity, type_path, !object_flag(world, entity, type_path, *inverted), *inverted) {
                    warn!("{error}");
                }
            });
        });
    }
}

/// Whether the feature behind a marker is on for `entity`.
pub fn object_flag(world: &World, entity: Entity, type_path: &str, inverted: bool) -> bool {
    has_component(world, entity, type_path) != inverted
}

pub fn set_object_flag(world: &mut World, entity: Entity, type_path: &str, on: bool, inverted: bool) -> Result<(), String> {
    let want_marker = on != inverted;
    if has_component(world, entity, type_path) == want_marker {
        return Ok(());
    }
    let command = if want_marker { ToggleReflectComponent::insert(entity, type_path) } else { ToggleReflectComponent::remove(entity, type_path) };
    reflect_fields::execute_command(world, command)
}

pub fn update_object_card(world: &mut World) {
    let selected = world
        .resource::<InspectorSelection>()
        .0
        .filter(|e| world.get::<Mesh3d>(*e).is_some() && world.get::<EditorInternal>(*e).is_none());
    let Some(card) = world.query_filtered::<&ChildOf, With<ObjectCardRoot>>().iter(world).next().map(|c| c.parent()) else { return };
    if let Some(mut node) = world.get_mut::<Node>(card) {
        let display = if selected.is_some() { Display::Flex } else { Display::None };
        if node.display != display {
            crate::ui::set_display(&mut node, display);
        }
    }
    let Some(entity) = selected else { return };
    let chips: Vec<(Entity, &'static str, bool)> = world.query::<(Entity, &ObjectChip)>().iter(world).map(|(e, c)| (e, c.0, c.1)).collect();
    for (chip, type_path, inverted) in chips {
        let on = object_flag(world, entity, type_path, inverted);
        let Some(child) = world.get::<Children>(chip).and_then(|c| c.first().copied()) else { continue };
        let text = if on { "On" } else { "Off" };
        if let Some(mut t) = world.get_mut::<Text>(child) {
            if t.0 != text {
                t.0 = text.into();
            }
        }
        if let Some(mut color) = world.get_mut::<TextColor>(child) {
            let next = TextColor(if on { ACCENT_CYAN } else { TEXT_MUTED });
            if *color != next {
                *color = next;
            }
        }
    }
}

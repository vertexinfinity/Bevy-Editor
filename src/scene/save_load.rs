//! Scene serialization and deserialization using Bevy's DynamicWorld.

use std::fs::File;
use std::io::Write;
use std::path::Path;
use bevy::{
    prelude::*,
    world_serialization::DynamicWorldRoot,
};

/// Marker component for entities internal to the editor that should NOT be serialized.
#[derive(Component, Reflect, Default, Clone, Copy)]
#[reflect(Component, Default)]
pub struct EditorInternal;

/// Marker component for authorable scene entities.
///
/// Ensures only user scene entities are serialized or cleared,
/// preventing accidental destruction of editor UI, windows, or Bevy resource entities.
#[derive(Component, Reflect, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct SceneEntity;

/// Serializes the scene entities to a BSN / RON scene file.
///
/// Only entities with `SceneEntity` are extracted, and internal editor entities (`EditorInternal`)
/// are strictly excluded.
pub fn save_world_to_scene(world: &World, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let type_registry = world.resource::<AppTypeRegistry>().read();

    // Query for user authorable SceneEntity entities only without mutably borrowing world
    let scene_entities: Vec<Entity> = if let Some(mut state) =
        bevy::ecs::query::QueryState::<Entity, (With<SceneEntity>, Without<EditorInternal>, Without<DynamicWorldRoot>)>::try_new(world)
    {
        state.iter(world).collect()
    } else {
        Vec::new()
    };

    let dynamic_world = bevy::world_serialization::DynamicWorldBuilder::from_world(world, &type_registry)
        .deny_component::<EditorInternal>()
        .deny_component::<SceneEntity>()
        .deny_component::<DynamicWorldRoot>()
        .deny_component::<bevy::camera::visibility::VisibilityClass>()
        .deny_component::<bevy::camera::visibility::ViewVisibility>()
        .extract_entities(scene_entities.into_iter())
        .remove_empty_entities()
        .build();

    let serialized_scene = dynamic_world.serialize(&type_registry)?;

    let mut file = File::create(path)?;
    file.write_all(serialized_scene.as_bytes())?;

    info!("Saved scene ({} entities) to {:?}", dynamic_world.entities.len(), path);
    Ok(())
}

/// Loads a scene file and spawns it in the world.
#[allow(dead_code)]
pub fn load_scene_from_file(
    commands: &mut Commands,
    asset_server: &AssetServer,
    path_str: &str,
) -> Entity {
    info!("Loading scene from {}", path_str);
    commands
        .spawn((
            Name::new("Loaded Scene"),
            DynamicWorldRoot(asset_server.load(path_str.to_string())),
        ))
        .id()
}

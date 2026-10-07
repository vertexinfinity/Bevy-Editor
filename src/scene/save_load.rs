//! Scene serialization and deserialization using Bevy's DynamicWorld.

use std::fs::File;
use std::io::Write;
use std::path::Path;
use bevy::{
    prelude::*,
    world_serialization::{DynamicWorld, DynamicWorldRoot},
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

/// Serializes the current world to a RON scene file.
pub fn save_world_to_scene(world: &World, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let type_registry = world.resource::<AppTypeRegistry>().read();
    let dynamic_world = DynamicWorld::from_world(world);
    let serialized_ron = dynamic_world.serialize(&type_registry)?;

    let mut file = File::create(path)?;
    file.write_all(serialized_ron.as_bytes())?;

    info!("Saved scene to {:?}", path);
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

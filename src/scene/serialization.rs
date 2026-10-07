//! Abstract Scene Serialization & Deserialization Pipeline.
//!
//! Decouples scene format persistence from editor UI code.
//! Supports multiple format providers (.ron, .json, .binary, .scn) via [`SceneSerializer`]
//! and [`SceneLoader`] traits.

#![allow(dead_code)]

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use bevy::{
    prelude::*,
    world_serialization::{DynamicWorld, DynamicWorldRoot},
};

/// Trait defining a scene format serializer.
pub trait SceneSerializer: Send + Sync + 'static {
    fn format_extension(&self) -> &'static str;
    fn format_name(&self) -> &'static str;
    fn serialize(&self, world: &World, path: &Path) -> Result<(), String>;
}

/// Trait defining a scene format loader.
pub trait SceneLoader: Send + Sync + 'static {
    fn format_extension(&self) -> &'static str;
    fn load(&self, commands: &mut Commands, asset_server: &AssetServer, path: &Path) -> Result<Entity, String>;
}

/// Registry of pluggable scene serialization backends.
#[derive(Resource, Default)]
pub struct SceneSerializationRegistry {
    serializers: HashMap<&'static str, Arc<dyn SceneSerializer>>,
    loaders: HashMap<&'static str, Arc<dyn SceneLoader>>,
}

impl SceneSerializationRegistry {
    pub fn register_serializer<T: SceneSerializer>(&mut self, s: T) {
        self.serializers.insert(s.format_extension(), Arc::new(s));
    }

    pub fn register_loader<T: SceneLoader>(&mut self, l: T) {
        self.loaders.insert(l.format_extension(), Arc::new(l));
    }

    pub fn get_serializer(&self, ext: &str) -> Option<Arc<dyn SceneSerializer>> {
        self.serializers.get(ext).cloned()
    }

    pub fn get_loader(&self, ext: &str) -> Option<Arc<dyn SceneLoader>> {
        self.loaders.get(ext).cloned()
    }
}

// Built-in RON Format Backend

pub struct RonSceneSerializer;

impl SceneSerializer for RonSceneSerializer {
    fn format_extension(&self) -> &'static str { "ron" }
    fn format_name(&self) -> &'static str { "Bevy Scene RON (*.ron)" }

    fn serialize(&self, world: &World, path: &Path) -> Result<(), String> {
        let type_registry = world.resource::<AppTypeRegistry>().read();
        let dynamic_world = DynamicWorld::from_world(world);
        let serialized_ron = dynamic_world
            .serialize(&type_registry)
            .map_err(|e| format!("Serialization error: {e}"))?;

        let mut file = File::create(path).map_err(|e| format!("IO error: {e}"))?;
        file.write_all(serialized_ron.as_bytes()).map_err(|e| format!("IO error: {e}"))?;
        info!("Successfully saved scene to {:?}", path);
        Ok(())
    }
}

pub struct RonSceneLoader;

impl SceneLoader for RonSceneLoader {
    fn format_extension(&self) -> &'static str { "ron" }

    fn load(&self, commands: &mut Commands, asset_server: &AssetServer, path: &Path) -> Result<Entity, String> {
        let path_str = path.to_string_lossy().to_string();
        info!("Loading scene via RonSceneLoader from: {}", path_str);
        let id = commands
            .spawn((
                Name::new(format!("Scene: {}", path.file_name().and_then(|n| n.to_str()).unwrap_or("loaded"))),
                DynamicWorldRoot(asset_server.load(path_str)),
                Transform::IDENTITY,
            ))
            .id();
        Ok(id)
    }
}

pub struct SceneSerializationPlugin;

impl Plugin for SceneSerializationPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = SceneSerializationRegistry::default();
        registry.register_serializer(RonSceneSerializer);
        registry.register_loader(RonSceneLoader);

        app.insert_resource(registry);
    }
}

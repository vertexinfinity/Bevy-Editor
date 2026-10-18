//! Scene management module for Bevy-Editor.

pub mod components;
pub mod drag_drop;
pub mod environment;
pub mod primitives;
pub mod save_load;
pub mod serialization;
pub mod showcase;

use std::path::PathBuf;
use bevy::prelude::*;
pub use save_load::{EditorInternal, SceneEntity};

/// Tracks the currently open or saved scene file.
#[derive(Resource, Default, Debug, Clone)]
pub struct CurrentScene {
    pub file_path: Option<PathBuf>,
    #[allow(dead_code)]
    pub is_modified: bool,
}

pub struct SceneManagerPlugin;

impl Plugin for SceneManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentScene>()
            .register_type::<EditorInternal>()
            .register_type::<SceneEntity>()
            .register_type::<Transform>()
            .register_type::<GlobalTransform>()
            .register_type::<Visibility>()
            .register_type::<InheritedVisibility>()
            .register_type::<ViewVisibility>()
            .add_plugins((
                components::AuthorableComponentsPlugin,
                serialization::SceneSerializationPlugin,
                drag_drop::DragDropPlugin,
                environment::EnvironmentPlugin,
            ))
            .add_systems(Startup, primitives::spawn_starter_scene)
            .add_systems(Update, enforce_directional_light_limits);
    }
}

/// Enforces safe directional light limits to prevent shadow map cascade overflow and GPU DeviceLost errors.
pub fn enforce_directional_light_limits(
    mut commands: Commands,
    mut dir_lights: Query<(Entity, &mut DirectionalLight, Option<&Name>), Without<crate::core::play_mode::PreviewEntity>>,
) {
    let mut sun_count = 0;
    for (entity, mut light, name) in &mut dir_lights {
        sun_count += 1;
        if sun_count > 4 {
            // Despawn excessive duplicate sun lights created by recursive scene loading or stacking
            warn!("Despawning excessive duplicate directional light {:?} ({:?}) to protect GPU driver.", entity, name);
            commands.entity(entity).despawn();
        } else if sun_count > 1 {
            // Keep at most 1 shadow-casting directional light to protect shadow map cascade textures
            light.shadow_maps_enabled = false;
        }
    }
}

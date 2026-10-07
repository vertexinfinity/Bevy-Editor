//! Scene management module for Bevy-Editor.

pub mod components;
pub mod drag_drop;
pub mod primitives;
pub mod save_load;
pub mod serialization;

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
            .add_plugins((
                components::AuthorableComponentsPlugin,
                serialization::SceneSerializationPlugin,
                drag_drop::DragDropPlugin,
            ))
            .add_systems(Startup, primitives::spawn_starter_scene);
    }
}

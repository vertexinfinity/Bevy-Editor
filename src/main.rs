//! Bevy-Editor: Minimal, extensible, modular 3D Scene Viewer & Multi-Platform Exporter.

mod camera;
mod export;
mod scene;
mod ui;

use bevy::{feathers::FeathersPlugins, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Editor".to_string(),
                resolution: (1600, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            FeathersPlugins,
            camera::EditorCameraPlugin,
            scene::SceneManagerPlugin,
            ui::EditorUiPlugin,
            export::ExportPlugin,
        ))
        .run();
}

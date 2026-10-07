//! Bevy-Editor: Minimal, extensible, modular 3D Scene Viewer & Multi-Platform Exporter.

mod camera;
mod core;
mod export;
mod project;
mod scene;
mod ui;

use bevy::{asset::UnapprovedPathMode, feathers::FeathersPlugins, prelude::*};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy Editor".to_string(),
                        resolution: (1600, 900).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    unapproved_path_mode: UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .add_plugins((
            FeathersPlugins,
            core::CoreEditorPlugin,
            camera::EditorCameraPlugin,
            scene::SceneManagerPlugin,
            ui::EditorUiPlugin,
            export::ExportPlugin,
        ))
        .run();
}

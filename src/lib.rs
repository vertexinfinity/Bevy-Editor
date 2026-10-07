//! Bevy-Editor Library: Core engine, scene, export, and UI subsystems for Bevylite.

pub mod camera;
pub mod core;
pub mod export;
pub mod project;
pub mod remote;
pub mod scene;
pub mod ui;

use bevy::{asset::UnapprovedPathMode, feathers::FeathersPlugins, prelude::*};

/// Launches the visual Bevylite Editor window application.
pub fn run_editor() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy Editor — Bevylite".to_string(),
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
            MeshPickingPlugin,
            bevy::gizmos::transform_gizmo::TransformGizmoPlugin,
            bevy::gizmos_render::transform_gizmo_render::TransformGizmoRenderPlugin,
            core::CoreEditorPlugin,
            camera::EditorCameraPlugin,
            scene::SceneManagerPlugin,
            ui::EditorUiPlugin,
            export::ExportPlugin,
            remote::EditorRemotePlugin::default(),
        ))
        .run();
}

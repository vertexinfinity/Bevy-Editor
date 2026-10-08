//! Bevy-Editor Library: Core engine, scene, export, and UI subsystems for Bevylite.

pub mod assets;
pub mod camera;
pub mod core;
pub mod export;
pub mod plugins;
pub mod project;
pub mod remote;
pub mod scene;
pub mod ui;

use bevy::{
    asset::UnapprovedPathMode,
    feathers::FeathersPlugins,
    prelude::*,
    render::{
        settings::{Backends, PowerPreference, RenderCreation, WgpuSettings},
        RenderPlugin,
    },
};

/// Launches the visual Bevylite Editor window application.
pub fn run_editor() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        // Prioritize DX12 on Windows to prevent Vulkan laptop swapchain DeviceLost panics
                        backends: Some(Backends::DX12 | Backends::VULKAN),
                        power_preference: PowerPreference::HighPerformance,
                        ..default()
                    })),
                    ..default()
                })
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
            core::CoreEditorPlugin,
            camera::EditorCameraPlugin,
            scene::SceneManagerPlugin,
            ui::EditorUiPlugin,
            export::ExportPlugin,
            remote::EditorRemotePlugin::default(),
        ))
        .run();
}

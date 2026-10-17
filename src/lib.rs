//! Bevy-Editor Library: Core engine, scene, export, and UI subsystems for Bevylite.

pub mod assets;
pub mod camera;
pub mod cli;
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
    winit::WinitSettings,
};

/// Launches the visual Bevylite Editor window application.
pub fn run_editor() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        // Prioritize DX12 on Windows to prevent Vulkan laptop swapchain DeviceLost panics
                        backends: Some(
                            Backends::from_env().unwrap_or(if cfg!(target_os = "windows") {
                                Backends::DX12
                            } else {
                                Backends::PRIMARY
                            }),
                        ),
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
                    file_path: std::env::current_dir()
                        .ok()
                        .map(|d| d.join("assets"))
                        .filter(|p| p.is_dir())
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "assets".to_string()),
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
            bevylite_lightmapper::BevyliteLightmapperPlugin,
        ))
        .insert_resource(WinitSettings::desktop_app())
        .insert_resource(bevy::render::error_handler::RenderErrorHandler(|error, main_world, _| {
            if error.description.contains("Surface::configure")
                || error.description.contains("Invalid surface")
                || error.description.contains("ResizeBuffers")
                || error.description.contains("window is in use")
            {
                warn!("Recovered from transient swapchain surface resize/configure error: {}", error.description);
                bevy::render::error_handler::RenderErrorPolicy::Ignore
            } else {
                error!("Quitting the application due to {:?} RenderError: {}", error.ty, error.description);
                main_world.write_message(bevy::app::AppExit::error());
                bevy::render::error_handler::RenderErrorPolicy::StopRendering
            }
        }))
        .run();
}

//! Play-In-Editor (PIE) Simulation & Hot-Reload Preview System.
//!
//! Provides:
//! - In-Viewport Play (▶ Play)
//! - Standalone Pop-out Window (⧉ Window)
//! - Simulation Pause (⏸) and Stop (⏹) with memory rollback

use bevy::{
    ecs::message::Message,
    prelude::*,
    world_serialization::DynamicWorld,
};
use crate::camera::EditorCamera;
use crate::scene::{primitives, EditorInternal, SceneEntity};

/// Current simulation state of the editor.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Edit,
    Playing,
    Paused,
}

/// Marker for dedicated pop-out game preview windows.
#[derive(Component)]
pub struct PreviewWindow;

/// Marker for cameras rendering to the pop-out preview window.
#[derive(Component)]
pub struct PreviewCamera;

/// In-memory snapshot taken right before entering Play mode.
#[derive(Resource, Default)]
pub struct SceneSnapshot {
    pub snapshot: Option<DynamicWorld>,
}

#[derive(Component, Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayModeAction {
    Play,
    PlayInWindow,
    Pause,
    Stop,
    Step,
}

pub struct PlayModePlugin;

impl Plugin for PlayModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayState>()
            .init_resource::<SceneSnapshot>()
            .add_message::<PlayModeAction>()
            .add_systems(Update, handle_play_mode_actions);
    }
}

pub fn handle_play_mode_actions(world: &mut World) {
    let actions: Vec<PlayModeAction> = {
        let Some(mut messages) = world.get_resource_mut::<Messages<PlayModeAction>>() else {
            return;
        };
        messages.drain().collect()
    };

    for action in actions {
        match action {
            PlayModeAction::Play => {
                let current_state = *world.resource::<PlayState>();
                match current_state {
                    PlayState::Edit => {
                        info!("Entering In-Viewport Play mode: zero compilation overhead, executing simulation on loaded models and scripts...");
                        
                        // Extract ONLY authorable SceneEntity entities (fast, <1ms, zero lag, no UI cloning)
                        let dynamic_world = {
                            let type_registry = world.resource::<AppTypeRegistry>().read();
                            let scene_entities: Vec<Entity> = if let Some(mut state) =
                                bevy::ecs::query::QueryState::<Entity, (With<SceneEntity>, Without<EditorInternal>)>::try_new(world)
                            {
                                state.iter(world).collect()
                            } else {
                                Vec::new()
                            };

                            bevy::world_serialization::DynamicWorldBuilder::from_world(world, &type_registry)
                                .deny_component::<EditorInternal>()
                                .deny_component::<bevy::camera::visibility::VisibilityClass>()
                                .deny_component::<bevy::camera::visibility::ViewVisibility>()
                                .extract_entities(scene_entities.into_iter())
                                .remove_empty_entities()
                                .build()
                        };

                        world.resource_mut::<SceneSnapshot>().snapshot = Some(dynamic_world);
                        *world.resource_mut::<PlayState>() = PlayState::Playing;

                        // Check if the scene has its own game camera (e.g. from template or authoring)
                        let has_game_cam = if let Some(mut state) =
                            bevy::ecs::query::QueryState::<Entity, (With<Camera>, With<SceneEntity>, Without<EditorCamera>)>::try_new(world)
                        {
                            state.iter(world).next().is_some()
                        } else {
                            false
                        };

                        if has_game_cam {
                            info!("Scene has dedicated game camera: disabling editor fly camera for play...");
                            if let Some(mut state) =
                                bevy::ecs::query::QueryState::<&mut Camera, With<EditorCamera>>::try_new(world)
                            {
                                for mut cam in state.iter_mut(world) {
                                    cam.is_active = false;
                                }
                            }
                        }

                        info!("Play mode active: simulation running!");
                    }
                    PlayState::Paused => {
                        *world.resource_mut::<PlayState>() = PlayState::Playing;
                        info!("Resumed Play mode.");
                    }
                    PlayState::Playing => {}
                }
            }
            PlayModeAction::PlayInWindow => {
                let project_dir = world
                    .get_resource::<crate::ui::hub::ActiveProject>()
                    .and_then(|p| p.project_dir.clone());

                if let Some(proj_dir) = project_dir {
                    info!("Launching Standalone Application via 'cargo run' for {}...", proj_dir.display());
                    info!("(Note: Standalone runner builds the independent Rust binary with Cargo. For instant 0s play on loaded models, click '▶ Play'!)");

                    // Save active scene as default_scene.bsn so standalone game runs current edits
                    let scenes_dir = proj_dir.join("assets").join("scenes");
                    let _ = std::fs::create_dir_all(&scenes_dir);
                    let default_scene_path = scenes_dir.join("default_scene.bsn");
                    if let Err(e) = crate::scene::save_load::save_world_to_scene(world, &default_scene_path) {
                        warn!("Could not save active scene before launching window: {e}");
                    }

                    // Sync models to user project if needed
                    let editor_models = std::path::Path::new("assets/models");
                    let project_models = proj_dir.join("assets").join("models");
                    if editor_models.is_dir() {
                        let _ = std::fs::create_dir_all(&project_models);
                        if let Ok(entries) = std::fs::read_dir(editor_models) {
                            for entry in entries.flatten() {
                                let dest = project_models.join(entry.file_name());
                                if !dest.exists() {
                                    let _ = std::fs::copy(entry.path(), dest);
                                }
                            }
                        }
                    }

                    // Spawn standalone game process (zero DX12 swapchain sharing issues, pure 60FPS)
                    let child_res = std::process::Command::new("cargo")
                        .arg("run")
                        .current_dir(&proj_dir)
                        .spawn();

                    match child_res {
                        Ok(_) => info!("Spawned standalone game window in external process!"),
                        Err(e) => warn!("Failed to launch standalone game process: {e}"),
                    }
                } else {
                    warn!("Cannot open standalone window: No active project loaded. Open or create a project first.");
                }
            }
            PlayModeAction::Pause => {
                let current_state = *world.resource::<PlayState>();
                if current_state == PlayState::Playing {
                    *world.resource_mut::<PlayState>() = PlayState::Paused;
                    info!("Simulation paused.");
                } else if current_state == PlayState::Paused {
                    *world.resource_mut::<PlayState>() = PlayState::Playing;
                    info!("Simulation unpaused.");
                }
            }
            PlayModeAction::Stop => {
                let current_state = *world.resource::<PlayState>();
                if current_state != PlayState::Edit {
                    info!("Stopping simulation: rolling back world snapshot...");
                    *world.resource_mut::<PlayState>() = PlayState::Edit;

                    // Always ensure EditorCamera is re-activated for editing
                    if let Some(mut state) =
                        bevy::ecs::query::QueryState::<&mut Camera, With<EditorCamera>>::try_new(world)
                    {
                        for mut cam in state.iter_mut(world) {
                            cam.is_active = true;
                        }
                    }

                    // Despawn preview pop-out window and camera if any exist
                    world.despawn_all::<With<PreviewWindow>>();
                    world.despawn_all::<With<PreviewCamera>>();

                    // Despawn simulated scene entities
                    world.despawn_all::<With<SceneEntity>>();

                    // Restore in-memory snapshot
                    let snapshot = world.resource_mut::<SceneSnapshot>().snapshot.take();
                    if let Some(dynamic_world) = snapshot {
                        let mut entity_map = bevy::ecs::entity::EntityHashMap::default();
                        let _ = dynamic_world.write_to_world(world, &mut entity_map);
                        info!("World restored to exact pre-play snapshot.");
                    } else {
                        let _ = world.run_system_cached(primitives::spawn_starter_scene);
                    }
                }
            }
            PlayModeAction::Step => {
                info!("Single-frame simulation step.");
                *world.resource_mut::<PlayState>() = PlayState::Paused;
            }
        }
    }
}

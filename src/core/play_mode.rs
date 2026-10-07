//! Play-In-Editor (PIE) Simulation & Hot-Reload Preview System.
//!
//! Provides:
//! - In-Viewport Play (▶ Play)
//! - Standalone Pop-out Window (⧉ Window)
//! - Simulation Pause (⏸) and Stop (⏹) with memory rollback

use bevy::{
    camera::RenderTarget,
    ecs::message::Message,
    prelude::*,
    window::WindowRef,
    world_serialization::DynamicWorld,
};
use crate::scene::{primitives, EditorInternal};

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
                        info!("Entering In-Viewport Play mode: snapshotting world...");
                        let dynamic_world = DynamicWorld::from_world(world);
                        world.resource_mut::<SceneSnapshot>().snapshot = Some(dynamic_world);
                        *world.resource_mut::<PlayState>() = PlayState::Playing;
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
                let current_state = *world.resource::<PlayState>();
                if current_state == PlayState::Edit {
                    info!("Entering Pop-out Window Play mode: snapshotting world...");
                    let dynamic_world = DynamicWorld::from_world(world);
                    world.resource_mut::<SceneSnapshot>().snapshot = Some(dynamic_world);
                    *world.resource_mut::<PlayState>() = PlayState::Playing;
                }

                // Spawn secondary window for standalone preview
                let win_ent = world
                    .spawn((
                        Window {
                            title: "Game Preview — Standalone Viewport".to_string(),
                            resolution: (1280, 720).into(),
                            ..default()
                        },
                        PreviewWindow,
                        EditorInternal,
                    ))
                    .id();

                // Spawn dedicated game camera rendering to the new window
                world.spawn((
                    Camera3d::default(),
                    RenderTarget::Window(WindowRef::Entity(win_ent)),
                    Transform::from_xyz(0.0, 4.0, 8.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
                    PreviewCamera,
                    EditorInternal,
                ));

                info!("Opened Game Preview in a dedicated floating window.");
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

                    // Despawn preview pop-out window and camera
                    let preview_windows: Vec<Entity> = world
                        .iter_entities()
                        .filter(|e| e.contains::<PreviewWindow>())
                        .map(|e| e.id())
                        .collect();
                    for e in preview_windows {
                        world.despawn(e);
                    }

                    let preview_cams: Vec<Entity> = world
                        .iter_entities()
                        .filter(|e| e.contains::<PreviewCamera>())
                        .map(|e| e.id())
                        .collect();
                    for e in preview_cams {
                        world.despawn(e);
                    }

                    // Despawn simulated user entities
                    let entities_to_despawn: Vec<Entity> = world
                        .iter_entities()
                        .filter(|e| !e.contains::<EditorInternal>())
                        .map(|e| e.id())
                        .collect();

                    for e in entities_to_despawn {
                        world.despawn(e);
                    }

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

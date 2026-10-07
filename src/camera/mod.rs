//! Viewport and fly-through camera setup using FreeCamera.

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    prelude::*,
};
use crate::scene::EditorInternal;

/// Marker for the editor's primary viewport camera.
#[derive(Component)]
pub struct EditorCamera;

pub struct EditorCameraPlugin;

impl Plugin for EditorCameraPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FreeCameraPlugin>() {
            app.add_plugins(FreeCameraPlugin);
        }
        app.add_systems(Startup, setup_editor_camera);
    }
}

fn setup_editor_camera(mut commands: Commands) {
    commands.spawn((
        EditorCamera,
        EditorInternal,
        Name::new("Editor Viewport Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        FreeCamera {
            sensitivity: 0.002,
            walk_speed: 12.0,
            run_speed: 30.0,
            ..default()
        },
    ));
}

//! Open the bundled QA scene directly in a separate editor instance.
//! Run from the repository root: cargo run --example editor_showcase
use bevy::prelude::*;
use bevy_editor_lib::{scene::{showcase::spawn_editor_showcase, SceneEntity}, ui::{hub::ActiveProject, EditorAppState}};

fn main() {
    bevy_editor_lib::create_editor_app().add_systems(PostStartup, open_sample).run();
}

fn open_sample(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<Entity, With<SceneEntity>>,
    mut active: ResMut<ActiveProject>,
    mut next: ResMut<NextState<EditorAppState>>,
) {
    for entity in &existing { commands.entity(entity).despawn(); }
    active.set_project_dir(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples/editor_showcase"));
    spawn_editor_showcase(&mut commands, &mut meshes, &mut materials);
    next.set(EditorAppState::InEditor);
}

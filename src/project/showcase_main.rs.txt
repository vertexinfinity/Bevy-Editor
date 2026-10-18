//! Standalone version of the editor showcase. Space opens/closes the gate.
use bevy::prelude::*;
mod showcase_scene;

#[derive(Resource)]
struct Demo { gate: Entity, beacon: Entity, open: bool }

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window {
            title: "Bevylite Editor Showcase".to_string(), resolution: (1280, 720).into(), ..default()
        }), ..default() }))
        .add_systems(Startup, setup)
        .add_systems(Update, animate)
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let sample = showcase_scene::spawn_showcase(&mut commands, &mut meshes, &mut materials);
    commands.entity(sample.camera).insert(Camera::default());
    commands.insert_resource(Demo { gate: sample.gate, beacon: sample.beacon, open: false });
}

fn animate(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, mut demo: ResMut<Demo>, mut transforms: Query<&mut Transform>) {
    if keys.just_pressed(KeyCode::Space) { demo.open = !demo.open; }
    if let Ok(mut gate) = transforms.get_mut(demo.gate) {
        let target = if demo.open { 3.7 } else { 1.8 };
        gate.translation.y += (target - gate.translation.y) * (time.delta_secs() * 5.0).min(1.0);
    }
    if let Ok(mut beacon) = transforms.get_mut(demo.beacon) { beacon.rotate_y(time.delta_secs()); }
}

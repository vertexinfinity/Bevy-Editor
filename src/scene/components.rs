//! Authorable components that can be attached to entities in the editor.

use bevy::prelude::*;

/// Custom script component allowing user scripts (Lua, Rhai, or custom behavior).
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct ScriptComponent {
    pub script_file: String,
    pub enabled: bool,
}

impl Default for ScriptComponent {
    fn default() -> Self {
        Self {
            script_file: "scripts/main.lua".to_string(),
            enabled: true,
        }
    }
}

/// Dynamic velocity for physics or character movement.
#[derive(Component, Reflect, Default, Clone, Debug)]
#[reflect(Component, Default)]
pub struct Velocity {
    pub linear: Vec3,
}

/// Continuous rotation animation around an axis.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct SpinAnimation {
    pub speed: f32,
    pub axis: Vec3,
}

impl Default for SpinAnimation {
    fn default() -> Self {
        Self {
            speed: 1.5,
            axis: Vec3::Y,
        }
    }
}

/// Gameplay tag or identifier.
#[derive(Component, Reflect, Default, Clone, Debug)]
#[reflect(Component, Default)]
pub struct GameplayTag {
    pub tag: String,
}

pub struct AuthorableComponentsPlugin;

impl Plugin for AuthorableComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ScriptComponent>()
            .register_type::<Velocity>()
            .register_type::<SpinAnimation>()
            .register_type::<GameplayTag>()
            .add_systems(Update, animate_spinning_objects);
    }
}

fn animate_spinning_objects(time: Res<Time>, mut query: Query<(&SpinAnimation, &mut Transform)>) {
    for (spin, mut transform) in &mut query {
        if spin.axis.length_squared() > 0.001 {
            let dir = spin.axis.normalize();
            transform.rotate_axis(Dir3::new_unchecked(dir), spin.speed * time.delta_secs());
        }
    }
}

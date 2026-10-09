//! Authorable components that can be attached to entities in the editor.
//!
//! Includes extensible script public property serialization & object reference assignment.

use std::collections::HashMap;
use bevy::prelude::*;
use crate::core::play_mode::PlayState;

/// Types of public script properties that can be inspected and serialized.
#[derive(Reflect, Clone, Debug, PartialEq)]
pub enum ScriptPropertyVal {
    Float(f32),
    Int(i64),
    Bool(bool),
    String(String),
    Vec3(Vec3),
}

/// Custom script component allowing user scripts (Lua, Rhai, or custom behavior).
///
/// Features:
/// - Public property exposure and serialization to scene files
/// - Object/entity assignment slots (linking other scene entities or target scripts)
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct ScriptComponent {
    pub script_file: String,
    pub enabled: bool,
    /// Exposed public properties edited in inspector and serialized to scene
    pub properties: HashMap<String, ScriptPropertyVal>,
    /// Assigned entity references (e.g. target_actor, trigger_volume, camera_focus)
    pub entity_refs: HashMap<String, Entity>,
}

impl Default for ScriptComponent {
    fn default() -> Self {
        let mut props = HashMap::new();
        props.insert("move_speed".to_string(), ScriptPropertyVal::Float(5.0));
        props.insert("jump_force".to_string(), ScriptPropertyVal::Float(10.0));
        props.insert("is_active".to_string(), ScriptPropertyVal::Bool(true));
        props.insert("actor_name".to_string(), ScriptPropertyVal::String("PlayerHero".to_string()));

        Self {
            script_file: "scripts/player.lua".to_string(),
            enabled: true,
            properties: props,
            entity_refs: HashMap::new(),
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

/// Interactive action assigned to a UI button, triggered on click during Play mode.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default)]
pub enum UiButtonClickAction {
    LogMessage(String),
    ToggleLight,
    SpawnCube,
    PrintStats,
    CustomAction(String),
}

impl Default for UiButtonClickAction {
    fn default() -> Self {
        Self::LogMessage("Button Clicked!".to_string())
    }
}

impl UiButtonClickAction {
    pub fn execute(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        lights: &mut Query<&mut Visibility, With<PointLight>>,
    ) {
        match self {
            Self::LogMessage(msg) => {
                info!("[UI Action] {}", msg);
            }
            Self::ToggleLight => {
                for mut vis in lights.iter_mut() {
                    *vis = match *vis {
                        Visibility::Visible | Visibility::Inherited => Visibility::Hidden,
                        Visibility::Hidden => Visibility::Visible,
                    };
                }
                info!("[UI Action] Toggled scene lights!");
            }
            Self::SpawnCube => {
                let mesh = meshes.add(Cuboid::new(0.5, 0.5, 0.5));
                let mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.9, 0.8, 0.2),
                    ..default()
                });
                commands.spawn((
                    crate::scene::SceneEntity,
                    Name::new("Spawned Dynamic Cube"),
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 3.0, 0.0),
                    Velocity { linear: Vec3::new(0.0, -1.0, 0.0) },
                ));
                info!("[UI Action] Spawned dynamic cube!");
            }
            Self::PrintStats => {
                info!("[UI Action] Simulation running at regular tick.");
            }
            Self::CustomAction(name) => {
                info!("[UI Action] Triggered custom callback: {}", name);
            }
        }
    }
}

pub struct AuthorableComponentsPlugin;

impl Plugin for AuthorableComponentsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ScriptPropertyVal>()
            .register_type::<ScriptComponent>()
            .register_type::<Velocity>()
            .register_type::<SpinAnimation>()
            .register_type::<GameplayTag>()
            .register_type::<UiButtonClickAction>()
            .add_systems(
                Update,
                (
                    animate_spinning_objects,
                    simulate_velocities,
                    handle_ui_button_interactions,
                ),
            );
    }
}

fn animate_spinning_objects(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    mut query: Query<(&SpinAnimation, &mut Transform)>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    for (spin, mut transform) in &mut query {
        if spin.axis.length_squared() > 0.001 {
            let dir = spin.axis.normalize();
            transform.rotate_axis(Dir3::new_unchecked(dir), spin.speed * time.delta_secs());
        }
    }
}

/// Simulates linear velocity when in Play mode.
fn simulate_velocities(
    time: Res<Time>,
    play_state: Option<Res<PlayState>>,
    mut query: Query<(&Velocity, &mut Transform)>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    for (vel, mut transform) in &mut query {
        transform.translation += vel.linear * time.delta_secs();
    }
}

/// Handles interactive UI button clicks during Play mode.
fn handle_ui_button_interactions(
    play_state: Option<Res<PlayState>>,
    interaction_query: Query<&UiButtonClickAction, Added<bevy::ui::Pressed>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<&mut Visibility, With<PointLight>>,
) {
    if let Some(state) = play_state {
        if *state != PlayState::Playing {
            return;
        }
    } else {
        return;
    }

    for action in &interaction_query {
        action.execute(&mut commands, &mut meshes, &mut materials, &mut lights);
    }
}

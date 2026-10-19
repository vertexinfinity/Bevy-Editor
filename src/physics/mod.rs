//! Physics materials, colliders and a lightweight Play-mode simulation.
//!
//! ## Authoring
//! * [`RigidBody`]: `kind` Static / Dynamic / Kinematic, `mass` (0 = density × volume),
//!   gravity scale, damping, initial velocities, per-axis position/rotation locks, `can_sleep`.
//! * [`Collider`]: `Auto` (sphere for spherical meshes, else the mesh bounding box; flat meshes
//!   such as planes become a 0.5 m slab under their surface), `Box` (full size), `Sphere`
//!   (`size.x` = radius), `Capsule` (`size.x` = radius, `size.y` = total height, local Y),
//!   `offset`, `is_trigger`. Sizes are local and scaled by the Transform.
//! * [`PhysicsMaterial`]: static/dynamic friction, restitution, density and PhysX-style combine
//!   rules. Built-in presets: Default, Ice, Rubber, Metal, Wood, Concrete, Bouncy, Sticky. User
//!   presets live in `assets/physics/<name>.physmat.json`.
//! * A collider without a rigid body is static. With `PhysicsSettings::auto_static_meshes`
//!   (default on) every other scene mesh acts as a static Auto collider during Play.
//!
//! All three are reflected, so they show in the Components card, work with `editor/set_field`
//! and are saved in RON scenes. The "Physics" inspector card adds/removes them with undo and
//! the selected object's collider is outlined in the viewport.
//!
//! ## Simulation
//! Runs only on Play-mode copies (`PreviewEntity`) while `PlayState::Playing`, at a fixed rate
//! (`fixed_hz`, default 120 Hz, at most 0.25 s of simulation per frame) driven by virtual time, so Pause
//! freezes it and `editor/time_scale` scales it. Stop discards it. See [`solver`] for the method.
//! A dynamic body's `Velocity` component becomes its start velocity (and is removed from the
//! runtime copy so it is not integrated twice). Kinematic bodies follow their Transform and
//! push dynamic bodies with the velocity implied by that motion. Overlaps with triggers are
//! sent as [`TriggerEvent`] messages.
//!
//! ## Limits
//! Brute-force O(n²) broad phase (fine for a few hundred bodies); no continuous collision
//! detection (very fast small objects can tunnel through thin colliders); capsule–box contacts
//! use three sample spheres; capsule inertia is approximated by its bounding box; the centre of
//! mass is the entity origin; no joints; per-body (not island) sleeping; locks are world axes;
//! scene meshes with arbitrary geometry collide as their bounding box.
//!
//! ## Remote methods
//! | Method | Params | Purpose |
//! | :-- | :-- | :-- |
//! | `editor/physics_presets` | | Built-in and custom presets with their values |
//! | `editor/physics_material` | `entity`/`name`, `preset`?, `friction`?, `static_friction`?, `dynamic_friction`?, `restitution`?, `density`?, `friction_combine`?, `restitution_combine`?, `remove`? | Set (or remove) an entity's material; `preset` is applied first, other params override it |
//! | `editor/rigid_body` | `entity`/`name`, `kind`?, `mass`?, `gravity_scale`?, `linear_damping`?, `angular_damping`?, `velocity`?, `angular_velocity`?, `lock_position`?, `lock_rotation`?, `can_sleep`?, `add_collider`? (default true), `remove`? | Add / edit / remove a rigid body; adds a fitted collider when missing |
//! | `editor/collider` | `entity`/`name`, `shape`?, `size`?, `radius`?, `height`?, `offset`?, `is_trigger`?, `remove`? | Add / edit / remove a collider (new ones are fitted to the mesh) |
//! | `editor/physics_settings` | `gravity`?, `fixed_hz`?, `velocity_iterations`?, `position_iterations`?, `auto_static_meshes`?, `sleeping`?, `show_all_colliders`? | Read or change global settings (saved to `assets/physics/settings.json`) |
//! | `editor/physics_save_preset` | `name`, `entity`/`source`? (copy its material), `base`? (preset), material params?, `apply`? | Save a custom preset file |
//! | `editor/physics_state` | | Runtime bodies (pose, velocity, mass, sleeping) and trigger overlaps during Play |
//! | `editor/physics_game_view_screenshot` | `path` | PNG of the Game View window (debug aid for agents) |
//! | `editor/physics_reveal_card` | | Scrolls the inspector to the Physics card (debug aid for screenshots) |
//!
//! Edits go through the undo history. Lock params accept `true` or `[x, y, z]` booleans.
pub mod components;
pub mod editor;
pub mod presets;
pub mod rpc;
pub mod runtime;
pub mod solver;

use bevy::prelude::*;
use serde_json::Value;

pub use components::*;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        let settings = std::fs::read_to_string(rpc::settings_path()).ok().and_then(|s| serde_json::from_str::<PhysicsSettings>(&s).ok()).unwrap_or_default();
        app.register_type::<PhysicsMaterial>()
            .register_type::<RigidBody>()
            .register_type::<Collider>()
            .register_type::<PhysicsSettings>()
            .insert_resource(settings)
            .init_resource::<runtime::PhysicsRuntime>()
            .init_resource::<editor::PhysicsCardMessage>()
            .add_message::<TriggerEvent>()
            .add_systems(Update, runtime::simulate_physics.in_set(crate::core::play_mode::PlaySystems::Simulation))
            .add_systems(Last, editor::sync_physics_sidecar);

        if app.is_plugin_added::<bevy::gizmos::GizmoPlugin>() {
            app.add_systems(PostUpdate, editor::draw_collider_gizmos.run_if(in_state(crate::ui::EditorAppState::InEditor)));
        }
        app.init_resource::<crate::ui::inspector::extensions::InspectorSectionRegistry>();
        app.world_mut().resource_mut::<crate::ui::inspector::extensions::InspectorSectionRegistry>().register(crate::ui::inspector::extensions::InspectorSection {
            id: "bevylite.physics",
            title: "Physics",
            order: 30,
            spawn: editor::spawn_physics_card,
        });
        app.add_systems(Update, editor::update_physics_card.run_if(in_state(crate::ui::EditorAppState::InEditor)));
    }
}

/// Remote methods owned by this module; `None` means the method is not ours.
pub fn dispatch(world: &mut World, method: &str, params: Option<Value>) -> Option<Result<Value, (i32, String)>> {
    rpc::dispatch(world, method, params)
}

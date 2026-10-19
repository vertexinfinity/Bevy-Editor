//! Authorable physics components and the global [`PhysicsSettings`] resource.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// How two materials' values are combined at a contact. When the two sides disagree the
/// rule with the higher priority wins (Average < Minimum < Multiply < Maximum), like PhysX.
#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[reflect(Default)]
pub enum CombineRule {
    #[default]
    Average,
    Minimum,
    Multiply,
    Maximum,
}

impl CombineRule {
    pub const ALL: [Self; 4] = [Self::Average, Self::Minimum, Self::Multiply, Self::Maximum];

    fn priority(self) -> u8 {
        match self {
            Self::Average => 0,
            Self::Minimum => 1,
            Self::Multiply => 2,
            Self::Maximum => 3,
        }
    }

    /// Combines `a` and `b` using whichever of the two rules has priority.
    pub fn combine(rule_a: Self, a: f32, rule_b: Self, b: f32) -> f32 {
        let rule = if rule_a.priority() >= rule_b.priority() { rule_a } else { rule_b };
        match rule {
            Self::Average => (a + b) * 0.5,
            Self::Minimum => a.min(b),
            Self::Multiply => a * b,
            Self::Maximum => a.max(b),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.to_ascii_lowercase();
        Self::ALL.into_iter().find(|r| {
            let name = format!("{r:?}").to_ascii_lowercase();
            name == s || name.starts_with(&s) && s.len() >= 3
        })
    }
}

/// Surface properties used by colliders. Pick one of the presets in the Physics card or type
/// your own values; "Save as Preset" stores it under the `preset` name.
#[derive(Component, Reflect, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[reflect(Component, Default, @bevy::annotations::Tooltip("Friction, bounciness and density of a collider"))]
#[serde(default)]
pub struct PhysicsMaterial {
    #[reflect(@bevy::annotations::DisplayName("Preset Name"), @bevy::annotations::Tooltip("Preset this material came from; also the name used by Save as Preset"))]
    pub preset: String,
    #[reflect(@bevy::annotations::ValueRange(0.0..=2.0), @bevy::annotations::Step(0.05), @bevy::annotations::Tooltip("Friction coefficient while at rest (resists starting to slide)"))]
    pub static_friction: f32,
    #[reflect(@bevy::annotations::ValueRange(0.0..=2.0), @bevy::annotations::Step(0.05), @bevy::annotations::Tooltip("Friction coefficient while sliding"))]
    pub dynamic_friction: f32,
    #[reflect(@bevy::annotations::DisplayName("Restitution (Bounciness)"), @bevy::annotations::ValueRange(0.0..=1.0), @bevy::annotations::Step(0.05), @bevy::annotations::Tooltip("0 = no bounce, 1 = perfectly elastic"))]
    pub restitution: f32,
    #[reflect(@bevy::annotations::ValueRange(1.0..=20000.0), @bevy::annotations::Step(50.0), @bevy::annotations::Unit("kg/m³"), @bevy::annotations::Tooltip("Mass per volume; automatic body mass = density × collider volume"))]
    pub density: f32,
    pub friction_combine: CombineRule,
    pub restitution_combine: CombineRule,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            preset: "Default".into(),
            static_friction: 0.6,
            dynamic_friction: 0.5,
            restitution: 0.1,
            density: 1000.0,
            friction_combine: CombineRule::Average,
            restitution_combine: CombineRule::Average,
        }
    }
}

impl PhysicsMaterial {
    /// Compares physical values only (ignores the preset name).
    pub fn same_values(&self, other: &Self) -> bool {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        close(self.static_friction, other.static_friction)
            && close(self.dynamic_friction, other.dynamic_friction)
            && close(self.restitution, other.restitution)
            && close(self.density, other.density)
            && self.friction_combine == other.friction_combine
            && self.restitution_combine == other.restitution_combine
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[reflect(Default)]
pub enum BodyKind {
    /// Never moves; infinite mass.
    Static,
    /// Moved by gravity, contacts and friction.
    #[default]
    Dynamic,
    /// Moved by scripts / animation (Transform); pushes dynamic bodies but is never pushed.
    Kinematic,
}

impl BodyKind {
    pub const ALL: [Self; 3] = [Self::Static, Self::Dynamic, Self::Kinematic];
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.to_ascii_lowercase();
        Self::ALL.into_iter().find(|k| format!("{k:?}").to_ascii_lowercase() == s)
    }
}

/// Per-axis lock (world axes).
#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[reflect(Default)]
pub struct AxisLocks {
    pub x: bool,
    pub y: bool,
    pub z: bool,
}

impl AxisLocks {
    pub fn as_array(self) -> [bool; 3] {
        [self.x, self.y, self.z]
    }
}

/// Makes a mesh take part in the Play-mode simulation.
#[derive(Component, Reflect, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[reflect(Component, Default, @bevy::annotations::Tooltip("Simulated body; needs a Collider to touch anything"))]
#[serde(default)]
pub struct RigidBody {
    #[reflect(@bevy::annotations::DisplayName("Body Type"))]
    pub kind: BodyKind,
    #[reflect(@bevy::annotations::ValueRange(0.0..=100000.0), @bevy::annotations::Step(0.5), @bevy::annotations::Unit("kg"), @bevy::annotations::Tooltip("0 = automatic (material density × collider volume)"))]
    pub mass: f32,
    #[reflect(@bevy::annotations::ValueRange(-5.0..=5.0), @bevy::annotations::Step(0.1), @bevy::annotations::Tooltip("Multiplier on the global gravity"))]
    pub gravity_scale: f32,
    #[reflect(@bevy::annotations::ValueRange(0.0..=10.0), @bevy::annotations::Step(0.05), @bevy::annotations::Tooltip("Air drag on linear motion"))]
    pub linear_damping: f32,
    #[reflect(@bevy::annotations::ValueRange(0.0..=10.0), @bevy::annotations::Step(0.05), @bevy::annotations::Tooltip("Drag on spinning"))]
    pub angular_damping: f32,
    #[reflect(@bevy::annotations::Unit("m/s"), @bevy::annotations::Step(0.5), @bevy::annotations::Tooltip("Velocity when Play starts"))]
    pub initial_linear_velocity: Vec3,
    #[reflect(@bevy::annotations::Unit("rad/s"), @bevy::annotations::Step(0.5), @bevy::annotations::Tooltip("Spin when Play starts"))]
    pub initial_angular_velocity: Vec3,
    #[reflect(@bevy::annotations::Tooltip("Freeze movement along world axes"))]
    pub lock_position: AxisLocks,
    #[reflect(@bevy::annotations::Tooltip("Freeze rotation around world axes"))]
    pub lock_rotation: AxisLocks,
    #[reflect(@bevy::annotations::Tooltip("Let the body sleep when it comes to rest (saves work, removes jitter)"))]
    pub can_sleep: bool,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            kind: BodyKind::Dynamic,
            mass: 0.0,
            gravity_scale: 1.0,
            linear_damping: 0.02,
            angular_damping: 0.08,
            initial_linear_velocity: Vec3::ZERO,
            initial_angular_velocity: Vec3::ZERO,
            lock_position: AxisLocks::default(),
            lock_rotation: AxisLocks::default(),
            can_sleep: true,
        }
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[reflect(Default)]
pub enum ColliderShape {
    /// Fitted to the mesh: a sphere for spherical meshes, otherwise the mesh's bounding box.
    #[default]
    Auto,
    Box,
    Sphere,
    /// Capsule along local Y.
    Capsule,
}

impl ColliderShape {
    pub const ALL: [Self; 4] = [Self::Auto, Self::Box, Self::Sphere, Self::Capsule];
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.to_ascii_lowercase();
        let s = if s == "cube" || s == "cuboid" { "box".to_string() } else { s };
        Self::ALL.into_iter().find(|k| format!("{k:?}").to_ascii_lowercase() == s)
    }
}

/// Collision shape in the entity's local space (scaled by its Transform).
#[derive(Component, Reflect, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[reflect(Component, Default, @bevy::annotations::Tooltip("Collision shape; without a Rigid Body it is a static collider"))]
#[serde(default)]
pub struct Collider {
    pub shape: ColliderShape,
    #[reflect(@bevy::annotations::Unit("m"), @bevy::annotations::Step(0.05), @bevy::annotations::Tooltip("Box: full size. Sphere: x = radius. Capsule: x = radius, y = total height. Ignored for Auto"))]
    pub size: Vec3,
    #[reflect(@bevy::annotations::Unit("m"), @bevy::annotations::Step(0.05), @bevy::annotations::Tooltip("Shape centre relative to the entity origin. Ignored for Auto"))]
    pub offset: Vec3,
    #[reflect(@bevy::annotations::DisplayName("Is Trigger"), @bevy::annotations::Tooltip("Detects overlaps (TriggerEvent) without blocking"))]
    pub is_trigger: bool,
}

impl Default for Collider {
    fn default() -> Self {
        Self { shape: ColliderShape::Auto, size: Vec3::ONE, offset: Vec3::ZERO, is_trigger: false }
    }
}

/// Global simulation settings (editable over RPC `editor/physics_settings`; persisted to
/// `assets/physics/settings.json`).
#[derive(Resource, Reflect, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[reflect(Resource, Default)]
#[serde(default)]
pub struct PhysicsSettings {
    pub gravity: Vec3,
    /// Fixed simulation rate in Hz.
    pub fixed_hz: f32,
    pub velocity_iterations: u32,
    pub position_iterations: u32,
    /// Scene meshes without RigidBody/Collider act as static box colliders during Play.
    pub auto_static_meshes: bool,
    pub sleeping: bool,
    /// Draw every collider in the viewport, not only the selected one.
    pub show_all_colliders: bool,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            fixed_hz: 120.0,
            velocity_iterations: 12,
            position_iterations: 4,
            auto_static_meshes: true,
            sleeping: true,
            show_all_colliders: false,
        }
    }
}

/// Sent when a body starts or stops overlapping a trigger collider.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerEvent {
    pub trigger: Entity,
    pub other: Entity,
    pub entered: bool,
}

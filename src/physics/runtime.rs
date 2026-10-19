//! ECS bridge: mirrors Play-mode preview entities into the [`PhysicsWorld`], steps it at a
//! fixed rate while Playing, and writes dynamic poses back to `Transform`.
use super::components::*;
use super::solver::{Body, Motion, PhysicsWorld, Shape, SolverConfig, SurfaceMaterial};
use crate::core::play_mode::{PlayState, PreviewEntity};
use crate::scene::components::Velocity;
use crate::scene::EditorInternal;
use bevy::{camera::primitives::Aabb, math::Affine3A, mesh::VertexAttributeValues, prelude::*};
use std::collections::{HashMap, HashSet};

/// Runtime simulation state; empty while editing.
#[derive(Resource, Default)]
pub struct PhysicsRuntime {
    pub world: PhysicsWorld,
    pub index: HashMap<Entity, usize>,
    accumulator: f32,
    written: HashMap<Entity, (Vec3, Quat)>,
    overlaps: HashSet<(u64, u64)>,
    pub steps: u64,
    active: bool,
}

/// Fits a collider to a mesh: a sphere when every vertex is (nearly) equidistant from the
/// bounds centre, otherwise the bounding box. Flat meshes (planes) get a 0.5 m thick slab
/// below their surface so fast objects do not tunnel through.
pub fn fit_collider(mesh: Option<&Mesh>, aabb: Option<&Aabb>) -> Collider {
    let positions = mesh.and_then(|m| m.try_attribute_option(Mesh::ATTRIBUTE_POSITION).ok().flatten()).and_then(|v| match v {
        VertexAttributeValues::Float32x3(p) if !p.is_empty() => Some(p),
        _ => None,
    });
    let (min, max) = if let Some(p) = positions {
        p.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), v| (lo.min(Vec3::from_array(*v)), hi.max(Vec3::from_array(*v))))
    } else if let Some(aabb) = aabb {
        (Vec3::from(aabb.center - aabb.half_extents), Vec3::from(aabb.center + aabb.half_extents))
    } else {
        (Vec3::splat(-0.5), Vec3::splat(0.5))
    };
    let center = (min + max) * 0.5;
    let mut half = (max - min) * 0.5;
    if let Some(p) = positions {
        let r_max = half.max_element();
        let equal_extents = half.min_element() > r_max * 0.97;
        if equal_extents && r_max > 1e-4 && p.len() >= 12 && p.iter().all(|v| ((Vec3::from_array(*v) - center).length() - r_max).abs() < r_max * 0.03) {
            return Collider { shape: ColliderShape::Sphere, size: Vec3::splat(r_max), offset: center, is_trigger: false };
        }
    }
    let mut offset = center;
    const THIN: f32 = 0.01;
    if half.y < THIN {
        offset.y = center.y + half.y - 0.25;
        half.y = 0.25;
    }
    if half.x < THIN {
        half.x = 0.05;
    }
    if half.z < THIN {
        half.z = 0.05;
    }
    Collider { shape: ColliderShape::Box, size: half * 2.0, offset, is_trigger: false }
}

/// Resolves the collider (fitting Auto to the mesh) into a solver shape and offset at `scale`.
pub fn resolve_shape(collider: &Collider, mesh: Option<&Mesh>, aabb: Option<&Aabb>, scale: Vec3) -> (Shape, Vec3, Collider) {
    let resolved = if collider.shape == ColliderShape::Auto { Collider { is_trigger: collider.is_trigger, ..fit_collider(mesh, aabb) } } else { collider.clone() };
    let s = scale.abs();
    let size = resolved.size.abs();
    let shape = match resolved.shape {
        ColliderShape::Sphere => Shape::Sphere { radius: (size.x * s.max_element()).max(0.001) },
        ColliderShape::Capsule => {
            let radius = (size.x * s.x.max(s.z)).max(0.001);
            Shape::Capsule { radius, half_height: (size.y * s.y * 0.5 - radius).max(0.0) }
        }
        _ => Shape::Box { half: (size * s * 0.5).max(Vec3::splat(0.0005)) },
    };
    (shape, resolved.offset * scale, resolved)
}

fn surface(material: Option<&PhysicsMaterial>) -> SurfaceMaterial {
    let m = material.cloned().unwrap_or_default();
    SurfaceMaterial {
        static_friction: m.static_friction,
        dynamic_friction: m.dynamic_friction,
        restitution: m.restitution,
        friction_combine: m.friction_combine,
        restitution_combine: m.restitution_combine,
    }
}

/// Builds a solver body from authored components.
#[allow(clippy::too_many_arguments)]
pub fn build_body(entity: Entity, position: Vec3, rotation: Quat, scale: Vec3, rb: Option<&RigidBody>, collider: Option<&Collider>, material: Option<&PhysicsMaterial>, mesh: Option<&Mesh>, aabb: Option<&Aabb>) -> Body {
    let default_collider = Collider::default();
    let (shape, offset, resolved) = resolve_shape(collider.unwrap_or(&default_collider), mesh, aabb, scale);
    let motion = match rb.map(|r| r.kind) {
        Some(BodyKind::Dynamic) => Motion::Dynamic,
        Some(BodyKind::Kinematic) => Motion::Kinematic,
        _ => Motion::Static,
    };
    let mut body = Body::new(entity.to_bits(), motion, position, rotation, shape).with_material(surface(material));
    body.shape_offset = offset;
    body.is_trigger = resolved.is_trigger;
    let density = material.map(|m| m.density).unwrap_or(PhysicsMaterial::default().density).max(0.001);
    if let Some(rb) = rb {
        body.set_mass(if rb.mass > 0.0 { rb.mass } else { density * shape.volume() });
        body.gravity_scale = rb.gravity_scale;
        body.linear_damping = rb.linear_damping;
        body.angular_damping = rb.angular_damping;
        body.lock_linear = rb.lock_position.as_array();
        body.lock_angular = rb.lock_rotation.as_array();
        body.can_sleep = rb.can_sleep;
        if motion == Motion::Dynamic {
            body.linear_velocity = rb.initial_linear_velocity;
            body.angular_velocity = rb.initial_angular_velocity;
        }
    } else {
        body.set_mass(0.0);
    }
    body
}

fn world_pose(tf: &Transform, parent: Option<&ChildOf>, globals: &Query<&GlobalTransform>) -> (Vec3, Quat, Vec3) {
    if let Some(parent_gt) = parent.and_then(|p| globals.get(p.parent()).ok()) {
        let (s, r, t) = parent_gt.mul_transform(*tf).to_scale_rotation_translation();
        (t, r, s)
    } else {
        (tf.translation, tf.rotation, tf.scale)
    }
}

type PhysicsQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Transform,
        Option<&'static ChildOf>,
        Option<Ref<'static, RigidBody>>,
        Option<Ref<'static, Collider>>,
        Option<Ref<'static, PhysicsMaterial>>,
        Option<&'static Mesh3d>,
        Option<&'static Aabb>,
        Option<&'static Velocity>,
    ),
    (With<PreviewEntity>, Without<EditorInternal>),
>;

/// Steps the simulation while Playing; freezes it while Paused; clears it on Stop.
#[allow(clippy::too_many_arguments)]
pub fn simulate_physics(
    time: Res<Time>,
    state: Res<PlayState>,
    settings: Res<PhysicsSettings>,
    meshes: Res<Assets<Mesh>>,
    mut runtime: ResMut<PhysicsRuntime>,
    mut bodies: PhysicsQuery,
    globals: Query<&GlobalTransform>,
    mut commands: Commands,
    mut trigger_events: MessageWriter<TriggerEvent>,
) {
    if *state == PlayState::Edit {
        if runtime.active {
            *runtime = PhysicsRuntime::default();
        }
        return;
    }
    if *state == PlayState::Paused {
        return;
    }
    runtime.active = true;
    let frame_dt = time.delta_secs();
    let rt = &mut *runtime;

    // 1. Sync ECS → solver.
    let mut seen = HashSet::new();
    for (entity, tf, parent, rb, collider, material, mesh3d, aabb, velocity) in bodies.iter() {
        let participates = rb.is_some() || collider.is_some() || (mesh3d.is_some() && settings.auto_static_meshes);
        if !participates {
            continue;
        }
        seen.insert(entity);
        let (pos, rot, scale) = world_pose(tf, parent, &globals);
        let mesh = mesh3d.and_then(|m| meshes.get(&m.0));
        let changed = rb.as_ref().is_some_and(|r| r.is_changed()) || collider.as_ref().is_some_and(|c| c.is_changed()) || material.as_ref().is_some_and(|m| m.is_changed());
        match rt.index.get(&entity).copied() {
            None => {
                let mut body = build_body(entity, pos, rot, scale, rb.as_deref(), collider.as_deref(), material.as_deref(), mesh, aabb);
                if body.motion == Motion::Dynamic {
                    if let Some(v) = velocity {
                        // Physics owns this body's motion; take Velocity as its start velocity.
                        body.linear_velocity += v.linear;
                        commands.entity(entity).remove::<Velocity>();
                    }
                }
                rt.index.insert(entity, rt.world.bodies.len());
                rt.world.bodies.push(body);
                rt.written.insert(entity, (tf.translation, tf.rotation));
            }
            Some(i) => {
                if changed {
                    let old = rt.world.bodies[i].clone();
                    let mut body = build_body(entity, old.position, old.rotation, scale, rb.as_deref(), collider.as_deref(), material.as_deref(), mesh, aabb);
                    if body.motion == Motion::Dynamic && old.motion == Motion::Dynamic {
                        body.linear_velocity = old.linear_velocity;
                        body.angular_velocity = old.angular_velocity;
                    }
                    rt.world.bodies[i] = body;
                }
                let body = &mut rt.world.bodies[i];
                if body.motion == Motion::Dynamic {
                    if rt.written.get(&entity) != Some(&(tf.translation, tf.rotation)) {
                        // Moved by something else (script, RPC): teleport.
                        body.position = pos;
                        body.rotation = rot;
                        body.wake();
                    }
                } else {
                    if body.motion == Motion::Kinematic && frame_dt > 0.0 {
                        body.linear_velocity = (pos - body.position) / frame_dt;
                        let (axis, angle) = (rot * body.rotation.inverse()).to_axis_angle();
                        let angle = if angle > std::f32::consts::PI { angle - std::f32::consts::TAU } else { angle };
                        body.angular_velocity = if axis.is_finite() { axis * angle / frame_dt } else { Vec3::ZERO };
                    }
                    body.position = pos;
                    body.rotation = rot;
                }
            }
        }
    }
    if seen.len() != rt.index.len() {
        let keep: Vec<Body> = rt.world.bodies.drain(..).filter(|b| seen.contains(&Entity::from_bits(b.key))).collect();
        rt.world.bodies = keep;
        rt.index = rt.world.bodies.iter().enumerate().map(|(i, b)| (Entity::from_bits(b.key), i)).collect();
        rt.written.retain(|e, _| seen.contains(e));
    }

    // 2. Fixed steps.
    let dt = 1.0 / settings.fixed_hz.clamp(15.0, 1000.0);
    let config = SolverConfig {
        gravity: settings.gravity,
        velocity_iterations: settings.velocity_iterations.clamp(1, 100) as usize,
        position_iterations: settings.position_iterations.min(50) as usize,
        allow_sleep: settings.sleeping,
    };
    rt.accumulator += frame_dt.min(0.25);
    // Enough substeps to keep real time even at the 4 Hz unfocused-window redraw rate.
    let max_substeps = (0.25 / dt).ceil() as u32;
    let mut substeps = 0;
    let mut overlaps = HashSet::new();
    while rt.accumulator >= dt {
        rt.world.step(&config, dt);
        overlaps.extend(rt.world.trigger_overlaps.iter().copied());
        rt.accumulator -= dt;
        rt.steps += 1;
        substeps += 1;
        if substeps >= max_substeps {
            rt.accumulator = 0.0;
            break;
        }
    }
    if substeps > 0 {
        for (trigger, other) in overlaps.difference(&rt.overlaps) {
            trigger_events.write(TriggerEvent { trigger: Entity::from_bits(*trigger), other: Entity::from_bits(*other), entered: true });
        }
        for (trigger, other) in rt.overlaps.difference(&overlaps) {
            trigger_events.write(TriggerEvent { trigger: Entity::from_bits(*trigger), other: Entity::from_bits(*other), entered: false });
        }
        rt.overlaps = overlaps;
    }

    // 3. Write dynamic poses back.
    for body in &rt.world.bodies {
        if body.motion != Motion::Dynamic {
            continue;
        }
        let entity = Entity::from_bits(body.key);
        let Ok((_, mut tf, parent, ..)) = bodies.get_mut(entity) else { continue };
        let (translation, rotation) = match parent.and_then(|p| globals.get(p.parent()).ok()) {
            Some(parent_gt) => {
                let local = parent_gt.affine().inverse() * Affine3A::from_scale_rotation_translation(Vec3::ONE, body.rotation, body.position);
                let (_, r, t) = local.to_scale_rotation_translation();
                (t, r)
            }
            None => (body.position, body.rotation),
        };
        if tf.translation != translation || tf.rotation != rotation {
            tf.translation = translation;
            tf.rotation = rotation;
        }
        rt.written.insert(entity, (tf.translation, tf.rotation));
    }
}

impl PhysicsRuntime {
    pub fn is_active(&self) -> bool {
        self.active
    }
    pub fn trigger_overlaps(&self) -> Vec<(Entity, Entity)> {
        self.overlaps.iter().map(|(a, b)| (Entity::from_bits(*a), Entity::from_bits(*b))).collect()
    }
}

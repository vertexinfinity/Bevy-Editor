//! A small, ECS-independent rigid body solver.
//!
//! * Shapes: sphere, oriented box, capsule (Y axis).
//! * Narrow phase: sphere–sphere, sphere–box, box–box (SAT over 15 axes + face clipping,
//!   up to 4 contact points), capsules via closest-point spheres.
//! * Sequential impulses with warm starting, Coulomb friction (static/dynamic), restitution
//!   with a velocity threshold, then a non-energy-adding position correction pass.
//! * Per-body sleeping removes resting jitter.
use super::components::CombineRule;
use bevy::math::{Mat3, Quat, Vec3};
use std::collections::HashMap;

const SLOP: f32 = 0.005;
const POSITION_BETA: f32 = 0.25;
const MAX_CORRECTION: f32 = 0.2;
const RESTITUTION_THRESHOLD: f32 = 0.6;
const STATIC_FRICTION_SPEED: f32 = 0.12;
const SLEEP_LINEAR: f32 = 0.06;
const SLEEP_ANGULAR: f32 = 0.08;
const TIME_TO_SLEEP: f32 = 0.5;
const WAKE_SPEED: f32 = 0.25;
const BROADPHASE_MARGIN: f32 = 0.02;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Sphere { radius: f32 },
    Box { half: Vec3 },
    /// Segment from `-half_height` to `+half_height` along local Y, inflated by `radius`.
    Capsule { radius: f32, half_height: f32 },
}

impl Shape {
    pub fn volume(&self) -> f32 {
        use std::f32::consts::PI;
        match *self {
            Shape::Sphere { radius } => 4.0 / 3.0 * PI * radius.powi(3),
            Shape::Box { half } => 8.0 * half.x * half.y * half.z,
            Shape::Capsule { radius, half_height } => PI * radius * radius * 2.0 * half_height + 4.0 / 3.0 * PI * radius.powi(3),
        }
    }

    /// Principal moments of inertia for a unit mass.
    pub fn unit_inertia(&self) -> Vec3 {
        match *self {
            Shape::Sphere { radius } => Vec3::splat(0.4 * radius * radius),
            Shape::Box { half } => {
                let s = half * 2.0;
                Vec3::new(s.y * s.y + s.z * s.z, s.x * s.x + s.z * s.z, s.x * s.x + s.y * s.y) / 12.0
            }
            Shape::Capsule { radius, half_height } => {
                // Approximated as a box enclosing the capsule.
                let s = Vec3::new(2.0 * radius, 2.0 * (half_height + radius), 2.0 * radius);
                Vec3::new(s.y * s.y + s.z * s.z, s.x * s.x + s.z * s.z, s.x * s.x + s.y * s.y) / 12.0
            }
        }
    }

    pub fn bounding_radius(&self) -> f32 {
        match *self {
            Shape::Sphere { radius } => radius,
            Shape::Box { half } => half.length(),
            Shape::Capsule { radius, half_height } => radius + half_height,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Static,
    Dynamic,
    Kinematic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceMaterial {
    pub static_friction: f32,
    pub dynamic_friction: f32,
    pub restitution: f32,
    pub friction_combine: CombineRule,
    pub restitution_combine: CombineRule,
}

impl Default for SurfaceMaterial {
    fn default() -> Self {
        Self { static_friction: 0.6, dynamic_friction: 0.5, restitution: 0.1, friction_combine: CombineRule::Average, restitution_combine: CombineRule::Average }
    }
}

#[derive(Clone, Debug)]
pub struct Body {
    /// Caller-defined identity (the ECS entity bits).
    pub key: u64,
    pub motion: Motion,
    pub position: Vec3,
    pub rotation: Quat,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    pub shape: Shape,
    /// Shape centre in body space.
    pub shape_offset: Vec3,
    pub material: SurfaceMaterial,
    pub mass: f32,
    pub inv_mass: f32,
    pub inv_inertia_local: Vec3,
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub lock_linear: [bool; 3],
    pub lock_angular: [bool; 3],
    pub is_trigger: bool,
    pub can_sleep: bool,
    pub sleeping: bool,
    pub sleep_timer: f32,
}

impl Body {
    pub fn new(key: u64, motion: Motion, position: Vec3, rotation: Quat, shape: Shape) -> Self {
        let mut body = Self {
            key,
            motion,
            position,
            rotation,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            shape,
            shape_offset: Vec3::ZERO,
            material: SurfaceMaterial::default(),
            mass: 0.0,
            inv_mass: 0.0,
            inv_inertia_local: Vec3::ZERO,
            gravity_scale: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.05,
            lock_linear: [false; 3],
            lock_angular: [false; 3],
            is_trigger: false,
            can_sleep: true,
            sleeping: false,
            sleep_timer: 0.0,
        };
        body.set_mass(shape.volume() * 1000.0);
        body
    }

    pub fn set_mass(&mut self, mass: f32) {
        if self.motion != Motion::Dynamic || mass <= 0.0 {
            self.mass = if self.motion == Motion::Dynamic { mass.max(0.0) } else { 0.0 };
            self.inv_mass = 0.0;
            self.inv_inertia_local = Vec3::ZERO;
            return;
        }
        self.mass = mass;
        self.inv_mass = 1.0 / mass;
        let inertia = self.shape.unit_inertia() * mass;
        self.inv_inertia_local = Vec3::new(1.0 / inertia.x.max(1e-6), 1.0 / inertia.y.max(1e-6), 1.0 / inertia.z.max(1e-6));
    }

    pub fn with_material(mut self, material: SurfaceMaterial) -> Self {
        self.material = material;
        self
    }

    fn simulated(&self) -> bool {
        self.motion == Motion::Dynamic && !self.sleeping
    }

    fn eff_inv_mass(&self) -> f32 {
        if self.simulated() { self.inv_mass } else { 0.0 }
    }

    fn inv_inertia_world(&self) -> Mat3 {
        if !self.simulated() {
            return Mat3::ZERO;
        }
        let r = Mat3::from_quat(self.rotation);
        let mut m = r * Mat3::from_diagonal(self.inv_inertia_local) * r.transpose();
        if self.lock_angular.iter().any(|l| *l) {
            let mut cols = m.to_cols_array_2d();
            for (c, col) in cols.iter_mut().enumerate() {
                for (r, value) in col.iter_mut().enumerate() {
                    if self.lock_angular[c] || self.lock_angular[r] {
                        *value = 0.0;
                    }
                }
            }
            m = Mat3::from_cols_array_2d(&cols);
        }
        m
    }

    fn lin_mask(&self) -> Vec3 {
        Vec3::new(
            if self.lock_linear[0] { 0.0 } else { 1.0 },
            if self.lock_linear[1] { 0.0 } else { 1.0 },
            if self.lock_linear[2] { 0.0 } else { 1.0 },
        )
    }

    fn ang_mask(&self) -> Vec3 {
        Vec3::new(
            if self.lock_angular[0] { 0.0 } else { 1.0 },
            if self.lock_angular[1] { 0.0 } else { 1.0 },
            if self.lock_angular[2] { 0.0 } else { 1.0 },
        )
    }

    pub fn shape_center(&self) -> Vec3 {
        self.position + self.rotation * self.shape_offset
    }

    fn speed_measure(&self) -> f32 {
        self.linear_velocity.length() + self.angular_velocity.length() * self.shape.bounding_radius()
    }

    pub fn wake(&mut self) {
        self.sleeping = false;
        self.sleep_timer = 0.0;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SolverConfig {
    pub gravity: Vec3,
    pub velocity_iterations: usize,
    pub position_iterations: usize,
    pub allow_sleep: bool,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self { gravity: Vec3::new(0.0, -9.81, 0.0), velocity_iterations: 12, position_iterations: 4, allow_sleep: true }
    }
}

/// One contact from the narrow phase. `normal` points from A to B; `point_a` is A's deepest
/// point inside B and `point_b` B's deepest point inside A, so `depth = (point_a - point_b)·normal`.
#[derive(Clone, Copy, Debug)]
pub struct RawContact {
    pub point_a: Vec3,
    pub point_b: Vec3,
    pub normal: Vec3,
    pub depth: f32,
}

impl RawContact {
    fn flipped(self) -> Self {
        Self { point_a: self.point_b, point_b: self.point_a, normal: -self.normal, depth: self.depth }
    }
}

#[derive(Clone, Copy, Debug)]
struct SolverPoint {
    local_a: Vec3,
    local_b: Vec3,
    normal: Vec3,
    ra: Vec3,
    rb: Vec3,
    t1: Vec3,
    t2: Vec3,
    normal_mass: f32,
    t1_mass: f32,
    t2_mass: f32,
    bias: f32,
    friction: f32,
    jn: f32,
    jt1: f32,
    jt2: f32,
}

struct Manifold {
    a: usize,
    b: usize,
    points: Vec<SolverPoint>,
}

#[derive(Clone, Copy)]
struct CachedImpulse {
    local_a: Vec3,
    jn: f32,
    jt: Vec3,
}

/// The simulation world. Add bodies to `bodies` and call [`PhysicsWorld::step`].
#[derive(Default)]
pub struct PhysicsWorld {
    pub bodies: Vec<Body>,
    cache: HashMap<(u64, u64), Vec<CachedImpulse>>,
    /// Pairs (trigger key, other key) currently overlapping.
    pub trigger_overlaps: Vec<(u64, u64)>,
}

#[derive(Clone, Copy)]
struct WorldShape {
    shape: Shape,
    center: Vec3,
    rotation: Quat,
}

impl PhysicsWorld {
    pub fn step(&mut self, config: &SolverConfig, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        // 1. Forces.
        for b in &mut self.bodies {
            if !b.simulated() {
                continue;
            }
            b.linear_velocity += config.gravity * b.gravity_scale * dt;
            b.linear_velocity *= 1.0 / (1.0 + dt * b.linear_damping.max(0.0));
            b.angular_velocity *= 1.0 / (1.0 + dt * b.angular_damping.max(0.0));
            b.linear_velocity *= b.lin_mask();
            b.angular_velocity *= b.ang_mask();
        }

        // 2. Collision detection (O(n²) over pairs that involve a moving body).
        let mut manifolds = Vec::new();
        self.trigger_overlaps.clear();
        let shapes: Vec<WorldShape> = self.bodies.iter().map(|b| WorldShape { shape: b.shape, center: b.shape_center(), rotation: b.rotation }).collect();
        let mut contacts = Vec::new();
        for i in 0..self.bodies.len() {
            for j in (i + 1)..self.bodies.len() {
                let (a, b) = (&self.bodies[i], &self.bodies[j]);
                let moving = |x: &Body| x.motion != Motion::Static;
                let active = |x: &Body| x.simulated() || x.motion == Motion::Kinematic;
                let any_trigger = a.is_trigger || b.is_trigger;
                let dynamic_pair = a.simulated() || b.simulated() || (active(a) && b.motion == Motion::Dynamic) || (active(b) && a.motion == Motion::Dynamic);
                if !any_trigger && !dynamic_pair {
                    continue;
                }
                if any_trigger && !(moving(a) || moving(b)) {
                    continue;
                }
                let reach = a.shape.bounding_radius() + b.shape.bounding_radius() + BROADPHASE_MARGIN;
                if shapes[i].center.distance_squared(shapes[j].center) > reach * reach {
                    continue;
                }
                contacts.clear();
                collide(&shapes[i], &shapes[j], &mut contacts);
                if contacts.is_empty() {
                    continue;
                }
                if any_trigger {
                    if a.is_trigger {
                        self.trigger_overlaps.push((a.key, b.key));
                    }
                    if b.is_trigger {
                        self.trigger_overlaps.push((b.key, a.key));
                    }
                    continue;
                }
                manifolds.push((i, j, contacts.clone()));
            }
        }

        // 3. Wake sleeping bodies hit by something moving.
        for (i, j, _) in &manifolds {
            let (si, sj) = (self.bodies[*i].speed_measure(), self.bodies[*j].speed_measure());
            if self.bodies[*i].sleeping && !self.bodies[*j].sleeping && self.bodies[*j].motion != Motion::Static && sj > WAKE_SPEED {
                self.bodies[*i].wake();
            }
            if self.bodies[*j].sleeping && !self.bodies[*i].sleeping && self.bodies[*i].motion != Motion::Static && si > WAKE_SPEED {
                self.bodies[*j].wake();
            }
        }

        // 4. Build constraints with warm starting.
        let mut solver: Vec<Manifold> = Vec::with_capacity(manifolds.len());
        let mut new_cache: HashMap<(u64, u64), Vec<CachedImpulse>> = HashMap::new();
        for (i, j, raw) in manifolds {
            let (a, b) = (&self.bodies[i], &self.bodies[j]);
            if !a.simulated() && !b.simulated() {
                // Keep impulses of resting stacks so waking does not lose warm starting.
                if let Some(old) = self.cache.get(&(a.key, b.key)) {
                    new_cache.insert((a.key, b.key), old.clone());
                }
                continue;
            }
            let (ma, mb) = (a.material, b.material);
            let static_friction = CombineRule::combine(ma.friction_combine, ma.static_friction, mb.friction_combine, mb.static_friction).max(0.0);
            let dynamic_friction = CombineRule::combine(ma.friction_combine, ma.dynamic_friction, mb.friction_combine, mb.dynamic_friction).max(0.0).min(static_friction.max(0.0) + 1e-6);
            let restitution = CombineRule::combine(ma.restitution_combine, ma.restitution, mb.restitution_combine, mb.restitution).clamp(0.0, 1.0);
            let cached = self.cache.get(&(a.key, b.key));
            let (ima, imb) = (a.eff_inv_mass(), b.eff_inv_mass());
            let (ia, ib) = (a.inv_inertia_world(), b.inv_inertia_world());
            let (ra_inv, rb_inv) = (a.rotation.inverse(), b.rotation.inverse());
            let mut points = Vec::with_capacity(raw.len());
            for c in raw {
                let n = c.normal;
                let mid = (c.point_a + c.point_b) * 0.5;
                let ra = mid - a.position;
                let rb = mid - b.position;
                let k = |dir: Vec3| -> f32 {
                    let ka = ima * (a.lin_mask() * dir).dot(dir) + (ia * ra.cross(dir)).cross(ra).dot(dir);
                    let kb = imb * (b.lin_mask() * dir).dot(dir) + (ib * rb.cross(dir)).cross(rb).dot(dir);
                    let sum = ka + kb;
                    if sum > 1e-9 { 1.0 / sum } else { 0.0 }
                };
                let t1 = n.any_orthonormal_vector();
                let t2 = n.cross(t1);
                let rel = (b.linear_velocity + b.angular_velocity.cross(rb)) - (a.linear_velocity + a.angular_velocity.cross(ra));
                let vn = rel.dot(n);
                let tangential = (rel - n * vn).length();
                let bias = if vn < -RESTITUTION_THRESHOLD { -restitution * vn } else { 0.0 };
                let local_a = ra_inv * (c.point_a - a.position);
                let mut point = SolverPoint {
                    local_a,
                    local_b: rb_inv * (c.point_b - b.position),
                    normal: n,
                    ra,
                    rb,
                    t1,
                    t2,
                    normal_mass: k(n),
                    t1_mass: k(t1),
                    t2_mass: k(t2),
                    bias,
                    friction: if tangential < STATIC_FRICTION_SPEED { static_friction } else { dynamic_friction },
                    jn: 0.0,
                    jt1: 0.0,
                    jt2: 0.0,
                };
                if let Some(old) = cached.and_then(|list| list.iter().find(|o| o.local_a.distance_squared(local_a) < 0.04 * 0.04)) {
                    point.jn = old.jn;
                    point.jt1 = old.jt.dot(t1);
                    point.jt2 = old.jt.dot(t2);
                }
                points.push(point);
            }
            solver.push(Manifold { a: i, b: j, points });
        }

        // Warm start.
        for m in &solver {
            for p in &m.points {
                let impulse = p.normal * p.jn + p.t1 * p.jt1 + p.t2 * p.jt2;
                apply_impulse(&mut self.bodies, m.a, m.b, p.ra, p.rb, impulse);
            }
        }

        // 5. Velocity iterations.
        for _ in 0..config.velocity_iterations.max(1) {
            for m in &mut solver {
                for p in &mut m.points {
                    // Friction (disc clamp against the current normal impulse).
                    let rel = relative_velocity(&self.bodies, m.a, m.b, p.ra, p.rb);
                    let d1 = -rel.dot(p.t1) * p.t1_mass;
                    let d2 = -rel.dot(p.t2) * p.t2_mass;
                    let (old1, old2) = (p.jt1, p.jt2);
                    let (mut n1, mut n2) = (old1 + d1, old2 + d2);
                    let limit = p.friction * p.jn;
                    let len = (n1 * n1 + n2 * n2).sqrt();
                    if len > limit && len > 0.0 {
                        let s = limit / len;
                        n1 *= s;
                        n2 *= s;
                    }
                    p.jt1 = n1;
                    p.jt2 = n2;
                    let impulse = p.t1 * (n1 - old1) + p.t2 * (n2 - old2);
                    apply_impulse(&mut self.bodies, m.a, m.b, p.ra, p.rb, impulse);

                    // Normal.
                    let rel = relative_velocity(&self.bodies, m.a, m.b, p.ra, p.rb);
                    let vn = rel.dot(p.normal);
                    let delta = p.normal_mass * (-vn + p.bias);
                    let new = (p.jn + delta).max(0.0);
                    let applied = new - p.jn;
                    p.jn = new;
                    apply_impulse(&mut self.bodies, m.a, m.b, p.ra, p.rb, p.normal * applied);
                }
            }
        }

        // 6. Integrate positions.
        for b in &mut self.bodies {
            if !b.simulated() {
                continue;
            }
            b.position += b.linear_velocity * dt;
            let w = b.angular_velocity;
            if w.length_squared() > 0.0 {
                let dq = Quat::from_xyzw(w.x, w.y, w.z, 0.0) * b.rotation;
                b.rotation = Quat::from_xyzw(
                    b.rotation.x + 0.5 * dt * dq.x,
                    b.rotation.y + 0.5 * dt * dq.y,
                    b.rotation.z + 0.5 * dt * dq.z,
                    b.rotation.w + 0.5 * dt * dq.w,
                )
                .normalize();
            }
        }

        // 7. Position correction (pseudo impulses applied directly to poses).
        for _ in 0..config.position_iterations {
            for m in &solver {
                for p in &m.points {
                    let (a, b) = (&self.bodies[m.a], &self.bodies[m.b]);
                    let pa = a.position + a.rotation * p.local_a;
                    let pb = b.position + b.rotation * p.local_b;
                    let separation = (pb - pa).dot(p.normal);
                    let c = (POSITION_BETA * (separation + SLOP)).clamp(-MAX_CORRECTION, 0.0);
                    if c >= 0.0 {
                        continue;
                    }
                    let mid = (pa + pb) * 0.5;
                    let ra = mid - a.position;
                    let rb = mid - b.position;
                    let (ima, imb) = (a.eff_inv_mass(), b.eff_inv_mass());
                    let (ia, ib) = (a.inv_inertia_world(), b.inv_inertia_world());
                    let n = p.normal;
                    let k = ima * (a.lin_mask() * n).dot(n) + (ia * ra.cross(n)).cross(ra).dot(n) + imb * (b.lin_mask() * n).dot(n) + (ib * rb.cross(n)).cross(rb).dot(n);
                    if k <= 1e-9 {
                        continue;
                    }
                    let impulse = n * (-c / k);
                    let da = ia * ra.cross(impulse);
                    let db = ib * rb.cross(impulse);
                    let a_mask = a.lin_mask();
                    let b_mask = b.lin_mask();
                    {
                        let a = &mut self.bodies[m.a];
                        a.position -= impulse * ima * a_mask;
                        a.rotation = rotate_by(a.rotation, -da);
                    }
                    {
                        let b = &mut self.bodies[m.b];
                        b.position += impulse * imb * b_mask;
                        b.rotation = rotate_by(b.rotation, db);
                    }
                }
            }
        }

        // 8. Cache impulses for warm starting.
        for m in &solver {
            let key = (self.bodies[m.a].key, self.bodies[m.b].key);
            new_cache.insert(key, m.points.iter().map(|p| CachedImpulse { local_a: p.local_a, jn: p.jn, jt: p.t1 * p.jt1 + p.t2 * p.jt2 }).collect());
        }
        self.cache = new_cache;

        // 9. Sleep.
        for b in &mut self.bodies {
            if !b.simulated() {
                continue;
            }
            if config.allow_sleep && b.can_sleep && b.linear_velocity.length_squared() < SLEEP_LINEAR * SLEEP_LINEAR && b.angular_velocity.length_squared() < SLEEP_ANGULAR * SLEEP_ANGULAR {
                b.sleep_timer += dt;
                if b.sleep_timer > TIME_TO_SLEEP {
                    b.sleeping = true;
                    b.linear_velocity = Vec3::ZERO;
                    b.angular_velocity = Vec3::ZERO;
                }
            } else {
                b.sleep_timer = 0.0;
            }
        }
    }

    pub fn wake_all(&mut self) {
        for b in &mut self.bodies {
            b.wake();
        }
    }
}

fn rotate_by(rotation: Quat, delta: Vec3) -> Quat {
    let angle = delta.length();
    if angle < 1e-9 {
        return rotation;
    }
    (Quat::from_axis_angle(delta / angle, angle) * rotation).normalize()
}

fn relative_velocity(bodies: &[Body], a: usize, b: usize, ra: Vec3, rb: Vec3) -> Vec3 {
    let (a, b) = (&bodies[a], &bodies[b]);
    (b.linear_velocity + b.angular_velocity.cross(rb)) - (a.linear_velocity + a.angular_velocity.cross(ra))
}

fn apply_impulse(bodies: &mut [Body], a: usize, b: usize, ra: Vec3, rb: Vec3, impulse: Vec3) {
    {
        let body = &mut bodies[a];
        if body.simulated() {
            let ii = body.inv_inertia_world();
            body.linear_velocity -= impulse * body.inv_mass * body.lin_mask();
            body.angular_velocity -= ii * ra.cross(impulse);
        }
    }
    let body = &mut bodies[b];
    if body.simulated() {
        let ii = body.inv_inertia_world();
        body.linear_velocity += impulse * body.inv_mass * body.lin_mask();
        body.angular_velocity += ii * rb.cross(impulse);
    }
}

// ---------------------------------------------------------------------------------------------
// Narrow phase
// ---------------------------------------------------------------------------------------------

fn collide(a: &WorldShape, b: &WorldShape, out: &mut Vec<RawContact>) {
    match (a.shape, b.shape) {
        (Shape::Sphere { radius: r1 }, Shape::Sphere { radius: r2 }) => out.extend(sphere_sphere(a.center, r1, b.center, r2)),
        (Shape::Sphere { radius }, Shape::Box { half }) => out.extend(box_sphere(b.center, b.rotation, half, a.center, radius).map(RawContact::flipped)),
        (Shape::Box { half }, Shape::Sphere { radius }) => out.extend(box_sphere(a.center, a.rotation, half, b.center, radius)),
        (Shape::Box { half: ha }, Shape::Box { half: hb }) => box_box(a.center, a.rotation, ha, b.center, b.rotation, hb, out),
        (Shape::Capsule { radius, half_height }, _) => {
            let (p0, p1) = segment(a.center, a.rotation, half_height);
            match b.shape {
                Shape::Sphere { radius: rs } => {
                    let q = closest_on_segment(p0, p1, b.center);
                    out.extend(sphere_sphere(q, radius, b.center, rs));
                }
                Shape::Capsule { radius: r2, half_height: h2 } => {
                    let (q0, q1) = segment(b.center, b.rotation, h2);
                    let (s, t) = closest_segments(p0, p1, q0, q1);
                    out.extend(sphere_sphere(s, radius, t, r2));
                }
                Shape::Box { half } => {
                    for s in capsule_samples(p0, p1) {
                        out.extend(box_sphere(b.center, b.rotation, half, s, radius).map(RawContact::flipped));
                    }
                }
            }
        }
        (_, Shape::Capsule { .. }) => {
            let start = out.len();
            collide(b, a, out);
            for c in &mut out[start..] {
                *c = c.flipped();
            }
        }
    }
}

fn segment(center: Vec3, rotation: Quat, half_height: f32) -> (Vec3, Vec3) {
    let axis = rotation * Vec3::Y * half_height;
    (center - axis, center + axis)
}

fn capsule_samples(p0: Vec3, p1: Vec3) -> Vec<Vec3> {
    if p0.distance_squared(p1) < 1e-8 { vec![p0] } else { vec![p0, (p0 + p1) * 0.5, p1] }
}

fn sphere_sphere(ca: Vec3, ra: f32, cb: Vec3, rb: f32) -> Option<RawContact> {
    let d = cb - ca;
    let dist = d.length();
    let depth = ra + rb - dist;
    if depth < 0.0 {
        return None;
    }
    let n = if dist > 1e-6 { d / dist } else { Vec3::Y };
    Some(RawContact { point_a: ca + n * ra, point_b: cb - n * rb, normal: n, depth })
}

/// Box is A, sphere is B.
fn box_sphere(cb: Vec3, qb: Quat, half: Vec3, cs: Vec3, radius: f32) -> Option<RawContact> {
    let local = qb.inverse() * (cs - cb);
    let clamped = local.clamp(-half, half);
    let diff = local - clamped;
    let dist_sq = diff.length_squared();
    if dist_sq > 1e-10 {
        if dist_sq > radius * radius {
            return None;
        }
        let dist = dist_sq.sqrt();
        let n = qb * (diff / dist);
        return Some(RawContact { point_a: cb + qb * clamped, point_b: cs - n * radius, normal: n, depth: radius - dist });
    }
    // Centre inside the box: push out through the nearest face.
    let gaps = half - local.abs();
    let axis = if gaps.x <= gaps.y && gaps.x <= gaps.z { 0 } else if gaps.y <= gaps.z { 1 } else { 2 };
    let sign = if local[axis] >= 0.0 { 1.0 } else { -1.0 };
    let mut dir = Vec3::ZERO;
    dir[axis] = sign;
    let mut face = local;
    face[axis] = sign * half[axis];
    let n = qb * dir;
    Some(RawContact { point_a: cb + qb * face, point_b: cs - n * radius, normal: n, depth: radius + gaps[axis] })
}

fn closest_on_segment(p0: Vec3, p1: Vec3, q: Vec3) -> Vec3 {
    let d = p1 - p0;
    let len = d.length_squared();
    if len < 1e-12 {
        return p0;
    }
    p0 + d * ((q - p0).dot(d) / len).clamp(0.0, 1.0)
}

/// Closest points between segments p0-p1 and q0-q1 (Ericson, RTCD 5.1.9).
fn closest_segments(p0: Vec3, p1: Vec3, q0: Vec3, q1: Vec3) -> (Vec3, Vec3) {
    let d1 = p1 - p0;
    let d2 = q1 - q0;
    let r = p0 - q0;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);
    let (s, t);
    if a <= 1e-9 && e <= 1e-9 {
        return (p0, q0);
    }
    if a <= 1e-9 {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= 1e-9 {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s0 = if denom > 1e-9 { ((b * f - c * e) / denom).clamp(0.0, 1.0) } else { 0.0 };
            let mut t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t0 = 0.0;
                s0 = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t0 = 1.0;
                s0 = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = s0;
            t = t0;
        }
    }
    (p0 + d1 * s, q0 + d2 * t)
}

fn axes(q: Quat) -> [Vec3; 3] {
    [q * Vec3::X, q * Vec3::Y, q * Vec3::Z]
}

fn box_box(ca: Vec3, qa: Quat, ha: Vec3, cb: Vec3, qb: Quat, hb: Vec3, out: &mut Vec<RawContact>) {
    let ua = axes(qa);
    let ub = axes(qb);
    let t = cb - ca;
    // Returns (overlap, normal A→B) or None if separated along the axis.
    let test = |l: Vec3| -> Option<(f32, Vec3)> {
        let ra = ha.x * ua[0].dot(l).abs() + ha.y * ua[1].dot(l).abs() + ha.z * ua[2].dot(l).abs();
        let rb = hb.x * ub[0].dot(l).abs() + hb.y * ub[1].dot(l).abs() + hb.z * ub[2].dot(l).abs();
        let d = t.dot(l);
        let overlap = ra + rb - d.abs();
        if overlap < 0.0 { None } else { Some((overlap, if d < 0.0 { -l } else { l })) }
    };
    let mut best_a = (f32::MAX, Vec3::ZERO, 0usize);
    for (i, axis) in ua.iter().enumerate() {
        let Some((o, n)) = test(*axis) else { return };
        if o < best_a.0 {
            best_a = (o, n, i);
        }
    }
    let mut best_b = (f32::MAX, Vec3::ZERO, 0usize);
    for (j, axis) in ub.iter().enumerate() {
        let Some((o, n)) = test(*axis) else { return };
        if o < best_b.0 {
            best_b = (o, n, j);
        }
    }
    let mut best_e = (f32::MAX, Vec3::ZERO, 0usize, 0usize);
    for (i, a_axis) in ua.iter().enumerate() {
        for (j, b_axis) in ub.iter().enumerate() {
            let l = a_axis.cross(*b_axis);
            if l.length_squared() < 1e-6 {
                continue;
            }
            let Some((o, n)) = test(l.normalize()) else { return };
            if o < best_e.0 {
                best_e = (o, n, i, j);
            }
        }
    }
    let best_face = best_a.0.min(best_b.0);
    if best_e.0 * 1.05 + 0.005 < best_face {
        // Edge-edge contact.
        let n = best_e.1;
        let (i, j) = (best_e.2, best_e.3);
        let mut pa = ca;
        for k in 0..3 {
            if k != i {
                pa += ua[k] * ha[k] * ua[k].dot(n).signum();
            }
        }
        let mut pb = cb;
        for k in 0..3 {
            if k != j {
                pb -= ub[k] * hb[k] * ub[k].dot(n).signum();
            }
        }
        let (sa, sb) = closest_segments(pa - ua[i] * ha[i], pa + ua[i] * ha[i], pb - ub[j] * hb[j], pb + ub[j] * hb[j]);
        out.push(RawContact { point_a: sa, point_b: sb, normal: n, depth: best_e.0 });
        return;
    }
    if best_a.0 <= best_b.0 * 1.05 + 0.005 {
        for (on_inc, on_ref, depth) in face_clip(ca, ua, ha, best_a.2, best_a.1, cb, ub, hb) {
            // Reference is A: point on A's face, incident point lies inside A.
            out.push(RawContact { point_a: on_ref, point_b: on_inc, normal: best_a.1, depth });
        }
    } else {
        let nr = -best_b.1; // points B→A
        for (on_inc, on_ref, depth) in face_clip(cb, ub, hb, best_b.2, nr, ca, ua, ha) {
            out.push(RawContact { point_a: on_inc, point_b: on_ref, normal: best_b.1, depth });
        }
    }
}

/// Clips the incident box face against the reference face. `nr` points from reference to
/// incident. Returns (point on incident, projection on reference face, depth).
#[allow(clippy::too_many_arguments)]
fn face_clip(rc: Vec3, ru: [Vec3; 3], rh: Vec3, ri: usize, nr: Vec3, ic: Vec3, iu: [Vec3; 3], ih: Vec3) -> Vec<(Vec3, Vec3, f32)> {
    // Incident face: the face of the other box most anti-parallel to nr.
    let mut j = 0;
    let mut best = -1.0;
    for (k, axis) in iu.iter().enumerate() {
        let d = axis.dot(nr).abs();
        if d > best {
            best = d;
            j = k;
        }
    }
    let s = if iu[j].dot(nr) > 0.0 { -1.0 } else { 1.0 };
    let fc = ic + iu[j] * s * ih[j];
    let (k1, k2) = ((j + 1) % 3, (j + 2) % 3);
    let e1 = iu[k1] * ih[k1];
    let e2 = iu[k2] * ih[k2];
    let mut poly = vec![fc + e1 + e2, fc - e1 + e2, fc - e1 - e2, fc + e1 - e2];
    let (r1, r2) = ((ri + 1) % 3, (ri + 2) % 3);
    for (axis, extent) in [(ru[r1], rh[r1]), (-ru[r1], rh[r1]), (ru[r2], rh[r2]), (-ru[r2], rh[r2])] {
        poly = clip_polygon(&poly, axis, axis.dot(rc) + extent);
        if poly.is_empty() {
            return Vec::new();
        }
    }
    let ref_face = rc + nr * rh[ri];
    let mut points: Vec<(Vec3, Vec3, f32)> = poly
        .into_iter()
        .filter_map(|p| {
            let sep = nr.dot(p - ref_face);
            (sep <= 0.0).then(|| (p, p - nr * sep, -sep))
        })
        .collect();
    if points.len() > 4 {
        let dirs = [ru[r1] + ru[r2], ru[r1] - ru[r2], -ru[r1] - ru[r2], -ru[r1] + ru[r2]];
        let mut keep: Vec<usize> = Vec::new();
        for d in dirs {
            let idx = (0..points.len()).max_by(|a, b| points[*a].0.dot(d).total_cmp(&points[*b].0.dot(d))).unwrap_or(0);
            if !keep.contains(&idx) {
                keep.push(idx);
            }
        }
        points = keep.into_iter().map(|i| points[i]).collect();
    }
    points
}

fn clip_polygon(poly: &[Vec3], n: Vec3, d: f32) -> Vec<Vec3> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let da = n.dot(a) - d;
        let db = n.dot(b) - d;
        if da <= 0.0 {
            out.push(a);
        }
        if da * db < 0.0 {
            out.push(a + (b - a) * (da / (da - db)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    fn ground(material: SurfaceMaterial) -> Body {
        Body::new(1, Motion::Static, Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY, Shape::Box { half: Vec3::new(50.0, 0.5, 50.0) }).with_material(material)
    }

    fn run(world: &mut PhysicsWorld, seconds: f32, mut each: impl FnMut(&PhysicsWorld)) {
        let config = SolverConfig::default();
        for _ in 0..(seconds / DT) as usize {
            world.step(&config, DT);
            each(world);
        }
    }

    fn bounce_peak(restitution: f32) -> f32 {
        let mat = SurfaceMaterial { restitution, restitution_combine: CombineRule::Maximum, ..default_mat() };
        let mut world = PhysicsWorld::default();
        world.bodies.push(ground(SurfaceMaterial { restitution: 0.0, ..default_mat() }));
        world.bodies.push(Body::new(2, Motion::Dynamic, Vec3::new(0.0, 5.0, 0.0), Quat::IDENTITY, Shape::Sphere { radius: 0.5 }).with_material(mat));
        let mut bounced = false;
        let mut peak: f32 = 0.0;
        run(&mut world, 3.0, |w| {
            let b = &w.bodies[1];
            if b.linear_velocity.y > 0.5 {
                bounced = true;
            }
            if bounced {
                peak = peak.max(b.position.y - 0.5);
            }
        });
        peak
    }

    fn default_mat() -> SurfaceMaterial {
        SurfaceMaterial::default()
    }

    #[test]
    fn ball_bounces_lower_with_low_restitution() {
        let high = bounce_peak(0.9);
        let low = bounce_peak(0.2);
        // Dropped from 4.5 m above the ground: e² × h.
        assert!(high > 2.5, "bouncy ball peak {high}");
        assert!(low < 0.5, "dull ball peak {low}");
        assert!(high > low * 3.0);
    }

    #[test]
    fn box_comes_to_rest_on_ground() {
        let mut world = PhysicsWorld::default();
        world.bodies.push(ground(default_mat()));
        world.bodies.push(Body::new(2, Motion::Dynamic, Vec3::new(0.3, 3.0, -0.2), Quat::from_rotation_y(0.4), Shape::Box { half: Vec3::splat(0.5) }));
        run(&mut world, 4.0, |_| {});
        let b = &world.bodies[1];
        assert!((b.position.y - 0.5).abs() < 0.02, "rest height {}", b.position.y);
        assert!(b.linear_velocity.length() < 0.05);
        assert!(b.sleeping, "box should sleep once at rest");
        assert!((b.position.x - 0.3).abs() < 0.02 && (b.position.z + 0.2).abs() < 0.02, "no drift: {:?}", b.position);
    }

    #[test]
    fn tilted_box_tips_over_and_lands_flat() {
        let mut world = PhysicsWorld::default();
        world.bodies.push(ground(default_mat()));
        world.bodies.push(Body::new(2, Motion::Dynamic, Vec3::new(0.0, 2.0, 0.0), Quat::from_euler(bevy::math::EulerRot::XYZ, 0.5, 0.3, 0.7), Shape::Box { half: Vec3::splat(0.5) }));
        run(&mut world, 6.0, |_| {});
        let b = &world.bodies[1];
        assert!((b.position.y - 0.5).abs() < 0.03, "landed flat: {:?}", b.position);
        let flat = [Vec3::X, Vec3::Y, Vec3::Z].iter().any(|a| (b.rotation * *a).dot(Vec3::Y).abs() > 0.995);
        assert!(flat, "a face is down: {:?}", b.rotation);
        assert!(b.sleeping);
    }

    #[test]
    fn stack_of_boxes_settles() {
        let mut world = PhysicsWorld::default();
        world.bodies.push(ground(default_mat()));
        for i in 0..4 {
            world.bodies.push(Body::new(10 + i, Motion::Dynamic, Vec3::new(0.0, 0.5 + i as f32 * 1.02, 0.0), Quat::IDENTITY, Shape::Box { half: Vec3::splat(0.5) }));
        }
        run(&mut world, 5.0, |_| {});
        for (i, b) in world.bodies.iter().skip(1).enumerate() {
            let expected = 0.5 + i as f32;
            assert!((b.position.y - expected).abs() < 0.05, "box {i} at {:?}", b.position);
            assert!(b.position.x.abs() < 0.05 && b.position.z.abs() < 0.05, "box {i} drifted {:?}", b.position);
        }
    }

    fn slide_distance(material: SurfaceMaterial) -> f32 {
        let mut world = PhysicsWorld::default();
        world.bodies.push(ground(default_mat()));
        let mut b = Body::new(2, Motion::Dynamic, Vec3::new(0.0, 0.5, 0.0), Quat::IDENTITY, Shape::Box { half: Vec3::splat(0.5) }).with_material(material);
        b.linear_velocity = Vec3::new(6.0, 0.0, 0.0);
        world.bodies.push(b);
        run(&mut world, 4.0, |_| {});
        world.bodies[1].position.x
    }

    #[test]
    fn ice_slides_farther_than_rubber() {
        let ice = SurfaceMaterial { static_friction: 0.1, dynamic_friction: 0.03, restitution: 0.05, friction_combine: CombineRule::Minimum, restitution_combine: CombineRule::Average };
        let rubber = SurfaceMaterial { static_friction: 1.0, dynamic_friction: 0.8, restitution: 0.8, friction_combine: CombineRule::Average, restitution_combine: CombineRule::Maximum };
        let (di, dr) = (slide_distance(ice), slide_distance(rubber));
        assert!(di > dr * 3.0, "ice {di} vs rubber {dr}");
        assert!(dr > 0.5 && dr < 5.0, "rubber stops after a short slide: {dr}");
    }

    #[test]
    fn sphere_rolls_down_ramp_and_box_rests_on_shallow_ramp() {
        let mut world = PhysicsWorld::default();
        let tilt = Quat::from_rotation_z(-0.3);
        world.bodies.push(Body::new(1, Motion::Static, Vec3::ZERO, tilt, Shape::Box { half: Vec3::new(10.0, 0.5, 4.0) }));
        let up = tilt * Vec3::Y;
        world.bodies.push(Body::new(2, Motion::Dynamic, up * 1.01, Quat::IDENTITY, Shape::Sphere { radius: 0.5 }));
        // Friction 1.0 > tan(0.3) ≈ 0.31, so a high-friction box stays put.
        let sticky = SurfaceMaterial { static_friction: 1.0, dynamic_friction: 0.9, friction_combine: CombineRule::Maximum, ..default_mat() };
        world.bodies.push(Body::new(3, Motion::Dynamic, up * 1.01 + tilt * Vec3::new(-3.0, 0.0, 0.0), tilt, Shape::Box { half: Vec3::splat(0.5) }).with_material(sticky));
        let start_box = world.bodies[2].position;
        run(&mut world, 1.5, |_| {});
        assert!(world.bodies[1].position.x > 2.0, "sphere rolled downhill: {:?}", world.bodies[1].position);
        assert!(world.bodies[2].position.distance(start_box) < 0.1, "box stayed: {:?}", world.bodies[2].position);
    }

    #[test]
    fn triggers_report_overlap_without_blocking() {
        let mut world = PhysicsWorld::default();
        let mut t = Body::new(1, Motion::Static, Vec3::ZERO, Quat::IDENTITY, Shape::Box { half: Vec3::splat(1.0) });
        t.is_trigger = true;
        world.bodies.push(t);
        world.bodies.push(Body::new(2, Motion::Dynamic, Vec3::new(0.0, 3.0, 0.0), Quat::IDENTITY, Shape::Sphere { radius: 0.25 }));
        let mut seen = false;
        run(&mut world, 1.0, |w| seen |= w.trigger_overlaps.contains(&(1, 2)));
        assert!(seen);
        assert!(world.bodies[1].position.y < -1.0, "fell through the trigger");
    }

    #[test]
    fn combine_rules() {
        assert_eq!(CombineRule::combine(CombineRule::Average, 0.2, CombineRule::Maximum, 0.8), 0.8);
        assert_eq!(CombineRule::combine(CombineRule::Minimum, 0.2, CombineRule::Average, 0.8), 0.2);
        assert!((CombineRule::combine(CombineRule::Average, 0.2, CombineRule::Average, 0.8) - 0.5).abs() < 1e-6);
    }
}

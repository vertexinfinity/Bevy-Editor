//! JSON-RPC methods (see the module docs in `physics/mod.rs`).
use super::components::*;
use super::editor::{edit_physics, fitted_collider, set_shape_size, PhysicsSnapshot};
use super::presets;
use super::runtime::PhysicsRuntime;
use super::solver::{Motion, Shape};
use crate::core::play_mode::PlaySession;
use crate::scene::SceneEntity;
use bevy::{inspector::InspectorSelection, prelude::*};
use serde_json::{json, Value};

type RpcResult = Result<Value, (i32, String)>;

pub fn dispatch(world: &mut World, method: &str, params: Option<Value>) -> Option<RpcResult> {
    let method = method.strip_prefix("editor/").unwrap_or(method);
    let p = params.unwrap_or(Value::Null);
    Some(match method {
        "physics_presets" => Ok(presets_json()),
        "physics_material" => physics_material(world, &p),
        "rigid_body" => rigid_body(world, &p),
        "collider" => collider(world, &p),
        "physics_settings" => physics_settings(world, &p),
        "physics_save_preset" => save_preset(world, &p),
        "physics_state" => Ok(physics_state(world)),
        "physics_game_view_screenshot" => game_view_screenshot(world, &p),
        "physics_reveal_card" => reveal_card(world),
        _ => return None,
    })
}

fn invalid(msg: impl Into<String>) -> (i32, String) {
    (-32602, msg.into())
}

fn failed(msg: String) -> (i32, String) {
    (-32010, msg)
}

fn resolve_entity(world: &mut World, p: &Value, name_key: &str) -> Result<Entity, (i32, String)> {
    if let Some(bits) = p.get("entity").and_then(Value::as_u64) {
        return Entity::try_from_bits(bits).filter(|e| world.get_entity(*e).is_ok()).ok_or_else(|| invalid(format!("No entity exists with ID {bits}")));
    }
    if let Some(name) = p.get(name_key).and_then(Value::as_str) {
        let mut q = world.query_filtered::<(Entity, &Name), With<SceneEntity>>();
        return q.iter(world).find(|(_, n)| n.as_str() == name).map(|(e, _)| e).ok_or_else(|| invalid(format!("No scene entity named '{name}'")));
    }
    world.resource::<InspectorSelection>().0.filter(|e| world.get_entity(*e).is_ok()).ok_or_else(|| invalid("Pass 'entity' or 'name', or select an entity"))
}

fn f32_param(p: &Value, key: &str) -> Result<Option<f32>, (i32, String)> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().map(|v| Some(v as f32)).ok_or_else(|| invalid(format!("'{key}' must be a number"))),
    }
}

fn vec3_param(p: &Value, key: &str) -> Result<Option<Vec3>, (i32, String)> {
    match p.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() == 3 && a.iter().all(Value::is_number) => Ok(Some(Vec3::new(a[0].as_f64().unwrap_or(0.0) as f32, a[1].as_f64().unwrap_or(0.0) as f32, a[2].as_f64().unwrap_or(0.0) as f32))),
        Some(Value::Number(n)) => Ok(Some(Vec3::splat(n.as_f64().unwrap_or(0.0) as f32))),
        _ => Err(invalid(format!("'{key}' must be [x, y, z]"))),
    }
}

fn bool_param(p: &Value, key: &str) -> Option<bool> {
    p.get(key).and_then(Value::as_bool)
}

fn locks_param(p: &Value, key: &str) -> Option<AxisLocks> {
    match p.get(key)? {
        Value::Bool(b) => Some(AxisLocks { x: *b, y: *b, z: *b }),
        Value::Array(a) if a.len() == 3 => Some(AxisLocks { x: a[0].as_bool().unwrap_or(false), y: a[1].as_bool().unwrap_or(false), z: a[2].as_bool().unwrap_or(false) }),
        _ => None,
    }
}

fn snapshot_json(entity: Entity, s: &PhysicsSnapshot) -> Value {
    json!({ "entity": entity.to_bits(), "rigid_body": s.rigid_body, "collider": s.collider, "material": s.material })
}

fn presets_json() -> Value {
    let custom = presets::custom_presets();
    let mut list: Vec<Value> = presets::builtin_presets().into_iter().map(|p| json!({ "builtin": true, "material": p })).collect();
    for (m, path) in custom {
        if !presets::is_builtin(&m.preset) {
            list.push(json!({ "builtin": false, "path": path.to_string_lossy(), "material": m }));
        }
    }
    json!({ "presets": list, "custom_dir": presets::PRESET_DIR })
}

/// Applies material params (`friction` sets both coefficients) on top of `base`.
fn material_from_params(p: &Value, mut m: PhysicsMaterial) -> Result<PhysicsMaterial, (i32, String)> {
    if let Some(v) = f32_param(p, "friction")? {
        m.static_friction = v;
        m.dynamic_friction = v;
    }
    if let Some(v) = f32_param(p, "static_friction")? {
        m.static_friction = v;
    }
    if let Some(v) = f32_param(p, "dynamic_friction")? {
        m.dynamic_friction = v;
    }
    if let Some(v) = f32_param(p, "restitution")? {
        m.restitution = v.clamp(0.0, 1.0);
    }
    if let Some(v) = f32_param(p, "density")? {
        if v <= 0.0 {
            return Err(invalid("'density' must be > 0"));
        }
        m.density = v;
    }
    for (key, slot) in [("friction_combine", &mut m.friction_combine), ("restitution_combine", &mut m.restitution_combine)] {
        if let Some(s) = p.get(key).and_then(Value::as_str) {
            *slot = CombineRule::parse(s).ok_or_else(|| invalid(format!("'{key}' must be Average, Minimum, Multiply or Maximum")))?;
        }
    }
    Ok(m)
}

fn physics_material(world: &mut World, p: &Value) -> RpcResult {
    let entity = resolve_entity(world, p, "name")?;
    if bool_param(p, "remove") == Some(true) {
        let s = edit_physics(world, entity, "Remove Physics Material", |_, s| s.material = None).map_err(failed)?;
        return Ok(snapshot_json(entity, &s));
    }
    let base = match p.get("preset").and_then(Value::as_str) {
        Some(name) => presets::find_preset(name).ok_or_else(|| invalid(format!("Unknown preset '{name}' (see editor/physics_presets)")))?,
        None => world.get::<PhysicsMaterial>(entity).cloned().unwrap_or_default(),
    };
    let material = material_from_params(p, base)?;
    let s = edit_physics(world, entity, "Edit Physics Material", |_, s| s.material = Some(material)).map_err(failed)?;
    Ok(snapshot_json(entity, &s))
}

fn rigid_body(world: &mut World, p: &Value) -> RpcResult {
    let entity = resolve_entity(world, p, "name")?;
    if bool_param(p, "remove") == Some(true) {
        let s = edit_physics(world, entity, "Remove Rigid Body", |_, s| s.rigid_body = None).map_err(failed)?;
        return Ok(snapshot_json(entity, &s));
    }
    let mut rb = world.get::<RigidBody>(entity).cloned().unwrap_or_default();
    if let Some(kind) = p.get("kind").and_then(Value::as_str) {
        rb.kind = BodyKind::parse(kind).ok_or_else(|| invalid("'kind' must be Static, Dynamic or Kinematic"))?;
    }
    if let Some(v) = f32_param(p, "mass")? {
        rb.mass = v.max(0.0);
    }
    if let Some(v) = f32_param(p, "gravity_scale")? {
        rb.gravity_scale = v;
    }
    if let Some(v) = f32_param(p, "linear_damping")? {
        rb.linear_damping = v.max(0.0);
    }
    if let Some(v) = f32_param(p, "angular_damping")? {
        rb.angular_damping = v.max(0.0);
    }
    if let Some(v) = vec3_param(p, "initial_linear_velocity")?.or(vec3_param(p, "velocity")?) {
        rb.initial_linear_velocity = v;
    }
    if let Some(v) = vec3_param(p, "initial_angular_velocity")?.or(vec3_param(p, "angular_velocity")?) {
        rb.initial_angular_velocity = v;
    }
    if let Some(l) = locks_param(p, "lock_position") {
        rb.lock_position = l;
    }
    if let Some(l) = locks_param(p, "lock_rotation") {
        rb.lock_rotation = l;
    }
    if let Some(b) = bool_param(p, "can_sleep") {
        rb.can_sleep = b;
    }
    let fitted = fitted_collider(world, entity);
    let add_collider = bool_param(p, "add_collider").unwrap_or(true);
    let s = edit_physics(world, entity, "Edit Rigid Body", |_, s| {
        s.rigid_body = Some(rb);
        if add_collider {
            s.collider.get_or_insert(fitted);
        }
    })
    .map_err(failed)?;
    Ok(snapshot_json(entity, &s))
}

fn collider(world: &mut World, p: &Value) -> RpcResult {
    let entity = resolve_entity(world, p, "name")?;
    if bool_param(p, "remove") == Some(true) {
        let s = edit_physics(world, entity, "Remove Collider", |_, s| s.collider = None).map_err(failed)?;
        return Ok(snapshot_json(entity, &s));
    }
    let fitted = fitted_collider(world, entity);
    let existing = world.get::<Collider>(entity).cloned();
    let mut c = existing.clone().unwrap_or_else(|| fitted.clone());
    if let Some(shape) = p.get("shape").and_then(Value::as_str) {
        let shape = ColliderShape::parse(shape).ok_or_else(|| invalid("'shape' must be Auto, Box, Sphere or Capsule"))?;
        if shape != c.shape {
            c.shape = shape;
            set_shape_size(&mut c, &fitted);
        }
    }
    if let Some(v) = vec3_param(p, "size")? {
        c.size = v;
    }
    if let Some(v) = f32_param(p, "radius")? {
        c.size.x = v;
    }
    if let Some(v) = f32_param(p, "height")? {
        c.size.y = v;
    }
    if let Some(v) = vec3_param(p, "offset")? {
        c.offset = v;
    }
    if let Some(b) = bool_param(p, "is_trigger") {
        c.is_trigger = b;
    }
    let s = edit_physics(world, entity, "Edit Collider", |_, s| s.collider = Some(c)).map_err(failed)?;
    Ok(snapshot_json(entity, &s))
}

pub fn settings_path() -> std::path::PathBuf {
    std::path::Path::new(presets::PRESET_DIR).join("settings.json")
}

fn physics_settings(world: &mut World, p: &Value) -> RpcResult {
    let mut s = world.resource::<PhysicsSettings>().clone();
    let before = s.clone();
    if let Some(g) = vec3_param(p, "gravity")? {
        s.gravity = g;
    }
    if let Some(v) = f32_param(p, "fixed_hz")? {
        s.fixed_hz = v.clamp(15.0, 1000.0);
    }
    if let Some(v) = p.get("velocity_iterations").and_then(Value::as_u64) {
        s.velocity_iterations = v.clamp(1, 100) as u32;
    }
    if let Some(v) = p.get("position_iterations").and_then(Value::as_u64) {
        s.position_iterations = v.min(50) as u32;
    }
    if let Some(b) = bool_param(p, "auto_static_meshes") {
        s.auto_static_meshes = b;
    }
    if let Some(b) = bool_param(p, "sleeping") {
        s.sleeping = b;
    }
    if let Some(b) = bool_param(p, "show_all_colliders") {
        s.show_all_colliders = b;
    }
    let mut saved = None;
    if s != before {
        *world.resource_mut::<PhysicsSettings>() = s.clone();
        world.resource_mut::<PhysicsRuntime>().world.wake_all();
        let path = settings_path();
        let result = std::fs::create_dir_all(presets::PRESET_DIR).and_then(|_| std::fs::write(&path, serde_json::to_string_pretty(&s).unwrap_or_default()));
        saved = Some(match result {
            Ok(()) => path.to_string_lossy().to_string(),
            Err(e) => format!("not saved: {e}"),
        });
    }
    Ok(json!({ "settings": s, "saved_to": saved }))
}

fn save_preset(world: &mut World, p: &Value) -> RpcResult {
    let name = p.get("name").or_else(|| p.get("preset")).and_then(Value::as_str).ok_or_else(|| invalid("Missing 'name' (the preset name)"))?.to_string();
    let source = if p.get("entity").is_some() || p.get("source").is_some() { Some(resolve_entity(world, p, "source")?) } else { None };
    let base = match (p.get("base").and_then(Value::as_str), source) {
        (Some(b), _) => presets::find_preset(b).ok_or_else(|| invalid(format!("Unknown base preset '{b}'")))?,
        (None, Some(e)) => world.get::<PhysicsMaterial>(e).cloned().unwrap_or_default(),
        (None, None) => PhysicsMaterial::default(),
    };
    let mut material = material_from_params(p, base)?;
    material.preset = name.clone();
    let path = presets::save_preset(&name, &material).map_err(failed)?;
    if let (Some(entity), true) = (source, bool_param(p, "apply").unwrap_or(true)) {
        let m = material.clone();
        edit_physics(world, entity, "Apply Physics Material", |_, s| s.material = Some(m)).map_err(failed)?;
    }
    Ok(json!({ "status": "saved", "path": path.to_string_lossy(), "material": material }))
}

fn physics_state(world: &mut World) -> Value {
    let reverse: std::collections::HashMap<Entity, Entity> = world.resource::<PlaySession>().entity_map.iter().map(|(src, rt)| (*rt, *src)).collect();
    let runtime = world.resource::<PhysicsRuntime>();
    let bodies: Vec<Value> = runtime
        .world
        .bodies
        .iter()
        .map(|b| {
            let e = Entity::from_bits(b.key);
            let name = world.get::<Name>(e).map(|n| n.as_str().to_string());
            let shape = match b.shape {
                Shape::Sphere { radius } => json!({ "sphere": { "radius": radius } }),
                Shape::Box { half } => json!({ "box": { "size": (half * 2.0).to_array() } }),
                Shape::Capsule { radius, half_height } => json!({ "capsule": { "radius": radius, "height": 2.0 * (half_height + radius) } }),
            };
            json!({
                "entity": b.key,
                "source_entity": reverse.get(&e).map(|s| s.to_bits()),
                "name": name,
                "motion": match b.motion { Motion::Static => "Static", Motion::Dynamic => "Dynamic", Motion::Kinematic => "Kinematic" },
                "position": b.position.to_array(),
                "rotation_xyzw": b.rotation.to_array(),
                "linear_velocity": b.linear_velocity.to_array(),
                "angular_velocity": b.angular_velocity.to_array(),
                "mass": b.mass,
                "sleeping": b.sleeping,
                "trigger": b.is_trigger,
                "shape": shape,
            })
        })
        .collect();
    let triggers: Vec<Value> = runtime.trigger_overlaps().into_iter().map(|(t, o)| json!({ "trigger": t.to_bits(), "other": o.to_bits() })).collect();
    json!({ "active": runtime.is_active(), "steps": runtime.steps, "bodies": bodies, "trigger_overlaps": triggers })
}

/// Captures the Game View window (the stock `editor/screenshot` only sees the main window).
fn game_view_screenshot(world: &mut World, p: &Value) -> RpcResult {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let path = p.get("path").and_then(Value::as_str).ok_or_else(|| invalid("Missing 'path'"))?.to_string();
    let window = world.resource::<PlaySession>().window.filter(|w| world.get::<Window>(*w).is_some()).ok_or_else(|| invalid("Game View is not open (enter Play mode first)"))?;
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    world.spawn(Screenshot::window(window)).observe(save_to_disk(path.clone()));
    Ok(json!({ "status": "capturing", "path": path }))
}

/// Scrolls the inspector so the Physics card is at the top (for screenshots by agents).
fn reveal_card(world: &mut World) -> RpcResult {
    let card = world
        .query_filtered::<&ChildOf, With<super::editor::PhysicsCardRoot>>()
        .iter(world)
        .next()
        .map(|c| c.parent())
        .ok_or_else(|| invalid("Physics card not found (is the editor open?)"))?;
    let mut scroller = world.get::<ChildOf>(card).map(|c| c.parent());
    while let Some(e) = scroller {
        if world.get::<Node>(e).is_some_and(|n| n.overflow.y == OverflowAxis::Scroll) {
            break;
        }
        scroller = world.get::<ChildOf>(e).map(|c| c.parent());
    }
    let scroller = scroller.ok_or_else(|| invalid("Inspector scroll area not found"))?;
    let top = |world: &World, e: Entity| -> Option<(f32, f32)> {
        let node = world.get::<ComputedNode>(e)?;
        let t = world.get::<UiGlobalTransform>(e)?;
        Some((t.translation.y - node.size.y * 0.5, node.inverse_scale_factor))
    };
    let ((card_top, inv), (scroll_top, _)) = top(world, card).zip(top(world, scroller)).ok_or_else(|| invalid("Layout not computed yet"))?;
    let current = world.get::<ScrollPosition>(scroller).map(|s| s.0).unwrap_or_default();
    let y = (current.y + (card_top - scroll_top) * inv).max(0.0);
    world.entity_mut(scroller).insert(ScrollPosition(Vec2::new(current.x, y)));
    Ok(json!({ "status": "ok", "scroll_y": y }))
}

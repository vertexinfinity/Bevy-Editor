//! Remote methods for the reflection-driven inspector features.
//!
//! | Method | Params | Purpose |
//! | :-- | :-- | :-- |
//! | `editor/components` | `entity`/`name` | Reflected components with fields, values, units, ranges |
//! | `editor/set_field` | `entity`/`name`, `component`, `field`, `value` | Edit any reflected field (undoable) |
//! | `editor/add_component` | `entity`/`name`, `component` | Insert a component by type name using its `Default` |
//! | `editor/remove_component` | `entity`/`name`, `component` | Remove a component |
//! | `editor/component_types` | `filter`? | Registered component types that can be added |
//! | `editor/world` | `set`? `{field: value}` | Read or edit World / Environment settings |
//! | `editor/camera_rendering` | `entity`/`name`, `features`?, `anti_aliasing`?, `tonemapping`?, `preview`? | Camera post-process stack |
//! | `editor/mesh_rendering` | `entity`/`name`, `cast_shadows`?, `receive_shadows`?, `wireframe`?, `show_bounds`? | Per-object rendering flags |
//! | `editor/viewport` | `wireframe`?, `grid`?, `gizmos`?, `bounds`? | Viewport overlay toggles |
//! | `editor/time_scale` | `scale`? | Simulation speed (`Time<Virtual>` relative speed) |
//! | `editor/cameras` | | Every camera (editor, overlay, scene, game view) with order, HDR and activity |
//!
//! Unknown methods return `None` so the main dispatcher can report them.
use bevy::{ecs::reflect::ReflectComponent, prelude::*};
use serde_json::{json, Value};

use super::{invalid_params, resolve_target_entity};
use crate::core::reflect_fields;
use crate::ui::inspector::camera_card::{self, AntiAliasing, CAMERA_FEATURES, ViewportPreview};
use crate::ui::inspector::object_card::{self, OBJECT_TOGGLES};

type RpcResult = Result<Value, (i32, String)>;

pub(super) fn dispatch(world: &mut World, method: &str, params: Option<Value>) -> Option<RpcResult> {
    let method = method.strip_prefix("editor/").unwrap_or(method);
    Some(match method {
        "components" => components(world, params),
        "set_field" => set_field(world, params),
        "add_component" => toggle_component(world, params, true),
        "remove_component" => toggle_component(world, params, false),
        "component_types" => component_types(world, params),
        "world" => world_settings(world, params),
        "camera_rendering" => camera_rendering(world, params),
        "mesh_rendering" => mesh_rendering(world, params),
        "viewport" => viewport(world, params),
        "time_scale" => time_scale(world, params),
        "cameras" => cameras(world),
        _ => return None,
    })
}

fn str_param<'a>(params: &'a Option<Value>, key: &str) -> Result<&'a str, (i32, String)> {
    params
        .as_ref()
        .and_then(|p| p.get(key))
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params(format!("Missing string param '{key}'")))
}

/// JSON scalars become the text the inspector would accept.
fn value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn err(e: String) -> (i32, String) {
    (-32010, e)
}

fn components(world: &mut World, params: Option<Value>) -> RpcResult {
    let entity = resolve_target_entity(world, params.as_ref())?;
    Ok(reflect_fields::describe_entity_json(world, entity))
}

fn set_field(world: &mut World, params: Option<Value>) -> RpcResult {
    let entity = resolve_target_entity(world, params.as_ref())?;
    let component = str_param(&params, "component")?.to_string();
    let field = params.as_ref().and_then(|p| p.get("field")).and_then(Value::as_str).unwrap_or("").to_string();
    let value = params.as_ref().and_then(|p| p.get("value")).map(value_text).ok_or_else(|| invalid_params("Missing param 'value'"))?;
    reflect_fields::set_field_with_undo(world, entity, &component, &field, &value).map_err(err)?;
    Ok(json!({
        "status": "ok",
        "entity": entity.to_bits(),
        "component": component,
        "field": field,
        "value": reflect_fields::read_field(world, entity, &component, &field),
    }))
}

fn toggle_component(world: &mut World, params: Option<Value>, insert: bool) -> RpcResult {
    let entity = resolve_target_entity(world, params.as_ref())?;
    let component = str_param(&params, "component")?.to_string();
    let command = if insert {
        reflect_fields::ToggleReflectComponent::insert(entity, component.clone())
    } else {
        reflect_fields::ToggleReflectComponent::remove(entity, component.clone())
    };
    reflect_fields::execute_command(world, command).map_err(err)?;
    Ok(json!({ "status": "ok", "entity": entity.to_bits(), "component": component, "present": insert }))
}

fn component_types(world: &mut World, params: Option<Value>) -> RpcResult {
    let filter = params.as_ref().and_then(|p| p.get("filter")).and_then(Value::as_str).map(str::to_lowercase);
    let registry = world.resource::<AppTypeRegistry>().read();
    let mut types: Vec<_> = registry
        .iter()
        .filter(|r| r.data::<ReflectComponent>().is_some() && r.data::<ReflectDefault>().is_some())
        .map(|r| r.type_info().type_path().to_string())
        .filter(|p| !reflect_fields::is_hidden(p))
        .filter(|p| filter.as_ref().is_none_or(|f| p.to_lowercase().contains(f)))
        .collect();
    types.sort();
    Ok(json!({ "count": types.len(), "types": types }))
}

fn world_settings(world: &mut World, params: Option<Value>) -> RpcResult {
    let entity = crate::scene::environment::world_settings_entity(world).ok_or_else(|| err("World settings entity is missing".into()))?;
    if let Some(set) = params.as_ref().and_then(|p| p.get("set")).and_then(Value::as_object) {
        for (field, value) in set {
            reflect_fields::set_field_with_undo(world, entity, "WorldEnvironment", field, &value_text(value)).map_err(|e| err(format!("{field}: {e}")))?;
        }
    }
    let mut described = reflect_fields::describe_entity_json(world, entity);
    let fields = described["components"]
        .as_array_mut()
        .and_then(|c| c.iter_mut().find(|c| c["type_path"].as_str().is_some_and(|p| p.ends_with("WorldEnvironment"))))
        .map(|c| c["fields"].take())
        .unwrap_or(Value::Null);
    Ok(json!({ "fields": fields }))
}

fn camera_entity(world: &mut World, params: &Option<Value>) -> Result<Entity, (i32, String)> {
    if params.as_ref().is_some_and(|p| p.get("entity").is_some() || p.get("name").is_some()) {
        let entity = resolve_target_entity(world, params.as_ref())?;
        if world.get::<Camera3d>(entity).is_none() {
            return Err(invalid_params("Target entity is not a 3D camera"));
        }
        return Ok(entity);
    }
    if let Ok(selected) = resolve_target_entity(world, None) {
        if world.get::<Camera3d>(selected).is_some() {
            return Ok(selected);
        }
    }
    world
        .query_filtered::<Entity, (With<Camera3d>, With<crate::scene::SceneEntity>)>()
        .iter(world)
        .min()
        .ok_or_else(|| invalid_params("No scene camera found; spawn one with editor/spawn_primitive {\"kind\":\"camera\"}"))
}

fn camera_rendering(world: &mut World, params: Option<Value>) -> RpcResult {
    let camera = camera_entity(world, &params)?;
    if let Some(features) = params.as_ref().and_then(|p| p.get("features")).and_then(Value::as_object) {
        for (name, on) in features {
            let feature = CAMERA_FEATURES
                .iter()
                .find(|f| f.1.eq_ignore_ascii_case(name) || f.0.eq_ignore_ascii_case(name))
                .ok_or_else(|| invalid_params(format!("Unknown feature '{name}'")))?;
            let on = on.as_bool().ok_or_else(|| invalid_params(format!("Feature '{name}' expects true/false")))?;
            camera_card::set_feature(world, camera, feature.1, on, feature.3).map_err(err)?;
        }
    }
    if let Some(mode) = params.as_ref().and_then(|p| p.get("anti_aliasing")).and_then(Value::as_str) {
        let mode = AntiAliasing::parse(mode).ok_or_else(|| invalid_params("anti_aliasing must be off, msaa, fxaa, smaa or taa"))?;
        camera_card::set_anti_aliasing(world, camera, mode);
    }
    if let Some(tonemapping) = params.as_ref().and_then(|p| p.get("tonemapping")).and_then(Value::as_str) {
        reflect_fields::set_field_with_undo(world, camera, "Tonemapping", "", tonemapping).map_err(err)?;
    }
    if let Some(preview) = params.as_ref().and_then(|p| p.get("preview")).and_then(Value::as_bool) {
        world.resource_mut::<ViewportPreview>().source = preview.then_some(camera);
    }
    let features: serde_json::Map<String, Value> = CAMERA_FEATURES
        .iter()
        .map(|f| (f.1.to_string(), Value::Bool(camera_card::has_component(world, camera, f.1))))
        .collect();
    Ok(json!({
        "camera": camera.to_bits(),
        "features": features,
        "anti_aliasing": camera_card::anti_aliasing(world, camera).label(),
        "tonemapping": reflect_fields::read_field(world, camera, "Tonemapping", ""),
        "preview": world.resource::<ViewportPreview>().source == Some(camera),
    }))
}

fn mesh_rendering(world: &mut World, params: Option<Value>) -> RpcResult {
    let entity = resolve_target_entity(world, params.as_ref())?;
    let keys = ["cast_shadows", "receive_shadows", "wireframe", "show_bounds"];
    for ((_, type_path, inverted), key) in OBJECT_TOGGLES.iter().zip(keys) {
        if let Some(on) = params.as_ref().and_then(|p| p.get(key)).and_then(Value::as_bool) {
            object_card::set_object_flag(world, entity, type_path, on, *inverted).map_err(err)?;
        }
    }
    let state: serde_json::Map<String, Value> = OBJECT_TOGGLES
        .iter()
        .zip(keys)
        .map(|((_, type_path, inverted), key)| (key.to_string(), Value::Bool(object_card::object_flag(world, entity, type_path, *inverted))))
        .collect();
    Ok(json!({ "entity": entity.to_bits(), "state": state }))
}

fn viewport(world: &mut World, params: Option<Value>) -> RpcResult {
    let get = |key: &str| params.as_ref().and_then(|p| p.get(key)).and_then(Value::as_bool);
    let mut settings = world.resource_mut::<crate::core::selection::ViewportGizmoSettings>();
    if let Some(v) = get("wireframe") { settings.show_wireframe = v; }
    if let Some(v) = get("grid") { settings.show_grid = v; }
    if let Some(v) = get("gizmos") { settings.show_gizmos = v; }
    if let Some(v) = get("bounds") { settings.show_bounds = v; }
    Ok(json!({
        "wireframe": settings.show_wireframe,
        "grid": settings.show_grid,
        "gizmos": settings.show_gizmos,
        "bounds": settings.show_bounds,
    }))
}

fn time_scale(world: &mut World, params: Option<Value>) -> RpcResult {
    if let Some(scale) = params.as_ref().and_then(|p| p.get("scale")).and_then(Value::as_f64) {
        if !(0.0..=16.0).contains(&scale) {
            return Err(invalid_params("scale must be between 0 and 16"));
        }
        world.resource_mut::<Time<Virtual>>().set_relative_speed(scale as f32);
    }
    Ok(json!({ "scale": world.resource::<Time<Virtual>>().relative_speed() }))
}

fn cameras(world: &mut World) -> RpcResult {
    let list: Vec<Value> = world
        .query::<(Entity, &Camera, Option<&Name>, Has<bevy::camera::Hdr>, Has<Camera3d>, Has<crate::scene::SceneEntity>)>()
        .iter(world)
        .map(|(e, camera, name, hdr, is_3d, scene)| {
            json!({
                "entity": e.to_bits(),
                "name": name.map(|n| n.as_str().to_string()),
                "order": camera.order,
                "active": camera.is_active,
                "hdr": hdr,
                "camera3d": is_3d,
                "scene_camera": scene,
            })
        })
        .collect();
    Ok(json!({ "cameras": list }))
}
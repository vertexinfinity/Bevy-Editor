//! JSON-RPC methods of the shader editor (see the module docs in `mod.rs`).
use bevy::prelude::*;
use serde_json::{json, Value};

use super::component::{set_shader_ref_with_undo, ShaderMaterialRef};
use super::library::{self, ShaderLibrary};
use super::ops::{self, ShaderEditorState};

type RpcResult = Result<Value, (i32, String)>;

fn invalid(message: impl Into<String>) -> (i32, String) {
    (-32602, message.into())
}

fn str_param<'a>(params: &'a Option<Value>, key: &str) -> Option<&'a str> {
    params.as_ref().and_then(|p| p.get(key)).and_then(Value::as_str)
}

fn resolve_entity(world: &mut World, params: &Option<Value>) -> Result<Entity, (i32, String)> {
    if let Some(bits) = params.as_ref().and_then(|p| p.get("entity")).and_then(Value::as_u64) {
        return Entity::try_from_bits(bits).filter(|e| world.get_entity(*e).is_ok()).ok_or_else(|| invalid(format!("No entity exists with ID {bits}")));
    }
    if let Some(name) = str_param(params, "name") {
        let mut scene = world.query_filtered::<(Entity, &Name), With<crate::scene::SceneEntity>>();
        if let Some((e, _)) = scene.iter(world).find(|(_, n)| n.as_str() == name) {
            return Ok(e);
        }
        let mut all = world.query::<(Entity, &Name)>();
        return all.iter(world).find(|(_, n)| n.as_str() == name).map(|(e, _)| e).ok_or_else(|| invalid(format!("No entity found with name '{name}'")));
    }
    ops::selected_entity(world).ok_or_else(|| invalid("No target entity specified and no entity is currently selected"))
}

/// `color` accepts "#RRGGBB[AA]" or [r, g, b, a?] (sRGB 0..1).
fn color_param(params: &Option<Value>) -> Result<Option<Color>, (i32, String)> {
    let Some(value) = params.as_ref().and_then(|p| p.get("color")) else { return Ok(None) };
    if let Some(hex) = value.as_str() {
        return crate::core::reflect_fields::parse_color(hex).map(Some).map_err(invalid);
    }
    if let Some(list) = value.as_array() {
        let c: Vec<f32> = list.iter().filter_map(Value::as_f64).map(|v| v as f32).collect();
        if c.len() >= 3 {
            return Ok(Some(Color::srgba(c[0], c[1], c[2], c.get(3).copied().unwrap_or(1.0))));
        }
    }
    Err(invalid("color must be \"#RRGGBB\" or [r, g, b, a]"))
}

/// `params` accepts [p0, p1, ...] or {"p0": v, "Rim Power": v, ...} (labels from the shader header).
fn params_param(params: &Option<Value>, meta: Option<&library::ShaderMeta>) -> Result<Option<[Option<f32>; 4]>, (i32, String)> {
    let Some(value) = params.as_ref().and_then(|p| p.get("params")) else { return Ok(None) };
    let mut out = [None; 4];
    if let Some(list) = value.as_array() {
        for (i, v) in list.iter().take(4).enumerate() {
            out[i] = v.as_f64().map(|v| v as f32);
        }
        return Ok(Some(out));
    }
    if let Some(map) = value.as_object() {
        for (key, v) in map {
            let lower = key.to_ascii_lowercase();
            let index = match lower.as_str() {
                "p0" | "0" => Some(0),
                "p1" | "1" => Some(1),
                "p2" | "2" => Some(2),
                "p3" | "3" => Some(3),
                _ => meta.and_then(|m| m.params.iter().position(|p| p.label.to_ascii_lowercase() == lower)),
            };
            let index = index.ok_or_else(|| invalid(format!("Unknown parameter '{key}'")))?;
            out[index] = Some(v.as_f64().ok_or_else(|| invalid(format!("Parameter '{key}' must be a number")))? as f32);
        }
        return Ok(Some(out));
    }
    Err(invalid("params must be an array or an object"))
}

fn meta_json(meta: &library::ShaderMeta) -> Value {
    let params: Vec<Value> = (0..4)
        .filter(|i| meta.declared[*i] || !meta.declared.iter().any(|d| *d))
        .map(|i| {
            let p = &meta.params[i];
            json!({ "slot": format!("p{i}"), "label": p.label, "default": p.default, "min": p.min, "max": p.max })
        })
        .collect();
    json!({
        "name": meta.name,
        "description": meta.description,
        "alpha": meta.alpha.label(),
        "color": { "label": meta.color_label, "default": crate::core::reflect_fields::color_hex(meta.color) },
        "params": params,
    })
}

fn entry_json(entry: &library::ShaderEntry) -> Value {
    json!({
        "path": entry.path,
        "template": entry.template,
        "meta": meta_json(&entry.meta),
        "valid": entry.errors.is_empty(),
        "errors": entry.errors.iter().map(|e| e.to_json()).collect::<Vec<_>>(),
    })
}

fn users_of(world: &mut World, path: &str) -> Vec<Value> {
    world
        .query::<(Entity, &ShaderMaterialRef, Option<&Name>)>()
        .iter(world)
        .filter(|(_, r, _)| r.shader == path)
        .map(|(e, _, n)| json!({ "entity": e.to_bits(), "name": n.map(|n| n.as_str().to_string()) }))
        .collect()
}

pub fn dispatch(world: &mut World, method: &str, params: Option<Value>) -> Option<RpcResult> {
    let method = method.strip_prefix("editor/").unwrap_or(method);
    Some(match method {
        "shader_editor" => shader_editor(world, &params),
        "shaders" => shaders(world),
        "shader_get" => shader_get(world, &params),
        "shader_set" => shader_set(world, &params),
        "shader_validate" => shader_validate(&params),
        "shader_create" => shader_create(world, &params),
        "shader_delete" => shader_delete(world, &params),
        "shader_apply" => shader_apply(world, &params),
        "shader_revert" => shader_revert(world, &params),
        "shader_params" => shader_params(world, &params),
        _ => return None,
    })
}

fn shader_editor(world: &mut World, params: &Option<Value>) -> RpcResult {
    let open = params.as_ref().and_then(|p| p.get("open")).and_then(Value::as_bool).unwrap_or(true);
    super::ui::set_open(world, open);
    if let Some(path) = str_param(params, "path") {
        ops::open_doc(world, path).map_err(invalid)?;
    }
    let state = world.resource::<ShaderEditorState>();
    Ok(json!({ "open": state.open, "current": state.current, "status": state.status }))
}

fn shaders(world: &mut World) -> RpcResult {
    ops::rescan(world);
    let library = world.resource::<ShaderLibrary>();
    let user: Vec<Value> = library.entries.iter().filter(|e| !e.template).map(entry_json).collect();
    let templates: Vec<Value> = library.entries.iter().filter(|e| e.template).map(entry_json).collect();
    Ok(json!({
        "shaders": user,
        "templates": templates,
        "folder": library::disk_path(library::SHADER_DIR).to_string_lossy(),
        "contract": "Define `fn shade(in: ShaderInput) -> vec4<f32>`. ShaderInput: frag_coord, world_position, normal, uv, view_dir, time, front_facing. Uniforms: material.color, p0()..p3(). Helpers: lambert, fresnel, hash11, hash21, noise2, fbm2, hue_shift, camera_position, PI, LIGHT_DIR. Header: //! name:, //! alpha: opaque|blend|add, //! color: Label = #RRGGBB, //! p0: Label = default (min..max).",
    }))
}

fn shader_get(world: &mut World, params: &Option<Value>) -> RpcResult {
    let path = library::normalize_path(str_param(params, "path").ok_or_else(|| invalid("Missing 'path'"))?);
    let source = ShaderLibrary::read_source(&path).ok_or_else(|| invalid(format!("Shader not found: {path}")))?;
    let entry = ops::shader_meta(world, &path);
    let used_by = users_of(world, &path);
    let errors: Vec<Value> = world.resource::<ShaderLibrary>().get(&path).map(|e| e.errors.iter().map(|d| d.to_json()).collect()).unwrap_or_default();
    Ok(json!({
        "path": path,
        "source": source,
        "meta": entry.as_ref().map(meta_json),
        "errors": errors,
        "used_by": used_by,
    }))
}

fn shader_set(world: &mut World, params: &Option<Value>) -> RpcResult {
    let path = library::normalize_path(str_param(params, "path").ok_or_else(|| invalid("Missing 'path'"))?);
    let source = str_param(params, "source").ok_or_else(|| invalid("Missing 'source'"))?.to_string();
    let result = ops::save_and_apply(world, &path, &source);
    // Keep an open editor tab in sync with the new file contents.
    let reopen = world.resource::<ShaderEditorState>().docs.contains_key(&path);
    if reopen {
        let errors = result.clone().err().unwrap_or_default();
        world.resource_mut::<ShaderEditorState>().docs.insert(path.clone(), ops::Doc::new(&ShaderLibrary::read_source(&path).unwrap_or_default(), errors));
        world.resource_mut::<ShaderEditorState>().list_dirty = true;
    }
    ops::rescan(world);
    match result {
        Ok(()) => Ok(json!({ "path": path, "saved": true, "applied": true, "errors": [] })),
        Err(errors) => Ok(json!({
            "path": path,
            "saved": true,
            "applied": false,
            "note": "The file was saved but did not validate; objects keep rendering with the last good version.",
            "errors": errors.iter().map(|e| e.to_json()).collect::<Vec<_>>(),
        })),
    }
}

fn shader_validate(params: &Option<Value>) -> RpcResult {
    let source = str_param(params, "source").ok_or_else(|| invalid("Missing 'source'"))?;
    Ok(match library::validate(source) {
        Ok(()) => json!({ "valid": true, "errors": [] }),
        Err(errors) => json!({ "valid": false, "errors": errors.iter().map(|e| e.to_json()).collect::<Vec<_>>() }),
    })
}

fn shader_create(world: &mut World, params: &Option<Value>) -> RpcResult {
    let template = str_param(params, "template").unwrap_or("unlit_color");
    let name = str_param(params, "name");
    let path = ops::create_from_template(world, name, template, false).map_err(invalid)?;
    let meta = ops::shader_meta(world, &path);
    Ok(json!({ "path": path, "template": template, "meta": meta.as_ref().map(meta_json) }))
}

fn shader_delete(world: &mut World, params: &Option<Value>) -> RpcResult {
    let path = library::normalize_path(str_param(params, "path").ok_or_else(|| invalid("Missing 'path'"))?);
    ops::delete_shader(world, &path).map_err(invalid)?;
    Ok(json!({ "deleted": path }))
}

fn shader_apply(world: &mut World, params: &Option<Value>) -> RpcResult {
    let entity = resolve_entity(world, params)?;
    let path = library::normalize_path(str_param(params, "path").or_else(|| str_param(params, "shader")).ok_or_else(|| invalid("Missing 'path'"))?);
    let meta = ops::shader_meta(world, &path).ok_or_else(|| invalid(format!("Shader not found: {path}")))?;
    let color = color_param(params)?;
    let values = params_param(params, Some(&meta))?;
    let applied = ops::apply_to_entity(world, entity, &path, color, values).map_err(invalid)?;
    Ok(json!({ "entity": entity.to_bits(), "material": applied.to_json(), "meta": meta_json(&meta), "undoable": true }))
}

fn shader_revert(world: &mut World, params: &Option<Value>) -> RpcResult {
    let entity = resolve_entity(world, params)?;
    ops::revert_entity(world, entity).map_err(invalid)?;
    Ok(json!({ "entity": entity.to_bits(), "material": "standard" }))
}

fn shader_params(world: &mut World, params: &Option<Value>) -> RpcResult {
    let entity = resolve_entity(world, params)?;
    let current = world.get::<ShaderMaterialRef>(entity).cloned().ok_or_else(|| invalid("Entity does not use a shader material (call editor/shader_apply first)"))?;
    let meta = ops::shader_meta(world, &current.shader);
    let color = color_param(params)?;
    let values = params_param(params, meta.as_ref())?;
    let mut next = current.clone();
    if let Some(color) = color {
        next.color = color;
    }
    if let Some(values) = values {
        for (i, v) in values.iter().enumerate() {
            if let Some(v) = v {
                next.set_param(i, *v);
            }
        }
    }
    set_shader_ref_with_undo(world, entity, Some(next.clone()), "Edit shader parameters").map_err(invalid)?;
    let labels: Vec<Value> = meta
        .as_ref()
        .map(|m| (0..4).map(|i| json!({ "slot": format!("p{i}"), "label": m.params[i].label, "value": next.params()[i] })).collect())
        .unwrap_or_default();
    Ok(json!({ "entity": entity.to_bits(), "material": next.to_json(), "params": labels, "changed": next != current }))
}

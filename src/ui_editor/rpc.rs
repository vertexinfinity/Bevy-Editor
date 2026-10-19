//! Remote (JSON-RPC / MCP) methods of the UI Editor. See the module docs in `mod.rs`.
use bevy::{inspector::InspectorSelection, prelude::*};
use serde_json::{json, Map, Value};

use super::model::{self, UiElementKind};
use super::UiEditorState;

type RpcResult = Result<Value, (i32, String)>;

fn bad(msg: impl Into<String>) -> (i32, String) {
    (-32602, msg.into())
}

fn failed(msg: impl Into<String>) -> (i32, String) {
    (-32010, msg.into())
}

pub fn element_json(world: &World, e: Entity, recursive: bool, props: bool) -> Value {
    let mut obj = Map::new();
    obj.insert("entity".into(), json!(e.to_bits()));
    obj.insert("name".into(), json!(model::display_name(world, e)));
    obj.insert("kind".into(), json!(model::kind_of(world, e).id()));
    obj.insert("parent".into(), json!(model::ui_parent(world, e).map(|p| p.to_bits())));
    if let Some(n) = world.get::<ComputedNode>(e) {
        let s = n.size() * n.inverse_scale_factor;
        obj.insert("computed_size".into(), json!([s.x.round(), s.y.round()]));
    }
    if props {
        obj.insert("props".into(), Value::Object(model::get_props(world, e)));
    }
    let children = model::ui_children(world, e);
    if recursive {
        obj.insert("children".into(), Value::Array(children.iter().map(|c| element_json(world, *c, true, props)).collect()));
    } else {
        obj.insert("children".into(), json!(children.iter().map(|c| c.to_bits()).collect::<Vec<_>>()));
    }
    Value::Object(obj)
}

fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "none".into(),
        other => other.to_string(),
    }
}

/// Target element from `entity` / `name`; falls back to the selected element.
fn target(world: &mut World, p: &Value) -> Result<Entity, (i32, String)> {
    if let Some(e) = model::resolve_element(world, p, "entity", "name").map_err(bad)? {
        if !model::is_game_ui(world, e) {
            return Err(bad("Entity is not an authored game UI element"));
        }
        return Ok(e);
    }
    world
        .resource::<InspectorSelection>()
        .0
        .filter(|e| model::is_game_ui(world, *e))
        .ok_or_else(|| bad("Pass 'entity' or 'name' (or select a UI element)"))
}

/// `parent` may be an entity id, a name, null / "root".
fn parent_param(world: &mut World, p: &Value) -> Result<Option<Option<Entity>>, (i32, String)> {
    match p.get("parent") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) if s.eq_ignore_ascii_case("root") || s.is_empty() => Ok(Some(None)),
        Some(Value::Number(n)) => {
            let bits = n.as_u64().ok_or_else(|| bad("parent must be an entity id"))?;
            let e = Entity::try_from_bits(bits).filter(|e| world.get_entity(*e).is_ok()).ok_or_else(|| bad(format!("No entity with id {bits}")))?;
            Ok(Some(Some(e)))
        }
        Some(Value::String(name)) => {
            let e = model::resolve_element(world, &json!({ "name": name }), "entity", "name").map_err(bad)?;
            Ok(Some(e))
        }
        Some(_) => Err(bad("parent must be an entity id, a name, or null")),
    }
}

fn collect_props(p: &Value, skip: &[&str]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(obj) = p.as_object() {
        for (k, v) in obj {
            if k == "props" {
                if let Some(inner) = v.as_object() {
                    out.extend(inner.iter().map(|(k, v)| (k.clone(), value_text(v))));
                }
            } else if !skip.contains(&k.as_str()) {
                out.push((k.clone(), value_text(v)));
            }
        }
    }
    out
}

pub fn dispatch(world: &mut World, method: &str, params: Option<Value>) -> Option<RpcResult> {
    let method = method.strip_prefix("editor/").unwrap_or(method);
    let p = params.unwrap_or(Value::Null);
    Some(match method {
        "ui_editor" => ui_editor(world, &p),
        "ui_tree" => ui_tree(world, &p),
        "ui_add" => ui_add(world, &p),
        "ui_set" => ui_set(world, &p),
        "ui_get" => target(world, &p).map(|e| element_json(world, e, true, true)),
        "ui_delete" => ui_delete(world, &p),
        "ui_move" => ui_move(world, &p),
        "ui_duplicate" => target(world, &p).and_then(|e| {
            let copy = model::duplicate_element(world, e).map_err(failed)?;
            Ok(element_json(world, copy, true, true))
        }),
        "ui_pointer" => super::pointer::rpc(world, &p),
        "ui_screenshot" => super::pointer::screenshot(world, &p),
        _ => return None,
    })
}

fn ui_editor(world: &mut World, p: &Value) -> RpcResult {
    let mut st = world.resource_mut::<UiEditorState>();
    match p.get("open").and_then(Value::as_bool) {
        Some(open) => st.open = open,
        None if p.get("show_game_ui").is_none() => st.open = !st.open,
        None => {}
    }
    if let Some(show) = p.get("show_game_ui").and_then(Value::as_bool) {
        st.show_game_ui = show;
    }
    Ok(json!({
        "open": st.open,
        "show_game_ui": st.show_game_ui,
        "kinds": UiElementKind::ALL.iter().map(|k| k.id()).collect::<Vec<_>>(),
        "properties": model::PROP_KEYS,
        "anchors": model::ANCHORS,
    }))
}

fn ui_tree(world: &mut World, p: &Value) -> RpcResult {
    let props = p.get("props").and_then(Value::as_bool).unwrap_or(true);
    let roots = model::ui_roots(world);
    let count = model::ui_tree(world).len();
    let selected = world.resource::<InspectorSelection>().0.filter(|e| model::is_game_ui(world, *e)).map(|e| e.to_bits());
    Ok(json!({
        "count": count,
        "selected": selected,
        "roots": roots.iter().map(|r| element_json(world, *r, true, props)).collect::<Vec<_>>(),
    }))
}

fn ui_add(world: &mut World, p: &Value) -> RpcResult {
    let kind_str = p.get("kind").and_then(Value::as_str).ok_or_else(|| bad("Missing 'kind'"))?;
    let kind = UiElementKind::parse(kind_str)
        .ok_or_else(|| bad(format!("Unknown kind '{kind_str}' (use {})", UiElementKind::ALL.iter().map(|k| k.id()).collect::<Vec<_>>().join(", "))))?;
    let parent = match parent_param(world, p)? {
        Some(parent) => parent,
        None => None,
    };
    let text = p.get("text").and_then(Value::as_str).or_else(|| p.get("image").and_then(Value::as_str));
    let name = p.get("name").and_then(Value::as_str);
    let e = model::add_element(world, kind, parent, text, name).map_err(failed)?;
    let props = collect_props(p, &["kind", "parent", "text", "name", "image"]);
    let mut warning = None;
    if !props.is_empty() {
        if let Err(err) = model::apply_props_undoable(world, e, &props) {
            warning = Some(err);
        }
    }
    let mut out = element_json(world, e, true, true);
    if let Some(w) = warning {
        out["warning"] = json!(w);
    }
    Ok(out)
}

fn ui_set(world: &mut World, p: &Value) -> RpcResult {
    let e = target(world, p)?;
    let mut props = collect_props(p, &["entity", "name"]);
    if let Some(n) = p.get("rename").and_then(Value::as_str) {
        props.retain(|(k, _)| k != "rename");
        props.push(("name".into(), n.into()));
    }
    if props.is_empty() {
        return Err(bad("No properties given (see editor/ui_editor for the list)"));
    }
    model::apply_props_undoable(world, e, &props).map_err(bad)?;
    Ok(element_json(world, e, false, true))
}

fn ui_delete(world: &mut World, p: &Value) -> RpcResult {
    let e = target(world, p)?;
    let name = model::display_name(world, e);
    model::delete_element(world, e).map_err(failed)?;
    Ok(json!({ "deleted": e.to_bits(), "name": name }))
}

fn ui_move(world: &mut World, p: &Value) -> RpcResult {
    let e = target(world, p)?;
    if let Some(delta) = p.get("delta").and_then(Value::as_i64) {
        model::shift_element(world, e, delta.clamp(-1, 1) as i32).map_err(failed)?;
        return Ok(element_json(world, e, false, true));
    }
    let index = p.get("index").and_then(Value::as_u64).map(|i| i as usize);
    let parent = match parent_param(world, p)? {
        Some(parent) => parent,
        None => world.get::<ChildOf>(e).map(|c| c.parent()),
    };
    // `index` counts element siblings (widget parts are skipped).
    let raw_index = match (parent, index) {
        (Some(par), Some(i)) => {
            let siblings: Vec<Entity> = model::ui_children(world, par).into_iter().filter(|s| *s != e).collect();
            Some(siblings.get(i).map(|s| model::child_index(world, par, *s)).unwrap_or(usize::MAX))
        }
        _ => None,
    };
    model::move_element(world, e, parent, raw_index).map_err(failed)?;
    Ok(element_json(world, e, false, true))
}

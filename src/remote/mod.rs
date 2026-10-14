//! Remote control and inspection module via Bevy Remote Protocol (BRP) and JSON-RPC.
//!
//! Enables external tools, scripts, and MCP (Model Context Protocol) agents to:
//! - Inspect active scene hierarchy, entities, and components (`editor/status`, `editor/scene_tree`)
//! - Select entities programmatically (`editor/select`)
//! - Spawn 3D primitives or assets (`editor/spawn_primitive`, `editor/spawn_model`)
//! - Trigger Play-In-Editor (PIE) simulation (`editor/play_mode`)
//! - Take camera snapshots or query transforms

use bevy::prelude::*;

pub const DEFAULT_EDITOR_BRP_PORT: u16 = 15703;

pub struct EditorRemotePlugin {
    pub port: u16,
}

impl Default for EditorRemotePlugin {
    fn default() -> Self {
        Self {
            port: DEFAULT_EDITOR_BRP_PORT,
        }
    }
}

#[cfg(not(feature = "remote"))]
impl Plugin for EditorRemotePlugin {
    fn build(&self, _app: &mut App) {
        // No-op when remote feature is disabled to keep Bevylite lightweight
    }
}

#[cfg(feature = "remote")]
impl Plugin for EditorRemotePlugin {
    fn build(&self, app: &mut App) {
        inner::build_remote(self, app);
    }
}

#[cfg(feature = "remote")]
mod inner {
    use super::*;
    use bevy::{
        inspector::InspectorSelection,
        remote::{
            error_codes, http::RemoteHttpPlugin, BrpError, BrpResult, RemotePlugin,
        },
    };
    use serde_json::{json, Value};
    use crate::core::play_mode::{PlayModeAction, PlayState};
    use crate::scene::SceneEntity;

    pub fn build_remote(plugin: &EditorRemotePlugin, app: &mut App) {
        app.add_plugins((
            RemotePlugin::default()
                .with_method_main("editor/status", status_handler)
                .with_method_main("editor/scene_tree", scene_tree_handler)
                .with_method_main("editor/select", select_entity_handler)
                .with_method_main("editor/spawn_primitive", spawn_primitive_handler)
                .with_method_main("editor/play_mode", play_mode_handler),
            RemoteHttpPlugin::default()
                .with_address(std::net::Ipv4Addr::LOCALHOST)
                .with_port(plugin.port),
        ));

        info!("Editor Remote BRP Server listening on http://127.0.0.1:{}", plugin.port);
    }

#[cfg(feature = "remote")]
/// Helper to format invalid param errors
fn invalid_params(message: impl Into<String>) -> BrpError {
    BrpError {
        code: error_codes::INVALID_PARAMS,
        message: message.into(),
        data: None,
    }
}

/// RPC: `editor/status` - reports current editor selection, play mode, and entity count.
fn status_handler(In(_): In<Option<Value>>, world: &mut World) -> BrpResult {
    let play_state = world
        .get_resource::<PlayState>()
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "Unknown".to_string());

    let selection = world
        .get_resource::<InspectorSelection>()
        .and_then(|sel| sel.0)
        .map(|ent| {
            let name = world.get::<Name>(ent).map(|n| n.as_str().to_string());
            json!({
                "entity": ent.to_bits(),
                "name": name,
            })
        });

    let mut scene_query = world.query_filtered::<Entity, With<SceneEntity>>();
    let entity_count = scene_query.iter(world).count();

    Ok(json!({
        "status": "ready",
        "play_state": play_state,
        "selected_entity": selection,
        "scene_entity_count": entity_count,
        "port": DEFAULT_EDITOR_BRP_PORT,
    }))
}

/// RPC: `editor/scene_tree` - lists all authorable scene entities.
fn scene_tree_handler(In(_): In<Option<Value>>, world: &mut World) -> BrpResult {
    let mut scene_query = world.query_filtered::<(Entity, Option<&Name>, Option<&Transform>), With<SceneEntity>>();
    let entities: Vec<Value> = scene_query
        .iter(world)
        .map(|(entity, name, transform)| {
            let name_str = name.map(|n| n.as_str().to_string()).unwrap_or_else(|| "Unnamed".to_string());
            let pos = transform.map(|t| [t.translation.x, t.translation.y, t.translation.z]).unwrap_or([0.0, 0.0, 0.0]);
            json!({
                "entity": entity.to_bits(),
                "name": name_str,
                "position": pos,
            })
        })
        .collect();

    Ok(json!({ "tree": entities }))
}

/// RPC: `editor/select` - selects an entity by ID or name.
fn select_entity_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let params = params.ok_or_else(|| invalid_params("Expected params with 'entity' or 'name'"))?;

    let target_entity = if let Some(bits) = params.get("entity").and_then(Value::as_u64) {
        Entity::try_from_bits(bits)
            .filter(|e| world.get_entity(*e).is_ok())
            .ok_or_else(|| invalid_params(format!("No entity exists with ID {bits}")))?
    } else if let Some(name_str) = params.get("name").and_then(Value::as_str) {
        let mut named_query = world.query::<(Entity, &Name)>();
        named_query
            .iter(world)
            .find(|(_, name)| name.as_str() == name_str)
            .map(|(e, _)| e)
            .ok_or_else(|| invalid_params(format!("No entity found with name '{name_str}'")))?
    } else {
        return Err(invalid_params("Must provide either 'entity' (u64) or 'name' (string)"));
    };

    if let Some(mut selection) = world.get_resource_mut::<InspectorSelection>() {
        selection.0 = Some(target_entity);
    }

    Ok(json!({
        "selected": target_entity.to_bits(),
    }))
}

/// RPC: `editor/spawn_primitive` - spawns a Cube, Sphere, or Point Light.
fn spawn_primitive_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let kind = params
        .as_ref()
        .and_then(|p| p.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("cube")
        .to_lowercase();

    let name = params
        .as_ref()
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("Spawned {}", kind));

    let entity = match kind.as_str() {
        "cube" => {
            let mesh = world.resource_mut::<Assets<Mesh>>().add(Cuboid::new(1.0, 1.0, 1.0));
            let material = world.resource_mut::<Assets<StandardMaterial>>().add(Color::srgb(0.2, 0.7, 0.9));
            world.spawn((
                SceneEntity,
                Name::new(name),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(0.0, 0.5, 0.0),
            )).id()
        }
        "sphere" => {
            let mesh = world.resource_mut::<Assets<Mesh>>().add(Sphere::new(0.5));
            let material = world.resource_mut::<Assets<StandardMaterial>>().add(Color::srgb(0.9, 0.3, 0.2));
            world.spawn((
                SceneEntity,
                Name::new(name),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(0.0, 0.5, 0.0),
            )).id()
        }
        "light" => {
            world.spawn((
                SceneEntity,
                Name::new(name),
                PointLight {
                    intensity: 100_000.0,
                    range: 20.0,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_xyz(0.0, 4.0, 0.0),
            )).id()
        }
        _ => return Err(invalid_params(format!("Unknown primitive kind '{kind}'. Valid kinds: 'cube', 'sphere', 'light'"))),
    };

    // Auto-select the newly spawned entity
    if let Some(mut selection) = world.get_resource_mut::<InspectorSelection>() {
        selection.0 = Some(entity);
    }

    Ok(json!({
        "spawned": entity.to_bits(),
        "kind": kind,
    }))
}

/// RPC: `editor/play_mode` - controls simulation state ("play", "pause", "stop").
fn play_mode_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let action = params
        .as_ref()
        .and_then(|p| p.get("action"))
        .and_then(Value::as_str)
        .unwrap_or("play")
        .to_lowercase();

    let play_action = match action.as_str() {
        "play" => PlayModeAction::Play,
        "pause" => PlayModeAction::Pause,
        "stop" => PlayModeAction::Stop,
        _ => return Err(invalid_params(format!("Invalid action '{action}'. Use 'play', 'pause', or 'stop'"))),
    };

    world.write_message(play_action);

    Ok(json!({
        "status": "ok",
        "action": action,
    }))
}
}

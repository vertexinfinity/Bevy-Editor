//! Remote control, MCP JSON-RPC Server, and Interactive Console for Bevylite.
//!
//! Always-on, zero-overhead HTTP JSON-RPC 2.0 server (`http://127.0.0.1:15703`)
//! and `stdin` command console enabling AI agents (MCP), test scripts, and developers to:
//! - Inspect active scene hierarchy, entities, and bake progress (`editor/status`, `editor/scene_tree`)
//! - Select entities programmatically (`editor/select`)
//! - Spawn 3D primitives and lights (`editor/spawn_primitive`)
//! - Mutate transforms and PBR materials (`editor/set_transform`, `editor/set_material`)
//! - Create, clear, save, and load scenes (`editor/new_scene`, `editor/clear_scene`, `editor/save_scene`, `editor/load_scene`)
//! - Trigger GPU RTX / CPU lightmap baking (`editor/bake`)
//! - Capture viewport/window screenshots to PNG (`editor/screenshot`)
//! - Control Play-In-Editor simulation (`editor/play_mode`) and exit (`editor/exit`)

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;

use bevy::app::AppExit;
use bevy::ecs::system::RunSystemOnce;
use bevy::inspector::InspectorSelection;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevylite_lightmapper::{
    ActiveLightmap, BakeLightmapRequest, LightmapBackend, LightmapBakeProgress, LightmapConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod features;

use crate::core::play_mode::{PlayModeAction, PlayState};
use crate::scene::{primitives, save_load, CurrentScene, EditorInternal, SceneEntity};

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

struct RemoteRpcCall {
    method: String,
    params: Option<Value>,
    reply_tx: Sender<Result<Value, (i32, String)>>,
}

#[derive(Resource)]
struct RemoteRpcChannel {
    rx: Mutex<Receiver<RemoteRpcCall>>,
}

/// Tag component storing primitive metadata so procedural meshes roundtrip cleanly during save/load.
#[derive(Component, Clone, Debug, Serialize, Deserialize, Reflect)]
#[reflect(Component)]
pub struct PrimitiveMeta {
    pub kind: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SavedSceneManifest {
    #[serde(default)]
    pub entities: Vec<SavedEntityEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SavedEntityEntry {
    pub name: String,
    pub kind: String,
    pub translation: [f32; 3],
    pub rotation_xyzw: [f32; 4],
    pub scale: [f32; 3],
    pub base_color_rgba: Option<[f32; 4]>,
    pub metallic: Option<f32>,
    pub roughness: Option<f32>,
    pub emissive_rgb: Option<[f32; 3]>,
    pub unlit: Option<bool>,
    pub light_intensity: Option<f32>,
    pub light_range: Option<f32>,
    pub light_color_rgb: Option<[f32; 3]>,
    pub shadows_enabled: Option<bool>,
}

impl Plugin for EditorRemotePlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = mpsc::channel::<RemoteRpcCall>();
        let port = self.port;

        // 1. Spawn lightweight HTTP JSON-RPC 2.0 listener on 127.0.0.1:port
        let http_tx = tx.clone();
        std::thread::Builder::new()
            .name("bevylite-rpc-http".to_string())
            .spawn(move || {
                let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
                match TcpListener::bind(addr) {
                    Ok(listener) => {
                        info!(
                            "🌐 Editor Remote JSON-RPC Server listening on http://127.0.0.1:{} (and stdin console)",
                            port
                        );
                        for stream in listener.incoming().flatten() {
                            let req_tx = http_tx.clone();
                            std::thread::spawn(move || {
                                handle_http_connection(stream, req_tx);
                            });
                        }
                    }
                    Err(err) => {
                        warn!(
                            "Could not bind Editor Remote HTTP server on 127.0.0.1:{}: {}",
                            port, err
                        );
                    }
                }
            })
            .ok();

        // 2. Spawn interactive stdin console listener
        let stdin_tx = tx;
        std::thread::Builder::new()
            .name("bevylite-console-stdin".to_string())
            .spawn(move || {
                let stdin = std::io::stdin();
                let reader = BufReader::new(stdin.lock());
                for line in reader.lines().map_while(Result::ok) {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if let Some((method, params)) = parse_console_command(trimmed) {
                        let (reply_tx, reply_rx) = mpsc::channel();
                        if stdin_tx
                            .send(RemoteRpcCall {
                                method,
                                params,
                                reply_tx,
                            })
                            .is_ok()
                        {
                            if let Ok(res) = reply_rx.recv() {
                                match res {
                                    Ok(val) => println!(
                                        "[Console OK] {}",
                                        serde_json::to_string_pretty(&val).unwrap_or_default()
                                    ),
                                    Err((_, msg)) => eprintln!("[Console Error] {}", msg),
                                }
                            }
                        }
                    }
                }
            })
            .ok();

        app.register_type::<PrimitiveMeta>()
            .insert_resource(RemoteRpcChannel { rx: Mutex::new(rx) })
            .add_systems(Update, process_remote_rpc_requests);
    }
}

fn handle_http_connection(mut stream: TcpStream, tx: Sender<RemoteRpcCall>) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        return;
    }

    if request_line.starts_with("OPTIONS") {
        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let mut content_length = 0usize;
    loop {
        let mut header_line = String::new();
        if reader.read_line(&mut header_line).is_err() {
            return;
        }
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 && reader.read_exact(&mut body).is_err() {
        return;
    }

    let body_json: Value = if body.is_empty() {
        json!({ "method": "editor/status", "id": 1 })
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => {
                write_json_response(
                    &mut stream,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": format!("Parse error: {e}") }
                    }),
                );
                return;
            }
        }
    };

    let id = body_json.get("id").cloned().unwrap_or(json!(1));
    let method = body_json
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("editor/status")
        .to_string();
    let params = body_json.get("params").cloned();

    let (reply_tx, reply_rx) = mpsc::channel();
    if tx
        .send(RemoteRpcCall {
            method,
            params,
            reply_tx,
        })
        .is_err()
    {
        return;
    }

    match reply_rx.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(Ok(result)) => {
            write_json_response(
                &mut stream,
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": result
                }),
            );
        }
        Ok(Err((code, message))) => {
            write_json_response(
                &mut stream,
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": code, "message": message }
                }),
            );
        }
        Err(_) => {
            write_json_response(
                &mut stream,
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32000, "message": "Request timed out waiting for main world" }
                }),
            );
        }
    }
}

fn write_json_response(stream: &mut TcpStream, payload: &Value) {
    let body = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// Parses human-friendly console commands from `stdin` into `(method, params)`.
fn parse_console_command(input: &str) -> Option<(String, Option<Value>)> {
    if input.starts_with('{') {
        if let Ok(val) = serde_json::from_str::<Value>(input) {
            let method = val.get("method")?.as_str()?.to_string();
            let params = val.get("params").cloned();
            return Some((method, params));
        }
    }

    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts.first()?.to_lowercase();
    match cmd.as_str() {
        "help" | "?" => {
            println!(
                "Bevylite Console Commands:\n\
                 - status                              : Show editor & bake status\n\
                 - tree                                : List all scene entities\n\
                 - select <name|id>                    : Select an entity\n\
                 - spawn <cube|sphere|plane|light> [name] [x y z] : Spawn object\n\
                 - move <x> <y> <z>                    : Move selected entity\n\
                 - scale <sx> <sy> <sz>                : Scale selected entity\n\
                 - rotate <rx> <ry> <rz>               : Rotate selected entity (degrees)\n\
                 - color <r> <g> <b> [metallic] [rough]: Set material on selected entity\n\
                 - delete [name|id]                    : Delete entity\n\
                 - bake [gpu|cpu] [resolution]         : Bake lightmaps\n\
                 - clear_lightmaps                     : Remove baked lightmaps from all meshes\n\
                 - screenshot [path]                   : Save window screenshot PNG\n\
                 - save [path]                         : Save scene to file\n\
                 - load <path>                         : Load scene from file\n\
                 - new                                 : Reset to starter scene\n\
                 - clear                               : Clear all scene entities\n\
                 - play | pause | stop                 : Control Play-In-Editor\n\
                 - exit | quit                         : Exit editor"
            );
            None
        }
        "status" => Some(("editor/status".to_string(), None)),
        "tree" | "ls" => Some(("editor/scene_tree".to_string(), None)),
        "select" => {
            let target = parts.get(1)?;
            if let Ok(id) = target.parse::<u64>() {
                Some(("editor/select".to_string(), Some(json!({ "entity": id }))))
            } else {
                let name = parts[1..].join(" ");
                Some(("editor/select".to_string(), Some(json!({ "name": name }))))
            }
        }
        "spawn" => {
            let kind = parts.get(1).copied().unwrap_or("cube");
            let name = parts.get(2).copied();
            let pos = if parts.len() >= 6 {
                Some([
                    parts[3].parse::<f32>().unwrap_or(0.0),
                    parts[4].parse::<f32>().unwrap_or(0.5),
                    parts[5].parse::<f32>().unwrap_or(0.0),
                ])
            } else {
                None
            };
            let mut p = json!({ "kind": kind });
            if let Some(n) = name {
                p["name"] = json!(n);
            }
            if let Some(xyz) = pos {
                p["position"] = json!(xyz);
            }
            Some(("editor/spawn_primitive".to_string(), Some(p)))
        }
        "move" | "pos" => {
            let x = parts.get(1)?.parse::<f32>().ok()?;
            let y = parts.get(2)?.parse::<f32>().ok()?;
            let z = parts.get(3)?.parse::<f32>().ok()?;
            Some((
                "editor/set_transform".to_string(),
                Some(json!({ "position": [x, y, z] })),
            ))
        }
        "scale" => {
            let x = parts.get(1)?.parse::<f32>().ok()?;
            let y = parts.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(x);
            let z = parts.get(3).and_then(|s| s.parse::<f32>().ok()).unwrap_or(x);
            Some((
                "editor/set_transform".to_string(),
                Some(json!({ "scale": [x, y, z] })),
            ))
        }
        "rotate" | "rot" => {
            let rx = parts.get(1)?.parse::<f32>().ok()?;
            let ry = parts.get(2)?.parse::<f32>().ok()?;
            let rz = parts.get(3)?.parse::<f32>().ok()?;
            Some((
                "editor/set_transform".to_string(),
                Some(json!({ "rotation_euler": [rx, ry, rz] })),
            ))
        }
        "color" | "material" => {
            let r = parts.get(1)?.parse::<f32>().ok()?;
            let g = parts.get(2)?.parse::<f32>().ok()?;
            let b = parts.get(3)?.parse::<f32>().ok()?;
            let mut p = json!({ "color": [r, g, b, 1.0] });
            if let Some(m) = parts.get(4).and_then(|s| s.parse::<f32>().ok()) {
                p["metallic"] = json!(m);
            }
            if let Some(rough) = parts.get(5).and_then(|s| s.parse::<f32>().ok()) {
                p["roughness"] = json!(rough);
            }
            Some(("editor/set_material".to_string(), Some(p)))
        }
        "delete" | "rm" => {
            if let Some(target) = parts.get(1) {
                if let Ok(id) = target.parse::<u64>() {
                    Some(("editor/delete_entity".to_string(), Some(json!({ "entity": id }))))
                } else {
                    Some((
                        "editor/delete_entity".to_string(),
                        Some(json!({ "name": parts[1..].join(" ") })),
                    ))
                }
            } else {
                Some(("editor/delete_entity".to_string(), Some(json!({}))))
            }
        }
        "bake" => {
            let backend = parts.get(1).copied().unwrap_or("gpu");
            let resolution = parts
                .get(2)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(512);
            Some((
                "editor/bake".to_string(),
                Some(json!({ "backend": backend, "resolution": resolution })),
            ))
        }
        "clear_lightmaps" | "clear_bake" => {
            Some(("editor/clear_lightmaps".to_string(), Some(json!({}))))
        }
        "screenshot" | "snap" => {
            let path = parts
                .get(1)
                .copied()
                .unwrap_or("screenshots/editor_capture.png");
            Some(("editor/screenshot".to_string(), Some(json!({ "path": path }))))
        }
        "save" => {
            let path = parts
                .get(1)
                .copied()
                .unwrap_or("assets/scenes/saved_scene.scn.ron");
            Some(("editor/save_scene".to_string(), Some(json!({ "path": path }))))
        }
        "load" => {
            let path = parts
                .get(1)
                .copied()
                .unwrap_or("assets/scenes/saved_scene.scn.ron");
            Some(("editor/load_scene".to_string(), Some(json!({ "path": path }))))
        }
        "new" => Some(("editor/new_scene".to_string(), Some(json!({ "starter": true })))),
        "clear" => Some(("editor/clear_scene".to_string(), Some(json!({})))),
        "play" | "pause" | "stop" => {
            Some(("editor/play_mode".to_string(), Some(json!({ "action": cmd }))))
        }
        "exit" | "quit" => Some(("editor/exit".to_string(), None)),
        _ => {
            eprintln!("Unknown command '{}'. Type 'help' for available commands.", cmd);
            None
        }
    }
}

fn process_remote_rpc_requests(world: &mut World) {
    let calls: Vec<RemoteRpcCall> = {
        let Some(channel) = world.get_resource::<RemoteRpcChannel>() else {
            return;
        };
        let Ok(rx) = channel.rx.lock() else {
            return;
        };
        rx.try_iter().collect()
    };

    for call in calls {
        let response = dispatch_rpc_method(world, &call.method, call.params);
        let _ = call.reply_tx.send(response);
    }
}

fn invalid_params(msg: impl Into<String>) -> (i32, String) {
    (-32602, msg.into())
}

fn ensure_in_editor(world: &mut World) {
    let is_in_hub = world
        .get_resource::<State<crate::ui::EditorAppState>>()
        .is_some_and(|s| *s.get() == crate::ui::EditorAppState::ProjectHub);
    if is_in_hub {
        if let Some(mut next_state) =
            world.get_resource_mut::<NextState<crate::ui::EditorAppState>>()
        {
            next_state.set(crate::ui::EditorAppState::InEditor);
        }
    }
}

fn dispatch_rpc_method(
    world: &mut World,
    method: &str,
    params: Option<Value>,
) -> Result<Value, (i32, String)> {
    match method {
        "editor/status" | "status" => rpc_status(world),
        "editor/enter_editor" | "enter_editor" => {
            ensure_in_editor(world);
            Ok(json!({ "status": "ok", "app_state": "InEditor" }))
        }
        "editor/scene_tree" | "scene_tree" => {
            ensure_in_editor(world);
            rpc_scene_tree(world)
        }
        "editor/select" | "select" => {
            ensure_in_editor(world);
            rpc_select(world, params)
        }
        "editor/spawn_primitive" | "spawn_primitive" => {
            ensure_in_editor(world);
            rpc_spawn_primitive(world, params)
        }
        "editor/set_transform" | "set_transform" => {
            ensure_in_editor(world);
            rpc_set_transform(world, params)
        }
        "editor/set_material" | "set_material" => {
            ensure_in_editor(world);
            rpc_set_material(world, params)
        }
        "editor/delete_entity" | "delete_entity" => {
            ensure_in_editor(world);
            rpc_delete_entity(world, params)
        }
        "editor/new_scene" | "new_scene" => {
            ensure_in_editor(world);
            rpc_new_scene(world, params, true)
        }
        "editor/clear_scene" | "clear_scene" => {
            ensure_in_editor(world);
            rpc_new_scene(world, params, false)
        }
        "editor/save_scene" | "save_scene" => {
            ensure_in_editor(world);
            rpc_save_scene(world, params)
        }
        "editor/load_scene" | "load_scene" => {
            ensure_in_editor(world);
            rpc_load_scene(world, params)
        }
        "editor/bake" | "bake" => {
            ensure_in_editor(world);
            rpc_bake(world, params)
        }
        "editor/clear_lightmaps" | "clear_lightmaps" => {
            ensure_in_editor(world);
            rpc_clear_lightmaps(world)
        }
        "editor/screenshot" | "screenshot" => {
            ensure_in_editor(world);
            rpc_screenshot(world, params)
        }
        "editor/play_mode" | "play_mode" => {
            ensure_in_editor(world);
            rpc_play_mode(world, params)
        }
        "editor/exit" | "exit" => {
            world.write_message(AppExit::Success);
            Ok(json!({ "status": "exiting" }))
        }
        _ => {
            ensure_in_editor(world);
            features::dispatch(world, method, params)
                .unwrap_or_else(|| Err((-32601, format!("Unknown RPC method '{method}'"))))
        }
    }
}

/// Resolves an entity from `params["entity"]` (u64), `params["name"]` (str), or current selection.
fn resolve_target_entity(world: &mut World, params: Option<&Value>) -> Result<Entity, (i32, String)> {
    if let Some(p) = params {
        if let Some(bits) = p.get("entity").and_then(Value::as_u64) {
            return Entity::try_from_bits(bits)
                .filter(|e| world.get_entity(*e).is_ok())
                .ok_or_else(|| invalid_params(format!("No entity exists with ID {bits}")));
        }
        if let Some(name_str) = p.get("name").and_then(Value::as_str) {
            let mut q = world.query_filtered::<(Entity, &Name), With<SceneEntity>>();
            if let Some((e, _)) = q.iter(world).find(|(_, n)| n.as_str() == name_str) {
                return Ok(e);
            }
            let mut q_all = world.query::<(Entity, &Name)>();
            if let Some((e, _)) = q_all.iter(world).find(|(_, n)| n.as_str() == name_str) {
                return Ok(e);
            }
            return Err(invalid_params(format!(
                "No entity found with name '{name_str}'"
            )));
        }
    }

    world
        .get_resource::<InspectorSelection>()
        .and_then(|s| s.0)
        .filter(|e| world.get_entity(*e).is_ok())
        .ok_or_else(|| {
            invalid_params("No target entity specified and no entity is currently selected")
        })
}

fn rpc_status(world: &mut World) -> Result<Value, (i32, String)> {
    let app_state = world
        .get_resource::<State<crate::ui::EditorAppState>>()
        .map(|s| format!("{:?}", s.get()))
        .unwrap_or_else(|| "Unknown".to_string());

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

    let bake_info = {
        let prog = world.get_resource::<LightmapBakeProgress>();
        let active = world.get_resource::<ActiveLightmap>();
        json!({
            "is_baking": prog.map(|p| p.is_baking).unwrap_or(false),
            "progress": prog.map(|p| p.progress).unwrap_or(0.0),
            "status": prog.map(|p| p.status.clone()).unwrap_or_else(|| "Idle".to_string()),
            "last_baked_time_ms": prog.map(|p| p.last_baked_time_ms).unwrap_or(0),
            "has_active_lightmap": active.and_then(|a| a.handle.as_ref()).is_some(),
            "lightmap_resolution": active.map(|a| a.resolution).unwrap_or(0),
            "backend": active.and_then(|a| a.backend_used).map(|b| format!("{:?}", b)),
        })
    };

    let current_scene = world
        .get_resource::<CurrentScene>()
        .and_then(|c| c.file_path.as_ref())
        .map(|p| p.to_string_lossy().to_string());

    Ok(json!({
        "status": "ready",
        "app_state": app_state,
        "play_state": play_state,
        "selected_entity": selection,
        "scene_entity_count": entity_count,
        "bake": bake_info,
        "current_scene": current_scene,
        "port": DEFAULT_EDITOR_BRP_PORT,
    }))
}

fn rpc_scene_tree(world: &mut World) -> Result<Value, (i32, String)> {
    let mut entities_data = Vec::new();
    let mut scene_query = world.query_filtered::<(
        Entity,
        Option<&Name>,
        Option<&Transform>,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&bevy::pbr::Lightmap>,
        Option<&PointLight>,
        Option<&DirectionalLight>,
    ), With<SceneEntity>>();

    let raw_list: Vec<_> = scene_query
        .iter(world)
        .map(|(e, name, tf, mesh, mat, lm, pl, dl)| {
            (
                e,
                name.map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| "Unnamed".to_string()),
                tf.copied(),
                mesh.is_some(),
                mat.map(|m| m.0.id()),
                lm.is_some(),
                if dl.is_some() {
                    Some("Directional")
                } else if pl.is_some() {
                    Some("Point")
                } else {
                    None
                },
            )
        })
        .collect();

    let materials = world.get_resource::<Assets<StandardMaterial>>();

    for (entity, name_str, tf, has_mesh, mat_id, has_lightmap, light_type) in raw_list {
        let (pos, rot_deg, scale) = if let Some(t) = tf {
            let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);
            (
                [t.translation.x, t.translation.y, t.translation.z],
                [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()],
                [t.scale.x, t.scale.y, t.scale.z],
            )
        } else {
            ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
        };

        let mat_color = mat_id
            .and_then(|id| materials.and_then(|m| m.get(id)))
            .map(|mat| mat.base_color.to_srgba().to_f32_array());

        entities_data.push(json!({
            "entity": entity.to_bits(),
            "name": name_str,
            "position": pos,
            "rotation_euler": rot_deg,
            "scale": scale,
            "has_mesh": has_mesh,
            "has_material": mat_id.is_some(),
            "material_color": mat_color,
            "has_lightmap": has_lightmap,
            "light_type": light_type,
        }));
    }

    Ok(json!({ "tree": entities_data, "count": entities_data.len() }))
}

fn rpc_select(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
    let target_entity = resolve_target_entity(world, params.as_ref())?;
    let name = world
        .get::<Name>(target_entity)
        .map(|n| n.as_str().to_string());

    if let Some(mut selection) = world.get_resource_mut::<InspectorSelection>() {
        selection.0 = Some(target_entity);
    }

    Ok(json!({
        "selected": target_entity.to_bits(),
        "name": name,
    }))
}

fn ensure_mesh_uv1(mesh: &mut Mesh) {
    if !mesh.contains_attribute(Mesh::ATTRIBUTE_UV_1) {
        let uv1 = if let Some(bevy::render::mesh::VertexAttributeValues::Float32x2(uv0)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        {
            Some(uv0.clone())
        } else if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            Some(vec![[0.0, 0.0]; pos.len()])
        } else {
            None
        };
        if let Some(uv_data) = uv1 {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv_data);
        }
    }
}

fn rpc_spawn_primitive(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
    let kind = params
        .as_ref()
        .and_then(|p| p.get("kind").or_else(|| p.get("shape")))
        .and_then(Value::as_str)
        .unwrap_or("cube")
        .to_lowercase();

    let name = params
        .as_ref()
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("Spawned {}", kind));

    let pos = params
        .as_ref()
        .and_then(|p| p.get("position"))
        .and_then(Value::as_array)
        .and_then(|arr| {
            Some(Vec3::new(
                arr.first()?.as_f64()? as f32,
                arr.get(1)?.as_f64()? as f32,
                arr.get(2)?.as_f64()? as f32,
            ))
        });

    let scale = params
        .as_ref()
        .and_then(|p| p.get("scale"))
        .and_then(Value::as_array)
        .and_then(|arr| {
            Some(Vec3::new(
                arr.first()?.as_f64()? as f32,
                arr.get(1)?.as_f64()? as f32,
                arr.get(2)?.as_f64()? as f32,
            ))
        })
        .unwrap_or(Vec3::ONE);

    let color = params
        .as_ref()
        .and_then(|p| p.get("color"))
        .and_then(Value::as_array)
        .and_then(|arr| {
            Some(Color::srgb(
                arr.first()?.as_f64()? as f32,
                arr.get(1)?.as_f64()? as f32,
                arr.get(2)?.as_f64()? as f32,
            ))
        });

    let entity = match kind.as_str() {
        "cube" | "box" => {
            let mut m = Cuboid::new(1.0, 1.0, 1.0).mesh().build();
            ensure_mesh_uv1(&mut m);
            let mesh = world.resource_mut::<Assets<Mesh>>().add(m);
            let mat = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: color.unwrap_or(Color::srgb(0.2, 0.7, 0.9)),
                    perceptual_roughness: 0.35,
                    ..default()
                });
            let default_pos = pos.unwrap_or(Vec3::new(0.0, 0.5, 0.0));
            world
                .spawn((
                    SceneEntity,
                    PrimitiveMeta {
                        kind: "cube".to_string(),
                    },
                    Name::new(name.clone()),
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(default_pos).with_scale(scale),
                ))
                .id()
        }
        "sphere" => {
            let mut m = Sphere::new(0.5).mesh().ico(5).unwrap();
            ensure_mesh_uv1(&mut m);
            let mesh = world.resource_mut::<Assets<Mesh>>().add(m);
            let mat = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: color.unwrap_or(Color::srgb(0.9, 0.3, 0.2)),
                    perceptual_roughness: 0.25,
                    ..default()
                });
            let default_pos = pos.unwrap_or(Vec3::new(0.0, 0.5, 0.0));
            world
                .spawn((
                    SceneEntity,
                    PrimitiveMeta {
                        kind: "sphere".to_string(),
                    },
                    Name::new(name.clone()),
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(default_pos).with_scale(scale),
                ))
                .id()
        }
        "plane" | "ground" => {
            let mut m = Plane3d::default().mesh().size(10.0, 10.0).build();
            ensure_mesh_uv1(&mut m);
            let mesh = world.resource_mut::<Assets<Mesh>>().add(m);
            let mat = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: color.unwrap_or(Color::srgb(0.22, 0.24, 0.28)),
                    perceptual_roughness: 0.8,
                    ..default()
                });
            let default_pos = pos.unwrap_or(Vec3::ZERO);
            world
                .spawn((
                    SceneEntity,
                    PrimitiveMeta {
                        kind: "plane".to_string(),
                    },
                    Name::new(name.clone()),
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(default_pos).with_scale(scale),
                ))
                .id()
        }
        "cylinder" => {
            let mut m = Cylinder::new(0.5, 1.0).mesh().build();
            ensure_mesh_uv1(&mut m);
            let mesh = world.resource_mut::<Assets<Mesh>>().add(m);
            let mat = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: color.unwrap_or(Color::srgb(0.3, 0.85, 0.4)),
                    perceptual_roughness: 0.4,
                    ..default()
                });
            let default_pos = pos.unwrap_or(Vec3::new(0.0, 0.5, 0.0));
            world
                .spawn((
                    SceneEntity,
                    PrimitiveMeta {
                        kind: "cylinder".to_string(),
                    },
                    Name::new(name.clone()),
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(default_pos).with_scale(scale),
                ))
                .id()
        }
        "light" | "point_light" => {
            let default_pos = pos.unwrap_or(Vec3::new(0.0, 4.0, 0.0));
            world
                .spawn((
                    SceneEntity,
                    PrimitiveMeta {
                        kind: "point_light".to_string(),
                    },
                    Name::new(name.clone()),
                    PointLight {
                        color: color.unwrap_or(Color::WHITE),
                        intensity: 100_000.0,
                        range: 25.0,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    crate::scene::components::LightBakeMode::Mixed,
                    Transform::from_translation(default_pos),
                ))
                .id()
        }
        "sun" | "directional_light" => {
            let default_pos = pos.unwrap_or(Vec3::new(4.0, 10.0, 4.0));
            world
                .spawn((
                    SceneEntity,
                    PrimitiveMeta {
                        kind: "directional_light".to_string(),
                    },
                    Name::new(name.clone()),
                    DirectionalLight {
                        color: color.unwrap_or(Color::WHITE),
                        illuminance: 12_000.0,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    crate::scene::components::LightBakeMode::Mixed,
                    Transform::from_translation(default_pos).looking_at(Vec3::ZERO, Vec3::Y),
                ))
                .id()
        }
        "spot_light" | "spot" => {
            let default_pos = pos.unwrap_or(Vec3::new(0.0, 5.0, 3.0));
            world
                .spawn((
                    SceneEntity,
                    PrimitiveMeta { kind: "spot_light".to_string() },
                    Name::new(name.clone()),
                    SpotLight {
                        color: color.unwrap_or(Color::WHITE),
                        intensity: 400_000.0,
                        range: 30.0,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    crate::scene::components::LightBakeMode::Mixed,
                    Transform::from_translation(default_pos).looking_at(Vec3::ZERO, Vec3::Y),
                ))
                .id()
        }
        "camera" | "camera3d" => {
            let default_pos = pos.unwrap_or(Vec3::new(0.0, 3.0, 8.0));
            world
                .spawn((
                    SceneEntity,
                    PrimitiveMeta { kind: "camera".to_string() },
                    Name::new(name.clone()),
                    Camera3d::default(),
                    Camera { is_active: false, ..default() },
                    Transform::from_translation(default_pos).looking_at(Vec3::ZERO, Vec3::Y),
                ))
                .id()
        }
        _ => {
            return Err(invalid_params(format!(
                "Unknown primitive kind '{kind}'. Valid kinds: 'cube', 'sphere', 'plane', 'cylinder', 'point_light', 'directional_light', 'spot_light', 'camera'"
            )))
        }
    };

    if let Some(mut selection) = world.get_resource_mut::<InspectorSelection>() {
        selection.0 = Some(entity);
    }

    Ok(json!({
        "spawned": entity.to_bits(),
        "entity_bits": entity.to_bits(),
        "name": name,
        "kind": kind,
    }))
}

fn rpc_set_transform(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
    let p = params.ok_or_else(|| invalid_params("Expected params object"))?;
    let entity = resolve_target_entity(world, Some(&p))?;

    let mut tf = world
        .get_mut::<Transform>(entity)
        .ok_or_else(|| invalid_params("Target entity does not have a Transform component"))?;

    if let Some(pos_arr) = p
        .get("position")
        .or_else(|| p.get("translation"))
        .and_then(Value::as_array)
    {
        if pos_arr.len() >= 3 {
            tf.translation = Vec3::new(
                pos_arr[0].as_f64().unwrap_or(0.0) as f32,
                pos_arr[1].as_f64().unwrap_or(0.0) as f32,
                pos_arr[2].as_f64().unwrap_or(0.0) as f32,
            );
        }
    }

    if let Some(rot_arr) = p.get("rotation_euler").and_then(Value::as_array) {
        if rot_arr.len() >= 3 {
            let rx = (rot_arr[0].as_f64().unwrap_or(0.0) as f32).to_radians();
            let ry = (rot_arr[1].as_f64().unwrap_or(0.0) as f32).to_radians();
            let rz = (rot_arr[2].as_f64().unwrap_or(0.0) as f32).to_radians();
            tf.rotation = Quat::from_euler(EulerRot::XYZ, rx, ry, rz);
        }
    }

    if let Some(scale_arr) = p.get("scale").and_then(Value::as_array) {
        if scale_arr.len() >= 3 {
            tf.scale = Vec3::new(
                scale_arr[0].as_f64().unwrap_or(1.0) as f32,
                scale_arr[1].as_f64().unwrap_or(1.0) as f32,
                scale_arr[2].as_f64().unwrap_or(1.0) as f32,
            );
        }
    }

    let (rx, ry, rz) = tf.rotation.to_euler(EulerRot::XYZ);
    Ok(json!({
        "entity": entity.to_bits(),
        "position": [tf.translation.x, tf.translation.y, tf.translation.z],
        "rotation_euler": [rx.to_degrees(), ry.to_degrees(), rz.to_degrees()],
        "scale": [tf.scale.x, tf.scale.y, tf.scale.z],
    }))
}

fn rpc_set_material(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
    let p = params.ok_or_else(|| invalid_params("Expected params object"))?;
    let entity = resolve_target_entity(world, Some(&p))?;

    let mat_handle = world
        .get::<MeshMaterial3d<StandardMaterial>>(entity)
        .map(|m| m.0.clone())
        .ok_or_else(|| {
            invalid_params("Target entity does not have a MeshMaterial3d<StandardMaterial>")
        })?;

    let (updated_rgba, updated_metallic, updated_roughness) = {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let mut mat = materials
            .get_mut(&mat_handle)
            .ok_or_else(|| invalid_params("Material asset not found in Assets<StandardMaterial>"))?;

        if let Some(col_arr) = p
            .get("color")
            .or_else(|| p.get("base_color"))
            .and_then(Value::as_array)
        {
            if col_arr.len() >= 3 {
                let r = col_arr[0].as_f64().unwrap_or(1.0) as f32;
                let g = col_arr[1].as_f64().unwrap_or(1.0) as f32;
                let b = col_arr[2].as_f64().unwrap_or(1.0) as f32;
                let a = col_arr.get(3).and_then(Value::as_f64).unwrap_or(1.0) as f32;
                mat.base_color = Color::srgba(r, g, b, a);
            }
        }

        if let Some(em_arr) = p.get("emissive").and_then(Value::as_array) {
            if em_arr.len() >= 3 {
                let r = em_arr[0].as_f64().unwrap_or(0.0) as f32;
                let g = em_arr[1].as_f64().unwrap_or(0.0) as f32;
                let b = em_arr[2].as_f64().unwrap_or(0.0) as f32;
                mat.emissive = LinearRgba::rgb(r, g, b);
            }
        }

        if let Some(m) = p.get("metallic").and_then(Value::as_f64) {
            mat.metallic = (m as f32).clamp(0.0, 1.0);
        }

        if let Some(r) = p
            .get("roughness")
            .or_else(|| p.get("perceptual_roughness"))
            .and_then(Value::as_f64)
        {
            mat.perceptual_roughness = (r as f32).clamp(0.0, 1.0);
        }

        if let Some(u) = p.get("unlit").and_then(Value::as_bool) {
            mat.unlit = u;
        }

        (
            mat.base_color.to_srgba().to_f32_array(),
            mat.metallic,
            mat.perceptual_roughness,
        )
    };

    if let Some(mut mat_comp) = world.get_mut::<MeshMaterial3d<StandardMaterial>>(entity) {
        mat_comp.set_changed();
    }

    Ok(json!({
        "entity": entity.to_bits(),
        "color": updated_rgba,
        "metallic": updated_metallic,
        "roughness": updated_roughness,
    }))
}

fn rpc_delete_entity(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
    let entity = resolve_target_entity(world, params.as_ref())?;
    let name = world.get::<Name>(entity).map(|n| n.as_str().to_string());

    if let Some(mut sel) = world.get_resource_mut::<InspectorSelection>() {
        if sel.0 == Some(entity) {
            sel.0 = None;
        }
    }

    world.despawn(entity);

    Ok(json!({
        "deleted": entity.to_bits(),
        "name": name,
    }))
}

fn rpc_new_scene(
    world: &mut World,
    params: Option<Value>,
    default_starter: bool,
) -> Result<Value, (i32, String)> {
    let spawn_starter = params
        .as_ref()
        .and_then(|p| p.get("starter"))
        .and_then(Value::as_bool)
        .unwrap_or(default_starter);

    let mut q = world.query_filtered::<Entity, (With<SceneEntity>, Without<EditorInternal>)>();
    let to_despawn: Vec<Entity> = q.iter(world).collect();
    let removed_count = to_despawn.len();

    for e in to_despawn {
        world.despawn(e);
    }

    if let Some(mut sel) = world.get_resource_mut::<InspectorSelection>() {
        sel.0 = None;
    }
    if let Some(mut cur) = world.get_resource_mut::<CurrentScene>() {
        cur.file_path = None;
        cur.is_modified = false;
    }

    if spawn_starter {
        let _ = world.run_system_once(primitives::spawn_starter_scene);
    }

    let mut scene_query = world.query_filtered::<Entity, With<SceneEntity>>();
    let new_count = scene_query.iter(world).count();

    Ok(json!({
        "status": "ok",
        "removed_entities": removed_count,
        "scene_entity_count": new_count,
        "starter_spawned": spawn_starter,
    }))
}

fn sidecar_manifest_path(scene_path: &Path) -> PathBuf {
    let mut s = scene_path.as_os_str().to_os_string();
    s.push(".bevylite.json");
    PathBuf::from(s)
}

fn rpc_save_scene(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
    let path_str = params
        .as_ref()
        .and_then(|p| p.get("path"))
        .and_then(Value::as_str)
        .unwrap_or("assets/scenes/saved_scene.scn.ron");

    let path = PathBuf::from(path_str);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| (-32001, format!("Failed to create parent directory: {e}")))?;
        }
    }

    // 1. Save Bevy DynamicWorld RON scene
    save_load::save_world_to_scene(world, &path)
        .map_err(|e| (-32002, format!("Failed to save RON scene: {e}")))?;

    // 2. Also build & save deterministic JSON scene manifest so procedural primitives and materials roundtrip 100%
    let mut entries = Vec::new();
    let mut q = world.query_filtered::<(
        Option<&Name>,
        Option<&PrimitiveMeta>,
        Option<&Transform>,
        Option<&Mesh3d>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&PointLight>,
        Option<&DirectionalLight>,
        Option<&SpotLight>,
    ), (With<SceneEntity>, Without<EditorInternal>)>();

    let raw: Vec<_> = q
        .iter(world)
        .map(|(n, meta, tf, m, mat, pl, dl, sl)| {
            let name = n
                .map(|x| x.as_str().to_string())
                .unwrap_or_else(|| "Entity".to_string());
            let kind = if let Some(pm) = meta {
                pm.kind.clone()
            } else if dl.is_some() {
                "directional_light".to_string()
            } else if pl.is_some() {
                "point_light".to_string()
            } else if m.is_some() {
                let lname = name.to_lowercase();
                if lname.contains("plane") || lname.contains("ground") {
                    "plane".to_string()
                } else if lname.contains("sphere") {
                    "sphere".to_string()
                } else if lname.contains("cylinder") {
                    "cylinder".to_string()
                } else {
                    "cube".to_string()
                }
            } else {
                "empty".to_string()
            };
            (
                name,
                kind,
                tf.copied().unwrap_or(Transform::IDENTITY),
                mat.map(|x| x.0.id()),
                pl.copied(),
                dl.copied(),
                sl.copied(),
            )
        })
        .collect();

    let materials = world.get_resource::<Assets<StandardMaterial>>();
    for (name, kind, tf, mat_id, pl, dl, sl) in raw {
        let mat_data = mat_id.and_then(|id| materials.and_then(|m| m.get(id)));
        let (light_intensity, light_range, light_color_rgb, shadows_enabled) =
            if let Some(p_light) = pl {
                let c = p_light.color.to_srgba();
                (
                    Some(p_light.intensity),
                    Some(p_light.range),
                    Some([c.red, c.green, c.blue]),
                    Some(p_light.shadow_maps_enabled),
                )
            } else if let Some(s_light) = sl {
                let c = s_light.color.to_srgba();
                (
                    Some(s_light.intensity),
                    Some(s_light.range),
                    Some([c.red, c.green, c.blue]),
                    Some(s_light.shadow_maps_enabled),
                )
            } else if let Some(d_light) = dl {
                let c = d_light.color.to_srgba();
                (
                    Some(d_light.illuminance),
                    None,
                    Some([c.red, c.green, c.blue]),
                    Some(d_light.shadow_maps_enabled),
                )
            } else {
                (None, None, None, None)
            };

        entries.push(SavedEntityEntry {
            name,
            kind,
            translation: tf.translation.to_array(),
            rotation_xyzw: tf.rotation.to_array(),
            scale: tf.scale.to_array(),
            base_color_rgba: mat_data.map(|m| m.base_color.to_srgba().to_f32_array()),
            metallic: mat_data.map(|m| m.metallic),
            roughness: mat_data.map(|m| m.perceptual_roughness),
            emissive_rgb: mat_data.map(|m| [m.emissive.red, m.emissive.green, m.emissive.blue]),
            unlit: mat_data.map(|m| m.unlit),
            light_intensity,
            light_range,
            light_color_rgb,
            shadows_enabled,
        });
    }

    let manifest = SavedSceneManifest {
        entities: entries.clone(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| (-32003, format!("JSON serialization error: {e}")))?;

    let manifest_path = sidecar_manifest_path(&path);
    std::fs::write(&manifest_path, &manifest_json)
        .map_err(|e| (-32004, format!("Failed to write scene manifest: {e}")))?;
    crate::scene::environment::save_environment(world, &path).map_err(|e| (-32004, format!("Failed to write world settings: {e}")))?;

    if let Some(mut cur) = world.get_resource_mut::<CurrentScene>() {
        cur.file_path = Some(path.clone());
        cur.is_modified = false;
    }

    Ok(json!({
        "status": "saved",
        "path": path.to_string_lossy(),
        "manifest_path": manifest_path.to_string_lossy(),
        "entities_saved": entries.len(),
    }))
}

fn rpc_load_scene(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
    let path_str = params
        .as_ref()
        .and_then(|p| p.get("path"))
        .and_then(Value::as_str)
        .unwrap_or("assets/scenes/saved_scene.scn.ron");

    let clear_existing = params
        .as_ref()
        .and_then(|p| p.get("clear"))
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let path = PathBuf::from(path_str);
    let manifest_path = sidecar_manifest_path(&path);

    if !path.exists() && !manifest_path.exists() {
        return Err(invalid_params(format!(
            "Scene file does not exist: {}",
            path.display()
        )));
    }

    if clear_existing {
        let mut q = world.query_filtered::<Entity, (With<SceneEntity>, Without<EditorInternal>)>();
        let to_despawn: Vec<Entity> = q.iter(world).collect();
        for e in to_despawn {
            world.despawn(e);
        }
        if let Some(mut sel) = world.get_resource_mut::<InspectorSelection>() {
            sel.0 = None;
        }
    }

    let mut loaded_count = 0usize;

    // Prefer deterministic manifest if present (preserves procedural meshes & materials)
    // A sidecar holding only world settings (written by UI saves) is not an entity manifest.
    let manifest_has_entities = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .is_some_and(|doc| doc.get("entities").is_some());
    let json_source = if manifest_has_entities {
        Some(manifest_path)
    } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
        Some(path.clone())
    } else {
        None
    };

    if let Some(jpath) = json_source {
        let raw_str = std::fs::read_to_string(&jpath)
            .map_err(|e| (-32005, format!("Failed to read scene manifest: {e}")))?;
        let manifest: SavedSceneManifest = serde_json::from_str(&raw_str)
            .map_err(|e| (-32006, format!("Invalid scene manifest JSON: {e}")))?;

        for entry in manifest.entities {
            let tf = Transform {
                translation: Vec3::from_array(entry.translation),
                rotation: Quat::from_array(entry.rotation_xyzw),
                scale: Vec3::from_array(entry.scale),
            };

            match entry.kind.as_str() {
                "cube" | "sphere" | "plane" | "cylinder" => {
                    let mut m = match entry.kind.as_str() {
                        "sphere" => Sphere::new(0.5).mesh().ico(5).unwrap(),
                        "plane" => Plane3d::default().mesh().size(20.0, 20.0).build(),
                        "cylinder" => Cylinder::new(0.5, 1.0).mesh().build(),
                        _ => Cuboid::new(1.0, 1.0, 1.0).mesh().build(),
                    };
                    ensure_mesh_uv1(&mut m);
                    let mesh_handle = world.resource_mut::<Assets<Mesh>>().add(m);

                    let rgba = entry.base_color_rgba.unwrap_or([0.8, 0.8, 0.8, 1.0]);
                    let em = entry.emissive_rgb.unwrap_or([0.0, 0.0, 0.0]);
                    let mat_handle = world.resource_mut::<Assets<StandardMaterial>>().add(
                        StandardMaterial {
                            base_color: Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3]),
                            metallic: entry.metallic.unwrap_or(0.0),
                            perceptual_roughness: entry.roughness.unwrap_or(0.5),
                            emissive: LinearRgba::rgb(em[0], em[1], em[2]),
                            unlit: entry.unlit.unwrap_or(false),
                            ..default()
                        },
                    );

                    world.spawn((
                        SceneEntity,
                        PrimitiveMeta {
                            kind: entry.kind.clone(),
                        },
                        Name::new(entry.name),
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(mat_handle),
                        tf,
                    ));
                    loaded_count += 1;
                }
                "point_light" => {
                    let c = entry.light_color_rgb.unwrap_or([1.0, 1.0, 1.0]);
                    world.spawn((
                        SceneEntity,
                        PrimitiveMeta {
                            kind: "point_light".to_string(),
                        },
                        Name::new(entry.name),
                        PointLight {
                            color: Color::srgb(c[0], c[1], c[2]),
                            intensity: entry.light_intensity.unwrap_or(100_000.0),
                            range: entry.light_range.unwrap_or(20.0),
                            shadow_maps_enabled: entry.shadows_enabled.unwrap_or(true),
                            ..default()
                        },
                        tf,
                    ));
                    loaded_count += 1;
                }
                "directional_light" => {
                    let c = entry.light_color_rgb.unwrap_or([1.0, 1.0, 1.0]);
                    world.spawn((
                        SceneEntity,
                        PrimitiveMeta {
                            kind: "directional_light".to_string(),
                        },
                        Name::new(entry.name),
                        DirectionalLight {
                            color: Color::srgb(c[0], c[1], c[2]),
                            illuminance: entry.light_intensity.unwrap_or(12_000.0),
                            shadow_maps_enabled: entry.shadows_enabled.unwrap_or(true),
                            ..default()
                        },
                        tf,
                    ));
                    loaded_count += 1;
                }
                "spot_light" => {
                    let c = entry.light_color_rgb.unwrap_or([1.0, 1.0, 1.0]);
                    world.spawn((
                        SceneEntity,
                        PrimitiveMeta { kind: "spot_light".to_string() },
                        Name::new(entry.name),
                        SpotLight {
                            color: Color::srgb(c[0], c[1], c[2]),
                            intensity: entry.light_intensity.unwrap_or(400_000.0),
                            range: entry.light_range.unwrap_or(30.0),
                            shadow_maps_enabled: entry.shadows_enabled.unwrap_or(true),
                            ..default()
                        },
                        tf,
                    ));
                    loaded_count += 1;
                }
                "camera" => {
                    world.spawn((
                        SceneEntity,
                        PrimitiveMeta { kind: "camera".to_string() },
                        Name::new(entry.name),
                        Camera3d::default(),
                        Camera { is_active: false, ..default() },
                        tf,
                    ));
                    loaded_count += 1;
                }
                _ => {
                    world.spawn((SceneEntity, Name::new(entry.name), tf));
                    loaded_count += 1;
                }
            }
        }
    } else {
        let asset_server = world.resource::<AssetServer>().clone();
        let rel_path = path
            .strip_prefix("assets/")
            .or_else(|_| path.strip_prefix("assets\\"))
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        world.spawn((
            SceneEntity,
            Name::new("Loaded Scene Root"),
            bevy::world_serialization::DynamicWorldRoot(asset_server.load(rel_path)),
        ));
        loaded_count = 1;
    }

    if let Some(mut cur) = world.get_resource_mut::<CurrentScene>() {
        cur.file_path = Some(path.clone());
        cur.is_modified = false;
    }

    Ok(json!({
        "status": "loaded",
        "path": path.to_string_lossy(),
        "entities_loaded": loaded_count,
    }))
}

fn rpc_bake(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
    let backend_str = params
        .as_ref()
        .and_then(|p| p.get("backend"))
        .and_then(Value::as_str)
        .unwrap_or("gpu")
        .to_lowercase();

    let backend = match backend_str.as_str() {
        "cpu" => LightmapBackend::Cpu,
        _ => LightmapBackend::Gpu,
    };

    let resolution = params
        .as_ref()
        .and_then(|p| p.get("resolution"))
        .and_then(Value::as_u64)
        .map(|r| (r as u32).clamp(64, 4096))
        .unwrap_or(512);

    let samples = params
        .as_ref()
        .and_then(|p| p.get("samples").or_else(|| p.get("hemisphere_samples")))
        .and_then(Value::as_u64)
        .map(|s| (s as u32).clamp(4, 512))
        .unwrap_or(64);

    let cfg = LightmapConfig {
        resolution,
        samples_per_pixel: samples,
        backend,
        ..default()
    };

    if let Some(mut res_cfg) = world.get_resource_mut::<LightmapConfig>() {
        *res_cfg = cfg.clone();
    }

    world.write_message(BakeLightmapRequest {
        custom_config: Some(cfg),
    });

    Ok(json!({
        "status": "baking_started",
        "backend": format!("{:?}", backend),
        "resolution": resolution,
        "samples": samples,
    }))
}

fn rpc_clear_lightmaps(world: &mut World) -> Result<Value, (i32, String)> {
    let mut lightmapped_entities = Vec::new();
    let mut query = world.query_filtered::<Entity, With<bevy::pbr::Lightmap>>();
    for entity in query.iter(world) {
        lightmapped_entities.push(entity);
    }

    let count = lightmapped_entities.len();
    for entity in lightmapped_entities {
        world.entity_mut(entity).remove::<bevy::pbr::Lightmap>();
        if let Some(mut mesh) = world.get_mut::<Mesh3d>(entity) {
            mesh.set_changed();
        }
        if let Some(mut mat) = world.get_mut::<MeshMaterial3d<StandardMaterial>>(entity) {
            mat.set_changed();
        }
    }

    if let Some(mut active) = world.get_resource_mut::<ActiveLightmap>() {
        active.handle = None;
        active.backend_used = None;
    }

    if let Some(mut progress) = world.get_resource_mut::<LightmapBakeProgress>() {
        progress.is_baking = false;
        progress.progress = 0.0;
        progress.status = "Lightmaps cleared".to_string();
        progress.completion_timer = 2.0;
    }

    Ok(json!({
        "status": "ok",
        "cleared_count": count,
    }))
}

fn rpc_screenshot(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
    let path_str = params
        .as_ref()
        .and_then(|p| p.get("path"))
        .and_then(Value::as_str)
        .unwrap_or("screenshots/editor_capture.png");

    let path = PathBuf::from(path_str);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| (-32007, format!("Failed to create screenshot directory: {e}")))?;
        }
    }

    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));

    let abs_path = std::env::current_dir()
        .map(|cwd| cwd.join(&path))
        .unwrap_or_else(|_| path.clone());

    Ok(json!({
        "status": "capturing",
        "path": abs_path.to_string_lossy(),
    }))
}

fn rpc_play_mode(world: &mut World, params: Option<Value>) -> Result<Value, (i32, String)> {
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
        _ => {
            return Err(invalid_params(format!(
                "Invalid action '{action}'. Use 'play', 'pause', or 'stop'"
            )))
        }
    };

    world.write_message(play_action);

    Ok(json!({
        "status": "ok",
        "action": action,
    }))
}

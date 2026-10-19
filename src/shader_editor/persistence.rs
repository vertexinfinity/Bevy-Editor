//! Scene sidecar for shader assignments.
//!
//! `ShaderMaterialRef` is reflected, so Bevy's RON scene keeps it. The editor's JSON scene
//! manifest (preferred by `editor/load_scene` because it rebuilds procedural meshes) only knows
//! standard materials, so this module mirrors assignments into `<scene>.shaders.json` whenever
//! a scene is saved and re-applies them by entity name after a load.
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::component::ShaderMaterialRef;
use crate::scene::{CurrentScene, SceneEntity};

#[derive(Serialize, Deserialize)]
struct SidecarEntry {
    name: String,
    shader: String,
    color: [f32; 4],
    params: [f32; 4],
}

/// Pending restore after a scene load: sidecar entries and the time left to retry.
#[derive(Resource, Default)]
pub struct PendingShaderRestore {
    entries: Vec<SidecarEntry>,
    seconds_left: f32,
}

pub fn sidecar_path(scene: &Path) -> PathBuf {
    let mut name = scene.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".shaders.json");
    scene.with_file_name(name)
}

fn write_sidecar(world: &mut World, scene: &Path) {
    let entries: Vec<SidecarEntry> = world
        .query_filtered::<(&Name, &ShaderMaterialRef), With<SceneEntity>>()
        .iter(world)
        .map(|(name, r)| SidecarEntry {
            name: name.as_str().to_string(),
            shader: r.shader.clone(),
            color: r.color.to_srgba().to_f32_array(),
            params: r.params(),
        })
        .collect();
    let path = sidecar_path(scene);
    if entries.is_empty() {
        let _ = std::fs::remove_file(&path);
        return;
    }
    match serde_json::to_string_pretty(&entries) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                warn!("Could not write shader sidecar {}: {e}", path.display());
            }
        }
        Err(e) => warn!("Could not serialize shader sidecar: {e}"),
    }
}

/// Saves (scene file just rewritten) or schedules a restore (scene just loaded) on scene changes.
/// A save is recognised by the scene file's modification time changing since we last looked.
pub fn track_scene_saves(world: &mut World, mut seen: Local<std::collections::HashMap<PathBuf, SystemTime>>) {
    let Some(scene) = world.get_resource_ref::<CurrentScene>() else { return };
    if !scene.is_changed() || scene.is_added() {
        return;
    }
    let Some(path) = scene.file_path.clone() else { return };
    let Some(modified) = std::fs::metadata(&path).and_then(|m| m.modified()).ok() else { return };
    let fresh = SystemTime::now().duration_since(modified).is_ok_and(|age| age < Duration::from_secs(3));
    let saved = match seen.insert(path.clone(), modified) {
        Some(previous) => previous != modified,
        None => fresh,
    };
    if saved {
        write_sidecar(world, &path);
    } else if let Ok(text) = std::fs::read_to_string(sidecar_path(&path)) {
        if let Ok(entries) = serde_json::from_str::<Vec<SidecarEntry>>(&text) {
            *world.resource_mut::<PendingShaderRestore>() = PendingShaderRestore { entries, seconds_left: 5.0 };
        }
    }
}

/// Re-applies sidecar assignments to loaded entities that lack one (retries while assets spawn).
pub fn restore_shader_assignments(world: &mut World) {
    if world.resource::<PendingShaderRestore>().entries.is_empty() {
        return;
    }
    let dt = world.resource::<Time<Real>>().delta_secs();
    let candidates: Vec<(Entity, String)> = world
        .query_filtered::<(Entity, &Name), (With<Mesh3d>, Without<ShaderMaterialRef>)>()
        .iter(world)
        .map(|(e, n)| (e, n.as_str().to_string()))
        .collect();
    let mut pending = std::mem::take(&mut *world.resource_mut::<PendingShaderRestore>());
    pending.entries.retain(|entry| {
        let Some((entity, _)) = candidates.iter().find(|(_, n)| *n == entry.name) else { return true };
        let [r, g, b, a] = entry.color;
        world.entity_mut(*entity).insert(ShaderMaterialRef {
            shader: entry.shader.clone(),
            color: Color::srgba(r, g, b, a),
            p0: entry.params[0],
            p1: entry.params[1],
            p2: entry.params[2],
            p3: entry.params[3],
        });
        false
    });
    pending.seconds_left -= dt;
    if pending.seconds_left > 0.0 {
        *world.resource_mut::<PendingShaderRestore>() = pending;
    }
}

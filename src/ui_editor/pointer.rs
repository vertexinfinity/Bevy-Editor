//! Scripted mouse input for remote testing (`editor/ui_pointer`): injects real picking
//! `PointerInput` messages, one step per frame, so clicks and drags go through the same path as
//! the user's mouse (editor canvas selection/drag in Edit mode, widgets in Play mode).
use std::collections::VecDeque;

use bevy::{
    camera::NormalizedRenderTarget,
    picking::{
        pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
        PickingSystems,
    },
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};
use serde_json::{json, Value};

#[derive(Resource, Default)]
pub struct PointerScript {
    steps: VecDeque<(Vec2, Option<PointerAction>)>,
    /// Window the script targets (defaults to the primary window).
    window: Option<Entity>,
    last: Vec2,
}

pub fn install(app: &mut App) {
    app.init_resource::<PointerScript>().add_systems(First, emit_pointer_steps.in_set(PickingSystems::Input));
}

fn emit_pointer_steps(
    mut script: ResMut<PointerScript>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut out: MessageWriter<PointerInput>,
) {
    let Some((pos, action)) = script.steps.pop_front() else { return };
    let window = script.window.or_else(|| primary.single().ok());
    let Some(target) = WindowRef::Primary.normalize(window) else { return };
    let location = Location { target: NormalizedRenderTarget::Window(target), position: pos };
    let delta = pos - script.last;
    script.last = pos;
    out.write(PointerInput::new(PointerId::Mouse, location.clone(), PointerAction::Move { delta }));
    if let Some(action) = action {
        out.write(PointerInput::new(PointerId::Mouse, location, action));
    }
}

/// `{x, y, to_x?, to_y?, steps?, window?}`: click at (x, y), or drag from (x, y) to (to_x, to_y).
/// `window` is an entity id (e.g. the Game View window); coordinates are logical pixels.
pub fn rpc(world: &mut World, p: &Value) -> Result<Value, (i32, String)> {
    let num = |k: &str| p.get(k).and_then(Value::as_f64).map(|v| v as f32);
    let (x, y) = (num("x").ok_or((-32602, "Missing x".to_string()))?, num("y").ok_or((-32602, "Missing y".to_string()))?);
    let from = Vec2::new(x, y);
    let to = match (num("to_x"), num("to_y")) {
        (Some(a), Some(b)) => Some(Vec2::new(a, b)),
        _ => None,
    };
    let window = target_window(world, p);
    let steps = p.get("steps").and_then(Value::as_u64).unwrap_or(8).clamp(1, 120) as usize;
    let mut script = world.resource_mut::<PointerScript>();
    script.window = window;
    script.steps.push_back((from, None));
    script.steps.push_back((from, None));
    script.steps.push_back((from, Some(PointerAction::Press(PointerButton::Primary))));
    if let Some(to) = to {
        for i in 1..=steps {
            script.steps.push_back((from.lerp(to, i as f32 / steps as f32), None));
        }
        script.steps.push_back((to, Some(PointerAction::Release(PointerButton::Primary))));
    } else {
        script.steps.push_back((from, Some(PointerAction::Release(PointerButton::Primary))));
    }
    Ok(json!({ "queued_frames": script.steps.len() }))
}

/// `window`: an entity id, or `"game_view"` for the Play-mode window; default primary.
fn target_window(world: &World, p: &Value) -> Option<Entity> {
    match p.get("window") {
        Some(Value::String(s)) if s == "game_view" => world.resource::<crate::core::play_mode::PlaySession>().window,
        Some(v) => v.as_u64().and_then(Entity::try_from_bits),
        None => None,
    }
}

/// `{path, window?}`: screenshot of the primary window or the Game View (`window: "game_view"`).
pub fn screenshot(world: &mut World, p: &Value) -> Result<Value, (i32, String)> {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let path = p.get("path").and_then(Value::as_str).ok_or((-32602, "Missing path".to_string()))?.to_string();
    let shot = match target_window(world, p) {
        Some(w) => Screenshot::window(w),
        None if p.get("window").is_some() => return Err((-32602, "Window not found (is Play mode running?)".to_string())),
        None => Screenshot::primary_window(),
    };
    world.spawn(shot).observe(save_to_disk(path.clone()));
    Ok(json!({ "status": "capturing", "path": path }))
}

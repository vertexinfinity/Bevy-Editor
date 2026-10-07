//! Extensible Editor Tool System.
//!
//! Tools define how the user interacts with objects in the viewport (Select, Move, Rotate, Scale).
//! Any plugin can register new tools (e.g. TerrainTool, PaintTool, NavMeshTool) via [`ToolRegistry`].

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use bevy::prelude::*;

/// Trait defining an editor tool.
pub trait EditorTool: Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn icon(&self) -> &'static str;
    fn shortcut_key(&self) -> Option<KeyCode>;
    fn on_activate(&self, _world: &mut World) {}
    fn on_deactivate(&self, _world: &mut World) {}
}

/// Active tool in the editor.
#[derive(Resource)]
pub struct ActiveTool {
    pub current_id: &'static str,
}

impl Default for ActiveTool {
    fn default() -> Self {
        Self { current_id: "translate" }
    }
}

/// Registry of all installed editor tools.
#[derive(Resource, Default)]
pub struct ToolRegistry {
    tools: HashMap<&'static str, Arc<dyn EditorTool>>,
    order: Vec<&'static str>,
}

impl ToolRegistry {
    pub fn register<T: EditorTool>(&mut self, tool: T) {
        let id = tool.id();
        self.tools.insert(id, Arc::new(tool));
        if !self.order.contains(&id) {
            self.order.push(id);
        }
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn EditorTool>> {
        self.tools.get(id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn EditorTool>> {
        self.order.iter().filter_map(|id| self.get(id)).collect()
    }
}

// Built-in tools

pub struct SelectTool;
impl EditorTool for SelectTool {
    fn id(&self) -> &'static str { "select" }
    fn name(&self) -> &'static str { "Select" }
    fn icon(&self) -> &'static str { "⬚" }
    fn shortcut_key(&self) -> Option<KeyCode> { Some(KeyCode::KeyQ) }
}

pub struct TranslateTool;
impl EditorTool for TranslateTool {
    fn id(&self) -> &'static str { "translate" }
    fn name(&self) -> &'static str { "Move" }
    fn icon(&self) -> &'static str { "✥" }
    fn shortcut_key(&self) -> Option<KeyCode> { Some(KeyCode::KeyW) }
}

pub struct RotateTool;
impl EditorTool for RotateTool {
    fn id(&self) -> &'static str { "rotate" }
    fn name(&self) -> &'static str { "Rotate" }
    fn icon(&self) -> &'static str { "↻" }
    fn shortcut_key(&self) -> Option<KeyCode> { Some(KeyCode::KeyE) }
}

pub struct ScaleTool;
impl EditorTool for ScaleTool {
    fn id(&self) -> &'static str { "scale" }
    fn name(&self) -> &'static str { "Scale" }
    fn icon(&self) -> &'static str { "⤢" }
    fn shortcut_key(&self) -> Option<KeyCode> { Some(KeyCode::KeyR) }
}

/// System switching active tool via Q/W/E/R hotkeys (when RMB is not held).
pub fn handle_tool_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    registry: Res<ToolRegistry>,
    mut active_tool: ResMut<ActiveTool>,
) {
    // Only switch tools if RMB is not held (RMB is reserved for camera flight)
    if mouse.pressed(MouseButton::Right) {
        return;
    }

    for tool in registry.list() {
        if let Some(key) = tool.shortcut_key() {
            if keys.just_pressed(key) {
                active_tool.current_id = tool.id();
                info!("Switched active tool to: {}", tool.name());
                break;
            }
        }
    }
}

pub struct ToolPlugin;

impl Plugin for ToolPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = ToolRegistry::default();
        registry.register(SelectTool);
        registry.register(TranslateTool);
        registry.register(RotateTool);
        registry.register(ScaleTool);

        app.insert_resource(registry)
            .init_resource::<ActiveTool>()
            .add_systems(Update, handle_tool_shortcuts);
    }
}

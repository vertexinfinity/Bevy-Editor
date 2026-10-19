//! Shader editor: WGSL material authoring with live preview.
//!
//! ## Design
//! * **Shaders** are `.wgsl` files in `assets/shaders/` (user) and `assets/shaders/templates/`
//!   (starters: Unlit Color, Fresnel Rim, Toon, Hologram, Dissolve, Scrolling Gradient). A file
//!   defines `fn shade(in: ShaderInput) -> vec4<f32>`; a hidden prelude supplies the vertex
//!   output layout, the uniforms (`material.color`, `p0()..p3()`), time, the camera position and
//!   helpers, and a generated `@fragment` entry point calls `shade` (like Godot's
//!   `shader_type spatial` built-ins). `//!` header lines declare the display name, blend mode,
//!   color label/default and named float parameters with ranges (Unity/Godot property hints):
//!   `//! p0: Rim Power = 3.0 (0.5..8)`.
//! * **Validation**: the composed module is plain WGSL, so naga (same version as wgpu) parses
//!   and validates it before anything reaches the GPU. Errors are mapped back to the user's
//!   line/column; invalid source never replaces the last good shader.
//! * **Material**: one [`EditorShaderMaterial`] type. Its fragment shader handle is part of the
//!   bind-group data (pipeline key) and swapped in by `Material::specialize`, so many different
//!   shaders render at once. Edits replace the `Shader` asset under the same handle, which hot
//!   reloads every pipeline using it. A second "no UV" compile serves meshes without UV_0.
//! * **Persistence**: [`ShaderMaterialRef`] (reflected; shader path, color, p0..p3) is the source
//!   of truth saved with scenes. [`component::sync_shader_materials`] builds/updates the material
//!   for every entity carrying it — new scenes, loaded scenes and Play-mode copies alike — and
//!   restores the stashed standard material when the component is removed. All assignment and
//!   parameter edits go through the undo history. Because `editor/load_scene` prefers the JSON
//!   manifest (which only knows standard materials), assignments are also mirrored to a
//!   `<scene>.shaders.json` sidecar on save and re-applied by entity name after load
//!   ([`persistence`]).
//!
//! ## Remote methods (also exposed to agents through the `bevylite_editor_rpc` MCP tool)
//! | Method | Params | Purpose |
//! | :-- | :-- | :-- |
//! | `editor/shader_editor` | `open`? (default true), `path`? | Open/close the window, optionally opening a file |
//! | `editor/shaders` | | List user shaders and templates with metadata, params and validation errors |
//! | `editor/shader_get` | `path` | Source, metadata, current errors and entities using the shader |
//! | `editor/shader_set` | `path`, `source` | Validate + save + hot-apply; returns `errors` `[{line, column, message}]` (last good shader kept on error) |
//! | `editor/shader_validate` | `source` | Validate without saving |
//! | `editor/shader_create` | `name`, `template`? | New `shaders/<name>.wgsl` from a template (`toon`, `hologram`, ...) |
//! | `editor/shader_delete` | `path` | Delete a user shader file |
//! | `editor/shader_apply` | `entity`/`name` (default: selection), `path`, `color`?, `params`? | Assign a shader to a mesh (undoable) |
//! | `editor/shader_revert` | `entity`/`name` | Back to the standard material (undoable) |
//! | `editor/shader_params` | `entity`/`name`, `color`?, `params`? | Edit color (`"#RRGGBB"` or `[r,g,b,a]`) and params (`[p0..]` or `{"p0"|"<label>": v}`) |
//!
//! Paths may be given as `toon`, `toon.wgsl`, `shaders/toon.wgsl` or `shaders/templates/toon.wgsl`.
use bevy::prelude::*;
use serde_json::Value;

pub mod component;
pub mod library;
pub mod material;
pub mod ops;
pub mod persistence;
pub mod rpc;
pub mod text_buffer;
pub mod ui;

pub use component::ShaderMaterialRef;
pub use material::EditorShaderMaterial;

pub struct ShaderEditorPlugin;

impl Plugin for ShaderEditorPlugin {
    fn build(&self, app: &mut App) {
        crate::ui::windows::register_window(
            app,
            crate::ui::windows::EditorWindowEntry { id: "shader_editor", label: "🎨 Shader Editor", toggle: ui::toggle_window },
        );
        app.add_plugins(MaterialPlugin::<EditorShaderMaterial>::default())
            .register_type::<ShaderMaterialRef>()
            .init_resource::<library::ShaderLibrary>()
            .init_resource::<ops::ShaderEditorState>()
            .init_resource::<persistence::PendingShaderRestore>()
            .add_systems(PreUpdate, ui::handle_code_keyboard.after(bevy::input::InputSystems))
            .add_systems(Update, (watch_shader_files, persistence::track_scene_saves, persistence::restore_shader_assignments, component::sync_shader_materials, ui::update_shader_editor_ui).chain())
            .add_systems(OnExit(crate::ui::EditorAppState::InEditor), ui::despawn_window);
    }
}

/// Picks up shader files added or edited outside the editor (hot reload), once per second.
fn watch_shader_files(world: &mut World) {
    let dt = world.resource::<Time<Real>>().delta_secs();
    {
        let mut library = world.resource_mut::<library::ShaderLibrary>();
        library.scan_timer -= dt;
        if library.scan_timer > 0.0 {
            return;
        }
        library.scan_timer = 1.0;
    }
    let before: Vec<(String, usize, u64)> = world.resource::<library::ShaderLibrary>().entries.iter().map(|e| (e.path.clone(), e.errors.len(), e.revision)).collect();
    ops::rescan(world);
    let after: Vec<(String, usize, u64)> = world.resource::<library::ShaderLibrary>().entries.iter().map(|e| (e.path.clone(), e.errors.len(), e.revision)).collect();
    // Clean open documents follow external edits.
    let mut state = world.resource_mut::<ops::ShaderEditorState>();
    let mut reloaded = false;
    for (path, doc) in state.docs.iter_mut() {
        if doc.dirty() {
            continue;
        }
        if let Some(source) = library::ShaderLibrary::read_source(path) {
            if source != doc.saved {
                let (line, col) = (doc.buffer.line, doc.buffer.col);
                *doc = ops::Doc::new(&source, library::validate(&source).err().unwrap_or_default());
                doc.buffer.set_caret(line, col);
                reloaded = true;
            }
        }
    }
    if before != after || reloaded {
        state.list_dirty = true;
        state.params_dirty = true;
    }
}

/// Remote methods owned by this module; `None` means the method is not ours.
pub fn dispatch(world: &mut World, method: &str, params: Option<Value>) -> Option<Result<Value, (i32, String)>> {
    rpc::dispatch(world, method, params)
}

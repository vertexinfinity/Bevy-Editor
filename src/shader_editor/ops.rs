//! Editor operations shared by the window and the remote API.
use std::collections::HashMap;

use bevy::{prelude::*, shader::Shader};

use super::component::{set_shader_ref_with_undo, ShaderMaterialRef};
use super::library::{self, ShaderDiagnostic, ShaderLibrary};
use super::text_buffer::TextBuffer;

/// One open file in the code editor.
pub struct Doc {
    pub buffer: TextBuffer,
    /// Text as last saved / loaded (dirty = buffer differs).
    pub saved: String,
    pub scroll: usize,
    pub hscroll: usize,
    /// Live validation of the buffer (may be newer than the applied shader).
    pub errors: Vec<ShaderDiagnostic>,
    /// Seconds until the next live validation (None = up to date).
    pub check_in: Option<f32>,
}

impl Doc {
    pub fn new(source: &str, errors: Vec<ShaderDiagnostic>) -> Self {
        Self { buffer: TextBuffer::new(source), saved: source.to_string(), scroll: 0, hscroll: 0, errors, check_in: None }
    }
    pub fn dirty(&self) -> bool {
        self.buffer.text() != self.saved
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StatusKind {
    #[default]
    Info,
    Ok,
    Error,
}

#[derive(Resource, Default)]
pub struct ShaderEditorState {
    pub open: bool,
    pub current: Option<String>,
    pub docs: HashMap<String, Doc>,
    pub focused: bool,
    pub code_hovered: bool,
    pub status: String,
    pub status_kind: StatusKind,
    pub template_picker: bool,
    /// Delete needs a second click before this time (seconds since startup).
    pub confirm_delete_until: f64,
    /// Forces the file list / parameter panel to rebuild.
    pub list_dirty: bool,
    pub params_dirty: bool,
    /// Measured advance of one code glyph (logical px).
    pub char_width: f32,
    pub visible_rows: usize,
    pub visible_cols: usize,
    pub blink: f32,
    /// Live parameter drag: entity and its assignment before the drag.
    pub drag_before: Option<(Entity, Option<ShaderMaterialRef>)>,
    pub ensure_caret_visible: bool,
}

impl ShaderEditorState {
    pub fn set_status(&mut self, kind: StatusKind, text: impl Into<String>) {
        self.status = text.into();
        self.status_kind = kind;
    }
    pub fn doc(&self) -> Option<&Doc> {
        self.current.as_ref().and_then(|p| self.docs.get(p))
    }
    pub fn doc_mut(&mut self) -> Option<&mut Doc> {
        let path = self.current.clone()?;
        self.docs.get_mut(&path)
    }
}

fn with_library<R>(world: &mut World, f: impl FnOnce(&mut ShaderLibrary, &mut Assets<Shader>) -> R) -> R {
    world.resource_scope(|world, mut library: Mut<ShaderLibrary>| {
        let mut shaders = world.resource_mut::<Assets<Shader>>();
        f(&mut library, &mut shaders)
    })
}

pub fn rescan(world: &mut World) {
    let skip = {
        let state = world.resource::<ShaderEditorState>();
        state.current.clone().filter(|p| state.docs.get(p).is_some_and(Doc::dirty))
    };
    with_library(world, |library, shaders| library.rescan(shaders, skip.as_deref()));
}

/// Opens (or focuses) a file in the editor.
pub fn open_doc(world: &mut World, path: &str) -> Result<(), String> {
    let path = library::normalize_path(path);
    let errors = with_library(world, |library, shaders| library.ensure(shaders, &path).map(|e| e.errors.clone()))
        .ok_or_else(|| format!("Shader not found: {path}"))?;
    let mut state = world.resource_mut::<ShaderEditorState>();
    if !state.docs.contains_key(&path) {
        let source = ShaderLibrary::read_source(&path).unwrap_or_default();
        state.docs.insert(path.clone(), Doc::new(&source, errors));
    }
    state.current = Some(path.clone());
    state.list_dirty = true;
    state.params_dirty = true;
    state.ensure_caret_visible = true;
    state.set_status(StatusKind::Info, format!("Opened {path}"));
    Ok(())
}

/// Validates, saves and (when valid) hot-swaps a shader. Invalid source is still saved so no
/// work is lost, but the GPU keeps the last good version.
pub fn save_and_apply(world: &mut World, path: &str, source: &str) -> Result<(), Vec<ShaderDiagnostic>> {
    let path = library::normalize_path(path);
    let mut source = source.replace("\r\n", "\n");
    if !source.ends_with('\n') {
        source.push('\n');
    }
    let write = with_library(world, |library, shaders| {
        library.write_file(&path, &source)?;
        if library.get(&path).is_none() {
            library.ensure(shaders, &path);
        }
        Ok::<_, String>(library.apply_source(shaders, &path, &source))
    });
    let result = match write {
        Ok(result) => result,
        Err(message) => Err(vec![ShaderDiagnostic { line: 0, column: 0, message }]),
    };
    let mut state = world.resource_mut::<ShaderEditorState>();
    if let Some(doc) = state.docs.get_mut(&path) {
        doc.saved = source.clone();
        if doc.buffer.text() != source {
            let (line, col) = (doc.buffer.line, doc.buffer.col);
            doc.buffer = TextBuffer::new(&source);
            doc.buffer.set_caret(line, col);
        }
        doc.errors = result.clone().err().unwrap_or_default();
        doc.check_in = None;
    }
    match &result {
        Ok(()) => state.set_status(StatusKind::Ok, format!("Saved and applied {path}")),
        Err(errors) => {
            let first = errors.first().map(ShaderDiagnostic::short).unwrap_or_default();
            state.set_status(StatusKind::Error, format!("Saved with {} error(s); last good shader kept. {first}", errors.len()));
        }
    }
    state.list_dirty = true;
    state.params_dirty = true;
    result
}

pub fn save_current(world: &mut World) {
    let Some((path, text)) = world.resource::<ShaderEditorState>().current.clone().and_then(|path| {
        let text = world.resource::<ShaderEditorState>().docs.get(&path)?.buffer.text();
        Some((path, text))
    }) else {
        world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Info, "Open a shader first");
        return;
    };
    let _ = save_and_apply(world, &path, &text);
}

/// Discards unsaved edits of the current file.
pub fn revert_current(world: &mut World) {
    let mut state = world.resource_mut::<ShaderEditorState>();
    let Some(doc) = state.doc_mut() else { return };
    let saved = doc.saved.clone();
    let (line, col) = (doc.buffer.line, doc.buffer.col);
    doc.buffer = TextBuffer::new(&saved);
    doc.buffer.set_caret(line, col);
    doc.check_in = Some(0.0);
    state.set_status(StatusKind::Info, "Reverted unsaved changes");
}

/// Creates `shaders/<name>.wgsl` from a template; returns the new path.
pub fn create_from_template(world: &mut World, name: Option<&str>, template: &str, unique: bool) -> Result<String, String> {
    let source = world.resource::<ShaderLibrary>().template_source(template).ok_or_else(|| format!("Unknown template '{template}'"))?;
    let base = name.map(str::to_string).unwrap_or_else(|| library::file_stem(&library::normalize_path(template)));
    let path = if unique {
        world.resource::<ShaderLibrary>().unique_path(&base)
    } else {
        let path = format!("{}/{}.wgsl", library::SHADER_DIR, library::sanitize_name(&base));
        if library::disk_path(&path).exists() {
            return Err(format!("{path} already exists"));
        }
        path
    };
    with_library(world, |library, _| library.write_file(&path, &source))?;
    rescan(world);
    open_doc(world, &path)?;
    world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Ok, format!("Created {path} from template {template}"));
    Ok(path)
}

pub fn duplicate_current(world: &mut World) -> Result<String, String> {
    let (path, text) = {
        let state = world.resource::<ShaderEditorState>();
        let path = state.current.clone().ok_or("Open a shader to duplicate")?;
        let text = state.docs.get(&path).map(|d| d.buffer.text()).unwrap_or_default();
        (path, text)
    };
    let new_path = world.resource::<ShaderLibrary>().unique_path(&format!("{}_copy", library::file_stem(&path)));
    with_library(world, |library, _| library.write_file(&new_path, &text))?;
    rescan(world);
    open_doc(world, &new_path)?;
    world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Ok, format!("Duplicated to {new_path}"));
    Ok(new_path)
}

pub fn delete_shader(world: &mut World, path: &str) -> Result<(), String> {
    let path = library::normalize_path(path);
    if path.starts_with(library::TEMPLATE_DIR) {
        return Err("Templates cannot be deleted".into());
    }
    let disk = library::disk_path(&path);
    std::fs::remove_file(&disk).map_err(|e| format!("Cannot delete {}: {e}", disk.display()))?;
    world.resource_mut::<ShaderLibrary>().entries.retain(|e| e.path != path);
    let mut state = world.resource_mut::<ShaderEditorState>();
    state.docs.remove(&path);
    if state.current.as_deref() == Some(path.as_str()) {
        state.current = None;
    }
    state.list_dirty = true;
    state.params_dirty = true;
    state.set_status(StatusKind::Info, format!("Deleted {path} (objects using it keep the compiled shader until reload)"));
    Ok(())
}

pub fn shader_meta(world: &mut World, path: &str) -> Option<library::ShaderMeta> {
    with_library(world, |library, shaders| library.ensure(shaders, path).map(|e| e.meta.clone()))
}

/// Assigns a shader to a mesh entity (undoable). Missing params fall back to the shader defaults.
pub fn apply_to_entity(world: &mut World, entity: Entity, path: &str, color: Option<Color>, params: Option<[Option<f32>; 4]>) -> Result<ShaderMaterialRef, String> {
    let path = library::normalize_path(path);
    if world.get::<Mesh3d>(entity).is_none() {
        return Err("Entity has no mesh to shade".into());
    }
    let meta = shader_meta(world, &path).ok_or_else(|| format!("Shader not found: {path}"))?;
    let existing = world.get::<ShaderMaterialRef>(entity).cloned().filter(|r| r.shader == path);
    let base = existing.unwrap_or_else(|| {
        let d = meta.defaults();
        ShaderMaterialRef { shader: path.clone(), color: meta.color, p0: d[0], p1: d[1], p2: d[2], p3: d[3] }
    });
    let mut next = base;
    if let Some(color) = color {
        next.color = color;
    }
    if let Some(params) = params {
        for (i, value) in params.iter().enumerate() {
            if let Some(v) = value {
                next.set_param(i, *v);
            }
        }
    }
    set_shader_ref_with_undo(world, entity, Some(next.clone()), "Apply shader material")?;
    world.resource_mut::<ShaderEditorState>().params_dirty = true;
    Ok(next)
}

pub fn revert_entity(world: &mut World, entity: Entity) -> Result<(), String> {
    if world.get::<ShaderMaterialRef>(entity).is_none() {
        return Err("Entity does not use a shader material".into());
    }
    set_shader_ref_with_undo(world, entity, None, "Revert to standard material")?;
    world.resource_mut::<ShaderEditorState>().params_dirty = true;
    Ok(())
}

pub fn selected_entity(world: &World) -> Option<Entity> {
    world.get_resource::<bevy::inspector::InspectorSelection>().and_then(|s| s.0).filter(|e| world.get_entity(*e).is_ok())
}

pub fn apply_current_to_selected(world: &mut World) {
    let current = world.resource::<ShaderEditorState>().current.clone();
    let result = match (current, selected_entity(world)) {
        (None, _) => Err("Open a shader first".to_string()),
        (_, None) => Err("Select a mesh object first".to_string()),
        (Some(path), Some(entity)) => apply_to_entity(world, entity, &path, None, None).map(|_| path),
    };
    let mut state = world.resource_mut::<ShaderEditorState>();
    match result {
        Ok(path) => state.set_status(StatusKind::Ok, format!("Applied {path} to the selected object (Ctrl+Z to undo)")),
        Err(e) => state.set_status(StatusKind::Error, e),
    }
}

pub fn revert_selected(world: &mut World) {
    let result = selected_entity(world).ok_or_else(|| "Select an object first".to_string()).and_then(|e| revert_entity(world, e));
    let mut state = world.resource_mut::<ShaderEditorState>();
    match result {
        Ok(()) => state.set_status(StatusKind::Ok, "Restored the standard material (Ctrl+Z to undo)"),
        Err(e) => state.set_status(StatusKind::Error, e),
    }
}

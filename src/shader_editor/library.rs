//! Shader files on disk, the hidden WGSL prelude, naga validation and `//!` metadata.
//!
//! A user shader is a WGSL fragment body that defines `fn shade(in: ShaderInput) -> vec4<f32>`.
//! It is wrapped between [`PRELUDE_HEADER`] (vertex output layout, bindings, helpers) and a
//! footer holding the real `@fragment` entry point, so the final module is plain WGSL that naga
//! can validate completely before it reaches the GPU.
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::shader::Shader;

/// Folder (relative to the asset root) holding user shaders.
pub const SHADER_DIR: &str = "shaders";
/// Folder (relative to the asset root) holding templates.
pub const TEMPLATE_DIR: &str = "shaders/templates";

/// Built-in copies of the starter templates; the on-disk files win when present.
pub const BUILTIN_TEMPLATES: &[(&str, &str)] = &[
    ("unlit_color", include_str!("../../assets/shaders/templates/unlit_color.wgsl")),
    ("fresnel_rim", include_str!("../../assets/shaders/templates/fresnel_rim.wgsl")),
    ("toon", include_str!("../../assets/shaders/templates/toon.wgsl")),
    ("hologram", include_str!("../../assets/shaders/templates/hologram.wgsl")),
    ("dissolve", include_str!("../../assets/shaders/templates/dissolve.wgsl")),
    ("uv_scroll_gradient", include_str!("../../assets/shaders/templates/uv_scroll_gradient.wgsl")),
];

const UV_FIELD: &str = "    @location(2) uv: vec2<f32>,";
const NO_UV_FIELD: &str = "    // (mesh has no UV_0 attribute)";

/// Everything user code can see. Kept in sync with the templates' built-ins comment.
pub const PRELUDE_HEADER: &str = r#"// ---- Bevylite shader prelude (generated, not part of your file) ----
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
@@UV@@
}
struct BevyliteView {
    clip_from_world: mat4x4<f32>,
    unjittered_clip_from_world: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
}
struct BevyliteGlobals {
    time: f32,
    delta_time: f32,
    frame_count: u32,
}
struct ShaderParams {
    color: vec4<f32>,
    params: vec4<f32>,
}
struct ShaderInput {
    frag_coord: vec4<f32>,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    view_dir: vec3<f32>,
    time: f32,
    front_facing: bool,
}
@group(0) @binding(0) var<uniform> bevylite_view: BevyliteView;
@group(0) @binding(11) var<uniform> bevylite_globals: BevyliteGlobals;
@group(3) @binding(0) var<uniform> material: ShaderParams;
const PI: f32 = 3.14159265359;
const LIGHT_DIR: vec3<f32> = vec3<f32>(0.40824829, 0.81649658, 0.40824829);
fn p0() -> f32 { return material.params.x; }
fn p1() -> f32 { return material.params.y; }
fn p2() -> f32 { return material.params.z; }
fn p3() -> f32 { return material.params.w; }
fn camera_position() -> vec3<f32> { return bevylite_view.world_from_view[3].xyz; }
fn lambert(n: vec3<f32>) -> f32 { return 0.2 + 0.8 * max(dot(normalize(n), LIGHT_DIR), 0.0); }
fn fresnel(n: vec3<f32>, v: vec3<f32>, power: f32) -> f32 { return pow(1.0 - clamp(dot(normalize(n), normalize(v)), 0.0, 1.0), max(power, 0.0001)); }
fn hash11(x: f32) -> f32 { return fract(sin(x * 127.1) * 43758.5453); }
fn hash21(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x);
    let b = mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x);
    return mix(a, b, u.y);
}
fn fbm2(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var q = p;
    for (var i = 0; i < 5; i = i + 1) {
        value = value + amplitude * noise2(q);
        q = q * 2.0 + vec2<f32>(1.7, 9.2);
        amplitude = amplitude * 0.5;
    }
    return value;
}
fn hue_shift(color: vec3<f32>, angle: f32) -> vec3<f32> {
    let k = vec3<f32>(0.57735027);
    let c = cos(angle);
    return color * c + cross(k, color) * sin(angle) + k * dot(k, color) * (1.0 - c);
}
// ---- end of prelude; your file starts on the next line ----
"#;

const FOOTER: &str = r#"
// ---- Bevylite entry point (generated) ----
@fragment
fn fragment(vo: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    var s: ShaderInput;
    s.frag_coord = vo.position;
    s.world_position = vo.world_position.xyz;
    s.normal = normalize(vo.world_normal);
    s.uv = @@UVEXPR@@;
    s.view_dir = normalize(camera_position() - vo.world_position.xyz);
    s.time = bevylite_globals.time;
    s.front_facing = is_front;
    return shade(s);
}
"#;

/// Number of prelude lines before the user's first line.
pub fn header_lines() -> u32 {
    PRELUDE_HEADER.lines().count() as u32
}

/// Builds the full WGSL module for a user shader.
pub fn compose(user: &str, with_uv: bool) -> String {
    let header = PRELUDE_HEADER.replace("@@UV@@", if with_uv { UV_FIELD } else { NO_UV_FIELD });
    let footer = FOOTER.replace("@@UVEXPR@@", if with_uv { "vo.uv" } else { "vec2<f32>(0.0, 0.0)" });
    let mut out = String::with_capacity(header.len() + user.len() + footer.len() + 1);
    out.push_str(&header);
    out.push_str(user);
    if !user.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&footer);
    out
}

/// One validation problem, located in the user's file (`line` is 1-based; 0 = generated code).
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderDiagnostic {
    pub line: u32,
    pub column: u32,
    pub message: String,
}

impl ShaderDiagnostic {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "line": self.line, "column": self.column, "message": self.message })
    }
    pub fn short(&self) -> String {
        if self.line == 0 { self.message.clone() } else { format!("Line {}:{} — {}", self.line, self.column, self.message) }
    }
}

fn map_location(composed: &str, user: &str, line: u32, column: u32, message: String) -> ShaderDiagnostic {
    let header = header_lines();
    let user_lines = user.lines().count().max(1) as u32;
    let _ = composed;
    if line > header && line <= header + user_lines {
        ShaderDiagnostic { line: line - header, column, message }
    } else if line > header + user_lines {
        let hint = if message.contains("shade") {
            " — define `fn shade(in: ShaderInput) -> vec4<f32>`"
        } else {
            " (in the generated entry point)"
        };
        ShaderDiagnostic { line: 0, column: 0, message: format!("{message}{hint}") }
    } else {
        ShaderDiagnostic { line: 0, column: 0, message: format!("{message} (conflicts with a prelude declaration)") }
    }
}

fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source {
        let part = inner.to_string();
        if !text.contains(&part) {
            text.push_str(": ");
            text.push_str(&part);
        }
        source = inner.source();
    }
    text
}

/// Parses and validates a user shader with naga. `Ok` means the GPU will accept it.
pub fn validate(user: &str) -> Result<(), Vec<ShaderDiagnostic>> {
    let mut diags = Vec::new();
    for (index, line) in user.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        if code.contains("@group") || code.contains("@binding") {
            diags.push(ShaderDiagnostic {
                line: index as u32 + 1,
                column: code.find('@').unwrap_or(0) as u32 + 1,
                message: "Resource bindings are provided by the prelude (`material`, time); remove @group/@binding".into(),
            });
        }
        if code.contains("@fragment") || code.contains("@vertex") {
            diags.push(ShaderDiagnostic {
                line: index as u32 + 1,
                column: code.find('@').unwrap_or(0) as u32 + 1,
                message: "Entry points are generated; write `fn shade(in: ShaderInput) -> vec4<f32>` instead".into(),
            });
        }
    }
    if !diags.is_empty() {
        return Err(diags);
    }
    let composed = compose(user, true);
    let module = match naga::front::wgsl::parse_str(&composed) {
        Ok(module) => module,
        Err(error) => {
            // Some labels carry an undefined (empty) span; use the first real one.
            let primary = error.labels().find(|(span, _)| span.to_range().is_some_and(|r| !r.is_empty()));
            let (line, column) = primary
                .map(|(span, _)| span.location(&composed))
                .or_else(|| error.location(&composed))
                .map(|l| (l.line_number, l.line_position))
                .unwrap_or((0, 0));
            let mut message = error.message().to_string();
            if let Some((_, label)) = primary {
                if !label.is_empty() && !message.contains(label) {
                    message = format!("{message} ({label})");
                }
            }
            return Err(vec![map_location(&composed, user, line, column, message)]);
        }
    };
    let mut validator = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default());
    if let Err(error) = validator.validate(&module) {
        let primary = error.spans().find(|(span, _)| span.to_range().is_some_and(|r| !r.is_empty()));
        let (line, column) = primary
            .map(|(span, _)| span.location(&composed))
            .or_else(|| error.location(&composed))
            .map(|l| (l.line_number, l.line_position))
            .unwrap_or((0, 0));
        let mut message = error_chain(error.as_inner());
        if let Some((_, label)) = primary {
            if !label.is_empty() && !message.contains(label.as_str()) {
                message = format!("{message} ({label})");
            }
        }
        return Err(vec![map_location(&composed, user, line, column, message)]);
    }
    Ok(())
}

/// How the material blends; set with `//! alpha: opaque|blend|add|premultiplied`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShaderAlpha {
    #[default]
    Opaque,
    Blend,
    Add,
    Premultiplied,
}

impl ShaderAlpha {
    pub fn alpha_mode(self) -> AlphaMode {
        match self {
            Self::Opaque => AlphaMode::Opaque,
            Self::Blend => AlphaMode::Blend,
            Self::Add => AlphaMode::Add,
            Self::Premultiplied => AlphaMode::Premultiplied,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Opaque => "opaque",
            Self::Blend => "blend",
            Self::Add => "add",
            Self::Premultiplied => "premultiplied",
        }
    }
}

/// A named float parameter declared with `//! p0: Label = default (min..max)`.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamMeta {
    pub label: String,
    pub default: f32,
    pub min: f32,
    pub max: f32,
}

impl ParamMeta {
    fn fallback(index: usize) -> Self {
        Self { label: format!("Param {index}"), default: 0.5, min: 0.0, max: 1.0 }
    }
    pub fn step(&self) -> f32 {
        let span = (self.max - self.min).abs();
        if span <= 1.0 { 0.01 } else if span <= 10.0 { 0.1 } else { 1.0 }
    }
}

/// Header metadata parsed from `//!` lines (Godot `uniform` hints / Unity property blocks).
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderMeta {
    pub name: String,
    pub description: String,
    pub alpha: ShaderAlpha,
    pub color_label: String,
    pub color: Color,
    pub params: [ParamMeta; 4],
    /// Which `pN` slots were declared (undeclared ones are hidden in the parameter panel).
    pub declared: [bool; 4],
}

impl ShaderMeta {
    pub fn parse(file_stem: &str, source: &str) -> Self {
        let mut meta = Self {
            name: crate::core::reflect_fields::prettify(file_stem),
            description: String::new(),
            alpha: ShaderAlpha::Opaque,
            color_label: "Base Color".into(),
            color: Color::WHITE,
            params: std::array::from_fn(ParamMeta::fallback),
            declared: [false; 4],
        };
        for line in source.lines() {
            let Some(rest) = line.trim_start().strip_prefix("//!") else { continue };
            let Some((key, value)) = rest.split_once(':') else { continue };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "name" => meta.name = value.to_string(),
                "description" => meta.description = value.to_string(),
                "alpha" => {
                    meta.alpha = match value.to_ascii_lowercase().as_str() {
                        "blend" => ShaderAlpha::Blend,
                        "add" | "additive" => ShaderAlpha::Add,
                        "premultiplied" => ShaderAlpha::Premultiplied,
                        _ => ShaderAlpha::Opaque,
                    }
                }
                "color" => {
                    let (label, default) = value.split_once('=').unwrap_or((value, ""));
                    if !label.trim().is_empty() {
                        meta.color_label = label.trim().to_string();
                    }
                    if let Ok(color) = crate::core::reflect_fields::parse_color(default.trim()) {
                        meta.color = color;
                    }
                }
                "p0" | "p1" | "p2" | "p3" => {
                    let index = (key.as_bytes()[1] - b'0') as usize;
                    meta.declared[index] = true;
                    let (label, rest) = value.split_once('=').unwrap_or((value, ""));
                    let param = &mut meta.params[index];
                    if !label.trim().is_empty() {
                        param.label = label.trim().to_string();
                    }
                    let (default, range) = rest.split_once('(').unwrap_or((rest, ""));
                    if let Ok(v) = default.trim().parse::<f32>() {
                        param.default = v;
                    }
                    let range = range.trim_end_matches(')').trim();
                    if let Some((min, max)) = range.split_once("..") {
                        if let (Ok(min), Ok(max)) = (min.trim().parse::<f32>(), max.trim().parse::<f32>()) {
                            if max > min {
                                param.min = min;
                                param.max = max;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        meta
    }

    pub fn defaults(&self) -> [f32; 4] {
        std::array::from_fn(|i| self.params[i].default)
    }
}

/// One shader known to the library, with GPU handles that stay stable across edits.
pub struct ShaderEntry {
    pub path: String,
    pub template: bool,
    /// Source currently applied to the GPU (the last version that validated).
    pub applied_source: String,
    pub meta: ShaderMeta,
    pub handle: Handle<Shader>,
    pub handle_no_uv: Handle<Shader>,
    /// Errors from the most recent validation attempt (empty = applied cleanly).
    pub errors: Vec<ShaderDiagnostic>,
    pub modified: Option<std::time::SystemTime>,
    /// Bumped whenever the applied source or metadata changes (materials refresh their alpha mode).
    pub revision: u64,
}

/// All shaders found in `assets/shaders` and `assets/shaders/templates`, keyed by asset path.
#[derive(Resource, Default)]
pub struct ShaderLibrary {
    pub entries: Vec<ShaderEntry>,
    pub scan_timer: f32,
}

/// Error shader used until a file first validates (magenta checker so problems are visible).
const ERROR_SHADER: &str = "fn shade(in: ShaderInput) -> vec4<f32> {\n    let c = floor(in.uv * 8.0);\n    let k = (c.x + c.y) - 2.0 * floor((c.x + c.y) * 0.5);\n    return vec4<f32>(mix(vec3<f32>(1.0, 0.0, 1.0), vec3<f32>(0.1, 0.0, 0.1), k), 1.0);\n}\n";

/// Absolute asset root (matches the editor's `AssetPlugin::file_path`).
pub fn asset_root() -> PathBuf {
    std::env::current_dir()
        .ok()
        .map(|d| d.join("assets"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| PathBuf::from("assets"))
}

/// Normalizes `"toon"`, `"toon.wgsl"`, `"shaders/toon.wgsl"` or `"assets/shaders/toon.wgsl"` to an asset path.
pub fn normalize_path(input: &str) -> String {
    let mut path = input.trim().replace('\\', "/");
    if let Some(stripped) = path.strip_prefix("assets/") {
        path = stripped.to_string();
    }
    if !path.ends_with(".wgsl") {
        path.push_str(".wgsl");
    }
    if !path.contains('/') {
        // Bare names prefer a user shader, then a template of that name.
        let user = format!("{SHADER_DIR}/{path}");
        let template = format!("{TEMPLATE_DIR}/{path}");
        let stem = path.trim_end_matches(".wgsl").to_string();
        let template_exists = disk_path(&template).exists() || BUILTIN_TEMPLATES.iter().any(|(n, _)| *n == stem);
        path = if !disk_path(&user).exists() && template_exists { template } else { user };
    }
    path
}

pub fn disk_path(asset_path: &str) -> PathBuf {
    asset_root().join(asset_path)
}

pub fn file_stem(asset_path: &str) -> String {
    Path::new(asset_path).file_stem().and_then(|s| s.to_str()).unwrap_or("shader").to_string()
}

/// A file name made only of `[a-z0-9_]`.
pub fn sanitize_name(name: &str) -> String {
    let mut out: String = name
        .trim()
        .trim_end_matches(".wgsl")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() { "new_shader".into() } else { out }
}

fn modified_time(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

fn wgsl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "wgsl")).collect())
        .unwrap_or_default();
    files.sort();
    files
}

impl ShaderLibrary {
    pub fn get(&self, path: &str) -> Option<&ShaderEntry> {
        self.entries.iter().find(|e| e.path == path)
    }

    pub fn get_mut(&mut self, path: &str) -> Option<&mut ShaderEntry> {
        self.entries.iter_mut().find(|e| e.path == path)
    }

    pub fn templates(&self) -> impl Iterator<Item = &ShaderEntry> {
        self.entries.iter().filter(|e| e.template)
    }

    /// Reads the source of a shader: the file on disk, else a built-in template copy.
    pub fn read_source(path: &str) -> Option<String> {
        if let Ok(text) = std::fs::read_to_string(disk_path(path)) {
            return Some(text.replace("\r\n", "\n"));
        }
        let stem = file_stem(path);
        path.starts_with(TEMPLATE_DIR)
            .then(|| BUILTIN_TEMPLATES.iter().find(|(name, _)| *name == stem).map(|(_, src)| src.replace("\r\n", "\n")))
            .flatten()
    }

    /// Ensures an entry exists for `path`, loading and compiling it the first time.
    pub fn ensure(&mut self, shaders: &mut Assets<Shader>, path: &str) -> Option<&ShaderEntry> {
        if self.get(path).is_none() {
            let source = Self::read_source(path)?;
            let template = path.starts_with(TEMPLATE_DIR);
            let label = format!("bevylite_shader_editor/{path}");
            let handle = shaders.add(Shader::from_wgsl(compose(ERROR_SHADER, true), label.clone()));
            let handle_no_uv = shaders.add(Shader::from_wgsl(compose(ERROR_SHADER, false), format!("{label}#no_uv")));
            self.entries.push(ShaderEntry {
                path: path.to_string(),
                template,
                applied_source: String::new(),
                meta: ShaderMeta::parse(&file_stem(path), &source),
                handle,
                handle_no_uv,
                errors: Vec::new(),
                modified: modified_time(&disk_path(path)),
                revision: 0,
            });
            let _ = self.apply_source(shaders, path, &source);
        }
        self.get(path)
    }

    /// Validates `source`; on success swaps it into both shader assets (hot reload).
    /// On failure the previous good shader stays active and the errors are stored.
    pub fn apply_source(&mut self, shaders: &mut Assets<Shader>, path: &str, source: &str) -> Result<(), Vec<ShaderDiagnostic>> {
        let entry = self.get_mut(path).ok_or_else(|| vec![ShaderDiagnostic { line: 0, column: 0, message: format!("Unknown shader {path}") }])?;
        match validate(source) {
            Ok(()) => {
                let label = format!("bevylite_shader_editor/{path}");
                let _ = shaders.insert(entry.handle.id(), Shader::from_wgsl(compose(source, true), label.clone()));
                let _ = shaders.insert(entry.handle_no_uv.id(), Shader::from_wgsl(compose(source, false), format!("{label}#no_uv")));
                entry.applied_source = source.to_string();
                entry.meta = ShaderMeta::parse(&file_stem(path), source);
                entry.errors.clear();
                entry.revision += 1;
                Ok(())
            }
            Err(errors) => {
                entry.errors = errors.clone();
                Err(errors)
            }
        }
    }

    /// Scans the shader folders, adding new files and hot-reloading files changed on disk.
    /// `skip` is a path with unsaved editor changes that must not be overwritten.
    pub fn rescan(&mut self, shaders: &mut Assets<Shader>, skip: Option<&str>) {
        let root = asset_root();
        let mut found: Vec<String> = Vec::new();
        for (dir, _) in [(SHADER_DIR, false), (TEMPLATE_DIR, true)] {
            for file in wgsl_files(&root.join(dir)) {
                if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                    found.push(format!("{dir}/{name}"));
                }
            }
        }
        for (name, _) in BUILTIN_TEMPLATES {
            let path = format!("{TEMPLATE_DIR}/{name}.wgsl");
            if !found.contains(&path) {
                found.push(path);
            }
        }
        for path in &found {
            if self.get(path).is_none() {
                self.ensure(shaders, path);
                continue;
            }
            let disk = modified_time(&disk_path(path));
            let changed = self.get(path).is_some_and(|e| disk.is_some() && e.modified != disk);
            if changed {
                if let Some(entry) = self.get_mut(path) {
                    entry.modified = disk;
                }
                if Some(path.as_str()) != skip {
                    if let Some(source) = Self::read_source(path) {
                        let _ = self.apply_source(shaders, path, &source);
                    }
                }
            }
        }
        // Deleted files disappear from the list; their GPU shaders stay alive for materials still using them.
        self.entries.retain(|e| found.contains(&e.path) || disk_path(&e.path).exists());
        self.entries.sort_by(|a, b| a.template.cmp(&b.template).then_with(|| a.path.cmp(&b.path)));
    }

    /// Writes a file and records its new modification time so the watcher does not reload it.
    pub fn write_file(&mut self, path: &str, source: &str) -> Result<(), String> {
        let disk = disk_path(path);
        if let Some(parent) = disk.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
        }
        std::fs::write(&disk, source).map_err(|e| format!("Cannot write {}: {e}", disk.display()))?;
        if let Some(entry) = self.get_mut(path) {
            entry.modified = modified_time(&disk);
        }
        Ok(())
    }

    /// A free `shaders/<name>.wgsl` path (appends `_2`, `_3`, ... on collisions).
    pub fn unique_path(&self, name: &str) -> String {
        let base = sanitize_name(name);
        let mut candidate = format!("{SHADER_DIR}/{base}.wgsl");
        let mut n = 2;
        while self.get(&candidate).is_some() || disk_path(&candidate).exists() {
            candidate = format!("{SHADER_DIR}/{base}_{n}.wgsl");
            n += 1;
        }
        candidate
    }

    /// Resolves a template by stem, name or path.
    pub fn template_source(&self, template: &str) -> Option<String> {
        let key = template.trim().to_ascii_lowercase();
        let stem = sanitize_name(&key);
        let by_name = self
            .templates()
            .find(|e| e.meta.name.to_ascii_lowercase() == key || file_stem(&e.path) == stem || e.path == key)
            .map(|e| e.path.clone());
        let path = by_name.unwrap_or_else(|| format!("{TEMPLATE_DIR}/{stem}.wgsl"));
        Self::read_source(&path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_validate() {
        for (name, source) in BUILTIN_TEMPLATES {
            assert_eq!(validate(source), Ok(()), "template {name} should validate");
            let meta = ShaderMeta::parse(name, source);
            assert!(meta.declared.iter().any(|d| *d), "template {name} declares params");
        }
    }

    #[test]
    fn errors_map_to_user_lines() {
        let source = "fn shade(in: ShaderInput) -> vec4<f32> {\n    let x = 1.0;\n    return vec4<f32>(x, undefined_name, 0.0, 1.0);\n}\n";
        let errors = validate(source).unwrap_err();
        assert_eq!(errors[0].line, 3, "{errors:?}");
        let missing = validate("fn other() {}\n").unwrap_err();
        assert_eq!(missing[0].line, 0);
        assert!(missing[0].message.contains("shade"), "{missing:?}");
        let typed = validate("fn shade(in: ShaderInput) -> vec4<f32> {\n    return 1.0;\n}\n").unwrap_err();
        assert_eq!(typed[0].line, 2, "{typed:?}");
        let binding = validate("@group(3) @binding(1) var<uniform> x: f32;\n").unwrap_err();
        assert_eq!(binding[0].line, 1);
    }

    #[test]
    fn meta_parses_params() {
        let meta = ShaderMeta::parse("x", "//! name: Test\n//! alpha: blend\n//! color: Tint = #FF0000\n//! p1: Speed = 2.5 (0..10)\n");
        assert_eq!(meta.name, "Test");
        assert_eq!(meta.alpha, ShaderAlpha::Blend);
        assert_eq!(meta.color_label, "Tint");
        assert_eq!(meta.params[1], ParamMeta { label: "Speed".into(), default: 2.5, min: 0.0, max: 10.0 });
        assert_eq!(meta.declared, [false, true, false, false]);
        assert_eq!(normalize_path("toon"), "shaders/templates/toon.wgsl");
        assert_eq!(normalize_path("my_new_one"), "shaders/my_new_one.wgsl");
        assert_eq!(normalize_path("assets\\shaders\\templates\\toon.wgsl"), "shaders/templates/toon.wgsl");
        assert_eq!(sanitize_name("My Cool Shader!"), "my_cool_shader");
    }
}

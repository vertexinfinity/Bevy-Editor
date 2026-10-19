//! Built-in and user physics material presets (`assets/physics/*.physmat.json`).
use super::components::{CombineRule, PhysicsMaterial};
use std::path::{Path, PathBuf};

/// Folder holding user presets, relative to the working directory (the project root).
pub const PRESET_DIR: &str = "assets/physics";
pub const PRESET_EXT: &str = ".physmat.json";

fn preset(name: &str, sf: f32, df: f32, restitution: f32, density: f32, fc: CombineRule, rc: CombineRule) -> PhysicsMaterial {
    PhysicsMaterial {
        preset: name.into(),
        static_friction: sf,
        dynamic_friction: df,
        restitution,
        density,
        friction_combine: fc,
        restitution_combine: rc,
    }
}

/// Built-in presets, in picker order.
pub fn builtin_presets() -> Vec<PhysicsMaterial> {
    use CombineRule::*;
    vec![
        PhysicsMaterial::default(),
        preset("Ice", 0.1, 0.03, 0.05, 917.0, Minimum, Average),
        preset("Rubber", 1.0, 0.8, 0.8, 1100.0, Average, Maximum),
        preset("Metal", 0.6, 0.4, 0.2, 7850.0, Average, Average),
        preset("Wood", 0.5, 0.4, 0.3, 600.0, Average, Average),
        preset("Concrete", 0.8, 0.65, 0.1, 2400.0, Average, Average),
        preset("Bouncy", 0.5, 0.4, 0.95, 900.0, Average, Maximum),
        preset("Sticky", 2.0, 1.8, 0.0, 1200.0, Maximum, Minimum),
    ]
}

pub fn is_builtin(name: &str) -> bool {
    builtin_presets().iter().any(|p| p.preset.eq_ignore_ascii_case(name))
}

fn slug(name: &str) -> String {
    let s: String = name.trim().chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '_' }).collect();
    if s.is_empty() { "preset".into() } else { s }
}

pub fn preset_path(name: &str) -> PathBuf {
    Path::new(PRESET_DIR).join(format!("{}{PRESET_EXT}", slug(name)))
}

/// User presets found on disk, sorted by name. Unreadable files are skipped with a warning.
pub fn custom_presets() -> Vec<(PhysicsMaterial, PathBuf)> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir(PRESET_DIR) else { return out };
    for entry in dir.flatten() {
        let path = entry.path();
        if !path.to_string_lossy().ends_with(PRESET_EXT) {
            continue;
        }
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<PhysicsMaterial>(&s).map_err(|e| e.to_string())) {
            Ok(mut material) => {
                if material.preset.trim().is_empty() {
                    material.preset = path.file_name().map(|n| n.to_string_lossy().trim_end_matches(PRESET_EXT).to_string()).unwrap_or_default();
                }
                out.push((material, path));
            }
            Err(error) => bevy::log::warn!("Skipping physics preset {}: {error}", path.display()),
        }
    }
    out.sort_by(|a, b| a.0.preset.to_lowercase().cmp(&b.0.preset.to_lowercase()));
    out
}

/// Every preset (built-ins first, then custom ones that do not shadow a built-in).
pub fn all_presets() -> Vec<PhysicsMaterial> {
    let mut all = builtin_presets();
    for (material, _) in custom_presets() {
        if !all.iter().any(|p| p.preset.eq_ignore_ascii_case(&material.preset)) {
            all.push(material);
        }
    }
    all
}

pub fn find_preset(name: &str) -> Option<PhysicsMaterial> {
    all_presets().into_iter().find(|p| p.preset.eq_ignore_ascii_case(name.trim()))
}

/// Writes `material` as a user preset called `name`. Built-in names are reserved.
pub fn save_preset(name: &str, material: &PhysicsMaterial) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Preset name is empty".into());
    }
    if is_builtin(name) {
        return Err(format!("'{name}' is a built-in preset; choose another name"));
    }
    std::fs::create_dir_all(PRESET_DIR).map_err(|e| format!("Cannot create {PRESET_DIR}: {e}"))?;
    let mut material = material.clone();
    material.preset = name.to_string();
    let path = preset_path(name);
    let json = serde_json::to_string_pretty(&material).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// Display label for a material: its preset name, with `*` when values were edited.
pub fn material_label(material: Option<&PhysicsMaterial>) -> String {
    let Some(material) = material else { return "Default".into() };
    match find_preset(&material.preset) {
        Some(p) if p.same_values(material) => p.preset,
        Some(p) => format!("{}*", p.preset),
        None if material.preset.trim().is_empty() => "Custom".into(),
        None => format!("{}*", material.preset),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_cover_requested_presets() {
        for name in ["Default", "Ice", "Rubber", "Metal", "Wood", "Concrete", "Bouncy", "Sticky"] {
            assert!(is_builtin(name), "{name}");
        }
        assert!(save_preset("Ice", &PhysicsMaterial::default()).is_err());
        assert_eq!(preset_path("My Mat!").file_name().unwrap().to_string_lossy(), "my_mat_.physmat.json");
    }
}

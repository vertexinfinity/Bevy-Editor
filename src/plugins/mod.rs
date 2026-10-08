//! Plugin & Crate Management System for Bevylite.
//!
//! Provides:
//! - Catalog of community & official Bevy plugins (Physics, Audio, Rendering, AI, UI, Scripting).
//! - Active project Cargo.toml crate inspection and injection.
//! - User-defined custom crate addition from crates.io or git.
//! - Build-time crate / feature toggle checkboxes.

use std::fs;
use std::path::Path;
use bevy::prelude::*;

/// Information about a Bevy plugin / crate available in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub category: PluginCategory,
    pub version: &'static str,
    pub features: &'static [&'static str],
    pub default_enabled: bool,
    pub supports_web: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginCategory {
    Core,
    Physics,
    Rendering,
    Audio,
    UI,
    Scripting,
    Custom,
}

impl PluginCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Core => "Engine & Core",
            Self::Physics => "Physics & Collisions",
            Self::Rendering => "Graphics & VFX",
            Self::Audio => "Audio & Sound",
            Self::UI => "UI & Widgets",
            Self::Scripting => "Scripting & Gameplay",
            Self::Custom => "Custom Crates",
        }
    }
}

/// Catalog of well-known and tested Bevy crates.
pub static PLUGIN_CATALOG: &[PluginEntry] = &[
    // Core Engine Crates
    PluginEntry {
        name: "bevy_pbr",
        description: "Physically-Based Rendering (PBR) pipeline with standard materials & shadows",
        category: PluginCategory::Core,
        version: "0.15",
        features: &["tonemapping_luts", "pbr_transmission_textures"],
        default_enabled: true,
        supports_web: true,
    },
    PluginEntry {
        name: "bevy_gltf",
        description: "3D scene and mesh loader for standard .gltf and binary .glb files",
        category: PluginCategory::Core,
        version: "0.15",
        features: &["png", "jpeg"],
        default_enabled: true,
        supports_web: true,
    },
    PluginEntry {
        name: "bevy_scene",
        description: "Dynamic ECS scene serialization and deserialization (.ron / .bsn)",
        category: PluginCategory::Core,
        version: "0.15",
        features: &["serialize"],
        default_enabled: true,
        supports_web: true,
    },
    PluginEntry {
        name: "bevy_audio",
        description: "Spatial audio listener and background sound playback",
        category: PluginCategory::Audio,
        version: "0.15",
        features: &["vorbis", "wav"],
        default_enabled: true,
        supports_web: true,
    },
    PluginEntry {
        name: "bevy_ui",
        description: "Built-in Flexbox UI layout and 2D node rendering engine",
        category: PluginCategory::UI,
        version: "0.15",
        features: &[],
        default_enabled: true,
        supports_web: true,
    },
    // Physics
    PluginEntry {
        name: "avian3d",
        description: "Modern ECS-driven 3D physics engine with rigidbodies, colliders, and joints",
        category: PluginCategory::Physics,
        version: "0.2",
        features: &["3d", "f32"],
        default_enabled: false,
        supports_web: true,
    },
    PluginEntry {
        name: "bevy_rapier3d",
        description: "Production-ready Rapier 3D physics integration with character controllers",
        category: PluginCategory::Physics,
        version: "0.28",
        features: &["simd-stable"],
        default_enabled: false,
        supports_web: true,
    },
    // Rendering & VFX
    PluginEntry {
        name: "bevy_hanabi",
        description: "GPU-accelerated particle system with customizable emission and turbulence",
        category: PluginCategory::Rendering,
        version: "0.14",
        features: &["3d"],
        default_enabled: false,
        supports_web: false,
    },
    PluginEntry {
        name: "bevy_atmosphere",
        description: "Physically-accurate procedural sky and atmospheric scattering",
        category: PluginCategory::Rendering,
        version: "0.10",
        features: &[],
        default_enabled: false,
        supports_web: true,
    },
    // Audio
    PluginEntry {
        name: "bevy_kira_audio",
        description: "Interactive audio library powered by Kira with sound tweens and audio channels",
        category: PluginCategory::Audio,
        version: "0.21",
        features: &["ogg", "mp3"],
        default_enabled: false,
        supports_web: true,
    },
    // UI
    PluginEntry {
        name: "bevy_egui",
        description: "Immediate mode GUI integration for debug panels and menus",
        category: PluginCategory::UI,
        version: "0.31",
        features: &["default_fonts"],
        default_enabled: false,
        supports_web: true,
    },
    // Scripting & Gameplay
    PluginEntry {
        name: "bevy_mod_scripting",
        description: "Multi-language scripting runtime supporting Lua (mlua) and Rhai",
        category: PluginCategory::Scripting,
        version: "0.9",
        features: &["lua54"],
        default_enabled: false,
        supports_web: true,
    },
];

/// State managing the Plugin Manager dialog and active crate selections.
#[derive(Resource, Default)]
pub struct PluginManagerState {
    pub is_open: bool,
    pub search_query: String,
    pub selected_category: Option<PluginCategory>,
    /// Crates currently enabled for the active project
    pub enabled_crates: Vec<String>,
    /// Custom user crates added via the UI
    pub custom_crates: Vec<String>,
    pub status_message: Option<(String, bool)>, // (message, is_success)
}

impl PluginManagerState {
    /// Loads enabled crates by inspecting the active project's Cargo.toml.
    pub fn refresh_from_project(&mut self, project_dir: &Path) {
        let manifest_path = project_dir.join("Cargo.toml");
        let Ok(content) = fs::read_to_string(&manifest_path) else { return };

        let mut enabled = Vec::new();

        // Scan [dependencies] block
        let mut in_deps = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_deps = trimmed == "[dependencies]";
                continue;
            }
            if in_deps && !trimmed.is_empty() && !trimmed.starts_with('#') {
                if let Some((crate_name, _)) = trimmed.split_once('=') {
                    enabled.push(crate_name.trim().to_string());
                }
            }
        }

        // Also check if bevy features are declared in dependencies
        for plugin in PLUGIN_CATALOG {
            if content.contains(&format!("\"{}\"", plugin.name)) || enabled.contains(&plugin.name.to_string()) {
                if !enabled.contains(&plugin.name.to_string()) {
                    enabled.push(plugin.name.to_string());
                }
            }
        }

        self.enabled_crates = enabled;
    }

    /// Adds a crate to the active project's Cargo.toml.
    pub fn add_crate_to_project(&mut self, project_dir: &Path, crate_name: &str, version: &str) -> Result<(), String> {
        let manifest_path = project_dir.join("Cargo.toml");
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read Cargo.toml: {e}"))?;

        if content.contains(&format!("{} =", crate_name)) || content.contains(&format!("{crate_name} =")) {
            return Ok(()); // Already present
        }

        let new_dep_line = format!("\n{} = \"{}\"", crate_name, version);

        let updated = if let Some(pos) = content.find("[dependencies]") {
            let insert_pos = pos + "[dependencies]".len();
            format!("{}{}{}", &content[..insert_pos], new_dep_line, &content[insert_pos..])
        } else {
            format!("{}\n\n[dependencies]{}\n", content.trim_end(), new_dep_line)
        };

        fs::write(&manifest_path, updated)
            .map_err(|e| format!("Failed to update Cargo.toml: {e}"))?;

        if !self.enabled_crates.contains(&crate_name.to_string()) {
            self.enabled_crates.push(crate_name.to_string());
        }

        self.status_message = Some((format!("Added '{}' ({}) to Cargo.toml", crate_name, version), true));
        Ok(())
    }

    /// Removes a crate from the active project's Cargo.toml.
    pub fn remove_crate_from_project(&mut self, project_dir: &Path, crate_name: &str) -> Result<(), String> {
        let manifest_path = project_dir.join("Cargo.toml");
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read Cargo.toml: {e}"))?;

        let mut lines: Vec<&str> = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(crate_name) && (trimmed.contains('=') || trimmed.contains('{')) {
                continue; // Skip this dependency line
            }
            lines.push(line);
        }

        fs::write(&manifest_path, lines.join("\n"))
            .map_err(|e| format!("Failed to update Cargo.toml: {e}"))?;

        self.enabled_crates.retain(|c| c != crate_name);
        self.status_message = Some((format!("Removed '{}' from Cargo.toml", crate_name), true));
        Ok(())
    }
}

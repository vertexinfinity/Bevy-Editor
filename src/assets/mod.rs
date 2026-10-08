//! Shared asset templates and creation helpers for scripts, scenes, materials, etc.
//!
//! Exposes creation functions used across the Asset Browser, Inspector, CLI, and MCP.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Supported asset templates that can be created in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetTemplateKind {
    LuaScript,
    RhaiScript,
    RustScript,
    SceneRon,
    MaterialRon,
    GameplayTagList,
}

impl AssetTemplateKind {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::LuaScript => "lua",
            Self::RhaiScript => "rhai",
            Self::RustScript => "rs",
            Self::SceneRon => "ron",
            Self::MaterialRon => "mat.ron",
            Self::GameplayTagList => "tags.ron",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::LuaScript => "Lua Script (.lua)",
            Self::RhaiScript => "Rhai Script (.rhai)",
            Self::RustScript => "Rust Module (.rs)",
            Self::SceneRon => "Scene (.ron)",
            Self::MaterialRon => "Material (.mat.ron)",
            Self::GameplayTagList => "Tags (.tags.ron)",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::LuaScript | Self::RhaiScript | Self::RustScript => "📜",
            Self::SceneRon => "🎬",
            Self::MaterialRon => "🎨",
            Self::GameplayTagList => "🏷️",
        }
    }

    pub fn default_content(&self, base_name: &str) -> String {
        match self {
            Self::LuaScript => format!(
r#"-- {base_name}.lua
-- Bevylite Actor Script

local actor = {{
    move_speed = 5.0,
    jump_force = 10.0,
    is_active = true,
}}

function actor:on_start()
    print("[{base_name}] Initialized")
end

function actor:on_update(dt)
    -- Insert per-frame logic here
end

return actor
"#),
            Self::RhaiScript => format!(
r#"// {base_name}.rhai
// Bevylite Embedded Logic Script

let move_speed = 5.0;
let is_active = true;

fn on_update(delta) {{
    // Logic goes here
}}
"#),
            Self::RustScript => format!(
r#"//! {base_name}.rs
//! Bevylite Game Logic Component

use bevy::prelude::*;

#[derive(Component, Reflect, Default, Clone, Debug)]
#[reflect(Component, Default)]
pub struct {base_name} {{
    pub speed: f32,
    pub active: bool,
}}
"#),
            Self::SceneRon => format!(
r#"(
  resources: {{}},
  entities: {{
    "1v0": (
      components: {{
        "bevy_ecs::name::Name": "{base_name}",
        "bevy_editor_lib::scene::save_load::SceneEntity": (),
        "bevy_transform::components::transform::Transform": (
          translation: (0.0, 0.0, 0.0),
          rotation: (0.0, 0.0, 0.0, 1.0),
          scale: (1.0, 1.0, 1.0),
        ),
      }},
    ),
  }},
)
"#),
            Self::MaterialRon => format!(
r#"(
  base_color: Rgba(red: 0.8, green: 0.8, blue: 0.8, alpha: 1.0),
  perceptual_roughness: 0.5,
  metallic: 0.0,
  reflectance: 0.5,
)
"#),
            Self::GameplayTagList => format!(
r#"[
  "Interactable",
  "Player",
  "Enemy",
  "Hazard",
  "Collectable",
]
"#),
        }
    }
}

/// Helper service for creating and opening assets across GUI, CLI, and MCP.
pub struct AssetCreator;

impl AssetCreator {
    /// Creates an asset file from a template under the target directory.
    pub fn create_asset(
        target_dir: &Path,
        raw_name: &str,
        kind: AssetTemplateKind,
    ) -> Result<PathBuf, String> {
        let clean_name = raw_name.trim();
        if clean_name.is_empty() {
            return Err("Asset name cannot be empty".to_string());
        }

        let file_stem = if let Some(stripped) = clean_name.strip_suffix(&format!(".{}", kind.extension())) {
            stripped
        } else {
            clean_name
        };

        let file_name = format!("{file_stem}.{}", kind.extension());
        let full_path = target_dir.join(&file_name);

        if full_path.exists() {
            return Err(format!("File already exists: {:?}", full_path));
        }

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let content = kind.default_content(file_stem);
        fs::write(&full_path, content).map_err(|e| e.to_string())?;

        Ok(full_path)
    }

    /// Opens the specified file using the system's default application (or VS Code).
    pub fn open_in_system_editor(path: &Path) -> Result<(), String> {
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };

        if !abs_path.exists() {
            return Err(format!("File not found: {:?}", abs_path));
        }

        #[cfg(target_os = "windows")]
        {
            // Try launching directly with shell association: 'explorer' or 'cmd /c start'
            let status = Command::new("cmd")
                .args(["/c", "start", "", abs_path.to_str().unwrap_or_default()])
                .spawn();

            match status {
                Ok(_) => Ok(()),
                Err(err) => Err(format!("Failed to open editor: {}", err)),
            }
        }

        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .arg(abs_path)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }

        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open")
                .arg(abs_path)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    }
}

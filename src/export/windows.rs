//! Windows standalone desktop exporter.

use std::fs;
use std::process::Command;
use bevy::log::info;
use crate::export::{ExportContext, ExportResult, Exporter};

#[derive(Default)]
pub struct WindowsExporter;

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

impl Exporter for WindowsExporter {
    fn id(&self) -> &'static str {
        "windows"
    }

    fn name(&self) -> &'static str {
        "Windows Desktop (.exe)"
    }

    fn description(&self) -> &'static str {
        "Exports a standalone Windows desktop executable with embedded scene assets"
    }

    fn export(&self, ctx: &ExportContext) -> Result<ExportResult, String> {
        let export_dir = ctx.output_dir.join("windows");
        fs::create_dir_all(&export_dir).map_err(|e| format!("Failed to create export directory: {e}"))?;
        let export_dir = export_dir.canonicalize()
            .map_err(|e| format!("Failed to resolve export directory: {e}"))?;

        let project_dir = ctx.project_dir.as_ref()
            .ok_or_else(|| "Select a Cargo project to build".to_string())?;
        let project_dir = project_dir.canonicalize()
            .map_err(|e| format!("Failed to resolve project directory: {e}"))?;
        let manifest_path = project_dir.join("Cargo.toml");
        if !manifest_path.is_file() {
            return Err(format!("No Cargo.toml found in {}", project_dir.display()));
        }

        // Ensure the game project has an isolated [workspace] table and compatible Bevy dependencies
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            let mut updated = content.clone();
            if !updated.contains("[workspace]") {
                info!("Isolating target game project from outer cargo workspaces by appending [workspace] table...");
                updated = format!("{}\n\n# Ensure root workspace isolation for standalone builds\n[workspace]\n", updated.trim_end());
            }
            if updated.contains("version = \"0.15\"") {
                info!("Migrating user project Cargo.toml from Bevy 0.15 to Bevy git main (0.20-dev)...");
                updated = updated.replace(
                    "version = \"0.15\", default-features = false, features = [\n    \"bevy_core_pipeline\",\n    \"bevy_pbr\",\n    \"bevy_render\",\n    \"bevy_winit\",\n    \"bevy_asset\",\n    \"bevy_gltf\",\n    \"bevy_scene\",\n    \"tonemapping_luts\",\n    \"png\",\n]",
                    "git = \"https://github.com/bevyengine/bevy\", branch = \"main\", default-features = false, features = [\"3d\", \"serialize\", \"png\", \"tonemapping_luts\"]",
                );
                // Also remove stale Cargo.lock if migrating from 0.15 to avoid uuid conflicts
                let stale_lock = project_dir.join("Cargo.lock");
                if stale_lock.is_file() {
                    let _ = fs::remove_file(stale_lock);
                }
            }
            if updated != content {
                let _ = fs::write(&manifest_path, updated);
            }
        }

        // Auto-migrate legacy Bevy API syntax in starter projects if present
        let main_src = project_dir.join("src").join("main.rs");
        if main_src.is_file() {
            if let Ok(src_content) = fs::read_to_string(&main_src) {
                if src_content.contains("shadows_enabled")
                    || src_content.contains("(1280.0, 720.0)")
                    || src_content.contains("(1280.0_f32, 720.0_f32)")
                    || src_content.contains("DynamicSceneRoot")
                {
                    info!("Auto-patching legacy template syntax in {} for Bevy engine compatibility...", main_src.display());
                    let patched = src_content
                        .replace("shadows_enabled", "shadow_maps_enabled")
                        .replace("(1280.0, 720.0)", "(1280, 720)")
                        .replace("(1280.0_f32, 720.0_f32)", "(1280, 720)")
                        .replace("DynamicSceneRoot", "bevy::world_serialization::DynamicWorldRoot");
                    let _ = fs::write(&main_src, patched);
                }
            }
        }

        // Ensure models from editor library are synchronized to user project
        let editor_models = std::path::Path::new("assets/models");
        let project_models = project_dir.join("assets").join("models");
        if editor_models.is_dir() {
            let _ = fs::create_dir_all(&project_models);
            if let Ok(entries) = fs::read_dir(editor_models) {
                for entry in entries.flatten() {
                    let dest = project_models.join(entry.file_name());
                    if !dest.exists() {
                        let _ = fs::copy(entry.path(), dest);
                    }
                }
            }
        }

        let assets_dir = export_dir.join("assets");
        fs::create_dir_all(&assets_dir).map_err(|e| format!("Failed to create assets directory: {e}"))?;

        // Recursively sync the project's assets directory if present
        let project_assets = project_dir.join("assets");
        if project_assets.is_dir() {
            let _ = copy_dir_all(&project_assets, &assets_dir);
        }

        if let Some(scene_path) = &ctx.scene_path {
            if scene_path.exists() {
                let file_name = scene_path.file_name().unwrap_or_default();
                let dest = assets_dir.join(file_name);
                fs::copy(scene_path, &dest).map_err(|e| format!("Failed to copy scene asset: {e}"))?;
            }
        }

        // Sanitize any copied scene files to ensure editor-only SceneEntity is stripped and fields are compatible
        for bsn_candidate in [
            assets_dir.join("scenes").join("default_scene.bsn"),
            assets_dir.join("default_scene.bsn"),
            project_dir.join("assets").join("scenes").join("default_scene.bsn"),
        ] {
            if bsn_candidate.is_file() {
                if let Ok(content) = fs::read_to_string(&bsn_candidate) {
                    let mut cleaned = content.clone();
                    if cleaned.contains("bevy_editor_lib::scene::save_load::SceneEntity") {
                        cleaned = cleaned
                            .replace("\"bevy_editor_lib::scene::save_load::SceneEntity\": (),", "")
                            .replace("\"bevy_editor_lib::scene::save_load::SceneEntity\": ()", "");
                    }
                    if cleaned.contains("shadows_enabled") {
                        cleaned = cleaned.replace("shadows_enabled", "shadow_maps_enabled");
                    }
                    if cleaned != content {
                        let _ = fs::write(&bsn_candidate, cleaned);
                    }
                }
            }
        }

        // Verify & resolve Cargo dependencies first
        info!("Resolving project dependencies for {}...", manifest_path.display());
        let check_res = Command::new("cargo")
            .current_dir(&project_dir)
            .args(["check", "--manifest-path"])
            .arg(&manifest_path)
            .output();
        if let Ok(check_out) = check_res {
            let stderr_str = String::from_utf8_lossy(&check_out.stderr);
            // Print cargo check output to terminal
            for line in stderr_str.lines() {
                info!("[cargo check] {line}");
            }
            if !check_out.status.success() {
                // Extract error lines (or last 6 lines) for clean UI status display
                let concise_err: String = stderr_str
                    .lines()
                    .filter(|l| l.contains("error[") || l.contains("error:") || l.starts_with("  -->"))
                    .take(4)
                    .collect::<Vec<_>>()
                    .join(" | ");
                let err_summary = if concise_err.is_empty() {
                    stderr_str.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
                } else {
                    concise_err
                };
                return Err(format!("Cargo check failed: {err_summary}"));
            }
        }

        let target_dir = export_dir.join("target");
        info!("[Export:Windows] Starting cargo build in {} (target-dir: {})...", project_dir.display(), target_dir.display());

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&project_dir)
            .args(["build", "--manifest-path"])
            .arg(&manifest_path)
            .arg("--target-dir")
            .arg(&target_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if ctx.release {
            cmd.arg("--release");
        }

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn cargo build: {e}"))?;

        // Read and stream stderr (cargo logs progress and errors to stderr)
        let stderr = child.stderr.take();
        let stdout = child.stdout.take();

        let stderr_handle = std::thread::spawn(move || {
            let mut captured = String::new();
            if let Some(pipe) = stderr {
                use std::io::{BufRead, BufReader};
                let reader = BufReader::new(pipe);
                for line in reader.lines().flatten() {
                    info!("[cargo build] {line}");
                    captured.push_str(&line);
                    captured.push('\n');
                }
            }
            captured
        });

        let stdout_handle = std::thread::spawn(move || {
            if let Some(pipe) = stdout {
                use std::io::{BufRead, BufReader};
                let reader = BufReader::new(pipe);
                for line in reader.lines().flatten() {
                    info!("[cargo build stdout] {line}");
                }
            }
        });

        let status = child.wait().map_err(|e| format!("Failed to wait for cargo build: {e}"))?;
        let _ = stdout_handle.join();
        let captured_err = stderr_handle.join().unwrap_or_default();

        if !status.success() {
            let last_lines: String = captured_err
                .lines()
                .rev()
                .take(15)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            return Err(format!("Windows build failed (exit code {:?}):\n{}", status.code(), last_lines));
        }

        let bin_folder = target_dir.join(if ctx.release { "release" } else { "debug" });
        if bin_folder.is_dir() {
            let _ = copy_dir_all(&assets_dir, &bin_folder.join("assets"));
        }

        info!("[Export:Windows] Build completed successfully.");
        Ok(ExportResult {
            primary_artifact: bin_folder.clone(),
            message: format!("Windows project build completed in {}", bin_folder.display()),
        })
    }
}


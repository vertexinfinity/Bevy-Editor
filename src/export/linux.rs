//! Linux standalone desktop exporter.

use std::fs;
use std::process::Command;
use bevy::log::info;
use crate::export::{ExportContext, ExportResult, Exporter};

#[derive(Default)]
pub struct LinuxExporter;

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

impl Exporter for LinuxExporter {
    fn id(&self) -> &'static str {
        "linux"
    }

    fn name(&self) -> &'static str {
        "Linux Desktop (ELF)"
    }

    fn description(&self) -> &'static str {
        "Exports a standalone Linux desktop executable with embedded scene assets"
    }

    fn export(&self, ctx: &ExportContext) -> Result<ExportResult, String> {
        let export_dir = ctx.output_dir.join("linux");
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

        // Ensure root workspace isolation
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            if !content.contains("[workspace]") {
                info!("Isolating target game project from outer cargo workspaces by appending [workspace] table...");
                let updated = format!("{}\n\n# Ensure root workspace isolation for standalone builds\n[workspace]\n", content.trim_end());
                let _ = fs::write(&manifest_path, updated);
            }
        }

        // Auto-patch legacy template syntax
        let main_src = project_dir.join("src").join("main.rs");
        if main_src.is_file() {
            if let Ok(src_content) = fs::read_to_string(&main_src) {
                if src_content.contains("shadow_maps_enabled") || src_content.contains("(1280.0, 720.0)") {
                    info!("Auto-patching legacy template syntax in {} for Bevy 0.15 compatibility...", main_src.display());
                    let patched = src_content
                        .replace("shadow_maps_enabled", "shadows_enabled")
                        .replace("(1280.0, 720.0)", "(1280.0_f32, 720.0_f32)");
                    let _ = fs::write(&main_src, patched);
                }
            }
        }

        let assets_dir = export_dir.join("assets");
        fs::create_dir_all(&assets_dir).map_err(|e| format!("Failed to create assets directory: {e}"))?;

        // Recursively sync project assets
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

        let target_dir = export_dir.join("target");
        info!("[Export:Linux] Starting cargo build in {} (target-dir: {})...", project_dir.display(), target_dir.display());

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&project_dir)
            .args(["build", "--manifest-path"])
            .arg(&manifest_path)
            .arg("--target-dir")
            .arg(&target_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // When building from a non-Linux host (e.g. Windows), check if x86_64-unknown-linux-gnu target is available
        #[cfg(not(target_os = "linux"))]
        {
            let target_check = Command::new("rustup")
                .args(["target", "list", "--installed"])
                .output();
            if let Ok(out) = target_check {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.contains("x86_64-unknown-linux-gnu") {
                    cmd.args(["--target", "x86_64-unknown-linux-gnu"]);
                } else if stdout.contains("x86_64-unknown-linux-musl") {
                    cmd.args(["--target", "x86_64-unknown-linux-musl"]);
                }
            }
        }

        if ctx.release {
            cmd.arg("--release");
        }

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn cargo build for Linux: {e}"))?;

        let stderr = child.stderr.take();
        let stdout = child.stdout.take();

        let stderr_handle = std::thread::spawn(move || {
            let mut captured = String::new();
            if let Some(pipe) = stderr {
                use std::io::{BufRead, BufReader};
                let reader = BufReader::new(pipe);
                for line in reader.lines().flatten() {
                    info!("[cargo build linux] {line}");
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
                    info!("[cargo build linux stdout] {line}");
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
            return Err(format!("Linux build failed (exit code {:?}):\n{}", status.code(), last_lines));
        }

        // Determine destination bin folder
        let config_folder = if ctx.release { "release" } else { "debug" };
        let bin_folder = if target_dir.join("x86_64-unknown-linux-gnu").join(config_folder).is_dir() {
            target_dir.join("x86_64-unknown-linux-gnu").join(config_folder)
        } else if target_dir.join("x86_64-unknown-linux-musl").join(config_folder).is_dir() {
            target_dir.join("x86_64-unknown-linux-musl").join(config_folder)
        } else {
            target_dir.join(config_folder)
        };

        if bin_folder.is_dir() {
            let _ = copy_dir_all(&assets_dir, &bin_folder.join("assets"));
        }

        info!("[Export:Linux] Build completed successfully.");
        Ok(ExportResult {
            primary_artifact: bin_folder.clone(),
            message: format!("Linux project build completed in {}", bin_folder.display()),
        })
    }
}

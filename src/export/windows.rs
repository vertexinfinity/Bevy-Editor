//! Windows standalone desktop exporter.

use std::fs;
use std::process::Command;
use bevy::log::info;
use crate::export::{ExportContext, ExportResult, Exporter};

#[derive(Default)]
pub struct WindowsExporter;

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

        let assets_dir = export_dir.join("assets");
        fs::create_dir_all(&assets_dir).map_err(|e| format!("Failed to create assets directory: {e}"))?;

        if let Some(scene_path) = &ctx.scene_path {
            if scene_path.exists() {
                let file_name = scene_path.file_name().unwrap_or_default();
                let dest = assets_dir.join(file_name);
                fs::copy(scene_path, &dest).map_err(|e| format!("Failed to copy scene asset: {e}"))?;
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
            if !check_out.status.success() {
                let err_msg = String::from_utf8_lossy(&check_out.stderr);
                return Err(format!("Dependency resolution or cargo check failed: {err_msg}"));
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

        info!("[Export:Windows] Build completed successfully.");
        Ok(ExportResult {
            primary_artifact: target_dir.join(if ctx.release { "release" } else { "debug" }),
            message: format!("Windows project build completed in {}", target_dir.display()),
        })
    }
}

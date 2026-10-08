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
        let mut cmd = Command::new("cargo");
        cmd.current_dir(&project_dir)
            .args(["build", "--manifest-path"])
            .arg(&manifest_path)
            .arg("--target-dir")
            .arg(&target_dir);
        if ctx.release {
            cmd.arg("--release");
        }

        let output = cmd.output().map_err(|e| format!("Failed to run cargo build: {e}"))?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Windows build failed: {err_msg}"));
        }

        Ok(ExportResult {
            primary_artifact: target_dir.join(if ctx.release { "release" } else { "debug" }),
            message: format!("Windows project build completed in {}", target_dir.display()),
        })
    }
}

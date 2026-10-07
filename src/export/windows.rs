//! Windows standalone desktop exporter.

use std::fs;
use std::process::Command;
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

        let assets_dir = export_dir.join("assets");
        fs::create_dir_all(&assets_dir).map_err(|e| format!("Failed to create assets directory: {e}"))?;

        if let Some(scene_path) = &ctx.scene_path {
            if scene_path.exists() {
                let file_name = scene_path.file_name().unwrap_or_default();
                let dest = assets_dir.join(file_name);
                fs::copy(scene_path, &dest).map_err(|e| format!("Failed to copy scene asset: {e}"))?;
            }
        }

        let mut cmd = Command::new("cargo");
        cmd.args(["build"]);
        if ctx.release {
            cmd.arg("--release");
        }

        let output = cmd.output().map_err(|e| format!("Failed to run cargo build: {e}"))?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Windows build failed: {err_msg}"));
        }

        Ok(ExportResult {
            primary_artifact: export_dir.clone(),
            message: format!("Windows executable package built at {:?}", export_dir),
        })
    }
}

use std::fs;
use std::process::Command;
use bevy::log::info;
use crate::export::{ExportContext, ExportResult, Exporter};

#[derive(Default)]
pub struct AndroidExporter;

impl Exporter for AndroidExporter {
    fn id(&self) -> &'static str {
        "android"
    }

    fn name(&self) -> &'static str {
        "Android (APK / Gradle Project)"
    }

    fn description(&self) -> &'static str {
        "Exports a standalone Android APK using cargo-apk or creates an Android Studio Gradle project"
    }

    fn export(&self, ctx: &ExportContext) -> Result<ExportResult, String> {
        let export_dir = ctx.output_dir.join("android");
        fs::create_dir_all(&export_dir).map_err(|e| format!("Failed to create export directory: {e}"))?;
        let project_dir = ctx.project_dir.as_ref()
            .ok_or_else(|| "Select a Cargo project to build".to_string())?
            .canonicalize()
            .map_err(|e| format!("Failed to resolve project directory: {e}"))?;
        let manifest_path = project_dir.join("Cargo.toml");
        if !manifest_path.is_file() {
            return Err(format!("No Cargo.toml found in {}", project_dir.display()));
        }

        // 1. Prepare assets directory inside the export bundle
        let assets_dir = export_dir.join("assets");
        fs::create_dir_all(&assets_dir).map_err(|e| format!("Failed to create assets directory: {e}"))?;

        // 2. If a scene path is provided, copy the scene into assets
        if let Some(scene_path) = &ctx.scene_path {
            if scene_path.exists() {
                let file_name = scene_path.file_name().unwrap_or_default();
                let dest = assets_dir.join(file_name);
                fs::copy(scene_path, &dest).map_err(|e| format!("Failed to copy scene asset: {e}"))?;
            }
        }

        // 3. Check dependencies & toolchain for Android
        let has_cargo_apk = Command::new("cargo")
            .args(["apk", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if has_cargo_apk {
            // Verify or install rust target aarch64-linux-android
            let target_check = Command::new("rustup")
                .args(["target", "list", "--installed"])
                .output();
            if let Ok(out) = target_check {
                let targets = String::from_utf8_lossy(&out.stdout);
                if !targets.contains("aarch64-linux-android") {
                    info!("Resolving missing Android Rust target: adding aarch64-linux-android via rustup...");
                    let _ = Command::new("rustup")
                        .args(["target", "add", "aarch64-linux-android"])
                        .status();
                }
            }

            // Run cargo-apk build
            let mut cmd = Command::new("cargo");
            cmd.current_dir(&project_dir)
                .env("CARGO_TARGET_DIR", export_dir.join("target"))
                .args(["apk", "build", "--target", "aarch64-linux-android"]);
            if ctx.release {
                cmd.arg("--release");
            }

            let output = cmd.output().map_err(|e| format!("Failed to execute cargo-apk: {e}"))?;
            if !output.status.success() {
                let err_msg = String::from_utf8_lossy(&output.stderr);
                return Err(format!("cargo-apk build error: {err_msg}"));
            }

            Ok(ExportResult {
                primary_artifact: export_dir.clone(),
                message: format!("Android APK successfully built for target aarch64-linux-android in {:?}", export_dir.join("target")),
            })
        } else {
            // Scaffold the Android Gradle project based on Bevy mobile template
            let manifest_content = format!(
                r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="org.bevyengine.{}">
    <application
        android:hasCode="true"
        android:label="{}">
        <activity
            android:name="android.app.NativeActivity"
            android:configChanges="orientation|keyboardHidden|screenSize|screenLayout|density"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
            <meta-data
                android:name="android.app.lib_name"
                android:value="{}" />
        </activity>
    </application>
</manifest>"#,
                ctx.project_name.to_lowercase().replace(' ', "_"),
                ctx.project_name,
                ctx.project_name.to_lowercase().replace(' ', "_")
            );

            let manifest_path = export_dir.join("AndroidManifest.xml");
            fs::write(&manifest_path, manifest_content)
                .map_err(|e| format!("Failed to write AndroidManifest: {e}"))?;

            Ok(ExportResult {
                primary_artifact: export_dir.clone(),
                message: format!(
                    "Android Gradle template generated at {:?}. (Install 'cargo-apk' to directly compile APKs)",
                    export_dir
                ),
                zip_artifact: None,
            })
        }
    }
}

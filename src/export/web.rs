//! WebAssembly (WASM) & HTML5 Exporter for Bevy.
//!
//! Generates a standalone web distribution bundle including:
//! - WebAssembly binary (`.wasm`) compiled via `cargo build --target wasm32-unknown-unknown`
//! - JS bootstrap bindings (`index.html` & wasm loader)
//! - Assets folder synchronization for web server deployment

use std::fs;
use std::process::Command;
use bevy::log::info;
use crate::export::{ExportContext, ExportResult, Exporter};

#[derive(Default)]
pub struct WebExporter;

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

impl Exporter for WebExporter {
    fn id(&self) -> &'static str {
        "web"
    }

    fn name(&self) -> &'static str {
        "Web (WebAssembly / HTML5)"
    }

    fn description(&self) -> &'static str {
        "Exports a WebAssembly bundle suitable for modern web browsers and itch.io"
    }

    fn export(&self, ctx: &ExportContext) -> Result<ExportResult, String> {
        let export_dir = ctx.output_dir.join("web");
        fs::create_dir_all(&export_dir).map_err(|e| format!("Failed to create web export directory: {e}"))?;

        let project_dir = ctx.project_dir.as_ref()
            .ok_or_else(|| "Select a Cargo project to build".to_string())?;
        let project_dir = project_dir.canonicalize()
            .map_err(|e| format!("Failed to resolve project directory: {e}"))?;
        let manifest_path = project_dir.join("Cargo.toml");
        if !manifest_path.is_file() {
            return Err(format!("No Cargo.toml found in {}", project_dir.display()));
        }

        // 1. Prepare assets directory inside the export bundle
        let assets_dir = export_dir.join("assets");
        fs::create_dir_all(&assets_dir).map_err(|e| format!("Failed to create assets directory: {e}"))?;

        let project_assets = project_dir.join("assets");
        if project_assets.is_dir() {
            let _ = copy_dir_all(&project_assets, &assets_dir);
        }

        if let Some(scene_path) = &ctx.scene_path {
            if scene_path.exists() {
                let file_name = scene_path.file_name().unwrap_or_default();
                let dest = assets_dir.join(file_name);
                let _ = fs::copy(scene_path, &dest);
            }
        }

        let pkg_name = ctx.project_name.trim().replace(' ', "_").to_lowercase();

        // 2. HTML5 Web Shell Canvas with responsive styling and loader
        let index_html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no">
    <title>{}</title>
    <style>
        body, html {{
            margin: 0;
            padding: 0;
            width: 100%;
            height: 100%;
            overflow: hidden;
            background-color: #0d0f17;
            display: flex;
            align-items: center;
            justify-content: center;
            font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            color: #ececec;
        }}
        #loading {{
            position: absolute;
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 12px;
            font-size: 14px;
            letter-spacing: 0.5px;
        }}
        .spinner {{
            width: 36px;
            height: 36px;
            border: 3px solid rgba(0, 242, 254, 0.2);
            border-top-color: #00f2fe;
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }}
        @keyframes spin {{
            to {{ transform: rotate(360deg); }}
        }}
        canvas {{
            width: 100%;
            height: 100%;
            display: block;
            outline: none;
        }}
    </style>
</head>
<body>
    <div id="loading">
        <div class="spinner"></div>
        <div>Loading {}...</div>
    </div>
    <canvas id="bevy-canvas"></canvas>
    <script type="module">
        // Standard wasm-bindgen loader
        async function run() {{
            try {{
                const {{ default: init }} = await import('./{}.js');
                await init('./{}_bg.wasm');
                const loading = document.getElementById('loading');
                if (loading) loading.style.display = 'none';
            }} catch (err) {{
                console.error("WASM Load Error:", err);
                const loading = document.getElementById('loading');
                if (loading) {{
                    loading.innerHTML = '<span style="color: #ff5555">Failed to load WebAssembly bundle:</span><br/>' + err;
                }}
            }}
        }}
        run();
    </script>
</body>
</html>"#,
            ctx.project_name,
            ctx.project_name,
            pkg_name,
            pkg_name
        );

        fs::write(export_dir.join("index.html"), index_html)
            .map_err(|e| format!("Failed to create index.html: {e}"))?;

        // 3. Ensure wasm32-unknown-unknown target is installed via rustup
        info!("Verifying wasm32-unknown-unknown Rust target...");
        let target_check = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output();

        let mut has_wasm_target = false;
        if let Ok(out) = target_check {
            let targets = String::from_utf8_lossy(&out.stdout);
            if targets.contains("wasm32-unknown-unknown") {
                has_wasm_target = true;
            }
        }

        if !has_wasm_target {
            info!("Adding wasm32-unknown-unknown target via rustup...");
            let install_res = Command::new("rustup")
                .args(["target", "add", "wasm32-unknown-unknown"])
                .status();
            if let Ok(status) = install_res {
                if status.success() {
                    has_wasm_target = true;
                }
            }
        }

        if !has_wasm_target {
            return Ok(ExportResult {
                primary_artifact: export_dir.clone(),
                message: format!(
                    "Web shell scaffolded at {}. Run `rustup target add wasm32-unknown-unknown` to enable direct wasm compilation.",
                    export_dir.display()
                ),
                zip_artifact: None,
            });
        }

        // 4. Compile with Cargo for target wasm32-unknown-unknown
        let target_dir = export_dir.join("target");
        info!("[Export:Web] Compiling game for wasm32-unknown-unknown (target-dir: {})...", target_dir.display());

        let mut cmd = Command::new("cargo");
        cmd.current_dir(&project_dir)
            .args(["build", "--target", "wasm32-unknown-unknown", "--manifest-path"])
            .arg(&manifest_path)
            .arg("--target-dir")
            .arg(&target_dir);

        if ctx.release {
            cmd.arg("--release");
        }

        let build_output = cmd.output().map_err(|e| format!("Failed to spawn cargo build for web: {e}"))?;
        if !build_output.status.success() {
            let stderr_str = String::from_utf8_lossy(&build_output.stderr);
            let last_lines: String = stderr_str.lines().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
            return Err(format!("WebAssembly compilation failed:\n{last_lines}"));
        }

        // 5. Locate the compiled .wasm file in target-dir
        let profile_dir = if ctx.release { "release" } else { "debug" };
        let wasm_folder = target_dir.join("wasm32-unknown-unknown").join(profile_dir);
        let mut raw_wasm = wasm_folder.join(format!("{}.wasm", pkg_name));

        if !raw_wasm.is_file() {
            // Scan for any .wasm artifact in wasm_folder
            if let Ok(entries) = fs::read_dir(&wasm_folder) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("wasm") {
                        raw_wasm = path;
                        break;
                    }
                }
            }
        }

        let has_wasm_bindgen = Command::new("wasm-bindgen")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if has_wasm_bindgen && raw_wasm.is_file() {
            info!("Running wasm-bindgen for web bundle generation...");
            let bindgen_res = Command::new("wasm-bindgen")
                .args(["--out-dir"])
                .arg(&export_dir)
                .args(["--target", "web", "--no-typescript"])
                .arg(&raw_wasm)
                .output();

            if let Ok(bg_out) = bindgen_res {
                if bg_out.status.success() {
                    return Ok(ExportResult {
                        primary_artifact: export_dir.join("index.html"),
                        message: format!("WebAssembly distribution bundle ready in {}", export_dir.display()),
                        zip_artifact: None,
                    });
                }
            }
        }

        // Fallback: copy compiled wasm directly
        if raw_wasm.is_file() {
            let dest_wasm = export_dir.join(format!("{}_bg.wasm", pkg_name));
            let _ = fs::copy(&raw_wasm, &dest_wasm);
            let raw_dest = export_dir.join(format!("{}.wasm", pkg_name));
            let _ = fs::copy(&raw_wasm, &raw_dest);
        }

        Ok(ExportResult {
            primary_artifact: export_dir.join("index.html"),
            message: format!("WebAssembly export ready in {}. (Serve with `python -m http.server` or upload to itch.io)", export_dir.display()),
            zip_artifact: None,
        })
    }
}

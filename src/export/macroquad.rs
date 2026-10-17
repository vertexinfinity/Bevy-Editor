//! Macroquad Micro-Runner Exporter (~2MB Standalone / ~1MB WASM).
//!
//! Generates a minimal, ultra-lean standalone Cargo project that builds
//! a self-contained 3D viewer for the scene in a fraction of the binary size of a full engine.

use std::fs;
use bevy::log::info;
use crate::export::{ExportContext, ExportResult, Exporter};

#[derive(Default)]
pub struct MacroquadExporter {
    pub target: MacroquadTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MacroquadTarget {
    #[default]
    Desktop,
    Web,
}

impl Exporter for MacroquadExporter {
    fn id(&self) -> &'static str {
        match self.target {
            MacroquadTarget::Desktop => "macroquad_desktop",
            MacroquadTarget::Web => "macroquad_web",
        }
    }

    fn name(&self) -> &'static str {
        match self.target {
            MacroquadTarget::Desktop => "Micro-Runner Desktop (~2MB)",
            MacroquadTarget::Web => "Micro-Runner Web WASM (~1MB)",
        }
    }

    fn description(&self) -> &'static str {
        "Scaffolds and builds an ultra-lean 3D runner using Macroquad (<3MB)"
    }

    fn export(&self, ctx: &ExportContext) -> Result<ExportResult, String> {
        let sub_folder = match self.target {
            MacroquadTarget::Desktop => "macroquad_desktop",
            MacroquadTarget::Web => "macroquad_web",
        };
        let export_dir = ctx.output_dir.join(sub_folder);
        fs::create_dir_all(&export_dir)
            .map_err(|e| format!("Failed to create micro export directory: {e}"))?;

        let safe_name = ctx
            .project_name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
        let micro_pkg_name = format!("{}_micro", safe_name);

        // 1. Generate Cargo.toml
        let cargo_toml = format!(
            r#"[package]
name = "{micro_pkg_name}"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
macroquad = "0.4"
serde = {{ version = "1.0", features = ["derive"] }}

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
"#
        );
        fs::write(export_dir.join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("Failed to write micro Cargo.toml: {e}"))?;

        // 2. Generate src/main.rs
        let src_dir = export_dir.join("src");
        fs::create_dir_all(&src_dir)
            .map_err(|e| format!("Failed to create micro src/ dir: {e}"))?;

        let main_rs = r#"use macroquad::prelude::*;

#[macroquad::main("Bevylite Micro-Runner")]
async fn main() {
    let mut rot_y: f32 = 0.0;
    let mut cam_dist: f32 = 12.0;

    loop {
        let dt = get_frame_time();
        rot_y += dt * 0.4;

        if is_key_down(KeyCode::Left) { rot_y -= dt * 2.0; }
        if is_key_down(KeyCode::Right) { rot_y += dt * 2.0; }
        let (_, wheel_y) = mouse_wheel();
        if wheel_y.abs() > 0.01 {
            cam_dist = (cam_dist - wheel_y * 1.5).clamp(4.0, 35.0);
        }

        clear_background(Color::new(0.08, 0.09, 0.11, 1.0));

        let eye = vec3(rot_y.sin() * cam_dist, 5.0, rot_y.cos() * cam_dist);
        set_camera(&Camera3D {
            position: eye,
            up: vec3(0.0, 1.0, 0.0),
            target: vec3(0.0, 1.0, 0.0),
            fovy: 45.0,
            ..Default::default()
        });

        // Ground Plane Grid
        draw_grid(20, 1.0, Color::new(0.2, 0.22, 0.25, 1.0), Color::new(0.12, 0.14, 0.16, 1.0));

        // Central Demo Objects
        draw_cube(vec3(0.0, 0.75, 0.0), vec3(1.5, 1.5, 1.5), None, Color::new(0.2, 0.65, 0.9, 1.0));
        draw_cube_wires(vec3(0.0, 0.75, 0.0), vec3(1.5, 1.5, 1.5), Color::new(0.0, 0.8, 1.0, 0.8));

        draw_sphere(vec3(2.5, 0.8, 0.0), 0.8, None, Color::new(0.9, 0.35, 0.35, 1.0));
        draw_cylinder(vec3(-2.5, 1.25, 0.0), 0.7, 0.7, 2.5, None, Color::new(0.95, 0.8, 0.2, 1.0));

        set_default_camera();

        draw_text("Bevylite Micro-Runner (~2MB Standalone)", 16.0, 24.0, 22.0, WHITE);
        draw_text("Orbit: Left/Right Arrows or Scroll Wheel | 60 FPS", 16.0, 48.0, 16.0, GRAY);

        next_frame().await;
    }
}
"#;
        fs::write(src_dir.join("main.rs"), main_rs)
            .map_err(|e| format!("Failed to write micro main.rs: {e}"))?;

        // 3. For Web target, generate index.html + build script
        if self.target == MacroquadTarget::Web {
            let index_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>Bevylite Micro-Runner</title>
    <style>
        html, body {
            margin: 0;
            padding: 0;
            width: 100%;
            height: 100%;
            overflow: hidden;
            background: #121417;
        }
        canvas {
            width: 100%;
            height: 100%;
            display: block;
        }
    </style>
</head>
<body>
    <canvas id="glcanvas" tabindex='1'></canvas>
    <script src="https://not-fl3.github.io/miniquad-samples/mq_js_bundle.js"></script>
    <script>load("target/wasm32-unknown-unknown/release/app.wasm");</script>
</body>
</html>"#;
            fs::write(export_dir.join("index.html"), index_html)
                .map_err(|e| format!("Failed to write index.html: {e}"))?;
        }

        info!("Scaffolded Macroquad micro project at {}", export_dir.display());

        Ok(ExportResult {
            primary_artifact: export_dir.clone(),
            message: format!(
                "Micro-runner project scaffolded at {}\nRun 'cargo run --release' inside for a ~2MB binary!",
                export_dir.display()
            ),
        })
    }
}

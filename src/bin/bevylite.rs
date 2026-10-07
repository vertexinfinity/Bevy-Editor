//! Bevylite CLI: Command-Line Interface and MCP Server for the Bevy Game Engine Editor.
//!
//! Provides automated headless project management, multi-platform compilation,
//! scene inspection, and a Model Context Protocol (MCP) server for AI assistants.

use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use bevy_editor_lib::export::{ExportContext, Exporter};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "new" => handle_new(&args[2..]),
        "open" => handle_open(&args[2..]),
        "build" | "export" => handle_build(&args[2..]),
        "scene" => handle_scene(&args[2..]),
        "tools" => handle_tools(),
        "mcp" => handle_mcp(),
        "--version" | "-v" | "version" => {
            println!("bevylite v0.1.0 (Bevy Engine 0.20-dev)");
        }
        "--help" | "-h" | "help" => print_help(),
        other => {
            eprintln!("Unknown command: '{}'. Run 'bevylite --help' for usage.", other);
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!(r#"
◈ BEVYLITE CLI — Modern, Lightweight Game Engine Tooling

USAGE:
    bevylite <COMMAND> [OPTIONS]

COMMANDS:
    new <PROJECT_NAME>        Scaffold a clean, standalone Bevy game project
    open [PATH]               Launch the Bevylite visual editor (opens path if specified)
    build --target <TARGET>   Headlessly build and export for a target platform
                              [targets: windows, android, web, ios]
    scene inspect <FILE>      Headlessly inspect a .ron / .bsn scene file
    tools                     List all registered viewport editing tools
    mcp                       Start the Bevylite Model Context Protocol (MCP) server (stdio)
    version, -v               Print version information
    help, -h                  Print this help menu

EXAMPLES:
    bevylite new MyFirstGame
    bevylite open
    bevylite build --target android --output ./dist/android
    bevylite scene inspect ./assets/scenes/starter.ron
    bevylite mcp
"#);
}

fn handle_new(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: Missing project name. Usage: bevylite new <PROJECT_NAME> [--output <DIR>]");
        std::process::exit(1);
    }

    let project_name = &args[0];
    let target_dir = if args.len() >= 3 && args[1] == "--output" {
        PathBuf::from(&args[2])
    } else {
        env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(project_name)
    };

    println!("Scaffolding Bevy game project '{}' into {:?}...", project_name, target_dir);
    match bevy_editor_lib::project::ProjectScaffolder::create_project(&target_dir, project_name) {
        Ok(path) => {
            println!("Successfully created project at {:?}", path);
            println!("\nTo get started:");
            println!("  cd {:?}", path);
            println!("  cargo run");
        }
        Err(err) => {
            eprintln!("Failed to create project: {}", err);
            std::process::exit(1);
        }
    }
}

fn handle_open(args: &[String]) {
    if !args.is_empty() {
        println!("Opening Bevylite Editor with path: {}", args[0]);
    } else {
        println!("Launching Bevylite Editor...");
    }
    bevy_editor_lib::run_editor();
}

fn handle_build(args: &[String]) {
    let mut target = "windows".to_string();
    let mut output_dir = PathBuf::from("./dist");
    let mut project_dir = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--target" | "-t" => {
                if i + 1 < args.len() {
                    target = args[i + 1].to_lowercase();
                    i += 1;
                }
            }
            "--output" | "-o" => {
                if i + 1 < args.len() {
                    output_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--project" | "-p" => {
                if i + 1 < args.len() {
                    project_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!("Building export for target '{}' into {:?}...", target, output_dir);

    let config = ExportContext {
        project_name: "BevyGame".to_string(),
        project_dir,
        scene_path: None,
        output_dir: output_dir.clone(),
        release: true,
    };

    let result = match target.as_str() {
        "android" => {
            let exporter = bevy_editor_lib::export::android::AndroidExporter;
            exporter.export(&config)
        }
        "windows" => {
            let exporter = bevy_editor_lib::export::windows::WindowsExporter;
            exporter.export(&config)
        }
        "web" => {
            let exporter = bevy_editor_lib::export::web::WebExporter;
            exporter.export(&config)
        }
        "ios" => {
            let exporter = bevy_editor_lib::export::ios::IosExporter;
            exporter.export(&config)
        }
        other => {
            eprintln!("Unsupported target: '{}'. Valid targets are: windows, android, web, ios", other);
            std::process::exit(1);
        }
    };

    match result {
        Ok(res) => {
            println!("Export Succeeded!");
            println!("Artifact: {:?}", res.primary_artifact);
            println!("Details: {}", res.message);
        }
        Err(err) => {
            eprintln!("Export Failed: {}", err);
            std::process::exit(1);
        }
    }
}

fn handle_scene(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: bevylite scene <create|inspect|move|add-comp> [OPTIONS]");
        std::process::exit(1);
    }

    match args[0].as_str() {
        "create" => {
            if args.len() < 2 {
                eprintln!("Usage: bevylite scene create <FILE.ron> [--preset <starter|minimal|empty>]");
                std::process::exit(1);
            }
            let file_path = Path::new(&args[1]);
            let mut preset = "starter";
            if args.len() >= 4 && args[2] == "--preset" {
                preset = &args[3];
            }
            match create_scene_ron(file_path, preset) {
                Ok(msg) => println!("{}", msg),
                Err(err) => {
                    eprintln!("Failed to create scene: {}", err);
                    std::process::exit(1);
                }
            }
        }
        "inspect" => {
            if args.len() < 2 {
                eprintln!("Usage: bevylite scene inspect <FILE.ron>");
                std::process::exit(1);
            }
            let file_path = Path::new(&args[1]);
            match inspect_scene_ron(file_path) {
                Ok(report) => println!("{}", report),
                Err(err) => {
                    eprintln!("Inspection failed: {}", err);
                    std::process::exit(1);
                }
            }
        }
        "move" => {
            if args.len() < 3 {
                eprintln!("Usage: bevylite scene move <FILE.ron> <ENTITY_NAME> [--x <X>] [--y <Y>] [--z <Z>]");
                std::process::exit(1);
            }
            let file_path = Path::new(&args[1]);
            let entity_name = &args[2];
            let mut x = None;
            let mut y = None;
            let mut z = None;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--x" if i + 1 < args.len() => { x = args[i + 1].parse().ok(); i += 1; }
                    "--y" if i + 1 < args.len() => { y = args[i + 1].parse().ok(); i += 1; }
                    "--z" if i + 1 < args.len() => { z = args[i + 1].parse().ok(); i += 1; }
                    _ => {}
                }
                i += 1;
            }
            let translation = match (x, y, z) {
                (Some(vx), Some(vy), Some(vz)) => Some([vx, vy, vz]),
                (Some(vx), Some(vy), None) => Some([vx, vy, 0.0]),
                (Some(vx), None, None) => Some([vx, 0.0, 0.0]),
                _ => None,
            };
            match move_component_in_scene(file_path, entity_name, translation, None, None) {
                Ok(msg) => println!("{}", msg),
                Err(err) => {
                    eprintln!("Move failed: {}", err);
                    std::process::exit(1);
                }
            }
        }
        "add-comp" => {
            if args.len() < 4 {
                eprintln!("Usage: bevylite scene add-comp <FILE.ron> <ENTITY_NAME> <COMPONENT_TYPE> [VALUE]");
                std::process::exit(1);
            }
            let file_path = Path::new(&args[1]);
            let entity_name = &args[2];
            let comp_type = &args[3];
            let val = args.get(4).map(|s| s.as_str());
            match add_component_to_scene(file_path, entity_name, comp_type, val) {
                Ok(msg) => println!("{}", msg),
                Err(err) => {
                    eprintln!("Add component failed: {}", err);
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("Unknown scene command: '{}'. Valid: create, inspect, move, add-comp", other);
            std::process::exit(1);
        }
    }
}

fn handle_tools() {
    println!("Registered Bevylite Viewport Tools:");
    println!("  • [Q] Select Tool  (Marquee & Raycast Entity Selection)");
    println!("  • [W] Move Tool    (3-Axis Planar Translation Gizmo)");
    println!("  • [E] Rotate Tool  (Arcball & Gimbal Orientation)");
    println!("  • [R] Scale Tool   (3-Axis Uniform & Non-Uniform Scaling)");
}

/// Runs the Model Context Protocol (MCP) JSON-RPC server over stdio.
fn handle_mcp() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    eprintln!("[Bevylite MCP] Server starting on stdio...");

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }

        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            let err_response = json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": "Parse error" }
            });
            let _ = writeln!(stdout, "{}", err_response);
            let _ = stdout.flush();
            continue;
        };

        let req_id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let response = match method {
            "initialize" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {
                            "name": "bevylite-mcp",
                            "version": "0.1.0"
                        },
                        "capabilities": {
                            "tools": {}
                        }
                    }
                })
            }
            "notifications/initialized" => {
                continue;
            }
            "ping" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {}
                })
            }
            "tools/list" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "tools": [
                            {
                                "name": "bevylite_status",
                                "description": "Returns status, version, and architecture of Bevylite game engine editor.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {}
                                }
                            },
                            {
                                "name": "bevylite_create_project",
                                "description": "Scaffolds a new standalone Bevy game project with clean Cargo.toml and starter systems.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string", "description": "Name of the project" },
                                        "path": { "type": "string", "description": "Target directory path" }
                                    },
                                    "required": ["name"]
                                }
                            },
                            {
                                "name": "bevylite_create_scene",
                                "description": "Creates a new Bevy .ron scene file with customizable entities (camera, lighting, primitives).",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "path": { "type": "string", "description": "Target path for .ron scene file" },
                                        "preset": { "type": "string", "enum": ["starter", "minimal", "empty"], "description": "Scene template preset" }
                                    },
                                    "required": ["path"]
                                }
                            },
                            {
                                "name": "bevylite_inspect_scene",
                                "description": "Inspects a Bevy scene file (.ron or .bsn) and returns detailed entity breakdown, transforms, and components.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "file_path": { "type": "string", "description": "Path to the .ron or .bsn file" }
                                    },
                                    "required": ["file_path"]
                                }
                            },
                            {
                                "name": "bevylite_move_component",
                                "description": "Updates the 3D Transform (translation, rotation, scale) of an entity in a Bevy .ron scene file.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "file_path": { "type": "string", "description": "Path to the .ron scene file" },
                                        "entity_name": { "type": "string", "description": "Name of the entity to modify (e.g. 'Cube', 'Player Cube', 'Sun Light')" },
                                        "translation": {
                                            "type": "array",
                                            "items": { "type": "number" },
                                            "description": "[x, y, z] position in 3D world space"
                                        },
                                        "rotation": {
                                            "type": "array",
                                            "items": { "type": "number" },
                                            "description": "[rx, ry, rz, rw] quaternion orientation"
                                        },
                                        "scale": {
                                            "type": "array",
                                            "items": { "type": "number" },
                                            "description": "[sx, sy, sz] 3D scale"
                                        }
                                    },
                                    "required": ["file_path", "entity_name"]
                                }
                            },
                            {
                                "name": "bevylite_add_component",
                                "description": "Attaches a new component (e.g. SpinAnimation, PointLight, Velocity, GameplayTag) to an entity in a .ron scene.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "file_path": { "type": "string", "description": "Path to the .ron scene file" },
                                        "entity_name": { "type": "string", "description": "Name of the target entity" },
                                        "component_type": { "type": "string", "description": "Component type (SpinAnimation, PointLight, Velocity, GameplayTag)" },
                                        "value": { "type": "string", "description": "Optional component value or parameter" }
                                    },
                                    "required": ["file_path", "entity_name", "component_type"]
                                }
                            },
                            {
                                "name": "bevylite_export_build",
                                "description": "Headlessly builds and exports game for Windows, Android, Web, or iOS.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "target": { "type": "string", "enum": ["windows", "android", "web", "ios"], "description": "Target platform" },
                                        "output_dir": { "type": "string", "description": "Output directory" },
                                        "project_dir": { "type": "string", "description": "Windows Cargo project directory containing Cargo.toml" }
                                    },
                                    "required": ["target"]
                                }
                            }
                        ]
                    }
                })
            }
            "tools/call" => {
                let params = request.get("params");
                let tool_name = params.and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                let arguments = params.and_then(|p| p.get("arguments")).cloned().unwrap_or(json!({}));

                let tool_result = match tool_name {
                    "bevylite_status" => {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": "Bevylite v0.1.0 (Bevy 0.20-dev | Rust 2024)\nTheme: Obsidian & Electric Cyan\nPillars: Hierarchy, Inspector, Viewport, Asset Browser, PIE Simulation\nDocking: Drag-and-dock Left/Right/Floating supported."
                            }]
                        })
                    }
                    "bevylite_create_project" => {
                        let name = arguments.get("name").and_then(|n| n.as_str()).unwrap_or("NewBevyGame");
                        let path = arguments.get("path").and_then(|p| p.as_str()).map(PathBuf::from)
                            .unwrap_or_else(|| env::current_dir().unwrap_or_default().join(name));

                        match bevy_editor_lib::project::ProjectScaffolder::create_project(&path, name) {
                            Ok(p) => json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("Successfully created project '{}' at {:?}", name, p)
                                }]
                            }),
                            Err(e) => json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": format!("Error: {}", e) }]
                            }),
                        }
                    }
                    "bevylite_create_scene" => {
                        let path = arguments.get("path").and_then(|p| p.as_str()).unwrap_or("./assets/scenes/main.ron");
                        let preset = arguments.get("preset").and_then(|pr| pr.as_str()).unwrap_or("starter");
                        match create_scene_ron(Path::new(path), preset) {
                            Ok(msg) => {
                                log_mcp_bridge_event("create_scene", preset, path);
                                json!({
                                    "content": [{ "type": "text", "text": msg }]
                                })
                            }
                            Err(e) => json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": format!("Error: {}", e) }]
                            }),
                        }
                    }
                    "bevylite_inspect_scene" => {
                        let file = arguments.get("file_path").and_then(|f| f.as_str()).unwrap_or("");
                        match inspect_scene_ron(Path::new(file)) {
                            Ok(report) => json!({
                                "content": [{ "type": "text", "text": report }]
                            }),
                            Err(e) => json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": format!("Error: {}", e) }]
                            }),
                        }
                    }
                    "bevylite_move_component" | "bevylite_modify_transform" => {
                        let file = arguments.get("file_path").and_then(|f| f.as_str()).unwrap_or("");
                        let entity = arguments.get("entity_name").and_then(|e| e.as_str()).unwrap_or("");
                        let translation: Option<[f32; 3]> = arguments.get("translation").and_then(|t| {
                            let arr = t.as_array()?;
                            if arr.len() >= 3 {
                                Some([arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32])
                            } else {
                                None
                            }
                        });
                        let rotation: Option<[f32; 4]> = arguments.get("rotation").and_then(|r| {
                            let arr = r.as_array()?;
                            if arr.len() >= 4 {
                                Some([arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32, arr[3].as_f64()? as f32])
                            } else {
                                None
                            }
                        });
                        let scale: Option<[f32; 3]> = arguments.get("scale").and_then(|s| {
                            let arr = s.as_array()?;
                            if arr.len() >= 3 {
                                Some([arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32])
                            } else {
                                None
                            }
                        });

                        match move_component_in_scene(Path::new(file), entity, translation, rotation, scale) {
                            Ok(msg) => {
                                log_mcp_bridge_event("move", entity, &format!("{:?}", translation));
                                json!({
                                    "content": [{ "type": "text", "text": msg }]
                                })
                            }
                            Err(e) => json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": format!("Error: {}", e) }]
                            }),
                        }
                    }
                    "bevylite_add_component" => {
                        let file = arguments.get("file_path").and_then(|f| f.as_str()).unwrap_or("");
                        let entity = arguments.get("entity_name").and_then(|e| e.as_str()).unwrap_or("");
                        let comp_type = arguments.get("component_type").and_then(|c| c.as_str()).unwrap_or("");
                        let val = arguments.get("value").and_then(|v| v.as_str());

                        match add_component_to_scene(Path::new(file), entity, comp_type, val) {
                            Ok(msg) => {
                                log_mcp_bridge_event("add_component", entity, comp_type);
                                json!({
                                    "content": [{ "type": "text", "text": msg }]
                                })
                            }
                            Err(e) => json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": format!("Error: {}", e) }]
                            }),
                        }
                    }
                    "bevylite_export_build" => {
                        let target = arguments.get("target").and_then(|t| t.as_str()).unwrap_or("windows");
                        let out = arguments.get("output_dir").and_then(|o| o.as_str()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("./dist"));
                        let project_dir = arguments.get("project_dir").and_then(|p| p.as_str()).map(PathBuf::from);
                        let config = ExportContext {
                            project_name: "BevyGame".to_string(),
                            project_dir,
                            scene_path: None,
                            output_dir: out,
                            release: true,
                        };
                        let res = match target {
                            "android" => bevy_editor_lib::export::android::AndroidExporter.export(&config),
                            "windows" => bevy_editor_lib::export::windows::WindowsExporter.export(&config),
                            "web" => bevy_editor_lib::export::web::WebExporter.export(&config),
                            "ios" => bevy_editor_lib::export::ios::IosExporter.export(&config),
                            _ => Err("Invalid target".to_string()),
                        };
                        match res {
                            Ok(r) => {
                                log_mcp_bridge_event("export_build", target, &format!("{:?}", r.primary_artifact));
                                json!({
                                    "content": [{
                                        "type": "text",
                                        "text": format!("Export Succeeded [{}]\nArtifact: {:?}\nDetails: {}", target, r.primary_artifact, r.message)
                                    }]
                                })
                            }
                            Err(e) => json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": format!("Export Failed: {}", e) }]
                            }),
                        }
                    }
                    unknown => {
                        json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": format!("Unknown tool: {}", unknown) }]
                        })
                    }
                };

                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": tool_result
                })
            }
            _ => {
                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": { "code": -32601, "message": "Method not found" }
                })
            }
        };

        let _ = writeln!(stdout, "{}", response);
        let _ = stdout.flush();
    }
}

/// Creates a Bevy .ron scene file from predefined presets.
fn create_scene_ron(path: &Path, preset: &str) -> Result<String, String> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let ron_content = match preset {
        "minimal" => {
            r#"(
  resources: {},
  entities: {
    0: (
      components: {
        "bevy_core::name::Name": (
          name: "Main Camera",
        ),
        "bevy_transform::components::transform::Transform": (
          translation: (0.0, 3.0, 7.0),
          rotation: (-0.20, 0.00, 0.00, 0.97),
          scale: (1.00, 1.00, 1.00),
        ),
        "bevy_render::camera::camera::Camera3d": (),
      },
    ),
    1: (
      components: {
        "bevy_core::name::Name": (
          name: "Directional Light",
        ),
        "bevy_transform::components::transform::Transform": (
          translation: (4.00, 10.00, 4.00),
          rotation: (-0.38, 0.38, -0.16, 0.82),
          scale: (1.00, 1.00, 1.00),
        ),
        "bevy_light::directional_light::DirectionalLight": (
          illuminance: 10000.0,
          shadow_maps_enabled: true,
        ),
      },
    ),
  },
)"#
        }
        "empty" => {
            r#"(
  resources: {},
  entities: {},
)"#
        }
        _ => {
            r#"(
  resources: {},
  entities: {
    0: (
      components: {
        "bevy_core::name::Name": (
          name: "Main Camera",
        ),
        "bevy_transform::components::transform::Transform": (
          translation: (0.00, 4.00, 8.00),
          rotation: (-0.25, 0.00, 0.00, 0.96),
          scale: (1.00, 1.00, 1.00),
        ),
        "bevy_render::camera::camera::Camera3d": (),
      },
    ),
    1: (
      components: {
        "bevy_core::name::Name": (
          name: "Sun Light",
        ),
        "bevy_transform::components::transform::Transform": (
          translation: (5.00, 12.00, 5.00),
          rotation: (-0.38, 0.38, -0.16, 0.82),
          scale: (1.00, 1.00, 1.00),
        ),
        "bevy_light::directional_light::DirectionalLight": (
          illuminance: 12000.0,
          shadow_maps_enabled: true,
        ),
      },
    ),
    2: (
      components: {
        "bevy_core::name::Name": (
          name: "Player Cube",
        ),
        "bevy_transform::components::transform::Transform": (
          translation: (0.00, 1.00, 0.00),
          rotation: (0.00, 0.00, 0.00, 1.00),
          scale: (1.50, 1.50, 1.50),
        ),
        "bevy_render::mesh::Mesh3d": (),
      },
    ),
    3: (
      components: {
        "bevy_core::name::Name": (
          name: "Ground Plane",
        ),
        "bevy_transform::components::transform::Transform": (
          translation: (0.00, 0.00, 0.00),
          rotation: (0.00, 0.00, 0.00, 1.00),
          scale: (20.00, 1.00, 20.00),
        ),
        "bevy_render::mesh::Mesh3d": (),
      },
    ),
  },
)"#
        }
    };

    fs::write(path, ron_content)
        .map_err(|e| format!("Failed to write scene file: {e}"))?;

    Ok(format!("Successfully created '{}' scene at {:?}", preset, path))
}

/// Parses and returns a detailed inspection report of a .ron scene file.
fn inspect_scene_ron(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("Scene file does not exist: {:?}", path));
    }
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read scene: {e}"))?;

    let mut summary = format!("◈ Bevylite Scene Report: {:?}\n", path);
    summary.push_str(&format!("  File size: {} bytes\n\n", content.len()));

    let mut current_name: Option<String> = None;
    let mut current_trans: Option<String> = None;
    let mut current_rot: Option<String> = None;
    let mut current_scale: Option<String> = None;
    let mut current_comps: Vec<String> = Vec::new();
    let mut count = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name: \"") {
            if let Some(end) = trimmed[7..].find('"') {
                current_name = Some(trimmed[7..7 + end].to_string());
            }
        } else if trimmed.starts_with("translation: (") {
            if let Some(end) = trimmed.find(')') {
                current_trans = Some(trimmed[14..end].to_string());
            }
        } else if trimmed.starts_with("rotation: (") {
            if let Some(end) = trimmed.find(')') {
                current_rot = Some(trimmed[11..end].to_string());
            }
        } else if trimmed.starts_with("scale: (") {
            if let Some(end) = trimmed.find(')') {
                current_scale = Some(trimmed[8..end].to_string());
            }
        } else if trimmed.starts_with('"') && trimmed.contains("::") && trimmed.ends_with("\": (") {
            let comp = trimmed.trim_matches('"').split("\":").next().unwrap_or(trimmed);
            let short_name = comp.split("::").last().unwrap_or(comp);
            if short_name != "Name" && short_name != "Transform" {
                current_comps.push(short_name.to_string());
            }
        }

        if trimmed == "}," && current_name.is_some() {
            if let Some(name) = current_name.take() {
                count += 1;
                summary.push_str(&format!("  • Entity #{}: \"{}\"\n", count, name));
                if let Some(t) = current_trans.take() {
                    summary.push_str(&format!("    - Translation: ({})\n", t));
                }
                if let Some(r) = current_rot.take() {
                    summary.push_str(&format!("    - Rotation:    ({})\n", r));
                }
                if let Some(s) = current_scale.take() {
                    summary.push_str(&format!("    - Scale:       ({})\n", s));
                }
                if !current_comps.is_empty() {
                    summary.push_str(&format!("    - Components:  [{}]\n", current_comps.join(", ")));
                    current_comps.clear();
                }
                summary.push('\n');
            }
        }
    }

    if count == 0 {
        summary.push_str("  No named entities parsed.\n");
    } else {
        summary.push_str(&format!("Total Entities: {}\n", count));
    }

    Ok(summary)
}

/// Moves or rescales an entity's Transform component in a .ron scene file.
fn move_component_in_scene(
    path: &Path,
    entity_name: &str,
    translation: Option<[f32; 3]>,
    rotation: Option<[f32; 4]>,
    scale: Option<[f32; 3]>,
) -> Result<String, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read scene: {e}"))?;

    let target_needle = format!("name: \"{}\"", entity_name);
    let Some(name_pos) = content.find(&target_needle) else {
        return Err(format!("Entity with name '{}' not found in {:?}", entity_name, path));
    };

    let before_slice = &content[..name_pos];
    let block_start = before_slice.rfind("components: {").unwrap_or(0);
    let after_slice = &content[name_pos..];
    let block_end = name_pos + after_slice.find("},\n    ),").unwrap_or(after_slice.find("},").unwrap_or(after_slice.len()));

    let mut entity_block = content[block_start..block_end].to_string();

    if let Some([x, y, z]) = translation {
        let new_trans = format!("translation: ({:.2}, {:.2}, {:.2})", x, y, z);
        if let Some(pos) = entity_block.find("translation: (") {
            if let Some(end) = entity_block[pos..].find(')') {
                entity_block.replace_range(pos..pos + end + 1, &new_trans);
            }
        }
    }

    if let Some([rx, ry, rz, rw]) = rotation {
        let new_rot = format!("rotation: ({:.2}, {:.2}, {:.2}, {:.2})", rx, ry, rz, rw);
        if let Some(pos) = entity_block.find("rotation: (") {
            if let Some(end) = entity_block[pos..].find(')') {
                entity_block.replace_range(pos..pos + end + 1, &new_rot);
            }
        }
    }

    if let Some([sx, sy, sz]) = scale {
        let new_scale = format!("scale: ({:.2}, {:.2}, {:.2})", sx, sy, sz);
        if let Some(pos) = entity_block.find("scale: (") {
            if let Some(end) = entity_block[pos..].find(')') {
                entity_block.replace_range(pos..pos + end + 1, &new_scale);
            }
        }
    }

    let mut new_content = content[..block_start].to_string();
    new_content.push_str(&entity_block);
    new_content.push_str(&content[block_end..]);

    fs::write(path, new_content)
        .map_err(|e| format!("Failed to save scene: {e}"))?;

    let mut msg = format!("Updated Transform for '{}' in {:?}:\n", entity_name, path);
    if let Some(t) = translation {
        msg.push_str(&format!("  • Translation: ({:.2}, {:.2}, {:.2})\n", t[0], t[1], t[2]));
    }
    if let Some(r) = rotation {
        msg.push_str(&format!("  • Rotation:    ({:.2}, {:.2}, {:.2}, {:.2})\n", r[0], r[1], r[2], r[3]));
    }
    if let Some(s) = scale {
        msg.push_str(&format!("  • Scale:       ({:.2}, {:.2}, {:.2})\n", s[0], s[1], s[2]));
    }

    Ok(msg)
}

/// Attaches a new component to an entity in a .ron scene file.
fn add_component_to_scene(
    path: &Path,
    entity_name: &str,
    component_type: &str,
    value: Option<&str>,
) -> Result<String, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read scene: {e}"))?;

    let target_needle = format!("name: \"{}\"", entity_name);
    let Some(name_pos) = content.find(&target_needle) else {
        return Err(format!("Entity with name '{}' not found in {:?}", entity_name, path));
    };

    let after_slice = &content[name_pos..];
    let Some(close_pos) = after_slice.find("},") else {
        return Err("Could not find closing brace for entity components".to_string());
    };
    let insert_pos = name_pos + close_pos;

    let comp_full_name = match component_type {
        "SpinAnimation" => "bevy_editor::components::SpinAnimation",
        "PointLight" => "bevy_light::point_light::PointLight",
        "Velocity" => "bevy_editor::components::Velocity",
        "GameplayTag" => "bevy_editor::components::GameplayTag",
        other => other,
    };

    let comp_body = match value {
        Some(v) if !v.trim().is_empty() => format!("(\n          val: \"{}\",\n        ),\n", v),
        _ => "(),\n".to_string(),
    };

    let new_comp_snippet = format!("        \"{}\": {}\n", comp_full_name, comp_body);

    let mut new_content = content[..insert_pos].to_string();
    new_content.push_str(&new_comp_snippet);
    new_content.push_str(&content[insert_pos..]);

    fs::write(path, new_content)
        .map_err(|e| format!("Failed to save scene with new component: {e}"))?;

    Ok(format!("Successfully attached component '{}' to entity '{}' in {:?}", component_type, entity_name, path))
}

/// Logs an MCP event to `assets/.mcp_bridge.json` so the running editor displays live AI interaction.
fn log_mcp_bridge_event(action: &str, entity: &str, details: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let data = json!({
        "action": action,
        "entity": entity,
        "details": details,
        "time": now
    });

    let _ = fs::write("assets/.mcp_bridge.json", data.to_string());
}

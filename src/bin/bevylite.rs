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
            _ => {}
        }
        i += 1;
    }

    println!("Building export for target '{}' into {:?}...", target, output_dir);

    let config = ExportContext {
        project_name: "BevyGame".to_string(),
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
    if args.is_empty() || args[0] != "inspect" || args.len() < 2 {
        eprintln!("Usage: bevylite scene inspect <FILE.ron>");
        std::process::exit(1);
    }

    let file_path = Path::new(&args[1]);
    if !file_path.exists() {
        eprintln!("Error: Scene file does not exist: {:?}", file_path);
        std::process::exit(1);
    }

    match fs::read_to_string(file_path) {
        Ok(content) => {
            let entity_count = content.matches("(Entity(").count() + content.matches("entity:").count();
            let has_camera = content.contains("Camera") || content.contains("Camera3d");
            let has_light = content.contains("Light") || content.contains("DirectionalLight") || content.contains("PointLight");
            let has_mesh = content.contains("Mesh") || content.contains("Mesh3d");

            println!("◈ Scene Inspection: {:?}", file_path);
            println!("  Entities Identified: ~{}", entity_count.max(1));
            println!("  Camera Present:     {}", if has_camera { "Yes (3D Camera)" } else { "No" });
            println!("  Lighting Present:   {}", if has_light { "Yes (Directional/Point)" } else { "No" });
            println!("  Meshes Present:     {}", if has_mesh { "Yes (Static/Dynamic Mesh)" } else { "No" });
            println!("  File Size:          {} bytes", content.len());
        }
        Err(err) => {
            eprintln!("Failed to read scene file: {}", err);
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
                                "name": "bevylite_inspect_scene",
                                "description": "Inspects a Bevy scene file (.ron or .bsn) and returns entities, components, and stats.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "file_path": { "type": "string", "description": "Path to the .ron or .bsn file" }
                                    },
                                    "required": ["file_path"]
                                }
                            },
                            {
                                "name": "bevylite_export_build",
                                "description": "Headlessly builds and exports game for Windows, Android, Web, or iOS.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "target": { "type": "string", "enum": ["windows", "android", "web", "ios"], "description": "Target platform" },
                                        "output_dir": { "type": "string", "description": "Output directory" }
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
                    "bevylite_inspect_scene" => {
                        let file = arguments.get("file_path").and_then(|f| f.as_str()).unwrap_or("");
                        let p = Path::new(file);
                        if let Ok(content) = fs::read_to_string(p) {
                            let count = content.matches("(Entity(").count() + content.matches("entity:").count();
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("Scene File: {:?}\nEntities: ~{}\nFile Size: {} bytes", p, count.max(1), content.len())
                                }]
                            })
                        } else {
                            json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": format!("Cannot read scene file: {}", file) }]
                            })
                        }
                    }
                    "bevylite_export_build" => {
                        let target = arguments.get("target").and_then(|t| t.as_str()).unwrap_or("windows");
                        let out = arguments.get("output_dir").and_then(|o| o.as_str()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("./dist"));
                        let config = ExportContext {
                            project_name: "BevyGame".to_string(),
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
                            Ok(r) => json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("Export Succeeded [{}]\nArtifact: {:?}\nDetails: {}", target, r.primary_artifact, r.message)
                                }]
                            }),
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

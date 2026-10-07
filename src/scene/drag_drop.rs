//! Handles dragging and dropping 3D models (glTF/GLB) and scene files into the window.
//!
//! Smart Drag-and-Drop Pipeline:
//! - When a file is dropped from the OS (Downloads, Desktop, etc.):
//!   1. It is copied to the project `assets/` directory (into the hovered folder or `assets/models/`).
//!   2. The Project Explorer is notified to refresh so the new asset appears immediately.
//!   3. If the mouse was released over the 3D Viewport (and not solely over the Project Explorer panel),
//!      the model/scene is automatically spawned into the active scene!

use std::fs;
use std::path::PathBuf;
use bevy::{
    ecs::message::MessageWriter,
    gltf::GltfAssetLabel,
    prelude::*,
    window::FileDragAndDrop,
    world_serialization::{DynamicWorldRoot, WorldAssetRoot},
};
use crate::export::ExportStatus;
use crate::ui::asset_browser::{HoveredAssetDirectory, RefreshAssetBrowserMessage};

pub struct DragDropPlugin;

impl Plugin for DragDropPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_file_drag_and_drop);
    }
}

fn handle_file_drag_and_drop(
    mut messages: MessageReader<FileDragAndDrop>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut export_status: ResMut<ExportStatus>,
    hovered_dir: Res<HoveredAssetDirectory>,
    mut refresh_writer: MessageWriter<RefreshAssetBrowserMessage>,
    windows: Query<&Window>,
) {
    for msg in messages.read() {
        match msg {
            FileDragAndDrop::DroppedFile { path_buf, .. } => {
                let Some(extension) = path_buf.extension().and_then(|s| s.to_str()) else {
                    continue;
                };

                let ext = extension.to_lowercase();
                let file_name = path_buf
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Dropped Model".to_string());

                // Check cursor position relative to Project Explorer panel
                // Project Explorer rect: bottom-left (left: 12..352, bottom: 36..296)
                let is_over_project_explorer = if let Ok(window) = windows.single() {
                    if let Some(cursor_pos) = window.cursor_position() {
                        let win_height = window.height();
                        // In Bevy window coords, Y=0 is top, Y=win_height is bottom
                        let cursor_y_from_bottom = win_height - cursor_pos.y;
                        cursor_pos.x >= 0.0 && cursor_pos.x <= 360.0 && cursor_y_from_bottom >= 0.0 && cursor_y_from_bottom <= 300.0
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Determine target directory
                let target_dir: PathBuf = if hovered_dir.0.exists() && hovered_dir.0.is_dir() {
                    hovered_dir.0.clone()
                } else {
                    match ext.as_str() {
                        "glb" | "gltf" => PathBuf::from("assets/models"),
                        "ron" | "bsn" => PathBuf::from("assets/scenes"),
                        "png" | "jpg" | "jpeg" => PathBuf::from("assets/textures"),
                        _ => PathBuf::from("assets"),
                    }
                };

                // Ensure target directory exists
                let _ = fs::create_dir_all(&target_dir);

                // Copy source file to target directory inside assets/
                let destination = target_dir.join(&file_name);
                let copy_result = if path_buf.canonicalize().ok() != destination.canonicalize().ok() {
                    fs::copy(path_buf, &destination)
                } else {
                    Ok(0) // Already in destination
                };

                if let Err(e) = copy_result {
                    warn!("Failed to copy file to {:?}: {e}", destination);
                } else {
                    info!("Successfully copied file to: {:?}", destination);
                }

                // Request Project Explorer refresh so new file is visible
                refresh_writer.write(RefreshAssetBrowserMessage);

                // Calculate asset-relative path for Bevy AssetServer
                let relative_path = if let Ok(stripped) = destination.strip_prefix("assets") {
                    stripped.to_string_lossy().replace('\\', "/")
                } else {
                    destination.to_string_lossy().replace('\\', "/")
                };

                // If dropped over Project Explorer only, do not spawn into scene
                if is_over_project_explorer {
                    info!("File dropped into Project Explorer folder: {:?}", target_dir);
                    *export_status = ExportStatus::InProgress {
                        target: "Project Explorer".to_string(),
                        progress: format!("Imported '{file_name}' to {:?}", target_dir),
                    };
                    continue;
                }

                // If dropped on the scene viewport, spawn it!
                match ext.as_str() {
                    "gltf" | "glb" => {
                        info!("Spawning dropped 3D model into scene: {}", relative_path);

                        commands.spawn((
                            crate::scene::SceneEntity,
                            Name::new(file_name.clone()),
                            WorldAssetRoot(
                                asset_server.load(GltfAssetLabel::Scene(0).from_asset(relative_path.clone())),
                            ),
                            Transform::IDENTITY,
                        ));

                        *export_status = ExportStatus::InProgress {
                            target: "Viewer".to_string(),
                            progress: format!("Spawned model: {file_name}"),
                        };
                    }
                    "ron" | "bsn" => {
                        info!("Loading dropped scene into viewport: {}", relative_path);

                        commands.spawn((
                            crate::scene::SceneEntity,
                            Name::new(format!("Scene: {file_name}")),
                            DynamicWorldRoot(asset_server.load(relative_path)),
                            Transform::IDENTITY,
                        ));

                        *export_status = ExportStatus::InProgress {
                            target: "Viewer".to_string(),
                            progress: format!("Loaded scene: {file_name}"),
                        };
                    }
                    "fbx" => {
                        warn!("FBX format dropped: {:?}", path_buf);
                        *export_status = ExportStatus::Failed {
                            target: "Model Loader".to_string(),
                            error: format!(
                                "FBX is not natively supported. Please convert '{file_name}' to .glb/.gltf (e.g. via Blender)."
                            ),
                        };
                    }
                    "obj" | "stl" => {
                        *export_status = ExportStatus::Failed {
                            target: "Model Loader".to_string(),
                            error: format!(
                                "Format .{ext} requires optional loader plugin. Prefer .glb / .gltf."
                            ),
                        };
                    }
                    _ => {
                        info!("Imported file to assets: {}", file_name);
                    }
                }
            }
            FileDragAndDrop::HoveredFile { path_buf, .. } => {
                info!("Hovering file: {:?}", path_buf);
            }
            FileDragAndDrop::HoveredFileCanceled { .. } => {}
        }
    }
}

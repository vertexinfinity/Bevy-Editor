//! Handles dragging and dropping 3D models (glTF/GLB) and scene files into the window.

use bevy::{
    gltf::GltfAssetLabel,
    prelude::*,
    window::FileDragAndDrop,
    world_serialization::{DynamicWorldRoot, WorldAssetRoot},
};
use crate::export::ExportStatus;

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

                match ext.as_str() {
                    "gltf" | "glb" => {
                        info!("Loading dropped 3D model: {:?}", path_buf);
                        let path_str = path_buf.to_string_lossy().to_string();

                        // Spawn glTF model entity with scene root
                        commands.spawn((
                            crate::scene::SceneEntity,
                            Name::new(file_name.clone()),
                            WorldAssetRoot(
                                asset_server.load(GltfAssetLabel::Scene(0).from_asset(path_str)),
                            ),
                            Transform::IDENTITY,
                        ));

                        *export_status = ExportStatus::InProgress {
                            target: "Viewer".to_string(),
                            progress: format!("Spawned model: {file_name}"),
                        };
                    }
                    "ron" | "bsn" => {
                        info!("Loading dropped scene: {:?}", path_buf);
                        let path_str = path_buf.to_string_lossy().to_string();

                        commands.spawn((
                            crate::scene::SceneEntity,
                            Name::new(format!("Scene: {file_name}")),
                            DynamicWorldRoot(asset_server.load(path_str)),
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
                        info!("Dropped unsupported file type: .{}", ext);
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

//! Extensible multi-platform export engine for Bevy-Editor.

pub mod android;
pub mod ios;
pub mod web;
pub mod windows;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use bevy::ecs::message::Message;
use bevy::prelude::*;

/// Context provided to an [`Exporter`] when generating a build.
#[derive(Debug, Clone)]
pub struct ExportContext {
    pub project_name: String,
    pub scene_path: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub release: bool,
}

/// The result returned by an export execution.
#[derive(Debug, Clone)]
pub struct ExportResult {
    pub primary_artifact: PathBuf,
    pub message: String,
}

/// A pluggable export target (e.g. Android, Windows, WebAssembly, iOS).
pub trait Exporter: Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    #[allow(dead_code)]
    fn description(&self) -> &'static str;
    fn export(&self, ctx: &ExportContext) -> Result<ExportResult, String>;
}

/// Registry storing all available platform exporters.
#[derive(Resource, Default)]
pub struct ExportRegistry {
    exporters: HashMap<&'static str, Arc<dyn Exporter>>,
}

impl ExportRegistry {
    pub fn register<T: Exporter>(&mut self, exporter: T) {
        self.exporters.insert(exporter.id(), Arc::new(exporter));
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Exporter>> {
        self.exporters.get(id).cloned()
    }

    #[allow(dead_code)]
    pub fn list(&self) -> Vec<Arc<dyn Exporter>> {
        self.exporters.values().cloned().collect()
    }
}

/// Current status of export jobs.
#[derive(Resource, Default, Debug, Clone)]
pub enum ExportStatus {
    #[default]
    Idle,
    InProgress { target: String, progress: String },
    Success { target: String, message: String, #[allow(dead_code)] path: PathBuf },
    Failed { target: String, error: String },
}

/// Message triggered when an export is requested from the UI.
#[derive(Message, Debug, Clone)]
pub struct RequestExportMessage {
    pub target_id: String,
    pub output_dir: PathBuf,
    pub project_name: String,
    pub scene_path: Option<PathBuf>,
}

/// Plugin registering export types and built-in exporters.
pub struct ExportPlugin;

impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = ExportRegistry::default();
        registry.register(android::AndroidExporter::default());
        registry.register(windows::WindowsExporter::default());
        registry.register(web::WebExporter::default());
        registry.register(ios::IosExporter::default());

        app.insert_resource(registry)
            .init_resource::<ExportStatus>()
            .add_message::<RequestExportMessage>()
            .add_systems(Update, handle_export_requests);
    }
}

fn handle_export_requests(
    mut messages: MessageReader<RequestExportMessage>,
    registry: Res<ExportRegistry>,
    mut status: ResMut<ExportStatus>,
) {
    for msg in messages.read() {
        if let Some(exporter) = registry.get(&msg.target_id) {
            info!("Starting export for target '{}'...", msg.target_id);
            *status = ExportStatus::InProgress {
                target: exporter.name().to_string(),
                progress: "Gathering assets and building...".to_string(),
            };

            let ctx = ExportContext {
                project_name: msg.project_name.clone(),
                scene_path: msg.scene_path.clone(),
                output_dir: msg.output_dir.clone(),
                release: true,
            };

            match exporter.export(&ctx) {
                Ok(res) => {
                    info!("Export succeeded: {}", res.message);
                    *status = ExportStatus::Success {
                        target: exporter.name().to_string(),
                        message: res.message,
                        path: res.primary_artifact,
                    };
                }
                Err(err) => {
                    error!("Export failed: {}", err);
                    *status = ExportStatus::Failed {
                        target: exporter.name().to_string(),
                        error: err,
                    };
                }
            }
        } else {
            error!("Unknown export target ID: {}", msg.target_id);
        }
    }
}

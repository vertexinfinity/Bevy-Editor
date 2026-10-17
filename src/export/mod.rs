//! Extensible multi-platform export engine for Bevy-Editor.

pub mod android;
pub mod ios;
pub mod linux;
pub mod macroquad;
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
    pub project_dir: Option<PathBuf>,
    pub scene_path: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub release: bool,
}

/// The result returned by an export execution.
#[derive(Debug, Clone)]
pub struct ExportResult {
    pub primary_artifact: PathBuf,
    pub message: String,
    pub zip_artifact: Option<PathBuf>,
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
    pub project_dir: Option<PathBuf>,
    pub scene_path: Option<PathBuf>,
}

/// Background export job tracking (one export at a time).
#[derive(Resource, Default)]
pub struct ExportJob {
    rx: Option<std::sync::Mutex<std::sync::mpsc::Receiver<Result<ExportResult, String>>>>,
    started: Option<std::time::Instant>,
    name: String,
    /// (finished_at, success, message, artifact_path, zip_path)
    finished: Option<(std::time::Instant, bool, String, Option<PathBuf>, Option<PathBuf>)>,
}

#[derive(Component)]
struct ExportProgressRoot;
#[derive(Component)]
struct ExportProgressFill;
#[derive(Component)]
struct ExportProgressLabel;
#[derive(Component)]
struct ExportActionsRow;
#[derive(Component)]
struct OpenFolderButton;
#[derive(Component)]
struct OpenZipButton;

/// Plugin registering export types and built-in exporters.
pub struct ExportPlugin;

impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = ExportRegistry::default();
        registry.register(android::AndroidExporter::default());
        registry.register(windows::WindowsExporter::default());
        registry.register(linux::LinuxExporter::default());
        registry.register(web::WebExporter::default());
        registry.register(ios::IosExporter::default());
        registry.register(macroquad::MacroquadExporter { target: macroquad::MacroquadTarget::Desktop });
        registry.register(macroquad::MacroquadExporter { target: macroquad::MacroquadTarget::Web });

        app.insert_resource(registry)
            .init_resource::<ExportStatus>()
            .init_resource::<ExportJob>()
            .add_message::<RequestExportMessage>()
            .add_systems(Startup, spawn_export_progress_widget)
            .add_systems(
                Update,
                (
                    handle_export_requests,
                    poll_export_job,
                    update_export_progress_widget,
                    handle_export_action_clicks,
                ).chain(),
            );
    }
}

fn spawn_export_progress_widget(mut commands: Commands) {
    use crate::ui::theme::*;
    let root = commands
        .spawn((
            crate::scene::EditorInternal,
            ExportProgressRoot,
            GlobalZIndex(100),
            Node {
                position_type: PositionType::Absolute,
                right: px(12),
                bottom: px(34),
                width: px(320),
                padding: UiRect::all(px(10)),
                border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                display: Display::None,
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
        ))
        .id();

    // Status label
    commands.spawn((
        crate::scene::EditorInternal,
        ExportProgressLabel,
        Pickable::IGNORE,
        Text::new(""),
        TextFont { font_size: bevy::text::FontSize::Px(11.0), weight: FontWeight::MEDIUM, ..default() },
        TextColor(TEXT_PRIMARY),
        ChildOf(root),
    ));

    // Progress Bar Track
    let track = commands
        .spawn((
            crate::scene::EditorInternal,
            Pickable::IGNORE,
            Node {
                width: percent(100.0),
                height: px(6),
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(root),
        ))
        .id();

    commands.spawn((
        crate::scene::EditorInternal,
        ExportProgressFill,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: percent(0.0),
            width: percent(30.0),
            height: percent(100.0),
            ..default()
        },
        BackgroundColor(ACCENT_CYAN),
        ChildOf(track),
    ));

    // Action buttons row (Open Folder & Open ZIP)
    let actions_row = commands
        .spawn((
            crate::scene::EditorInternal,
            ExportActionsRow,
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                margin: UiRect::top(px(2)),
                display: Display::None,
                ..default()
            },
            ChildOf(root),
        ))
        .id();

    // "Open Folder" button
    let open_folder_btn = commands
        .spawn((
            crate::scene::EditorInternal,
            OpenFolderButton,
            Button,
            Node {
                padding: UiRect::axes(px(8), px(4)),
                border_radius: BorderRadius::all(px(4)),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(actions_row),
        ))
        .id();

    commands.spawn((
        crate::scene::EditorInternal,
        Pickable::IGNORE,
        Text::new("📁 Open Folder"),
        TextFont { font_size: bevy::text::FontSize::Px(10.5), ..default() },
        TextColor(TEXT_PRIMARY),
        ChildOf(open_folder_btn),
    ));

    // "Locate ZIP" button
    let open_zip_btn = commands
        .spawn((
            crate::scene::EditorInternal,
            OpenZipButton,
            Button,
            Node {
                padding: UiRect::axes(px(8), px(4)),
                border_radius: BorderRadius::all(px(4)),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(actions_row),
        ))
        .id();

    commands.spawn((
        crate::scene::EditorInternal,
        Pickable::IGNORE,
        Text::new("📦 Locate ZIP"),
        TextFont { font_size: bevy::text::FontSize::Px(10.5), ..default() },
        TextColor(ACCENT_CYAN),
        ChildOf(open_zip_btn),
    ));
}

fn handle_export_requests(
    mut messages: MessageReader<RequestExportMessage>,
    registry: Res<ExportRegistry>,
    mut status: ResMut<ExportStatus>,
    mut job: ResMut<ExportJob>,
) {
    for msg in messages.read() {
        if job.rx.is_some() {
            warn!("An export is already running; ignoring request for '{}'", msg.target_id);
            continue;
        }
        let Some(exporter) = registry.get(&msg.target_id) else {
            error!("Unknown export target ID: {}", msg.target_id);
            continue;
        };
        info!("Starting background export for target '{}'...", msg.target_id);
        *status = ExportStatus::InProgress {
            target: exporter.name().to_string(),
            progress: "Building in background...".to_string(),
        };

        let ctx = ExportContext {
            project_name: msg.project_name.clone(),
            project_dir: msg.project_dir.clone(),
            scene_path: msg.scene_path.clone(),
            output_dir: msg.output_dir.clone(),
            release: true,
        };
        let (tx, rx) = std::sync::mpsc::channel();
        job.name = exporter.name().to_string();
        job.started = Some(std::time::Instant::now());
        job.finished = None;
        job.rx = Some(std::sync::Mutex::new(rx));
        std::thread::spawn(move || {
            let _ = tx.send(exporter.export(&ctx));
        });
    }
}

fn poll_export_job(mut job: ResMut<ExportJob>, mut status: ResMut<ExportStatus>) {
    let result = match &job.rx {
        Some(rx) => match rx.lock().unwrap().try_recv() {
            Ok(r) => Some(r),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(_) => Some(Err("Export thread terminated unexpectedly".to_string())),
        },
        None => None,
    };
    let Some(result) = result else { return };
    let name = job.name.clone();
    job.rx = None;
    match result {
        Ok(res) => {
            info!("Export succeeded: {}", res.message);
            job.finished = Some((
                std::time::Instant::now(),
                true,
                res.message.clone(),
                Some(res.primary_artifact.clone()),
                res.zip_artifact.clone(),
            ));
            *status = ExportStatus::Success {
                target: name,
                message: res.message,
                path: res.primary_artifact,
            };
        }
        Err(err) => {
            error!("Export failed: {}", err);
            let short: String = err.lines().last().unwrap_or("error").chars().take(80).collect();
            job.finished = Some((
                std::time::Instant::now(),
                false,
                short,
                None,
                None,
            ));
            *status = ExportStatus::Failed { target: name, error: err };
        }
    }
}

fn update_export_progress_widget(
    job: Res<ExportJob>,
    time: Res<Time>,
    mut root: Query<&mut Node, (With<ExportProgressRoot>, Without<ExportProgressFill>, Without<ExportActionsRow>, Without<OpenZipButton>)>,
    mut fill: Query<(&mut Node, &mut BackgroundColor), (With<ExportProgressFill>, Without<ExportProgressRoot>, Without<ExportActionsRow>, Without<OpenZipButton>)>,
    mut label: Query<&mut Text, With<ExportProgressLabel>>,
    mut actions_row: Query<&mut Node, (With<ExportActionsRow>, Without<ExportProgressRoot>, Without<ExportProgressFill>, Without<OpenZipButton>)>,
    mut zip_btn_node: Query<&mut Node, (With<OpenZipButton>, Without<ExportProgressRoot>, Without<ExportProgressFill>, Without<ExportActionsRow>)>,
) {
    use crate::ui::theme::ACCENT_CYAN;
    let (Ok(mut root), Ok((mut fill_node, mut fill_bg)), Ok(mut text)) =
        (root.single_mut(), fill.single_mut(), label.single_mut())
    else {
        return;
    };

    if let (true, Some(started)) = (job.rx.is_some(), job.started) {
        root.display = Display::Flex;
        if let Ok(mut row) = actions_row.single_mut() {
            row.display = Display::None;
        }
        let secs = started.elapsed().as_secs();
        text.0 = format!("Building {}...  {}:{:02}", job.name, secs / 60, secs % 60);
        // Indeterminate sweeping bar animation
        let t = (time.elapsed_secs() * 0.7).fract();
        fill_node.left = percent(t * 100.0 - 30.0 * t);
        fill_node.width = percent(30.0);
        *fill_bg = BackgroundColor(ACCENT_CYAN);
    } else if let Some((at, ok, msg, _, zip_path)) = &job.finished {
        // Keep widget visible longer after build so user has time to click "Open Folder" or "Locate ZIP"
        if at.elapsed().as_secs_f32() > 25.0 {
            root.display = Display::None;
            if let Ok(mut row) = actions_row.single_mut() {
                row.display = Display::None;
            }
        } else {
            root.display = Display::Flex;
            fill_node.left = percent(0.0);
            fill_node.width = percent(100.0);
            *fill_bg = BackgroundColor(if *ok { Color::srgb(0.2, 0.8, 0.4) } else { Color::srgb(0.9, 0.3, 0.3) });
            text.0 = if *ok {
                format!("✅ {} build done!", job.name)
            } else {
                format!("❌ {} build FAILED: {msg}", job.name)
            };

            // Show action buttons if export was successful
            if let Ok(mut row) = actions_row.single_mut() {
                row.display = if *ok { Display::Flex } else { Display::None };
            }
            if let Ok(mut zip_node) = zip_btn_node.single_mut() {
                zip_node.display = if *ok && zip_path.is_some() { Display::Flex } else { Display::None };
            }
        }
    } else {
        root.display = Display::None;
        if let Ok(mut row) = actions_row.single_mut() {
            row.display = Display::None;
        }
    }
}

/// Helper system that opens the file explorer when clicking "Open Folder" or "Locate ZIP".
fn handle_export_action_clicks(
    job: Res<ExportJob>,
    folder_clicks: Query<&bevy::picking::events::Pointer<bevy::picking::events::PointerClick>, With<OpenFolderButton>>,
    zip_clicks: Query<&bevy::picking::events::Pointer<bevy::picking::events::PointerClick>, With<OpenZipButton>>,
) {
    let Some((_, ok, _, artifact_path, zip_path)) = &job.finished else { return };
    if !*ok { return };

    if !folder_clicks.is_empty() {
        if let Some(path) = artifact_path {
            reveal_in_file_manager(path);
        }
    }

    if !zip_clicks.is_empty() {
        if let Some(path) = zip_path {
            reveal_in_file_manager(path);
        }
    }
}

/// Opens the native OS file explorer pointing to the artifact.
pub fn reveal_in_file_manager(path: &std::path::Path) {
    let target = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer")
            .arg(target)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg(target)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(target)
            .spawn();
    }
}


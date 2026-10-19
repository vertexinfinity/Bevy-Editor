//! Lightweight performance counters for the editor (`editor/perf` over RPC).
//!
//! Counts how many UI components were modified in the last frame. In an idle editor these
//! should be zero: any steady non-zero count means a system rewrites UI every frame, which
//! forces Bevy to re-run UI layout and text shaping for nothing.
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct UiChurn {
    /// A few of the changed UI entities, for tracking down who rewrites them.
    pub samples: [Option<Entity>; 8],
    pub nodes: usize,
    pub texts: usize,
    pub backgrounds: usize,
    pub transforms: usize,
    pub frames: u64,
}

pub struct PerfPlugin;

impl Plugin for PerfPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        app.init_resource::<UiChurn>().add_systems(Last, count_churn);
    }
}

fn count_churn(
    mut churn: ResMut<UiChurn>,
    nodes: Query<Entity, Changed<Node>>,
    texts: Query<Entity, Changed<Text>>,
    backgrounds: Query<(), Changed<BackgroundColor>>,
    transforms: Query<(), (Changed<Transform>, With<Mesh3d>)>,
) {
    let mut samples = [None; 8];
    for (slot, entity) in samples.iter_mut().zip(nodes.iter().chain(texts.iter())) {
        *slot = Some(entity);
    }
    *churn = UiChurn {
        samples,
        nodes: nodes.iter().count(),
        texts: texts.iter().count(),
        backgrounds: backgrounds.iter().count(),
        transforms: transforms.iter().count(),
        frames: churn.frames + 1,
    };
}

/// Snapshot for the remote API.
pub fn perf_json(world: &mut World) -> serde_json::Value {
    let churn = *world.resource::<UiChurn>();
    let (fps, frame_ms) = world
        .get_resource::<DiagnosticsStore>()
        .map(|d| {
            (
                d.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|v| v.smoothed()),
                d.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME).and_then(|v| v.smoothed()),
            )
        })
        .unwrap_or((None, None));
    let entities = world.entities().count_spawned();
    let describe = |world: &World, entity: Entity| -> String {
        // Walk up to the nearest named ancestor so anonymous rows are attributable.
        let mut current = Some(entity);
        let mut path = Vec::new();
        while let Some(e) = current {
            if let Some(name) = world.get::<Name>(e) {
                path.push(name.as_str().to_string());
                break;
            }
            if let Some(text) = world.get::<Text>(e) {
                path.push(format!("text:{}", text.0.chars().take(24).collect::<String>()));
            }
            current = world.get::<ChildOf>(e).map(|c| c.parent());
        }
        path.join(" < ")
    };
    let samples: Vec<String> = churn.samples.iter().flatten().filter(|e| world.get_entity(**e).is_ok()).map(|e| describe(world, *e)).collect();
    let ui_nodes = world.query::<&Node>().iter(world).count();
    // Which panels own the most UI nodes (by nearest named ancestor).
    let mut owners: std::collections::HashMap<String, usize> = default();
    let node_entities: Vec<Entity> = world.query_filtered::<Entity, With<Node>>().iter(world).collect();
    for entity in node_entities {
        let mut current = Some(entity);
        let mut owner = String::from("(unnamed root)");
        while let Some(e) = current {
            if let Some(name) = world.get::<Name>(e) {
                owner = name.as_str().to_string();
            }
            current = world.get::<ChildOf>(e).map(|c| c.parent());
        }
        *owners.entry(owner).or_default() += 1;
    }
    let mut owners: Vec<_> = owners.into_iter().collect();
    owners.sort_by(|a, b| b.1.cmp(&a.1));
    owners.truncate(8);
    serde_json::json!({
        "fps": fps,
        "frame_time_ms": frame_ms,
        "entities": entities,
        "ui_nodes": ui_nodes,
        "changed_last_frame": {
            "nodes": churn.nodes,
            "texts": churn.texts,
            "background_colors": churn.backgrounds,
            "mesh_transforms": churn.transforms,
        },
        "frames": churn.frames,
        "changed_samples": samples,
        "ui_nodes_by_panel": owners,
    })
}

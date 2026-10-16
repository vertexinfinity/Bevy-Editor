//! Bevylite Lightmapper Plugin integrating CPU & GPU baking into Bevy scenes.

use bevy::asset::RenderAssetUsages;
use bevy::ecs::message::{Message, MessageReader};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::renderer::{RenderDevice, RenderQueue};

use crate::config::{LightmapBackend, LightmapConfig};
use crate::cpu::bake_lightmap_cpu;
use crate::extract::extract_bake_scene;
use crate::geometry::BakeScene;
use crate::gpu::bake_lightmap_gpu;

/// Message to trigger lightmap baking.
#[derive(Message, Clone, Debug, Default)]
pub struct BakeLightmapRequest {
    pub custom_config: Option<LightmapConfig>,
}

/// Real-time status and progress of the lightmap baking pipeline.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource, Default)]
pub struct LightmapBakeProgress {
    pub is_baking: bool,
    pub progress: f32, // 0.0 to 1.0
    pub status: String,
    pub last_baked_time_ms: u64,
}

impl Default for LightmapBakeProgress {
    fn default() -> Self {
        Self {
            is_baking: false,
            progress: 0.0,
            status: "Idle".to_string(),
            last_baked_time_ms: 0,
        }
    }
}

/// Stores handle of active baked lightmap texture.
#[derive(Resource, Clone, Debug, Default)]
pub struct ActiveLightmap {
    pub handle: Option<Handle<Image>>,
    pub resolution: u32,
    pub backend_used: Option<LightmapBackend>,
}

/// The main Bevylite Lightmapper plugin.
pub struct BevyliteLightmapperPlugin;

impl Plugin for BevyliteLightmapperPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightmapConfig>()
            .init_resource::<LightmapBakeProgress>()
            .init_resource::<ActiveLightmap>()
            .add_message::<BakeLightmapRequest>()
            .add_systems(Update, handle_bake_requests);
    }
}

/// Processes bake requests, running either GPU RTX compute or multithreaded CPU passes.
pub fn handle_bake_requests(
    mut commands: Commands,
    mut requests: MessageReader<BakeLightmapRequest>,
    config: Res<LightmapConfig>,
    mut progress: ResMut<LightmapBakeProgress>,
    mut active_lightmap: ResMut<ActiveLightmap>,
    mut images: ResMut<Assets<Image>>,
    mesh_query: Query<(Entity, &Mesh3d, &GlobalTransform, Option<&Name>)>,
    meshes: Res<Assets<Mesh>>,
    dir_lights: Query<(&DirectionalLight, &GlobalTransform)>,
    point_lights: Query<(&PointLight, &GlobalTransform)>,
    render_device: Option<Res<RenderDevice>>,
    render_queue: Option<Res<RenderQueue>>,
) {
    for req in requests.read() {
        let active_cfg = req.custom_config.as_ref().unwrap_or(&config).clone();
        progress.is_baking = true;
        progress.progress = 0.05;
        progress.status = format!("Extracting scene geometry for {} baking...", active_cfg.backend.label());

        let start_time = std::time::Instant::now();

        // 1. Extract scene geometry
        let scene: BakeScene = extract_bake_scene(&mesh_query, &meshes, &dir_lights, &point_lights);
        info!(
            "BakeScene extracted: {} triangles, {} meshes, {} lights",
            scene.triangles.len(),
            scene.meshes.len(),
            scene.lights.len()
        );

        if scene.triangles.is_empty() {
            warn!("No mesh triangles found to bake lightmaps for.");
            progress.is_baking = false;
            progress.status = "Failed: No 3D meshes in scene".to_string();
            return;
        }

        // 2. Execute Baking (GPU or CPU)
        let mut final_backend = active_cfg.backend;
        let baked_bytes_res: Result<Vec<u8>, String> = match active_cfg.backend {
            LightmapBackend::Gpu => {
                if let (Some(device), Some(queue)) = (&render_device, &render_queue) {
                    info!("Launching GPU RTX compute lightmap baker (resolution: {})...", active_cfg.resolution);
                    bake_lightmap_gpu(device, queue, &scene, &active_cfg, |pct, msg| {
                        debug!("[GPU Baker] {:.0}% - {}", pct * 100.0, msg);
                    })
                } else {
                    warn!("GPU RenderDevice not available, falling back to CPU baker");
                    final_backend = LightmapBackend::Cpu;
                    Ok(bake_lightmap_cpu(&scene, &active_cfg, |pct, msg| {
                        debug!("[CPU Baker Fallback] {:.0}% - {}", pct * 100.0, msg);
                    }))
                }
            }
            LightmapBackend::Cpu => {
                info!("Launching CPU multithreaded lightmap baker (resolution: {})...", active_cfg.resolution);
                Ok(bake_lightmap_cpu(&scene, &active_cfg, |pct, msg| {
                    debug!("[CPU Baker] {:.0}% - {}", pct * 100.0, msg);
                }))
            }
        };

        let baked_bytes = match baked_bytes_res {
            Ok(bytes) => bytes,
            Err(err) => {
                warn!("GPU baking encountered an error ({}). Falling back to CPU baker.", err);
                final_backend = LightmapBackend::Cpu;
                bake_lightmap_cpu(&scene, &active_cfg, |_, _| {})
            }
        };

        // 3. Construct Bevy Image
        let res = active_cfg.resolution;
        let image = Image::new_fill(
            Extent3d {
                width: res,
                height: res,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &baked_bytes,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );

        let image_handle = images.add(image);
        active_lightmap.handle = Some(image_handle.clone());
        active_lightmap.resolution = res;
        active_lightmap.backend_used = Some(final_backend);

        // 4. Attach Bevy native Lightmap component to scene entities
        let mut attached_count = 0;
        for target in &scene.meshes {
            commands.entity(target.entity).insert(bevy::pbr::Lightmap {
                image: image_handle.clone(),
                uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                bicubic_sampling: true,
            });
            attached_count += 1;
        }

        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        progress.is_baking = false;
        progress.progress = 1.0;
        progress.last_baked_time_ms = elapsed_ms;
        progress.status = format!(
            "Bake complete in {}ms using {}! Applied to {} meshes.",
            elapsed_ms,
            final_backend.label(),
            attached_count
        );

        info!("{}", progress.status);
    }
}

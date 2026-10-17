//! Bevylite Lightmapper Plugin integrating CPU & GPU baking into Bevy scenes.

use std::sync::mpsc::Receiver;
use std::time::Instant;

use bevy::asset::RenderAssetUsages;
use bevy::ecs::message::{Message, MessageReader};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::tasks::{AsyncComputeTaskPool, Task};

use crate::config::{LightmapBackend, LightmapConfig};
use crate::cpu::bake_lightmap_cpu;
use crate::extract::extract_bake_scene;
use crate::geometry::BakeScene;
use crate::gpu::{dispatch_gpu_bake, finish_gpu_bake, GpuBakeTaskState};

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
    pub completion_timer: f32,
}

impl Default for LightmapBakeProgress {
    fn default() -> Self {
        Self {
            is_baking: false,
            progress: 0.0,
            status: "Idle".to_string(),
            last_baked_time_ms: 0,
            completion_timer: 0.0,
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

/// In-flight baking job running asynchronously across frames.
#[derive(Resource)]
pub struct ActiveBakeJob {
    pub job: Option<BakeJobType>,
}

pub enum BakeJobType {
    Gpu {
        task_state: GpuBakeTaskState,
        scene: BakeScene,
        config: LightmapConfig,
        start_time: Instant,
    },
    Cpu {
        task: Task<Vec<u8>>,
        rx_progress: std::sync::Mutex<Receiver<(f32, String)>>,
        scene: BakeScene,
        config: LightmapConfig,
        start_time: Instant,
    },
}

/// The main Bevylite Lightmapper plugin.
pub struct BevyliteLightmapperPlugin;

impl Plugin for BevyliteLightmapperPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightmapConfig>()
            .init_resource::<LightmapBakeProgress>()
            .init_resource::<ActiveLightmap>()
            .add_message::<BakeLightmapRequest>()
            .add_systems(
                Update,
                (
                    ensure_new_meshes_have_uv1,
                    handle_bake_requests,
                    poll_active_bake_job,
                ),
            );
    }
}

/// Automatically ensures any newly spawned PBR mesh has `Mesh::ATTRIBUTE_UV_1` on Frame 0
/// so its GPU vertex buffer layout never has to reallocate when `Lightmap` is attached later.
pub fn ensure_new_meshes_have_uv1(
    new_meshes: Query<&Mesh3d, (Added<Mesh3d>, With<MeshMaterial3d<StandardMaterial>>)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for mesh_handle in new_meshes.iter() {
        let needs_uv1 = meshes
            .get(&mesh_handle.0)
            .is_some_and(|m| !m.contains_attribute(Mesh::ATTRIBUTE_UV_1));
        if needs_uv1 {
            if let Some(mut mesh_asset) = meshes.get_mut(&mesh_handle.0) {
                ensure_mesh_has_uv1(&mut mesh_asset);
            }
        }
    }
}

/// Triggers new bake jobs asynchronously when a request is received.
pub fn handle_bake_requests(
    mut commands: Commands,
    mut requests: MessageReader<BakeLightmapRequest>,
    config: Res<LightmapConfig>,
    mut progress: ResMut<LightmapBakeProgress>,
    mesh_query: Query<
        (Entity, &Mesh3d, &GlobalTransform, Option<&Name>),
        With<MeshMaterial3d<StandardMaterial>>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    dir_lights: Query<(&DirectionalLight, &GlobalTransform)>,
    point_lights: Query<(&PointLight, &GlobalTransform)>,
    render_device: Option<Res<RenderDevice>>,
    render_queue: Option<Res<RenderQueue>>,
    active_job: Option<Res<ActiveBakeJob>>,
) {
    for req in requests.read() {
        if active_job.is_some() || progress.is_baking {
            warn!("Bake already in progress. Ignoring new request.");
            continue;
        }

        let active_cfg = req.custom_config.as_ref().unwrap_or(&config).clone();
        progress.is_baking = true;
        progress.progress = 0.05;
        progress.completion_timer = 0.0;
        progress.status = format!("Extracting scene geometry for {} baking...", active_cfg.backend.label());

        // Ensure all target meshes have secondary UV channel (`ATTRIBUTE_UV_1`) before baking starts
        // so Bevy's RenderMesh vertex buffer layout includes `VERTEX_UVS_B` (`in.uv_b`) ahead of completion.
        for (_entity, mesh_handle, _, _) in mesh_query.iter() {
            let needs_uv1 = meshes
                .get(&mesh_handle.0)
                .is_some_and(|m| !m.contains_attribute(Mesh::ATTRIBUTE_UV_1));
            if needs_uv1 {
                if let Some(mut mesh_asset) = meshes.get_mut(&mesh_handle.0) {
                    ensure_mesh_has_uv1(&mut mesh_asset);
                }
            }
        }

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
            progress.completion_timer = 5.0;
            return;
        }

        match active_cfg.backend {
            LightmapBackend::Gpu => {
                if let (Some(device), Some(queue)) = (&render_device, &render_queue) {
                    info!("Dispatching GPU RTX compute lightmap baker (resolution: {})...", active_cfg.resolution);
                    match dispatch_gpu_bake(device, queue, &scene, &active_cfg) {
                        Ok(task_state) => {
                            commands.insert_resource(ActiveBakeJob {
                                job: Some(BakeJobType::Gpu {
                                    task_state,
                                    scene,
                                    config: active_cfg,
                                    start_time: Instant::now(),
                                }),
                            });
                            progress.progress = 0.2;
                            progress.status = "☀️ Baking (GPU RTX): Running compute shader...".to_string();
                        }
                        Err(err) => {
                            warn!("GPU dispatch failed: {}. Falling back to multithreaded CPU baker...", err);
                            spawn_cpu_bake(&mut commands, &mut progress, scene, active_cfg);
                        }
                    }
                } else {
                    warn!("GPU RenderDevice unavailable, falling back to CPU baker");
                    spawn_cpu_bake(&mut commands, &mut progress, scene, active_cfg);
                }
            }
            LightmapBackend::Cpu => {
                spawn_cpu_bake(&mut commands, &mut progress, scene, active_cfg);
            }
        }
    }
}

fn spawn_cpu_bake(
    commands: &mut Commands,
    progress: &mut LightmapBakeProgress,
    scene: BakeScene,
    config: LightmapConfig,
) {
    info!("Launching asynchronous multithreaded CPU lightmap baker (resolution: {})...", config.resolution);
    let (tx, rx) = std::sync::mpsc::channel();
    let scene_clone = scene.clone();
    let config_clone = config.clone();
    let task = AsyncComputeTaskPool::get().spawn(async move {
        bake_lightmap_cpu(&scene_clone, &config_clone, move |pct, msg| {
            let _ = tx.send((pct, msg.to_string()));
        })
    });

    commands.insert_resource(ActiveBakeJob {
        job: Some(BakeJobType::Cpu {
            task,
            rx_progress: std::sync::Mutex::new(rx),
            scene,
            config,
            start_time: Instant::now(),
        }),
    });
    progress.progress = 0.05;
    progress.status = "☀️ Baking (CPU): Multi-threaded ray tracing...".to_string();
}

/// Polls active bake jobs non-blockingly each frame, updating progress and attaching lightmaps on finish.
pub fn poll_active_bake_job(
    mut commands: Commands,
    time: Res<Time>,
    active_job: Option<ResMut<ActiveBakeJob>>,
    mut progress: ResMut<LightmapBakeProgress>,
    mut active_lightmap: ResMut<ActiveLightmap>,
    mut images: ResMut<Assets<Image>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    render_device: Option<Res<RenderDevice>>,
    mut meshes: Query<&mut Mesh3d>,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    // Tick down completion banner timer
    if progress.completion_timer > 0.0 {
        progress.completion_timer = (progress.completion_timer - time.delta_secs()).max(0.0);
    }

    let Some(mut job_res) = active_job else {
        return;
    };

    let Some(mut job) = job_res.job.take() else {
        commands.remove_resource::<ActiveBakeJob>();
        return;
    };

    match &mut job {
        BakeJobType::Gpu { task_state, scene: _, config: _, start_time } => {
            if let Some(device) = &render_device {
                let _ = device.poll(bevy::render::render_resource::PollType::Poll);
            }

            let elapsed = start_time.elapsed().as_secs_f32();
            let is_ready = task_state.receiver.lock().map(|r| r.try_recv().is_ok()).unwrap_or(false);

            if is_ready {
                // Compute pass mapped! Readback and post-process
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                progress.status = "Applying lightmap dilation & denoising...".to_string();
                progress.progress = 0.9;

                commands.remove_resource::<ActiveBakeJob>();

                let BakeJobType::Gpu { task_state, scene, config, .. } = job else { unreachable!() };
                match finish_gpu_bake(task_state, |pct, msg| {
                    debug!("[GPU Baker Finish] {:.0}% - {}", pct * 100.0, msg);
                }) {
                    Ok(baked_bytes) => {
                        apply_baked_lightmap(
                            &mut commands,
                            &mut active_lightmap,
                            &mut images,
                            &mut mesh_assets,
                            &mut material_assets,
                            &scene,
                            config.resolution,
                            LightmapBackend::Gpu,
                            baked_bytes,
                            &mut meshes,
                            &mut materials,
                        );
                        progress.is_baking = false;
                        progress.progress = 1.0;
                        progress.last_baked_time_ms = elapsed_ms;
                        progress.completion_timer = 6.0;
                        progress.status = format!(
                            "✅ Lightmap baked in {}ms (GPU RTX) — {} meshes updated",
                            elapsed_ms,
                            scene.meshes.len()
                        );
                        info!("{}", progress.status);
                    }
                    Err(err) => {
                        error!("GPU lightmap finish error: {}", err);
                        progress.is_baking = false;
                        progress.status = format!("❌ GPU Bake Error: {}", err);
                        progress.completion_timer = 6.0;
                    }
                }
            } else if elapsed > 60.0 {
                // Timeout after 60 seconds
                commands.remove_resource::<ActiveBakeJob>();
                progress.is_baking = false;
                progress.status = "❌ GPU Bake Error: Timed out after 60s".to_string();
                progress.completion_timer = 6.0;
            } else {
                // Keep job alive and animate progress
                let pct = (0.2 + (elapsed / 3.0) * 0.65).min(0.85);
                progress.progress = pct;
                progress.status = format!("☀️ Baking (GPU RTX compute): {:.1}s elapsed...", elapsed);
                job_res.job = Some(job);
            }
        }
        BakeJobType::Cpu { task, rx_progress, scene: _, config: _, start_time } => {
            if let Ok(rx) = rx_progress.lock() {
                while let Ok((pct, msg)) = rx.try_recv() {
                    progress.progress = pct;
                    progress.status = format!("☀️ Baking (CPU): {:.0}% - {}", pct * 100.0, msg);
                }
            }

            if task.is_finished() {
                let elapsed_ms = start_time.elapsed().as_millis() as u64;
                commands.remove_resource::<ActiveBakeJob>();

                let BakeJobType::Cpu { mut task, scene, config, .. } = job else { unreachable!() };
                if let Some(baked_bytes) = bevy::tasks::block_on(bevy::tasks::poll_once(&mut task)) {
                    apply_baked_lightmap(
                        &mut commands,
                        &mut active_lightmap,
                        &mut images,
                        &mut mesh_assets,
                        &mut material_assets,
                        &scene,
                        config.resolution,
                        LightmapBackend::Cpu,
                        baked_bytes,
                        &mut meshes,
                        &mut materials,
                    );
                    progress.is_baking = false;
                    progress.progress = 1.0;
                    progress.last_baked_time_ms = elapsed_ms;
                    progress.completion_timer = 6.0;
                    progress.status = format!(
                        "✅ Lightmap baked in {}ms (CPU) — {} meshes updated",
                        elapsed_ms,
                        scene.meshes.len()
                    );
                    info!("{}", progress.status);
                }
            } else {
                // Keep job alive
                job_res.job = Some(job);
            }
        }
    }
}

fn apply_baked_lightmap(
    commands: &mut Commands,
    active_lightmap: &mut ResMut<ActiveLightmap>,
    images: &mut ResMut<Assets<Image>>,
    mesh_assets: &mut ResMut<Assets<Mesh>>,
    material_assets: &mut ResMut<Assets<StandardMaterial>>,
    scene: &BakeScene,
    resolution: u32,
    backend: LightmapBackend,
    baked_bytes: Vec<u8>,
    meshes: &mut Query<&mut Mesh3d>,
    materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let mut image = Image::new_fill(
        Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &baked_bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.sampler = bevy::image::ImageSampler::linear();

    let image_handle = images.add(image);
    active_lightmap.handle = Some(image_handle.clone());
    active_lightmap.resolution = resolution;
    active_lightmap.backend_used = Some(backend);

    for target in &scene.meshes {
        // 1. Ensure the Mesh asset has `Mesh::ATTRIBUTE_UV_1` so Bevy's PBR shader defines `VERTEX_UVS_B`.
        // Only call `get_mut` when `ATTRIBUTE_UV_1` is missing so we don't emit spurious Mesh asset modification events.
        if let Ok(mut mesh_handle) = meshes.get_mut(target.entity) {
            let needs_uv1 = mesh_assets
                .get(&mesh_handle.0)
                .is_some_and(|m| !m.contains_attribute(Mesh::ATTRIBUTE_UV_1));
            if needs_uv1 {
                if let Some(mut mesh_asset) = mesh_assets.get_mut(&mesh_handle.0) {
                    ensure_mesh_has_uv1(&mut mesh_asset);
                }
            }
            mesh_handle.set_changed();
        }
        // 2. Set `lightmap_exposure` on the StandardMaterial to match Bevy's physical camera exposure (EV100 = 9.7).
        if let Ok(mut mat) = materials.get_mut(target.entity) {
            if let Some(mut mat_asset) = material_assets.get_mut(&mat.0) {
                mat_asset.lightmap_exposure = 600.0;
            }
            mat.set_changed();
        }

        commands.entity(target.entity).insert(bevy::pbr::Lightmap {
            image: image_handle.clone(),
            uv_rect: target.uv_rect,
            bicubic_sampling: true,
        });
    }
}

fn ensure_mesh_has_uv1(mesh_asset: &mut Mesh) {
    if !mesh_asset.contains_attribute(Mesh::ATTRIBUTE_UV_1) {
        let uv1_data = if let Some(bevy::render::mesh::VertexAttributeValues::Float32x2(uv0)) =
            mesh_asset.attribute(Mesh::ATTRIBUTE_UV_0)
        {
            Some(uv0.clone())
        } else if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(pos)) =
            mesh_asset.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            Some(vec![[0.0, 0.0]; pos.len()])
        } else {
            None
        };

        if let Some(uv1) = uv1_data {
            mesh_asset.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv1);
        }
    }
}

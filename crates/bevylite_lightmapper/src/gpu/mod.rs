//! Hardware-accelerated GPU lightmap baker using WGPU Compute Shaders.
//! Dispatches thousands of concurrent ray tracing operations across NVIDIA RTX / Vulkan / DX12 compute units.

use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bytemuck::{Pod, Zeroable};

use crate::config::LightmapConfig;
use crate::geometry::BakeScene;
use crate::postprocess::{multi_pass_dilate, smooth_lightmap};

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct GpuBakeParams {
    resolution: u32,
    triangle_count: u32,
    light_count: u32,
    samples_per_pixel: u32,
    sky_color: [f32; 3],
    direct_multiplier: f32,
    ao_intensity: f32,
    normal_bias: f32,
    _pad: [f32; 2],
}

/// Executes GPU lightmap baking via WGPU compute shaders.
pub fn bake_lightmap_gpu(
    device: &RenderDevice,
    queue: &RenderQueue,
    scene: &BakeScene,
    config: &LightmapConfig,
    mut progress_callback: impl FnMut(f32, &str),
) -> Result<Vec<u8>, String> {
    progress_callback(0.1, "Initializing GPU compute pipeline...");

    let shader_src = include_str!("lightmap_bake.wgsl");
    let shader = unsafe {
        device.create_shader_module(ShaderModuleDescriptor {
            label: Some("lightmap_bake_shader"),
            source: ShaderSource::Wgsl(shader_src.into()),
        })
    };

    let res = config.resolution;
    let total_pixels = (res * res) as usize;
    let pixel_byte_size = total_pixels * 16; // vec4<f32>

    // 1. Uniform Params
    let params_data = GpuBakeParams {
        resolution: res,
        triangle_count: scene.triangles.len() as u32,
        light_count: scene.lights.len() as u32,
        samples_per_pixel: config.samples_per_pixel,
        sky_color: config.sky_color,
        direct_multiplier: config.direct_light_multiplier,
        ao_intensity: config.ao_intensity,
        normal_bias: config.normal_bias,
        _pad: [0.0; 2],
    };

    let params_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("lightmap_params_buffer"),
        contents: bytemuck::bytes_of(&params_data),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    // 2. Storage Triangles Buffer
    let dummy_triangle = [crate::geometry::LightmapTriangle::zeroed()];
    let triangles_bytes = if scene.triangles.is_empty() {
        bytemuck::cast_slice(&dummy_triangle)
    } else {
        bytemuck::cast_slice(&scene.triangles)
    };
    let triangles_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("lightmap_triangles_buffer"),
        contents: triangles_bytes,
        usage: BufferUsages::STORAGE,
    });

    // 3. Storage Lights Buffer
    let dummy_light = [crate::geometry::GpuLight::zeroed()];
    let lights_bytes = if scene.lights.is_empty() {
        bytemuck::cast_slice(&dummy_light)
    } else {
        bytemuck::cast_slice(&scene.lights)
    };
    let lights_buffer = device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("lightmap_lights_buffer"),
        contents: lights_bytes,
        usage: BufferUsages::STORAGE,
    });

    // 4. Storage Output Pixels Buffer (vec4<f32> per pixel)
    let output_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("lightmap_output_buffer"),
        size: pixel_byte_size as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // 5. Staging Readback Buffer
    let staging_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("lightmap_staging_buffer"),
        size: pixel_byte_size as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // 6. Bind Group Layout
    let bind_group_layout = device.create_bind_group_layout(
        Some("lightmap_compute_bind_group_layout"),
        &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    );

    let bind_group = device.create_bind_group(
        Some("lightmap_compute_bind_group"),
        &bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: triangles_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: lights_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    );

    // 7. Pipeline Layout & Compute Pipeline
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("lightmap_compute_pipeline_layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_compute_pipeline(&RawComputePipelineDescriptor {
        label: Some("lightmap_compute_pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: default(),
        cache: None,
    });

    progress_callback(0.3, "Dispatching GPU RTX compute workgroups...");

    // 8. Command Encoder & Compute Pass
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("lightmap_bake_encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("lightmap_bake_compute_pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroups_x = (res + 7) / 8;
        let workgroups_y = (res + 7) / 8;
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    // Copy result to staging buffer for CPU readback
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, pixel_byte_size as u64);
    queue.submit(Some(encoder.finish()));

    progress_callback(0.7, "Reading back GPU texture buffers...");

    // 9. Readback GPU Staging Buffer
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(MapMode::Read, move |v| {
        let _ = sender.send(v);
    });

    // Poll the GPU until mapping completes
    let mut poll_count = 0;
    while receiver.try_recv().is_err() {
        let _ = device.poll(PollType::Poll);
        std::thread::yield_now();
        poll_count += 1;
        if poll_count > 50000 {
            return Err("GPU compute shader execution timed out after 50000 polls".to_string());
        }
    }

    let data = buffer_slice.get_mapped_range().map_err(|e| format!("{:?}", e))?;
    let floats: &[f32] = bytemuck::cast_slice(&data);

    // Extract RGB channels
    let mut float_rgb = vec![0.0f32; total_pixels * 3];
    for i in 0..total_pixels {
        float_rgb[i * 3] = floats[i * 4];
        float_rgb[i * 3 + 1] = floats[i * 4 + 1];
        float_rgb[i * 3 + 2] = floats[i * 4 + 2];
    }
    drop(data);
    staging_buffer.unmap();

    progress_callback(0.85, "Dilating UV seam borders...");
    let dilated = multi_pass_dilate(float_rgb, res as usize, res as usize, 3, config.dilation_passes);

    progress_callback(0.92, "Applying denoise filter...");
    let smoothed = smooth_lightmap(&dilated, res as usize, res as usize, 3);

    progress_callback(0.98, "Packing into texture...");
    let mut rgba_bytes = vec![0u8; total_pixels * 4];
    for i in 0..total_pixels {
        let r = (smoothed[i * 3].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (smoothed[i * 3 + 1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (smoothed[i * 3 + 2].clamp(0.0, 1.0) * 255.0) as u8;
        rgba_bytes[i * 4] = r;
        rgba_bytes[i * 4 + 1] = g;
        rgba_bytes[i * 4 + 2] = b;
        rgba_bytes[i * 4 + 3] = 255;
    }

    progress_callback(1.0, "GPU baking complete!");
    Ok(rgba_bytes)
}

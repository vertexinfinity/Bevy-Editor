//! Lightmapping configuration and backend selection.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Backend used for lightmap baking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default, Reflect)]
#[reflect(Default)]
pub enum LightmapBackend {
    /// Pure Rust multithreaded CPU ray-tracer & progressive interpolator.
    /// Highly deterministic, portable, and runs on all systems.
    Cpu,
    /// Hardware-accelerated GPU WGPU compute shader baking.
    /// Utilizes RTX/Vulkan/DX12 compute cores for ultra-fast parallel ray casting.
    #[default]
    Gpu,
}

impl LightmapBackend {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Cpu => "CPU (Multithreaded)",
            Self::Gpu => "GPU (RTX / WGPU Compute)",
        }
    }
}

/// Settings and quality parameters for baking lightmaps.
#[derive(Resource, Clone, Debug, Serialize, Deserialize, Reflect)]
#[reflect(Resource, Default)]
pub struct LightmapConfig {
    /// Active backend (CPU or GPU).
    pub backend: LightmapBackend,

    /// Width and height of the baked lightmap texture (typically 256, 512, 1024, or 2048).
    pub resolution: u32,

    /// Number of hemisphere rays sampled per texel for Ambient Occlusion and indirect light.
    pub samples_per_pixel: u32,

    /// Number of hierarchical progressive passes (inspired by ands/lightmapper).
    /// Pass count $N$ evaluates grid steps from $2^N$ down to 1, skipping flat areas.
    pub progressive_passes: u32,

    /// Variance error threshold below which texels are interpolated from neighbors instead of traced.
    pub interpolation_threshold: f32,

    /// Number of dilation passes to expand borders across UV seams (prevents dark edge bleeding).
    pub dilation_passes: u32,

    /// Sky / clear ambient color integrated across unoccluded hemisphere angles.
    pub sky_color: [f32; 3],

    /// Multiplier for direct light shadows and intensity.
    pub direct_light_multiplier: f32,

    /// Strength of Ambient Occlusion darkening.
    pub ao_intensity: f32,

    /// Surface offset bias to avoid self-shadowing acne.
    pub normal_bias: f32,
}

impl Default for LightmapConfig {
    fn default() -> Self {
        Self::standard()
    }
}

impl LightmapConfig {
    /// Fast draft preset for rapid interactive iteration (takes ~1-2 seconds).
    pub fn fast() -> Self {
        Self {
            backend: LightmapBackend::Gpu,
            resolution: 256,
            samples_per_pixel: 16,
            progressive_passes: 2,
            interpolation_threshold: 0.04,
            dilation_passes: 2,
            sky_color: [0.18, 0.22, 0.28],
            direct_light_multiplier: 1.0,
            ao_intensity: 1.0,
            normal_bias: 0.005,
        }
    }

    /// Standard production quality preset balancing speed and smoothness.
    pub fn standard() -> Self {
        Self {
            backend: LightmapBackend::Gpu,
            resolution: 512,
            samples_per_pixel: 48,
            progressive_passes: 3,
            interpolation_threshold: 0.02,
            dilation_passes: 3,
            sky_color: [0.20, 0.24, 0.32],
            direct_light_multiplier: 1.0,
            ao_intensity: 1.0,
            normal_bias: 0.005,
        }
    }

    /// High quality preset with dense sampling and 1024x1024 resolution.
    pub fn high() -> Self {
        Self {
            backend: LightmapBackend::Gpu,
            resolution: 1024,
            samples_per_pixel: 96,
            progressive_passes: 4,
            interpolation_threshold: 0.015,
            dilation_passes: 4,
            sky_color: [0.20, 0.24, 0.32],
            direct_light_multiplier: 1.0,
            ao_intensity: 1.0,
            normal_bias: 0.004,
        }
    }
}

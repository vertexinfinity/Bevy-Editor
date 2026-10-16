//! # Bevylite Lightmapper
//!
//! A modular, dual-mode (CPU & GPU RTX WGPU Compute) lightmap baker for Bevy scenes.
//! Directly inspired by and translated from Andreas Mantler's `ands/lightmapper`.
//!
//! ## Features
//! - **CPU Mode**: Pure Rust multithreaded progressive hierarchical baker with conservative UV clipping.
//! - **GPU Mode**: High-performance WGPU compute shader ray tracer executing thousands of rays concurrently on RTX / Vulkan / DX12.
//! - **Post-Processing**: Dilation filter to bleed colors across UV seam borders, plus box denoising.
//! - **Bevy Native Integration**: Attaches `bevy::pbr::Lightmap` directly to entities in the scene for immediate real-time rendering.

pub mod config;
pub mod cpu;
pub mod extract;
pub mod geometry;
pub mod gpu;
pub mod plugin;
pub mod postprocess;

pub use config::{LightmapBackend, LightmapConfig};
pub use extract::extract_bake_scene;
pub use geometry::{BakeScene, LightmapTriangle, LightmapVertex, SceneMeshTarget};
pub use plugin::{
    ActiveLightmap, BakeLightmapRequest, BevyliteLightmapperPlugin, LightmapBakeProgress,
};
pub use postprocess::{dilate_lightmap, multi_pass_dilate, smooth_lightmap};

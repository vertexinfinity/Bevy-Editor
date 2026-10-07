# Known Defects & Resolved Issues

> Confidence: 🟢 Confirmed

## Resolved Defects

### 1. glTF Embedded JPEG Texture Loading Failure
- **Symptom**: `Failed to load asset 'models/....glb' with asset loader 'bevy_gltf::loader::GltfLoader': You may need to add the feature for the file format: invalid image mime type: image/jpeg`
- **Cause**: Bevy's default image feature set only enables PNG and HDR by default (`common_api`). glTF models with embedded JPEG textures (or WebP, KTX2) require explicit image format features on the `bevy` crate dependency.
- **Resolution**: Enabled `jpeg`, `png`, `webp`, `bmp`, `tga`, `ktx2`, and `dds` features under `bevy` dependency in [`Cargo.toml`](file:///D:/Repos/Bevy/Bevy-Editor/Cargo.toml).
- **Date**: 2026-10-07

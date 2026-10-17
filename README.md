# Bevy Editor (Bevylite)

A minimal, modular, lightning-fast 3D Scene Editor, GPU Lightmapper, and Multi-Platform Exporter built for [Bevy Engine](https://bevy.org) (`v0.20-dev`).

[![Release](https://img.shields.io/github/v/release/vertexinfinity/Bevy-Editor?include_prereleases&label=latest%20release&color=00e5ff)](https://github.com/vertexinfinity/Bevy-Editor/releases)
[![Build Status](https://img.shields.io/github/actions/workflow/status/vertexinfinity/Bevy-Editor/release.yml?branch=main&label=build)](https://github.com/vertexinfinity/Bevy-Editor/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Bevy Compatibility](https://img.shields.io/badge/Bevy-0.20--dev-fd971f.svg)](https://bevy.org)

---

## 📥 Download Pre-Built Binaries

Pre-compiled standalone packages are automatically built for Windows, Linux, and macOS:

* **[Download Latest Stable Release](https://github.com/vertexinfinity/Bevy-Editor/releases/latest)** (Production-ready tagged builds)
* **[Browse All Releases & Nightlies](https://github.com/vertexinfinity/Bevy-Editor/releases)**

| Platform | Target Architecture | Package |
| :--- | :--- | :--- |
| 🪟 **Windows** | `x86_64-pc-windows-msvc` | `.zip` (Executable + bundled assets) |
| 🐧 **Linux** | `x86_64-unknown-linux-gnu` | `.tar.gz` (Standalone ELF + assets) |
| 🍎 **macOS (Apple Silicon)** | `aarch64-apple-darwin` (M1/M2/M3/M4) | `.tar.gz` |
| 🍏 **macOS (Intel)** | `x86_64-apple-darwin` | `.tar.gz` |

---

## 💡 About Bevylite & Relationship to Bevy

### Origin Story
> *When I first began working with Bevy, I was searching for an intuitive, standalone visual scene editor and level designer. At the time, I wasn't aware of other editor initiatives in the broader ecosystem, so I started building one from scratch to solve my own workflow needs. By the time I discovered other projects, Bevylite had already evolved into a tailored, ultra-fast tool with distinct architecture choices—so I decided to continue its development and share it with the community.*

### Why Bevylite? What sets it apart?
Bevylite is an independent, community-driven project designed with specific philosophies:
1. **Always Tracking Bleeding-Edge Bevy (`0.20-dev`)**: Rather than lagging behind stable releases, Bevylite actively tracks Bevy `main` so you can use next-generation features (like modern `bevy_feathers` UI tokens, `Solari` real-time ray tracing, and Bevy PBR lightmapping) today.
2. **Built-in Dual GPU RTX & CPU Lightmapper (`bevylite_lightmapper`)**: Features an integrated sub-second WGPU compute shader ray tracer that bakes ambient occlusion and soft contact shadows directly into native `bevy::pbr::Lightmap` assets.
3. **Headless & Remote Pair-Programming (JSON-RPC 2.0 & `stdin`)**: Bevylite runs an always-on remote automation server (`http://127.0.0.1:15703`). AI agents (MCP, Gemini, Claude, ChatGPT), CI runners, or custom scripts can inspect the scene tree, spawn objects, tune materials, bake lightmaps, and capture screenshots programmatically.
4. **Turnkey Multi-Platform Exporter**: Pluggable export pipeline that packages and generates deployable projects for Windows, Linux, Web/WASM, Android APK, and iOS Xcode.
5. **Fast Modular Tiers**: Decoupled feature flags allowing you to compile a stripped-down ~120-crate viewer (`--features lite`) up to a full production studio suite (`--features "remote,compressed_textures,solari"`).

*(Note: Bevylite is an independent open-source tool and is not the official Bevy editor project; it is built to empower creators using Bevy Engine).*

---

## Key Features

### 🎮 3D Viewport & Scene Authoring
- **First-Person Fly Camera**: Smooth WASD movement, right-click mouselook, Q/E vertical elevation, scroll-wheel speed scaling, and Shift turbo boost.
- **Native Entity & Component Inspector**: Uses Bevy's official `bevy_inspector` and `bevy_feathers` dark theme tokens, providing real-time entity tree inspection and live component editing.
- **Project Explorer & Asset Browser**: Built-in asset panel browsing `assets/` (3D models, textures, scenes, audio) with single-click spawning.
- **Drag-and-Drop 3D Models**: Uses Bevy's native `FileDragAndDrop`. Drag any `.glb`, `.gltf`, or `.ron` file directly from your file manager into the editor window to spawn it immediately.
- **Scene Serialization with Sidecar Manifest**: Save and reload scenes (`.scn.ron` + `.bevylite.json`) preserving custom primitive geometry, shapes, PBR materials, transforms, and lights.

---

### ☀️ Baked Illumination Engine (`bevylite_lightmapper`)
Bevylite includes a built-in, modular dual-mode lightmap baker inspired by Andreas Mantler's `ands/lightmapper`:
- **⚡ Hardware-Accelerated GPU Baking (RTX / WGPU Compute)**:
  - Dispatches thousands of concurrent ray tracing operations across NVIDIA RTX / Vulkan / DX12 compute units.
  - Bakes production-quality 512×512 lightmaps in **~15ms to 200ms**.
- **🖥️ Pure Rust Multithreaded CPU Baker**:
  - Progressive hierarchical sampling ($2^N$ down to 1), conservative UV triangle clipping, and neighbor interpolation.
  - 100% portable, deterministic fallback running on any hardware without compute shader requirements.
- **High-Fidelity Lighting**:
  - Direct sunlight and point light shadows via fast Möller-Trumbore ray-triangle intersection tests.
  - Cosine-weighted hemisphere ray sampling for Ambient Occlusion (AO) and sky bounce light.
  - Multi-pass UV seam dilation to eliminate edge bleeding and box denoising filter.
- **Native Bevy PBR Integration**:
  - Automatically provisions secondary lightmap UV channels (`ATTRIBUTE_UV_1`).
  - Assigns per-mesh UV atlas tiles (`uv_rect`) and attaches `bevy::pbr::Lightmap` directly to entities.
  - Compensates for Bevy's physical camera exposure (`EV100 = 9.7`) for vibrant real-time rendering.
- **Live Status Bar Widget**: Dedicated dynamic cyan progress bar with real-time percentage readouts and completion alerts.

---

### 🤖 Remote Automation Server (JSON-RPC 2.0 & `stdin` Console)
Bevylite features an always-on remote automation bridge running concurrently over HTTP (`http://127.0.0.1:15703`) and standard input (`stdin`). This allows AI agents (MCP), CI/CD pipelines, and scripts to control the editor programmatically:

| RPC Method | Console Command | Description |
| :--- | :--- | :--- |
| `editor/status` | `status` | Returns editor state, FPS, entity count, selection, and active bake progress. |
| `editor/enter_editor` | `enter_editor` | Transitions from the Project Hub into the active 3D Editor workspace. |
| `editor/scene_tree` | `tree` | Lists all scene entities with IDs, names, transforms, materials, and lightmaps. |
| `editor/select` | `select <id\|name>` | Sets the active inspector selection. |
| `editor/spawn_primitive` | `spawn <kind> [name] [x y z]` | Spawns `cube`, `sphere`, `cylinder`, `plane`, `point_light`, or `directional_light`. |
| `editor/set_transform` | `transform <id> ...` | Updates position, euler rotation, and scale. |
| `editor/set_material` | `material <id> ...` | Modifies base color, metallic, roughness, and unlit flags. |
| `editor/delete_entity` | `delete <id>` | Despawns a scene entity. |
| `editor/bake` | `bake [gpu\|cpu] [res] [spp]` | Triggers asynchronous lightmap baking with custom resolution and sample counts. |
| `editor/clear_lightmaps` | `clear_lightmaps` | Strips baked lightmaps from meshes and restores dynamic shading. |
| `editor/save_scene` | `save [path]` | Serializes scene and sidecar manifest to disk. |
| `editor/load_scene` | `load [path]` | Restores scene geometry, materials, and hierarchy from disk. |
| `editor/clear_scene` | `clear` | Clears all user scene entities. |
| `editor/screenshot` | `screenshot [path]` | Captures viewport framebuffer directly to PNG (or press `F12`). |
| `editor/play_mode` | `play` / `pause` / `stop` | Toggles simulation play mode. |
| `editor/exit` | `exit` / `quit` | Gracefully closes the editor. |

---

### 📦 Multi-Platform Exporter
Pluggable export architecture in `src/export/`:
- 🪟 **Windows**: Standalone optimized executable packaging with bundled assets.
- 🐧 **Linux**: Standalone ELF executable build with asset bundling.
- 🌐 **Web**: WebAssembly (WASM) / HTML5 canvas deployment.
- 🤖 **Android**: Direct `cargo-apk` build or turnkey Gradle project scaffolding.
- 🍎 **iOS**: Xcode project scaffolding for App Store and TestFlight deployment.

---

## Getting Started

### Prerequisites
- [Rust](https://rustup.rs/) (2024 edition or latest stable/nightly)
- GPU supporting Vulkan, DirectX 12, or Metal (for RTX compute lightmapper)

### Launching the Editor
```bash
# Clone the repository
git clone https://github.com/vertexinfinity/Bevy-Editor.git
cd Bevy-Editor

# Launch the visual 3D editor
cargo run

# Or launch with ultra-fast developer dynamic linking (~1-2s re-links)
cargo run --features dynamic_linking
```

### Viewport Controls
| Input | Action |
| :--- | :--- |
| **Right Click (Hold)** | Look around (first-person mouselook) |
| **W / A / S / D** | Fly forward / left / backward / right (while holding RMB) |
| **Q / E** | Fly down / up (while holding RMB) |
| **Left Shift** | Turbo speed boost |
| **Scroll Wheel** | Adjust fly navigation speed |
| **F** | Focus viewport camera on selected object |
| **F12** | Take instant screenshot to `screenshots/` |
| **Drag & Drop** | Drop `.glb`, `.gltf`, or `.ron` files directly from OS into viewport |

---

## Build Profiles (Feature Tiers)

You can customize compilation footprint depending on workflow:

| Profile | Command | Included Features |
| :--- | :--- | :--- |
| **⚡ Minimal Lite** | `cargo run --no-default-features --features lite` | Lightweight 3D scene viewer, camera controller, basic primitives |
| **◈ Standard (Default)** | `cargo run` | Feathers dark UI, Scene Hierarchy, Inspector, 3D Transform Gizmos, glTF |
| **🚀 Fast Dev Iteration** | `cargo run --features dynamic_linking` | Shared dynamic engine linking (~1.5s re-links) |
| **🛠️ Pro / Full** | `cargo run --features "remote,compressed_textures,solari"` | High-res compressed textures (KTX2/DDS), Solari real-time ray tracing |

---

## License

This project is dual-licensed under either of:

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))
- **MIT license** ([LICENSE-MIT](LICENSE-MIT))

at your option.

This dual licensing strictly mirrors [Bevy Engine's license](https://github.com/bevyengine/bevy#license), ensuring complete, frictionless compatibility for open-source and commercial game development alike.

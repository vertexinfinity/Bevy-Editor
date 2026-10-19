# Bevy Editor (Bevylite)

## Bundled editor showcase

Open **Editor Showcase** in the project hub to explore a nested courtyard scene,
linked gate/switch/trigger entities, 25 PBR samples, lighting, scripts and a hidden
64-entity hierarchy branch. See the [sample and QA checklist](samples/editor_showcase/README.md).
Create a disposable copy with `cargo run -- new MyShowcase --template editor_showcase --output D:\Projects`.

A minimal, modular, lightning-fast 3D Scene Editor, GPU Lightmapper, and Multi-Platform Exporter built for [Bevy Engine](https://bevy.org) (`v0.20-dev`).

[![Latest Release](https://img.shields.io/github/v/release/vertexinfinity/Bevy-Editor?include_prereleases&label=release&color=00e5ff)](https://github.com/vertexinfinity/Bevy-Editor/releases/latest)
[![CI Tests](https://img.shields.io/github/actions/workflow/status/vertexinfinity/Bevy-Editor/ci.yml?branch=main&label=tests)](https://github.com/vertexinfinity/Bevy-Editor/actions/workflows/ci.yml)
[![Release Builds](https://img.shields.io/github/actions/workflow/status/vertexinfinity/Bevy-Editor/release.yml?label=builds)](https://github.com/vertexinfinity/Bevy-Editor/actions/workflows/release.yml)
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
| 🍎 **macOS (Apple Silicon)** | `aarch64-apple-darwin` (M-Series) | `.tar.gz` |

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
- **Drag-and-Drop 3D Models**: Drag `.glb`/`.gltf` (native), `.fbx` (ufbx), `.obj` + `.mtl` (tobj), `.stl` or `.ply` files, or `.ron` scenes, from your file manager into the editor window. Files are copied into `assets/models/` and spawned; non-glTF models are parsed in the background and rebuilt from the file when a scene loads.
- **Undo / Redo everywhere**: Ctrl+Z / Ctrl+Y (or Ctrl+Shift+Z) undo gizmo moves (one step per drag), inspector edits, added/removed components, material changes, spawns, duplicates and deletes (Delete key or Del button).
- **Asset Browser grid view**: Files show as an icon grid with image thumbnails by default; the header button switches to the compact list.
- **Reflection-Driven Inspector**: A generic **Components** card renders every reflected component, using `bevy_annotations` (`DisplayName`, `Tooltip`, `ValueRange`, `Step`, `Unit`, `ReadOnly`) for labels, sliders limits, units and help text. Booleans and enums toggle on click, numbers have −/+ steppers (Shift = 10×), and every edit is undoable.
- **Camera Rendering Card**: HDR, bloom, auto exposure, SSAO, SSR, depth of field, chromatic aberration, vignette, sharpening, anti-aliasing (MSAA/FXAA/SMAA/TAA) and tonemapping for scene cameras, with *Preview in viewport* to see them while editing.
- **World & Environment**: Background color, ambient light, physical atmosphere sky, distance fog and volumetric fog, saved with the scene. Shown in the Inspector when nothing is selected.
- **Mesh Rendering Card**: Cast/receive shadows, per-object wireframe and bounds; a viewport *Wireframe* chip toggles global wireframe; the play bar has a simulation speed button.
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
| `editor/spawn_primitive` | `spawn <kind> [name] [x y z]` | Spawns `cube`, `sphere`, `cylinder`, `plane`, `point_light`, `directional_light`, `spot_light`, or `camera`. |
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
| `editor/components` | | Reflected components of an entity with fields, values, units, ranges and tooltips. |
| `editor/set_field` | | Edits any reflected field (`component`, `field` path, `value`), with undo. |
| `editor/add_component` / `editor/remove_component` | | Adds a component by type name using its `Default`, or removes one. |
| `editor/component_types` | | Lists registered components that can be added (optional `filter`). |
| `editor/world` | | Reads or edits World / Environment settings (`set: {field: value}`). |
| `editor/camera_rendering` | | Camera post-process stack: `features` on/off, `anti_aliasing`, `tonemapping`, viewport `preview`. |
| `editor/mesh_rendering` | | Per-object `cast_shadows`, `receive_shadows`, `wireframe`, `show_bounds`. |
| `editor/viewport` | | Viewport overlays: `wireframe`, `grid`, `gizmos`, `bounds`. |
| `editor/time_scale` | | Simulation speed (`Time<Virtual>` relative speed). |
| `editor/cameras` | | Every camera with order, HDR and activity (debugging). |
| `editor/import_model` | | Imports an `.fbx`/`.obj`/`.stl`/`.ply` file (`path`, optional `position`, `name`). |
| `editor/undo` / `editor/redo` / `editor/history` | | Steps the undo history or reports whether undo/redo is available. |
| `editor/key_chord` | | Sends real keyboard input such as `["ControlLeft", "KeyZ"]` to the editor window. |
| `editor/perf` | | FPS, frame time, entity and UI node counts, and how many UI components changed last frame. |
 `bevylite mcp` server exposes all of these to AI agents through the `bevylite_editor_rpc` tool, which forwards to the running editor.

---

### 📦 Multi-Platform Exporter
Pluggable export architecture in `src/export/`:
- 🪟 **Windows**: Standalone optimized executable packaging with bundled assets.
- 🐧 **Linux**: Standalone ELF executable build with asset bundling.
- 🌐 **Web**: WebAssembly (WASM) / HTML5 canvas deployment.
- 🤖 **Android**: Direct `cargo-apk` build or turnkey Gradle project scaffolding.
- 🍎 **iOS**: Xcode project scaffolding for App Store and TestFlight deployment.

---

## Sample games

The `samples/` workspace holds small but complete games, each with a title menu, HUD,
pause, game-over and restart flow (shared through `samples/sample_kit`):

| Sample | Genre | Run |
| :-- | :-- | :-- |
| Coin Courtyard | Third person: orbit camera, jumping, coin collecting against a timer | `cargo run -p third_person` |
| Target Range | First person: mouse look, hitscan shooting, ammo and reloads | `cargo run -p first_person` |
| Neon Runner | Endless runner: three lanes, jumps, coins, rising speed | `cargo run -p endless_runner` |
| Arena Survivor | Top-down twin-stick shooter with waves and a dash | `cargo run -p arena_shooter` |

Run them from the `samples/` folder, or use the **Sample Games** section of the Project Hub:
**Open in Editor** loads the sample's level (the same `src/level.rs` the game uses) and
**▶ Play** builds and launches the game in its own window.
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

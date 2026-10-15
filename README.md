# Bevy Editor

A minimal, modular, highly extensible, and fast-loading 3D Scene Viewer and Multi-Platform Exporter for [Bevy Engine](https://bevy.org) (`0.20-dev`).

Built on Bevy's native UI system (`bevy_ui` + `bevy_feathers`), built-in camera controllers (`bevy_camera_controller`), reflection inspector (`bevy_inspector`), and scene serialization (`bevy_scene`).

---

## Features

- **Fly-Through 3D Viewport**: Controlled via [`FreeCamera`](src/camera/mod.rs) with WASD movement, right-click mouse look, Q/E vertical elevation, and shift speed boost.
- **Native Entity & Component Inspector**: Uses Bevy's official [`bevy_inspector`](src/ui/inspector.rs) and `bevy_feathers` dark theme tokens, providing real-time entity tree inspection and property editing.
- **Project Explorer & Asset Browser**: Built-in [`asset_browser`](src/ui/asset_browser.rs) panel browsing the `assets/` directory (models, scenes, textures) with single-click spawning into the 3D world.
- **Drag-and-Drop 3D Models**: Uses Bevy's native [`FileDragAndDrop`](src/scene/drag_drop.rs). Drag any `.glb`, `.gltf`, or `.ron` file directly from Windows Explorer into the editor window to spawn it immediately.
- **Dynamic Scene Management**: Author, modify, save, and reload scenes (`.ron` / `.bsn`) via [`DynamicWorld`](src/scene/save_load.rs).
- **Extensible Multi-Platform Exporter**: Pluggable [`Exporter`](src/export/mod.rs) architecture:
  - 🪟 **Windows**: Standalone optimized executable packaging with bundled assets.
  - 🐧 **Linux**: Standalone ELF executable build with asset bundling.
  - 🌐 **Web**: WebAssembly (WASM) / HTML5 canvas deployment.
  - 🤖 **Android**: Direct `cargo-apk` build or turnkey Gradle project scaffolding (via Bevy's official mobile integration).
  - 🍎 **iOS**: Xcode project scaffolding for App Store / test flight deployment.

---

## Supported 3D Model Formats

| Format | Status | Notes |
| :--- | :--- | :--- |
| **glTF 2.0 (`.glb`, `.gltf`)** | 🟢 **Native First-Class** | Fully supported out-of-the-box by Bevy (`bevy_gltf`). Supports meshes, PBR materials, skeletal animations, skinning, morph targets, and embedded textures. |
| **FBX (`.fbx`)** | 🟡 **Requires Conversion** | Autodesk FBX is a proprietary format not built into Bevy core. Convert FBX to `.glb` in Blender or using `fbx2gltf` before dropping into the editor. |
| **OBJ (`.obj`) / STL (`.stl`)** | 🟡 **Plugin Supported** | Can be loaded via community plugins (`bevy_obj`, `bevy_stl`). |

---

## Architecture Overview

```
src/
├── main.rs            # Application entry point & plugin configuration
├── camera/            # Viewport camera & fly-through controller (FreeCamera)
├── scene/             # Scene primitives, drag-and-drop, and DynamicWorld save/load
│   ├── drag_drop.rs   # Native FileDragAndDrop listener (glTF, GLB, RON, BSN)
│   ├── primitives.rs  # Default 3D shapes (plane, cube, sphere, sun light)
│   └── save_load.rs   # Serialization excluding EditorInternal entities
├── ui/                # Feathers-based UI (Figma dark theme)
│   ├── asset_browser.rs# Project Explorer (assets/) with click-to-spawn
│   ├── menu.rs        # Top menu bar (New, Open, Save, Export targets)
│   ├── inspector.rs   # Split-pane entity tree and component details
│   └── status_bar.rs  # Bottom stats, camera guide, and export notifications
└── export/            # Multi-target export engine
    ├── mod.rs         # Exporter trait & ExportRegistry
    ├── android.rs     # Android APK & Gradle scaffolding
    ├── ios.rs         # iOS Xcode scaffolding
    ├── linux.rs       # Linux desktop packaging
    ├── web.rs         # WebAssembly bundle generation
    └── windows.rs     # Windows desktop packaging
```

---

## Unified Executable & CLI Tools (`bevylite`)

Bevylite is built as a single, ultra-fast binary serving as both an interactive 3D visual level editor and a headless command-line build & automation pipeline:

```bash
# Launch the 3D visual editor window
cargo run
# or directly:
bevylite

# Scaffold a clean, standalone Bevy game project
bevylite new MyGame

# Headless multi-platform build & export
bevylite build --target windows --output ./dist/windows
bevylite build --target web --output ./dist/web
bevylite build --target android --output ./dist/android

# Headless scene operations
bevylite scene inspect ./assets/scenes/starter.ron
bevylite scene add-comp ./assets/scenes/starter.ron "Cube" "PointLight"

# Start the Model Context Protocol (MCP) server for AI agent pair programming
bevylite mcp

# Run in headless interactive console mode
bevylite -c
```

---

## Build Profiles (Bevylite Tiers)

Users can select how lightweight they want the editor depending on workflow and compile constraints:

| Profile | Command | Included Features |
| :--- | :--- | :--- |
| **⚡ Minimal Lite** | `cargo run --no-default-features --features lite` | Bare minimum 3D scene viewer, camera controller, basic primitives |
| **◈ Standard (Default)** | `cargo run` | Feathers dark UI, Scene Hierarchy, Inspector, 3D Transform Gizmos, glTF |
| **🚀 Fast Dev Iteration** | `cargo run --features dynamic_linking` | Shared dynamic engine linking (~1.5s re-links during development) |
| **🛠️ Pro / Full** | `cargo run --features "remote,compressed_textures,solari"` | JSON-RPC MCP server, KTX2/DDS compression, Solari real-time ray tracing |

---

## Viewport Controls

| Input | Action |
| :--- | :--- |
| **Right Click (Hold)** | Look around (first-person mouselook) |
| **W / A / S / D** | Fly forward / left / backward / right (while holding RMB) |
| **Q / E** | Fly down / up (while holding RMB) |
| **Left Shift** | Turbo speed boost |
| **Scroll Wheel** | Adjust fly navigation speed |
| **F** | Focus viewport camera on selected object |
| **Drag & Drop** | Drop `.glb`, `.gltf`, or `.ron` files directly from OS into viewport |

---

## License

Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.


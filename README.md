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
  - 🤖 **Android**: Direct `cargo-apk` build or turnkey Gradle project scaffolding (via Bevy's official mobile integration).
  - 🪟 **Windows**: Standalone optimized executable packaging with bundled assets.
  - 🌐 **Web**: WebAssembly (WASM) / HTML5 canvas deployment.
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
    ├── windows.rs     # Windows desktop packaging
    ├── web.rs         # WebAssembly bundle generation
    └── ios.rs         # iOS Xcode scaffolding
```

---

## Controls

| Input | Action |
| :--- | :--- |
| **Right Click (Hold)** | Look around (first-person mouselook) |
| **W / A / S / D** | Fly forward / left / backward / right |
| **Q / E** | Fly down / up |
| **Left Shift** | Turbo speed boost |
| **Scroll Wheel** | Adjust fly speed |
| **Drag & Drop** | Drop `.glb`, `.gltf`, or `.ron` files from Windows Explorer into the window |

---

## License

Dual-licensed under MIT or Apache-2.0.

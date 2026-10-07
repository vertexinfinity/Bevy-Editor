# Bevy Editor — Roadmap & Architecture Status (TODO.md)

> **Core Architectural Rule**:
> The editor is designed as a collection of independent modules and plugins.
> Every feature is removable, replaceable, and extendable without modifying existing code.
> Communication between modules occurs strictly through **Messages/Events**, **Resources**, **Commands**, and **Traits** — never through tight cross-module internal coupling.

---

## 🏗️ The 6 Core Subsystems (MVP Pillars)

| Subsystem | Status | Architectural Role | Module Location |
| :--- | :---: | :--- | :--- |
| **1. ECS Scene System** | 🟢 Done | Scene abstraction (`SceneSerializer`, `SceneLoader`, `.ron`/`.scn`), dynamic world serialization, starter primitives | [`src/scene/`](file:///D:/Repos/Bevy/Bevy-Editor/src/scene/) |
| **2. Selection System** | 🟢 Done | Active entity selection, multi-select support, sync with `InspectorSelection`, 3D viewport picking | [`src/core/selection.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/selection.rs) |
| **3. Inspector System** | 🟢 Done | Clean Transform view (local/global toggle), reflection debug view toggle, dockable panel, component adder | [`src/ui/inspector.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/inspector.rs) |
| **4. Command / Undo System** | 🟢 Done | `EditorCommand` trait, `CommandHistory` with undo/redo stack, `Ctrl+Z` / `Ctrl+Y` shortcuts | [`src/core/commands.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/commands.rs) |
| **5. Panel System** | 🟢 Done | `EditorPanel` trait, `PanelRegistry`, modular open/close/toggle lifecycle | [`src/core/panels.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/panels.rs) |
| **6. Tool System** | 🟢 Done | `EditorTool` trait, `ToolRegistry`, active tool state, hotkeys (`Q`: Select, `W`: Move, `E`: Rotate, `R`: Scale) | [`src/core/tools.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/tools.rs) |

---

## 🎮 Play-In-Editor (PIE) & Simulation Subsystem

- 🟢 **Play State Machine**: `Edit` (authoring), `Playing` (live simulation), `Paused` (frozen frame inspection).
- 🟢 **Memory Snapshot & Rollback**:
  - Hitting `▶ Play` captures an in-memory snapshot of user entities via `DynamicWorld::from_world`.
  - Hitting `⏹ Stop` rolls back the scene to the exact pre-simulation state without polluting scene files.
- 🟢 **Top PIE Toolbar**: Centered Play, Pause, Stop, and Step controls with real-time state feedback.
- 🟢 **Simulation Systems**: `simulate_velocities` and script runtime run exclusively during `PlayState::Playing`.

---

## 📜 Scripting & Data Persistence

- 🟢 **Public Script Property Serialization**:
  - `ScriptComponent` exposes a typed property dictionary (`ScriptPropertyVal::Float`, `Int`, `Bool`, `String`, `Vec3`).
  - Serialized to `.ron` / `.bsn` using Bevy reflection and deserialized automatically upon scene loading.
- 🟢 **Object & Script Assignment**:
  - `entity_refs: HashMap<String, Entity>` allows linking target objects, cameras, doors, and triggers to script components.
- 🟡 **In-Editor Script Property Editor**:
  - Property rows in Inspector allowing live value tweaks for numbers, strings, and booleans.
- 🔴 **Live Hot-Reloading Script Engines**:
  - Bevy Lua integration (e.g. `bevy_mod_scripting` or `mlua`) with file watching.
  - Rhai embedded scripting backend.

---

## 📁 Project Scaffolding & Standalone Builds

- 🟢 **New Project Scaffolder ([`src/project/scaffold.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/project/scaffold.rs))**:
  - Creates clean, minimal, zero-error standalone Bevy game projects.
  - Generates `Cargo.toml`, `src/main.rs`, `assets/`, and sample rotating cube scene ready for `cargo run`.
- 🟢 **Runtime / Editor Separation**:
  - Editor internal entities are tagged with marker component `EditorInternal`.
  - Game code and exported projects run 100% standalone without editor dependencies.
- 🟢 **Multi-Platform Export Pipeline ([`src/export/`](file:///D:/Repos/Bevy/Bevy-Editor/src/export/))**:
  - Pluggable `Exporter` trait for Android (Gradle/NDK), Windows, WebAssembly, and iOS.

---

## 🔭 Detailed Status & Backlog

### 🟢 Completed Features
1. **Industry-Standard Fly Camera**:
   - Hold RMB to look around with mouse.
   - WASD + QE (Space/C) flight translation.
   - Shift sprint boost (2.5x).
   - Mouse scroll wheel adjusts flight speed smoothly.
   - Cursor automatically unlocked on RMB release.
2. **Interactive Menu Bar**:
   - "New Project..." (scaffolds clean standalone game).
   - "New Scene", "Open...", "Save", "Save As...".
   - "Export >" (Android, Windows, Web, iOS).
   - Hover highlights and reliable `On<PointerClick>` observers.
3. **Clean Transform Inspector**:
   - Clean default view (Position, Euler Rotation in degrees, Scale).
   - Local vs. Global coordinate space toggle.
   - Interactive nudge buttons (`[-X]`, `[+X]`, `[-Y 15°]`, etc.).
   - Debug View toggle for full engine reflection panel.
   - Dockable / collapsible right sidebar (`Dock ▶` / `◀ Dock`).
4. **Asset Browser & Viewport Drag-and-Drop**:
   - Browsing `assets/` directory with file icons.
   - Native drag-and-drop ingestion of 3D models (`.glb`/`.gltf`) and scenes (`.ron`).
   - Click-to-spawn items into the 3D scene.
5. **Component Authoring**:
   - `+ Script (Lua)`, `+ Spin Animation`, `+ Velocity`, `+ Point Light`, `+ Gameplay Tag`.

---

### 🟡 In Progress / Polish
- [ ] Visual Transform Gizmos in 3D viewport (using `bevy_gizmos` arrows for Translate/Rotate/Scale handles).
- [ ] Drag-to-assign Entity picker field in the Inspector for `entity_refs`.
- [ ] Console / Output Log dockable panel displaying engine logs and script print statements.

---

### 🔴 Future Extensibility Roadmap (Built on Top of the 6 Pillars)
- [ ] **Prefabs & Nested Scenes**: Reusable entity templates with property overrides.
- [ ] **Animation Curve Editor**: Dope sheet and keyframe timeline for transform and skeletal animations.
- [ ] **Material & Shader Graph Editor**: Node-based shader authoring.
- [ ] **Terrain & Sculpting Tools**: Heightmap terrain sculpting using `EditorTool` interface.
- [ ] **NavMesh & AI Tools**: Pathfinding mesh baking and waypoint authoring.
- [ ] **Particle System Editor**: Visual particle emitter tweaking with live viewport preview.
- [ ] **Multi-Viewport Layout**: Top/Front/Side orthographic projection viewports alongside perspective camera.
- [ ] **Scene Streaming & World Partitioning**: Chunk-based level loading for large open worlds.
- [ ] **Editor Plugin Marketplace / Add-on Loader**: Dynamically loaded editor plugins via shared libraries or crates.

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
- 🟢 **In-Editor Script Property Editor**:
  - Dedicated card in clean Inspector showing file path, public properties, and interactive live tuning buttons (`Speed -1`, `Speed +1`, `Jump +2`, `Toggle Active`).
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
### 🟢 Completed Features
1. **Industry-Standard Fly Camera**:
   - Hold RMB to look around with mouse.
   - WASD + QE (Space/C) flight translation.
   - Shift sprint boost (2.5x).
   - Mouse scroll wheel adjusts flight speed smoothly.
   - Cursor automatically unlocked on RMB release.
   - Camera Auto-Focus (`F` key) centering on selected entity.
2. **Interactive Menu Bar**:
   - "New Project..." (scaffolds clean standalone game).
   - "New Scene", "Open...", "Save", "Save As...".
   - "Export >" (Android, Windows, Web, iOS).
   - Hover highlights and reliable `On<PointerClick>` observers.
3. **Clean Transform Inspector**:
   - Clean default view (Position, Euler Rotation in degrees, Scale).
   - Local vs. Global coordinate space toggle.
   - Interactive nudge buttons (`[-X]`, `[+X]`, `[-Y 15°]`, etc.).
   - Debug View toggle for full engine reflection panel (`entity_tree_panel` + `details_panel`).
   - Dockable / collapsible right sidebar (`Dock ▶` / `◀ Dock`).
4. **Asset Browser & Viewport Drag-and-Drop**:
   - Browsing `assets/` directory with file icons.
   - Native drag-and-drop ingestion of 3D models (`.glb`/`.gltf`) and scenes (`.ron`).
   - Folder navigation with breadcrumbs (`📁 assets/models/`) and `⬆ Up` parent directory button.
   - Click-to-spawn items into the 3D scene.
   - Automatic image decoding for embedded JPEG, WebP, KTX2 textures in glTF models.
5. **Component Authoring**:
   - `+ Script (Lua)`, `+ Spin Animation`, `+ Velocity`, `+ Point Light`, `+ Gameplay Tag`.
6. **3D Viewport Raycast Picking & Gizmos**:
   - Global `On<PointerClick>` with `MeshPickingCamera`.
   - Ancestor traversal to automatically map clicked mesh children to authorable `SceneEntity` root.
   - **Visual 3D Selection Ring**: Radiant cyan ring drawn at selected object's base.
   - **Native Transform Gizmo Integration**: Connects `InspectorSelection` to `TransformGizmoFocus` and `TransformGizmoSettings` (Translate, Rotate, Scale).
7. **Hierarchical Scene Tree Panel (Outliner) ([`src/ui/hierarchy.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/hierarchy.rs))**:
   - Dockable left sidebar displaying all active scene entities.
   - Real-time entity count badge.
   - Quick primitive spawners (`+ Cube`, `+ Sphere`, `+ Light`).
   - Click-to-select rows with synchronized inspector view.
8. **Active Context Bottom-Right Badge**:
   - Real-time indicator displaying the selected entity name and ID (e.g. `Active: Cube (Entity(42v0))`).
9. **Responsive Docking System (Jackdaw-inspired)**:
   - Dockable Inspector with responsive auto-fit (`max_width: percent(45.0)`).
   - Dockable Project Explorer with collapsible header and `▼`/`▲` toggle.
   - Dockable Hierarchy Outliner with collapse button.
10. **Remote BRP Server & MCP Controller ([`src/remote/mod.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/remote/mod.rs))**:
    - JSON-RPC 2.0 over HTTP on loopback port `15703`.
    - Endpoints: `editor/status`, `editor/scene_tree`, `editor/select`, `editor/spawn_primitive`, `editor/play_mode`.
    - Bridge file polling (`assets/.mcp_bridge.json`) for live event status bar feedback.

---

### 🟡 Recommended Next Line of Items (Immediate MVP Next Steps)
1. **Interactive 3D Axis Gizmo Dragging & Raycast Picking**:
   - Verify and enhance direct viewport drag manipulation of transform handles (Red X, Green Y, Blue Z axes) with camera projection.
2. **3D Selection Bounding Box (AABB) & Wireframe Overlay**:
   - Draw wireframe bounding box around the active entity's mesh geometry in addition to the ground circle.
3. 🟢 **In-Editor Script Property Editor**:
   - Completed in `src/ui/inspector.rs` with live key-value display and mutation buttons.
4. **Console / Output Log Dockable Panel**:
   - Dockable bottom panel capturing engine logs, warnings, import events, and AI bridge commands.
5. **Drag-to-Assign Entity Reference Slot in Inspector**:
   - Allow dragging an entity from the Outliner into a script component's `entity_refs` target slot.

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

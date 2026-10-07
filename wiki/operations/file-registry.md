# File Registry & Architecture Map (filemap)

> **Wiki-First Navigation**: Consult this registry before initiating searches or grep sweeps.
> Confidence: 🟢 Confirmed

---

## 🧭 Executive Architecture & Subsystem Mapping

```
Bevy-Editor (src/)
├── main.rs                   # Application entry point, Plugin registration, Window & AssetPlugin config
├── camera/                   # Viewport Fly Camera & Mesh Picking Camera
│   └── mod.rs                # RMB look, WASD movement, scroll speed, MeshPickingCamera
├── core/                     # The 6 Architectural Pillars
│   ├── mod.rs                # CoreEditorPlugin aggregation
│   ├── commands.rs           # Command history, Undo/Redo stack, Ctrl+Z/Ctrl+Y
│   ├── panels.rs             # EditorPanel trait & PanelRegistry for modular panels
│   ├── play_mode.rs          # PIE simulation, snapshotting, rollback, pop-out preview
│   ├── selection.rs          # EditorSelection, sync with InspectorSelection, SelectionChangedMessage
│   └── tools.rs              # EditorTool trait & ToolRegistry (Select, Move, Rotate, Scale)
├── scene/                    # ECS Scene Hierarchy, Serialization & Ingestion
│   ├── mod.rs                # SceneManagerPlugin, CurrentScene tracking
│   ├── components.rs         # Authorable components (ScriptComponent, SpinAnimation, Velocity, etc.)
│   ├── drag_drop.rs          # Smart file ingestion, copy to assets, drop-target hit-testing
│   ├── primitives.rs         # Starter scene primitives (Sun, Ground, Cube, Sphere)
│   ├── save_load.rs          # DynamicWorld serialization & EditorInternal/SceneEntity markers
│   └── serialization.rs      # SceneSerializer / SceneLoader abstractions & RonSceneSerializer
├── ui/                       # Decoupled UI Panels & Overlays
│   ├── mod.rs                # EditorUiPlugin, system registration, dark theme
│   ├── theme.rs              # Design tokens (BG_SHELL, BORDER_ACCENT, AXIS_X/Y/Z)
│   ├── hierarchy.rs          # Scene Hierarchy (Outliner) with primitive spawners & collapse
│   ├── asset_browser.rs      # Project Explorer, dockable tab, directory hover detection
│   ├── inspector.rs          # Clean Transform view, Debug view, mesh click observer, responsive dock
│   ├── menu.rs               # Top menu bar (New/Open/Save Scene, New Project, Export)
│   ├── play_bar.rs           # Centered Play/Pause/Stop/Step PIE controls
│   ├── status_bar.rs         # Bottom status bar & active selection context badge
│   └── tools_bar.rs          # Viewport tool switcher overlay (Q/W/E/R)
├── project/                  # Standalone Project Creation
│   ├── mod.rs                # Project module entry
│   └── scaffold.rs           # Zero-error standalone Bevy game generator (Cargo.toml + main.rs)
├── remote/                   # Remote BRP & MCP Protocol Subsystem (JSON-RPC on port 15703)
│   └── mod.rs                # editor/status, editor/scene_tree, editor/select, editor/spawn_primitive
└── export/                   # Multi-Platform Exporters
    ├── mod.rs                # ExportPlugin, Exporter trait, ExportStatus tracking
    ├── android.rs            # Android Gradle / NDK packaging pipeline
    ├── windows.rs            # Windows standalone packaging pipeline
    ├── web.rs                # WebAssembly / wasm-bindgen packaging pipeline
    └── ios.rs                # iOS Xcode packaging pipeline
```

---

## 📋 Comprehensive File Registry

| Domain | File Path | Primary Structs / Symbols | Responsibility | Confidence |
| :--- | :--- | :--- | :--- | :---: |
| **App Entry** | [`src/main.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/main.rs) | `main` | Configures `WindowPlugin`, sets `UnapprovedPathMode::Allow`, builds plugins | 🟢 Confirmed |
| **Camera** | [`src/camera/mod.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/camera/mod.rs) | `FlyCamera`, `EditorCamera`, `MeshPickingCamera` | Viewport camera flight controller and 3D mesh raycast picking receiver | 🟢 Confirmed |
| **Core / Command** | [`src/core/commands.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/commands.rs) | `EditorCommand`, `CommandHistory` | Undo/Redo command pattern stack with shortcut listeners (`Ctrl+Z`, `Ctrl+Y`) | 🟢 Confirmed |
| **Core / Panels** | [`src/core/panels.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/panels.rs) | `EditorPanel`, `PanelRegistry`, `PanelState` | Modular registration and state management of editor panels | 🟢 Confirmed |
| **Core / PIE** | [`src/core/play_mode.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/play_mode.rs) | `PlayState`, `PlayModeState`, `WorldSnapshot` | Process-isolated simulation, in-memory world snapshot, and pop-out preview | 🟢 Confirmed |
| **Core / Select** | [`src/core/selection.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/selection.rs) | `EditorSelection`, `SelectionChangedMessage` | Tracks primary and multi-selection; synchronizes with `InspectorSelection` | 🟢 Confirmed |
| **Core / Tools** | [`src/core/tools.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/tools.rs) | `EditorTool`, `ToolRegistry`, `ActiveTool` | Modular transform tool registry (`Select`, `Translate`, `Rotate`, `Scale`) | 🟢 Confirmed |
| **Core / Mod** | [`src/core/mod.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/mod.rs) | `CoreEditorPlugin` | Aggregates tools, panels, commands, selection, and play mode | 🟢 Confirmed |
| **Scene / Props** | [`src/scene/components.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/scene/components.rs) | `ScriptComponent`, `SpinAnimation`, `Velocity` | Authorable gameplay and script components with serialized properties | 🟢 Confirmed |
| **Scene / Ingestion** | [`src/scene/drag_drop.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/scene/drag_drop.rs) | `DragDropPlugin`, `handle_file_drag_and_drop` | Copies dropped files to `assets/` subdirs, checks drop zone, spawns models | 🟢 Confirmed |
| **Scene / Prims** | [`src/scene/primitives.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/scene/primitives.rs) | `spawn_starter_scene` | Spawns starter lighting, ground plane, cube, and sphere | 🟢 Confirmed |
| **Scene / Save** | [`src/scene/save_load.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/scene/save_load.rs) | `EditorInternal`, `SceneEntity`, `save_world_to_scene` | DynamicWorld serialization and markers safeguarding editor UI from clearance | 🟢 Confirmed |
| **Scene / Serde** | [`src/scene/serialization.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/scene/serialization.rs) | `SceneSerializer`, `SceneLoader`, `RonSceneSerializer` | Decoupled scene serialization abstraction | 🟢 Confirmed |
| **Scene / Mod** | [`src/scene/mod.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/scene/mod.rs) | `SceneManagerPlugin`, `CurrentScene` | Manages active scene state and aggregates scene plugins | 🟢 Confirmed |
| **UI / Theme** | [`src/ui/theme.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/theme.rs) | `BG_SHELL`, `BORDER_ACCENT`, `AXIS_X`, `TEXT_PRIMARY` | Linear/Figma dark theme design tokens & colors | 🟢 Confirmed |
| **UI / Hierarchy** | [`src/ui/hierarchy.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/hierarchy.rs) | `HierarchyState`, `HierarchyDockRoot`, `SpawnPrimitiveMessage` | Dockable scene outliner tree, primitive spawners, selection sync | 🟢 Confirmed |
| **UI / Browser** | [`src/ui/asset_browser.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/asset_browser.rs) | `AssetItem`, `HoveredAssetDirectory`, `ProjectExplorerDockRoot` | Dockable Project Explorer with folder hover detection and live refresh | 🟢 Confirmed |
| **UI / Inspector** | [`src/ui/inspector.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/inspector.rs) | `InspectorState`, `InspectorDockRoot`, `select_entity_on_viewport_click` | Responsive dockable inspector, clean transform view, raycast mesh selector | 🟢 Confirmed |
| **UI / Menu** | [`src/ui/menu.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/menu.rs) | `MenuAction`, `MenuActionMessage`, `spawn_menu_bar` | Top application menu bar with file dialogs and action routing | 🟢 Confirmed |
| **UI / Play** | [`src/ui/play_bar.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/play_bar.rs) | `PlayBarPlugin`, `spawn_play_bar` | Centered top toolbar for Play, Pause, Stop, and Step controls | 🟢 Confirmed |
| **UI / Status** | [`src/ui/status_bar.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/status_bar.rs) | `ActiveContextChip`, `StatusText`, `update_status_bar` | Bottom status bar and real-time active entity context badge | 🟢 Confirmed |
| **UI / Tools** | [`src/ui/tools_bar.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/tools_bar.rs) | `ToolBtnMarker`, `spawn_tools_bar` | Floating viewport tool bar with tool active state highlights | 🟢 Confirmed |
| **UI / Mod** | [`src/ui/mod.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/mod.rs) | `EditorUiPlugin` | Aggregates all UI panels, message types, and global picking observers | 🟢 Confirmed |
| **Project / Gen** | [`src/project/scaffold.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/project/scaffold.rs) | `ProjectScaffolder` | Scaffolds clean standalone Bevy game projects with zero editor dependencies | 🟢 Confirmed |
| **Remote / BRP** | [`src/remote/mod.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/remote/mod.rs) | `EditorRemotePlugin`, `status_handler`, `scene_tree_handler` | Loopback JSON-RPC BRP server on port 15703 for MCP agents and automation | 🟢 Confirmed |
| **Export / Mod** | [`src/export/mod.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/export/mod.rs) | `Exporter`, `ExportStatus`, `ExportPlugin` | Extensible export pipeline and background task progress status | 🟢 Confirmed |
| **Export / Platforms** | `src/export/*.rs` | `AndroidExporter`, `WindowsExporter`, `WebExporter`, `IosExporter` | Platform-specific export implementations | 🟢 Confirmed |

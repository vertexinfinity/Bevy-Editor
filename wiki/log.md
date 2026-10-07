# Operations & Session Log

## [2026-10-07] - Filemap Creation & Adaptive Wiki Setup
- **Author**: Josh Thomson <ijoshuathomson@gmail.com>
- **Scope**: Established complete `wiki/` directory with filemap, architectural blueprint, procedural code recipes, and session logging.
- **Pages Created**:
  - `wiki/index.md`
  - `wiki/operations/file-registry.md`
  - `wiki/overview/architecture.md`
  - `wiki/memory/procedural.md`
  - `wiki/log.md`
- **Features Verified**:
  - Smart asset copy pipeline and UnapprovedPathMode override.
  - Responsive Inspector dock with auto-fit.
  - Dockable Project Explorer with folder hover detection.
  - Raycast 3D mesh selection with ancestor traversal.
  - Real-time active context badge in bottom right.

## [2026-10-07] - glTF Drag-and-Drop Ingestion, Folder Navigation & BRP MCP Server
- **Author**: Josh Thomson <ijoshuathomson@gmail.com>
- **Features Added**:
  1. **Project Explorer Folder Navigation**: Breadcrumb trail (`📁 assets/models/`), clickable folder rows, and parent folder navigation (`⬆ Up`).
  2. **Asset Path Normalization**: Stripped leading slashes (`.trim_start_matches('/')`) for glTF and scene loading via `AssetServer`.
  3. **Camera Auto-Focus Shortcut**: Pressing `F` smoothly positions and directs `EditorCamera` towards the selected entity's translation.
  4. **Remote BRP Server (`src/remote/mod.rs`)**:
     - Runs JSON-RPC over HTTP on loopback port `15703` (coexisting with Bevy default `15702`).
     - Registered RPC methods: `editor/status`, `editor/scene_tree`, `editor/select`, `editor/spawn_primitive`, `editor/play_mode`.
     - Enables MCP agents and automated tools to query scene state, select objects, and spawn primitives programmatically.
  5. **Subsystem Documentation**: Updated `wiki/overview/architecture.md`, `wiki/operations/file-registry.md`, and `TODO.md`.

## [2026-10-07] - Enabled Complete glTF Texture Support (JPEG, WebP, KTX2, etc.)
- **Author**: Josh Thomson <ijoshuathomson@gmail.com>
- **Fix**: Enabled image decoding features in `Cargo.toml` (`jpeg`, `png`, `webp`, `bmp`, `tga`, `ktx2`, `dds`) allowing `.glb` files with embedded JPEG textures to parse and load into the 3D scene without errors. Documented in `wiki/risks/defects.md`.

## [2026-10-07] - Wiki Registry Synchronization & 3D Bounding Box Gizmos
- **Author**: Josh Thomson <ijoshuathomson@gmail.com>
- **Updates**:
  1. **Wiki Synchronization**: Indexed [`src/ui/hierarchy.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/hierarchy.rs) (Scene Hierarchy panel) and [`src/ui/theme.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/ui/theme.rs) (Design Tokens) in [`wiki/operations/file-registry.md`](file:///D:/Repos/Bevy/Bevy-Editor/wiki/operations/file-registry.md) and [`wiki/overview/architecture.md`](file:///D:/Repos/Bevy/Bevy-Editor/wiki/overview/architecture.md).
  2. **TODO.md Updated**: Marked Hierarchy Outliner, Camera Auto-Focus, and Native Transform Gizmo synchronization as complete.
  3. **Visual 3D Selection Enhancements ([`src/core/selection.rs`](file:///D:/Repos/Bevy/Bevy-Editor/src/core/selection.rs))**:
     - Upgraded `draw_selection_gizmo` to query `bevy::camera::primitives::Aabb`.
     - Draws a 3D AABB wireframe box tightly enclosing the selected mesh geometry.
     - Renders Red (X), Green (Y), and Blue (Z) 3D coordinate orientation arrows.
     - Preserves the radiant cyan base ring on the floor plane.

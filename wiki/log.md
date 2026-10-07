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

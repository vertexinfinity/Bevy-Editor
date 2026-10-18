# Bevylite UI refresh: design direction and implementation outline

Status: first implementation pass is in place in the editor code. The images remain concept collages, not evidence that every illustrated feature exists. The specification below describes the design direction; the implementation outline marks implemented work and remaining gaps.

## Concept collages

1. [Workspace and Game View](01-workspace-game-view.png): shell, transport, panel interactions and focused game input.
2. [Hierarchy and assets](02-hierarchy-assets.png): tree/list/grid rows, selection, search, hover actions and context menus.
3. [Inspector and ECS relationships](03-inspector-relations-v2.png): typed property sections, reference navigation, optional selected-scope graph and inspector states.
4. [Materials](04-materials-v2.png): compact inspector, dedicated asset editor, color/texture pickers and validation states.

The courtyard art illustrates a viewport composition; adding those art assets is not part of the UI scope. The current procedural sample remains the verification fixture.

## The central change

Stop representing every UI entity as a bordered card or button. Establish hierarchy with layout, type weight, spacing and quiet surface contrast. A region should usually have one boundary treatment. A selected tree row should be a tonal fill, rather than a boxed card containing several boxed badges.

Keep one subtle divider at panel boundaries, between major inspector sections, and between a preview and its controls. Show a focus indicator on the active keyboard control, a temporary drop target while dragging, and a boundary around floating popovers/dialogs. Do not remove keyboard focus visibility in the name of minimalism.

## Visual rules

| Element | Default | Hover / selection / focus |
| --- | --- | --- |
| Panel | Flat neutral surface, one shared edge divider | Thin resize hit zone highlights during hover/drag |
| Inspector section | Borderless heading, optional separator below the previous section | Chevron and menu are discoverable, reset appears for changed values |
| Tree / list row | Transparent, one line, no rounded card | Neutral hover; restrained cyan selection; visible keyboard focus |
| Numeric / text field | Slightly darker fill, no permanent stroke | Fine cyan focus stroke only on the active field; invalid helper text when needed |
| Toolbar action | Quiet icon with tooltip | Tonal active/hover fill; persistent label only when needed |
| Dialog / popover | Distinct surface with one outer boundary | Clear focus and dismiss behavior |
| Entity reference | Readable name, icon and navigation action | Hover reveals pick/clear; missing targets offer relink |

Use Segoe UI on Windows with a proper fallback stack, approximately 13–14px body text, 14px semibold section headings and tabular/monospaced numeric values. Use a consistent 16px icon family instead of mixed emoji. Use a 4/8/12/16/24px spacing scale, roughly 28px list rows and 28–32px controls; offer a denser mode later. Use corner radii sparingly (about 3–4px for controls), without pill styling on every button.

Suggested tokens: canvas `#101318`, panel `#161B22`, input fill `#202630`, primary text `#E5E9F0`, secondary text `#929BAA`, separator `#2A3038`, accent `#45C5DB`. Check actual rendered contrast; this palette is a starting point, not a contrast certification. Axis colors stay local to X/Y/Z, status colors stay local to warnings/errors and Play state.

## What should be visible

- Always: project/scene identity, main menus, selection/manipulation tools, coordinate space, Play/Pause/Stop, panel tabs, selected entity identity and relevant component fields.
- Contextual: visibility/menu controls on hovered or selected rows, reset on changed properties, drag handles/drop previews during dragging, texture clear/replace controls when editing a slot.
- Behind overflow or an Advanced section: entity IDs, debug details, uncommon render settings, export/platform variants and diagnostics. Keep primary actions discoverable rather than hiding everything behind icons.
- Console/jobs: status summaries in the footer, with details in their tabs. Long-running work must not occupy the primary editing toolbar with several permanent badges.
- No selection: a helpful empty inspector, not editable zero-valued transform fields. Multi-selection: shared fields and explicit mixed values.

## Workspace and navigation

Use a slim menu row and a consistent tools/transport row. The center remains the viewport. Hierarchy is on the left, Inspector on the right, with Assets / Relations / Console / Jobs in the bottom dock. The bottom dock is part of the workspace layout; it should not float over an arbitrary portion of the game scene by default.

Start near 240–280px for the hierarchy and 320–380px for the inspector. Clamp widths to leave a usable center viewport. At smaller window widths, compact secondary toolbar labels and move secondary actions into overflow instead of wrapping toolbar text into multiple lines. Inspector property rows can switch to stacked labels at a narrow breakpoint. Long names truncate while retaining full names in a tooltip and the selected entity header.

Tabs should drag to a target stack or split zone, with a single translucent preview. Dividers resize immediately. Save layout by workspace and provide Reset Layout. The current edge resizing is a starting point; tab stacks, split trees and saved layouts are additional implementation work.

## Hierarchy and asset interactions

Hierarchy is an ECS tree: chevron, consistent entity-type icon, name and a contextual visibility/menu area. Remove persistent `ON` badges, numeric IDs and delete X controls from every row. Default rows have no border. Highlight a selected row across its width without changing its dimensions.

Click selects; the chevron expands; F focuses in the viewport; arrows navigate/expand; F2 renames; Ctrl/Cmd supports additive selection and Shift supports ranges. Keep focus and selection distinct. A context menu provides Duplicate, Rename, Focus, Reparent and Delete. Destructive actions should not sit beside every frequently clicked name.

Drag to reparent with an insertion/target preview, cycle prevention, undo support and a documented transform policy (preserve world transform by default). Search should find descendants inside collapsed branches. Preserve expansion state when the query is cleared and provide a clear-query action, result count, and useful empty state. Text input should support selection, paste and standard editing shortcuts.

Assets use a breadcrumb/folder area and a searchable/filterable list or thumbnail grid. List rows show an icon/preview, name and only useful secondary metadata. Grid items show a preview and one-line name. No permanent outlines per tile; select with a tonal fill. Double-click opens the appropriate asset editor, Enter opens, context menu handles secondary actions, and drag assigns compatible assets to slots or the viewport with a visible acceptance/rejection cue.

## Inspector and ECS relations

The selected entity header shows its name, type, pin/lock and overflow actions. Components and Relations are distinct tabs, so users can edit ordinary properties without a full relationship monitor taking space below every entity.

Sections share one presentation shell: heading, collapse state, compact actions and body. Built-in components and custom scripts should use the same typed field controls. Float/int fields accept typing and scrubbing; booleans use checkboxes/toggles; vectors use aligned axes; enums use dropdowns; assets and entity references use typed slots. Remove hard-coded Speed/Jump action buttons and multiline property dumps from the final inspector.

Define a section registration API with stable IDs, applicability, ordering, a spawn/render adapter and change notifications. Provide a typed script field schema with labels, groups, ranges, steps, defaults, read-only state and tooltips. An unknown component can fall back to reflection. A custom Rust renderer can replace a section where generic controls are insufficient. Edits go through shared actions/undo transactions instead of each card mutating arbitrary state independently.

Relations first show navigable lists: Parent, Children, References, Referenced By, and invalid links. Display names rather than raw entity IDs, while retaining IDs in optional diagnostics/tooltips. Clicking a link selects its target; a separate focus action also frames it. Picking from the hierarchy or dropping an entity assigns a compatible reference. Missing references keep their field identity and offer relinking.

The graph is optional, scoped to selection, initially one hop. Do not render the whole ECS world as a graph on every frame. Direction is explicit: `child -> parent` for ChildOf and `source -> target` for entity-reference fields. An incoming reference retains its original direction. Use relation labels and a legend in addition to color.

## Material editing

The entity inspector shows a material slot and a compact PBR editor. Use a small preview, material name, shader, base color, key texture slots, numeric/slider controls for Metallic and Roughness, and collapsed Emission/Advanced sections. Keep exact numeric entry alongside sliders. Replace the many +/- and preset buttons with these standard controls; presets belong in a secondary menu.

Base color opens a color popover with RGBA/HEX values and a clear color-space policy. Texture slots support search/pick, drag/drop, preview, clear and replace. An empty optional texture is valid and should not produce a warning. Warn when a referenced file is missing or an assignment is incompatible.

Double-clicking a material asset opens a larger editor with a preview on the left and properties on the right, separated once. Provide preview lighting and shape options without rebuilding the preview every frame. Distinguish shared assets from per-entity overrides: show usage count and a Make Unique action before a user unintentionally changes every object sharing a material. Multi-selection uses explicit mixed values.

## Game View and interaction ownership

Play opens a distinct Game View, with a focus indicator and concise controls. It contains game UI, not editor gizmos or authoring widgets. Keyboard/mouse input belongs to the game only while that view has input focus; losing focus clears held keys. Escape releases captured input, and a documented Stop shortcut or closing Game View ends the session. The editor transport remains available.

Pause stops simulation but leaves the image/window visible. Step advances exactly one simulation frame. Stop removes only preview-owned runtime objects and leaves the primary window, authoring entities, selection, hierarchy and material data intact. Repeated cycles and closing the game window must be regression tested.

Implemented in `src/core/play_mode.rs` and `src/core/play_input.rs`: Play opens a separate Game View, clones authoring entities into a preview scope, routes input through the focused game window, and Stop removes preview-owned state while preserving the editor window and authoring scene. Script references are remapped. Automated tests cover isolation, repeated Play/Stop and input ownership. Native visual/render-path verification remains important after camera/window changes; treat the collage as design intent rather than a claim of pixel parity.

## Implementation outline

| Order | Change | Main code areas | Completion evidence |
| --- | --- | --- | --- |
| 0 | Play isolation, window-scoped game input, showcase behavior and safe Stop (implemented; native runtime verification remains) | `src/core/play_mode.rs`, `play_input.rs`, `src/camera/mod.rs`, `src/scene/components.rs`, `src/scene/showcase.rs` | Automated ownership/input tests and repeated native Play/Pause/Step/Stop/close cycles; primary window and authoring IDs survive |
| 1 | Shared visual tokens and calmer section/list/field styling (first pass implemented) | `src/ui/theme.rs`, `src/ui/panel/card.rs`, reusable widget module | Representative states at normal/narrow widths; no default per-row/card borders; keyboard focus remains visible |
| 2 | Rework hierarchy and assets around flat rows and contextual actions | `src/ui/hierarchy.rs`, `src/ui/asset_browser.rs` | Search through collapsed groups, long names, selection, keyboard, rename, drag/drop, context menu, scrolling and error states |
| 3 | Typed inspector rows, script fields and section registration (first pass implemented) | `src/ui/inspector/types.rs`, `dock.rs`, `transform_card.rs`, `script_card.rs`, `components_card.rs` | Same field behavior for built-ins and one sample plugin/script; reset, validation and undo work |
| 4 | Compact PBR controls and material ownership/Make Unique (first pass implemented; dedicated preview/texture picker pending) | `src/ui/inspector/material_card.rs`, material preview/asset editor module | Numeric+slider edits, color/texture assignment, shared vs unique behavior, preview updates and missing-file states |
| 5 | Resizable panel widths and navigable relation lists implemented; tabbed docking, persistence and relation graph pending | `src/ui/layout.rs`, `src/ui/panel/*`, `relations_card.rs` | Dock/undock/resize/reset/persistence; links and graph agree with actual ECS data |
| 6 | Validate performance, accessibility and edge cases | Showcase fixture, automated tests, runtime profiling | Recorded input/layout/frame timings, focus/keyboard coverage and regression results |

Apply the visual change through shared components first; do not independently restyle dozens of spawn functions. `CardBuilder` should become a borderless section by default, with a distinct elevated surface only for an actual dialog/popover. Keep Bevy UI as the rendering/layout foundation and build reusable lightweight widgets above it; a visual refresh does not require an engine UI fork.

## Performance and acceptance

- Keep ECS-driven updates: selection changes row style in place; property changes update affected widgets; layout changes do not respawn the hierarchy.
- Cache relationship indexes and asset metadata; rebuild only after relevant changes. Virtualize long lists with a small overscan rather than spawning thousands of rows. Stable IDs preserve expansion, scroll, focus and in-progress text edits.
- Animate only small state transitions. Resize and scrub should follow the pointer immediately, with no easing on layout dimensions. Group a drag/scrub into one undo transaction.
- Profile the 113-entity fixture and a larger generated fixture separately. Target 60Hz interaction where hardware permits, p95 input-to-visible feedback below 50ms, and no repeated idle UI-tree reconstruction. Record CPU/UI/GPU costs rather than claiming the theme alone guarantees responsiveness.
- Check 1280x720 and 1920x1080 logical layouts, 100/125/150/200% scaling, long names, no selection, multiple selection, empty folders, broken references, small inspector widths and shader/asset loading.
- Test keyboard-only navigation, readable contrast, visible focus and tooltips. Color must not be the only signal for validation or graph link type.
- Treat the images as design intent. New capabilities such as multi-select, typed script editors, undo integration, tabbed docking and material previews must be tracked and verified independently before being advertised as implemented.

## Deliverables

The four collages, [generation prompts](PROMPTS.md), this implementation outline and the existing [showcase verification checklist](../../../samples/editor_showcase/README.md) form the design handoff. A first visual pass was applied through shared tokens and inspector components: hierarchy selection is quieter, inspector sections use fewer outlines, and typed fields are shared by built-in and script properties. This does not complete tabbed docking, graph visualization or a dedicated material asset editor.

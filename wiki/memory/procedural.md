# Procedural Recipes & Coding Patterns

> **Guideline**: Proven patterns and recipes for Bevy-Editor. Read on demand before implementing new features.
> Confidence: 🟢 Confirmed

---

## 1. Registering a New Modular Panel

To add a new dockable panel (e.g. Outliner, Console, Shader Graph) without modifying existing UI modules:

```rust
// 1. Define panel marker component and toggle message
#[derive(Component)]
pub struct OutlinerDockRoot;

#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleOutlinerDockMessage;

// 2. Register panel in PanelRegistry during plugin initialization
app.world_mut()
    .resource_mut::<PanelRegistry>()
    .register("outliner", "Outliner", true);

// 3. Register message and update system
app.add_message::<ToggleOutlinerDockMessage>()
   .add_systems(Startup, spawn_outliner_panel)
   .add_systems(Update, handle_outliner_toggle);
```

---

## 2. Safe Scene Clearance (Zero Engine Entity Despawn)

**Gotcha**: Never call `world.despawn(0v0)` or `world.iter_entities()` without filters. Engine resources and window entities will panic.

```rust
// CORRECT: Target only authorable user scene entities
world.despawn_all::<With<SceneEntity>>();

// Spawning new authorable entities must always attach SceneEntity:
commands.spawn((
    SceneEntity,
    Name::new("New Object"),
    Mesh3d(mesh_handle),
    Transform::IDENTITY,
));
```

---

## 3. Two-Way Inspector Transform Editing

```rust
// Reading transform values for UI display:
let (pos, rot, scale) = if state.is_global_coords {
    let t = global_transform.compute_transform();
    let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);
    (t.translation, Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()), t.scale)
} else {
    let (rx, ry, rz) = transform.rotation.to_euler(EulerRot::XYZ);
    (transform.translation, Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()), transform.scale)
};

// Mutating transform via action / nudge:
pub fn handle_transform_nudges(
    mut nudges: MessageReader<TransformNudgeAction>,
    selection: Res<InspectorSelection>,
    mut transforms: Query<&mut Transform>,
) {
    if let Some(selected) = selection.0 {
        if let Ok(mut transform) = transforms.get_mut(selected) {
            for nudge in nudges.read() {
                // Apply delta directly to component
            }
        }
    }
}
```

---

## 4. Smart Asset Ingestion & Drop Target Resolution

```rust
// Copy external file to project assets first, then load project-relative path
let destination = target_dir.join(&file_name);
fs::copy(source_path, &destination)?;

// Convert to project-relative path for AssetServer:
let relative_path = destination.strip_prefix("assets")?.to_string_lossy().replace('\\', "/");
let handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(relative_path));
```

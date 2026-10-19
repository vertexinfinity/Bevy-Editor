//! Makes Ctrl+Z / Ctrl+Y cover every authoring change, not only edits that were written
//! as explicit commands.
//!
//! - **Component edits** on the selected entity (gizmo drags, transform nudges, light and
//!   script buttons, add/remove component, material colour changes) are detected by
//!   comparing reflected snapshots and recorded as one undo step per finished change. A
//!   gizmo drag becomes a single step when the mouse is released.
//! - **Spawns** (hierarchy Add menu, duplicate, asset drops, RPC) are recorded so undo
//!   removes the new entity and redo brings it back.
//! - **Deletes** go through [`delete_with_undo`], which snapshots the entity and its
//!   children so undo restores them.
//!
//! Changes made by commands already in the history bump `CommandHistory::epoch`; the
//! tracker re-baselines on those so nothing is recorded twice.
use std::any::TypeId;

use bevy::{
    ecs::{entity::EntityHashMap, reflect::ReflectComponent},
    inspector::InspectorSelection,
    prelude::*,
    reflect::PartialReflect,
    world_serialization::{DynamicWorld, DynamicWorldBuilder},
};

use super::commands::{CommandHistory, EditorCommand};
use super::play_mode::{PlayState, PreviewEntity};
use crate::scene::{EditorInternal, SceneEntity};

/// Components that engines systems recompute or the editor toggles for its own UI.
const UNTRACKED: &[&str] = &[
    "GlobalTransform", "InheritedVisibility", "ViewVisibility", "VisibilityClass", "Children", "ChildOf",
    "TransformTreeChanged", "Aabb", "Frustum", "CascadesFrusta", "CubemapFrusta", "Cascades",
    "VisibleMeshEntities", "CubemapVisibleEntities", "CascadesVisibleEntities", "VisibleEntities",
    "RenderVisibleEntities", "TransformGizmoFocus", "Camera", "SyncToRenderWorld", "RenderEntity",
    "MainEntity", "PickingInteraction", "Hovered", "ComputedNode", "ComputedUiTargetCamera",
    "ComputedUiRenderTargetInfo", "CameraMainTextureUsages", "Lightmap", "NoAutomaticBatching",
    "TemporaryRenderEntity", "PreviewEntity", "EditorInternal",
    // UI layout results recomputed a frame after an edit.
    "UiGlobalTransform", "ComputedStackIndex", "ComputedTextBlock", "TextLayoutInfo", "ImageNodeSize",
    "ContentSize", "CalculatedClip", "Pressed", "Interaction", "ComputedNodeTarget",
];

fn short(path: &str) -> &str {
    let base = path.split('<').next().unwrap_or(path);
    base.rsplit("::").next().unwrap_or(base)
}

/// Reflected copy of an entity's authoring state.
pub struct Snapshot {
    components: Vec<(TypeId, Box<dyn PartialReflect>)>,
    material: Option<(AssetId<StandardMaterial>, StandardMaterial)>,
}

impl Snapshot {
    pub fn capture(world: &World, entity: Entity) -> Option<Self> {
        let entity_ref = world.get_entity(entity).ok()?;
        let registry = world.resource::<AppTypeRegistry>().read();
        let mut components = Vec::new();
        for id in entity_ref.archetype().components() {
            let Some(type_id) = world.components().get_info(*id).and_then(|i| i.type_id()) else { continue };
            let Some(registration) = registry.get(type_id) else { continue };
            if UNTRACKED.contains(&short(registration.type_info().type_path())) {
                continue;
            }
            let Some(reflect) = registration.data::<ReflectComponent>() else { continue };
            let Some(value) = reflect.reflect(entity_ref) else { continue };
            if let Ok(clone) = value.reflect_clone() {
                components.push((type_id, clone.into_partial_reflect()));
            }
        }
        let material = world
            .get::<MeshMaterial3d<StandardMaterial>>(entity)
            .and_then(|m| world.resource::<Assets<StandardMaterial>>().get(&m.0).map(|mat| (m.0.id(), mat.clone())));
        Some(Self { components, material })
    }

    fn same_as(&self, other: &Self) -> bool {
        if self.components.len() != other.components.len() {
            return false;
        }
        for (type_id, value) in &self.components {
            let Some((_, other_value)) = other.components.iter().find(|(t, _)| t == type_id) else { return false };
            let equal = value
                .reflect_partial_eq(other_value.as_ref())
                .unwrap_or_else(|| format!("{value:?}") == format!("{other_value:?}"));
            if !equal {
                return false;
            }
        }
        match (&self.material, &other.material) {
            (Some((a_id, a)), Some((b_id, b))) => a_id == b_id && format!("{a:?}") == format!("{b:?}"),
            (None, None) => true,
            _ => false,
        }
    }

    /// Puts the entity back into this state: overwrite tracked components, remove ones
    /// that were added since, and restore the material asset's values.
    pub fn restore(&self, world: &mut World, entity: Entity) -> Result<(), String> {
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let current = Snapshot::capture(world, entity).ok_or("Entity no longer exists")?;
        let mut entity_mut = world.get_entity_mut(entity).map_err(|_| "Entity no longer exists")?;
        for (type_id, _) in &current.components {
            if !self.components.iter().any(|(t, _)| t == type_id) {
                if let Some(reflect) = registry.get_type_data::<ReflectComponent>(*type_id) {
                    reflect.remove(&mut entity_mut);
                }
            }
        }
        for (type_id, value) in &self.components {
            if let Some(reflect) = registry.get_type_data::<ReflectComponent>(*type_id) {
                reflect.insert(&mut entity_mut, value.as_ref(), &registry);
            }
        }
        if let Some((id, material)) = &self.material {
            if let Some(mut target) = world.resource_mut::<Assets<StandardMaterial>>().get_mut(*id) {
                *target = material.clone();
            }
        }
        Ok(())
    }
}

struct EditComponents {
    entity: Entity,
    before: Snapshot,
    after: Snapshot,
}

impl EditorCommand for EditComponents {
    fn name(&self) -> &str {
        "Edit entity"
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        self.after.restore(world, self.entity)
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        self.before.restore(world, self.entity)
    }
}

/// Entities plus all their descendants, as a reflected scene fragment.
fn snapshot_entities(world: &World, roots: &[Entity]) -> DynamicWorld {
    let mut all = Vec::new();
    for root in roots {
        let mut stack = vec![*root];
        while let Some(e) = stack.pop() {
            if world.get_entity(e).is_err() {
                continue;
            }
            all.push(e);
            if let Some(children) = world.get::<Children>(e) {
                stack.extend(children.iter());
            }
        }
    }
    let registry = world.resource::<AppTypeRegistry>().read();
    DynamicWorldBuilder::from_world(world, &registry)
        .deny_component::<bevy::gizmos::transform_gizmo::TransformGizmoFocus>()
        .deny_component::<bevy::camera::visibility::VisibilityClass>()
        .deny_component::<bevy::camera::visibility::ViewVisibility>()
        .extract_entities(all.into_iter())
        .build()
}

/// Writes a fragment back and returns the new ids of `roots`.
fn restore_entities(world: &mut World, fragment: &DynamicWorld, roots: &[Entity]) -> Result<Vec<Entity>, String> {
    let mut map = EntityHashMap::default();
    fragment.write_to_world(world, &mut map).map_err(|e| format!("Cannot restore entities: {e}"))?;
    Ok(roots.iter().filter_map(|r| map.get(r).copied()).collect())
}

/// Spawned (redo) or removed (undo) entities, kept as a reflected fragment while absent.
struct SpawnOrDelete {
    /// Ids the entities had when the fragment was captured (keys of the fragment).
    original: Vec<Entity>,
    /// Ids they have in the world right now (empty while deleted).
    live: Vec<Entity>,
    fragment: Option<DynamicWorld>,
    spawned: bool,
}

impl SpawnOrDelete {
    fn remove(&mut self, world: &mut World) -> Result<(), String> {
        self.original = self.live.clone();
        self.fragment = Some(snapshot_entities(world, &self.live));
        for e in self.live.drain(..) {
            if let Ok(entity) = world.get_entity_mut(e) {
                entity.despawn();
            }
        }
        let selection = world.resource::<InspectorSelection>().0;
        if selection.is_some_and(|s| world.get_entity(s).is_err()) {
            world.resource_mut::<InspectorSelection>().0 = None;
        }
        Ok(())
    }
    fn bring_back(&mut self, world: &mut World) -> Result<(), String> {
        let fragment = self.fragment.take().ok_or("Nothing to restore")?;
        self.live = restore_entities(world, &fragment, &self.original)?;
        if let Some(first) = self.live.first() {
            world.resource_mut::<InspectorSelection>().0 = Some(*first);
        }
        Ok(())
    }
}

impl EditorCommand for SpawnOrDelete {
    fn name(&self) -> &str {
        if self.spawned { "Spawn entity" } else { "Delete entity" }
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        if self.spawned { self.bring_back(world) } else { self.remove(world) }
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        if self.spawned { self.remove(world) } else { self.bring_back(world) }
    }
}

/// Deletes a scene entity (and its children) so that Ctrl+Z restores it.
pub fn delete_with_undo(world: &mut World, entity: Entity) -> Result<(), String> {
    if world.get_entity(entity).is_err() {
        return Err("Entity no longer exists".into());
    }
    let command = SpawnOrDelete { original: vec![entity], live: vec![entity], fragment: None, spawned: false };
    super::reflect_fields::execute_command(world, command)
}

#[derive(Default)]
pub struct TrackerState {
    entity: Option<Entity>,
    baseline: Option<Snapshot>,
    epoch: u64,
    last_tick: Option<bevy::ecs::change_detection::Tick>,
    frames_in_edit: u32,
    material_cursor: bevy::ecs::message::MessageCursor<AssetEvent<StandardMaterial>>,
}

/// Records direct (non-command) edits and spawns as undo steps. Runs in `Last`.
pub fn track_undoable_changes(world: &mut World, mut state: Local<TrackerState>) {
    let editing = world.get_resource::<PlayState>().is_some_and(|s| *s == PlayState::Edit)
        && world.get_resource::<State<crate::ui::EditorAppState>>().is_some_and(|s| *s.get() == crate::ui::EditorAppState::InEditor);
    let Some(epoch) = world.get_resource::<CommandHistory>().map(|h| h.epoch) else { return };
    let this_tick = world.read_change_tick();
    let last_tick = state.last_tick.replace(this_tick);
    let material_events: Vec<AssetId<StandardMaterial>> = world
        .get_resource::<Messages<AssetEvent<StandardMaterial>>>()
        .map(|m| {
            state
                .material_cursor
                .read(m)
                .filter_map(|e| if let AssetEvent::Modified { id } = e { Some(*id) } else { None })
                .collect()
        })
        .unwrap_or_default();

    // New authoring roots this frame (ignored while loading scenes or right after start).
    let new_roots: Vec<Entity> = world
        .query_filtered::<(Entity, Option<&ChildOf>), (Added<SceneEntity>, Without<PreviewEntity>, Without<EditorInternal>)>()
        .iter(world)
        .filter(|(_, parent)| parent.is_none_or(|p| world.get::<SceneEntity>(p.parent()).is_none()))
        .map(|(e, _)| e)
        .collect();

    if !editing {
        state.frames_in_edit = 0;
        state.entity = None;
        return;
    }
    state.frames_in_edit = state.frames_in_edit.saturating_add(1);

    let history_changed = epoch != state.epoch;
    let scene_loading = world.get_resource_ref::<crate::scene::CurrentScene>().is_some_and(|c| c.is_changed());
    if !new_roots.is_empty() && !history_changed && !scene_loading && state.frames_in_edit > 30 && new_roots.len() <= 24 {
        let command = SpawnOrDelete { original: new_roots.clone(), live: new_roots, fragment: None, spawned: true };
        world.resource_mut::<CommandHistory>().record(Box::new(command));
    }

    let selected = world.resource::<InspectorSelection>().0.filter(|e| world.get::<SceneEntity>(*e).is_some());
    let epoch = world.resource::<CommandHistory>().epoch;
    if selected != state.entity || epoch != state.epoch || state.baseline.is_none() {
        state.entity = selected;
        state.epoch = epoch;
        state.baseline = selected.and_then(|e| Snapshot::capture(world, e));
        return;
    }
    let Some(entity) = selected else { return };

    // Keep a gizmo drag (or any mouse-held edit) as one step: wait for the release.
    let dragging = world.get_resource::<bevy::gizmos::transform_gizmo::TransformGizmoState>().is_some_and(|g| g.active)
        || world.get_resource::<ButtonInput<MouseButton>>().is_some_and(|m| m.pressed(MouseButton::Left));
    if dragging {
        return;
    }

    let Ok(entity_ref) = world.get_entity(entity) else { return };
    let touched = last_tick.is_none_or(|last| {
        entity_ref
            .archetype()
            .components()
            .iter()
            .any(|id| entity_ref.get_change_ticks_by_id(*id).is_some_and(|t| t.is_changed(last, this_tick)))
    }) || world
        .get::<MeshMaterial3d<StandardMaterial>>(entity)
        .is_some_and(|m| material_events.contains(&m.0.id()));
    if !touched {
        return;
    }
    let Some(current) = Snapshot::capture(world, entity) else { return };
    let changed = state.baseline.as_ref().is_some_and(|b| !b.same_as(&current));
    if !changed {
        return;
    }
    let before = state.baseline.replace(Snapshot::capture(world, entity).unwrap_or(Snapshot { components: Vec::new(), material: None }));
    if let Some(before) = before {
        world.resource_mut::<CommandHistory>().record(Box::new(EditComponents { entity, before, after: current }));
    }
    state.epoch = world.resource::<CommandHistory>().epoch;
}

/// Delete key removes the selection (with undo) when no text field is being edited.
pub fn delete_key(world: &mut World) {
    let pressed = world.resource::<ButtonInput<KeyCode>>().just_pressed(KeyCode::Delete);
    if !pressed || world.get_resource::<PlayState>().is_some_and(|s| *s != PlayState::Edit) {
        return;
    }
    if world.get_resource::<crate::ui::inspector::property_editor::PropertyEditState>().is_some_and(|e| e.is_editing())
        || world.get_resource::<crate::ui::hierarchy::HierarchyState>().is_some_and(|h| h.search_focused)
    {
        return;
    }
    if let Some(entity) = world.resource::<InspectorSelection>().0.filter(|e| world.get::<SceneEntity>(*e).is_some()) {
        if let Err(error) = delete_with_undo(world, entity) {
            warn!("{error}");
        }
    }
}

pub struct UndoTrackerPlugin;

impl Plugin for UndoTrackerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, delete_key.run_if(in_state(crate::ui::EditorAppState::InEditor)))
            .add_systems(Last, track_undoable_changes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        {
            let registry = world.resource::<AppTypeRegistry>();
            let mut registry = registry.write();
            registry.register::<Transform>();
            registry.register::<Name>();
            registry.register::<SceneEntity>();
            registry.register::<PointLight>();
        }
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<CommandHistory>();
        world.init_resource::<InspectorSelection>();
        world
    }

    #[test]
    fn snapshot_restore_reverts_edits_and_added_components() {
        let mut world = world();
        let e = world.spawn((SceneEntity, Name::new("Box"), Transform::from_xyz(1.0, 0.0, 0.0))).id();
        let before = Snapshot::capture(&world, e).unwrap();
        world.get_mut::<Transform>(e).unwrap().translation.x = 5.0;
        world.entity_mut(e).insert(PointLight::default());
        let after = Snapshot::capture(&world, e).unwrap();
        assert!(!before.same_as(&after));
        before.restore(&mut world, e).unwrap();
        assert_eq!(world.get::<Transform>(e).unwrap().translation.x, 1.0);
        assert!(world.get::<PointLight>(e).is_none());
        after.restore(&mut world, e).unwrap();
        assert_eq!(world.get::<Transform>(e).unwrap().translation.x, 5.0);
    }

    #[test]
    fn delete_is_undoable() {
        let mut world = world();
        let e = world.spawn((SceneEntity, Name::new("Crate"), Transform::from_xyz(2.0, 0.0, 0.0))).id();
        delete_with_undo(&mut world, e).unwrap();
        assert!(world.get_entity(e).is_err());
        world.resource_scope(|world, mut h: Mut<CommandHistory>| h.undo(world)).unwrap();
        let restored = world.query::<(&Name, &Transform)>().iter(&world).find(|(n, _)| n.as_str() == "Crate").map(|(_, t)| t.translation.x);
        assert_eq!(restored, Some(2.0));
    }
}

//! Editor integration: undoable physics edits, the "Physics" inspector card, collider gizmos
//! and a `<scene>.physics.json` sidecar for the remote JSON scene manifest.
use super::components::*;
use super::presets;
use super::runtime::{fit_collider, resolve_shape};
use super::solver::Shape;
use crate::core::commands::EditorCommand;
use crate::core::play_mode::{PlayState, PreviewEntity};
use crate::core::reflect_fields;
use crate::scene::{CurrentScene, EditorInternal, SceneEntity};
use crate::ui::inspector::camera_card::toggle_row;
use crate::ui::theme::*;
use bevy::{
    camera::primitives::Aabb,
    inspector::InspectorSelection,
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------------------------
// Undo
// ---------------------------------------------------------------------------------------------

/// The three physics components of one entity.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PhysicsSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rigid_body: Option<RigidBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collider: Option<Collider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<PhysicsMaterial>,
}

impl PhysicsSnapshot {
    pub fn read(world: &World, entity: Entity) -> Self {
        Self {
            rigid_body: world.get::<RigidBody>(entity).cloned(),
            collider: world.get::<Collider>(entity).cloned(),
            material: world.get::<PhysicsMaterial>(entity).cloned(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rigid_body.is_none() && self.collider.is_none() && self.material.is_none()
    }

    fn apply(&self, world: &mut World, entity: Entity) -> Result<(), String> {
        let mut e = world.get_entity_mut(entity).map_err(|_| "Entity no longer exists".to_string())?;
        match &self.rigid_body {
            Some(v) => e.insert(v.clone()),
            None => e.remove::<RigidBody>(),
        };
        match &self.collider {
            Some(v) => e.insert(v.clone()),
            None => e.remove::<Collider>(),
        };
        match &self.material {
            Some(v) => e.insert(v.clone()),
            None => e.remove::<PhysicsMaterial>(),
        };
        Ok(())
    }
}

/// One undo step replacing an entity's physics components.
pub struct PhysicsEdit {
    pub entity: Entity,
    pub label: &'static str,
    pub before: PhysicsSnapshot,
    pub after: PhysicsSnapshot,
}

impl EditorCommand for PhysicsEdit {
    fn name(&self) -> &str {
        self.label
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        self.after.apply(world, self.entity)
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        self.before.apply(world, self.entity)
    }
}

/// Applies `edit` to the entity's physics components through the undo history.
pub fn edit_physics(world: &mut World, entity: Entity, label: &'static str, edit: impl FnOnce(&World, &mut PhysicsSnapshot)) -> Result<PhysicsSnapshot, String> {
    if world.get_entity(entity).is_err() {
        return Err("Entity no longer exists".into());
    }
    let before = PhysicsSnapshot::read(world, entity);
    let mut after = before.clone();
    edit(world, &mut after);
    if after != before {
        reflect_fields::execute_command(world, PhysicsEdit { entity, label, before, after: after.clone() })?;
    }
    Ok(after)
}

/// A collider fitted to the entity's mesh.
pub fn fitted_collider(world: &World, entity: Entity) -> Collider {
    let mesh = world.get::<Mesh3d>(entity).and_then(|m| world.get_resource::<Assets<Mesh>>().and_then(|a| a.get(&m.0)));
    fit_collider(mesh, world.get::<Aabb>(entity))
}

// ---------------------------------------------------------------------------------------------
// Inspector card
// ---------------------------------------------------------------------------------------------

#[derive(Component)]
pub struct PhysicsCardRoot;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum PhysicsChip {
    Body,
    BodyType,
    Collider,
    Shape,
    Trigger,
    Material,
    SavePreset,
}

#[derive(Component)]
struct PhysicsCardStatus;

/// Last status message shown at the bottom of the card.
#[derive(Resource, Default)]
pub struct PhysicsCardMessage(pub String);

pub fn spawn_physics_card(commands: &mut Commands, body: Entity) {
    commands.entity(body).insert(PhysicsCardRoot);
    let rows: [(&str, PhysicsChip); 7] = [
        ("Rigid Body", PhysicsChip::Body),
        ("Body Type", PhysicsChip::BodyType),
        ("Collider", PhysicsChip::Collider),
        ("Shape", PhysicsChip::Shape),
        ("Is Trigger", PhysicsChip::Trigger),
        ("Material Preset", PhysicsChip::Material),
        ("Save Material as Preset", PhysicsChip::SavePreset),
    ];
    for (label, chip_kind) in rows {
        let chip = toggle_row(commands, body, label, "");
        commands.entity(chip).insert(chip_kind).observe(move |click: On<PointerClick>, mut commands: Commands| {
            if click.button != PointerButton::Primary {
                return;
            }
            commands.queue(move |world: &mut World| {
                if *world.resource::<PlayState>() != PlayState::Edit {
                    return;
                }
                let Some(entity) = world.resource::<InspectorSelection>().0 else { return };
                let message = match click_chip(world, entity, chip_kind) {
                    Ok(Some(message)) => message,
                    Ok(None) => return,
                    Err(error) => {
                        warn!("{error}");
                        error
                    }
                };
                world.resource_mut::<PhysicsCardMessage>().0 = message;
            });
        });
    }
    commands.spawn((
        EditorInternal,
        PhysicsCardStatus,
        Pickable::IGNORE,
        Text::new(""),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default() },
        TextColor(TEXT_MUTED),
        Node { margin: UiRect::top(px(6)), ..default() },
        ChildOf(body),
    ));
}

fn next<T: Copy + PartialEq>(all: &[T], current: T) -> T {
    all[(all.iter().position(|v| *v == current).unwrap_or(0) + 1) % all.len()]
}

fn click_chip(world: &mut World, entity: Entity, chip: PhysicsChip) -> Result<Option<String>, String> {
    match chip {
        PhysicsChip::Body => {
            let has = world.get::<RigidBody>(entity).is_some();
            let collider = fitted_collider(world, entity);
            edit_physics(world, entity, if has { "Remove Rigid Body" } else { "Add Rigid Body" }, |_, s| {
                if has {
                    s.rigid_body = None;
                } else {
                    s.rigid_body = Some(RigidBody::default());
                    s.collider.get_or_insert(collider);
                }
            })?;
        }
        PhysicsChip::BodyType => {
            if world.get::<RigidBody>(entity).is_none() {
                return Ok(Some("Add a Rigid Body first".into()));
            }
            edit_physics(world, entity, "Change Body Type", |_, s| {
                if let Some(rb) = &mut s.rigid_body {
                    rb.kind = next(&BodyKind::ALL, rb.kind);
                }
            })?;
        }
        PhysicsChip::Collider => {
            let has = world.get::<Collider>(entity).is_some();
            let collider = fitted_collider(world, entity);
            edit_physics(world, entity, if has { "Remove Collider" } else { "Add Collider" }, |_, s| {
                s.collider = if has { None } else { Some(collider) };
            })?;
        }
        PhysicsChip::Shape => {
            if world.get::<Collider>(entity).is_none() {
                return Ok(Some("Add a Collider first".into()));
            }
            let fitted = fitted_collider(world, entity);
            edit_physics(world, entity, "Change Collider Shape", |_, s| {
                if let Some(c) = &mut s.collider {
                    c.shape = next(&ColliderShape::ALL, c.shape);
                    set_shape_size(c, &fitted);
                }
            })?;
        }
        PhysicsChip::Trigger => {
            if world.get::<Collider>(entity).is_none() {
                return Ok(Some("Add a Collider first".into()));
            }
            edit_physics(world, entity, "Toggle Trigger", |_, s| {
                if let Some(c) = &mut s.collider {
                    c.is_trigger = !c.is_trigger;
                }
            })?;
        }
        PhysicsChip::Material => {
            let all = presets::all_presets();
            let current = world.get::<PhysicsMaterial>(entity).map(|m| m.preset.to_lowercase()).unwrap_or_else(|| "default".into());
            let index = all.iter().position(|p| p.preset.to_lowercase() == current).map(|i| (i + 1) % all.len()).unwrap_or(0);
            let preset = all[index].clone();
            let name = preset.preset.clone();
            edit_physics(world, entity, "Apply Physics Material", |_, s| s.material = Some(preset))?;
            return Ok(Some(format!("Material: {name}")));
        }
        PhysicsChip::SavePreset => {
            let material = world.get::<PhysicsMaterial>(entity).cloned().unwrap_or_default();
            let mut name = material.preset.trim().to_string();
            if name.is_empty() || presets::is_builtin(&name) {
                let base = world.get::<Name>(entity).map(|n| n.as_str().to_string()).unwrap_or_else(|| "Custom".into());
                name = format!("{base} Material");
            }
            let path = presets::save_preset(&name, &material)?;
            edit_physics(world, entity, "Name Physics Material", |_, s| {
                let mut m = material.clone();
                m.preset = name.clone();
                s.material = Some(m);
            })?;
            return Ok(Some(format!("Saved '{name}' to {}", path.display())));
        }
    }
    Ok(None)
}

/// Gives a newly chosen explicit shape sensible dimensions from the mesh fit.
pub fn set_shape_size(collider: &mut Collider, fitted: &Collider) {
    let extent = if fitted.shape == ColliderShape::Sphere { Vec3::splat(fitted.size.x * 2.0) } else { fitted.size };
    collider.offset = fitted.offset;
    collider.size = match collider.shape {
        ColliderShape::Sphere => Vec3::splat(extent.max_element() * 0.5),
        ColliderShape::Capsule => Vec3::new(extent.x.max(extent.z) * 0.5, extent.y, 0.0),
        _ => extent,
    };
}

pub fn update_physics_card(world: &mut World) {
    let selected = world
        .resource::<InspectorSelection>()
        .0
        .filter(|e| world.get::<Mesh3d>(*e).is_some() && world.get::<EditorInternal>(*e).is_none() && world.get::<PreviewEntity>(*e).is_none());
    let Some(card) = world.query_filtered::<&ChildOf, With<PhysicsCardRoot>>().iter(world).next().map(|c| c.parent()) else { return };
    if let Some(mut node) = world.get_mut::<Node>(card) {
        let display = if selected.is_some() { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    let Some(entity) = selected else { return };
    let rb = world.get::<RigidBody>(entity).cloned();
    let collider = world.get::<Collider>(entity).cloned();
    let material = world.get::<PhysicsMaterial>(entity).cloned();
    let chips: Vec<(Entity, PhysicsChip)> = world.query::<(Entity, &PhysicsChip)>().iter(world).map(|(e, c)| (e, *c)).collect();
    for (chip, kind) in chips {
        let (text, on) = match kind {
            PhysicsChip::Body => if rb.is_some() { ("Remove".to_string(), true) } else { ("+ Add".to_string(), false) },
            PhysicsChip::BodyType => match &rb {
                Some(rb) => (format!("{:?}", rb.kind), true),
                None => ("Static (no body)".into(), false),
            },
            PhysicsChip::Collider => if collider.is_some() { ("Remove".to_string(), true) } else { ("+ Add".to_string(), false) },
            PhysicsChip::Shape => match &collider {
                Some(c) if c.shape == ColliderShape::Auto => (format!("Auto ({:?})", fitted_collider(world, entity).shape), true),
                Some(c) => (format!("{:?}", c.shape), true),
                None => ("—".into(), false),
            },
            PhysicsChip::Trigger => match &collider {
                Some(c) => (if c.is_trigger { "On" } else { "Off" }.into(), c.is_trigger),
                None => ("—".into(), false),
            },
            PhysicsChip::Material => (presets::material_label(material.as_ref()), material.is_some()),
            PhysicsChip::SavePreset => ("Save".into(), false),
        };
        let Some(child) = world.get::<Children>(chip).and_then(|c| c.first().copied()) else { continue };
        if let Some(mut t) = world.get_mut::<Text>(child) {
            if t.0 != text {
                t.0 = text;
            }
        }
        if let Some(mut color) = world.get_mut::<TextColor>(child) {
            let next = TextColor(if on { ACCENT_CYAN } else { TEXT_MUTED });
            if *color != next {
                *color = next;
            }
        }
    }
    let message = world.resource::<PhysicsCardMessage>().0.clone();
    let hint = if message.is_empty() { "Values are editable in the Components card. Save uses the material's Preset Name.".to_string() } else { message };
    let status: Vec<Entity> = world.query_filtered::<Entity, With<PhysicsCardStatus>>().iter(world).collect();
    for e in status {
        if let Some(mut t) = world.get_mut::<Text>(e) {
            if t.0 != hint {
                t.0 = hint.clone();
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Gizmos
// ---------------------------------------------------------------------------------------------

#[allow(clippy::type_complexity)]
pub fn draw_collider_gizmos(
    selection: Res<InspectorSelection>,
    settings: Res<PhysicsSettings>,
    view: Res<crate::core::selection::ViewportGizmoSettings>,
    meshes: Res<Assets<Mesh>>,
    query: Query<(Entity, &GlobalTransform, Option<&Collider>, Option<&RigidBody>, Option<&Mesh3d>, Option<&Aabb>), (Or<(With<Collider>, With<RigidBody>)>, Without<PreviewEntity>, Without<EditorInternal>)>,
    mut gizmos: Gizmos,
) {
    if !view.show_gizmos {
        return;
    }
    for (entity, gt, collider, rb, mesh3d, aabb) in &query {
        if !settings.show_all_colliders && selection.0 != Some(entity) {
            continue;
        }
        let (scale, rotation, translation) = gt.to_scale_rotation_translation();
        let default_collider = Collider::default();
        let collider = collider.unwrap_or(&default_collider);
        let (shape, offset, _) = resolve_shape(collider, mesh3d.and_then(|m| meshes.get(&m.0)), aabb, scale);
        let color = if collider.is_trigger {
            Color::srgba(1.0, 0.85, 0.2, 0.9)
        } else {
            match rb.map(|r| r.kind) {
                Some(BodyKind::Dynamic) => Color::srgba(0.3, 1.0, 0.45, 0.9),
                Some(BodyKind::Kinematic) => Color::srgba(1.0, 0.55, 0.15, 0.9),
                _ => Color::srgba(0.55, 0.7, 1.0, 0.9),
            }
        };
        let iso = Isometry3d::new(translation + rotation * offset, rotation);
        match shape {
            Shape::Box { half } => {
                gizmos.primitive_3d(&Cuboid::from_size(half * 2.0), iso, color);
            }
            Shape::Sphere { radius } => {
                gizmos.sphere(iso, radius, color);
            }
            Shape::Capsule { radius, half_height } => {
                gizmos.primitive_3d(&Capsule3d::new(radius, half_height * 2.0), iso, color);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Scene sidecar
// ---------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Default)]
struct SidecarEntry {
    name: String,
    #[serde(flatten)]
    physics: PhysicsSnapshot,
}

#[derive(Serialize, Deserialize, Default)]
struct Sidecar {
    entities: Vec<SidecarEntry>,
}

pub fn sidecar_path(scene: &Path) -> PathBuf {
    let mut s = scene.as_os_str().to_owned();
    s.push(".physics.json");
    PathBuf::from(s)
}

/// Remembers a few scene entities so a scene change can be told apart: if they all vanished
/// the scene was (re)loaded, otherwise it was saved.
#[derive(Default)]
pub struct SidecarWatch {
    sample: Vec<Entity>,
}

/// The remote `editor/save_scene` / `editor/load_scene` JSON manifest only keeps meshes,
/// materials and lights, so physics components are mirrored into `<scene>.physics.json` on
/// save and restored (by entity name) on load. RON scene loads already keep them.
pub fn sync_physics_sidecar(world: &mut World, mut watch: Local<SidecarWatch>, mut last_tick: Local<Option<bevy::ecs::change_detection::Tick>>) {
    let changed_path = {
        let Some(scene) = world.get_resource_ref::<CurrentScene>() else { return };
        let tick = scene.last_changed();
        let changed = last_tick.is_some_and(|t| t != tick);
        *last_tick = Some(tick);
        (changed && !scene.is_modified).then(|| scene.file_path.clone()).flatten()
    };
    if let Some(path) = changed_path {
        let loaded = !watch.sample.is_empty() && watch.sample.iter().all(|e| world.get_entity(*e).is_err());
        if loaded {
            restore_sidecar(world, &path);
        } else if let Err(error) = write_sidecar(world, &path) {
            warn!("Could not write physics sidecar: {error}");
        }
    }
    watch.sample = world.query_filtered::<Entity, (With<SceneEntity>, Without<EditorInternal>)>().iter(world).take(16).collect();
}

fn write_sidecar(world: &mut World, scene: &Path) -> Result<(), String> {
    let entities: Vec<SidecarEntry> = world
        .query_filtered::<(Entity, &Name), (With<SceneEntity>, Without<EditorInternal>, Without<PreviewEntity>)>()
        .iter(world)
        .map(|(e, n)| SidecarEntry { name: n.as_str().to_string(), physics: PhysicsSnapshot::read(world, e) })
        .filter(|e| !e.physics.is_empty())
        .collect();
    let path = sidecar_path(scene);
    if entities.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let json = serde_json::to_string_pretty(&Sidecar { entities }).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

fn restore_sidecar(world: &mut World, scene: &Path) {
    let Ok(text) = std::fs::read_to_string(sidecar_path(scene)) else { return };
    let sidecar: Sidecar = match serde_json::from_str(&text) {
        Ok(s) => s,
        Err(error) => {
            warn!("Invalid physics sidecar for {}: {error}", scene.display());
            return;
        }
    };
    let targets: Vec<(Entity, String)> = world
        .query_filtered::<(Entity, &Name), (With<SceneEntity>, Without<RigidBody>, Without<Collider>, Without<PhysicsMaterial>)>()
        .iter(world)
        .map(|(e, n)| (e, n.as_str().to_string()))
        .collect();
    let mut restored = 0;
    for entry in sidecar.entities {
        if let Some((entity, _)) = targets.iter().find(|(_, n)| *n == entry.name) {
            if entry.physics.apply(world, *entity).is_ok() {
                restored += 1;
            }
        }
    }
    if restored > 0 {
        info!("Restored physics components on {restored} entities from {}", sidecar_path(scene).display());
    }
}

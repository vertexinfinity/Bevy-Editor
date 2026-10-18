//! Selection Management Subsystem.
//!
//! Manages active entity selection (single and multi-select), entity focus,
//! and notifies listeners via [`SelectionChangedMessage`].

#![allow(dead_code)]

use bevy::{
    ecs::message::{Message, MessageWriter},
    inspector::InspectorSelection,
    prelude::*,
};

/// Tracks selected entities in the editor.
#[derive(Resource, Default, Debug, Clone)]
pub struct EditorSelection {
    pub selected: Vec<Entity>,
}

impl EditorSelection {
    pub fn primary(&self) -> Option<Entity> {
        self.selected.first().copied()
    }

    pub fn set_single(&mut self, entity: Entity) {
        self.selected.clear();
        self.selected.push(entity);
    }

    pub fn clear(&mut self) {
        self.selected.clear();
    }

    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected.contains(&entity)
    }

    pub fn toggle(&mut self, entity: Entity) {
        if let Some(pos) = self.selected.iter().position(|e| *e == entity) {
            self.selected.remove(pos);
        } else {
            self.selected.push(entity);
        }
    }
}

#[derive(Message, Clone, Debug)]
pub struct SelectionChangedMessage {
    pub selected: Vec<Entity>,
}

/// Keeps `EditorSelection` and `InspectorSelection` synchronized.
pub fn sync_editor_selection(
    mut editor_sel: ResMut<EditorSelection>,
    inspector_sel: Res<InspectorSelection>,
    mut writer: MessageWriter<SelectionChangedMessage>,
) {
    if inspector_sel.is_changed() {
        if let Some(ent) = inspector_sel.0 {
            if editor_sel.primary() != Some(ent) {
                editor_sel.set_single(ent);
                writer.write(SelectionChangedMessage {
                    selected: editor_sel.selected.clone(),
                });
            }
        } else if !editor_sel.selected.is_empty() {
            editor_sel.clear();
            writer.write(SelectionChangedMessage {
                selected: Vec::new(),
            });
        }
    }
}

/// User-configurable viewport gizmo and overlay visibility settings.
#[derive(Resource, Debug, Clone)]
pub struct ViewportGizmoSettings {
    pub show_bounds: bool,
    pub show_gizmos: bool,
    pub show_grid: bool,
    /// Draw every mesh as wireframe (Bevy `WireframeConfig::global`).
    pub show_wireframe: bool,
}

impl Default for ViewportGizmoSettings {
    fn default() -> Self {
        Self {
            show_bounds: true,
            show_gizmos: true,
            show_grid: true,
            show_wireframe: false,
        }
    }
}

pub struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorSelection>()
            .init_resource::<ViewportGizmoSettings>()
            .add_message::<SelectionChangedMessage>()
            .add_systems(
                Update,
                (
                    sync_editor_selection,
                    sync_selection_and_tools_to_gizmo,
                    draw_selection_gizmo,
                    handle_gizmo_toggle_shortcuts,
                ),
            );
    }
}

/// Keyboard shortcuts for viewport overlays:
/// - Press 'B' to toggle mesh bounding boxes.
/// - Press 'G' to toggle transform gizmos.
pub fn handle_gizmo_toggle_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<ViewportGizmoSettings>,
    hierarchy_state: Option<Res<crate::ui::hierarchy::HierarchyState>>,
    text_edit_state: Option<Res<crate::ui::inspector::TransformTextEditState>>,
) {
    if let Some(hs) = hierarchy_state {
        if hs.search_focused {
            return;
        }
    }
    if let Some(ts) = text_edit_state {
        if ts.active.is_some() {
            return;
        }
    }

    if keys.just_pressed(KeyCode::KeyB) {
        settings.show_bounds = !settings.show_bounds;
        info!("Toggled Bounding Box Gizmos: {}", settings.show_bounds);
    }
    if keys.just_pressed(KeyCode::KeyG) {
        settings.show_gizmos = !settings.show_gizmos;
        info!("Toggled Transform Gizmos: {}", settings.show_gizmos);
    }
}

/// Synchronizes `InspectorSelection` to Bevy's native interactive `TransformGizmoFocus`,
/// and updates `TransformGizmoSettings` according to the active tool.
pub fn sync_selection_and_tools_to_gizmo(
    play_state: Res<crate::core::play_mode::PlayState>,
    inspector_sel: Res<InspectorSelection>,
    active_tool: Res<crate::core::tools::ActiveTool>,
    gizmo_settings: Res<ViewportGizmoSettings>,
    mut settings: ResMut<bevy::gizmos::transform_gizmo::TransformGizmoSettings>,
    focused_entities: Query<Entity, With<bevy::gizmos::transform_gizmo::TransformGizmoFocus>>,
    mut commands: Commands,
) {
    // If gizmos are disabled globally, remove focus to hide handles
    if !gizmo_settings.show_gizmos || *play_state != crate::core::play_mode::PlayState::Edit {
        for e in &focused_entities {
            commands.entity(e).remove::<bevy::gizmos::transform_gizmo::TransformGizmoFocus>();
        }
        return;
    }

    // 1. Sync manipulation mode with active tool (Translate, Rotate, Scale)
    settings.mode = match active_tool.current_id {
        "translate" => bevy::gizmos::transform_gizmo::TransformGizmoMode::Translate,
        "rotate" => bevy::gizmos::transform_gizmo::TransformGizmoMode::Rotate,
        "scale" => bevy::gizmos::transform_gizmo::TransformGizmoMode::Scale,
        _ => bevy::gizmos::transform_gizmo::TransformGizmoMode::Translate,
    };

    // 2. Attach TransformGizmoFocus to the selected entity
    let target = inspector_sel.0;
    for e in &focused_entities {
        if Some(e) != target {
            commands.entity(e).remove::<bevy::gizmos::transform_gizmo::TransformGizmoFocus>();
        }
    }

    if let Some(e) = target {
        if !focused_entities.contains(e) {
            commands.entity(e).insert(bevy::gizmos::transform_gizmo::TransformGizmoFocus);
        }
    }
}

/// Recursively computes the combined world-space Axis-Aligned Bounding Box (AABB)
/// enclosing an entity and all its descendant meshes (e.g. glTF scenes).
/// Returns `Some((world_center, world_size))` if any mesh bounds are found, or `None` if non-spatial.
pub fn compute_entity_world_aabb(
    root: Entity,
    transforms: &Query<&GlobalTransform>,
    aabbs: &Query<&bevy::camera::primitives::Aabb>,
    mesh_handles: &Query<&Mesh3d>,
    meshes: &Assets<Mesh>,
    children_query: &Query<&Children>,
) -> Option<(Vec3, Vec3)> {
    let mut min_pt = Vec3::splat(f32::INFINITY);
    let mut max_pt = Vec3::splat(f32::NEG_INFINITY);
    let mut found = false;

    let mut stack = vec![root];
    while let Some(ent) = stack.pop() {
        if let Ok(gt) = transforms.get(ent) {
            // Check direct Aabb component first
            if let Ok(aabb) = aabbs.get(ent) {
                let center: Vec3 = aabb.center.into();
                let half: Vec3 = aabb.half_extents.into();
                for dx in [-half.x, half.x] {
                    for dy in [-half.y, half.y] {
                        for dz in [-half.z, half.z] {
                            let local_corner = center + Vec3::new(dx, dy, dz);
                            let world_corner = gt.transform_point(local_corner);
                            min_pt = min_pt.min(world_corner);
                            max_pt = max_pt.max(world_corner);
                            found = true;
                        }
                    }
                }
            } else if let Ok(mesh_handle) = mesh_handles.get(ent) {
                // If Aabb component is not yet attached, compute from the Mesh asset
                use bevy::camera::primitives::MeshAabb;
                if let Some(mesh) = meshes.get(&mesh_handle.0) {
                    if let Some(aabb) = mesh.get_aabb() {
                        let center: Vec3 = aabb.center.into();
                        let half: Vec3 = aabb.half_extents.into();
                        for dx in [-half.x, half.x] {
                            for dy in [-half.y, half.y] {
                                for dz in [-half.z, half.z] {
                                    let local_corner = center + Vec3::new(dx, dy, dz);
                                    let world_corner = gt.transform_point(local_corner);
                                    min_pt = min_pt.min(world_corner);
                                    max_pt = max_pt.max(world_corner);
                                    found = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Ok(children) = children_query.get(ent) {
            for child in children.iter() {
                stack.push(child);
            }
        }
    }

    if found {
        let center = (min_pt + max_pt) * 0.5;
        let size = (max_pt - min_pt).max(Vec3::splat(0.05));
        Some((center, size))
    } else {
        None
    }
}

/// Renders a true 3D AABB bounding box, glowing cyan base ring, and coordinate axes on the selected object.
pub fn draw_selection_gizmo(
    inspector_sel: Res<InspectorSelection>,
    transforms: Query<&GlobalTransform>,
    aabbs: Query<&bevy::camera::primitives::Aabb>,
    mesh_handles: Query<&Mesh3d>,
    meshes: Res<Assets<Mesh>>,
    children_query: Query<&Children>,
    gizmo_settings: Res<ViewportGizmoSettings>,
    mut gizmos: Gizmos,
) {
    let Some(selected) = inspector_sel.0 else { return };
    let Ok(gt) = transforms.get(selected) else { return };
    let pos = gt.translation();

    // 1. Draw 3D AABB bounding box calculated from actual mesh bounds across the entity hierarchy
    if gizmo_settings.show_bounds {
        if let Some((world_center, world_size)) = compute_entity_world_aabb(
            selected,
            &transforms,
            &aabbs,
            &mesh_handles,
            &meshes,
            &children_query,
        ) {
            let cuboid = Cuboid::new(world_size.x, world_size.y, world_size.z);
            gizmos.primitive_3d(
                &cuboid,
                Isometry3d::new(world_center, Quat::IDENTITY),
                Color::srgba(0.0, 0.95, 1.0, 0.70),
            );
        }
    }

    // 2. Transform Gizmo orientation overlays
    if gizmo_settings.show_gizmos {
        // Radiant electric cyan selection ring flat on the ground
        gizmos.circle(
            Isometry3d::new(
                pos + Vec3::new(0.0, 0.05, 0.0),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
            ),
            1.25,
            Color::srgb(0.0, 0.95, 1.0),
        );

        // Coordinate axis arrows (Red = X, Green = Y, Blue = Z) indicating transform orientation
        let axis_len = 1.4;
        let rot = gt.to_scale_rotation_translation().1;
        let x_axis = rot * Vec3::X * axis_len;
        let y_axis = rot * Vec3::Y * axis_len;
        let z_axis = rot * Vec3::Z * axis_len;

        gizmos.arrow(pos, pos + x_axis, Color::srgb(0.95, 0.25, 0.25));
        gizmos.arrow(pos, pos + y_axis, Color::srgb(0.25, 0.85, 0.35));
        gizmos.arrow(pos, pos + z_axis, Color::srgb(0.25, 0.65, 0.98));
    }
}

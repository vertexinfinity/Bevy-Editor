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

pub struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorSelection>()
            .add_message::<SelectionChangedMessage>()
            .add_systems(
                Update,
                (
                    sync_editor_selection,
                    sync_selection_and_tools_to_gizmo,
                    draw_selection_gizmo,
                ),
            );
    }
}

/// Synchronizes `InspectorSelection` to Bevy's native interactive `TransformGizmoFocus`,
/// and updates `TransformGizmoSettings` according to the active tool.
pub fn sync_selection_and_tools_to_gizmo(
    inspector_sel: Res<InspectorSelection>,
    active_tool: Res<crate::core::tools::ActiveTool>,
    mut settings: ResMut<bevy::gizmos::transform_gizmo::TransformGizmoSettings>,
    focused_entities: Query<Entity, With<bevy::gizmos::transform_gizmo::TransformGizmoFocus>>,
    mut commands: Commands,
) {
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

/// Renders a 3D AABB bounding box, glowing cyan base ring, and coordinate axes on the selected object.
pub fn draw_selection_gizmo(
    inspector_sel: Res<InspectorSelection>,
    transforms: Query<&GlobalTransform>,
    aabbs: Query<&bevy::camera::primitives::Aabb>,
    active_tool: Res<crate::core::tools::ActiveTool>,
    mut gizmos: Gizmos,
) {
    let Some(selected) = inspector_sel.0 else { return };
    let Ok(gt) = transforms.get(selected) else { return };
    let pos = gt.translation();

    // 1. Draw 3D AABB bounding box or fallback bounding wireframe
    if let Ok(aabb) = aabbs.get(selected) {
        let center = gt.transform_point(aabb.center.into());
        let half_extents = aabb.half_extents;
        let scale = gt.to_scale_rotation_translation().0;
        let cuboid = Cuboid::new(
            half_extents.x * scale.x * 2.0,
            half_extents.y * scale.y * 2.0,
            half_extents.z * scale.z * 2.0,
        );

        gizmos.primitive_3d(
            &cuboid,
            Isometry3d::new(center, gt.to_scale_rotation_translation().1),
            Color::srgba(0.0, 0.95, 1.0, 0.65),
        );
    } else {
        // Fallback wireframe box around transform origin
        let cuboid = Cuboid::new(1.6, 1.6, 1.6);
        gizmos.primitive_3d(
            &cuboid,
            Isometry3d::new(pos + Vec3::new(0.0, 0.75, 0.0), gt.to_scale_rotation_translation().1),
            Color::srgba(0.0, 0.95, 1.0, 0.45),
        );
    }

    // 2. Radiant electric cyan selection ring flat on the ground
    gizmos.circle(
        Isometry3d::new(
            pos + Vec3::new(0.0, 0.05, 0.0),
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        ),
        1.25,
        Color::srgb(0.0, 0.95, 1.0),
    );

    // 3. Coordinate axis arrows (Red = X, Green = Y, Blue = Z) indicating transform orientation
    let axis_len = 1.4;
    let rot = gt.to_scale_rotation_translation().1;
    let x_axis = rot * Vec3::X * axis_len;
    let y_axis = rot * Vec3::Y * axis_len;
    let z_axis = rot * Vec3::Z * axis_len;

    gizmos.arrow(pos, pos + x_axis, Color::srgb(0.95, 0.25, 0.25));
    gizmos.arrow(pos, pos + y_axis, Color::srgb(0.25, 0.85, 0.35));
    gizmos.arrow(pos, pos + z_axis, Color::srgb(0.25, 0.65, 0.98));

    // 4. Mode-specific gizmo overlays
    if active_tool.current_id == "rotate" {
        // Draw 3-axis rotation rings (Radius 1.35)
        let ring_radius = 1.35;
        // Pitch (X-axis ring - Red)
        gizmos.circle(
            Isometry3d::new(pos, rot * Quat::from_rotation_y(core::f32::consts::FRAC_PI_2)),
            ring_radius,
            Color::srgb(0.95, 0.25, 0.25),
        );
        // Yaw (Y-axis ring - Green)
        gizmos.circle(
            Isometry3d::new(pos, rot * Quat::from_rotation_x(core::f32::consts::FRAC_PI_2)),
            ring_radius,
            Color::srgb(0.25, 0.85, 0.35),
        );
        // Roll (Z-axis ring - Blue)
        gizmos.circle(
            Isometry3d::new(pos, rot),
            ring_radius,
            Color::srgb(0.25, 0.65, 0.98),
        );
    }
}

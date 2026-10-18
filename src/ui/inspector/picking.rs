//! 3D viewport entity raycast picking and selection synchronization.
use bevy::{picking::{events::PointerClick, pointer::PointerButton}, prelude::*, inspector::InspectorSelection};
use crate::scene::{EditorInternal};
/// Global observer: clicks on non-editor entities in the 3D scene select them in the inspector.
pub fn select_entity_on_viewport_click(
    click: On<PointerClick>,
    play_state: Res<crate::core::play_mode::PlayState>,
    scene_entities: Query<(), With<crate::scene::SceneEntity>>,
    parent_query: Query<&ChildOf>,
    editor_internal: Query<(), With<EditorInternal>>,
    gizmo_marker: Query<(), With<bevy::gizmos::transform_gizmo::TransformGizmoMeshMarker>>,
    gizmo_root: Query<(), With<bevy::gizmos::transform_gizmo::TransformGizmoRoot>>,
    gizmo_state: Option<Res<bevy::gizmos::transform_gizmo::TransformGizmoState>>,
    mut selection: ResMut<InspectorSelection>,
) {
    if *play_state != crate::core::play_mode::PlayState::Edit || click.button != PointerButton::Primary {
        return;
    }

    // Do not select editor UI elements or transform gizmo handles
    if editor_internal.contains(click.entity)
        || gizmo_marker.contains(click.entity)
        || gizmo_root.contains(click.entity)
    {
        return;
    }

    // If transform gizmo is actively being dragged or was just dragged, do not change selection
    if let Some(state) = gizmo_state {
        if state.active || state.axis.is_some() {
            return;
        }
    }

    // Check if clicked entity is a child of gizmo root
    let mut curr_ancestor = click.entity;
    while let Ok(child_of) = parent_query.get(curr_ancestor) {
        let parent = child_of.parent();
        if gizmo_root.contains(parent) || editor_internal.contains(parent) {
            return;
        }
        curr_ancestor = parent;
    }

    // Resolve clicked entity or its ancestor marked with SceneEntity
    // Climb to the TOPMOST SceneEntity ancestor (glTF descendants are all tagged SceneEntity)
    let mut current = click.entity;
    let mut top: Option<Entity> = None;
    loop {
        if scene_entities.contains(current) {
            top = Some(current);
        }
        if let Ok(child_of) = parent_query.get(current) {
            current = child_of.parent();
        } else {
            break;
        }
    }
    if let Some(root) = top {
        info!("Selected 3D scene object: {:?}", root);
        selection.0 = Some(root);
        return;
    }

    // No fallback: bubbled clicks reach the Window entity and must never become the selection.
}
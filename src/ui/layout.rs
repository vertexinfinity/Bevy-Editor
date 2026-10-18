//! Keep viewport overlays and the bottom dock inside the space between side panels.
use bevy::prelude::*;
use super::{hierarchy::HierarchyDockRoot, inspector::{InspectorDockRoot, InspectorState, PanelDockZone}};

pub fn update_workspace_layout(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    hierarchy: Query<&ComputedNode, With<HierarchyDockRoot>>,
    inspector: Query<&ComputedNode, With<InspectorDockRoot>>,
    inspector_state: Res<InspectorState>,
    mut nodes: Query<(&Name, &mut Node), With<crate::scene::EditorInternal>>,
) {
    let Ok(window) = windows.single() else { return; };
    let left = hierarchy.single().map(|node| node.size().x * node.inverse_scale_factor).unwrap_or(250.0);
    let right = if inspector_state.dock_zone == PanelDockZone::Right {
        inspector.single().map(|node| node.size().x * node.inverse_scale_factor).unwrap_or(380.0)
    } else { 0.0 };
    let available = (window.width() - left - right - 12.0).max(120.0);
    for (name, mut node) in &mut nodes {
        match name.as_str() {
            "ViewportTabBar" | "ViewportToolsBar" => {
                let offset = px(left + 8.0);
                if node.left != offset { node.left = offset; }
            }
            "ProjectExplorerDock" => {
                let offset = px(left + 6.0);
                let width = px(available);
                if node.left != offset { node.left = offset; }
                if node.max_width != width { node.max_width = width; }
                let max_height = px((window.height() * 0.55).max(100.0));
                if node.max_height != max_height { node.max_height = max_height; }
            }
            _ => {}
        }
    }
}

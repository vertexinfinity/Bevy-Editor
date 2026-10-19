//! Streamlined Entity Inspector subsystem with modular docking, transform, scripts, and picking.

pub mod types;
pub mod picking;
pub mod components_card;
pub mod script_card;
pub mod transform_card;
pub mod material_card;
pub mod active_scripts_card;
pub mod light_card;
pub mod dock;
pub mod relations_card;

// Re-export all public symbols for backwards compatibility
pub use types::*;
pub use picking::*;
pub use components_card::*;
pub use script_card::*;
pub use transform_card::*;
pub use material_card::*;
pub use active_scripts_card::*;
pub use light_card::*;
pub use dock::*;


pub mod property_editor;
pub mod extensions;

pub mod camera_card;
pub mod object_card;
pub mod reflect_card;
pub mod world_card;

use bevy::prelude::*;
use extensions::{InspectorSection, InspectorSectionRegistry};

/// Reflection-driven cards: World environment, Camera rendering, and generic Components.
pub struct InspectorFeaturesPlugin;

impl Plugin for InspectorFeaturesPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy::pbr::wireframe::WireframePlugin>() {
            app.add_plugins(bevy::pbr::wireframe::WireframePlugin::default());
        }
        app.init_resource::<InspectorSectionRegistry>()
            .init_resource::<camera_card::ViewportPreview>();
        let mut registry = app.world_mut().resource_mut::<InspectorSectionRegistry>();
        registry.register(InspectorSection { id: "bevylite.world", title: "World & Environment", order: 0, spawn: world_card::spawn_world_card });
        registry.register(InspectorSection { id: "bevylite.camera_rendering", title: "Camera Rendering", order: 10, spawn: camera_card::spawn_camera_card });
        registry.register(InspectorSection { id: "bevylite.mesh_rendering", title: "Mesh Rendering", order: 20, spawn: object_card::spawn_object_card });
        registry.register(InspectorSection { id: "bevylite.components", title: "Components", order: 100, spawn: reflect_card::spawn_reflect_card });
        app.add_systems(
            Update,
            (
                world_card::update_world_card,
                camera_card::update_camera_card,
                reflect_card::update_reflect_card,
                camera_card::sync_viewport_preview,
                object_card::update_object_card,
                sync_global_wireframe,
                camera_card::strip_blocked_runtime_features,
                camera_card::sync_overlay_cameras,
                dock::manage_debug_panels,
            )
                .run_if(in_state(crate::ui::EditorAppState::InEditor)),
        );
    }
}
/// Mirrors the viewport "Wireframe" chip onto Bevy's global wireframe switch.
fn sync_global_wireframe(
    settings: Res<crate::core::selection::ViewportGizmoSettings>,
    mut config: ResMut<bevy::pbr::wireframe::WireframeConfig>,
) {
    if settings.is_changed() && config.global != settings.show_wireframe {
        config.global = settings.show_wireframe;
    }
}
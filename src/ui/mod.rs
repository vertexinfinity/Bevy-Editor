//! UI module for Bevy-Editor.

pub mod asset_browser;
pub mod inspector;
pub mod menu;
pub mod play_bar;
pub mod status_bar;
pub mod tools_bar;

use bevy::{
    feathers::{dark_theme::create_dark_theme, theme::UiTheme},
    inspector::InspectorPlugin,
    prelude::*,
};

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<InspectorPlugin>() {
            app.add_plugins(InspectorPlugin);
        }

        app.insert_resource(UiTheme(create_dark_theme()))
            .init_resource::<inspector::InspectorState>()
            .init_resource::<asset_browser::AssetBrowserState>()
            .init_resource::<asset_browser::HoveredAssetDirectory>()
            // Register messages
            .add_message::<menu::MenuActionMessage>()
            .add_message::<asset_browser::SpawnAssetMessage>()
            .add_message::<asset_browser::ToggleExplorerDockMessage>()
            .add_message::<asset_browser::RefreshAssetBrowserMessage>()
            .add_message::<asset_browser::NavigateAssetFolderMessage>()
            .add_message::<inspector::ToggleDebugModeMessage>()
            .add_message::<inspector::ToggleDockMessage>()
            .add_message::<inspector::ToggleCoordModeMessage>()
            .add_message::<inspector::TransformNudgeAction>()
            .add_message::<inspector::AddComponentKind>()
            // Global 3D scene picking observer
            .add_observer(inspector::select_entity_on_viewport_click)
            // Startup UI systems
            .add_systems(
                Startup,
                (
                    menu::spawn_menu_bar,
                    play_bar::spawn_play_bar,
                    tools_bar::spawn_tools_bar,
                    status_bar::spawn_status_bar,
                    asset_browser::spawn_asset_browser,
                    inspector::spawn_inspector_panel,
                ),
            )
            // Update UI systems
            .add_systems(
                Update,
                (
                    menu::handle_menu_actions,
                    tools_bar::update_active_tool_highlight,
                    status_bar::update_status_bar,
                    asset_browser::handle_spawn_asset_messages,
                    asset_browser::handle_explorer_dock_and_refresh,
                    inspector::handle_inspector_toggle_messages,
                    inspector::handle_transform_nudges,
                    inspector::handle_add_component_actions,
                    inspector::update_clean_transform_labels,
                ),
            );
    }
}

//! UI module for Bevy-Editor.

pub mod asset_browser;
pub mod hierarchy;
pub mod hub;
pub mod inspector;
pub mod panel;
pub mod menu;
pub mod play_bar;
pub mod plugin_manager;
pub mod status_bar;
pub mod theme;
pub mod tools_bar;

pub use hub::EditorAppState;

use bevy::{
    feathers::{dark_theme::create_dark_theme, theme::UiTheme},
    inspector::InspectorPlugin,
    prelude::*,
};

fn setup_editor_fonts(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut default_font_source: ResMut<bevy::text::DefaultFontSource>,
) {
    let lucide_handle: Handle<Font> = asset_server.load("fonts/lucide.ttf");

    let mut font_sources = vec![
        bevy::text::FontSource::Default,
        bevy::text::FontSource::Handle(lucide_handle.clone()),
    ];

    let has_symbol = std::path::Path::new("assets/fonts/seguisym.ttf").exists();
    let symbol_handle: Handle<Font> = if has_symbol {
        let handle = asset_server.load("fonts/seguisym.ttf");
        font_sources.push(bevy::text::FontSource::Handle(handle.clone()));
        handle
    } else {
        lucide_handle.clone()
    };

    let has_emoji = std::path::Path::new("assets/fonts/seguiemj.ttf").exists();
    let emoji_handle: Handle<Font> = if has_emoji {
        let handle = asset_server.load("fonts/seguiemj.ttf");
        font_sources.push(bevy::text::FontSource::Handle(handle.clone()));
        handle
    } else {
        lucide_handle.clone()
    };

    default_font_source.0 = bevy::text::FontSource::List(font_sources);

    commands.insert_resource(theme::EditorFonts {
        symbol: symbol_handle,
        emoji: emoji_handle,
        lucide: lucide_handle,
    });
}

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<InspectorPlugin>() {
            app.add_plugins(InspectorPlugin);
        }

        app.insert_resource(UiTheme(create_dark_theme()))
            .init_state::<EditorAppState>()
            .init_resource::<hub::ActiveProject>()
            .init_resource::<menu::FileMenuState>()
            .init_resource::<menu::BuildMenuState>()
            .init_resource::<menu::ActiveWorkspace>()
            .init_resource::<hierarchy::HierarchyState>()
            .init_resource::<inspector::InspectorState>()
            .init_resource::<asset_browser::AssetBrowserState>()
            .init_resource::<asset_browser::HoveredAssetDirectory>()
            .init_resource::<asset_browser::LastAssetClick>()
            .init_resource::<menu::McpBridgeState>()
            .init_resource::<crate::plugins::PluginManagerState>()
            // Register messages
            .add_message::<menu::MenuActionMessage>()
            .add_message::<menu::SwitchWorkspaceMessage>()
            .add_message::<plugin_manager::TogglePluginManagerMessage>()
            .add_message::<plugin_manager::ToggleCrateMessage>()
            .add_message::<plugin_manager::FilterPluginCategoryMessage>()
            .add_message::<plugin_manager::AddCustomCrateMessage>()
            .add_message::<hierarchy::ToggleHierarchyDockMessage>()
            .add_message::<hierarchy::SpawnPrimitiveMessage>()
            .add_message::<asset_browser::SpawnAssetMessage>()
            .add_message::<asset_browser::ToggleExplorerDockMessage>()
            .add_message::<asset_browser::RefreshAssetBrowserMessage>()
            .add_message::<asset_browser::NavigateAssetFolderMessage>()
            .add_message::<asset_browser::SetAssetFilterMessage>()
            .add_message::<asset_browser::CreateAssetAction>()
            .add_message::<inspector::ToggleDebugModeMessage>()
            .add_message::<inspector::ToggleDockMessage>()
            .add_message::<inspector::ToggleCoordModeMessage>()
            .add_message::<inspector::TransformNudgeAction>()
            .add_message::<inspector::AddComponentKind>()
            .add_message::<inspector::ScriptPropAction>()
            .add_message::<panel::TogglePanelMessage>()
            .add_message::<panel::TogglePanelCollapseMessage>()
            .add_message::<panel::SetPanelDockMessage>()
            // Global 3D scene picking observer
            .add_observer(inspector::select_entity_on_viewport_click)
            // PreStartup / Startup UI systems
            .add_systems(PreStartup, setup_editor_fonts)
            .add_systems(OnEnter(EditorAppState::ProjectHub), hub::spawn_project_hub)
            .add_systems(OnExit(EditorAppState::ProjectHub), hub::despawn_project_hub)
            .add_systems(
                Update,
                hub::update_project_path_labels.run_if(in_state(EditorAppState::ProjectHub)),
            )
            .add_systems(
                OnEnter(EditorAppState::InEditor),
                (
                    menu::spawn_menu_bar,
                    play_bar::spawn_play_bar,
                    tools_bar::spawn_tools_bar,
                    hierarchy::spawn_hierarchy_panel,
                    status_bar::spawn_status_bar,
                    asset_browser::spawn_asset_browser,
                    inspector::spawn_inspector_panel,
                    plugin_manager::spawn_plugin_manager_modal,
                ),
            )

            // Update UI systems (only active while InEditor)
            .add_systems(
                Update,
                (
                    menu::handle_menu_actions,
                    menu::update_dropdown_menus_visibility,
                    menu::handle_switch_workspace,
                    menu::update_window_and_menu_title,
                    menu::update_mcp_bridge_status,
                    tools_bar::update_active_tool_highlight,
                    hierarchy::update_hierarchy_list,
                    hierarchy::handle_spawn_primitives,
                    hierarchy::handle_hierarchy_collapse,
                    hierarchy::handle_hierarchy_search_input,
                    status_bar::update_status_bar,
                    plugin_manager::handle_toggle_plugin_manager,
                    plugin_manager::handle_filter_category_messages,
                    plugin_manager::update_plugin_scroll_indicator,
                    plugin_manager::handle_toggle_crate_action,
                )
                    .run_if(in_state(EditorAppState::InEditor)),
            )
            .add_systems(
                Update,
                (
                    asset_browser::handle_spawn_asset_messages,
                    asset_browser::handle_explorer_dock_and_refresh,
                    asset_browser::handle_create_asset_actions,
                    inspector::handle_inspector_toggle_messages,
                    inspector::handle_transform_nudges,
                    inspector::handle_add_component_actions,
                    inspector::handle_script_prop_actions,
                    inspector::update_clean_transform_labels,
                    panel::systems::handle_panel_collapse_messages,
                    panel::systems::handle_panel_visibility_messages,
                )
                    .run_if(in_state(EditorAppState::InEditor)),
            );
    }
}

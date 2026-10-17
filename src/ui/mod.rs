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

    let mut font_sources = Vec::new();

    // 1. Primary: Instant embedded default font (FiraMono - 0ms memory load for all UI text)
    font_sources.push(bevy::text::FontSource::Default);

    // 2. Lucide icon font glyphs
    font_sources.push(bevy::text::FontSource::Handle(lucide_handle.clone()));

    // 3. Local optional font file overrides if present in assets/
    if std::path::Path::new("assets/fonts/seguisym.ttf").exists() {
        let handle = asset_server.load("fonts/seguisym.ttf");
        font_sources.push(bevy::text::FontSource::Handle(handle));
    }
    if std::path::Path::new("assets/fonts/seguiemj.ttf").exists() {
        let handle = asset_server.load("fonts/seguiemj.ttf");
        font_sources.push(bevy::text::FontSource::Handle(handle));
    }

    // 4. System font fallbacks (queried strictly for characters missing from Default + Lucide)
    #[cfg(windows)]
    {
        font_sources.push(bevy::text::FontSource::Family("Segoe UI".into()));
        font_sources.push(bevy::text::FontSource::Family("Segoe UI Symbol".into()));
    }
    #[cfg(target_os = "linux")]
    {
        font_sources.push(bevy::text::FontSource::Family("Ubuntu".into()));
        font_sources.push(bevy::text::FontSource::Family("DejaVu Sans".into()));
    }
    #[cfg(target_os = "macos")]
    {
        font_sources.push(bevy::text::FontSource::Family("SF Pro".into()));
    }

    default_font_source.0 = bevy::text::FontSource::List(font_sources);

    commands.insert_resource(theme::EditorFonts {
        symbol: Handle::default(),
        emoji: Handle::default(),
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
            .init_resource::<hub::ProjectRegistry>()
            .init_resource::<menu::FileMenuState>()
            .init_resource::<menu::BuildMenuState>()
            .init_resource::<menu::LightmapSettingsMenuState>()
            .init_resource::<menu::ActiveWorkspace>()
            .init_resource::<hierarchy::HierarchyState>()
            .init_resource::<inspector::InspectorState>()
            .init_resource::<inspector::TransformTextEditState>()
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
            .add_message::<asset_browser::SetDefaultSceneMessage>()
            .add_message::<inspector::ToggleDebugModeMessage>()
            .add_message::<inspector::ToggleDockMessage>()
            .add_message::<inspector::ToggleCoordModeMessage>()
            .add_message::<inspector::TransformNudgeAction>()
            .add_message::<inspector::AddComponentKind>()
            .add_message::<inspector::ScriptPropAction>()
            .add_message::<inspector::TransformSetAxisAction>()
            .add_message::<inspector::MaterialNudgeAction>()
            .add_message::<inspector::LightModeAction>()
            .add_message::<inspector::ToggleScriptEnabledAction>()
            .add_message::<inspector::SelectScriptEntityAction>()
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
                    menu::sync_lightmap_backend_label,
                    tools_bar::update_active_tool_highlight,
                    tools_bar::update_viewport_chips,
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
                    asset_browser::handle_set_default_scene,
                    inspector::handle_inspector_toggle_messages,
                    inspector::handle_transform_nudges,
                    inspector::handle_add_component_actions,
                    inspector::handle_script_prop_actions,
                    inspector::handle_material_nudges,
                    inspector::update_light_card,
                    inspector::handle_light_mode_actions,
                    inspector::handle_transform_text_input,
                    inspector::update_active_scripts_panel,
                    inspector::handle_toggle_script_enabled,
                    inspector::update_clean_transform_labels,
                    panel::systems::handle_panel_collapse_messages,
                    panel::systems::handle_panel_visibility_messages,
                )
                    .run_if(in_state(EditorAppState::InEditor)),
            );
    }
}

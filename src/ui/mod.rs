//! UI module for Bevy-Editor.

pub mod asset_browser;
pub mod inspector;
pub mod menu;
pub mod status_bar;

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
            .add_systems(
                Startup,
                (
                    menu::spawn_menu_bar,
                    status_bar::spawn_status_bar,
                    asset_browser::spawn_asset_browser,
                    inspector::spawn_inspector_panel.spawn(),
                ),
            )
            .add_systems(
                Update,
                (
                    menu::handle_menu_actions,
                    status_bar::update_status_bar,
                    asset_browser::handle_asset_item_interactions,
                ),
            );
    }
}

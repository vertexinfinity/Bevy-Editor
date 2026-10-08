//! Plugin & Crate Manager Modal UI for Bevylite.
//!
//! Provides an interactive dialog inside the editor:
//! - Catalog of official and community Bevy plugins with category badges and feature descriptions.
//! - Checkboxes showing which crates/features are currently enabled in the active project's Cargo.toml.
//! - 1-click toggle to add or remove crates from Cargo.toml.
//! - Search bar and category filters.
//! - Custom crate input to add any package from crates.io.

use bevy::{
    ecs::message::{Message, MessageReader, MessageWriter},
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::plugins::{PluginCategory, PluginEntry, PluginManagerState, PLUGIN_CATALOG};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

#[derive(Component)]
pub struct PluginManagerModalRoot;

#[derive(Component)]
pub struct PluginListContainer;

#[derive(Component)]
pub struct PluginSearchText;

#[derive(Component)]
pub struct PluginStatusLabel;

#[derive(Component)]
pub struct CrateToggleCheckbox {
    pub crate_name: String,
    pub version: String,
}

#[derive(Message, Clone, Debug)]
pub struct TogglePluginManagerMessage;

#[derive(Message, Clone, Debug)]
pub struct ToggleCrateMessage {
    pub crate_name: String,
    pub version: String,
}

#[derive(Message, Clone, Debug)]
pub struct FilterPluginCategoryMessage {
    pub category: Option<PluginCategory>,
}

#[derive(Message, Clone, Debug)]
pub struct AddCustomCrateMessage {
    pub crate_name: String,
}

pub fn spawn_plugin_manager_modal(mut commands: Commands) {
    let root = commands
        .spawn((
            EditorInternal,
            PluginManagerModalRoot,
            GlobalZIndex(200),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.025, 0.04, 0.75)),
        ))
        .id();

    // Dialog Shell
    let dialog = commands
        .spawn((
            EditorInternal,
            Node {
                width: px(780),
                max_width: percent(92.0),
                height: px(580),
                max_height: percent(90.0),
                padding: UiRect::all(px(16)),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                border_radius: radius_panel(),
                row_gap: px(12),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_CARD),
            ChildOf(root),
        ))
        .id();

    // Header: Title + Close Button
    let header = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::bottom(px(8)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(dialog),
        ))
        .id();

    let title_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            },
            ChildOf(header),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("🧩"),
        TextFont { font_size: bevy::text::FontSize::Px(16.0), ..default() },
        ChildOf(title_group),
    ));

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Plugin & Crate Manager"),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(title_group),
    ));

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("— Manage game build features, plugins & crates.io dependencies"),
        TextFont { font_size: bevy::text::FontSize::Px(11.0), ..default() },
        TextColor(TEXT_MUTED),
        ChildOf(title_group),
    ));

    // Close button (✕)
    let close_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(4)),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
            ChildOf(header),
        ))
        .observe(|click: On<PointerClick>, mut writer: MessageWriter<TogglePluginManagerMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(TogglePluginManagerMessage);
            }
        })
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_CARD);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("✕"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(close_btn),
    ));

    // Categories filter bar
    let categories_bar = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(dialog),
        ))
        .id();

    spawn_category_filter_tab(&mut commands, categories_bar, "All Plugins", None);
    spawn_category_filter_tab(&mut commands, categories_bar, "Engine & Core", Some(PluginCategory::Core));
    spawn_category_filter_tab(&mut commands, categories_bar, "Physics", Some(PluginCategory::Physics));
    spawn_category_filter_tab(&mut commands, categories_bar, "Graphics & VFX", Some(PluginCategory::Rendering));
    spawn_category_filter_tab(&mut commands, categories_bar, "Audio", Some(PluginCategory::Audio));
    spawn_category_filter_tab(&mut commands, categories_bar, "UI", Some(PluginCategory::UI));
    spawn_category_filter_tab(&mut commands, categories_bar, "Scripting", Some(PluginCategory::Scripting));

    // Scrollable crate list container
    let list_scroll = commands
        .spawn((
            EditorInternal,
            PluginListContainer,
            Node {
                width: percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::clip_y(),
                padding: UiRect::axes(px(2), px(4)),
                ..default()
            },
            ChildOf(dialog),
        ))
        .id();

    // Populate catalog items
    for plugin in PLUGIN_CATALOG {
        spawn_plugin_item(&mut commands, list_scroll, plugin);
    }

    // Footer: Status Message + Web Build Indicator + Add Custom Crate
    let footer = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::top(px(8)),
                border: UiRect::top(px(1)),
                ..default()
            },
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(dialog),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        PluginStatusLabel,
        Pickable::IGNORE,
        Text::new("Select crates to include in your project builds (Windows, Web, Android)."),
        TextFont { font_size: bevy::text::FontSize::Px(11.0), ..default() },
        TextColor(TEXT_SECONDARY),
        ChildOf(footer),
    ));
}

fn spawn_category_filter_tab(
    commands: &mut Commands,
    parent: Entity,
    label: &'static str,
    cat: Option<PluginCategory>,
) {
    let btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border_radius: radius_button(),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<FilterPluginCategoryMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(FilterPluginCategoryMessage { category: cat });
            }
        })
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_CARD);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont { font_size: bevy::text::FontSize::Px(10.5), ..default() },
        TextColor(TEXT_PRIMARY),
        ChildOf(btn),
    ));
}

fn spawn_plugin_item(commands: &mut Commands, parent: Entity, plugin: &'static PluginEntry) {
    let item_card = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100.0),
                padding: UiRect::axes(px(10), px(8)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .id();

    // Left info group
    let left_info = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                max_width: percent(75.0),
                ..default()
            },
            ChildOf(item_card),
        ))
        .id();

    let name_row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            ChildOf(left_info),
        ))
        .id();

    // Crate name
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(plugin.name),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(name_row),
    ));

    // Version pill
    let ver_pill = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(4), px(1)),
                border_radius: radius_pill(),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(name_row),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(format!("v{}", plugin.version)),
        TextFont { font_size: bevy::text::FontSize::Px(9.5), ..default() },
        TextColor(TEXT_MUTED),
        ChildOf(ver_pill),
    ));

    // Category tag
    let cat_pill = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(5), px(1)),
                border_radius: radius_pill(),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(name_row),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(plugin.category.label()),
        TextFont { font_size: bevy::text::FontSize::Px(9.5), ..default() },
        TextColor(TEXT_SECONDARY),
        ChildOf(cat_pill),
    ));

    if plugin.supports_web {
        let web_pill = commands
            .spawn((
                EditorInternal,
                Node {
                    padding: UiRect::axes(px(4), px(1)),
                    border_radius: radius_pill(),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BackgroundColor(BG_INPUT),
                BorderColor::all(BORDER_SUBTLE),
                ChildOf(name_row),
            ))
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new("🌐 Web OK"),
            TextFont { font_size: bevy::text::FontSize::Px(9.0), ..default() },
            TextColor(ACCENT_EMERALD),
            ChildOf(web_pill),
        ));
    }

    // Description text
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(plugin.description),
        TextFont { font_size: bevy::text::FontSize::Px(11.0), ..default() },
        TextColor(TEXT_SECONDARY),
        ChildOf(left_info),
    ));

    // Right Action: Toggle Checkbox Button
    let toggle_btn = commands
        .spawn((
            EditorInternal,
            CrateToggleCheckbox {
                crate_name: plugin.name.to_string(),
                version: plugin.version.to_string(),
            },
            Node {
                padding: UiRect::axes(px(10), px(4)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(item_card),
        ))
        .observe(|click: On<PointerClick>, query: Query<&CrateToggleCheckbox>, mut writer: MessageWriter<ToggleCrateMessage>| {
            if click.button == PointerButton::Primary {
                if let Ok(info) = query.get(click.entity) {
                    writer.write(ToggleCrateMessage {
                        crate_name: info.crate_name.clone(),
                        version: info.version.clone(),
                    });
                }
            }
        })
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_INPUT);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Enable ☑"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(toggle_btn),
    ));
}

pub fn handle_toggle_plugin_manager(
    mut messages: MessageReader<TogglePluginManagerMessage>,
    mut state: ResMut<PluginManagerState>,
    project: Res<crate::ui::hub::ActiveProject>,
    mut modal_query: Query<&mut Node, With<PluginManagerModalRoot>>,
) {
    for _ in messages.read() {
        state.is_open = !state.is_open;
        if state.is_open {
            if let Some(ref dir) = project.project_dir {
                state.refresh_from_project(dir);
            }
        }
        for mut node in &mut modal_query {
            node.display = if state.is_open { Display::Flex } else { Display::None };
        }
    }
}

pub fn handle_toggle_crate_action(
    mut messages: MessageReader<ToggleCrateMessage>,
    mut state: ResMut<PluginManagerState>,
    project: Res<crate::ui::hub::ActiveProject>,
    mut status_text: Query<&mut Text, With<PluginStatusLabel>>,
    checkbox_query: Query<(Entity, &CrateToggleCheckbox, &Children)>,
    mut button_text_query: Query<&mut Text, Without<PluginStatusLabel>>,
    mut button_bg_query: Query<&mut BackgroundColor>,
) {
    for msg in messages.read() {
        let Some(ref dir) = project.project_dir else {
            if let Ok(mut st) = status_text.single_mut() {
                st.0 = "⚠️ No active project loaded. Open a project to modify its Cargo.toml.".to_string();
            }
            continue;
        };

        let is_enabled = state.enabled_crates.contains(&msg.crate_name);

        if is_enabled {
            let _ = state.remove_crate_from_project(dir, &msg.crate_name);
        } else {
            let _ = state.add_crate_to_project(dir, &msg.crate_name, &msg.version);
        }

        if let Some((ref feedback, _)) = state.status_message {
            if let Ok(mut st) = status_text.single_mut() {
                st.0 = feedback.clone();
            }
        }

        // Update button visual state
        for (entity, info, children) in &checkbox_query {
            if info.crate_name == msg.crate_name {
                let active = state.enabled_crates.contains(&msg.crate_name);
                if let Ok(mut bg) = button_bg_query.get_mut(entity) {
                    *bg = BackgroundColor(if active { ACCENT_CYAN_DIM } else { BG_INPUT });
                }
                for child in children.iter() {
                    if let Ok(mut text) = button_text_query.get_mut(child) {
                        text.0 = if active { "Enabled ✓".to_string() } else { "Enable ☑".to_string() };
                    }
                }
            }
        }
    }
}

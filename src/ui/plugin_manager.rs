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
pub struct PluginScrollTrack;

#[derive(Component)]
pub struct PluginScrollThumb;

#[derive(Component)]
pub struct CategoryFilterTab {
    pub category: Option<PluginCategory>,
}

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
                width: px(820),
                max_width: percent(92.0),
                height: px(540),
                max_height: percent(85.0),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                border_radius: radius_panel(),
                row_gap: px(10),
                overflow: Overflow::clip(),
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
                flex_wrap: FlexWrap::Wrap,
                row_gap: px(4),
                ..default()
            },
            ChildOf(dialog),
        ))
        .id();

    spawn_category_filter_tab(&mut commands, categories_bar, "All Plugins", None, true);
    spawn_category_filter_tab(&mut commands, categories_bar, "Engine & Core", Some(PluginCategory::Core), false);
    spawn_category_filter_tab(&mut commands, categories_bar, "Physics", Some(PluginCategory::Physics), false);
    spawn_category_filter_tab(&mut commands, categories_bar, "Graphics & VFX", Some(PluginCategory::Rendering), false);
    spawn_category_filter_tab(&mut commands, categories_bar, "Audio", Some(PluginCategory::Audio), false);
    spawn_category_filter_tab(&mut commands, categories_bar, "UI", Some(PluginCategory::UI), false);
    spawn_category_filter_tab(&mut commands, categories_bar, "Scripting", Some(PluginCategory::Scripting), false);

    // List wrapper row (contains scrollable items + visual scrollbar track)
    let list_wrapper = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100.0),
                flex_grow: 1.0,
                flex_shrink: 1.0,
                min_height: px(150),
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                overflow: Overflow::clip(),
                ..default()
            },
            ChildOf(dialog),
        ))
        .id();

    // Scrollable crate list container
    let list_scroll = commands
        .spawn((
            EditorInternal,
            PluginListContainer,
            ScrollPosition::default(),
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                height: percent(100.0),
                min_height: px(150),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                overflow: Overflow::scroll_y(),
                padding: UiRect::axes(px(4), px(4)),
                ..default()
            },
            ChildOf(list_wrapper),
        ))
        .observe(|scroll: On<bevy::picking::events::PointerScroll>, mut q_scroll: Query<(&mut ScrollPosition, &ComputedNode)>| {
            if let Ok((mut pos, computed)) = q_scroll.get_mut(scroll.entity) {
                let delta_y = match scroll.unit {
                    bevy::input::mouse::MouseScrollUnit::Line => scroll.y * 36.0,
                    bevy::input::mouse::MouseScrollUnit::Pixel => scroll.y,
                };
                let max_offset = (computed.content_size().y - computed.size().y).max(0.0) * computed.inverse_scale_factor();
                pos.y = (pos.y - delta_y).clamp(0.0, max_offset.max(5000.0));
            }
        })
        .id();

    // Visual scrollbar track
    let track = commands
        .spawn((
            EditorInternal,
            PluginScrollTrack,
            Node {
                width: px(8),
                height: percent(100.0),
                border_radius: radius_pill(),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(list_wrapper),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        PluginScrollThumb,
        Node {
            width: percent(100.0),
            height: px(50),
            border_radius: radius_pill(),
            margin: UiRect::top(px(0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.949, 0.996, 0.35)),
        ChildOf(track),
    ));

    // Populate catalog items initially
    for plugin in PLUGIN_CATALOG {
        spawn_plugin_item(&mut commands, list_scroll, plugin, false);
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
    is_active: bool,
) {
    let bg_color = if is_active { ACCENT_CYAN_DIM } else { BG_CARD };
    let border_color = if is_active { BORDER_ACCENT } else { BORDER_SUBTLE };
    let text_color = if is_active { ACCENT_CYAN } else { TEXT_SECONDARY };

    let btn = commands
        .spawn((
            EditorInternal,
            CategoryFilterTab { category: cat },
            Node {
                padding: UiRect::axes(px(10), px(4)),
                border_radius: radius_button(),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg_color),
            BorderColor::all(border_color),
            ChildOf(parent),
        ))
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<FilterPluginCategoryMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(FilterPluginCategoryMessage { category: cat });
            }
        })
        .observe(move |trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>, state: Res<PluginManagerState>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                if state.selected_category != cat {
                    *bg = BackgroundColor(BG_HOVER);
                }
            }
        })
        .observe(move |trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>, state: Res<PluginManagerState>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                if state.selected_category == cat {
                    *bg = BackgroundColor(ACCENT_CYAN_DIM);
                } else {
                    *bg = BackgroundColor(BG_CARD);
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            weight: if is_active { FontWeight::BOLD } else { FontWeight::MEDIUM },
            ..default()
        },
        TextColor(text_color),
        ChildOf(btn),
    ));
}

fn spawn_plugin_item(commands: &mut Commands, parent: Entity, plugin: &'static PluginEntry, is_enabled: bool) {
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
    let btn_bg = if is_enabled { ACCENT_CYAN_DIM } else { BG_INPUT };
    let btn_text = if is_enabled { "Enabled ✓" } else { "Enable ☑" };

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
            BackgroundColor(btn_bg),
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
        .observe(|trigger: On<PointerOut>, query: Query<&CrateToggleCheckbox>, state: Res<PluginManagerState>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                let active = query.get(trigger.entity).map(|c| state.enabled_crates.contains(&c.crate_name)).unwrap_or(false);
                *bg = BackgroundColor(if active { ACCENT_CYAN_DIM } else { BG_INPUT });
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(btn_text),
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
    mut commands: Commands,
    mut messages: MessageReader<TogglePluginManagerMessage>,
    mut state: ResMut<PluginManagerState>,
    project: Res<crate::ui::hub::ActiveProject>,
    mut modal_query: Query<&mut Node, With<PluginManagerModalRoot>>,
    list_query: Query<Entity, With<PluginListContainer>>,
    children_query: Query<&Children>,
) {
    for _ in messages.read() {
        state.is_open = !state.is_open;
        if state.is_open {
            if let Some(ref dir) = project.project_dir {
                state.refresh_from_project(dir);
            }
            // Refresh list items to accurately show enabled state
            if let Ok(container) = list_query.single() {
                if let Ok(children) = children_query.get(container) {
                    for child in children.iter() {
                        commands.entity(child).despawn();
                    }
                }
                for plugin in PLUGIN_CATALOG {
                    if state.selected_category.is_none() || state.selected_category == Some(plugin.category) {
                        let is_enabled = state.enabled_crates.contains(&plugin.name.to_string());
                        spawn_plugin_item(&mut commands, container, plugin, is_enabled);
                    }
                }
            }
        }
        for mut node in &mut modal_query {
            crate::ui::set_display(&mut node, if state.is_open { Display::Flex } else { Display::None });
        }
    }
}

pub fn handle_filter_category_messages(
    mut commands: Commands,
    mut messages: MessageReader<FilterPluginCategoryMessage>,
    mut state: ResMut<PluginManagerState>,
    list_query: Query<Entity, With<PluginListContainer>>,
    mut scroll_query: Query<&mut ScrollPosition, With<PluginListContainer>>,
    children_query: Query<&Children>,
    mut tab_query: Query<(Entity, &CategoryFilterTab, &Children)>,
    mut bg_query: Query<&mut BackgroundColor>,
    mut border_query: Query<&mut BorderColor>,
    mut text_query: Query<(&mut TextColor, &mut TextFont)>,
) {
    for msg in messages.read() {
        state.selected_category = msg.category;

        // Reset scroll position to top when filter changes
        if let Ok(mut scroll) = scroll_query.single_mut() {
            scroll.y = 0.0;
        }

        // Update category tab button visuals
        for (tab_ent, tab_info, children) in &mut tab_query {
            let is_active = tab_info.category == state.selected_category;
            if let Ok(mut bg) = bg_query.get_mut(tab_ent) {
                *bg = BackgroundColor(if is_active { ACCENT_CYAN_DIM } else { BG_CARD });
            }
            if let Ok(mut border) = border_query.get_mut(tab_ent) {
                *border = BorderColor::all(if is_active { BORDER_ACCENT } else { BORDER_SUBTLE });
            }
            for child in children.iter() {
                if let Ok((mut text_color, mut text_font)) = text_query.get_mut(child) {
                    text_color.0 = if is_active { ACCENT_CYAN } else { TEXT_SECONDARY };
                    text_font.weight = if is_active { FontWeight::BOLD } else { FontWeight::MEDIUM };
                }
            }
        }

        // Despawn old items and respawn filtered items
        let Ok(container) = list_query.single() else { continue };
        if let Ok(children) = children_query.get(container) {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        for plugin in PLUGIN_CATALOG {
            let matches_category = match state.selected_category {
                Some(cat) => plugin.category == cat,
                None => true,
            };

            if matches_category {
                let is_enabled = state.enabled_crates.contains(&plugin.name.to_string());
                spawn_plugin_item(&mut commands, container, plugin, is_enabled);
            }
        }
    }
}

pub fn update_plugin_scroll_indicator(
    list_query: Query<(&ScrollPosition, &ComputedNode), With<PluginListContainer>>,
    mut thumb_query: Query<&mut Node, With<PluginScrollThumb>>,
    track_query: Query<&ComputedNode, With<PluginScrollTrack>>,
) {
    let Ok((scroll, list_node)) = list_query.single() else { return };
    let Ok(track_node) = track_query.single() else { return };
    let Ok(mut thumb_node) = thumb_query.single_mut() else { return };

    let content_height = list_node.content_size().y * list_node.inverse_scale_factor();
    let visible_height = list_node.size().y * list_node.inverse_scale_factor();
    let track_height = track_node.size().y * track_node.inverse_scale_factor();

    if content_height <= visible_height || content_height <= 0.0 || track_height <= 0.0 {
        crate::ui::set_display(&mut thumb_node, Display::None);
        return;
    }

    crate::ui::set_display(&mut thumb_node, Display::Flex);
    let thumb_height = ((visible_height / content_height) * track_height).clamp(24.0, track_height);
    let max_scroll = (content_height - visible_height).max(1.0);
    let scroll_fraction = (scroll.y / max_scroll).clamp(0.0, 1.0);
    let max_thumb_top = track_height - thumb_height;
    let thumb_top = scroll_fraction * max_thumb_top;

    thumb_node.height = px(thumb_height);
    thumb_node.margin = UiRect::top(px(thumb_top));
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

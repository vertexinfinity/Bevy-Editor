//! Top Menu Bar & Application Header for Bevylite.
//!
//! Provides brand identity, project management, scene operations,
//! layout presets, and multi-platform export triggers.

use bevy::{
    ecs::{
        message::{Message, MessageWriter},
        system::RunSystemOnce,
    },
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::export::RequestExportMessage;
use crate::scene::{primitives, save_load, CurrentScene, EditorInternal};
use crate::ui::theme::*;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    NewProject,
    OpenProject,
    NewScene,
    OpenScene,
    SaveScene,
    SaveAsScene,
    SetAsDefaultScene,
    OpenPluginManager,
    ToggleBuildMenu,
    BuildAndroid,
    BuildWindows,
    BuildLinux,
    BuildWeb,
    BuildIos,
    BakeLightmaps,
    ToggleLightmapBackend,
}

#[derive(Message, Clone, Copy, Debug)]
pub struct MenuActionMessage(pub MenuAction);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WorkspacePreset {
    #[default]
    Default,
    LevelDesign,
    Programming,
    Debugging,
    Assets,
}

impl WorkspacePreset {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::LevelDesign => "Level Design",
            Self::Programming => "Programming",
            Self::Debugging => "Debugging",
            Self::Assets => "Assets",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Default => Self::LevelDesign,
            Self::LevelDesign => Self::Programming,
            Self::Programming => Self::Debugging,
            Self::Debugging => Self::Assets,
            Self::Assets => Self::Default,
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ActiveWorkspace(pub WorkspacePreset);

#[derive(Message, Clone, Copy, Debug)]
pub struct SwitchWorkspaceMessage(pub WorkspacePreset);

#[derive(Component)]
pub struct WorkspaceSelectorLabel;

#[derive(Component)]
pub struct WindowTitleLabel;

#[derive(Resource, Default)]
pub struct FileMenuState(pub bool);

#[derive(Component)]
pub struct FileMenuPanel;

#[derive(Resource, Default)]
pub struct BuildMenuState(pub bool);

#[derive(Component)]
pub struct BuildMenuPanel;

/// Spawns the top unified header and menu bar.
pub fn spawn_menu_bar(mut commands: Commands, current_ws: Res<ActiveWorkspace>) {
    let bar_root = commands
        .spawn((
            EditorInternal,
            Name::new("TopMenuBar"),
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                height: px(38),
                border: UiRect::bottom(px(1)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(12), px(4)),
                ..default()
            },
            BackgroundColor(BG_SHELL),
            BorderColor::all(BORDER_SUBTLE),
        ))
        .id();

    // 1. Left Container: Brand Badge + File Dropdown + Quick Save Icon
    let left_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    // Bevylite Brand Logo Badge
    let brand_pill = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                padding: UiRect::axes(px(8), px(3)),
                margin: UiRect::right(px(6)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(left_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("BL"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(brand_pill),
    ));

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("BEVYLITE"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            weight: FontWeight::SEMIBOLD,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(brand_pill),
    ));

    // File Menu Dropdown Trigger
    let file_dropdown_btn = commands
        .spawn((
            EditorInternal,
            Node {
                position_type: PositionType::Relative,
                padding: UiRect::axes(px(10), px(4)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(left_group),
        ))
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
        .observe(|click: On<PointerClick>, mut file_menu_state: ResMut<FileMenuState>| {
            if click.button == PointerButton::Primary {
                file_menu_state.0 = !file_menu_state.0;
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("File  ▾"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(file_dropdown_btn),
    ));

    // File Dropdown Popup Panel
    let file_menu_popup = commands
        .spawn((
            EditorInternal,
            FileMenuPanel,
            GlobalZIndex(150),
            Node {
                position_type: PositionType::Absolute,
                top: px(32),
                left: px(0),
                width: px(170),
                padding: UiRect::all(px(4)),
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                display: Display::None,
                border: UiRect::all(px(1)),
                border_radius: radius_panel(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(file_dropdown_btn),
        ))
        .id();

    // Items inside File dropdown
    spawn_dropdown_item(&mut commands, file_menu_popup, "+ New Project...", MenuAction::NewProject);
    spawn_dropdown_item(&mut commands, file_menu_popup, "📂 Open Project...", MenuAction::OpenProject);
    spawn_dropdown_separator(&mut commands, file_menu_popup);
    spawn_dropdown_item(&mut commands, file_menu_popup, "📄 New Scene", MenuAction::NewScene);
    spawn_dropdown_item(&mut commands, file_menu_popup, "📂 Open Scene...", MenuAction::OpenScene);
    spawn_dropdown_item(&mut commands, file_menu_popup, "💾 Save Scene", MenuAction::SaveScene);
    spawn_dropdown_item(&mut commands, file_menu_popup, "💾 Save Scene As...", MenuAction::SaveAsScene);
    spawn_dropdown_separator(&mut commands, file_menu_popup);
    spawn_dropdown_item(&mut commands, file_menu_popup, "⭐ Set as Default Scene", MenuAction::SetAsDefaultScene);
    spawn_dropdown_separator(&mut commands, file_menu_popup);
    spawn_dropdown_item(&mut commands, file_menu_popup, "🧩 Plugins & Crates...", MenuAction::OpenPluginManager);

    // Quick Action: Save Scene Shortcut Button
    let quick_save_btn = commands
        .spawn((
            EditorInternal,
            MenuAction::SaveScene,
            Node {
                padding: UiRect::axes(px(8), px(4)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(left_group),
        ))
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
        .observe(|click: On<PointerClick>, query: Query<&MenuAction>, mut writer: MessageWriter<MenuActionMessage>| {
            if click.button == PointerButton::Primary {
                if let Ok(action) = query.get(click.entity) {
                    writer.write(MenuActionMessage(*action));
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("💾 Save"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(quick_save_btn),
    ));

    // Active Project - Scene Name Display Pill
    let title_pill = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(px(8), px(3)),
                margin: UiRect::left(px(6)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(left_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        WindowTitleLabel,
        Pickable::IGNORE,
        Text::new("Bevylite — [No Project] - Untitled Scene"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(title_pill),
    ));

    // Center Container: Play-In-Editor (PIE) Transport Controls
    let center_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    crate::ui::play_bar::spawn_play_controls(&mut commands, center_group);

    // 2. Right Container: Platform Exports + Workspace Switcher + MCP Status
    let right_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                ..default()
            },
            ChildOf(bar_root),
        ))
        .id();

    // Lightmap Backend Toggle Button (GPU / CPU)
    let backend_toggle_btn = commands
        .spawn((
            EditorInternal,
            MenuAction::ToggleLightmapBackend,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(right_group),
        ))
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
        .observe(|click: On<PointerClick>, query: Query<&MenuAction>, mut writer: MessageWriter<MenuActionMessage>| {
            if click.button == PointerButton::Primary {
                if let Ok(action) = query.get(click.entity) {
                    writer.write(MenuActionMessage(*action));
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        LightmapBackendLabel,
        Pickable::IGNORE,
        Text::new("⚡ GPU (RTX)"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(ACCENT_AMBER),
        ChildOf(backend_toggle_btn),
    ));

    // Bake Lightmaps Button
    let bake_button = commands
        .spawn((
            EditorInternal,
            MenuAction::BakeLightmaps,
            Node {
                padding: UiRect::axes(px(10), px(3)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(ACCENT_CYAN_DIM),
            BorderColor::all(BORDER_ACCENT),
            ChildOf(right_group),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(ACCENT_CYAN_DIM);
            }
        })
        .observe(|click: On<PointerClick>, query: Query<&MenuAction>, mut writer: MessageWriter<MenuActionMessage>| {
            if click.button == PointerButton::Primary {
                if let Ok(action) = query.get(click.entity) {
                    writer.write(MenuActionMessage(*action));
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("☀️ Bake Lightmaps"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            weight: FontWeight::SEMIBOLD,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(bake_button),
    ));

    let build_button = commands
        .spawn((
            EditorInternal,
            MenuAction::ToggleBuildMenu,
            Node {
                position_type: PositionType::Relative,
                padding: UiRect::axes(px(10), px(3)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(ACCENT_CYAN_DIM),
            BorderColor::all(BORDER_ACCENT),
            ChildOf(right_group),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(ACCENT_CYAN_DIM);
            }
        })
        .observe(|click: On<PointerClick>, query: Query<&MenuAction>, mut writer: MessageWriter<MenuActionMessage>| {
            if click.button == PointerButton::Primary {
                if let Ok(action) = query.get(click.entity) {
                    writer.write(MenuActionMessage(*action));
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Build  ▾"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            weight: FontWeight::SEMIBOLD,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(build_button),
    ));

    let build_menu = commands
        .spawn((
            EditorInternal,
            BuildMenuPanel,
            GlobalZIndex(150),
            Node {
                position_type: PositionType::Absolute,
                top: px(32),
                left: px(0),
                width: px(136),
                padding: UiRect::all(px(4)),
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                display: Display::None,
                border: UiRect::all(px(1)),
                border_radius: radius_panel(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(build_button),
        ))
        .id();

    for (label, action) in [
        ("Windows", MenuAction::BuildWindows),
        ("Linux", MenuAction::BuildLinux),
        ("Android", MenuAction::BuildAndroid),
        ("Web", MenuAction::BuildWeb),
        ("iOS", MenuAction::BuildIos),
    ] {
        spawn_build_target_button(&mut commands, build_menu, label, action);
    }

    // MCP Live Status Pill
    let mcp_pill = commands
        .spawn((
            EditorInternal,
            McpStatusLabel,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_ACCENT),
            ChildOf(right_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("MCP: READY"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(mcp_pill),
    ));

    // Workspace layout selector button (Clicking cycles workspaces: Default -> Level Design -> Programming -> Debugging -> Assets)
    let layout_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(right_group),
        ))
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
        .observe(|click: On<PointerClick>, ws: Res<ActiveWorkspace>, mut writer: MessageWriter<SwitchWorkspaceMessage>| {
            if click.button == PointerButton::Primary {
                writer.write(SwitchWorkspaceMessage(ws.0.next()));
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        WorkspaceSelectorLabel,
        Pickable::IGNORE,
        Text::new(format!("Workspace: {}", current_ws.0.label())),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.0),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(layout_btn),
    ));

    // Engine Version Badge & Compatibility Indicator
    let engine_pill = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(right_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Bevy: 0.20-dev"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.5),
            weight: FontWeight::MEDIUM,
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(engine_pill),
    ));
}



#[derive(Component)]
pub struct McpStatusLabel;

#[derive(Component)]
pub struct LightmapBackendLabel;

/// Synchronizes the menu backend label with the active LightmapConfig.
pub fn sync_lightmap_backend_label(
    config: Option<Res<bevylite_lightmapper::LightmapConfig>>,
    mut query: Query<&mut Text, With<LightmapBackendLabel>>,
) {
    if let Some(config) = config {
        if config.is_changed() {
            for mut text in query.iter_mut() {
                text.0 = match config.backend {
                    bevylite_lightmapper::LightmapBackend::Gpu => "⚡ GPU (RTX)".to_string(),
                    bevylite_lightmapper::LightmapBackend::Cpu => "🖥️ CPU".to_string(),
                };
            }
        }
    }
}

fn spawn_build_target_button(
    commands: &mut Commands,
    parent: Entity,
    label: &'static str,
    action: MenuAction,
) {
    let button = commands
        .spawn((
            EditorInternal,
            action,
            Node {
                width: percent(100.0),
                padding: UiRect::axes(px(8), px(6)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(parent),
        ))
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
        .observe(|click: On<PointerClick>, query: Query<&MenuAction>, mut writer: MessageWriter<MenuActionMessage>, mut build_state: ResMut<BuildMenuState>| {
            if click.button == PointerButton::Primary {
                build_state.0 = false;
                if let Ok(action) = query.get(click.entity) {
                    writer.write(MenuActionMessage(*action));
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(button),
    ));
}

fn spawn_dropdown_item(
    commands: &mut Commands,
    parent: Entity,
    label: &'static str,
    action: MenuAction,
) {
    let button = commands
        .spawn((
            EditorInternal,
            action,
            Node {
                width: percent(100.0),
                padding: UiRect::axes(px(8), px(5)),
                border_radius: radius_button(),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
            ChildOf(parent),
        ))
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
        .observe(|click: On<PointerClick>, query: Query<&MenuAction>, mut writer: MessageWriter<MenuActionMessage>, mut state: ResMut<FileMenuState>| {
            if click.button == PointerButton::Primary {
                state.0 = false;
                if let Ok(action) = query.get(click.entity) {
                    writer.write(MenuActionMessage(*action));
                }
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(button),
    ));
}

fn spawn_dropdown_separator(commands: &mut Commands, parent: Entity) {
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Node {
            width: percent(100.0),
            height: px(1),
            margin: UiRect::axes(px(0), px(2)),
            ..default()
        },
        BackgroundColor(BORDER_SUBTLE),
        ChildOf(parent),
    ));
}

pub fn update_dropdown_menus_visibility(
    file_state: Res<FileMenuState>,
    build_state: Res<BuildMenuState>,
    mut file_panels: Query<&mut Node, (With<FileMenuPanel>, Without<BuildMenuPanel>)>,
    mut build_panels: Query<&mut Node, (With<BuildMenuPanel>, Without<FileMenuPanel>)>,
) {
    for mut node in &mut file_panels {
        node.display = if file_state.0 { Display::Flex } else { Display::None };
    }
    for mut node in &mut build_panels {
        node.display = if build_state.0 { Display::Flex } else { Display::None };
    }
}

pub fn update_build_menu_visibility(
    state: Res<BuildMenuState>,
    mut panels: Query<&mut Node, With<BuildMenuPanel>>,
) {
    for mut node in &mut panels {
        node.display = if state.0 { Display::Flex } else { Display::None };
    }
}


/// Handles executing menu actions received via MenuActionMessage.
pub fn handle_menu_actions(world: &mut World) {
    let actions: Vec<MenuAction> = {
        let Some(mut messages) = world.get_resource_mut::<Messages<MenuActionMessage>>() else {
            return;
        };
        messages.drain().map(|m| m.0).collect()
    };

    for action in actions {
        match action {
            MenuAction::NewProject => {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    if let Ok(project_dir) = crate::project::ProjectScaffolder::create_project(&folder, "MyBevyGame") {
                        world.resource_mut::<crate::ui::hub::ActiveProject>().set_project_dir(project_dir.clone());
                        if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                            *status = crate::export::ExportStatus::Success {
                                target: "Project Scaffolder".to_string(),
                                message: "Created clean, zero-error standalone Bevy project".to_string(),
                                path: project_dir,
                            };
                        }
                    }
                }
            }
            MenuAction::OpenProject => {
                if let Some(folder) = rfd::FileDialog::new()
                    .set_title("Open Bevy Game Project (Select Folder with Cargo.toml)")
                    .pick_folder()
                {
                    if folder.join("Cargo.toml").is_file() {
                        let is_editor_repo = folder.file_name().and_then(|n| n.to_str()).map(|n| n.eq_ignore_ascii_case("Bevy-Editor")).unwrap_or(false);
                        if is_editor_repo {
                            warn!("Selected folder is the Bevy-Editor application repo itself, not a user game project.");
                            if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                                *status = crate::export::ExportStatus::Failed {
                                    target: "Open Project".to_string(),
                                    error: "Selected folder is Bevy-Editor itself. Please select your game project folder.".to_string(),
                                };
                            }
                        } else {
                            info!("Opened active user project: {}", folder.display());
                            world.resource_mut::<crate::ui::hub::ActiveProject>().set_project_dir(folder.clone());
                            if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                                *status = crate::export::ExportStatus::Success {
                                    target: "Project Loaded".to_string(),
                                    message: format!("Loaded user project from {}", folder.display()),
                                    path: folder,
                                };
                            }
                        }
                    } else {
                        warn!("Folder does not contain Cargo.toml: {}", folder.display());
                    }
                }
            }
            MenuAction::NewScene => {
                info!("Creating new scene: resetting user scene entities...");
                world.despawn_all::<With<crate::scene::SceneEntity>>();

                if let Some(mut current_scene) = world.get_resource_mut::<CurrentScene>() {
                    current_scene.file_path = None;
                    current_scene.is_modified = false;
                }

                // Re-spawn starter scene
                let _ = world.run_system_once(primitives::spawn_starter_scene);
                info!("New scene created successfully.");
            }
            MenuAction::OpenScene => {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Bevy Scene (*.ron)", &["ron", "bsn"])
                    .pick_file()
                {
                    info!("Selected scene to open: {:?}", file);
                    world.despawn_all::<With<crate::scene::SceneEntity>>();

                    let path_str = file.to_string_lossy().to_string();
                    let asset_server = world.resource::<AssetServer>().clone();
                    world.spawn((
                        crate::scene::SceneEntity,
                        Name::new(format!("Scene: {}", file.file_name().and_then(|n| n.to_str()).unwrap_or("loaded"))),
                        bevy::world_serialization::DynamicWorldRoot(asset_server.load(path_str)),
                        Transform::IDENTITY,
                    ));

                    if let Some(mut current_scene) = world.get_resource_mut::<CurrentScene>() {
                        current_scene.file_path = Some(file);
                        current_scene.is_modified = false;
                    }
                }
            }
            MenuAction::SaveScene => {
                let existing_path = world.resource::<CurrentScene>().file_path.clone();
                if let Some(path) = existing_path {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Ok(()) = save_load::save_world_to_scene(world, &path) {
                        info!("Saved scene to {}", path.display());
                        if let Some(mut current_scene) = world.get_resource_mut::<CurrentScene>() {
                            current_scene.is_modified = false;
                        }
                        if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                            *status = crate::export::ExportStatus::Success {
                                target: "Scene Saved".to_string(),
                                message: format!("Saved scene to {}", path.file_name().and_then(|n| n.to_str()).unwrap_or("scene.bsn")),
                                path,
                            };
                        }
                    }
                } else {
                    save_as_dialog(world);
                }
            }
            MenuAction::SaveAsScene => {
                save_as_dialog(world);
            }
            MenuAction::SetAsDefaultScene => {
                let project_dir = world.resource::<crate::ui::hub::ActiveProject>().project_dir.clone();
                let Some(project_dir) = project_dir else {
                    warn!("Cannot set default scene: No active game project loaded.");
                    if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                        *status = crate::export::ExportStatus::Failed {
                            target: "Default Scene".to_string(),
                            error: "Open or create a game project first before setting default scene.".to_string(),
                        };
                    }
                    continue;
                };

                let scenes_dir = project_dir.join("assets").join("scenes");
                let _ = std::fs::create_dir_all(&scenes_dir);
                let default_scene_path = scenes_dir.join("default_scene.bsn");

                // Save world to default_scene.bsn
                if let Err(e) = save_load::save_world_to_scene(world, &default_scene_path) {
                    warn!("Failed to save default scene: {e}");
                    if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                        *status = crate::export::ExportStatus::Failed {
                            target: "Default Scene".to_string(),
                            error: format!("Failed to save default scene: {e}"),
                        };
                    }
                    continue;
                }

                // Update CurrentScene tracking
                if let Some(mut current_scene) = world.get_resource_mut::<CurrentScene>() {
                    current_scene.file_path = Some(default_scene_path.clone());
                    current_scene.is_modified = false;
                }

                // Sync any editor model assets to project assets so referenced models are never missing
                let editor_models = std::path::Path::new("assets/models");
                let project_models = project_dir.join("assets").join("models");
                if editor_models.is_dir() {
                    let _ = std::fs::create_dir_all(&project_models);
                    if let Ok(entries) = std::fs::read_dir(editor_models) {
                        for entry in entries.flatten() {
                            let dest = project_models.join(entry.file_name());
                            if !dest.exists() {
                                let _ = std::fs::copy(entry.path(), dest);
                            }
                        }
                    }
                }

                // Wire project main.rs to load default_scene.bsn on startup
                let main_src_path = project_dir.join("src").join("main.rs");
                if main_src_path.is_file() {
                    if let Ok(main_src) = std::fs::read_to_string(&main_src_path) {
                        if !main_src.contains("default_scene.bsn") {
                            let updated_main = format!(
                                r#"//! Standalone Bevy Game created by Bevy Editor.

use bevy::prelude::*;
use bevy::world_serialization::DynamicWorldRoot;

fn main() {{
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {{
            primary_window: Some(Window {{
                title: "MyBevyGame".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }}),
            ..default()
        }}))
        .add_systems(Startup, setup_scene)
        .run();
}}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {{
    // Default Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // Directional Sunlight Fallback
    commands.spawn((
        DirectionalLight {{
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        }},
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Load default scene configured in Bevy Editor
    commands.spawn(DynamicWorldRoot(asset_server.load("scenes/default_scene.bsn")));
}}
"#
                            );
                            let _ = std::fs::write(&main_src_path, updated_main);
                        } else if main_src.contains("DynamicSceneRoot") || main_src.contains("(1280.0_f32, 720.0_f32)") {
                            let patched = main_src
                                .replace("DynamicSceneRoot", "bevy::world_serialization::DynamicWorldRoot")
                                .replace("(1280.0_f32, 720.0_f32)", "(1280, 720)");
                            let _ = std::fs::write(&main_src_path, patched);
                        }
                    }
                }

                info!("Set active scene as default scene in project: {}", default_scene_path.display());
                if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                    *status = crate::export::ExportStatus::Success {
                        target: "Default Scene".to_string(),
                        message: "Active scene set as build default (assets/scenes/default_scene.bsn)".to_string(),
                        path: default_scene_path,
                    };
                }
            }
            MenuAction::OpenPluginManager => {
                world.write_message(crate::ui::plugin_manager::TogglePluginManagerMessage);
            }
            MenuAction::ToggleBuildMenu => {
                if let Some(mut state) = world.get_resource_mut::<BuildMenuState>() {
                    state.0 = !state.0;
                }
            }
            MenuAction::BuildAndroid => {
                trigger_export(world, "android");
            }
            MenuAction::BuildWindows => {
                trigger_export(world, "windows");
            }
            MenuAction::BuildLinux => {
                trigger_export(world, "linux");
            }
            MenuAction::BuildWeb => {
                trigger_export(world, "web");
            }
            MenuAction::BuildIos => {
                trigger_export(world, "ios");
            }
            MenuAction::BakeLightmaps => {
                info!("Triggering lightmap bake request from Menu Bar");
                world.write_message(bevylite_lightmapper::BakeLightmapRequest::default());
            }
            MenuAction::ToggleLightmapBackend => {
                if let Some(mut config) = world.get_resource_mut::<bevylite_lightmapper::LightmapConfig>() {
                    config.backend = match config.backend {
                        bevylite_lightmapper::LightmapBackend::Gpu => bevylite_lightmapper::LightmapBackend::Cpu,
                        bevylite_lightmapper::LightmapBackend::Cpu => bevylite_lightmapper::LightmapBackend::Gpu,
                    };
                    info!("Switched lightmap bake backend to {}", config.backend.label());
                }
            }
        }
    }
}

fn save_as_dialog(world: &mut World) {
    if let Some(file) = rfd::FileDialog::new()
        .add_filter("Bevy Scene (*.bsn, *.ron)", &["bsn", "ron"])
        .set_file_name("scene.bsn")
        .save_file()
    {
        if let Some(parent) = file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(()) = save_load::save_world_to_scene(world, &file) {
            info!("Saved scene to {}", file.display());
            if let Some(mut current_scene) = world.get_resource_mut::<CurrentScene>() {
                current_scene.file_path = Some(file.clone());
                current_scene.is_modified = false;
            }
            if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                *status = crate::export::ExportStatus::Success {
                    target: "Scene Saved".to_string(),
                    message: format!("Saved scene to {}", file.file_name().and_then(|n| n.to_str()).unwrap_or("scene.bsn")),
                    path: file,
                };
            }
        }
    }
}

fn is_editor_repository(dir: &std::path::Path) -> bool {
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.eq_ignore_ascii_case("Bevy-Editor") {
        return true;
    }
    if let Ok(manifest) = std::fs::read_to_string(dir.join("Cargo.toml")) {
        if manifest.contains("name = \"bevy_editor\"") {
            return true;
        }
    }
    false
}

fn trigger_export(world: &mut World, target_id: &str) {
    if let Some(mut state) = world.get_resource_mut::<BuildMenuState>() {
        state.0 = false;
    }

    let mut project = world.resource::<crate::ui::hub::ActiveProject>().clone();

    // If no project selected or project points to Bevy-Editor itself, prompt user to pick their game project folder
    let needs_selection = match &project.project_dir {
        None => true,
        Some(dir) => is_editor_repository(dir),
    };

    if needs_selection {
        info!("No user game project selected (or pointed to Bevy-Editor). Requesting user project folder...");
        let Some(project_dir) = rfd::FileDialog::new()
            .set_title("Select User Game Project Folder (containing Cargo.toml)")
            .pick_folder()
        else {
            info!("Build cancelled by user (no folder selected).");
            return;
        };
        if is_editor_repository(&project_dir) {
            warn!("Selected folder is Bevy-Editor itself! Please select a standalone game project.");
            if let Some(mut status) = world.get_resource_mut::<crate::export::ExportStatus>() {
                *status = crate::export::ExportStatus::Failed {
                    target: format!("Build {target_id}"),
                    error: "Cannot build Bevy-Editor as a target game. Please choose your user project.".to_string(),
                };
            }
            return;
        }
        if !project_dir.join("Cargo.toml").is_file() {
            warn!("Selected project folder does not contain Cargo.toml: {}", project_dir.display());
            return;
        }
        project.set_project_dir(project_dir);
        *world.resource_mut::<crate::ui::hub::ActiveProject>() = project.clone();
    }

    let project_dir = project.project_dir.expect("project directory was initialized");
    if !project_dir.join("Cargo.toml").is_file() {
        warn!("Active project does not contain Cargo.toml: {}", project_dir.display());
        return;
    }
    let output_dir = project.build_dir.unwrap_or_else(|| {
        let parent = project_dir.parent().unwrap_or(&project_dir);
        parent.join(format!("{}-build", project.name.as_deref().unwrap_or("BevyGame")))
    });

    // Ensure active scene is saved to disk before building so the export always includes it
    let mut current_scene_path = world.resource::<CurrentScene>().file_path.clone();
    if current_scene_path.is_none() {
        let scenes_dir = project_dir.join("assets").join("scenes");
        let _ = std::fs::create_dir_all(&scenes_dir);
        let auto_scene = scenes_dir.join("default_scene.bsn");
        if let Ok(()) = save_load::save_world_to_scene(world, &auto_scene) {
            info!("Auto-saved active editor scene to {}", auto_scene.display());
            if let Some(mut cur) = world.get_resource_mut::<CurrentScene>() {
                cur.file_path = Some(auto_scene.clone());
                cur.is_modified = false;
            }
            current_scene_path = Some(auto_scene);
        }
    } else if let Some(ref path) = current_scene_path {
        // Save any pending changes to current scene before build
        let _ = save_load::save_world_to_scene(world, path);
    }

    world.write_message(RequestExportMessage {
        target_id: target_id.to_string(),
        output_dir,
        project_name: project.name.unwrap_or_else(|| "BevyGame".to_string()),
        project_dir: Some(project_dir),
        scene_path: current_scene_path,
    });
}

/// State tracking live commands received from the MCP Server bridge file.
#[derive(Resource, Default)]
pub struct McpBridgeState {
    pub last_timestamp: u64,
}

/// Polls `assets/.mcp_bridge.json` to reflect live AI/MCP operations directly in the editor UI.
pub fn update_mcp_bridge_status(
    mut mcp_state: ResMut<McpBridgeState>,
    mut label_query: Query<&mut Text, With<McpStatusLabel>>,
    mut export_status: ResMut<crate::export::ExportStatus>,
) {
    let bridge_path = std::path::Path::new("assets/.mcp_bridge.json");
    if let Ok(metadata) = std::fs::metadata(bridge_path) {
        if let Ok(modified) = metadata.modified() {
            let dur = modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            if dur > mcp_state.last_timestamp {
                mcp_state.last_timestamp = dur;
                if let Ok(content) = std::fs::read_to_string(bridge_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        let action = val.get("action").and_then(|a| a.as_str()).unwrap_or("Active");
                        let entity = val.get("entity").and_then(|e| e.as_str()).unwrap_or("");
                        let details = val.get("details").and_then(|d| d.as_str()).unwrap_or("");

                        if let Ok(mut txt) = label_query.single_mut() {
                            txt.0 = format!("MCP: {} ({})", action.to_uppercase(), entity);
                        }

                        *export_status = crate::export::ExportStatus::Success {
                            target: "MCP Controller".to_string(),
                            message: format!("AI Event: {} on '{}' -> {}", action, entity, details),
                            path: std::path::PathBuf::from("assets/.mcp_bridge.json"),
                        };
                    }
                }
            }
        }
    }
}

/// Handles switching workspace layout presets and updating UI layout accordingly.
pub fn handle_switch_workspace(
    mut active_ws: ResMut<ActiveWorkspace>,
    mut reader: MessageReader<SwitchWorkspaceMessage>,
    mut label_query: Query<&mut Text, With<WorkspaceSelectorLabel>>,
    mut hierarchy_state: ResMut<crate::ui::hierarchy::HierarchyState>,
    mut inspector_state: ResMut<crate::ui::inspector::InspectorState>,
    mut asset_browser_state: ResMut<crate::ui::asset_browser::AssetBrowserState>,
    mut dock_query: Query<&mut Node, With<crate::ui::hierarchy::HierarchyDockRoot>>,
    mut insp_query: Query<&mut Node, (With<crate::ui::inspector::InspectorDockRoot>, Without<crate::ui::hierarchy::HierarchyDockRoot>)>,
    mut asset_query: Query<&mut Node, (With<crate::ui::asset_browser::ProjectExplorerDockRoot>, Without<crate::ui::hierarchy::HierarchyDockRoot>, Without<crate::ui::inspector::InspectorDockRoot>)>,
) {
    for msg in reader.read() {
        active_ws.0 = msg.0;

        if let Ok(mut label) = label_query.single_mut() {
            label.0 = format!("Workspace: {}", active_ws.0.label());
        }

        match active_ws.0 {
            WorkspacePreset::Default => {
                hierarchy_state.is_collapsed = false;
                hierarchy_state.width = 250.0;
                inspector_state.is_collapsed = false;
                inspector_state.width = 380.0;
                asset_browser_state.is_collapsed = false;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(250.0);
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(380.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(250.0);
                    node.display = Display::Flex;
                }
            }
            WorkspacePreset::LevelDesign => {
                // Maximum viewport room: slim side panels, minimize asset browser
                hierarchy_state.is_collapsed = false;
                hierarchy_state.width = 220.0;
                inspector_state.is_collapsed = false;
                inspector_state.width = 320.0;
                asset_browser_state.is_collapsed = true;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(220.0);
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(320.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(32.0); // Collapsed
                }
            }
            WorkspacePreset::Programming => {
                // Collapse hierarchy, focus inspector and asset scripts
                hierarchy_state.is_collapsed = true;
                inspector_state.is_collapsed = false;
                inspector_state.width = 420.0;
                asset_browser_state.is_collapsed = false;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(34.0); // Collapsed
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(420.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(300.0);
                    node.display = Display::Flex;
                }
            }
            WorkspacePreset::Debugging => {
                // Expanded inspector for component diagnostics
                hierarchy_state.is_collapsed = false;
                hierarchy_state.width = 280.0;
                inspector_state.is_collapsed = false;
                inspector_state.width = 460.0;
                inspector_state.is_debug_view = true;
                asset_browser_state.is_collapsed = true;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(280.0);
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(460.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(32.0);
                }
            }
            WorkspacePreset::Assets => {
                // Focus bottom asset browser
                hierarchy_state.is_collapsed = false;
                hierarchy_state.width = 220.0;
                inspector_state.is_collapsed = false;
                inspector_state.width = 320.0;
                asset_browser_state.is_collapsed = false;

                if let Ok(mut node) = dock_query.single_mut() {
                    node.width = px(220.0);
                }
                if let Ok(mut node) = insp_query.single_mut() {
                    node.width = px(320.0);
                }
                if let Ok(mut node) = asset_query.single_mut() {
                    node.height = px(380.0);
                    node.display = Display::Flex;
                }
            }
        }
    }
}

/// Dynamically updates both the OS native Window title and the top menu bar title pill
/// to reflect the format: "(project name - scene name)".
pub fn update_window_and_menu_title(
    active_project: Res<crate::ui::hub::ActiveProject>,
    current_scene: Res<CurrentScene>,
    mut windows: Query<&mut Window>,
    mut title_labels: Query<&mut Text, With<WindowTitleLabel>>,
) {
    if !active_project.is_changed() && !current_scene.is_changed() {
        return;
    }

    let project_name = active_project
        .name
        .clone()
        .or_else(|| {
            active_project
                .project_dir
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "No Project".to_string());

    let scene_name = match &current_scene.file_path {
        Some(path) => path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("scene.bsn")
            .to_string(),
        None => "Untitled Scene".to_string(),
    };

    let title_string = format!("{} - {}{}", project_name, scene_name, if current_scene.is_modified { " *" } else { "" });
    let window_title = format!("Bevy Editor — Bevylite ({})", title_string);

    for mut window in &mut windows {
        if window.title != window_title {
            window.title = window_title.clone();
        }
    }

    for mut text in &mut title_labels {
        text.0 = title_string.clone();
    }
}



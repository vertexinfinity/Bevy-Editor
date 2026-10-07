//! Project Hub / Launcher Screen for Bevylite.
//!
//! Provides a dedicated welcome screen when starting the editor without an open project:
//! - Direct actions: "New Project", "Open Project"
//! - Recent Projects list (Name, path, last opened date, engine version badge)
//! - Starter Templates grid (Blank Project, 3D Game, First Person, Third Person, Strategy)
//! - 1-click launch into the editor workspace

use std::path::PathBuf;
use bevy::{
    ecs::message::Message,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

/// State of the Editor Application: Project Hub vs Active Editor.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorAppState {
    #[default]
    ProjectHub,
    InEditor,
}

/// Message fired to open a project from the hub.
#[derive(Message, Clone, Debug)]
pub struct OpenProjectMessage {
    pub path: PathBuf,
    pub name: String,
}

/// Message fired to create and open a project from a template.
#[derive(Message, Clone, Debug)]
pub struct CreateFromTemplateMessage {
    pub template_id: &'static str,
    pub project_name: String,
}

/// Marker component for the Project Hub root UI container.
#[derive(Component)]
pub struct ProjectHubRoot;

#[derive(Clone, Debug)]
pub struct RecentProjectItem {
    pub name: &'static str,
    pub path: &'static str,
    pub last_opened: &'static str,
    pub version: &'static str,
}

#[derive(Clone, Debug)]
pub struct ProjectTemplateItem {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

// User-configured / active local projects
const RECENT_PROJECTS: &[RecentProjectItem] = &[
    RecentProjectItem {
        name: "Bevylite Workspace",
        path: "D:/Repos/Bevy/Bevy-Editor",
        last_opened: "Current Workspace",
        version: "Bevy 0.20-dev",
    },
];

// Sample repository templates loadable directly
const TEMPLATES: &[ProjectTemplateItem] = &[
    ProjectTemplateItem {
        id: "blank",
        title: "Blank Project",
        description: "Empty canvas ready for custom game systems.",
        icon: "⬚",
    },
    ProjectTemplateItem {
        id: "3d_game",
        title: "3D Game (Sample)",
        description: "3D scene with ground, lighting, and sample props.",
        icon: "⬡",
    },
    ProjectTemplateItem {
        id: "first_person",
        title: "First Person (Sample)",
        description: "Fly & FPS camera controller with WASD movement.",
        icon: "◈",
    },
    ProjectTemplateItem {
        id: "third_person",
        title: "Third Person (Sample)",
        description: "Character controller with orbit camera.",
        icon: "◉",
    },
    ProjectTemplateItem {
        id: "strategy",
        title: "Strategy / Top-down (Sample)",
        description: "RTS camera with unit selection sample.",
        icon: "⊞",
    },
];

/// Spawns the dedicated Project Hub UI screen.
pub fn spawn_project_hub(mut commands: Commands) {
    let hub_root = commands
        .spawn((
            EditorInternal,
            ProjectHubRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100.0),
                height: percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::axes(px(40), px(32)),
                row_gap: px(24),
                ..default()
            },
            BackgroundColor(BG_SHELL),
        ))
        .id();

    // 1. Top Header: Product Identity & Actions
    let header = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100.0),
                max_width: px(1120),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                border: UiRect::bottom(px(1)),
                padding: UiRect::bottom(px(20)),
                ..default()
            },
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(hub_root),
        ))
        .id();

    // Brand Left
    let brand_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                ..default()
            },
            ChildOf(header),
        ))
        .id();

    // Brand badge
    let brand_badge = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(4)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_CARD),
            ChildOf(brand_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("BL"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SECTION),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(brand_badge),
    ));

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("BEVYLITE"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_TITLE),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(brand_group),
    ));

    // Header Action Buttons
    let actions_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(12),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(header),
        ))
        .id();

    // New Project Button (Primary Cyan)
    let new_btn = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(34),
                padding: UiRect::axes(px(16), px(6)),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(ACCENT_CYAN),
            ChildOf(actions_group),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(Color::srgb(0.2, 0.98, 1.0));
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(ACCENT_CYAN);
            }
        })
        .observe(|click: On<PointerClick>, mut state: ResMut<NextState<EditorAppState>>| {
            if click.button == PointerButton::Primary {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    let _ = crate::project::ProjectScaffolder::create_project(&folder, "MyBevyGame");
                }
                state.set(EditorAppState::InEditor);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("+ New Project"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(Color::srgb(0.04, 0.05, 0.07)), // High-contrast dark text on cyan
        ChildOf(new_btn),
    ));

    // Open Project Button
    let open_btn = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(34),
                padding: UiRect::axes(px(16), px(6)),
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(actions_group),
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
        .observe(|click: On<PointerClick>, mut state: ResMut<NextState<EditorAppState>>| {
            if click.button == PointerButton::Primary {
                let _ = rfd::FileDialog::new().pick_folder();
                state.set(EditorAppState::InEditor);
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Open Project..."),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(open_btn),
    ));

    // 2. Main Content Split: Left = Recent Projects (60%), Right = Templates (40%)
    let main_content = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100.0),
                max_width: px(1120),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(28),
                ..default()
            },
            ChildOf(hub_root),
        ))
        .id();

    // --- Left Section: Recent Projects ---
    let recents_section = commands
        .spawn((
            EditorInternal,
            Node {
                flex_grow: 1.4,
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            ChildOf(main_content),
        ))
        .id();

    // Section title
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Recent Projects"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SECTION),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(recents_section),
    ));

    let recents_list = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            ChildOf(recents_section),
        ))
        .id();

    for item in RECENT_PROJECTS {
        spawn_recent_project_card(&mut commands, recents_list, item);
    }

    // --- Right Section: Project Templates ---
    let templates_section = commands
        .spawn((
            EditorInternal,
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            ChildOf(main_content),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Templates"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SECTION),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(templates_section),
    ));

    let templates_grid = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            ChildOf(templates_section),
        ))
        .id();

    for tpl in TEMPLATES {
        spawn_template_card(&mut commands, templates_grid, tpl);
    }
}

fn spawn_recent_project_card(
    commands: &mut Commands,
    parent: Entity,
    item: &RecentProjectItem,
) {
    let card = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(58),
                padding: UiRect::axes(px(14), px(8)),
                border: UiRect::all(px(1)),
                border_radius: radius_card(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>, mut border: Query<&mut BorderColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
            if let Ok(mut border) = border.get_mut(trigger.entity) {
                *border = BorderColor::all(BORDER_ACCENT);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>, mut border: Query<&mut BorderColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_CARD);
            }
            if let Ok(mut border) = border.get_mut(trigger.entity) {
                *border = BorderColor::all(BORDER_SUBTLE);
            }
        })
        .observe(|click: On<PointerClick>, mut state: ResMut<NextState<EditorAppState>>| {
            if click.button == PointerButton::Primary {
                state.set(EditorAppState::InEditor);
            }
        })
        .id();

    // Left info (Name & Path)
    let left_info = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                ..default()
            },
            ChildOf(card),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(item.name),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(left_info),
    ));

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(item.path),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(left_info),
    ));

    // Right info (Date & Version pill)
    let right_info = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(12),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(card),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(item.last_opened),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(right_info),
    ));

    let version_badge = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(2)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(right_info),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(item.version),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL - 1.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(version_badge),
    ));
}

fn spawn_template_card(
    commands: &mut Commands,
    parent: Entity,
    tpl: &ProjectTemplateItem,
) {
    let card = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(58),
                padding: UiRect::axes(px(14), px(8)),
                border: UiRect::all(px(1)),
                border_radius: radius_card(),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>, mut border: Query<&mut BorderColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
            if let Ok(mut border) = border.get_mut(trigger.entity) {
                *border = BorderColor::all(BORDER_ACCENT);
            }
        })
        .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>, mut border: Query<&mut BorderColor>| {
            if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                *bg = BackgroundColor(BG_CARD);
            }
            if let Ok(mut border) = border.get_mut(trigger.entity) {
                *border = BorderColor::all(BORDER_SUBTLE);
            }
        })
        .observe(|click: On<PointerClick>, mut state: ResMut<NextState<EditorAppState>>| {
            if click.button == PointerButton::Primary {
                state.set(EditorAppState::InEditor);
            }
        })
        .id();

    // Template Icon
    let icon_box = commands
        .spawn((
            EditorInternal,
            Node {
                width: px(36),
                height: px(36),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(card),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(tpl.icon),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SECTION),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(icon_box),
    ));

    // Template Text Details
    let text_col = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                flex_grow: 1.0,
                ..default()
            },
            ChildOf(card),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(tpl.title),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(text_col),
    ));

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(tpl.description),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(text_col),
    ));
}

/// System to despawn the Hub when transitioning into InEditor.
pub fn despawn_project_hub(
    mut commands: Commands,
    query: Query<Entity, With<ProjectHubRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

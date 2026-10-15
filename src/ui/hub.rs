//! Project Hub / Launcher Screen for Bevylite.
//!
//! Provides a cleanly separated welcome screen:
//! - Section 1: "Your Projects" (Recent/active user projects with quick launch into editor)
//! - Section 2: "Start from a Template" (Rich starter templates with playable game mechanics)

use std::path::{Path, PathBuf};
use bevy::{
    ecs::message::Message,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::project::scaffold::{ProjectScaffolder, ProjectTemplateKind};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

/// State of the Editor Application: Project Hub vs Active Editor.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorAppState {
    #[default]
    ProjectHub,
    InEditor,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ActiveProject {
    pub project_dir: Option<PathBuf>,
    pub build_dir: Option<PathBuf>,
    pub name: Option<String>,
}

impl ActiveProject {
    pub fn set_project_dir(&mut self, project_dir: PathBuf) {
        let resolved = project_dir.canonicalize().unwrap_or(project_dir);
        let name = resolved
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("BevyGame")
            .to_string();
        let parent = resolved.parent().unwrap_or(&resolved);
        self.build_dir = Some(parent.join(format!("{name}-build")));
        self.project_dir = Some(resolved);
        self.name = Some(name);
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UserProjectEntry {
    pub name: String,
    pub path: String,
    pub last_opened: String,
    pub template: String,
}

#[derive(Resource, Debug, Clone)]
pub struct ProjectRegistry {
    pub projects: Vec<UserProjectEntry>,
}

impl Default for ProjectRegistry {
    fn default() -> Self {
        Self::load()
    }
}

impl ProjectRegistry {
    pub fn config_path() -> PathBuf {
        let local_dir = PathBuf::from(".bevy_editor");
        let _ = std::fs::create_dir_all(&local_dir);
        local_dir.join("recent_projects.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let mut projects = Vec::new();
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(loaded) = serde_json::from_str::<Vec<UserProjectEntry>>(&data) {
                projects = loaded;
            }
        }

        // If empty, auto-detect any existing projects in standard workspace paths
        if projects.is_empty() {
            let sample_dirs = [
                "D:\\Projects\\Bevy\\Bevy-project\\mybevygame\\mybevygame",
                "D:\\Projects\\Bevy\\Bevy-project\\mybevygame",
            ];
            for dir_str in sample_dirs {
                let p = PathBuf::from(dir_str);
                if p.join("Cargo.toml").is_file() {
                    let name = p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("MyBevyGame")
                        .to_string();
                    projects.push(UserProjectEntry {
                        name,
                        path: dir_str.to_string(),
                        last_opened: "Recent".to_string(),
                        template: "3D Game".to_string(),
                    });
                    break;
                }
            }
        }

        Self { projects }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(&self.projects) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn add_or_update(&mut self, path: &Path, name: &str, template: &str) {
        let path_str = path.to_string_lossy().to_string();
        self.projects.retain(|p| p.path != path_str);
        self.projects.insert(
            0,
            UserProjectEntry {
                name: name.to_string(),
                path: path_str,
                last_opened: "Just now".to_string(),
                template: template.to_string(),
            },
        );
        if self.projects.len() > 10 {
            self.projects.truncate(10);
        }
        self.save();
    }
}

#[derive(Component)]
pub struct HubProjectPathLabel;

#[derive(Component)]
pub struct HubBuildPathLabel;

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
pub struct ProjectTemplateItem {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub badge: &'static str,
    pub icon: &'static str,
}

const TEMPLATES: &[ProjectTemplateItem] = &[
    ProjectTemplateItem {
        id: "action3d",
        title: "3D Action / Character Controller",
        description: "Playable WASD character movement with jump, smooth 3rd-person follow camera, lighting, and spinning collectible props.",
        badge: "Playable • 3rd Person",
        icon: "🎮",
    },
    ProjectTemplateItem {
        id: "first_person",
        title: "First Person (FPS) Controller",
        description: "First-person camera with WASD navigation, mouse look, crosshair, and 3D arena geometry.",
        badge: "Playable • FPS",
        icon: "🎯",
    },
    ProjectTemplateItem {
        id: "orbit_viewer",
        title: "Orbit 3D Scene / Model Viewer",
        description: "Interactive turntable camera (rotate, pan, zoom), PBR materials, and showcase studio lighting.",
        badge: "Inspect • Turntable",
        icon: "🔍",
    },
    ProjectTemplateItem {
        id: "strategy",
        title: "Top-Down / Strategy (RTS)",
        description: "RTS isometric camera with WASD / pan controls, zoom, terrain grid, and click-to-move destination beacon.",
        badge: "Strategy • RTS",
        icon: "🗺",
    },
];

/// Spawns the dedicated Project Hub UI screen with cleanly separated sections.
pub fn spawn_project_hub(mut commands: Commands, registry: Res<ProjectRegistry>) {
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
                padding: UiRect::axes(px(48), px(28)),
                row_gap: px(20),
                overflow: Overflow::clip_y(),
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
                padding: UiRect::bottom(px(16)),
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

    // New Empty Project Button (Cyan)
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
        .observe(
            |click: On<PointerClick>,
             mut state: ResMut<NextState<EditorAppState>>,
             mut active_project: ResMut<ActiveProject>,
             mut registry: ResMut<ProjectRegistry>| {
                if click.button == PointerButton::Primary {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        if let Ok(project_dir) =
                            ProjectScaffolder::create_project(&folder, "MyBevyGame")
                        {
                            registry.add_or_update(&project_dir, "MyBevyGame", "Blank Project");
                            active_project.set_project_dir(project_dir);
                            state.set(EditorAppState::InEditor);
                        }
                    }
                }
            },
        )
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("+ New Empty Project"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(Color::srgb(0.04, 0.05, 0.07)),
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
        .observe(
            |click: On<PointerClick>,
             mut state: ResMut<NextState<EditorAppState>>,
             mut active_project: ResMut<ActiveProject>,
             mut registry: ResMut<ProjectRegistry>,
             mut commands: Commands,
             asset_server: Res<AssetServer>| {
                if click.button == PointerButton::Primary {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_title("Open Bevy Game Project (Folder with Cargo.toml)")
                        .pick_folder()
                    {
                        if folder.join("Cargo.toml").is_file() {
                            let name = folder
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("BevyGame")
                                .to_string();
                            registry.add_or_update(&folder, &name, "Loaded Project");
                            active_project.set_project_dir(folder.clone());

                            let default_scene =
                                folder.join("assets").join("scenes").join("default_scene.bsn");
                            if default_scene.is_file() {
                                let scene_path_str = default_scene.to_string_lossy().to_string();
                                commands.spawn((
                                    crate::scene::SceneEntity,
                                    Name::new("Default Scene"),
                                    bevy::world_serialization::DynamicWorldRoot(
                                        asset_server.load(scene_path_str),
                                    ),
                                    Transform::IDENTITY,
                                ));
                            }

                            state.set(EditorAppState::InEditor);
                        }
                    }
                }
            },
        )
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Open Project Folder..."),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(open_btn),
    ));

    // ==========================================
    // SECTION 1: "Your Projects"
    // ==========================================
    let section1 = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100.0),
                max_width: px(1120),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                ..default()
            },
            ChildOf(hub_root),
        ))
        .id();

    let s1_header = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Baseline,
                column_gap: px(12),
                ..default()
            },
            ChildOf(section1),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Your Projects"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SECTION),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(s1_header),
    ));

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Select an existing project to continue editing, or open a project folder."),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(s1_header),
    ));

    let projects_list = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            ChildOf(section1),
        ))
        .id();

    if registry.projects.is_empty() {
        // Empty state card
        let empty_card = commands
            .spawn((
                EditorInternal,
                Node {
                    width: percent(100.0),
                    padding: UiRect::axes(px(16), px(16)),
                    border: UiRect::all(px(1)),
                    border_radius: radius_card(),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(BG_CARD),
                BorderColor::all(BORDER_SUBTLE),
                ChildOf(projects_list),
            ))
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new("No projects found yet. Open an existing folder or choose a Starter Template below to create your first game!"),
            TextFont {
                font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
                ..default()
            },
            TextColor(TEXT_MUTED),
            ChildOf(empty_card),
        ));
    } else {
        for proj in &registry.projects {
            spawn_user_project_card(&mut commands, projects_list, proj);
        }
    }

    // Divider Line
    commands.spawn((
        EditorInternal,
        Node {
            width: percent(100.0),
            max_width: px(1120),
            height: px(1),
            margin: UiRect::axes(px(0), px(8)),
            ..default()
        },
        BackgroundColor(BORDER_SUBTLE),
        ChildOf(hub_root),
    ));

    // ==========================================
    // SECTION 2: "Start from a Template"
    // ==========================================
    let section2 = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100.0),
                max_width: px(1120),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            },
            ChildOf(hub_root),
        ))
        .id();

    let s2_header = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Baseline,
                column_gap: px(12),
                ..default()
            },
            ChildOf(section2),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Start from a Template"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SECTION),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(s2_header),
    ));

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Click a template to create a ready-to-run game project with playable mechanics, lighting, and sample scenes."),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(s2_header),
    ));

    let templates_grid = commands
        .spawn((
            EditorInternal,
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            ChildOf(section2),
        ))
        .id();

    for tpl in TEMPLATES {
        spawn_template_card(&mut commands, templates_grid, tpl);
    }
}

fn spawn_user_project_card(commands: &mut Commands, parent: Entity, item: &UserProjectEntry) {
    let project_path = item.path.clone();
    let project_name = item.name.clone();
    let project_template = item.template.clone();

    let card = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(56),
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
        .observe(
            |trigger: On<PointerOver>,
             mut bg: Query<&mut BackgroundColor>,
             mut border: Query<&mut BorderColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(BG_HOVER);
                }
                if let Ok(mut border) = border.get_mut(trigger.entity) {
                    *border = BorderColor::all(BORDER_ACCENT);
                }
            },
        )
        .observe(
            |trigger: On<PointerOut>,
             mut bg: Query<&mut BackgroundColor>,
             mut border: Query<&mut BorderColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(BG_CARD);
                }
                if let Ok(mut border) = border.get_mut(trigger.entity) {
                    *border = BorderColor::all(BORDER_SUBTLE);
                }
            },
        )
        .observe(
            move |click: On<PointerClick>,
                  mut state: ResMut<NextState<EditorAppState>>,
                  mut active_project: ResMut<ActiveProject>,
                  mut registry: ResMut<ProjectRegistry>,
                  mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<StandardMaterial>>,
                  existing_scene_entities: Query<Entity, With<crate::scene::SceneEntity>>| {
                if click.button == PointerButton::Primary {
                    let p = PathBuf::from(&project_path);
                    if p.join("Cargo.toml").is_file() {
                        registry.add_or_update(&p, &project_name, &project_template);
                        active_project.set_project_dir(p.clone());

                        // Clear any existing entities so they don't stack up
                        for ent in &existing_scene_entities {
                            commands.entity(ent).despawn();
                        }

                        // Spawn complete 3D scene with live meshes, materials, and controllers
                        let tpl_kind = ProjectTemplateKind::from_name(&project_template);
                        crate::scene::primitives::spawn_template_scene(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            tpl_kind,
                        );

                        state.set(EditorAppState::InEditor);
                    }
                }
            },
        )
        .id();

    // Left info (Icon, Name, Path)
    let left_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                ..default()
            },
            ChildOf(card),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("📦"),
        TextFont {
            font_size: bevy::text::FontSize::Px(16.0),
            ..default()
        },
        ChildOf(left_group),
    ));

    let text_col = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                ..default()
            },
            ChildOf(left_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(&item.name),
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
        Text::new(&item.path),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(text_col),
    ));

    // Right info (Template badge & Launch button)
    let right_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                ..default()
            },
            ChildOf(card),
        ))
        .id();

    let badge = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(right_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(&item.template),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL - 1.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(badge),
    ));

    let launch_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(10), px(4)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(ACCENT_CYAN_DIM),
            ChildOf(right_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Open in Editor ➜"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(launch_btn),
    ));
}

fn spawn_template_card(commands: &mut Commands, parent: Entity, tpl: &ProjectTemplateItem) {
    let template_id = tpl.id;
    let template_title = tpl.title;

    let card = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(62),
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
        .observe(
            |trigger: On<PointerOver>,
             mut bg: Query<&mut BackgroundColor>,
             mut border: Query<&mut BorderColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(BG_HOVER);
                }
                if let Ok(mut border) = border.get_mut(trigger.entity) {
                    *border = BorderColor::all(BORDER_ACCENT);
                }
            },
        )
        .observe(
            |trigger: On<PointerOut>,
             mut bg: Query<&mut BackgroundColor>,
             mut border: Query<&mut BorderColor>| {
                if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                    *bg = BackgroundColor(BG_CARD);
                }
                if let Ok(mut border) = border.get_mut(trigger.entity) {
                    *border = BorderColor::all(BORDER_SUBTLE);
                }
            },
        )
        .observe(
            move |click: On<PointerClick>,
                  mut state: ResMut<NextState<EditorAppState>>,
                  mut active_project: ResMut<ActiveProject>,
                  mut registry: ResMut<ProjectRegistry>,
                  mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<StandardMaterial>>,
                  existing_scene_entities: Query<Entity, With<crate::scene::SceneEntity>>| {
                if click.button == PointerButton::Primary {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_title(&format!("Create Project: {}", template_title))
                        .pick_folder()
                    {
                        let tpl_kind = ProjectTemplateKind::from_id(template_id);
                        let proj_name = format!(
                            "My{}",
                            tpl_kind
                                .display_name()
                                .split_whitespace()
                                .next()
                                .unwrap_or("Game")
                        );
                        if let Ok(project_dir) = ProjectScaffolder::create_project_with_template(
                            &folder,
                            &proj_name,
                            tpl_kind,
                        ) {
                            registry.add_or_update(
                                &project_dir,
                                &proj_name,
                                tpl_kind.display_name(),
                            );
                            active_project.set_project_dir(project_dir.clone());

                            // Clear any existing entities and spawn full rich playable scene directly
                            for ent in &existing_scene_entities {
                                commands.entity(ent).despawn();
                            }
                            crate::scene::primitives::spawn_template_scene(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                tpl_kind,
                            );

                            state.set(EditorAppState::InEditor);
                        }
                    }
                }
            },
        )
        .id();

    // Left info (Icon + Title + Description)
    let left_group = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(12),
                flex_grow: 1.0,
                ..default()
            },
            ChildOf(card),
        ))
        .id();

    let icon_box = commands
        .spawn((
            EditorInternal,
            Node {
                width: px(38),
                height: px(38),
                border_radius: radius_button(),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(left_group),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(tpl.icon),
        TextFont {
            font_size: bevy::text::FontSize::Px(18.0),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(icon_box),
    ));

    let details_col = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                flex_grow: 1.0,
                ..default()
            },
            ChildOf(left_group),
        ))
        .id();

    let title_row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(8),
                ..default()
            },
            ChildOf(details_col),
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
        ChildOf(title_row),
    ));

    let badge = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(ACCENT_CYAN_DIM),
            ChildOf(title_row),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(tpl.badge),
        TextFont {
            font_size: bevy::text::FontSize::Px(9.5),
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(badge),
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
        ChildOf(details_col),
    ));

    // Right Action: "Create Project ➜"
    let action_btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(12), px(6)),
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(card),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Create Project ➜"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(action_btn),
    ));
}

pub fn update_project_path_labels(
    active_project: Res<ActiveProject>,
    mut project_labels: Query<&mut Text, (With<HubProjectPathLabel>, Without<HubBuildPathLabel>)>,
    mut build_labels: Query<&mut Text, (With<HubBuildPathLabel>, Without<HubProjectPathLabel>)>,
) {
    if !active_project.is_changed() {
        return;
    }

    if let Ok(mut text) = project_labels.single_mut() {
        text.0 = active_project
            .project_dir
            .as_ref()
            .map(|path| format!("Project: {}", path.display()))
            .unwrap_or_else(|| "Project: Not selected".to_string());
    }
    if let Ok(mut text) = build_labels.single_mut() {
        text.0 = active_project
            .build_dir
            .as_ref()
            .map(|path| format!("Build: {}", path.display()))
            .unwrap_or_else(|| "Build: Sibling folder (default)".to_string());
    }
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

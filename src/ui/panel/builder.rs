//! Declarative builder for standardized editor panels.

use bevy::{
    ecs::message::MessageWriter,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::scene::EditorInternal;
use crate::ui::theme::*;
use super::types::*;

pub struct PanelBuilder {
    pub id: PanelId,
    pub title: String,
    pub icon: String,
    pub dock_zone: PanelDockZone,
    pub width: Val,
    pub height: Val,
    pub max_width: Val,
    pub max_height: Val,
    pub position_left: Val,
    pub position_right: Val,
    pub position_top: Val,
    pub position_bottom: Val,
    pub show_collapse: bool,
    pub show_close: bool,
}

impl PanelBuilder {
    pub fn new(id: PanelId, title: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            icon: icon.into(),
            dock_zone: PanelDockZone::Right,
            width: px(380),
            height: percent(100.0),
            max_width: Val::Auto,
            max_height: Val::Auto,
            position_left: Val::Auto,
            position_right: Val::Auto,
            position_top: Val::Auto,
            position_bottom: Val::Auto,
            show_collapse: true,
            show_close: false,
        }
    }

    pub fn dock_zone(mut self, zone: PanelDockZone) -> Self {
        self.dock_zone = zone;
        self
    }

    pub fn size(mut self, width: Val, height: Val) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn position(mut self, left: Val, top: Val, right: Val, bottom: Val) -> Self {
        self.position_left = left;
        self.position_top = top;
        self.position_right = right;
        self.position_bottom = bottom;
        self
    }

    pub fn show_close(mut self, enable: bool) -> Self {
        self.show_close = enable;
        self
    }

    /// Spawns the standard themed panel frame and returns (panel_root, header_actions_container, body_container).
    pub fn spawn_shell(self, commands: &mut Commands) -> (Entity, Entity, Entity) {
        let id = self.id;
        let title_label = format!("{}  {}", self.icon, self.title);

        let root = commands
            .spawn((
                EditorInternal,
                PanelRoot(id),
                PanelMetadata {
                    id,
                    title: self.title.clone(),
                    icon: self.icon.clone(),
                    dock_zone: self.dock_zone,
                    is_collapsed: false,
                    is_open: true,
                    default_width: 380.0,
                    default_height: 260.0,
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: self.position_left,
                    right: self.position_right,
                    top: self.position_top,
                    bottom: self.position_bottom,
                    width: self.width,
                    max_width: self.max_width,
                    height: self.height,
                    max_height: self.max_height,
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(px(1)),
                    border_radius: radius_panel(),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(BG_PANEL),
                BorderColor::all(BORDER_SUBTLE),
            ))
            .id();

        // 1. Draggable Header Bar
        let header = commands
            .spawn((
                EditorInternal,
                PanelHeaderBar(id),
                Node {
                    height: HEIGHT_PANEL_HEADER,
                    padding: UiRect::axes(px(8), px(4)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::bottom(px(1)),
                    ..default()
                },
                BackgroundColor(BG_CARD),
                BorderColor::all(BORDER_SUBTLE),
                ChildOf(root),
            ))
            .id();

        // Header Left: Icon & Title
        let header_left = commands
            .spawn((
                EditorInternal,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(6),
                    ..default()
                },
                ChildOf(header),
            ))
            .id();

        commands.spawn((
            EditorInternal,
            PanelTitleText(id),
            Pickable::IGNORE,
            Text::new(title_label),
            TextFont {
                font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
                weight: FontWeight::SEMIBOLD,
                ..default()
            },
            TextColor(TEXT_PRIMARY),
            ChildOf(header_left),
        ));

        // Header Right: Action Slots & Window Controls
        let header_right = commands
            .spawn((
                EditorInternal,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(4),
                    ..default()
                },
                ChildOf(header),
            ))
            .id();

        // Slot for custom action buttons (e.g. + New, Up, Sync)
        let actions_container = commands
            .spawn((
                EditorInternal,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(4),
                    ..default()
                },
                ChildOf(header_right),
            ))
            .id();

        // Collapse / Expand toggle button
        if self.show_collapse {
            let collapse_btn = commands
                .spawn((
                    EditorInternal,
                    PanelCollapseButton(id),
                    Node {
                        padding: UiRect::axes(px(6), px(2)),
                        border: UiRect::all(px(1)),
                        border_radius: radius_button(),
                        ..default()
                    },
                    BackgroundColor(BG_INPUT),
                    BorderColor::all(BORDER_SUBTLE),
                    ChildOf(header_right),
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
                .observe(move |click: On<PointerClick>, mut writer: MessageWriter<TogglePanelCollapseMessage>| {
                    if click.button == PointerButton::Primary {
                        writer.write(TogglePanelCollapseMessage(id));
                    }
                })
                .id();

            commands.spawn((
                EditorInternal,
                PanelCollapseLabel(id),
                Pickable::IGNORE,
                Text::new("v"),
                TextFont {
                    font_size: bevy::text::FontSize::Px(10.0),
                    ..default()
                },
                TextColor(TEXT_SECONDARY),
                ChildOf(collapse_btn),
            ));
        }

        // Optional Close Button
        if self.show_close {
            commands
                .spawn((
                    EditorInternal,
                    PanelCloseButton(id),
                    Node {
                        padding: UiRect::axes(px(6), px(2)),
                        border: UiRect::all(px(1)),
                        border_radius: radius_button(),
                        ..default()
                    },
                    BackgroundColor(BG_INPUT),
                    BorderColor::all(BORDER_SUBTLE),
                    ChildOf(header_right),
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
                .observe(move |click: On<PointerClick>, mut writer: MessageWriter<TogglePanelMessage>| {
                    if click.button == PointerButton::Primary {
                        writer.write(TogglePanelMessage(id));
                    }
                })
                .with_child((
                    EditorInternal,
                    Pickable::IGNORE,
                    Text::new("✕"),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(TEXT_MUTED),
                ));
        }

        // 2. Content Body Container
        let body = commands
            .spawn((
                EditorInternal,
                PanelBodyContainer(id),
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
                ChildOf(root),
            ))
            .id();

        (root, actions_container, body)
    }
}

//! Declarative CardBuilder for standardized inspector sections & custom script cards.

use bevy::{
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

/// Marker for card root containers.
#[derive(Component)]
pub struct CardRoot;

/// Marker for card header bars.
#[derive(Component)]
pub struct CardHeader;

/// Marker for card body containers.
#[derive(Component)]
pub struct CardBody;

/// Marker for card collapse arrow labels.
#[derive(Component)]
pub struct CardCollapseArrow;

/// Declarative builder for creating uniform, collapsible cards inside panels.
pub struct CardBuilder {
    pub title: String,
    pub icon: Option<String>,
    pub collapsible: bool,
    pub initially_collapsed: bool,
}

impl CardBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            icon: None,
            collapsible: true,
            initially_collapsed: false,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Spawns the card frame and returns `(card_root, header_actions_container, card_body)`.
    pub fn spawn_shell(self, commands: &mut Commands, parent: Entity) -> (Entity, Entity, Entity) {
        let card_root = commands
            .spawn((
                EditorInternal,
                CardRoot,
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_shrink: 0.0,
                    min_width: px(0),
                    padding: UiRect { left: px(2), right: px(2), top: SPACE_STANDARD, bottom: SPACE_COMPACT },
                    row_gap: px(10),
                    border: UiRect::top(px(1)),
                    border_radius: BorderRadius::ZERO,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                BorderColor::all(BORDER_SUBTLE),
                ChildOf(parent),
            ))
            .id();

        // Header bar
        let header = commands
            .spawn((
                EditorInternal,
                CardHeader,
                Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(card_root),
            ))
            .id();

        // Left: Icon + Title
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

        let title_text = if let Some(icon) = &self.icon {
            format!("{icon}  {}", self.title)
        } else {
            self.title.clone()
        };

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(title_text),
            TextFont {
                font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
                weight: FontWeight::SEMIBOLD,
                ..default()
            },
            TextColor(TEXT_PRIMARY),
            ChildOf(header_left),
        ));

        // Right: Custom actions container
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

        // Card Body
        let body = commands
            .spawn((
                EditorInternal,
                CardBody,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    display: if self.initially_collapsed { Display::None } else { Display::Flex },
                    ..default()
                },
                ChildOf(card_root),
            ))
            .id();

        // Collapse button if enabled
        if self.collapsible {
            let arrow_entity = commands
                .spawn((
                    EditorInternal,
                    CardCollapseArrow,
                    Pickable::IGNORE,
                    Text::new(if self.initially_collapsed { "›" } else { "⌄" }),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL),
                        weight: FontWeight::BOLD,
                        ..default()
                    },
                    TextColor(TEXT_SECONDARY),
                ))
                .id();

            let collapse_btn = commands
                .spawn((
                    EditorInternal,
                    Node {
                        padding: UiRect::axes(px(6), px(2)),
                        border_radius: radius_button(),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ChildOf(header_right),
                ))
                .add_child(arrow_entity)
                .id();

            commands.entity(collapse_btn)
                .observe(|trigger: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
                    if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                        *bg = BackgroundColor(BG_HOVER);
                    }
                })
                .observe(|trigger: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
                    if let Ok(mut bg) = bg.get_mut(trigger.entity) {
                        *bg = BackgroundColor(Color::NONE);
                    }
                })
                .observe(move |click: On<PointerClick>, mut body_query: Query<&mut Node>, mut arrow_query: Query<&mut Text>| {
                    if click.button == PointerButton::Primary {
                        if let Ok(mut body_node) = body_query.get_mut(body) {
                            let was_open = body_node.display == Display::Flex;
                            crate::ui::set_display(&mut body_node, if was_open { Display::None } else { Display::Flex });
                            if let Ok(mut arrow_text) = arrow_query.get_mut(arrow_entity) {
                                arrow_text.0 = if was_open { "›".to_string() } else { "⌄".to_string() };
                            }
                        }
                    }
                });
        }

        (card_root, header_right, body)
    }
}

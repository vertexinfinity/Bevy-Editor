//! Inspector card for DirectionalLight and PointLight illumination modes and settings.

use bevy::{
    ecs::message::{Message, MessageReader, MessageWriter},
    inspector::InspectorSelection,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::scene::components::LightBakeMode;
use crate::scene::EditorInternal;
use crate::ui::panel::CardBuilder;
use crate::ui::theme::*;

#[derive(Component)]
pub struct LightCardRoot;

#[derive(Component)]
pub struct LightModeLabel;

#[derive(Component)]
pub struct LightInfoLabel;

#[derive(Message, Clone, Debug)]
pub enum LightModeAction {
    SetMode(LightBakeMode),
    ToggleShadows,
}

pub(crate) fn spawn_light_card(commands: &mut Commands, parent: Entity) {
    let (card, _actions, body) = CardBuilder::new("Light & Shadows")
        .icon("💡")
        .collapsible(true)
        .spawn_shell(commands, parent);

    commands.entity(card).insert((
        LightCardRoot,
        Node {
            display: Display::None, // Hidden unless a light is selected
            ..default()
        },
    ));

    // 1. Illumination Mode Header Row
    let mode_row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(px(4)),
                ..default()
            },
            ChildOf(body),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Bake Mode:"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(mode_row),
    ));

    commands.spawn((
        EditorInternal,
        LightModeLabel,
        Pickable::IGNORE,
        Text::new("Mixed (Baked GI + Dynamic Spec)"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            weight: FontWeight::SEMIBOLD,
            ..default()
        },
        TextColor(ACCENT_CYAN),
        ChildOf(mode_row),
    ));

    // 2. 3-Way Mode Switch Buttons
    let btn_row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                margin: UiRect::bottom(px(6)),
                ..default()
            },
            ChildOf(body),
        ))
        .id();

    spawn_light_mode_btn(commands, btn_row, "Realtime", LightModeAction::SetMode(LightBakeMode::Realtime));
    spawn_light_mode_btn(commands, btn_row, "Baked", LightModeAction::SetMode(LightBakeMode::Baked));
    spawn_light_mode_btn(commands, btn_row, "Mixed", LightModeAction::SetMode(LightBakeMode::Mixed));
    spawn_light_mode_btn(commands, btn_row, "🌗 Shadows", LightModeAction::ToggleShadows);

    // 3. Information Summary
    commands.spawn((
        EditorInternal,
        LightInfoLabel,
        Pickable::IGNORE,
        Text::new("Affects GI: Yes | Specular: Realtime | Shadows: Active"),
        TextFont {
            font_size: bevy::text::FontSize::Px(9.5),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(body),
    ));
}

fn spawn_light_mode_btn(commands: &mut Commands, parent: Entity, label: &str, action: LightModeAction) {
    let btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(7), px(3)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
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
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<LightModeAction>| {
            if click.button == PointerButton::Primary {
                writer.write(action.clone());
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(9.5),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(btn),
    ));
}

/// Updates the light card visibility and labels based on the selected entity.
pub fn update_light_card(
    selection: Res<InspectorSelection>,
    dir_lights: Query<(&DirectionalLight, Option<&LightBakeMode>)>,
    point_lights: Query<(&PointLight, Option<&LightBakeMode>)>,
    mut card_query: Query<&mut Node, With<LightCardRoot>>,
    mut mode_query: Query<&mut Text, (With<LightModeLabel>, Without<LightInfoLabel>)>,
    mut info_query: Query<&mut Text, (With<LightInfoLabel>, Without<LightModeLabel>)>,
) {
    let Some(selected) = selection.0 else {
        for mut node in &mut card_query {
            node.display = Display::None;
        }
        return;
    };

    let (is_light, mode, shadows_on) = if let Ok((dir, bake_mode)) = dir_lights.get(selected) {
        (true, bake_mode.copied().unwrap_or(LightBakeMode::Mixed), dir.shadow_maps_enabled)
    } else if let Ok((point, bake_mode)) = point_lights.get(selected) {
        (true, bake_mode.copied().unwrap_or(LightBakeMode::Mixed), point.shadow_maps_enabled)
    } else {
        (false, LightBakeMode::Mixed, false)
    };

    for mut node in &mut card_query {
        node.display = if is_light { Display::Flex } else { Display::None };
    }

    if is_light {
        for mut text in &mut mode_query {
            text.0 = mode.label().to_string();
        }
        for mut text in &mut info_query {
            let shadow_str = if shadows_on { "Active" } else { "Disabled" };
            text.0 = match mode {
                LightBakeMode::Realtime => format!("Bake: Skipped | Dynamic: 100% | Shadows: {shadow_str}"),
                LightBakeMode::Baked => format!("Bake: Static GI | Dynamic: Off on Baked | Shadows: {shadow_str}"),
                LightBakeMode::Mixed => format!("Bake: Indirect/AO | Dynamic: Specular | Shadows: {shadow_str}"),
            };
        }
    }
}

/// Handles light mode button clicks from the inspector card.
pub fn handle_light_mode_actions(
    mut commands: Commands,
    mut actions: MessageReader<LightModeAction>,
    selection: Res<InspectorSelection>,
    mut dir_lights: Query<&mut DirectionalLight>,
    mut point_lights: Query<&mut PointLight>,
    _existing_modes: Query<Option<&LightBakeMode>>,
) {
    let Some(selected) = selection.0 else { return };

    for action in actions.read() {
        match action {
            LightModeAction::SetMode(new_mode) => {
                commands.entity(selected).insert(*new_mode);
                info!("Set light bake mode on {:?} to {:?}", selected, new_mode);
            }
            LightModeAction::ToggleShadows => {
                if let Ok(mut dir) = dir_lights.get_mut(selected) {
                    dir.shadow_maps_enabled = !dir.shadow_maps_enabled;
                    info!("Toggled DirectionalLight shadow maps: {}", dir.shadow_maps_enabled);
                }
                if let Ok(mut point) = point_lights.get_mut(selected) {
                    point.shadow_maps_enabled = !point.shadow_maps_enabled;
                    info!("Toggled PointLight shadow maps: {}", point.shadow_maps_enabled);
                }
            }
        }
    }
}

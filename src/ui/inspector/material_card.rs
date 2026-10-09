//! PBR StandardMaterial Card, quick color swatches, roughness/metallic tuners, and asset mutation systems.

use bevy::{
    ecs::message::{MessageReader, MessageWriter},
    inspector::InspectorSelection,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::scene::EditorInternal;
use crate::ui::panel::CardBuilder;
use crate::ui::theme::*;
use super::types::*;

pub(crate) fn spawn_material_card(commands: &mut Commands, parent: Entity) {
    let (card, actions, body) = CardBuilder::new("PBR Material")
        .icon("🎨")
        .collapsible(true)
        .spawn_shell(commands, parent);

    commands.entity(card).insert(MaterialCardRoot);

    // Color Swatch Indicator in Action Bar
    commands.spawn((
        EditorInternal,
        MaterialColorSwatch,
        Pickable::IGNORE,
        Node {
            width: px(16),
            height: px(16),
            border_radius: BorderRadius::all(px(3)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.7, 0.9)),
        BorderColor::all(BORDER_SUBTLE),
        ChildOf(actions),
    ));

    // 1. Color Palette Presets Row
    let palette_row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
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
        Text::new("Color:"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(palette_row),
    ));

    spawn_mat_color_btn(commands, palette_row, Color::srgb(0.9, 0.25, 0.25), "Red");
    spawn_mat_color_btn(commands, palette_row, Color::srgb(0.2, 0.8, 0.35), "Green");
    spawn_mat_color_btn(commands, palette_row, Color::srgb(0.2, 0.65, 0.95), "Blue");
    spawn_mat_color_btn(commands, palette_row, Color::srgb(0.95, 0.8, 0.15), "Gold");
    spawn_mat_color_btn(commands, palette_row, Color::srgb(0.75, 0.25, 0.85), "Purple");
    spawn_mat_color_btn(commands, palette_row, Color::srgb(0.92, 0.92, 0.92), "White");
    spawn_mat_color_btn(commands, palette_row, Color::srgb(0.12, 0.14, 0.18), "Dark");
    spawn_mat_color_btn(commands, palette_row, Color::srgb(0.7, 0.72, 0.78), "Steel");

    // 2. Roughness Tuning Row
    let roughness_row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
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
        Text::new("Rough:"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(roughness_row),
    ));

    spawn_mat_btn(commands, roughness_row, "-0.1", MaterialNudgeAction::AdjustRoughness(-0.1));
    spawn_mat_btn(commands, roughness_row, "+0.1", MaterialNudgeAction::AdjustRoughness(0.1));
    spawn_mat_btn(commands, roughness_row, "Gloss 0.1", MaterialNudgeAction::SetRoughness(0.1));
    spawn_mat_btn(commands, roughness_row, "Satin 0.5", MaterialNudgeAction::SetRoughness(0.5));
    spawn_mat_btn(commands, roughness_row, "Matte 0.9", MaterialNudgeAction::SetRoughness(0.9));

    // 3. Metallic Tuning Row
    let metallic_row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
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
        Text::new("Metal:"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(metallic_row),
    ));

    spawn_mat_btn(commands, metallic_row, "-0.1", MaterialNudgeAction::AdjustMetallic(-0.1));
    spawn_mat_btn(commands, metallic_row, "+0.1", MaterialNudgeAction::AdjustMetallic(0.1));
    spawn_mat_btn(commands, metallic_row, "Dielectric 0.0", MaterialNudgeAction::SetMetallic(0.0));
    spawn_mat_btn(commands, metallic_row, "Semi 0.5", MaterialNudgeAction::SetMetallic(0.5));
    spawn_mat_btn(commands, metallic_row, "Metal 1.0", MaterialNudgeAction::SetMetallic(1.0));

    // 4. Effects Row (Reflectance, Emissive, Unlit)
    let fx_row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
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
        Text::new("FX:"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(fx_row),
    ));

    spawn_mat_btn(commands, fx_row, "Refl -", MaterialNudgeAction::AdjustReflectance(-0.1));
    spawn_mat_btn(commands, fx_row, "Refl +", MaterialNudgeAction::AdjustReflectance(0.1));
    spawn_mat_btn(commands, fx_row, "💡 Glow", MaterialNudgeAction::ToggleEmissive);
    spawn_mat_btn(commands, fx_row, "☀️ Unlit", MaterialNudgeAction::ToggleUnlit);

    // 5. Information Summary Label
    commands.spawn((
        EditorInternal,
        MaterialInfoLabel,
        Pickable::IGNORE,
        Text::new("Roughness: 0.50 | Metallic: 0.00"),
        TextFont {
            font_size: bevy::text::FontSize::Px(10.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(body),
    ));
}

fn spawn_mat_color_btn(commands: &mut Commands, parent: Entity, color: Color, label: &'static str) {
    let btn = commands
        .spawn((
            EditorInternal,
            Node {
                width: px(18),
                height: px(18),
                border_radius: BorderRadius::all(px(3)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(color),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .observe(|trigger: On<PointerOver>, mut border: Query<&mut BorderColor>| {
            if let Ok(mut b) = border.get_mut(trigger.entity) {
                *b = BorderColor::all(ACCENT_CYAN);
            }
        })
        .observe(|trigger: On<PointerOut>, mut border: Query<&mut BorderColor>| {
            if let Ok(mut b) = border.get_mut(trigger.entity) {
                *b = BorderColor::all(BORDER_SUBTLE);
            }
        })
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<MaterialNudgeAction>| {
            if click.button == PointerButton::Primary {
                writer.write(MaterialNudgeAction::SetBaseColor(color));
            }
        })
        .id();

    let _ = (btn, label);
}

fn spawn_mat_btn(commands: &mut Commands, parent: Entity, label: &str, action: MaterialNudgeAction) {
    let btn = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(6), px(2)),
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
        .observe(move |click: On<PointerClick>, mut writer: MessageWriter<MaterialNudgeAction>| {
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

/// Handles material tweaking actions received from the Material card.
pub fn handle_material_nudges(
    mut actions: MessageReader<MaterialNudgeAction>,
    selection: Res<InspectorSelection>,
    materials_query: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(selected) = selection.0 else { return };
    let Ok(mat_handle) = materials_query.get(selected) else { return };
    let Some(mut mat) = materials.get_mut(&mat_handle.0) else { return };

    for action in actions.read() {
        match action {
            MaterialNudgeAction::SetBaseColor(color) => {
                mat.base_color = *color;
                info!("Updated material base color to {:?}", color);
            }
            MaterialNudgeAction::AdjustRoughness(delta) => {
                mat.perceptual_roughness = (mat.perceptual_roughness + delta).clamp(0.0, 1.0);
                info!("Updated material roughness to {:.2}", mat.perceptual_roughness);
            }
            MaterialNudgeAction::SetRoughness(val) => {
                mat.perceptual_roughness = (*val).clamp(0.0, 1.0);
                info!("Set material roughness to {:.2}", mat.perceptual_roughness);
            }
            MaterialNudgeAction::AdjustMetallic(delta) => {
                mat.metallic = (mat.metallic + delta).clamp(0.0, 1.0);
                info!("Updated material metallic to {:.2}", mat.metallic);
            }
            MaterialNudgeAction::SetMetallic(val) => {
                mat.metallic = (*val).clamp(0.0, 1.0);
                info!("Set material metallic to {:.2}", mat.metallic);
            }
            MaterialNudgeAction::AdjustReflectance(delta) => {
                mat.reflectance = (mat.reflectance + delta).clamp(0.0, 1.0);
                info!("Updated material reflectance to {:.2}", mat.reflectance);
            }
            MaterialNudgeAction::ToggleEmissive => {
                let is_glowing = mat.emissive.to_f32_array()[0] > 0.1;
                mat.emissive = if is_glowing {
                    LinearRgba::BLACK
                } else {
                    LinearRgba::rgb(2.0, 1.5, 0.5)
                };
                info!("Toggled material emissive glow: {}", !is_glowing);
            }
            MaterialNudgeAction::ToggleUnlit => {
                mat.unlit = !mat.unlit;
                info!("Toggled material unlit: {}", mat.unlit);
            }
        }
    }
}

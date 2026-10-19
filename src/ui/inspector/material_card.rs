//! Compact typed PBR inspector with explicit shared material ownership.
use super::{
    property_editor::{PropertyPath, spawn_property_field, spawn_property_help},
    types::*,
};
use crate::ui::{panel::CardBuilder, theme::*};
use crate::{
    core::commands::{CommandHistory, EditorCommand},
    scene::{EditorInternal, SceneEntity},
};
use bevy::{
    ecs::message::MessageReader,
    inspector::InspectorSelection,
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
};

#[derive(Component)]
pub(crate) struct MaterialSharingLabel;

pub(crate) fn spawn_material_card(commands: &mut Commands, parent: Entity) {
    let (card, actions, body) = CardBuilder::new("Material").spawn_shell(commands, parent);
    commands.entity(card).insert(MaterialCardRoot);
    commands.spawn((
        EditorInternal,
        MaterialColorSwatch,
        Pickable::IGNORE,
        Node {
            width: px(18),
            height: px(18),
            border_radius: radius_button(),
            ..default()
        },
        BackgroundColor(Color::WHITE),
        ChildOf(actions),
    ));
    commands.spawn((
        EditorInternal,
        MaterialSharingLabel,
        Pickable::IGNORE,
        Text::new(""),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(body),
    ));
    let unique = commands
        .spawn((
            EditorInternal,
            Node {
                padding: UiRect::axes(px(8), px(4)),
                align_self: AlignSelf::Start,
                border_radius: radius_button(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(body),
        ))
        .observe(|click: On<PointerClick>, mut commands: Commands| {
            if click.button == PointerButton::Primary {
                commands.queue(make_unique);
            }
        })
        .id();
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Make unique"),
        TextFont {
            font_size: bevy::text::FontSize::Px(12.0),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(unique),
    ));
    for (label, path) in [
        ("Base color", PropertyPath::BaseColor),
        ("Roughness", PropertyPath::Roughness),
        ("Metallic", PropertyPath::Metallic),
        ("Reflectance", PropertyPath::Reflectance),
        ("Unlit", PropertyPath::Unlit),
    ] {
        spawn_property_field(commands, body, label, path);
    }
    spawn_property_help(commands, body);
    // Kept for the existing dirty-driven selection synchronizer.
    commands.spawn((
        EditorInternal,
        MaterialInfoLabel,
        Pickable::IGNORE,
        Node {
            display: Display::None,
            ..default()
        },
        Text::new(""),
        ChildOf(body),
    ));
}

struct MakeUniqueMaterial {
    entity: Entity,
    before: Handle<StandardMaterial>,
    after: Handle<StandardMaterial>,
}
impl EditorCommand for MakeUniqueMaterial {
    fn name(&self) -> &str {
        "Make material unique"
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        world
            .get_entity_mut(self.entity)
            .map_err(|_| "Entity no longer exists")?
            .insert(MeshMaterial3d(self.after.clone()));
        Ok(())
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        world
            .get_entity_mut(self.entity)
            .map_err(|_| "Entity no longer exists")?
            .insert(MeshMaterial3d(self.before.clone()));
        Ok(())
    }
}
fn make_unique(world: &mut World) {
    if *world.resource::<crate::core::play_mode::PlayState>()
        != crate::core::play_mode::PlayState::Edit
    {
        return;
    }
    let Some(entity) = world.resource::<InspectorSelection>().0 else {
        return;
    };
    let Some(before) = world
        .get::<MeshMaterial3d<StandardMaterial>>(entity)
        .map(|m| m.0.clone())
    else {
        return;
    };
    let Some(material) = world
        .resource::<Assets<StandardMaterial>>()
        .get(&before)
        .cloned()
    else {
        return;
    };
    let after = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(material);
    world.resource_scope(|world, mut history: Mut<CommandHistory>| {
        if let Err(error) = history.execute(
            Box::new(MakeUniqueMaterial {
                entity,
                before,
                after,
            }),
            world,
        ) {
            warn!("{error}");
        }
    });
}
pub(crate) fn update_material_sharing(
    selection: Res<InspectorSelection>,
    handles: Query<&MeshMaterial3d<StandardMaterial>, With<SceneEntity>>,
    materials: Res<Assets<StandardMaterial>>,
    mut labels: Query<&mut Text, With<MaterialSharingLabel>>,
) {
    let label = selection
        .0
        .and_then(|e| handles.get(e).ok())
        .map(|selected| {
            let count = handles.iter().filter(|h| h.0 == selected.0).count();
            let texture = materials
                .get(&selected.0)
                .and_then(|m| m.base_color_texture.as_ref())
                .map(|_| "Base texture assigned")
                .unwrap_or("Base texture: none (optional)");
            format!(
                "Used by {count} {} · edits affect this asset\n{texture}",
                if count == 1 { "entity" } else { "entities" }
            )
        })
        .unwrap_or_default();
    for mut text in &mut labels {
        if text.0 != label {
            crate::ui::set_text(&mut text, label.clone());
        }
    }
}

/// Handles material tweaking actions received from the Material card.
pub fn handle_material_nudges(
    mut actions: MessageReader<MaterialNudgeAction>,
    selection: Res<InspectorSelection>,
    materials_query: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if actions.is_empty() {
        return;
    }
    let Some(selected) = selection.0 else { return };
    let Ok(mat_handle) = materials_query.get(selected) else {
        return;
    };
    let Some(mut mat) = materials.get_mut(&mat_handle.0) else {
        return;
    };

    for action in actions.read() {
        match action {
            MaterialNudgeAction::SetBaseColor(color) => {
                mat.base_color = *color;
                info!("Updated material base color to {:?}", color);
            }
            MaterialNudgeAction::AdjustRoughness(delta) => {
                mat.perceptual_roughness = (mat.perceptual_roughness + delta).clamp(0.0, 1.0);
                info!(
                    "Updated material roughness to {:.2}",
                    mat.perceptual_roughness
                );
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

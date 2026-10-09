//! Transform Card (Position, Rotation, Scale), interactive axis widgets, direct keyboard text typing, and transform update systems.

use bevy::{
    ecs::message::{MessageReader, MessageWriter},
    inspector::InspectorSelection,
    math::EulerRot,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};
use crate::scene::{components::*, EditorInternal};
use crate::ui::theme::*;
use super::types::*;

pub(crate) fn spawn_transform_value_row(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    property: TransformProperty,
) {
    let row = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(row),
    ));

    let axes = commands
        .spawn((
            EditorInternal,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(row),
        ))
        .id();

    for (axis, label) in [(TransformAxis::X, "X"), (TransformAxis::Y, "Y"), (TransformAxis::Z, "Z")] {
        let axis_cell = commands
            .spawn((
                EditorInternal,
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(3),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(axes),
            ))
            .id();

        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(label),
            TextFont {
                font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
                ..default()
            },
            TextColor(TEXT_MUTED),
            ChildOf(axis_cell),
        ));

        spawn_transform_axis_widget(commands, axis_cell, property, axis);
    }
}

fn spawn_transform_axis_widget(
    commands: &mut Commands,
    parent: Entity,
    property: TransformProperty,
    axis: TransformAxis,
) {
    let step = match property {
        TransformProperty::Position | TransformProperty::Scale => 0.1,
        TransformProperty::Rotation => 1.0,
    };

    let widget = commands
        .spawn((
            EditorInternal,
            Node {
                position_type: PositionType::Relative,
                width: px(78),
                height: px(24),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(parent),
        ))
        .id();

    let value_field = commands
        .spawn((
            EditorInternal,
            Node {
                width: px(50),
                height: px(22),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(widget),
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
        .observe(move |click: On<PointerClick>, mut text_edit: ResMut<TransformTextEditState>| {
            if click.button == PointerButton::Primary {
                text_edit.active = Some((property, axis));
                text_edit.buffer.clear();
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        TransformAxisValue { property, axis },
        Pickable::IGNORE,
        Text::new("0.00"),
        TextFont {
            font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(value_field),
    ));

    let left_arrow = spawn_transform_arrow(commands, widget, "‹", property, axis, -step, true);
    let right_arrow = spawn_transform_arrow(commands, widget, "›", property, axis, step, false);

    commands.entity(widget)
        .observe(move |_over: On<PointerOver>, mut nodes: Query<&mut Node>| {
            for arrow in [left_arrow, right_arrow] {
                if let Ok(mut node) = nodes.get_mut(arrow) {
                    node.display = Display::Flex;
                }
            }
        })
        .observe(move |_out: On<PointerOut>, mut nodes: Query<&mut Node>| {
            for arrow in [left_arrow, right_arrow] {
                if let Ok(mut node) = nodes.get_mut(arrow) {
                    node.display = Display::None;
                }
            }
        })
        .observe(move |drag: On<bevy::picking::events::PointerDrag>, mut writer: MessageWriter<TransformNudgeAction>| {
            if drag.button == PointerButton::Primary && drag.delta.x != 0.0 {
                writer.write(TransformNudgeAction::AdjustAxis {
                    property,
                    axis,
                    delta: drag.delta.x * step * 0.1,
                });
            }
        });
}

fn spawn_transform_arrow(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    property: TransformProperty,
    axis: TransformAxis,
    delta: f32,
    is_left: bool,
) -> Entity {
    let mut node = Node {
        display: Display::None,
        position_type: PositionType::Absolute,
        top: px(1),
        width: px(16),
        height: px(22),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    if is_left {
        node.left = px(0);
    } else {
        node.right = px(0);
    }

    let arrow = commands
        .spawn((
            EditorInternal,
            node,
            BackgroundColor(BG_HOVER),
            ChildOf(parent),
        ))
        .observe(move |mut click: On<PointerClick>, mut writer: MessageWriter<TransformNudgeAction>| {
            if click.button == PointerButton::Primary {
                click.propagate(false);
                writer.write(TransformNudgeAction::AdjustAxis { property, axis, delta });
            }
        })
        .id();

    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(arrow),
    ));

    arrow
}

pub fn handle_transform_nudges(
    mut nudges: MessageReader<TransformNudgeAction>,
    selection: Res<InspectorSelection>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(selected) = selection.0 else {
        return;
    };

    let Ok(mut transform) = transforms.get_mut(selected) else {
        return;
    };

    for nudge in nudges.read() {
        match *nudge {
            TransformNudgeAction::AdjustAxis { property, axis, delta } => match property {
                TransformProperty::Position => match axis {
                    TransformAxis::X => transform.translation.x += delta,
                    TransformAxis::Y => transform.translation.y += delta,
                    TransformAxis::Z => transform.translation.z += delta,
                },
                TransformProperty::Rotation => match axis {
                    TransformAxis::X => transform.rotate_local_x(delta.to_radians()),
                    TransformAxis::Y => transform.rotate_local_y(delta.to_radians()),
                    TransformAxis::Z => transform.rotate_local_z(delta.to_radians()),
                },
                TransformProperty::Scale => match axis {
                    TransformAxis::X => transform.scale.x += delta,
                    TransformAxis::Y => transform.scale.y += delta,
                    TransformAxis::Z => transform.scale.z += delta,
                },
            },
            TransformNudgeAction::TranslateX(dx) => transform.translation.x += dx,
            TransformNudgeAction::TranslateY(dy) => transform.translation.y += dy,
            TransformNudgeAction::TranslateZ(dz) => transform.translation.z += dz,
            TransformNudgeAction::RotateX(deg) => transform.rotate_local_x(deg.to_radians()),
            TransformNudgeAction::RotateY(deg) => transform.rotate_local_y(deg.to_radians()),
            TransformNudgeAction::RotateZ(deg) => transform.rotate_local_z(deg.to_radians()),
            TransformNudgeAction::ScaleUniform(s) => transform.scale *= s,
            TransformNudgeAction::ResetPosition => transform.translation = Vec3::ZERO,
            TransformNudgeAction::ResetRotation => transform.rotation = Quat::IDENTITY,
            TransformNudgeAction::ResetScale => transform.scale = Vec3::ONE,
        }
    }
}

/// Handles typing numeric values directly into transform fields.
pub fn handle_transform_text_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut char_events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut text_edit: ResMut<TransformTextEditState>,
    selection: Res<InspectorSelection>,
    mut transforms: Query<&mut Transform>,
) {
    if text_edit.active.is_none() {
        return;
    }

    // Cancel on escape
    if keyboard_input.just_pressed(KeyCode::Escape) {
        text_edit.active = None;
        text_edit.buffer.clear();
        return;
    }

    // Process character typing
    for ev in char_events.read() {
        if ev.state.is_pressed() {
            if let bevy::input::keyboard::Key::Character(ref ch) = ev.logical_key {
                for c in ch.chars() {
                    if c.is_ascii_digit() || c == '.' || c == '-' {
                        text_edit.buffer.push(c);
                    }
                }
            }
        }
    }

    if keyboard_input.just_pressed(KeyCode::Backspace) {
        text_edit.buffer.pop();
    }

    // Commit on Enter
    if keyboard_input.just_pressed(KeyCode::Enter) {
        if let Some((prop, axis)) = text_edit.active {
            if let Ok(parsed_val) = text_edit.buffer.trim().parse::<f32>() {
                if let Some(selected) = selection.0 {
                    if let Ok(mut transform) = transforms.get_mut(selected) {
                        match prop {
                            TransformProperty::Position => match axis {
                                TransformAxis::X => transform.translation.x = parsed_val,
                                TransformAxis::Y => transform.translation.y = parsed_val,
                                TransformAxis::Z => transform.translation.z = parsed_val,
                            },
                            TransformProperty::Rotation => match axis {
                                TransformAxis::X => {
                                    let (_, y, z) = transform.rotation.to_euler(EulerRot::XYZ);
                                    transform.rotation = Quat::from_euler(EulerRot::XYZ, parsed_val.to_radians(), y, z);
                                }
                                TransformAxis::Y => {
                                    let (x, _, z) = transform.rotation.to_euler(EulerRot::XYZ);
                                    transform.rotation = Quat::from_euler(EulerRot::XYZ, x, parsed_val.to_radians(), z);
                                }
                                TransformAxis::Z => {
                                    let (x, y, _) = transform.rotation.to_euler(EulerRot::XYZ);
                                    transform.rotation = Quat::from_euler(EulerRot::XYZ, x, y, parsed_val.to_radians());
                                }
                            },
                            TransformProperty::Scale => match axis {
                                TransformAxis::X => transform.scale.x = parsed_val,
                                TransformAxis::Y => transform.scale.y = parsed_val,
                                TransformAxis::Z => transform.scale.z = parsed_val,
                            },
                        }
                        info!("Manually set {:?} {:?} to {}", prop, axis, parsed_val);
                    }
                }
            }
        }
        text_edit.active = None;
        text_edit.buffer.clear();
    }
}

fn set_transform_axis_values(
    axis_labels: &mut Query<(&TransformAxisValue, &mut Text), With<TransformAxisValue>>,
    position: Vec3,
    rotation: Vec3,
    scale: Vec3,
    text_edit: &TransformTextEditState,
) {
    for (axis_value, mut text) in axis_labels.iter_mut() {
        if text_edit.active == Some((axis_value.property, axis_value.axis)) {
            text.0 = format!("{}_", text_edit.buffer);
            continue;
        }

        let vector = match axis_value.property {
            TransformProperty::Position => position,
            TransformProperty::Rotation => rotation,
            TransformProperty::Scale => scale,
        };
        let value = match axis_value.axis {
            TransformAxis::X => vector.x,
            TransformAxis::Y => vector.y,
            TransformAxis::Z => vector.z,
        };
        text.0 = if axis_value.property == TransformProperty::Rotation {
            format!("{value:.1}°")
        } else {
            format!("{value:.2}")
        };
    }
}

pub fn update_clean_transform_labels(
    selection: Res<InspectorSelection>,
    state: Res<InspectorState>,
    text_edit: Res<TransformTextEditState>,
    entities: Query<(
        Option<&Name>,
        Option<&Transform>,
        Option<&GlobalTransform>,
        Option<&ScriptComponent>,
        Option<&SpinAnimation>,
        Option<&Velocity>,
        Option<&GameplayTag>,
        Option<&Mesh3d>,
        Option<&DirectionalLight>,
        Option<&PointLight>,
        Option<&Visibility>,
        Option<&MeshMaterial3d<StandardMaterial>>,
    )>,
    materials: Res<Assets<StandardMaterial>>,
    mut name_label: Query<&mut Text, (With<SelectedEntityLabel>, Without<TransformAxisValue>, Without<ComponentsBadgesLabel>, Without<ScriptPropsLabel>, Without<MaterialInfoLabel>)>,
    mut axis_labels: Query<(&TransformAxisValue, &mut Text), With<TransformAxisValue>>,
    mut badges_label: Query<&mut Text, (With<ComponentsBadgesLabel>, Without<SelectedEntityLabel>, Without<TransformAxisValue>, Without<ScriptPropsLabel>, Without<MaterialInfoLabel>)>,
    mut script_label: Query<&mut Text, (With<ScriptPropsLabel>, Without<SelectedEntityLabel>, Without<TransformAxisValue>, Without<ComponentsBadgesLabel>, Without<MaterialInfoLabel>)>,
    mut script_card_query: Query<&mut Node, (With<ScriptCardRoot>, Without<CleanViewRoot>, Without<DebugViewRoot>, Without<MaterialCardRoot>)>,
    mut mat_card_query: Query<&mut Node, (With<MaterialCardRoot>, Without<CleanViewRoot>, Without<DebugViewRoot>, Without<ScriptCardRoot>)>,
    mut mat_info_label: Query<&mut Text, (With<MaterialInfoLabel>, Without<SelectedEntityLabel>, Without<TransformAxisValue>, Without<ComponentsBadgesLabel>, Without<ScriptPropsLabel>)>,
    mut mat_swatch: Query<&mut BackgroundColor, With<MaterialColorSwatch>>,
) {
    let Ok(mut name_txt) = name_label.single_mut() else { return };
    let Ok(mut badges_txt) = badges_label.single_mut() else { return };
    let Ok(mut script_txt) = script_label.single_mut() else { return };

    let Some(selected) = selection.0 else {
        name_txt.0 = "Selected: None (Click object in scene)".to_string();
        set_transform_axis_values(&mut axis_labels, Vec3::ZERO, Vec3::ZERO, Vec3::ONE, &text_edit);
        badges_txt.0 = "None".to_string();
        script_txt.0 = "No entity selected".to_string();
        if let Ok(mut mat_card) = mat_card_query.single_mut() {
            mat_card.display = Display::None;
        }
        return;
    };

    let Ok((name, transform, global_transform, script, spin, vel, tag, mesh, dir_light, pt_light, vis, mat_handle)) = entities.get(selected) else {
        name_txt.0 = format!("Selected: {:?} (Despawned)", selected);
        set_transform_axis_values(&mut axis_labels, Vec3::ZERO, Vec3::ZERO, Vec3::ONE, &text_edit);
        script_txt.0 = "Despawned".to_string();
        if let Ok(mut mat_card) = mat_card_query.single_mut() {
            mat_card.display = Display::None;
        }
        return;
    };

    let is_hidden = vis == Some(&Visibility::Hidden);
    let type_desc = if mesh.is_some() {
        "Mesh Prop"
    } else if dir_light.is_some() || pt_light.is_some() {
        "Light"
    } else {
        "Entity"
    };

    let base_name = if let Some(n) = name {
        n.as_str().to_string()
    } else {
        format!("{type_desc} #{}", selected.index())
    };

    name_txt.0 = if is_hidden {
        format!("Selected: {base_name} [Hidden]")
    } else {
        format!("Selected: {base_name}")
    };

    // Compute display position, rotation, scale
    let (pos, rot, scale) = if state.is_global_coords {
        if let Some(gt) = global_transform {
            let t = gt.compute_transform();
            let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);
            (t.translation, Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()), t.scale)
        } else {
            (Vec3::ZERO, Vec3::ZERO, Vec3::ONE)
        }
    } else if let Some(t) = transform {
        let (rx, ry, rz) = t.rotation.to_euler(EulerRot::XYZ);
        (t.translation, Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()), t.scale)
    } else {
        (Vec3::ZERO, Vec3::ZERO, Vec3::ONE)
    };

    set_transform_axis_values(&mut axis_labels, pos, rot, scale, &text_edit);

    // Collect badges
    let mut badges = Vec::new();
    if transform.is_some() { badges.push("[Transform]"); }
    if mesh.is_some() { badges.push("[Mesh3d]"); }
    if mat_handle.is_some() { badges.push("[PBR Material]"); }
    if dir_light.is_some() { badges.push("[DirectionalLight]"); }
    if pt_light.is_some() { badges.push("[PointLight]"); }
    if script.is_some() { badges.push("[Script]"); }
    if spin.is_some() { badges.push("[SpinAnimation]"); }
    if vel.is_some() { badges.push("[Velocity]"); }
    if tag.is_some() { badges.push("[GameplayTag]"); }

    badges_txt.0 = if badges.is_empty() {
        "None".to_string()
    } else {
        badges.join(" ")
    };

    if let Ok(mut card_node) = script_card_query.single_mut() {
        card_node.display = if script.is_some() { Display::Flex } else { Display::None };
    }

    if let Some(script_comp) = script {
        let mut lines = Vec::new();
        lines.push(format!("File: {}", script_comp.script_file));
        for (k, v) in &script_comp.properties {
            match v {
                ScriptPropertyVal::Float(f) => lines.push(format!("• {}: {:.2}", k, f)),
                ScriptPropertyVal::Int(i) => lines.push(format!("• {}: {}", k, i)),
                ScriptPropertyVal::Bool(b) => lines.push(format!("• {}: {}", k, b)),
                ScriptPropertyVal::String(s) => lines.push(format!("• {}: \"{}\"", k, s)),
                ScriptPropertyVal::Vec3(v) => lines.push(format!("• {}: ({:.1}, {:.1}, {:.1})", k, v.x, v.y, v.z)),
            }
        }
        script_txt.0 = lines.join("\n");
    } else {
        script_txt.0 = String::new();
    }

    // Material Card update
    if let Ok(mut mat_card) = mat_card_query.single_mut() {
        if let Some(handle) = mat_handle {
            mat_card.display = Display::Flex;
            if let Some(mat) = materials.get(&handle.0) {
                if let Ok(mut swatch_bg) = mat_swatch.single_mut() {
                    *swatch_bg = BackgroundColor(mat.base_color);
                }
                if let Ok(mut info_text) = mat_info_label.single_mut() {
                    let glow_str = if mat.emissive.to_f32_array()[0] > 0.1 { "On" } else { "Off" };
                    info_text.0 = format!(
                        "Roughness: {:.2} | Metallic: {:.2} | Glow: {} | Unlit: {}",
                        mat.perceptual_roughness, mat.metallic, glow_str, mat.unlit
                    );
                }
            }
        } else {
            mat_card.display = Display::None;
        }
    }
}
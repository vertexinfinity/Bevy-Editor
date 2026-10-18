//! Shared typed fields for built-in and plugin inspectors. Edits use command history.
use crate::ui::theme::*;
use crate::{
    core::{
        commands::{CommandHistory, EditorCommand},
        play_mode::PlayState,
        reflect_fields,
    },
    scene::{
        EditorInternal,
        components::{ScriptComponent, ScriptPropertyVal},
    },
};
use bevy::{
    ecs::message::MessageCursor,
    input::keyboard::{Key, KeyboardInput},
    inspector::InspectorSelection,
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    window::PrimaryWindow,
};

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyPath {
    BaseColor,
    Roughness,
    Metallic,
    Reflectance,
    Unlit,
    Script(String, Option<usize>),
    /// Any reflected component field, addressed by component type path and reflect path.
    Reflect { type_path: String, field: String },
}
#[derive(Clone, Debug)]
enum Target {
    Material(Handle<StandardMaterial>),
    Script(Entity),
    Reflect(Entity),
}
#[derive(Component, Clone)]
pub struct PropertyField {
    pub path: PropertyPath,
    /// Fixed entity to edit; None follows the inspector selection.
    pub entity: Option<Entity>,
}
#[derive(Component)]
struct PropertyValue;
#[derive(Component)]
struct PropertyHelp;
#[derive(Resource, Default)]
pub struct PropertyEditState {
    active: Option<ActiveEdit>,
}
impl PropertyEditState {
    pub fn is_editing(&self) -> bool {
        self.active.is_some()
    }
    pub fn cancel(&mut self) {
        self.active = None;
    }
}
struct ActiveEdit {
    field: Entity,
    selected: Entity,
    target: Target,
    path: PropertyPath,
    buffer: String,
    replace: bool,
    error: Option<String>,
}

pub fn spawn_property_field(
    commands: &mut Commands,
    parent: Entity,
    label: &str,
    path: PropertyPath,
) -> Entity {
    let row = commands
        .spawn((
            EditorInternal,
            Node {
                align_items: AlignItems::Center,
                min_height: px(30),
                column_gap: px(8),
                min_width: px(0),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Node {
            width: px(128),
            flex_shrink: 0.0,
            ..default()
        },
        Text::new(label),
        TextFont {
            font_size: bevy::text::FontSize::Px(13.0),
            ..default()
        },
        TextColor(TEXT_SECONDARY),
        ChildOf(row),
    ));
    spawn_property_value(commands, row, path)
}

/// Spawns only the clickable value box, for callers that lay out their own label row.
pub fn spawn_property_value(commands: &mut Commands, row: Entity, path: PropertyPath) -> Entity {
    spawn_property_value_for(commands, row, path, None)
}

/// Like [spawn_property_value] but bound to a fixed entity (e.g. world settings).
pub fn spawn_property_value_for(commands: &mut Commands, row: Entity, path: PropertyPath, entity: Option<Entity>) -> Entity {
    let field = commands
        .spawn((
            EditorInternal,
            PropertyField { path, entity },
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                height: px(28),
                padding: UiRect::axes(px(8), px(3)),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: radius_button(),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(BG_INPUT),
            BorderColor::all(Color::NONE),
            ChildOf(row),
        ))
        .observe(|mut click: On<PointerClick>, mut commands: Commands| {
            if click.button == PointerButton::Primary {
                click.propagate(false);
                let field = click.entity;
                commands.queue(move |world: &mut World| begin_edit(world, field));
            }
        })
        .id();
    commands.spawn((
        EditorInternal,
        PropertyValue,
        Pickable::IGNORE,
        TextLayout::no_wrap(),
        Text::new("—"),
        TextFont {
            font_size: bevy::text::FontSize::Px(13.0),
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        ChildOf(field),
    ));
    field
}

pub fn spawn_property_help(commands: &mut Commands, parent: Entity) {
    commands.spawn((
        EditorInternal,
        PropertyHelp,
        Pickable::IGNORE,
        Text::new("Click a value to edit · Enter applies · Esc cancels"),
        TextFont {
            font_size: bevy::text::FontSize::Px(11.5),
            ..default()
        },
        TextColor(TEXT_MUTED),
        ChildOf(parent),
    ));
}

fn resolve(world: &World, entity: Entity, path: &PropertyPath) -> Option<Target> {
    match path {
        PropertyPath::Script(..) => world
            .get::<ScriptComponent>(entity)
            .map(|_| Target::Script(entity)),
        PropertyPath::Reflect { type_path, field } => {
            reflect_fields::read_field(world, entity, type_path, field).map(|_| Target::Reflect(entity))
        }
        _ => world
            .get::<MeshMaterial3d<StandardMaterial>>(entity)
            .map(|m| Target::Material(m.0.clone())),
    }
}
fn read_value(world: &World, target: &Target, path: &PropertyPath) -> Option<String> {
    match target {
        Target::Reflect(entity) => {
            let PropertyPath::Reflect { type_path, field } = path else {
                return None;
            };
            reflect_fields::read_field(world, *entity, type_path, field)
        }
        Target::Script(entity) => {
            let PropertyPath::Script(key, axis) = path else {
                return None;
            };
            Some(
                match world.get::<ScriptComponent>(*entity)?.properties.get(key)? {
                    ScriptPropertyVal::Float(v) => v.to_string(),
                    ScriptPropertyVal::Int(v) => v.to_string(),
                    ScriptPropertyVal::Bool(v) => v.to_string(),
                    ScriptPropertyVal::String(v) => v.clone(),
                    ScriptPropertyVal::Vec3(v) => v[(*axis)?].to_string(),
                },
            )
        }
        Target::Material(handle) => {
            let material = world.resource::<Assets<StandardMaterial>>().get(handle)?;
            Some(match path {
                PropertyPath::BaseColor => {
                    let c = material.base_color.to_srgba();
                    format!(
                        "#{:02X}{:02X}{:02X}{:02X}",
                        (c.red * 255.0).round() as u8,
                        (c.green * 255.0).round() as u8,
                        (c.blue * 255.0).round() as u8,
                        (c.alpha * 255.0).round() as u8
                    )
                }
                PropertyPath::Roughness => material.perceptual_roughness.to_string(),
                PropertyPath::Metallic => material.metallic.to_string(),
                PropertyPath::Reflectance => material.reflectance.to_string(),
                PropertyPath::Unlit => material.unlit.to_string(),
                _ => return None,
            })
        }
    }
}
fn finite(value: &str) -> Result<f32, String> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| "Enter a finite number".into())
}
fn apply_value(
    world: &mut World,
    target: &Target,
    path: &PropertyPath,
    value: &str,
) -> Result<(), String> {
    match target {
        Target::Reflect(entity) => {
            let PropertyPath::Reflect { type_path, field } = path else {
                return Err("Invalid component field".into());
            };
            return reflect_fields::write_field(world, *entity, type_path, field, value);
        }
        Target::Script(entity) => {
            let PropertyPath::Script(key, axis) = path else {
                return Err("Invalid script field".into());
            };
            let mut script = world
                .get_mut::<ScriptComponent>(*entity)
                .ok_or("Script no longer exists")?;
            let property = script
                .properties
                .get_mut(key)
                .ok_or("Property no longer exists")?;
            match property {
                ScriptPropertyVal::Float(v) => *v = finite(value)?,
                ScriptPropertyVal::Int(v) => {
                    *v = value.trim().parse().map_err(|_| "Enter a whole number")?
                }
                ScriptPropertyVal::Bool(v) => {
                    *v = value.parse().map_err(|_| "Enter true or false")?
                }
                ScriptPropertyVal::String(v) => *v = value.into(),
                ScriptPropertyVal::Vec3(v) => {
                    v[axis.ok_or("Missing vector axis")?] = finite(value)?
                }
            }
        }
        Target::Material(handle) => {
            let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
            let mut material = materials
                .get_mut(handle)
                .ok_or("Material no longer exists")?;
            match path {
                PropertyPath::BaseColor => {
                    let hex = value.trim().trim_start_matches('#');
                    if !matches!(hex.len(), 6 | 8) || !hex.is_ascii() {
                        return Err("Use #RRGGBB or #RRGGBBAA".into());
                    }
                    let mut c = [255u8; 4];
                    for (i, channel) in c.iter_mut().enumerate().take(hex.len() / 2) {
                        *channel = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                            .map_err(|_| "Invalid hexadecimal color")?;
                    }
                    material.base_color = Color::srgba_u8(c[0], c[1], c[2], c[3]);
                }
                PropertyPath::Roughness => {
                    material.perceptual_roughness = finite(value)?.clamp(0.0, 1.0)
                }
                PropertyPath::Metallic => material.metallic = finite(value)?.clamp(0.0, 1.0),
                PropertyPath::Reflectance => material.reflectance = finite(value)?.clamp(0.0, 1.0),
                PropertyPath::Unlit => {
                    material.unlit = value.parse().map_err(|_| "Enter true or false")?
                }
                _ => return Err("Invalid material field".into()),
            }
        }
    }
    Ok(())
}
struct SetProperty {
    target: Target,
    path: PropertyPath,
    before: String,
    after: String,
    before_color: Option<Color>,
}
impl EditorCommand for SetProperty {
    fn name(&self) -> &str {
        "Edit inspector property"
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        apply_value(world, &self.target, &self.path, &self.after)
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        if let (Some(color), Target::Material(handle)) = (self.before_color, &self.target) {
            world
                .resource_mut::<Assets<StandardMaterial>>()
                .get_mut(handle)
                .ok_or("Material no longer exists")?
                .base_color = color;
            return Ok(());
        }
        apply_value(world, &self.target, &self.path, &self.before)
    }
}
fn commit(world: &mut World, edit: &ActiveEdit) -> Result<(), String> {
    let before = read_value(world, &edit.target, &edit.path).ok_or("Property no longer exists")?;
    if before == edit.buffer {
        return Ok(());
    }
    let command = SetProperty {
        target: edit.target.clone(),
        path: edit.path.clone(),
        before,
        after: edit.buffer.clone(),
        before_color: if edit.path == PropertyPath::BaseColor {
            if let Target::Material(handle) = &edit.target {
                world
                    .resource::<Assets<StandardMaterial>>()
                    .get(handle)
                    .map(|material| material.base_color)
            } else {
                None
            }
        } else {
            None
        },
    };
    world.resource_scope(|world, mut history: Mut<CommandHistory>| {
        history.execute(Box::new(command), world)
    })
}
fn begin_edit(world: &mut World, field: Entity) {
    if *world.resource::<PlayState>() != PlayState::Edit {
        return;
    }
    if let Some(mut previous) = world.resource_mut::<PropertyEditState>().active.take() {
        if let Err(error) = commit(world, &previous) {
            previous.error = Some(error);
            world.resource_mut::<PropertyEditState>().active = Some(previous);
            return;
        }
    }
    let Some(binding) = world.get::<PropertyField>(field).cloned() else {
        return;
    };
    let Some(selected) = binding.entity.or(world.resource::<InspectorSelection>().0) else {
        return;
    };
    let Some(target) = resolve(world, selected, &binding.path) else {
        return;
    };
    let Some(buffer) = read_value(world, &target, &binding.path) else {
        return;
    };
    let mut edit = ActiveEdit {
        field,
        selected,
        target,
        path: binding.path,
        buffer,
        replace: true,
        error: None,
    };
    if let (Target::Reflect(entity), PropertyPath::Reflect { type_path, field }) = (&edit.target, &edit.path) {
        if reflect_fields::field_meta(world, *entity, type_path, field).is_some_and(|m| m.read_only) {
            return;
        }
        // Booleans and unit enums advance on click instead of opening a text edit.
        if let Some(next) = reflect_fields::next_toggle_value(world, *entity, type_path, field) {
            edit.buffer = next;
            if let Err(error) = commit(world, &edit) {
                warn!("{error}");
            }
            world.resource_mut::<PropertyEditState>().active = None;
            return;
        }
    }
    let boolean = match &edit.target {
        Target::Reflect(_) => false,
        Target::Material(_) => edit.path == PropertyPath::Unlit,
        Target::Script(entity) => {
            if let PropertyPath::Script(key, _) = &edit.path {
                matches!(
                    world
                        .get::<ScriptComponent>(*entity)
                        .and_then(|s| s.properties.get(key)),
                    Some(ScriptPropertyVal::Bool(_))
                )
            } else {
                false
            }
        }
    };
    if boolean {
        edit.buffer = (edit.buffer != "true").to_string();
        if let Err(error) = commit(world, &edit) {
            warn!("{error}");
        }
        world.resource_mut::<PropertyEditState>().active = None;
    } else {
        world.resource_mut::<PropertyEditState>().active = Some(edit);
    }
    world.resource_mut::<super::TransformTextEditState>().active = None;
    world
        .resource_mut::<crate::ui::hierarchy::HierarchyState>()
        .search_focused = false;
}

/// Keyboard events are accepted only from the focused editor window.
pub fn update_property_editor(world: &mut World, mut cursor: Local<MessageCursor<KeyboardInput>>) {
    let selected = world.resource::<InspectorSelection>().0;
    let editable = *world.resource::<PlayState>() == PlayState::Edit;
    let primary = world
        .query_filtered::<(Entity, &Window), With<PrimaryWindow>>()
        .iter(world)
        .find(|(_, w)| w.focused)
        .map(|(e, _)| e);
    let events: Vec<_> = cursor
        .read(world.resource::<Messages<KeyboardInput>>())
        .cloned()
        .collect();
    let active = world.resource_mut::<PropertyEditState>().active.take();
    if let Some(mut edit) = active {
        let bound = world.get::<PropertyField>(edit.field).and_then(|f| f.entity).or(selected);
        if editable && bound == Some(edit.selected) && world.get_entity(edit.field).is_ok() {
            let mut keep = true;
            for event in events
                .iter()
                .filter(|e| Some(e.window) == primary && e.state.is_pressed())
            {
                match event.key_code {
                    KeyCode::Escape => {
                        keep = false;
                        break;
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter => match commit(world, &edit) {
                        Ok(()) => {
                            keep = false;
                            break;
                        }
                        Err(error) => edit.error = Some(error),
                    },
                    KeyCode::Backspace => {
                        if edit.replace {
                            edit.buffer.clear();
                        } else {
                            edit.buffer.pop();
                        }
                        edit.replace = false;
                        edit.error = None;
                    }
                    _ => {
                        if let Key::Character(text) = &event.logical_key {
                            let keys = world.resource::<ButtonInput<KeyCode>>();
                            let ctrl = keys.pressed(KeyCode::ControlLeft)
                                || keys.pressed(KeyCode::ControlRight);
                            if ctrl {
                                if event.key_code == KeyCode::KeyA {
                                    edit.replace = true;
                                }
                                continue;
                            }
                            if edit.replace {
                                edit.buffer.clear();
                                edit.replace = false;
                            }
                            edit.buffer.push_str(text);
                            edit.error = None;
                        }
                    }
                }
            }
            if keep {
                world.resource_mut::<PropertyEditState>().active = Some(edit);
            }
        }
    }
    let fields: Vec<_> = world
        .query::<(Entity, &PropertyField, &Children)>()
        .iter(world)
        .map(|(e, f, c)| (e, f.clone(), c.iter().collect::<Vec<_>>()))
        .collect();
    for (entity, field, children) in fields {
        let active = world
            .resource::<PropertyEditState>()
            .active
            .as_ref()
            .filter(|e| e.field == entity);
        let value = if let Some(edit) = active {
            format!("{}▏", edit.buffer)
        } else {
            field.entity.or(selected)
                .and_then(|s| resolve(world, s, &field.path))
                .and_then(|t| read_value(world, &t, &field.path))
                .unwrap_or_else(|| "—".into())
        };
        let color = if active.is_some_and(|e| e.error.is_some()) {
            ACCENT_CRIMSON
        } else if active.is_some() {
            ACCENT_CYAN
        } else {
            Color::NONE
        };
        if let Some(mut border) = world.get_mut::<BorderColor>(entity) {
            let next = BorderColor::all(color);
            if *border != next {
                *border = next;
            }
        }
        for child in children {
            if world.get::<PropertyValue>(child).is_some() {
                if let Some(mut text) = world.get_mut::<Text>(child) {
                    if text.0 != value {
                        text.0 = value.clone();
                    }
                }
            }
        }
    }
    let help = world
        .resource::<PropertyEditState>()
        .active
        .as_ref()
        .and_then(|e| e.error.clone())
        .unwrap_or_else(|| "Click a value to edit · Enter applies · Esc cancels".into());
    for mut text in world
        .query_filtered::<&mut Text, With<PropertyHelp>>()
        .iter_mut(world)
    {
        if text.0 != help {
            text.0 = help.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn material_edit_validates_and_undo_restores_shared_asset() {
        let mut world = World::new();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<CommandHistory>();
        let handle = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let target = Target::Material(handle.clone());
        assert!(apply_value(&mut world, &target, &PropertyPath::Roughness, "NaN").is_err());
        assert!(apply_value(&mut world, &target, &PropertyPath::BaseColor, "#GG11FF").is_err());
        let before = read_value(&world, &target, &PropertyPath::Roughness).unwrap();
        world
            .resource_scope(|world, mut history: Mut<CommandHistory>| {
                history.execute(
                    Box::new(SetProperty {
                        target,
                        path: PropertyPath::Roughness,
                        before: before.clone(),
                        after: "0.25".into(),
                        before_color: None,
                    }),
                    world,
                )
            })
            .unwrap();
        assert_eq!(
            world
                .resource::<Assets<StandardMaterial>>()
                .get(&handle)
                .unwrap()
                .perceptual_roughness,
            0.25
        );
        world
            .resource_scope(|world, mut history: Mut<CommandHistory>| history.undo(world))
            .unwrap();
        assert_eq!(
            world
                .resource::<Assets<StandardMaterial>>()
                .get(&handle)
                .unwrap()
                .perceptual_roughness,
            before.parse::<f32>().unwrap()
        );
    }
}

//! Generic "Components" card: renders every reflected component without a dedicated card.
//!
//! Field labels, units, ranges, steps, tooltips and read-only state come from
//! `bevy_annotations` (see [`crate::core::reflect_fields`]). Edits go through the shared
//! property editor, so they use the same keyboard flow and undo history as the other cards.
use bevy::{
    ecs::change_detection::Tick,
    inspector::InspectorSelection,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};

use super::property_editor::{PropertyPath, spawn_property_value_for};
use crate::core::play_mode::PlayState;
use crate::core::reflect_fields::{self, ComponentMeta, FieldKind, ToggleReflectComponent};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

#[derive(Component)]
pub struct ReflectCardBody;

#[derive(Component)]
struct ReflectCardHint;

#[derive(Component)]
struct ReflectCardRows;

/// Remembers what the card currently shows so it rebuilds only on structural change.
#[derive(Default)]
pub struct ReflectCardCache {
    shown: Option<(Entity, Vec<String>)>,
    archetype: Option<(Entity, bevy::ecs::archetype::ArchetypeId)>,
    last_tick: Option<Tick>,
}

pub fn spawn_reflect_card(commands: &mut Commands, body: Entity) {
    commands.entity(body).insert(ReflectCardBody);
    commands.spawn((
        EditorInternal,
        ReflectCardRows,
        Node { flex_direction: FlexDirection::Column, row_gap: px(2), ..default() },
        ChildOf(body),
    ));
    commands.spawn((
        EditorInternal,
        ReflectCardHint,
        Pickable::IGNORE,
        Text::new("Hover a field for help · click to edit · − / + step"),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default() },
        TextColor(TEXT_MUTED),
        Node { margin: UiRect::top(px(6)), ..default() },
        ChildOf(body),
    ));
}

fn structure(components: &[ComponentMeta]) -> Vec<String> {
    components
        .iter()
        .map(|c| {
            let fields: Vec<_> = c.fields.iter().map(|f| f.path.as_str()).collect();
            format!("{}|{}", c.type_path, fields.join(","))
        })
        .collect()
}

/// Exclusive system: rebuilds rows when the selection, its archetype, or a field layout changes.
pub fn update_reflect_card(world: &mut World, mut cache: Local<ReflectCardCache>) {
    let selected = world.resource::<InspectorSelection>().0.filter(|e| world.get_entity(*e).is_ok());
    let Some(card) = world.query_filtered::<&ChildOf, With<ReflectCardBody>>().iter(world).next().map(|c| c.parent()) else { return };
    let Some(rows) = world.query_filtered::<Entity, With<ReflectCardRows>>().iter(world).next() else { return };

    let this_tick = world.read_change_tick();
    let last_tick = cache.last_tick.replace(this_tick);
    let Some(entity) = selected else {
        if cache.shown.take().is_some() {
            clear_rows(world, rows);
        }
        set_display(world, card, false);
        return;
    };

    // Cheap checks first: a new archetype means components were added or removed;
    // a changed component may have switched an enum variant (e.g. Option None -> Some).
    let archetype = world.entity(entity).archetype().id();
    let moved = cache.archetype != Some((entity, archetype));
    let touched = moved
        || last_tick.is_none_or(|last| {
            reflect_fields::reflected_components(world, entity).iter().any(|(id, _, _)| {
                world.entity(entity).get_change_ticks_by_id(*id).is_some_and(|t| t.is_changed(last, this_tick))
            })
        });
    if !touched {
        return;
    }
    cache.archetype = Some((entity, archetype));

    let components = reflect_fields::describe_entity(world, entity);
    let signature = structure(&components);
    if cache.shown.as_ref().is_some_and(|(e, s)| *e == entity && *s == signature) {
        return;
    }
    cache.shown = Some((entity, signature));
    clear_rows(world, rows);
    set_display(world, card, !components.is_empty());
    let mut commands = world.commands();
    for component in &components {
        spawn_component_section(&mut commands, rows, entity, component, true);
    }
    world.flush();
}

fn clear_rows(world: &mut World, rows: Entity) {
    let children: Vec<Entity> = world.get::<Children>(rows).map(|c| c.iter().collect()).unwrap_or_default();
    for child in children {
        if let Ok(e) = world.get_entity_mut(child) {
            e.despawn();
        }
    }
}

fn set_display(world: &mut World, card: Entity, visible: bool) {
    if let Some(mut node) = world.get_mut::<Node>(card) {
        let display = if visible { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
}

fn hint_on_hover(text: &'static str) -> impl Fn(On<PointerOver>, Query<&mut Text, With<ReflectCardHint>>) {
    move |_: On<PointerOver>, mut hint: Query<&mut Text, With<ReflectCardHint>>| {
        for mut t in &mut hint {
            if t.0 != text {
                t.0 = text.to_string();
            }
        }
    }
}

fn small_button(commands: &mut Commands, parent: Entity, label: &str) -> Entity {
    let button = commands
        .spawn((
            EditorInternal,
            Node {
                width: px(22),
                height: px(24),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: radius_button(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(BG_INPUT),
            ChildOf(parent),
        ))
        .observe(|t: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(t.entity) { *bg = BackgroundColor(BG_HOVER); }
        })
        .observe(|t: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = bg.get_mut(t.entity) { *bg = BackgroundColor(BG_INPUT); }
        })
        .id();
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new(label),
        TextFont { font_size: bevy::text::FontSize::Px(13.0), ..default() },
        TextColor(TEXT_SECONDARY),
        ChildOf(button),
    ));
    button
}

/// Spawns one component's header and field rows, bound to ntity.
pub(crate) fn spawn_component_section(commands: &mut Commands, parent: Entity, entity: Entity, component: &ComponentMeta, removable: bool) {
    // Fixed sections (World settings) already sit under a card titled for them.
    if !removable {
        spawn_fields(commands, parent, entity, component);
        return;
    }
    let header = commands
        .spawn((
            EditorInternal,
            Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::top(px(8)),
                padding: UiRect::bottom(px(2)),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .id();
    let title = commands
        .spawn((
            EditorInternal,
            Text::new(component.name.clone()),
            TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_NORMAL), weight: FontWeight::SEMIBOLD, ..default() },
            TextColor(TEXT_PRIMARY),
            ChildOf(header),
        ))
        .id();
    if let Some(tip) = component.tooltip {
        commands.entity(title).observe(hint_on_hover(tip));
    }
    if removable {
        spawn_remove_button(commands, header, entity, component.type_path.clone());
    }
    spawn_fields(commands, parent, entity, component);
}

fn spawn_remove_button(commands: &mut Commands, header: Entity, entity: Entity, type_path: String) {
    let remove = small_button(commands, header, "✕");
    commands.entity(remove).observe(move |click: On<PointerClick>, mut commands: Commands| {
        if click.button != PointerButton::Primary {
            return;
        }
        let type_path = type_path.clone();
        commands.queue(move |world: &mut World| {
            if *world.resource::<PlayState>() != PlayState::Edit {
                return;
            }
            if let Err(error) = reflect_fields::execute_command(world, ToggleReflectComponent::remove(entity, type_path)) {
                warn!("{error}");
            }
        });
    });
}

fn spawn_fields(commands: &mut Commands, parent: Entity, entity: Entity, component: &ComponentMeta) {
    if component.fields.is_empty() {
        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new("Marker component (no fields)"),
            TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default() },
            TextColor(TEXT_MUTED),
            ChildOf(parent),
        ));
    }

    for field in &component.fields {
        let indent = 8.0 * field.depth as f32;
        if field.kind == FieldKind::Group {
            commands.spawn((
                EditorInternal,
                Pickable::IGNORE,
                Text::new(field.label.clone()),
                TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), weight: FontWeight::SEMIBOLD, ..default() },
                TextColor(TEXT_SECONDARY),
                Node { margin: UiRect { left: px(indent), top: px(4), ..default() }, ..default() },
                ChildOf(parent),
            ));
            continue;
        }
        let row = commands
            .spawn((
                EditorInternal,
                Node { align_items: AlignItems::Center, min_height: px(28), column_gap: px(4), padding: UiRect::left(px(indent)), ..default() },
                ChildOf(parent),
            ))
            .id();
        let label = commands
            .spawn((
                EditorInternal,
                Node { width: px((124.0 - indent).max(60.0)), flex_shrink: 0.0, ..default() },
                Text::new(field.label.clone()),
                TextFont { font_size: bevy::text::FontSize::Px(12.5), ..default() },
                TextColor(if field.read_only { TEXT_MUTED } else { TEXT_SECONDARY }),
                ChildOf(row),
            ))
            .id();
        if let Some(tip) = field.tooltip {
            commands.entity(label).observe(hint_on_hover(tip));
        }
        let numeric = matches!(field.kind, FieldKind::Float | FieldKind::Int) && !field.read_only;
        if numeric {
            spawn_nudge(commands, row, entity, &component.type_path, &field.path, -1.0);
        }
        let value = spawn_property_value_for(
            commands,
            row,
            PropertyPath::Reflect { type_path: component.type_path.clone(), field: field.path.clone() },
            Some(entity),
        );
        if field.read_only {
            commands.entity(value).insert(BackgroundColor(BG_PANEL));
        }
        if numeric {
            spawn_nudge(commands, row, entity, &component.type_path, &field.path, 1.0);
        }
        if let Some(unit) = field.unit {
            commands.spawn((
                EditorInternal,
                Pickable::IGNORE,
                Text::new(unit),
                TextLayout::no_wrap(),
                TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default() },
                TextColor(TEXT_MUTED),
                Node { min_width: px(22), ..default() },
                ChildOf(row),
            ));
        }
    }
}

fn spawn_nudge(commands: &mut Commands, row: Entity, entity: Entity, type_path: &str, path: &str, steps: f64) {
    let button = small_button(commands, row, if steps < 0.0 { "−" } else { "+" });
    let (type_path, path) = (type_path.to_string(), path.to_string());
    commands.entity(button).observe(move |mut click: On<PointerClick>, mut commands: Commands| {
        if click.button != PointerButton::Primary {
            return;
        }
        click.propagate(false);
        let (type_path, path) = (type_path.clone(), path.clone());
        commands.queue(move |world: &mut World| {
            if *world.resource::<PlayState>() != PlayState::Edit {
                return;
            }
            let shift = world.resource::<ButtonInput<KeyCode>>().any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
            let result = reflect_fields::nudge_field(world, entity, &type_path, &path, if shift { steps * 10.0 } else { steps })
                .and_then(|(_, after)| reflect_fields::set_field_with_undo(world, entity, &type_path, &path, &after));
            if let Err(error) = result {
                warn!("{error}");
            }
        });
    });
}

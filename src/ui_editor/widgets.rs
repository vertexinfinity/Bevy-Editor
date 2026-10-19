//! Runtime behaviour of UI Editor widgets: visual sync (edit + play) and Play-mode input.
use bevy::{
    ecs::{query::QueryFilter, system::SystemParam},
    picking::events::{PointerCancel, PointerClick, PointerDrag, PointerDragEnd, PointerOut, PointerOver, PointerPress, PointerRelease},
    picking::pointer::PointerButton,
    prelude::*,
    ui::Pressed,
};

use super::model::{UiCheckbox, UiPartRole, UiProgressBar, UiSlider, UiWidgetPart};
use crate::core::play_mode::{PlayState, PreviewEntity};
use crate::scene::components::UiButtonClickAction;

/// Original background of a Play-mode button copy, so hover/press tints can be undone.
#[derive(Component, Clone, Copy)]
pub struct ButtonBaseColor(pub Color);

/// Play-mode pointer state for widget copies.
#[derive(Resource, Default)]
pub struct WidgetInputState {
    hovered: Option<Entity>,
    dragging_slider: Option<Entity>,
}

pub fn install(app: &mut App) {
    app.init_resource::<WidgetInputState>().add_systems(
        Update,
        (restore_play_child_order, play_widget_input, tint_buttons, sync_widget_visuals).chain(),
    );
}

fn find_part(children: &Query<&Children>, parts: &Query<&UiWidgetPart>, root: Entity, role: UiPartRole) -> Option<Entity> {
    children
        .iter_descendants(root)
        .find(|c| parts.get(*c).is_ok_and(|p| p.role == role))
}

/// Keeps fills, thumbs and check marks in sync with widget values (scripts only change values).
#[allow(clippy::type_complexity)]
pub fn sync_widget_visuals(
    bars: Query<(Entity, &UiProgressBar), Changed<UiProgressBar>>,
    boxes: Query<(Entity, &UiCheckbox), Changed<UiCheckbox>>,
    sliders: Query<(Entity, &UiSlider), Changed<UiSlider>>,
    children: Query<&Children>,
    parts: Query<&UiWidgetPart>,
    mut nodes: Query<(&mut Node, &mut BackgroundColor)>,
) {
    for (e, bar) in &bars {
        if let Some(fill) = find_part(&children, &parts, e, UiPartRole::Fill) {
            if let Ok((mut node, mut bg)) = nodes.get_mut(fill) {
                if node.width != Val::Percent(bar.fraction() * 100.0) {
                    node.width = Val::Percent(bar.fraction() * 100.0);
                }
                if bg.0 != bar.fill_color {
                    bg.0 = bar.fill_color;
                }
            }
        }
    }
    for (e, cb) in &boxes {
        if let Some(mark) = find_part(&children, &parts, e, UiPartRole::CheckMark) {
            if let Ok((mut node, mut bg)) = nodes.get_mut(mark) {
                crate::ui::set_display(&mut node, if cb.checked { Display::Flex } else { Display::None });
                if bg.0 != cb.mark_color {
                    bg.0 = cb.mark_color;
                }
            }
        }
    }
    for (e, s) in &sliders {
        let t = s.fraction() * 100.0;
        if let Some(fill) = find_part(&children, &parts, e, UiPartRole::Fill) {
            if let Ok((mut node, mut bg)) = nodes.get_mut(fill) {
                if node.width != Val::Percent(t) {
                    node.width = Val::Percent(t);
                }
                if bg.0 != s.fill_color {
                    bg.0 = s.fill_color;
                }
            }
        }
        if let Some(thumb) = find_part(&children, &parts, e, UiPartRole::Thumb) {
            if let Ok((mut node, _)) = nodes.get_mut(thumb) {
                if node.left != Val::Percent(t) {
                    node.left = Val::Percent(t);
                }
            }
        }
    }
}

fn climb<F: QueryFilter>(start: Entity, parents: &Query<&ChildOf>, has: &Query<(), F>) -> Option<Entity> {
    let mut cur = Some(start);
    while let Some(e) = cur {
        if has.contains(e) {
            return Some(e);
        }
        cur = parents.get(e).ok().map(|c| c.parent());
    }
    None
}

fn slider_fraction(pos: Vec2, node: &ComputedNode, xf: &UiGlobalTransform) -> f32 {
    let inv = node.inverse_scale_factor;
    let width = node.size().x * inv;
    let left = xf.translation.x * inv - width * 0.5;
    if width <= 0.0 { 0.0 } else { (pos.x - left) / width }
}

#[derive(SystemParam)]
struct PointerMessages<'w, 's> {
    presses: MessageReader<'w, 's, PointerPress>,
    releases: MessageReader<'w, 's, PointerRelease>,
    drags: MessageReader<'w, 's, PointerDrag>,
    drag_ends: MessageReader<'w, 's, PointerDragEnd>,
    cancels: MessageReader<'w, 's, PointerCancel>,
    clicks: MessageReader<'w, 's, PointerClick>,
    overs: MessageReader<'w, 's, PointerOver>,
    outs: MessageReader<'w, 's, PointerOut>,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn play_widget_input(
    play: Res<PlayState>,
    mut state: ResMut<WidgetInputState>,
    mut msgs: PointerMessages,
    parents: Query<&ChildOf>,
    previews: Query<(), With<PreviewEntity>>,
    buttons: Query<(), (With<UiButtonClickAction>, With<PreviewEntity>)>,
    checkbox_q: Query<(), (With<UiCheckbox>, With<PreviewEntity>)>,
    slider_q: Query<(), (With<UiSlider>, With<PreviewEntity>)>,
    mut checkboxes: Query<&mut UiCheckbox, With<PreviewEntity>>,
    mut sliders: Query<(&mut UiSlider, &ComputedNode, &UiGlobalTransform), With<PreviewEntity>>,
    pressed: Query<Entity, (With<Pressed>, With<PreviewEntity>)>,
    mut commands: Commands,
) {
    let PointerMessages { presses, releases, drags, drag_ends, cancels, clicks, overs, outs } = &mut msgs;
    if *play != PlayState::Playing {
        presses.clear();
        releases.clear();
        drags.clear();
        drag_ends.clear();
        cancels.clear();
        clicks.clear();
        overs.clear();
        outs.clear();
        state.hovered = None;
        state.dragging_slider = None;
        return;
    }
    let mut set_slider = |e: Entity, pos: Vec2| {
        if let Ok((mut s, node, xf)) = sliders.get_mut(e) {
            let t = slider_fraction(pos, node, xf);
            let mut next = s.clone();
            next.set_fraction(t);
            if next.value != s.value {
                s.value = next.value;
            }
        }
    };
    for ev in presses.read() {
        if ev.button != PointerButton::Primary || !previews.contains(ev.entity) {
            continue;
        }
        if let Some(b) = climb(ev.entity, &parents, &buttons) {
            commands.entity(b).try_insert(Pressed);
        }
        if let Some(s) = climb(ev.entity, &parents, &slider_q) {
            state.dragging_slider = Some(s);
            set_slider(s, ev.pointer.position);
        }
    }
    for ev in drags.read() {
        if ev.button != PointerButton::Primary {
            continue;
        }
        if let Some(s) = state.dragging_slider {
            set_slider(s, ev.pointer.position);
        }
    }
    let released = releases.read().count() + drag_ends.read().count() + cancels.read().count();
    if released > 0 {
        for b in &pressed {
            commands.entity(b).try_remove::<Pressed>();
        }
        state.dragging_slider = None;
    }
    for ev in clicks.read() {
        if ev.button != PointerButton::Primary || !previews.contains(ev.entity) {
            continue;
        }
        if let Some(c) = climb(ev.entity, &parents, &checkbox_q) {
            if let Ok(mut cb) = checkboxes.get_mut(c) {
                cb.checked = !cb.checked;
                info!("[UI] Checkbox toggled: {}", cb.checked);
            }
        }
    }
    for ev in overs.read() {
        if let Some(b) = climb(ev.entity, &parents, &buttons) {
            state.hovered = Some(b);
        }
    }
    for ev in outs.read() {
        if climb(ev.entity, &parents, &buttons) == state.hovered {
            state.hovered = None;
        }
    }
}

/// Hover / press feedback for Play-mode button copies.
#[allow(clippy::type_complexity)]
fn tint_buttons(
    state: Res<WidgetInputState>,
    mut commands: Commands,
    mut buttons: Query<(Entity, &mut BackgroundColor, Option<&ButtonBaseColor>, Has<Pressed>), (With<UiButtonClickAction>, With<PreviewEntity>)>,
) {
    for (e, mut bg, base, pressed) in &mut buttons {
        let Some(base) = base.map(|b| b.0) else {
            commands.entity(e).try_insert(ButtonBaseColor(bg.0));
            continue;
        };
        let factor = if pressed { 0.75 } else if state.hovered == Some(e) { 1.2 } else { 1.0 };
        let c = base.to_srgba();
        let next = Color::srgba((c.red * factor).min(1.0), (c.green * factor).min(1.0), (c.blue * factor).min(1.0), c.alpha);
        if bg.0 != next {
            bg.0 = next;
        }
    }
}

/// The Play-mode copy re-links children in query order, which scrambles flex layouts.
/// Once per session, reorder each UI copy's children to match its authoring source.
fn restore_play_child_order(world: &mut World, mut done_for: Local<Option<Entity>>) {
    let session = world.resource::<crate::core::play_mode::PlaySession>();
    let Some(window) = session.window else {
        *done_for = None;
        return;
    };
    if *done_for == Some(window) || session.entity_map.is_empty() {
        return;
    }
    let pairs: Vec<(Entity, Entity)> = session.entity_map.iter().map(|(s, c)| (*s, *c)).collect();
    for (source, copy) in &pairs {
        if world.get::<Node>(*copy).is_none() {
            continue;
        }
        let Some(children) = world.get::<Children>(*source).map(|c| c.to_vec()) else { continue };
        let map = &world.resource::<crate::core::play_mode::PlaySession>().entity_map;
        let ordered: Vec<Entity> = children.iter().filter_map(|c| map.get(c).copied()).collect();
        let current: Vec<Entity> = world.get::<Children>(*copy).map(|c| c.to_vec()).unwrap_or_default();
        if !ordered.is_empty() && ordered.iter().all(|c| current.contains(c)) && current != ordered {
            world.entity_mut(*copy).insert_children(0, &ordered);
        }
    }
    *done_for = Some(window);
}

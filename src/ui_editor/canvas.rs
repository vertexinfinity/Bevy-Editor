//! Viewport interaction while the UI Editor is open: selection outline, click-to-select,
//! drag-to-move, and hiding authored game UI in the editor viewport.
use bevy::{
    ecs::message::MessageCursor,
    inspector::InspectorSelection,
    picking::events::{PointerClick, PointerDrag, PointerDragEnd, PointerDragStart, PointerPress},
    picking::pointer::PointerButton,
    prelude::*,
};

use super::{model, UiEditorState};
use crate::core::play_mode::{PlayState, PreviewEntity};
use crate::scene::{EditorInternal, SceneEntity};
use crate::ui::theme::*;

#[derive(Component)]
pub struct SelectionOutline;

#[derive(Component)]
pub struct SelectionOutlineLabel;

/// An in-progress canvas drag.
pub struct DragState {
    pub entity: Entity,
    pub start_offset: Vec2,
    pub before: model::CompSnap,
    pub moved: bool,
}

pub fn spawn_outline(world: &mut World) -> Entity {
    let outline = world
        .spawn((
            EditorInternal,
            SelectionOutline,
            Name::new("UiEditorSelectionOutline"),
            Pickable::IGNORE,
            GlobalZIndex(15),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(ACCENT_CYAN),
            BackgroundColor(Color::srgba(0.27, 0.77, 0.86, 0.06)),
        ))
        .id();
    world.spawn((
        EditorInternal,
        SelectionOutlineLabel,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(-2.0),
            bottom: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(ACCENT_CYAN),
        Text::new(""),
        TextFont { font_size: bevy::text::FontSize::Px(11.0), ..default() },
        TextColor(BG_SHELL),
        TextLayout::no_wrap(),
        ChildOf(outline),
    ));
    outline
}

/// Places the outline over the selected game UI element.
pub fn update_outline(world: &mut World) {
    let open = world.resource::<UiEditorState>().open;
    let selected = world.resource::<InspectorSelection>().0;
    let rect = selected
        .filter(|e| open && model::is_game_ui(world, *e))
        .and_then(|e| {
            let node = world.get::<ComputedNode>(e)?;
            let xf = world.get::<UiGlobalTransform>(e)?;
            let visible = world.get::<InheritedVisibility>(e).is_none_or(|v| v.get());
            if !visible || node.size() == Vec2::ZERO && world.get::<Node>(e).is_some_and(|n| n.display == Display::None) {
                return None;
            }
            let inv = node.inverse_scale_factor;
            let size = node.size() * inv;
            let center = xf.translation * inv;
            Some((e, center - size * 0.5, size))
        });
    let label = rect.map(|(e, _, size)| format!("{}  {}×{}", model::display_name(world, e), size.x.round(), size.y.round()));
    let mut q = world.query_filtered::<&mut Node, With<SelectionOutline>>();
    for mut node in q.iter_mut(world) {
        match rect {
            Some((_, min, size)) => {
                let next = (Display::Flex, Val::Px(min.x - 2.0), Val::Px(min.y - 2.0), Val::Px(size.x + 4.0), Val::Px(size.y + 4.0));
                if (node.display, node.left, node.top, node.width, node.height) != next {
                    node.display = next.0;
                    node.left = next.1;
                    node.top = next.2;
                    node.width = next.3;
                    node.height = next.4;
                }
            }
            None => {
                if node.display != Display::None {
                    node.display = Display::None;
                }
            }
        }
    }
    if let Some(label) = label {
        let mut q = world.query_filtered::<&mut Text, With<SelectionOutlineLabel>>();
        for mut t in q.iter_mut(world) {
            if t.0 != label {
                t.0 = label.clone();
            }
        }
    }
}

/// Resolves a picking hit to the authored element it belongs to.
fn hit_element(world: &World, hit: Entity) -> Option<Entity> {
    if world.get::<EditorInternal>(hit).is_some() {
        return None;
    }
    model::is_game_ui(world, hit).then(|| model::owner_of(world, hit))
}

#[derive(Default)]
pub struct CanvasCursors {
    press: MessageCursor<PointerPress>,
    click: MessageCursor<PointerClick>,
    drag_start: MessageCursor<PointerDragStart>,
    drag: MessageCursor<PointerDrag>,
    drag_end: MessageCursor<PointerDragEnd>,
}

/// Click selects the deepest authored element (not its root); dragging moves it.
pub fn canvas_interaction(world: &mut World, mut cursors: Local<CanvasCursors>) {
    let presses: Vec<_> = cursors.press.read(world.resource::<Messages<PointerPress>>()).map(|e| (e.entity, e.button)).collect();
    let clicks: Vec<_> = cursors.click.read(world.resource::<Messages<PointerClick>>()).map(|e| (e.entity, e.button)).collect();
    let starts: Vec<_> = cursors.drag_start.read(world.resource::<Messages<PointerDragStart>>()).map(|e| (e.entity, e.button)).collect();
    let drags: Vec<_> = cursors.drag.read(world.resource::<Messages<PointerDrag>>()).map(|e| (e.button, e.distance)).collect();
    let ends: Vec<_> = cursors.drag_end.read(world.resource::<Messages<PointerDragEnd>>()).map(|e| e.button).collect();
    let active = world.resource::<UiEditorState>().open && *world.resource::<PlayState>() == PlayState::Edit;
    if !active {
        world.resource_mut::<UiEditorState>().drag = None;
        return;
    }
    let mut select = None;
    for (hit, button) in presses.into_iter().chain(clicks) {
        if button == PointerButton::Primary {
            if let Some(e) = hit_element(world, hit) {
                select = Some(e);
            }
        }
    }
    if let Some(e) = select {
        world.resource_mut::<InspectorSelection>().0 = Some(e);
    }
    for (hit, button) in starts {
        if button != PointerButton::Primary {
            continue;
        }
        if let Some(e) = hit_element(world, hit) {
            let drag = DragState { entity: e, start_offset: model::offset_of(world, e), before: model::CompSnap::capture(world, e), moved: false };
            world.resource_mut::<UiEditorState>().drag = Some(drag);
        }
    }
    for (button, distance) in drags {
        if button != PointerButton::Primary {
            continue;
        }
        let Some((e, start)) = world.resource::<UiEditorState>().drag.as_ref().map(|d| (d.entity, d.start_offset)) else { continue };
        if world.get_entity(e).is_err() {
            world.resource_mut::<UiEditorState>().drag = None;
            continue;
        }
        model::set_offset(world, e, start + distance);
        if let Some(d) = world.resource_mut::<UiEditorState>().drag.as_mut() {
            d.moved = true;
        }
    }
    if ends.contains(&PointerButton::Primary) {
        if let Some(d) = world.resource_mut::<UiEditorState>().drag.take() {
            if d.moved && world.get_entity(d.entity).is_ok() {
                model::record_change(world, d.entity, d.before, "UI: move on canvas");
            }
        }
    }
}

/// Hides authored game UI in the editor viewport (Play-mode copies are unaffected).
/// Overrides the computed visibility only, so nothing hidden is saved into the scene.
#[allow(clippy::type_complexity)]
pub fn apply_game_ui_visibility(
    mut state: ResMut<UiEditorState>,
    mut nodes: Query<(&mut InheritedVisibility, &mut Visibility), (With<Node>, With<SceneEntity>, Without<EditorInternal>, Without<PreviewEntity>)>,
) {
    if !state.show_game_ui {
        for (mut iv, _) in &mut nodes {
            if iv.get() {
                *iv = InheritedVisibility::HIDDEN;
            }
        }
        state.ui_hidden_applied = true;
    } else if state.ui_hidden_applied {
        // Re-run visibility propagation next frame by touching every authored node.
        for (_, mut vis) in &mut nodes {
            vis.set_changed();
        }
        state.ui_hidden_applied = false;
    }
}

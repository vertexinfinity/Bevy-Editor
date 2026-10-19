//! The Shader Editor window: file list, code editor with line numbers, and parameter panel.
use bevy::{
    ecs::message::MessageCursor,
    input::keyboard::{Key, KeyboardInput},
    picking::{
        events::{PointerClick, PointerDrag, PointerDragEnd, PointerOut, PointerOver, PointerPress, PointerRelease, PointerScroll},
        pointer::PointerButton,
    },
    prelude::*,
    window::PrimaryWindow,
};

use super::component::{record_shader_ref_edit, set_shader_ref_with_undo, ShaderMaterialRef};
use super::library::{ShaderLibrary, ShaderMeta};
use super::ops::{self, ShaderEditorState, StatusKind};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

const LINE_H: f32 = 18.0;
const CODE_FONT: f32 = 13.0;
const GUTTER_W: f32 = 46.0;
const CODE_PAD: f32 = 8.0;
const MEASURE_CHARS: usize = 40;

const COLOR_KEYWORD: Color = Color::srgb(0.78, 0.55, 0.98);
const COLOR_TYPE: Color = ACCENT_CYAN;
const COLOR_NUMBER: Color = Color::srgb(0.98, 0.72, 0.36);
const COLOR_BUILTIN: Color = Color::srgb(0.38, 0.85, 0.62);
const COLOR_META: Color = Color::srgb(0.85, 0.66, 0.35);
const ERROR_ROW: Color = Color::srgba(0.94, 0.25, 0.25, 0.17);
const CURRENT_ROW: Color = Color::srgba(1.0, 1.0, 1.0, 0.035);

#[derive(Component)]
pub struct ShaderEditorRoot;
#[derive(Component)]
struct ShaderListPanel;
#[derive(Component)]
struct ParamPanel;
#[derive(Component)]
struct CodeArea;
#[derive(Component)]
struct CodeRow(usize);
#[derive(Component)]
struct RowGutter;
#[derive(Component, Default)]
struct RowText {
    cached: Option<String>,
}
#[derive(Component)]
struct Caret;
#[derive(Component)]
struct CharMeasure;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct TitleText;
#[derive(Component)]
struct CaretPosText;
#[derive(Component)]
struct DeleteLabel;
#[derive(Component)]
struct ButtonBase(Color);
#[derive(Component, Clone, Copy, PartialEq)]
struct ParamValueText(SliderTarget);
#[derive(Component, Clone, Copy, PartialEq)]
struct SliderFill(SliderTarget);
#[derive(Component)]
struct ColorSwatch;
#[derive(Component)]
struct ColorHexText;

#[derive(Clone, Copy, Debug, PartialEq)]
enum SliderTarget {
    Param(usize),
    Color(usize),
}

#[derive(Clone, Debug)]
enum Action {
    Close,
    Save,
    RevertFile,
    Open(String),
    TogglePicker,
    NewFromTemplate(String),
    Duplicate,
    Delete,
    ApplySelected,
    RevertSelected,
    Step(SliderTarget, f32),
}

/// Opens or closes the window (File menu entry and `editor/shader_editor`).
pub fn toggle_window(world: &mut World) {
    let open = !world.resource::<ShaderEditorState>().open;
    set_open(world, open);
}

pub fn set_open(world: &mut World, open: bool) {
    if open && world.query_filtered::<(), With<ShaderEditorRoot>>().iter(world).next().is_none() {
        spawn_window(world);
    }
    if open {
        ops::rescan(world);
    }
    let mut state = world.resource_mut::<ShaderEditorState>();
    state.open = open;
    state.list_dirty = true;
    state.params_dirty = true;
    if !open {
        state.focused = false;
    }
    if open && state.current.is_none() {
        let first = world.resource::<ShaderLibrary>().entries.first().map(|e| e.path.clone());
        if let Some(path) = first {
            let _ = ops::open_doc(world, &path);
        }
    }
    let mut roots = world.query_filtered::<&mut Node, With<ShaderEditorRoot>>();
    for mut node in roots.iter_mut(world) {
        node.display = if open { Display::Flex } else { Display::None };
    }
}

pub fn despawn_window(world: &mut World) {
    let roots: Vec<Entity> = world.query_filtered::<Entity, With<ShaderEditorRoot>>().iter(world).collect();
    for root in roots {
        world.despawn(root);
    }
    let mut state = world.resource_mut::<ShaderEditorState>();
    state.open = false;
    state.focused = false;
}

fn code_font(size: f32) -> TextFont {
    // The default font handle is Bevy's embedded FiraMono: monospaced, so columns line up.
    TextFont { font: Handle::<Font>::default().into(), font_size: bevy::text::FontSize::Px(size), ..default() }
}

fn ui_font(size: f32) -> TextFont {
    TextFont { font_size: bevy::text::FontSize::Px(size), ..default() }
}

fn label(commands: &mut Commands, parent: Entity, text: impl Into<String>, size: f32, color: Color) -> Entity {
    commands
        .spawn((EditorInternal, Pickable::IGNORE, Text::new(text.into()), ui_font(size), TextColor(color), ChildOf(parent)))
        .id()
}

fn button(commands: &mut Commands, parent: Entity, text: impl Into<String>, base: Color, action: Action) -> Entity {
    let id = commands
        .spawn((
            EditorInternal,
            ButtonBase(base),
            Node {
                padding: UiRect::axes(px(8), px(4)),
                border_radius: radius_button(),
                border: UiRect::all(px(1)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(base),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(parent),
        ))
        .observe(|t: On<PointerOver>, mut q: Query<&mut BackgroundColor>| {
            if let Ok(mut bg) = q.get_mut(t.entity) {
                *bg = BackgroundColor(BG_HOVER);
            }
        })
        .observe(|t: On<PointerOut>, mut q: Query<(&mut BackgroundColor, &ButtonBase)>| {
            if let Ok((mut bg, base)) = q.get_mut(t.entity) {
                *bg = BackgroundColor(base.0);
            }
        })
        .observe(move |mut click: On<PointerClick>, mut commands: Commands| {
            if click.button == PointerButton::Primary {
                click.propagate(false);
                let action = action.clone();
                commands.queue(move |world: &mut World| perform(world, action));
            }
        })
        .id();
    label(commands, id, text, 11.5, TEXT_PRIMARY);
    id
}

fn spawn_window(world: &mut World) {
    let mut commands = world.commands();
    let root = commands
        .spawn((
            EditorInternal,
            ShaderEditorRoot,
            GlobalZIndex(150),
            Node {
                position_type: PositionType::Absolute,
                left: px(250),
                top: px(70),
                width: px(1080),
                height: px(660),
                max_width: percent(96.0),
                max_height: percent(90.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                border_radius: radius_panel(),
                overflow: Overflow::clip(),
                display: Display::None,
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_CARD),
            BoxShadow::new(Color::srgba(0.0, 0.0, 0.0, 0.55), px(0), px(6), px(0), px(18)),
        ))
        .id();

    // Header (drag to move the window).
    let header = commands
        .spawn((
            EditorInternal,
            Node {
                height: px(36),
                flex_shrink: 0.0,
                padding: UiRect::axes(px(10), px(4)),
                column_gap: px(8),
                align_items: AlignItems::Center,
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(root),
        ))
        .observe(|drag: On<PointerDrag>, mut roots: Query<&mut Node, With<ShaderEditorRoot>>| {
            if drag.button != PointerButton::Primary {
                return;
            }
            for mut node in &mut roots {
                if let (Val::Px(left), Val::Px(top)) = (node.left, node.top) {
                    node.left = px((left + drag.delta.x).max(0.0));
                    node.top = px((top + drag.delta.y).max(0.0));
                }
            }
        })
        .id();
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("🎨 Shader Editor"),
        TextFont { font_size: bevy::text::FontSize::Px(13.5), weight: FontWeight::BOLD, ..default() },
        TextColor(TEXT_PRIMARY),
        ChildOf(header),
    ));
    commands.spawn((EditorInternal, TitleText, Pickable::IGNORE, Text::new(""), code_font(12.0), TextColor(TEXT_SECONDARY), ChildOf(header)));
    commands.spawn((EditorInternal, Pickable::IGNORE, Node { flex_grow: 1.0, ..default() }, ChildOf(header)));
    button(&mut commands, header, "💾 Save & Apply  (Ctrl+S)", ACCENT_CYAN_DIM, Action::Save);
    button(&mut commands, header, "↶ Revert File", BG_INPUT, Action::RevertFile);
    button(&mut commands, header, "✕", BG_INPUT, Action::Close);

    let body = commands
        .spawn((EditorInternal, Node { flex_grow: 1.0, min_height: px(0), flex_direction: FlexDirection::Row, ..default() }, ChildOf(root)))
        .id();

    // Left: file list.
    let left = commands
        .spawn((
            EditorInternal,
            Node {
                width: px(214),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                border: UiRect::right(px(1)),
                padding: UiRect::all(px(8)),
                row_gap: px(6),
                ..default()
            },
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(body),
        ))
        .id();
    commands
        .spawn((
            EditorInternal,
            ShaderListPanel,
            ScrollPosition::default(),
            Node { flex_grow: 1.0, min_height: px(0), flex_direction: FlexDirection::Column, row_gap: px(2), overflow: Overflow::scroll_y(), ..default() },
            ChildOf(left),
        ))
        .observe(scroll_panel);
    let actions = commands
        .spawn((EditorInternal, Node { flex_wrap: FlexWrap::Wrap, column_gap: px(4), row_gap: px(4), flex_shrink: 0.0, ..default() }, ChildOf(left)))
        .id();
    button(&mut commands, actions, "+ New", ACCENT_CYAN_DIM, Action::TogglePicker);
    button(&mut commands, actions, "⧉ Duplicate", BG_INPUT, Action::Duplicate);
    let delete = button(&mut commands, actions, "", BG_INPUT, Action::Delete);
    commands.spawn((EditorInternal, DeleteLabel, Pickable::IGNORE, Text::new("🗑 Delete"), ui_font(11.5), TextColor(TEXT_PRIMARY), ChildOf(delete)));

    // Center: code editor + status.
    let center = commands
        .spawn((EditorInternal, Node { flex_grow: 1.0, min_width: px(0), flex_direction: FlexDirection::Column, ..default() }, ChildOf(body)))
        .id();
    let code = commands
        .spawn((
            EditorInternal,
            CodeArea,
            Node { flex_grow: 1.0, min_height: px(0), flex_direction: FlexDirection::Column, overflow: Overflow::clip(), ..default() },
            BackgroundColor(BG_SHELL),
            ChildOf(center),
        ))
        .observe(|t: On<PointerOver>, mut state: ResMut<ShaderEditorState>| {
            let _ = t;
            state.code_hovered = true;
        })
        .observe(|t: On<PointerOut>, mut state: ResMut<ShaderEditorState>| {
            let _ = t;
            state.code_hovered = false;
        })
        .observe(|press: On<PointerPress>, mut commands: Commands| {
            if press.button != PointerButton::Primary {
                return;
            }
            let (entity, pos) = (press.entity, press.pointer.position);
            commands.queue(move |world: &mut World| click_code(world, entity, pos));
        })
        .observe(|scroll: On<PointerScroll>, mut state: ResMut<ShaderEditorState>| {
            let lines = match scroll.unit {
                bevy::input::mouse::MouseScrollUnit::Line => scroll.y * 3.0,
                bevy::input::mouse::MouseScrollUnit::Pixel => scroll.y / LINE_H,
            };
            if let Some(doc) = state.doc_mut() {
                let max = doc.buffer.lines.len().saturating_sub(1) as f32;
                doc.scroll = (doc.scroll as f32 - lines).round().clamp(0.0, max) as usize;
            }
        })
        .id();
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Node { position_type: PositionType::Absolute, left: px(0), top: px(0), bottom: px(0), width: px(GUTTER_W), ..default() },
        BackgroundColor(BG_PANEL),
        ChildOf(code),
    ));
    commands.spawn((
        EditorInternal,
        CharMeasure,
        Pickable::IGNORE,
        Visibility::Hidden,
        Node { position_type: PositionType::Absolute, left: px(0), top: px(0), ..default() },
        TextLayout::no_wrap(),
        Text::new("M".repeat(MEASURE_CHARS)),
        code_font(CODE_FONT),
        ChildOf(code),
    ));
    commands.spawn((
        EditorInternal,
        Caret,
        Pickable::IGNORE,
        GlobalZIndex(151),
        Node { position_type: PositionType::Absolute, width: px(2), height: px(LINE_H - 2.0), display: Display::None, ..default() },
        BackgroundColor(ACCENT_CYAN),
        ChildOf(code),
    ));
    let status = commands
        .spawn((
            EditorInternal,
            Node {
                flex_shrink: 0.0,
                min_height: px(44),
                padding: UiRect::axes(px(10), px(5)),
                column_gap: px(12),
                border: UiRect::top(px(1)),
                align_items: AlignItems::FlexStart,
                ..default()
            },
            BackgroundColor(BG_CARD),
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(center),
        ))
        .id();
    commands.spawn((
        EditorInternal,
        StatusText,
        Pickable::IGNORE,
        Node { flex_grow: 1.0, flex_shrink: 1.0, min_width: px(0), ..default() },
        Text::new(""),
        code_font(11.5),
        TextColor(TEXT_SECONDARY),
        ChildOf(status),
    ));
    commands.spawn((EditorInternal, CaretPosText, Pickable::IGNORE, Node { flex_shrink: 0.0, ..default() }, Text::new(""), code_font(11.5), TextColor(TEXT_MUTED), ChildOf(status)));

    // Right: parameters of the selected object.
    commands
        .spawn((
            EditorInternal,
            ParamPanel,
            ScrollPosition::default(),
            Node {
                width: px(282),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                border: UiRect::left(px(1)),
                padding: UiRect::all(px(10)),
                row_gap: px(7),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BorderColor::all(BORDER_SUBTLE),
            ChildOf(body),
        ))
        .observe(scroll_panel);
    world.flush();
}

fn scroll_panel(scroll: On<PointerScroll>, mut q: Query<(&mut ScrollPosition, &ComputedNode)>) {
    if let Ok((mut pos, computed)) = q.get_mut(scroll.entity) {
        let dy = match scroll.unit {
            bevy::input::mouse::MouseScrollUnit::Line => scroll.y * 30.0,
            bevy::input::mouse::MouseScrollUnit::Pixel => scroll.y,
        };
        let max = (computed.content_size().y - computed.size().y).max(0.0) * computed.inverse_scale_factor();
        pos.y = (pos.y - dy).clamp(0.0, max);
    }
}

/// Logical position of `pos` relative to the node's top-left corner.
fn local_position(world: &World, entity: Entity, pos: Vec2) -> Option<(Vec2, Vec2)> {
    let node = world.get::<ComputedNode>(entity)?;
    let xf = world.get::<UiGlobalTransform>(entity)?;
    let inv = node.inverse_scale_factor();
    let size = node.size() * inv;
    let top_left = xf.translation * inv - size / 2.0;
    Some((pos - top_left, size))
}

fn release_other_text_fields(world: &mut World) {
    if let Some(mut edit) = world.get_resource_mut::<crate::ui::inspector::property_editor::PropertyEditState>() {
        edit.cancel();
    }
    if let Some(mut edit) = world.get_resource_mut::<crate::ui::inspector::TransformTextEditState>() {
        edit.active = None;
    }
    if let Some(mut hierarchy) = world.get_resource_mut::<crate::ui::hierarchy::HierarchyState>() {
        hierarchy.search_focused = false;
    }
}

fn click_code(world: &mut World, area: Entity, pos: Vec2) {
    let Some((local, _)) = local_position(world, area, pos) else { return };
    release_other_text_fields(world);
    let mut state = world.resource_mut::<ShaderEditorState>();
    state.focused = true;
    state.blink = 0.0;
    let cw = state.char_width.max(1.0);
    let Some(doc) = state.doc_mut() else { return };
    let line = (local.y / LINE_H).floor().max(0.0) as usize + doc.scroll;
    let col = ((local.x - GUTTER_W - CODE_PAD) / cw).round().max(0.0) as usize + doc.hscroll;
    doc.buffer.set_caret(line, col);
}

fn perform(world: &mut World, action: Action) {
    match action {
        Action::Close => set_open(world, false),
        Action::Save => ops::save_current(world),
        Action::RevertFile => ops::revert_current(world),
        Action::Open(path) => {
            if let Err(e) = ops::open_doc(world, &path) {
                world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Error, e);
            }
        }
        Action::TogglePicker => {
            let mut state = world.resource_mut::<ShaderEditorState>();
            state.template_picker = !state.template_picker;
            state.list_dirty = true;
        }
        Action::NewFromTemplate(template) => {
            world.resource_mut::<ShaderEditorState>().template_picker = false;
            if let Err(e) = ops::create_from_template(world, None, &template, true) {
                world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Error, e);
            }
        }
        Action::Duplicate => {
            if let Err(e) = ops::duplicate_current(world) {
                world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Error, e);
            }
        }
        Action::Delete => {
            let now = world.resource::<Time<Real>>().elapsed_secs_f64();
            let (current, confirmed) = {
                let state = world.resource::<ShaderEditorState>();
                (state.current.clone(), now < state.confirm_delete_until)
            };
            let Some(path) = current else {
                world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Info, "Open a shader to delete");
                return;
            };
            if path.starts_with(super::library::TEMPLATE_DIR) {
                world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Error, "Templates cannot be deleted; use + New to make an editable copy");
            } else if confirmed {
                world.resource_mut::<ShaderEditorState>().confirm_delete_until = 0.0;
                if let Err(e) = ops::delete_shader(world, &path) {
                    world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Error, e);
                }
            } else {
                let mut state = world.resource_mut::<ShaderEditorState>();
                state.confirm_delete_until = now + 3.0;
                state.set_status(StatusKind::Info, format!("Click Delete again within 3 s to delete {path}"));
            }
        }
        Action::ApplySelected => ops::apply_current_to_selected(world),
        Action::RevertSelected => ops::revert_selected(world),
        Action::Step(target, direction) => {
            let Some(entity) = ops::selected_entity(world) else { return };
            let Some(before) = world.get::<ShaderMaterialRef>(entity).cloned() else { return };
            let meta = ops::shader_meta(world, &before.shader);
            let mut after = before.clone();
            match target {
                SliderTarget::Param(i) => {
                    let (min, max, step) = meta.as_ref().map(|m| (m.params[i].min, m.params[i].max, m.params[i].step())).unwrap_or((0.0, 1.0, 0.01));
                    let value = (before.params()[i] + step * direction).clamp(min.min(before.params()[i]), max.max(before.params()[i]));
                    after.set_param(i, (value / step).round() * step);
                }
                SliderTarget::Color(ch) => {
                    let mut c = before.color.to_srgba().to_f32_array();
                    c[ch] = (c[ch] + 0.02 * direction).clamp(0.0, 1.0);
                    after.color = Color::srgba(c[0], c[1], c[2], c[3]);
                }
            }
            if let Err(e) = set_shader_ref_with_undo(world, entity, Some(after), "Edit shader parameter") {
                world.resource_mut::<ShaderEditorState>().set_status(StatusKind::Error, e);
            }
        }
    }
}

fn slider_value(world: &mut World, track: Entity, target: SliderTarget, pos: Vec2, begin: bool) {
    let Some(entity) = ops::selected_entity(world) else { return };
    let Some(current) = world.get::<ShaderMaterialRef>(entity).cloned() else { return };
    let Some((local, size)) = local_position(world, track, pos) else { return };
    let frac = (local.x / size.x.max(1.0)).clamp(0.0, 1.0);
    if begin {
        world.resource_mut::<ShaderEditorState>().drag_before = Some((entity, Some(current.clone())));
    }
    let mut next = current.clone();
    match target {
        SliderTarget::Param(i) => {
            let meta = ops::shader_meta(world, &current.shader);
            let (min, max, step) = meta.as_ref().map(|m| (m.params[i].min, m.params[i].max, m.params[i].step())).unwrap_or((0.0, 1.0, 0.01));
            let value = min + frac * (max - min);
            next.set_param(i, ((value / step).round() * step).clamp(min, max));
        }
        SliderTarget::Color(ch) => {
            let mut c = current.color.to_srgba().to_f32_array();
            c[ch] = (frac * 255.0).round() / 255.0;
            next.color = Color::srgba(c[0], c[1], c[2], c[3]);
        }
    }
    if next != current {
        if let Some(mut r) = world.get_mut::<ShaderMaterialRef>(entity) {
            *r = next;
        }
    }
}

fn slider_commit(world: &mut World) {
    let Some((entity, before)) = world.resource_mut::<ShaderEditorState>().drag_before.take() else { return };
    if let Err(e) = record_shader_ref_edit(world, entity, before, "Edit shader parameter") {
        warn!("{e}");
    }
}

fn spawn_slider_row(commands: &mut Commands, parent: Entity, name: &str, target: SliderTarget) {
    let column = commands.spawn((EditorInternal, Node { flex_direction: FlexDirection::Column, row_gap: px(3), ..default() }, ChildOf(parent))).id();
    let top = commands.spawn((EditorInternal, Node { justify_content: JustifyContent::SpaceBetween, ..default() }, ChildOf(column))).id();
    label(commands, top, name, 11.5, TEXT_SECONDARY);
    commands.spawn((EditorInternal, ParamValueText(target), Pickable::IGNORE, Text::new("—"), code_font(11.5), TextColor(TEXT_PRIMARY), ChildOf(top)));
    let row = commands.spawn((EditorInternal, Node { align_items: AlignItems::Center, column_gap: px(5), ..default() }, ChildOf(column))).id();
    button(commands, row, "−", BG_INPUT, Action::Step(target, -1.0));
    let track = commands
        .spawn((
            EditorInternal,
            Node { flex_grow: 1.0, height: px(10), border_radius: radius_pill(), overflow: Overflow::clip(), ..default() },
            BackgroundColor(BG_INPUT),
            ChildOf(row),
        ))
        .observe(move |mut press: On<PointerPress>, mut commands: Commands| {
            if press.button == PointerButton::Primary {
                press.propagate(false);
                let (track, pos) = (press.entity, press.pointer.position);
                commands.queue(move |world: &mut World| slider_value(world, track, target, pos, true));
            }
        })
        .observe(move |drag: On<PointerDrag>, mut commands: Commands| {
            if drag.button == PointerButton::Primary {
                let (track, pos) = (drag.entity, drag.pointer.position);
                commands.queue(move |world: &mut World| slider_value(world, track, target, pos, false));
            }
        })
        .observe(|_: On<PointerDragEnd>, mut commands: Commands| commands.queue(slider_commit))
        .observe(|_: On<PointerRelease>, mut commands: Commands| commands.queue(slider_commit))
        .id();
    let fill_color = match target {
        SliderTarget::Color(0) => Color::srgb(0.85, 0.3, 0.3),
        SliderTarget::Color(1) => Color::srgb(0.3, 0.75, 0.4),
        SliderTarget::Color(_) => Color::srgb(0.3, 0.5, 0.95),
        SliderTarget::Param(_) => ACCENT_CYAN,
    };
    commands.spawn((
        EditorInternal,
        SliderFill(target),
        Pickable::IGNORE,
        Node { width: percent(0.0), height: percent(100.0), border_radius: radius_pill(), ..default() },
        BackgroundColor(fill_color.with_alpha(0.75)),
        ChildOf(track),
    ));
    button(commands, row, "+", BG_INPUT, Action::Step(target, 1.0));
}

fn section(commands: &mut Commands, parent: Entity, title: &str) {
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Node { margin: UiRect::top(px(4)), ..default() },
        Text::new(title),
        TextFont { font_size: bevy::text::FontSize::Px(10.5), weight: FontWeight::BOLD, ..default() },
        TextColor(TEXT_MUTED),
        ChildOf(parent),
    ));
}

fn rebuild_list(world: &mut World) {
    let Some(panel) = world.query_filtered::<Entity, With<ShaderListPanel>>().iter(world).next() else { return };
    let rows: Vec<(String, String, bool, usize)> = world
        .resource::<ShaderLibrary>()
        .entries
        .iter()
        .map(|e| (e.path.clone(), e.meta.name.clone(), e.template, e.errors.len()))
        .collect();
    let (current, picker, dirty_paths): (Option<String>, bool, Vec<String>) = {
        let state = world.resource::<ShaderEditorState>();
        (state.current.clone(), state.template_picker, state.docs.iter().filter(|(_, d)| d.dirty()).map(|(p, _)| p.clone()).collect())
    };
    world.entity_mut(panel).despawn_related::<Children>();
    let mut commands = world.commands();
    if picker {
        section(&mut commands, panel, "NEW SHADER FROM TEMPLATE");
        for (path, name, _, _) in rows.iter().filter(|r| r.2) {
            button(&mut commands, panel, format!("＋ {name}"), ACCENT_CYAN_DIM, Action::NewFromTemplate(path.clone()));
        }
        button(&mut commands, panel, "Cancel", BG_INPUT, Action::TogglePicker);
    }
    section(&mut commands, panel, "SHADERS  (assets/shaders)");
    if !rows.iter().any(|r| !r.2) {
        label(&mut commands, panel, "None yet — click + New to copy a template.", 10.5, TEXT_MUTED);
    }
    let mut in_templates = false;
    for (path, name, template, errors) in rows {
        if template && !in_templates {
            in_templates = true;
            section(&mut commands, panel, "TEMPLATES  (shaders/templates)");
        }
        let selected = current.as_deref() == Some(path.as_str());
        let base = if selected { ACCENT_CYAN_DIM } else { BG_PANEL };
        let row = commands
            .spawn((
                EditorInternal,
                ButtonBase(base),
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(px(8), px(4)),
                    border_radius: radius_button(),
                    border: UiRect::left(px(2)),
                    ..default()
                },
                BackgroundColor(base),
                BorderColor::all(if selected { ACCENT_CYAN } else { Color::NONE }),
                ChildOf(panel),
            ))
            .observe(|t: On<PointerOver>, mut q: Query<&mut BackgroundColor>| {
                if let Ok(mut bg) = q.get_mut(t.entity) {
                    *bg = BackgroundColor(BG_HOVER);
                }
            })
            .observe(|t: On<PointerOut>, mut q: Query<(&mut BackgroundColor, &ButtonBase)>| {
                if let Ok((mut bg, base)) = q.get_mut(t.entity) {
                    *bg = BackgroundColor(base.0);
                }
            })
            .observe({
                let path = path.clone();
                move |click: On<PointerClick>, mut commands: Commands| {
                    if click.button == PointerButton::Primary {
                        let path = path.clone();
                        commands.queue(move |world: &mut World| perform(world, Action::Open(path)));
                    }
                }
            })
            .id();
        let top = commands.spawn((EditorInternal, Pickable::IGNORE, Node { column_gap: px(6), align_items: AlignItems::Center, ..default() }, ChildOf(row))).id();
        let dirty = dirty_paths.contains(&path);
        label(&mut commands, top, format!("{name}{}", if dirty { " ●" } else { "" }), 12.0, if selected { ACCENT_CYAN } else { TEXT_PRIMARY });
        if errors > 0 {
            label(&mut commands, top, format!("⚠ {errors}"), 10.5, ACCENT_CRIMSON);
        }
        let file = path.rsplit('/').next().unwrap_or(&path).to_string();
        label(&mut commands, row, file, 10.0, TEXT_MUTED);
    }
    world.flush();
}

/// Key identifying what the parameter panel shows; a change triggers a rebuild.
#[derive(Default, PartialEq, Clone)]
pub struct ParamPanelKey {
    entity: Option<Entity>,
    has_mesh: bool,
    shader: Option<String>,
    revision: u64,
    current: Option<String>,
}

fn panel_key(world: &mut World) -> ParamPanelKey {
    let entity = ops::selected_entity(world);
    let shader = entity.and_then(|e| world.get::<ShaderMaterialRef>(e)).map(|r| r.shader.clone());
    let revision = shader.as_ref().and_then(|s| world.resource::<ShaderLibrary>().get(s)).map(|e| e.revision).unwrap_or(0);
    ParamPanelKey {
        entity,
        has_mesh: entity.is_some_and(|e| world.get::<Mesh3d>(e).is_some()),
        shader,
        revision,
        current: world.resource::<ShaderEditorState>().current.clone(),
    }
}

fn rebuild_params(world: &mut World, key: &ParamPanelKey) {
    let Some(panel) = world.query_filtered::<Entity, With<ParamPanel>>().iter(world).next() else { return };
    let name = key.entity.and_then(|e| world.get::<Name>(e)).map(|n| n.as_str().to_string()).unwrap_or_else(|| "Object".into());
    let meta: Option<ShaderMeta> = key.shader.clone().and_then(|s| ops::shader_meta(world, &s));
    let current_name = key.current.as_ref().and_then(|p| world.resource::<ShaderLibrary>().get(p)).map(|e| e.meta.name.clone());
    world.entity_mut(panel).despawn_related::<Children>();
    let mut commands = world.commands();
    section(&mut commands, panel, "MATERIAL OF SELECTION");
    let Some(_entity) = key.entity else {
        label(&mut commands, panel, "Select an object in the viewport or hierarchy to apply a shader and tune its parameters.", 11.5, TEXT_SECONDARY);
        world.flush();
        return;
    };
    label(&mut commands, panel, name, 13.0, TEXT_PRIMARY);
    if !key.has_mesh {
        label(&mut commands, panel, "This object has no mesh.", 11.5, TEXT_SECONDARY);
        world.flush();
        return;
    }
    let actions = commands.spawn((EditorInternal, Node { flex_wrap: FlexWrap::Wrap, column_gap: px(5), row_gap: px(5), ..default() }, ChildOf(panel))).id();
    let apply_label = match &current_name {
        Some(n) => format!("▶ Apply \"{n}\""),
        None => "▶ Apply shader".into(),
    };
    button(&mut commands, actions, apply_label, ACCENT_CYAN_DIM, Action::ApplySelected);
    if key.shader.is_some() {
        button(&mut commands, actions, "⟲ Revert to Standard", BG_INPUT, Action::RevertSelected);
    }
    let Some(meta) = meta else {
        label(&mut commands, panel, "Uses a standard material. Open a shader on the left and press Apply.", 11.5, TEXT_SECONDARY);
        world.flush();
        return;
    };
    section(&mut commands, panel, "SHADER");
    label(&mut commands, panel, format!("{}  ·  {}", meta.name, meta.alpha.label()), 12.0, ACCENT_CYAN);
    label(&mut commands, panel, key.shader.clone().unwrap_or_default(), 10.5, TEXT_MUTED);
    if !meta.description.is_empty() {
        label(&mut commands, panel, meta.description.clone(), 11.0, TEXT_SECONDARY);
    }
    section(&mut commands, panel, &meta.color_label.to_uppercase());
    let swatch_row = commands.spawn((EditorInternal, Node { align_items: AlignItems::Center, column_gap: px(8), ..default() }, ChildOf(panel))).id();
    commands.spawn((
        EditorInternal,
        ColorSwatch,
        Pickable::IGNORE,
        Node { width: px(36), height: px(20), border_radius: radius_button(), border: UiRect::all(px(1)), ..default() },
        BackgroundColor(Color::WHITE),
        BorderColor::all(BORDER_CARD),
        ChildOf(swatch_row),
    ));
    commands.spawn((EditorInternal, ColorHexText, Pickable::IGNORE, Text::new(""), code_font(12.0), TextColor(TEXT_PRIMARY), ChildOf(swatch_row)));
    for (ch, n) in ["Red", "Green", "Blue"].iter().enumerate() {
        spawn_slider_row(&mut commands, panel, n, SliderTarget::Color(ch));
    }
    section(&mut commands, panel, "PARAMETERS");
    let any_declared = meta.declared.iter().any(|d| *d);
    for i in 0..4 {
        if any_declared && !meta.declared[i] {
            continue;
        }
        let p = &meta.params[i];
        spawn_slider_row(&mut commands, panel, &format!("{}  (p{i})", p.label), SliderTarget::Param(i));
    }
    label(&mut commands, panel, "Drag sliders or use −/+ · every change is undoable (Ctrl+Z). Values are saved with the scene.", 10.5, TEXT_MUTED);
    world.flush();
}

fn update_param_values(world: &mut World) {
    let Some(entity) = ops::selected_entity(world) else { return };
    let Some(shader_ref) = world.get::<ShaderMaterialRef>(entity).cloned() else { return };
    let meta = world.resource::<ShaderLibrary>().get(&shader_ref.shader).map(|e| e.meta.clone());
    let srgb = shader_ref.color.to_srgba().to_f32_array();
    let value_of = |target: SliderTarget| -> (String, f32) {
        match target {
            SliderTarget::Param(i) => {
                let v = shader_ref.params()[i];
                let (min, max) = meta.as_ref().map(|m| (m.params[i].min, m.params[i].max)).unwrap_or((0.0, 1.0));
                (format!("{v:.3}"), ((v - min) / (max - min).max(1e-6)).clamp(0.0, 1.0))
            }
            SliderTarget::Color(ch) => (format!("{:.0}", srgb[ch] * 255.0), srgb[ch]),
        }
    };
    let mut texts = world.query::<(&ParamValueText, &mut Text)>();
    for (target, mut text) in texts.iter_mut(world) {
        let (s, _) = value_of(target.0);
        if text.0 != s {
            text.0 = s;
        }
    }
    let mut fills = world.query::<(&SliderFill, &mut Node)>();
    for (target, mut node) in fills.iter_mut(world) {
        let (_, f) = value_of(target.0);
        let w = percent(f * 100.0);
        if node.width != w {
            node.width = w;
        }
    }
    let hex = crate::core::reflect_fields::color_hex(shader_ref.color);
    let mut swatches = world.query_filtered::<&mut BackgroundColor, With<ColorSwatch>>();
    for mut bg in swatches.iter_mut(world) {
        if bg.0 != shader_ref.color {
            bg.0 = shader_ref.color;
        }
    }
    let mut hex_texts = world.query_filtered::<&mut Text, With<ColorHexText>>();
    for mut text in hex_texts.iter_mut(world) {
        if text.0 != hex {
            text.0 = hex.clone();
        }
    }
}

const KEYWORDS: &[&str] = &[
    "fn", "let", "var", "const", "return", "if", "else", "for", "while", "loop", "break", "continue", "struct", "discard", "switch",
    "case", "default", "true", "false", "override", "alias",
];
const BUILTINS: &[&str] = &[
    "in", "material", "p0", "p1", "p2", "p3", "lambert", "fresnel", "hash11", "hash21", "noise2", "fbm2", "hue_shift", "PI", "LIGHT_DIR",
    "ShaderInput", "camera_position", "shade", "sin", "cos", "tan", "mix", "clamp", "smoothstep", "step", "fract", "floor", "ceil",
    "abs", "min", "max", "pow", "exp", "sqrt", "length", "distance", "normalize", "dot", "cross", "reflect", "saturate", "select",
    "atan2", "sign", "round", "textureSample",
];

fn is_type(word: &str) -> bool {
    matches!(word, "f32" | "i32" | "u32" | "bool" | "f16")
        || ["vec2", "vec3", "vec4", "mat2x2", "mat3x3", "mat4x4", "array"].iter().any(|t| word.starts_with(t))
}

/// Splits a line into colored segments (comments, keywords, types, numbers, built-ins).
fn highlight(line: &str) -> Vec<(String, Color)> {
    let mut out: Vec<(String, Color)> = Vec::new();
    let push = |out: &mut Vec<(String, Color)>, text: String, color: Color| {
        if let Some(last) = out.last_mut() {
            if last.1 == color {
                last.0.push_str(&text);
                return;
            }
        }
        out.push((text, color));
    };
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            let rest: String = chars[i..].iter().collect();
            let color = if rest.starts_with("//!") { COLOR_META } else { TEXT_MUTED };
            push(&mut out, rest, color);
            break;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit())) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.') {
                i += 1;
            }
            push(&mut out, chars[start..i].iter().collect(), COLOR_NUMBER);
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let color = if KEYWORDS.contains(&word.as_str()) {
                COLOR_KEYWORD
            } else if is_type(&word) {
                COLOR_TYPE
            } else if BUILTINS.contains(&word.as_str()) {
                COLOR_BUILTIN
            } else {
                TEXT_PRIMARY
            };
            push(&mut out, word, color);
            continue;
        }
        let color = if c == '@' { COLOR_KEYWORD } else { TEXT_SECONDARY };
        push(&mut out, c.to_string(), color);
        i += 1;
    }
    out
}

/// Per-frame window refresh: rows, caret, status, list and parameter panel.
pub fn update_shader_editor_ui(world: &mut World, mut last_key: Local<ParamPanelKey>) {
    let open = world.resource::<ShaderEditorState>().open;
    let Some(area) = world.query_filtered::<Entity, With<CodeArea>>().iter(world).next() else { return };
    if !open {
        return;
    }
    let dt = world.resource::<Time<Real>>().delta_secs();

    // Lose focus when clicking anywhere outside the code area.
    let pressed = world.resource::<ButtonInput<MouseButton>>().get_just_pressed().next().is_some();
    {
        let mut state = world.resource_mut::<ShaderEditorState>();
        if pressed && !state.code_hovered {
            state.focused = false;
        }
        state.blink = (state.blink + dt) % 1.0;
    }

    // Measure the monospace advance once layout is known.
    let measured = world
        .query_filtered::<&ComputedNode, With<CharMeasure>>()
        .iter(world)
        .next()
        .map(|n| n.size().x * n.inverse_scale_factor() / MEASURE_CHARS as f32)
        .filter(|w| *w > 1.0);
    if let Some(w) = measured {
        world.resource_mut::<ShaderEditorState>().char_width = w;
    }
    let (area_size, _) = world
        .get::<ComputedNode>(area)
        .map(|n| (n.size() * n.inverse_scale_factor(), ()))
        .unwrap_or((Vec2::ZERO, ()));

    // Live validation (debounced) and caret auto-scroll.
    let (rows_wanted, cols) = {
        let mut state = world.resource_mut::<ShaderEditorState>();
        let cw = if state.char_width > 1.0 { state.char_width } else { CODE_FONT * 0.6 };
        let rows = ((area_size.y / LINE_H).floor() as usize).max(1);
        let cols = (((area_size.x - GUTTER_W - CODE_PAD * 2.0) / cw).floor() as usize).max(8);
        state.visible_rows = rows;
        state.visible_cols = cols;
        let ensure = std::mem::take(&mut state.ensure_caret_visible);
        if let Some(doc) = state.doc_mut() {
            if let Some(t) = doc.check_in.as_mut() {
                *t -= dt;
                if *t <= 0.0 {
                    doc.check_in = None;
                    doc.errors = super::library::validate(&doc.buffer.text()).err().unwrap_or_default();
                }
            }
            if ensure {
                if doc.buffer.line < doc.scroll {
                    doc.scroll = doc.buffer.line;
                } else if doc.buffer.line >= doc.scroll + rows {
                    doc.scroll = doc.buffer.line + 1 - rows;
                }
                if doc.buffer.col < doc.hscroll {
                    doc.hscroll = doc.buffer.col.saturating_sub(4);
                } else if doc.buffer.col + 2 > doc.hscroll + cols {
                    doc.hscroll = doc.buffer.col + 2 - cols;
                }
            }
        }
        (if area_size.y > 0.0 { rows } else { 0 }, cols)
    };

    // Keep exactly `rows_wanted` row entities.
    let mut rows: Vec<(Entity, usize)> = world.query::<(Entity, &CodeRow)>().iter(world).map(|(e, r)| (e, r.0)).collect();
    rows.sort_by_key(|r| r.1);
    if rows.len() > rows_wanted {
        for (e, _) in rows.drain(rows_wanted..) {
            world.despawn(e);
        }
    }
    for index in rows.len()..rows_wanted {
        let row = world
            .spawn((
                EditorInternal,
                CodeRow(index),
                Pickable::IGNORE,
                Node { height: px(LINE_H), flex_shrink: 0.0, align_items: AlignItems::Center, ..default() },
                BackgroundColor(Color::NONE),
                ChildOf(area),
            ))
            .id();
        world.spawn((
            EditorInternal,
            RowGutter,
            Pickable::IGNORE,
            Node { width: px(GUTTER_W - 8.0), margin: UiRect::right(px(8.0 + CODE_PAD)), justify_content: JustifyContent::FlexEnd, flex_shrink: 0.0, ..default() },
            TextLayout { justify: Justify::Right, linebreak: LineBreak::NoWrap },
            Text::new(""),
            code_font(11.5),
            TextColor(TEXT_MUTED),
            ChildOf(row),
        ));
        world.spawn((EditorInternal, RowText::default(), Pickable::IGNORE, TextLayout::no_wrap(), Text::new(""), code_font(CODE_FONT), TextColor(TEXT_PRIMARY), ChildOf(row)));
        rows.push((row, index));
    }

    // Snapshot what to draw.
    let (lines, scroll, hscroll, caret, error_lines, focused, blink, cw, title, status, status_kind, caret_text) = {
        let state = world.resource::<ShaderEditorState>();
        let cw = if state.char_width > 1.0 { state.char_width } else { CODE_FONT * 0.6 };
        match state.doc() {
            Some(doc) => {
                let errors: Vec<u32> = doc.errors.iter().map(|e| e.line).collect();
                let visible: Vec<Option<String>> = (0..rows_wanted)
                    .map(|i| doc.buffer.lines.get(doc.scroll + i).map(|l| l.chars().skip(doc.hscroll).take(cols + 1).collect()))
                    .collect();
                let path = state.current.clone().unwrap_or_default();
                let title = format!("{path}{}", if doc.dirty() { "  ● unsaved" } else { "" });
                let (status, kind) = if !doc.errors.is_empty() {
                    let mut text = format!("✖ {} error(s){}:", doc.errors.len(), if doc.dirty() { " in unsaved edits" } else { "; the last good shader stays active" });
                    for e in doc.errors.iter().take(3) {
                        text.push_str("\n  ");
                        text.push_str(&e.short());
                    }
                    (text, StatusKind::Error)
                } else {
                    (state.status.clone(), state.status_kind)
                };
                let caret_text = format!("Ln {}, Col {}  ·  {} lines", doc.buffer.line + 1, doc.buffer.col + 1, doc.buffer.lines.len());
                (visible, doc.scroll, doc.hscroll, Some((doc.buffer.line, doc.buffer.col)), errors, state.focused, state.blink, cw, title, status, kind, caret_text)
            }
            None => (
                vec![None; rows_wanted],
                0,
                0,
                None,
                Vec::new(),
                false,
                0.0,
                cw,
                "no file open".to_string(),
                if state.status.is_empty() { "Pick a shader on the left, or + New to create one from a template.".to_string() } else { state.status.clone() },
                state.status_kind,
                String::new(),
            ),
        }
    };

    for (row, index) in rows.iter().copied() {
        let content = lines.get(index).cloned().flatten();
        let number = scroll + index + 1;
        let is_error = content.is_some() && error_lines.contains(&(number as u32));
        let is_current = focused && caret.is_some_and(|(l, _)| l + 1 == number);
        let bg = if is_error { ERROR_ROW } else if is_current { CURRENT_ROW } else { Color::NONE };
        if let Some(mut b) = world.get_mut::<BackgroundColor>(row) {
            if b.0 != bg {
                b.0 = bg;
            }
        }
        let children: Vec<Entity> = world.get::<Children>(row).map(|c| c.iter().collect()).unwrap_or_default();
        for child in children {
            if world.get::<RowGutter>(child).is_some() {
                let text = if content.is_some() { format!("{number:>4}") } else { String::new() };
                if let Some(mut t) = world.get_mut::<Text>(child) {
                    if t.0 != text {
                        t.0 = text;
                    }
                }
                let color = if is_error { ACCENT_CRIMSON } else if is_current { TEXT_PRIMARY } else { TEXT_MUTED };
                if let Some(mut c) = world.get_mut::<TextColor>(child) {
                    if c.0 != color {
                        c.0 = color;
                    }
                }
            } else if let Some(cached) = world.get::<RowText>(child).map(|r| r.cached.clone()) {
                if cached != content {
                    world.entity_mut(child).despawn_related::<Children>();
                    if let Some(text) = &content {
                        for (segment, color) in highlight(text) {
                            world.spawn((EditorInternal, TextSpan::new(segment), code_font(CODE_FONT), TextColor(color), ChildOf(child)));
                        }
                    }
                    if let Some(mut r) = world.get_mut::<RowText>(child) {
                        r.cached = content.clone();
                    }
                }
            }
        }
    }

    // Caret.
    let caret_node = caret.filter(|(l, c)| focused && *l >= scroll && *l < scroll + rows_wanted && *c >= hscroll).map(|(l, c)| {
        (px(GUTTER_W + CODE_PAD + (c - hscroll) as f32 * cw - 1.0), px((l - scroll) as f32 * LINE_H + 1.0))
    });
    let mut carets = world.query_filtered::<&mut Node, With<Caret>>();
    for mut node in carets.iter_mut(world) {
        match caret_node {
            Some((left, top)) if blink < 0.6 => {
                node.display = Display::Flex;
                node.left = left;
                node.top = top;
            }
            _ => node.display = Display::None,
        }
    }

    let set_text = |world: &mut World, text: String, color: Option<Color>, which: u8| {
        let mut q = world.query::<(&mut Text, &mut TextColor, Option<&TitleText>, Option<&StatusText>, Option<&CaretPosText>)>();
        for (mut t, mut c, title, status, caret) in q.iter_mut(world) {
            let hit = match which {
                0 => title.is_some(),
                1 => status.is_some(),
                _ => caret.is_some(),
            };
            if hit {
                if t.0 != text {
                    t.0 = text.clone();
                }
                if let Some(color) = color {
                    if c.0 != color {
                        c.0 = color;
                    }
                }
            }
        }
    };
    set_text(world, title, None, 0);
    let status_color = match status_kind {
        StatusKind::Error => ACCENT_CRIMSON,
        StatusKind::Ok => ACCENT_EMERALD,
        StatusKind::Info => TEXT_SECONDARY,
    };
    set_text(world, status, Some(status_color), 1);
    set_text(world, caret_text, None, 2);

    // Delete button label reflects the confirmation step.
    let now = world.resource::<Time<Real>>().elapsed_secs_f64();
    let confirm = now < world.resource::<ShaderEditorState>().confirm_delete_until;
    let mut deletes = world.query_filtered::<(&mut Text, &mut TextColor), With<DeleteLabel>>();
    for (mut t, mut c) in deletes.iter_mut(world) {
        let want = if confirm { "Confirm delete?" } else { "🗑 Delete" };
        if t.0 != want {
            t.0 = want.into();
            c.0 = if confirm { ACCENT_CRIMSON } else { TEXT_PRIMARY };
        }
    }

    if std::mem::take(&mut world.resource_mut::<ShaderEditorState>().list_dirty) {
        rebuild_list(world);
    }
    let key = panel_key(world);
    let forced = std::mem::take(&mut world.resource_mut::<ShaderEditorState>().params_dirty);
    if forced || key != *last_key {
        rebuild_params(world, &key);
        *last_key = key;
    }
    update_param_values(world);
}

#[derive(Default)]
pub struct Modifiers {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

/// Code editor keyboard input. Runs right after Bevy's input systems; while the code area has
/// focus it consumes the frame's keyboard input so viewport/tool shortcuts and scene undo
/// do not fire while typing.
pub fn handle_code_keyboard(world: &mut World, mut cursor: Local<MessageCursor<KeyboardInput>>, mut mods: Local<Modifiers>) {
    let events: Vec<KeyboardInput> = cursor.read(world.resource::<Messages<KeyboardInput>>()).cloned().collect();
    let primary = world.query_filtered::<(Entity, &Window), With<PrimaryWindow>>().iter(world).find(|(_, w)| w.focused).map(|(e, _)| e);
    if primary.is_none() {
        *mods = Modifiers::default();
    }
    let active = {
        let state = world.resource::<ShaderEditorState>();
        state.open && state.focused && state.current.is_some()
    };
    let mut save = false;
    let mut handled = false;
    let was_dirty = world.resource::<ShaderEditorState>().doc().is_some_and(|d| d.dirty());
    for event in events.iter().filter(|e| Some(e.window) == primary) {
        let pressed = event.state.is_pressed();
        match event.key_code {
            KeyCode::ControlLeft | KeyCode::ControlRight => mods.ctrl = pressed,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => mods.shift = pressed,
            KeyCode::AltLeft | KeyCode::AltRight => mods.alt = pressed,
            _ => {}
        }
        if !active || !pressed {
            continue;
        }
        handled = true;
        let mut state = world.resource_mut::<ShaderEditorState>();
        let page = state.visible_rows.max(2) - 1;
        state.blink = 0.0;
        if event.key_code == KeyCode::Escape {
            state.focused = false;
            continue;
        }
        let Some(doc) = state.doc_mut() else { continue };
        let b = &mut doc.buffer;
        let mut edited = true;
        let ctrl = mods.ctrl && !mods.alt;
        match event.key_code {
            KeyCode::KeyS if ctrl => {
                save = true;
                edited = false;
            }
            KeyCode::KeyZ if ctrl && mods.shift => {
                b.redo();
            }
            KeyCode::KeyZ if ctrl => {
                b.undo();
            }
            KeyCode::KeyY if ctrl => {
                b.redo();
            }
            KeyCode::KeyD if ctrl => b.duplicate_line(),
            KeyCode::KeyK if ctrl && mods.shift => b.delete_line(),
            KeyCode::Enter | KeyCode::NumpadEnter => b.newline(),
            KeyCode::Backspace => b.backspace(),
            KeyCode::Delete => b.delete(),
            KeyCode::Tab if mods.shift => b.outdent(),
            KeyCode::Tab => b.tab(),
            KeyCode::ArrowLeft if ctrl => {
                b.word_left();
                edited = false;
            }
            KeyCode::ArrowRight if ctrl => {
                b.word_right();
                edited = false;
            }
            KeyCode::ArrowLeft => {
                b.left();
                edited = false;
            }
            KeyCode::ArrowRight => {
                b.right();
                edited = false;
            }
            KeyCode::ArrowUp => {
                b.vertical(-1);
                edited = false;
            }
            KeyCode::ArrowDown => {
                b.vertical(1);
                edited = false;
            }
            KeyCode::PageUp => {
                b.vertical(-(page as isize));
                edited = false;
            }
            KeyCode::PageDown => {
                b.vertical(page as isize);
                edited = false;
            }
            KeyCode::Home if ctrl => {
                b.set_caret(0, 0);
                edited = false;
            }
            KeyCode::End if ctrl => {
                let last = b.lines.len() - 1;
                let len = b.line_len(last);
                b.set_caret(last, len);
                edited = false;
            }
            KeyCode::Home => {
                b.home();
                edited = false;
            }
            KeyCode::End => {
                b.end();
                edited = false;
            }
            _ => {
                if ctrl {
                    edited = false;
                } else {
                    let text = match (&event.text, &event.logical_key) {
                        (Some(t), _) if t.chars().all(|c| !c.is_control()) => Some(t.to_string()),
                        (_, Key::Character(t)) => Some(t.to_string()),
                        (_, Key::Space) => Some(" ".to_string()),
                        _ => None,
                    };
                    match text {
                        Some(t) if !t.is_empty() => b.insert_str(&t),
                        _ => edited = false,
                    }
                }
            }
        }
        if edited {
            doc.check_in = Some(0.35);
        }
        state.ensure_caret_visible = true;
    }
    if active && handled {
        world.resource_mut::<Messages<KeyboardInput>>().clear();
        world.resource_mut::<ButtonInput<KeyCode>>().reset_all();
    } else if active {
        // Swallow held keys too (e.g. W held while typing must not fly the camera).
        world.resource_mut::<ButtonInput<KeyCode>>().reset_all();
    }
    if save {
        ops::save_current(world);
    }
    if handled && world.resource::<ShaderEditorState>().doc().is_some_and(|d| d.dirty()) != was_dirty {
        world.resource_mut::<ShaderEditorState>().list_dirty = true;
    }
}

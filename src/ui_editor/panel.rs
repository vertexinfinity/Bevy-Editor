//! The UI Editor tool window: element palette, game UI tree and the property sheet.
use bevy::{
    ecs::message::MessageCursor,
    input::keyboard::{Key, KeyboardInput},
    inspector::InspectorSelection,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
    window::PrimaryWindow,
};

use super::model::{self, UiElementKind};
use super::{canvas, UiEditorState};
use crate::core::play_mode::PlayState;
use crate::scene::EditorInternal;
use crate::ui::theme::*;

#[derive(Component)]
pub struct UiEditorWindow;
#[derive(Component)]
struct TreeList;
#[derive(Component)]
struct PropsList;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct GameUiToggleText;

/// Key of the panel-local image path used when adding an Image element.
pub const PALETTE_IMAGE: &str = "palette_image";

/// Click-to-edit text box bound to a property of the selected element.
#[derive(Component, Clone, Copy)]
struct EdField {
    key: &'static str,
}
#[derive(Component)]
struct EdFieldText;
/// Button that sets `key` to `value`; highlighted while it is the current value.
#[derive(Component, Clone, Copy)]
struct EdChip {
    key: &'static str,
    value: &'static str,
}
#[derive(Component, Clone, Copy)]
struct EdSwatch {
    key: &'static str,
}
#[derive(Component, Clone, Copy)]
struct EdBase(Color);

/// Active text entry in the panel.
pub struct TextEdit {
    field: Entity,
    key: &'static str,
    target: Option<Entity>,
    buffer: String,
    replace: bool,
}

#[derive(Clone, Copy, Debug)]
enum PanelAction {
    Close,
    ToggleGameUi,
    Add(UiElementKind),
    MoveUp,
    MoveDown,
    Duplicate,
    Delete,
    ToRoot,
    Deselect,
    SelectRow(Entity),
    Chip(&'static str, &'static str),
    BeginEdit,
}

const FONT: f32 = 12.0;

fn text(world: &mut World, parent: Entity, s: &str, size: f32, color: Color) -> Entity {
    world
        .spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(s.to_string()),
            TextFont { font_size: bevy::text::FontSize::Px(size), ..default() },
            TextColor(color),
            TextLayout::no_wrap(),
            ChildOf(parent),
        ))
        .id()
}

fn container(world: &mut World, parent: Entity, node: Node) -> Entity {
    world.spawn((EditorInternal, node, ChildOf(parent))).id()
}

fn clickable(world: &mut World, parent: Entity, action: PanelAction, node: Node, bg: Color) -> Entity {
    world
        .spawn((
            EditorInternal,
            Node { border: UiRect::all(Val::Px(1.0)), border_radius: radius_button(), ..node },
            BackgroundColor(bg),
            EdBase(bg),
            BorderColor::all(Color::NONE),
            ChildOf(parent),
        ))
        .observe(move |mut click: On<PointerClick>, mut commands: Commands| {
            if click.button == PointerButton::Primary {
                click.propagate(false);
                let field = click.entity;
                commands.queue(move |world: &mut World| run_action(world, action, field));
            }
        })
        .observe(|over: On<PointerOver>, mut q: Query<&mut BorderColor>| {
            if let Ok(mut b) = q.get_mut(over.entity) {
                *b = BorderColor::all(BORDER_ACCENT);
            }
        })
        .observe(|out: On<PointerOut>, mut q: Query<&mut BorderColor>| {
            if let Ok(mut b) = q.get_mut(out.entity) {
                *b = BorderColor::all(Color::NONE);
            }
        })
        .id()
}

fn button(world: &mut World, parent: Entity, label: &str, action: PanelAction, grow: bool) -> Entity {
    let b = clickable(
        world,
        parent,
        action,
        Node {
            padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_grow: if grow { 1.0 } else { 0.0 },
            ..default()
        },
        BG_CARD,
    );
    text(world, b, label, FONT, TEXT_PRIMARY);
    b
}

fn section(world: &mut World, parent: Entity, title: &str) {
    let row = container(world, parent, Node { margin: UiRect::top(Val::Px(4.0)), flex_shrink: 0.0, ..default() });
    text(world, row, title, 10.5, TEXT_MUTED);
}

fn label_cell(world: &mut World, row: Entity, label: &str, width: f32) {
    let cell = container(world, row, Node { width: Val::Px(width), flex_shrink: 0.0, ..default() });
    text(world, cell, label, FONT, TEXT_SECONDARY);
}

fn field(world: &mut World, row: Entity, key: &'static str) -> Entity {
    let f = clickable(
        world,
        row,
        PanelAction::BeginEdit,
        Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            min_width: Val::Px(0.0),
            height: Val::Px(24.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            ..default()
        },
        BG_INPUT,
    );
    world.entity_mut(f).insert(EdField { key });
    let t = text(world, f, "—", FONT, TEXT_PRIMARY);
    world.entity_mut(t).insert(EdFieldText);
    f
}

fn row(world: &mut World, parent: Entity) -> Entity {
    container(world, parent, Node { align_items: AlignItems::Center, column_gap: Val::Px(6.0), min_height: Val::Px(26.0), flex_shrink: 0.0, ..default() })
}

fn field_row(world: &mut World, parent: Entity, label: &str, key: &'static str) {
    let r = row(world, parent);
    label_cell(world, r, label, 84.0);
    field(world, r, key);
}

fn pair_row(world: &mut World, parent: Entity, label: &str, a: (&str, &'static str), b: (&str, &'static str)) {
    let r = row(world, parent);
    label_cell(world, r, label, 84.0);
    text(world, r, a.0, 11.0, TEXT_MUTED);
    field(world, r, a.1);
    text(world, r, b.0, 11.0, TEXT_MUTED);
    field(world, r, b.1);
}

fn color_row(world: &mut World, parent: Entity, label: &str, key: &'static str) {
    let r = row(world, parent);
    label_cell(world, r, label, 84.0);
    world.spawn((
        EditorInternal,
        EdSwatch { key },
        Pickable::IGNORE,
        Node { width: Val::Px(18.0), height: Val::Px(18.0), border: UiRect::all(Val::Px(1.0)), border_radius: radius_pill(), flex_shrink: 0.0, ..default() },
        BackgroundColor(Color::NONE),
        BorderColor::all(BORDER_CARD),
        ChildOf(r),
    ));
    field(world, r, key);
}

fn chips_row(world: &mut World, parent: Entity, label: &str, key: &'static str, options: &[(&str, &'static str)]) {
    let r = row(world, parent);
    label_cell(world, r, label, 84.0);
    let wrap = container(world, r, Node { flex_wrap: FlexWrap::Wrap, flex_grow: 1.0, flex_basis: Val::Px(0.0), column_gap: Val::Px(3.0), row_gap: Val::Px(3.0), ..default() });
    for (display, value) in options {
        let c = clickable(
            world,
            wrap,
            PanelAction::Chip(key, value),
            Node { padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)), ..default() },
            BG_CARD,
        );
        world.entity_mut(c).insert(EdChip { key, value });
        text(world, c, display, 11.0, TEXT_PRIMARY);
    }
}

fn anchor_grid(world: &mut World, parent: Entity) {
    let r = row(world, parent);
    label_cell(world, r, "Anchor", 84.0);
    let grid = container(world, r, Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(2.0), ..default() });
    let names = [["top_left", "top", "top_right"], ["left", "center", "right"], ["bottom_left", "bottom", "bottom_right"]];
    for line in names {
        let lr = container(world, grid, Node { column_gap: Val::Px(2.0), ..default() });
        for name in line {
            let c = clickable(
                world,
                lr,
                PanelAction::Chip("anchor", name),
                Node { width: Val::Px(22.0), height: Val::Px(16.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                BG_INPUT,
            );
            world.entity_mut(c).insert(EdChip { key: "anchor", value: name });
            // A dot placed where the anchor sits inside the cell.
            let (jx, jy) = match name {
                "top_left" => (JustifyContent::FlexStart, AlignItems::FlexStart),
                "top" => (JustifyContent::Center, AlignItems::FlexStart),
                "top_right" => (JustifyContent::FlexEnd, AlignItems::FlexStart),
                "left" => (JustifyContent::FlexStart, AlignItems::Center),
                "center" => (JustifyContent::Center, AlignItems::Center),
                "right" => (JustifyContent::FlexEnd, AlignItems::Center),
                "bottom_left" => (JustifyContent::FlexStart, AlignItems::FlexEnd),
                "bottom" => (JustifyContent::Center, AlignItems::FlexEnd),
                _ => (JustifyContent::FlexEnd, AlignItems::FlexEnd),
            };
            if let Some(mut n) = world.get_mut::<Node>(c) {
                n.justify_content = jx;
                n.align_items = jy;
                n.padding = UiRect::all(Val::Px(2.0));
            }
            world.spawn((
                EditorInternal,
                Pickable::IGNORE,
                Node { width: Val::Px(6.0), height: Val::Px(6.0), border_radius: BorderRadius::all(Val::Px(3.0)), ..default() },
                BackgroundColor(TEXT_SECONDARY),
                ChildOf(c),
            ));
        }
    }
    let side = container(world, r, Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(3.0), ..default() });
    for (display, value) in [("Stretch", "stretch"), ("Flow (in layout)", "none")] {
        let c = clickable(world, side, PanelAction::Chip("anchor", value), Node { padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)), ..default() }, BG_CARD);
        world.entity_mut(c).insert(EdChip { key: "anchor", value });
        text(world, c, display, 11.0, TEXT_PRIMARY);
    }
}

pub fn spawn_window(world: &mut World) -> Entity {
    let root = world
        .spawn((
            EditorInternal,
            UiEditorWindow,
            Name::new("UiEditorWindow"),
            GlobalZIndex(140),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Px(70.0),
                bottom: Val::Px(34.0),
                width: Val::Px(372.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(5.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: radius_panel(),
                ..default()
            },
            BackgroundColor(BG_PANEL),
            BorderColor::all(BORDER_CARD),
        ))
        // Clicks inside the window must not fall through to the viewport.
        .observe(|mut click: On<PointerClick>| click.propagate(false))
        .id();

    // Header
    let header = container(world, root, Node { align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() });
    let title = container(world, header, Node { flex_grow: 1.0, ..default() });
    text(world, title, "🧱 UI Editor", 14.0, TEXT_PRIMARY);
    let toggle = button(world, header, "Game UI: shown", PanelAction::ToggleGameUi, false);
    if let Some(child) = world.get::<Children>(toggle).and_then(|c| c.first().copied()) {
        world.entity_mut(child).insert(GameUiToggleText);
    }
    button(world, header, "✕", PanelAction::Close, false);

    // Palette
    section(world, root, "ADD ELEMENT  ·  goes into the selected container, else at the root");
    let palette = container(world, root, Node { flex_wrap: FlexWrap::Wrap, column_gap: Val::Px(4.0), row_gap: Val::Px(4.0), ..default() });
    for kind in UiElementKind::ALL {
        let b = clickable(
            world,
            palette,
            PanelAction::Add(kind),
            Node { width: Val::Px(84.0), padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)), justify_content: JustifyContent::Center, ..default() },
            BG_CARD,
        );
        text(world, b, &format!("+ {}", kind.label()), 11.5, TEXT_PRIMARY);
    }
    field_row(world, root, "Image path", PALETTE_IMAGE);

    // Tree
    section(world, root, "GAME UI TREE");
    let tools = container(world, root, Node { column_gap: Val::Px(4.0), ..default() });
    button(world, tools, "▲ Up", PanelAction::MoveUp, true);
    button(world, tools, "▼ Down", PanelAction::MoveDown, true);
    button(world, tools, "Duplicate", PanelAction::Duplicate, true);
    button(world, tools, "Delete", PanelAction::Delete, true);
    let tools2 = container(world, root, Node { column_gap: Val::Px(4.0), ..default() });
    button(world, tools2, "Move to root", PanelAction::ToRoot, true);
    button(world, tools2, "Deselect", PanelAction::Deselect, true);
    world.spawn((
        EditorInternal,
        TreeList,
        bevy::ui_widgets::ScrollArea,
        Node {
            flex_direction: FlexDirection::Column,
            min_height: Val::Px(60.0),
            max_height: Val::Px(190.0),
            flex_shrink: 0.0,
            padding: UiRect::all(Val::Px(4.0)),
            row_gap: Val::Px(1.0),
            overflow: Overflow::scroll_y(),
            border_radius: radius_card(),
            ..default()
        },
        BackgroundColor(BG_SHELL),
        ChildOf(root),
    ));

    // Properties
    section(world, root, "PROPERTIES  ·  click a value, type, Enter (Esc cancels) · Ctrl+Z undoes");
    world.spawn((
        EditorInternal,
        PropsList,
        bevy::ui_widgets::ScrollArea,
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            row_gap: Val::Px(2.0),
            padding: UiRect::right(Val::Px(4.0)),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ChildOf(root),
    ));
    let status = container(world, root, Node { padding: UiRect::top(Val::Px(3.0)), border: UiRect::top(Val::Px(1.0)), ..default() });
    world.entity_mut(status).insert(BorderColor::all(BORDER_SUBTLE));
    let st = text(world, status, "", 11.0, TEXT_SECONDARY);
    world.entity_mut(st).insert(StatusText);
    root
}

fn build_props(world: &mut World, list: Entity, target: Option<Entity>) {
    world.entity_mut(list).despawn_children();
    let Some(e) = target else {
        text(world, list, "Select a game UI element (tree, viewport click, or Hierarchy).", FONT, TEXT_MUTED);
        return;
    };
    let kind = model::kind_of(world, e);
    let has = |world: &World, key: &str| model::get_prop(world, e, key).is_some();
    section(world, list, &format!("ELEMENT · {}", kind.label().to_uppercase()));
    field_row(world, list, "Name", "name");
    chips_row(world, list, "Visible", "visible", &[("Shown", "true"), ("Hidden", "false")]);

    section(world, list, "RECT");
    anchor_grid(world, list);
    pair_row(world, list, "Offset", ("X", "x"), ("Y", "y"));
    pair_row(world, list, "Size", ("W", "width"), ("H", "height"));
    pair_row(world, list, "Min size", ("W", "min_width"), ("H", "min_height"));
    chips_row(world, list, "Self align", "align_self", &[("Auto", "auto"), ("Start", "start"), ("Center", "center"), ("End", "end"), ("Stretch", "stretch")]);

    if kind != UiElementKind::Text && kind != UiElementKind::Spacer {
        section(world, list, "LAYOUT (CHILDREN)");
        chips_row(world, list, "Direction", "direction", &[("Row", "row"), ("Column", "column"), ("Row ⇠", "row_reverse"), ("Col ⇡", "column_reverse")]);
        chips_row(world, list, "Justify", "justify", &[("Start", "start"), ("Center", "center"), ("End", "end"), ("Between", "space_between"), ("Around", "space_around"), ("Evenly", "space_evenly")]);
        chips_row(world, list, "Align", "align", &[("Start", "start"), ("Center", "center"), ("End", "end"), ("Stretch", "stretch")]);
        pair_row(world, list, "Gap / grow", ("gap", "gap"), ("grow", "grow"));
        chips_row(world, list, "Wrap", "wrap", &[("No", "false"), ("Yes", "true")]);
        chips_row(world, list, "Clip", "clip", &[("No", "false"), ("Yes", "true")]);
    } else {
        field_row(world, list, "Flex grow", "grow");
    }

    section(world, list, "SPACING  ·  1–4 values: 8 · 8 16 · t r b l");
    field_row(world, list, "Padding", "padding");
    field_row(world, list, "Margin", "margin");

    section(world, list, "STYLE  ·  colors #RRGGBB[AA] or none");
    color_row(world, list, "Background", "background");
    pair_row(world, list, "Border", ("w", "border_width"), ("radius", "radius"));
    color_row(world, list, "Border color", "border_color");

    if has(world, "text") {
        section(world, list, "TEXT");
        field_row(world, list, "Text", "text");
        field_row(world, list, "Font size", "font_size");
        color_row(world, list, "Text color", "text_color");
        chips_row(world, list, "Text align", "text_align", &[("Left", "left"), ("Center", "center"), ("Right", "right")]);
    }
    if has(world, "action") {
        section(world, list, "BUTTON (PLAY MODE CLICK)");
        chips_row(world, list, "On click", "action", &[("Log", "log"), ("Toggle light", "toggle_light"), ("Spawn cube", "spawn_cube"), ("Stats", "print_stats"), ("Custom", "custom")]);
        field_row(world, list, "Message/name", "action_arg");
    }
    if has(world, "value") {
        section(world, list, if kind == UiElementKind::Slider { "SLIDER" } else { "PROGRESS BAR" });
        field_row(world, list, "Value", "value");
        pair_row(world, list, "Range", ("min", "min"), ("max", "max"));
        if has(world, "step") {
            field_row(world, list, "Step", "step");
        }
        color_row(world, list, "Fill color", "fill_color");
    }
    if has(world, "checked") {
        section(world, list, "CHECKBOX");
        chips_row(world, list, "Checked", "checked", &[("On", "true"), ("Off", "false")]);
        color_row(world, list, "Mark color", "fill_color");
    }
    if has(world, "image") {
        section(world, list, "IMAGE");
        field_row(world, list, "Image path", "image");
        color_row(world, list, "Tint", "tint");
    }
}

fn build_tree(world: &mut World, list: Entity, rows: &[(Entity, usize, String, UiElementKind)], selected: Option<Entity>) {
    world.entity_mut(list).despawn_children();
    if rows.is_empty() {
        text(world, list, "No game UI yet. Use the palette above.", FONT, TEXT_MUTED);
        return;
    }
    for (e, depth, name, kind) in rows {
        let bg = if Some(*e) == selected { ACCENT_CYAN_DIM } else { Color::NONE };
        let r = clickable(
            world,
            list,
            PanelAction::SelectRow(*e),
            Node { padding: UiRect { left: Val::Px(6.0 + *depth as f32 * 14.0), right: Val::Px(6.0), top: Val::Px(2.0), bottom: Val::Px(2.0) }, column_gap: Val::Px(6.0), flex_shrink: 0.0, ..default() },
            bg,
        );
        text(world, r, name, FONT, if Some(*e) == selected { ACCENT_CYAN } else { TEXT_PRIMARY });
        text(world, r, kind.label(), 10.5, TEXT_MUTED);
    }
}

fn selected_element(world: &World) -> Option<Entity> {
    world.resource::<InspectorSelection>().0.filter(|e| model::is_game_ui(world, *e)).map(|e| model::owner_of(world, e))
}

fn set_status(world: &mut World, msg: impl Into<String>) {
    world.resource_mut::<UiEditorState>().status = msg.into();
}

fn report<T>(world: &mut World, result: Result<T, String>, ok: &str) {
    match result {
        Ok(_) => set_status(world, ok),
        Err(e) => set_status(world, format!("⚠ {e}")),
    }
}

/// Parent for new elements: the selected container, else the selected element's parent, else root.
pub fn insertion_parent(world: &World, selected: Option<Entity>) -> Option<Entity> {
    let sel = selected?;
    if model::kind_of(world, sel).is_container() {
        Some(sel)
    } else {
        model::ui_parent(world, sel)
    }
}

fn run_action(world: &mut World, action: PanelAction, source: Entity) {
    if *world.resource::<PlayState>() != PlayState::Edit && !matches!(action, PanelAction::Close | PanelAction::ToggleGameUi) {
        set_status(world, "Stop Play mode to edit UI.");
        return;
    }
    let sel = selected_element(world);
    match action {
        PanelAction::Close => world.resource_mut::<UiEditorState>().open = false,
        PanelAction::ToggleGameUi => {
            let mut st = world.resource_mut::<UiEditorState>();
            st.show_game_ui = !st.show_game_ui;
        }
        PanelAction::Add(kind) => {
            let parent = insertion_parent(world, sel);
            let image = world.resource::<UiEditorState>().image_path.clone();
            let txt = (kind == UiElementKind::Image).then_some(image.as_str()).filter(|s| !s.is_empty());
            let r = model::add_element(world, kind, parent, txt, None);
            report(world, r, &format!("Added {}", kind.label()));
        }
        PanelAction::MoveUp | PanelAction::MoveDown => {
            let Some(e) = sel else { return set_status(world, "Select an element first.") };
            let r = model::shift_element(world, e, if matches!(action, PanelAction::MoveUp) { -1 } else { 1 });
            report(world, r, "Moved");
        }
        PanelAction::Duplicate => {
            let Some(e) = sel else { return set_status(world, "Select an element first.") };
            let r = model::duplicate_element(world, e);
            report(world, r, "Duplicated");
        }
        PanelAction::Delete => {
            let Some(e) = sel else { return set_status(world, "Select an element first.") };
            let r = model::delete_element(world, e);
            report(world, r, "Deleted (Ctrl+Z restores)");
        }
        PanelAction::ToRoot => {
            let Some(e) = sel else { return set_status(world, "Select an element first.") };
            let r = model::move_element(world, e, None, None).and_then(|_| model::apply_props_undoable(world, e, &[("anchor".into(), "center".into())]));
            report(world, r, "Moved to root (centered)");
        }
        PanelAction::Deselect => world.resource_mut::<InspectorSelection>().0 = None,
        PanelAction::SelectRow(e) => {
            if world.get_entity(e).is_ok() {
                world.resource_mut::<InspectorSelection>().0 = Some(e);
            }
        }
        PanelAction::Chip(key, value) => {
            let Some(e) = sel else { return };
            let r = model::apply_props_undoable(world, e, &[(key.to_string(), value.to_string())]);
            report(world, r, &format!("{key} = {value}"));
        }
        PanelAction::BeginEdit => begin_edit(world, source),
    }
    let sel = selected_element(world);
    world.resource_mut::<UiEditorState>().status_sel = sel;
}

fn begin_edit(world: &mut World, field: Entity) {
    commit_edit(world);
    let Some(binding) = world.get::<EdField>(field).copied() else { return };
    let (target, value) = if binding.key == PALETTE_IMAGE {
        (None, world.resource::<UiEditorState>().image_path.clone())
    } else {
        let Some(e) = selected_element(world) else { return };
        let Some(v) = model::get_prop(world, e, binding.key) else { return };
        (Some(e), v)
    };
    world.resource_mut::<crate::ui::inspector::property_editor::PropertyEditState>().cancel();
    world.resource_mut::<crate::ui::inspector::TransformTextEditState>().active = None;
    world.resource_mut::<crate::ui::hierarchy::HierarchyState>().search_focused = false;
    world.resource_mut::<UiEditorState>().editing = Some(TextEdit { field, key: binding.key, target, buffer: value, replace: true });
}

/// Applies the active text edit (if any). Returns false and keeps editing on a parse error.
fn commit_edit(world: &mut World) -> bool {
    let Some(edit) = world.resource_mut::<UiEditorState>().editing.take() else { return true };
    match edit.target {
        None => {
            world.resource_mut::<UiEditorState>().image_path = edit.buffer.trim().to_string();
            true
        }
        Some(e) if world.get_entity(e).is_ok() => {
            let current = model::get_prop(world, e, edit.key);
            if current.as_deref() == Some(edit.buffer.as_str()) {
                return true;
            }
            match model::apply_props_undoable(world, e, &[(edit.key.to_string(), edit.buffer.clone())]) {
                Ok(()) => {
                    set_status(world, format!("{} = {}", edit.key, edit.buffer));
                    world.resource_mut::<UiEditorState>().status_sel = Some(e);
                    true
                }
                Err(err) => {
                    set_status(world, format!("⚠ {err}"));
                    world.resource_mut::<UiEditorState>().editing = Some(edit);
                    false
                }
            }
        }
        Some(_) => true,
    }
}

/// While a panel text box is focused, other editor shortcuts must not see the keys.
pub fn swallow_keys_while_editing(state: Res<UiEditorState>, mut keys: ResMut<ButtonInput<KeyCode>>) {
    if state.editing.is_some() {
        keys.reset_all();
    }
}

#[derive(Default)]
pub struct KeyState {
    ctrl: bool,
}

pub fn text_input(world: &mut World, mut cursor: Local<MessageCursor<KeyboardInput>>, mut mods: Local<KeyState>) {
    let events: Vec<KeyboardInput> = cursor.read(world.resource::<Messages<KeyboardInput>>()).cloned().collect();
    // Another editor text box took focus, or the field vanished.
    let other_focus = world.resource::<crate::ui::inspector::property_editor::PropertyEditState>().is_editing()
        || world.resource::<crate::ui::hierarchy::HierarchyState>().search_focused
        || world.resource::<crate::ui::inspector::TransformTextEditState>().active.is_some();
    let field_gone = world.resource::<UiEditorState>().editing.as_ref().is_some_and(|ed| world.get_entity(ed.field).is_err());
    if other_focus || field_gone || *world.resource::<PlayState>() != PlayState::Edit {
        world.resource_mut::<UiEditorState>().editing = None;
    }
    let primary = world.query_filtered::<(Entity, &Window), With<PrimaryWindow>>().iter(world).find(|(_, w)| w.focused).map(|(e, _)| e);
    for ev in events {
        if matches!(ev.key_code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            mods.ctrl = ev.state.is_pressed();
        }
        if !ev.state.is_pressed() || Some(ev.window) != primary || world.resource::<UiEditorState>().editing.is_none() {
            continue;
        }
        match ev.key_code {
            KeyCode::Escape => {
                world.resource_mut::<UiEditorState>().editing = None;
                set_status(world, "Edit cancelled");
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Tab => {
                commit_edit(world);
            }
            KeyCode::Backspace => {
                let mut st = world.resource_mut::<UiEditorState>();
                if let Some(ed) = st.editing.as_mut() {
                    if ed.replace {
                        ed.buffer.clear();
                    } else {
                        ed.buffer.pop();
                    }
                    ed.replace = false;
                }
            }
            _ => {
                if let Key::Character(chars) = &ev.logical_key {
                    let ctrl = mods.ctrl;
                    let mut st = world.resource_mut::<UiEditorState>();
                    if let Some(ed) = st.editing.as_mut() {
                        if ctrl {
                            if ev.key_code == KeyCode::KeyA {
                                ed.replace = true;
                            }
                            continue;
                        }
                        if ed.replace {
                            ed.buffer.clear();
                            ed.replace = false;
                        }
                        ed.buffer.push_str(chars);
                    }
                } else if ev.logical_key == Key::Space {
                    let mut st = world.resource_mut::<UiEditorState>();
                    if let Some(ed) = st.editing.as_mut() {
                        if ed.replace {
                            ed.buffer.clear();
                            ed.replace = false;
                        }
                        ed.buffer.push(' ');
                    }
                }
            }
        }
    }
}

/// Opens/closes the window, rebuilds tree and property sheet when they change, refreshes values.
pub fn update_window(world: &mut World) {
    let open = world.resource::<UiEditorState>().open;
    let existing = world.resource::<UiEditorState>().window;
    match (open, existing.filter(|e| world.get_entity(*e).is_ok())) {
        (true, None) => {
            let w = spawn_window(world);
            let o = canvas::spawn_outline(world);
            let mut st = world.resource_mut::<UiEditorState>();
            st.window = Some(w);
            st.outline = Some(o);
            st.tree_sig.clear();
            st.tree_built = false;
            st.props_for = None;
            st.props_built = false;
        }
        (false, Some(w)) => {
            world.entity_mut(w).despawn();
            let outline = world.resource_mut::<UiEditorState>().outline.take();
            if let Some(o) = outline.filter(|o| world.get_entity(*o).is_ok()) {
                world.entity_mut(o).despawn();
            }
            let mut st = world.resource_mut::<UiEditorState>();
            st.window = None;
            st.editing = None;
            return;
        }
        (false, None) => return,
        (true, Some(_)) => {}
    }

    let selected = selected_element(world);
    // Tree
    let tree: Vec<(Entity, usize, String, UiElementKind)> = model::ui_tree(world)
        .into_iter()
        .map(|(e, d)| (e, d, model::display_name(world, e), model::kind_of(world, e)))
        .collect();
    let sig: Vec<(Entity, usize, String)> = tree.iter().map(|(e, d, n, _)| (*e, *d, n.clone())).collect();
    let rebuild_tree = {
        let st = world.resource::<UiEditorState>();
        !st.tree_built || st.tree_sig != sig || st.tree_sel != selected
    };
    if world.resource::<UiEditorState>().status_sel != selected {
        // A new selection makes the last action message stale.
        let mut st = world.resource_mut::<UiEditorState>();
        st.status.clear();
        st.status_sel = selected;
    }
    if rebuild_tree {
        if let Some(list) = world.query_filtered::<Entity, With<TreeList>>().iter(world).next() {
            build_tree(world, list, &tree, selected);
            let mut st = world.resource_mut::<UiEditorState>();
            st.tree_sig = sig;
            st.tree_sel = selected;
            st.tree_built = true;
        }
    }
    // Property sheet
    let props_key = selected.map(|e| (e, model::kind_of(world, e)));
    let rebuild_props = {
        let st = world.resource::<UiEditorState>();
        !st.props_built || st.props_for != props_key
    };
    if rebuild_props {
        if let Some(list) = world.query_filtered::<Entity, With<PropsList>>().iter(world).next() {
            build_props(world, list, selected);
            let mut st = world.resource_mut::<UiEditorState>();
            st.props_for = props_key;
            st.props_built = true;
        }
    }
    refresh_values(world, selected);
}

fn refresh_values(world: &mut World, selected: Option<Entity>) {
    let (editing_field, buffer, image_path, show, status) = {
        let st = world.resource::<UiEditorState>();
        (
            st.editing.as_ref().map(|e| e.field),
            st.editing.as_ref().map(|e| e.buffer.clone()).unwrap_or_default(),
            st.image_path.clone(),
            st.show_game_ui,
            st.status.clone(),
        )
    };
    // Field texts
    let fields: Vec<(Entity, &'static str, Vec<Entity>)> = world
        .query::<(Entity, &EdField, &Children)>()
        .iter(world)
        .map(|(e, f, c)| (e, f.key, c.to_vec()))
        .collect();
    for (fe, key, children) in fields {
        let editing = editing_field == Some(fe);
        let value = if editing {
            format!("{buffer}▏")
        } else if key == PALETTE_IMAGE {
            if image_path.is_empty() { "(none: placeholder box)".into() } else { image_path.clone() }
        } else {
            selected.and_then(|e| model::get_prop(world, e, key)).unwrap_or_else(|| "—".into())
        };
        for c in children {
            if world.get::<EdFieldText>(c).is_some() {
                if let Some(mut t) = world.get_mut::<Text>(c) {
                    if t.0 != value {
                        t.0 = value.clone();
                    }
                }
            }
        }
        let want = if editing { BORDER_ACCENT } else { Color::NONE };
        if let Some(mut b) = world.get_mut::<BorderColor>(fe) {
            if editing && b.top != want {
                *b = BorderColor::all(want);
            }
        }
    }
    // Chip highlight
    let chips: Vec<(Entity, EdChip, Color)> = world.query::<(Entity, &EdChip, &EdBase)>().iter(world).map(|(e, c, b)| (e, *c, b.0)).collect();
    for (ce, chip, base) in chips {
        let current = selected.and_then(|e| model::get_prop(world, e, chip.key)).unwrap_or_default();
        let on = current == chip.value || current.starts_with(&format!("{}:", chip.value));
        let color = if on { ACCENT_CYAN_DIM.with_alpha(0.45) } else { base };
        if let Some(mut bg) = world.get_mut::<BackgroundColor>(ce) {
            if bg.0 != color {
                bg.0 = color;
            }
        }
    }
    // Swatches
    let swatches: Vec<(Entity, &'static str)> = world.query::<(Entity, &EdSwatch)>().iter(world).map(|(e, s)| (e, s.key)).collect();
    for (se, key) in swatches {
        let c = selected.and_then(|e| model::get_prop(world, e, key)).and_then(|v| model::parse_color(&v).ok()).unwrap_or(Color::NONE);
        if let Some(mut bg) = world.get_mut::<BackgroundColor>(se) {
            if bg.0 != c {
                bg.0 = c;
            }
        }
    }
    // Header toggle + status
    let toggle_label = if show { "Game UI: shown" } else { "Game UI: hidden" };
    for mut t in world.query_filtered::<&mut Text, With<GameUiToggleText>>().iter_mut(world) {
        if t.0 != toggle_label {
            t.0 = toggle_label.into();
        }
    }
    let status = if status.is_empty() {
        match selected {
            Some(e) => format!("Selected: {} — drag it in the viewport to move", model::display_name(world, e)),
            None => "Click game UI in the viewport to select it.".into(),
        }
    } else {
        status
    };
    for mut t in world.query_filtered::<&mut Text, With<StatusText>>().iter_mut(world) {
        if t.0 != status {
            t.0 = status.clone();
        }
    }
}

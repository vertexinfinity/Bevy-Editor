//! Game UI data model: element kinds, runtime widget components, spawning, the string-keyed
//! property API shared by the panel and RPC, and undoable editor commands.
use bevy::{
    ecs::entity::EntityHashMap,
    inspector::InspectorSelection,
    prelude::*,
    text::FontSize,
    world_serialization::{DynamicWorld, DynamicWorldBuilder},
};

use crate::core::{commands::EditorCommand, play_mode::PreviewEntity, reflect_fields};
use crate::scene::{components::UiButtonClickAction, EditorInternal, SceneEntity};

/// Authoring kind of a game UI element; drives defaults and which properties the panel shows.
#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
#[reflect(Default)]
pub enum UiElementKind {
    #[default]
    Panel,
    Row,
    Column,
    Text,
    Button,
    Image,
    ProgressBar,
    Checkbox,
    Slider,
    Spacer,
}

impl UiElementKind {
    pub const ALL: [UiElementKind; 10] = [
        Self::Panel, Self::Row, Self::Column, Self::Text, Self::Button,
        Self::Image, Self::ProgressBar, Self::Checkbox, Self::Slider, Self::Spacer,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Panel => "panel",
            Self::Row => "row",
            Self::Column => "column",
            Self::Text => "text",
            Self::Button => "button",
            Self::Image => "image",
            Self::ProgressBar => "progress_bar",
            Self::Checkbox => "checkbox",
            Self::Slider => "slider",
            Self::Spacer => "spacer",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Panel => "Panel",
            Self::Row => "Row",
            Self::Column => "Column",
            Self::Text => "Text",
            Self::Button => "Button",
            Self::Image => "Image",
            Self::ProgressBar => "Progress Bar",
            Self::Checkbox => "Checkbox",
            Self::Slider => "Slider",
            Self::Spacer => "Spacer",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        let alias = match s.as_str() {
            "container" | "panel_container" => "panel",
            "label" => "text",
            "progress" | "progressbar" | "bar" => "progress_bar",
            "toggle" | "check" => "checkbox",
            "img" => "image",
            "hbox" => "row",
            "vbox" => "column",
            other => other,
        };
        Self::ALL.into_iter().find(|k| k.id() == alias)
    }

    /// Whether children can be added under this element in the tree.
    pub fn is_container(self) -> bool {
        matches!(self, Self::Panel | Self::Row | Self::Column | Self::Button | Self::Image)
    }
}

/// Marks an entity as authored by the UI Editor.
#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component, Default)]
pub struct UiElement {
    pub kind: UiElementKind,
}

/// Role of an internal child that belongs to a composite widget (button label, bar fill, ...).
#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[reflect(Default)]
pub enum UiPartRole {
    #[default]
    Label,
    Fill,
    Track,
    Box,
    CheckMark,
    Thumb,
}

/// Internal child of a composite widget; hidden from the UI tree and resolved to its owner on click.
#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component, Default)]
pub struct UiWidgetPart {
    pub role: UiPartRole,
}

/// Horizontal fill bar. Scripts read/write `value`; the fill child follows automatically.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct UiProgressBar {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub fill_color: Color,
}

impl Default for UiProgressBar {
    fn default() -> Self {
        Self { value: 0.6, min: 0.0, max: 1.0, fill_color: Color::srgb(0.2, 0.78, 0.45) }
    }
}

impl UiProgressBar {
    pub fn fraction(&self) -> f32 {
        let span = self.max - self.min;
        if span.abs() < f32::EPSILON { 0.0 } else { ((self.value - self.min) / span).clamp(0.0, 1.0) }
    }
}

/// Toggle box. Clicking it in Play mode flips `checked`.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct UiCheckbox {
    pub checked: bool,
    pub mark_color: Color,
}

impl Default for UiCheckbox {
    fn default() -> Self {
        Self { checked: true, mark_color: Color::srgb(0.27, 0.77, 0.86) }
    }
}

/// Horizontal slider. Pressing or dragging it in Play mode sets `value` (snapped to `step` when > 0).
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
pub struct UiSlider {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub fill_color: Color,
}

impl Default for UiSlider {
    fn default() -> Self {
        Self { value: 0.5, min: 0.0, max: 1.0, step: 0.0, fill_color: Color::srgb(0.27, 0.77, 0.86) }
    }
}

impl UiSlider {
    pub fn fraction(&self) -> f32 {
        let span = self.max - self.min;
        if span.abs() < f32::EPSILON { 0.0 } else { ((self.value - self.min) / span).clamp(0.0, 1.0) }
    }

    /// Sets the value from a 0..1 fraction, applying range and step.
    pub fn set_fraction(&mut self, t: f32) {
        let mut v = self.min + t.clamp(0.0, 1.0) * (self.max - self.min);
        if self.step > 0.0 {
            v = self.min + ((v - self.min) / self.step).round() * self.step;
        }
        self.value = v.clamp(self.min.min(self.max), self.max.max(self.min));
    }
}

pub fn register_types(app: &mut App) {
    app.register_type::<UiElementKind>()
        .register_type::<UiElement>()
        .register_type::<UiPartRole>()
        .register_type::<UiWidgetPart>()
        .register_type::<UiProgressBar>()
        .register_type::<UiCheckbox>()
        .register_type::<UiSlider>();
}

// ---------------------------------------------------------------------------------------------
// Queries over the authored game UI
// ---------------------------------------------------------------------------------------------

/// Authoring (not Play-mode copy) game UI node.
pub fn is_game_ui(world: &World, e: Entity) -> bool {
    let Ok(r) = world.get_entity(e) else { return false };
    r.contains::<Node>() && r.contains::<SceneEntity>() && !r.contains::<EditorInternal>() && !r.contains::<PreviewEntity>()
}

pub fn is_part(world: &World, e: Entity) -> bool {
    world.get::<UiWidgetPart>(e).is_some()
}

pub fn ui_parent(world: &World, e: Entity) -> Option<Entity> {
    world.get::<ChildOf>(e).map(|c| c.parent()).filter(|p| world.get::<Node>(*p).is_some())
}

/// Climbs from a widget part to the element that owns it.
pub fn owner_of(world: &World, mut e: Entity) -> Entity {
    while is_part(world, e) {
        match ui_parent(world, e) {
            Some(p) => e = p,
            None => break,
        }
    }
    e
}

pub fn kind_of(world: &World, e: Entity) -> UiElementKind {
    if let Some(el) = world.get::<UiElement>(e) {
        return el.kind;
    }
    if world.get::<UiButtonClickAction>(e).is_some() {
        UiElementKind::Button
    } else if world.get::<Text>(e).is_some() {
        UiElementKind::Text
    } else if world.get::<ImageNode>(e).is_some() {
        UiElementKind::Image
    } else if world.get::<UiProgressBar>(e).is_some() {
        UiElementKind::ProgressBar
    } else if world.get::<UiCheckbox>(e).is_some() {
        UiElementKind::Checkbox
    } else if world.get::<UiSlider>(e).is_some() {
        UiElementKind::Slider
    } else {
        UiElementKind::Panel
    }
}

/// Element children (parts excluded) in layout order.
pub fn ui_children(world: &World, e: Entity) -> Vec<Entity> {
    world
        .get::<Children>(e)
        .map(|c| c.iter().filter(|c| is_game_ui(world, *c) && !is_part(world, *c)).collect())
        .unwrap_or_default()
}

pub fn ui_roots(world: &mut World) -> Vec<Entity> {
    let mut q = world.query_filtered::<Entity, (With<Node>, With<SceneEntity>, Without<EditorInternal>, Without<PreviewEntity>)>();
    let all: Vec<Entity> = q.iter(world).collect();
    let mut roots: Vec<Entity> = all
        .into_iter()
        .filter(|e| ui_parent(world, *e).is_none() && !is_part(world, *e))
        .collect();
    roots.sort_by_key(|e| e.index());
    roots
}

/// Pre-order (entity, depth) walk of the element tree.
pub fn ui_tree(world: &mut World) -> Vec<(Entity, usize)> {
    fn walk(world: &World, e: Entity, depth: usize, out: &mut Vec<(Entity, usize)>) {
        out.push((e, depth));
        for c in ui_children(world, e) {
            walk(world, c, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    for r in ui_roots(world) {
        walk(world, r, 0, &mut out);
    }
    out
}

pub fn display_name(world: &World, e: Entity) -> String {
    world.get::<Name>(e).map(|n| n.as_str().to_string()).unwrap_or_else(|| format!("{} {}", kind_of(world, e).label(), e.index()))
}

fn unique_name(world: &mut World, base: &str) -> String {
    let mut q = world.query::<&Name>();
    let names: std::collections::HashSet<String> = q.iter(world).map(|n| n.as_str().to_string()).collect();
    if !names.contains(base) {
        return base.to_string();
    }
    (2..).map(|i| format!("{base} {i}")).find(|n| !names.contains(n)).unwrap_or_else(|| base.to_string())
}

/// Resolves `entity` (bits) or `name` from RPC params to an authored UI element.
pub fn resolve_element(world: &mut World, params: &serde_json::Value, key_entity: &str, key_name: &str) -> Result<Option<Entity>, String> {
    if let Some(bits) = params.get(key_entity).and_then(|v| v.as_u64()) {
        let e = Entity::try_from_bits(bits).filter(|e| world.get_entity(*e).is_ok()).ok_or_else(|| format!("No entity with id {bits}"))?;
        return Ok(Some(e));
    }
    if let Some(name) = params.get(key_name).and_then(|v| v.as_str()) {
        let mut q = world.query_filtered::<(Entity, &Name), (With<Node>, With<SceneEntity>, Without<PreviewEntity>, Without<EditorInternal>)>();
        let mut found: Vec<Entity> = q.iter(world).filter(|(_, n)| n.as_str() == name).map(|(e, _)| e).collect();
        found.sort_by_key(|e| e.index());
        return found.first().copied().map(Some).ok_or_else(|| format!("No UI element named '{name}'"));
    }
    Ok(None)
}

// ---------------------------------------------------------------------------------------------
// Value formatting / parsing
// ---------------------------------------------------------------------------------------------

pub fn fmt_num(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() { "0".into() } else { s.to_string() }
}

pub fn fmt_val(v: Val) -> String {
    match v {
        Val::Auto => "auto".into(),
        Val::Px(x) => format!("{}px", fmt_num(x)),
        Val::Percent(x) => format!("{}%", fmt_num(x)),
        Val::Vw(x) => format!("{}vw", fmt_num(x)),
        Val::Vh(x) => format!("{}vh", fmt_num(x)),
        Val::VMin(x) => format!("{}vmin", fmt_num(x)),
        Val::VMax(x) => format!("{}vmax", fmt_num(x)),
        Val::Em(x) => format!("{}em", fmt_num(x)),
        Val::Rem(x) => format!("{}rem", fmt_num(x)),
    }
}

pub fn parse_val(s: &str) -> Result<Val, String> {
    let t = s.trim().to_ascii_lowercase();
    if t.is_empty() || t == "auto" {
        return Ok(Val::Auto);
    }
    let units: [(&str, fn(f32) -> Val); 9] = [
        ("vmin", Val::VMin), ("vmax", Val::VMax), ("px", Val::Px), ("%", Val::Percent),
        ("vw", Val::Vw), ("vh", Val::Vh), ("rem", Val::Rem), ("em", Val::Em), ("", Val::Px),
    ];
    for (suffix, make) in units {
        if let Some(num) = t.strip_suffix(suffix) {
            if let Ok(v) = num.trim().parse::<f32>() {
                if v.is_finite() {
                    return Ok(make(v));
                }
            }
        }
    }
    Err(format!("'{s}' is not a size (use 120, 120px, 50%, 10vw, auto)"))
}

fn val_px(v: Val) -> f32 {
    if let Val::Px(x) = v { x } else { 0.0 }
}

pub fn fmt_rect(r: UiRect) -> String {
    if r.top == r.bottom && r.left == r.right {
        if r.top == r.left {
            fmt_val(r.top)
        } else {
            format!("{} {}", fmt_val(r.top), fmt_val(r.left))
        }
    } else {
        format!("{} {} {} {}", fmt_val(r.top), fmt_val(r.right), fmt_val(r.bottom), fmt_val(r.left))
    }
}

/// CSS shorthand: `a`, `v h`, `t h b`, `t r b l`.
pub fn parse_rect(s: &str) -> Result<UiRect, String> {
    let parts: Vec<Val> = s.split([' ', ',']).filter(|p| !p.trim().is_empty()).map(parse_val).collect::<Result<_, _>>()?;
    let (t, r, b, l) = match parts.as_slice() {
        [a] => (*a, *a, *a, *a),
        [v, h] => (*v, *h, *v, *h),
        [t, h, b] => (*t, *h, *b, *h),
        [t, r, b, l] => (*t, *r, *b, *l),
        _ => return Err("Use 1 to 4 sizes, e.g. 8 or 8 16".into()),
    };
    Ok(UiRect::new(l, r, t, b))
}

pub fn fmt_color(c: Color) -> String {
    if c.alpha() <= 0.0 { "none".into() } else { reflect_fields::color_hex(c) }
}

pub fn parse_color(s: &str) -> Result<Color, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "none" | "transparent" | "" => Ok(Color::NONE),
        "white" => Ok(Color::WHITE),
        "black" => Ok(Color::BLACK),
        _ => reflect_fields::parse_color(s),
    }
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "shown" | "visible" => Ok(true),
        "false" | "0" | "no" | "off" | "hidden" => Ok(false),
        _ => Err(format!("'{s}' is not true/false")),
    }
}

fn parse_f32(s: &str) -> Result<f32, String> {
    s.trim().trim_end_matches("px").parse::<f32>().ok().filter(|v| v.is_finite()).ok_or_else(|| format!("'{s}' is not a number"))
}

fn font_px(f: &TextFont) -> f32 {
    match f.font_size {
        FontSize::Px(v) => v,
        FontSize::Rem(v) => v * 16.0,
        _ => 16.0,
    }
}

// ---------------------------------------------------------------------------------------------
// Anchors
// ---------------------------------------------------------------------------------------------

pub const ANCHORS: [&str; 11] = [
    "top_left", "top", "top_right", "left", "center", "right", "bottom_left", "bottom", "bottom_right", "stretch", "none",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Edge {
    Start,
    Center,
    End,
    Stretch,
}

fn anchor_edges(name: &str) -> Option<(Edge, Edge)> {
    use Edge::*;
    Some(match name {
        "top_left" => (Start, Start),
        "top" | "top_center" => (Center, Start),
        "top_right" => (End, Start),
        "left" | "center_left" => (Start, Center),
        "center" | "middle" => (Center, Center),
        "right" | "center_right" => (End, Center),
        "bottom_left" => (Start, End),
        "bottom" | "bottom_center" => (Center, End),
        "bottom_right" => (End, End),
        "stretch" | "fill" => (Stretch, Stretch),
        _ => return None,
    })
}

fn edge_of(start: Val, end: Val) -> Edge {
    match (start, end) {
        (Val::Percent(p), Val::Auto) if (p - 50.0).abs() < 0.01 => Edge::Center,
        (Val::Auto, Val::Auto) => Edge::Start,
        (Val::Auto, _) => Edge::End,
        (_, Val::Auto) => Edge::Start,
        _ => Edge::Stretch,
    }
}

pub fn read_anchor(node: &Node) -> String {
    if node.position_type == PositionType::Relative {
        return "none".into();
    }
    let h = edge_of(node.left, node.right);
    let v = edge_of(node.top, node.bottom);
    ANCHORS
        .iter()
        .find(|a| anchor_edges(a) == Some((h, v)))
        .map(|a| a.to_string())
        .unwrap_or_else(|| "custom".into())
}

/// Offset from the anchor reference point, in px (positive = right / down).
fn read_offset(node: &Node) -> Vec2 {
    if node.position_type == PositionType::Relative {
        return Vec2::new(val_px(node.left), val_px(node.top));
    }
    let axis = |start: Val, end: Val, margin: Val| match edge_of(start, end) {
        Edge::Start | Edge::Stretch => val_px(start),
        Edge::End => -val_px(end),
        Edge::Center => val_px(margin),
    };
    Vec2::new(axis(node.left, node.right, node.margin.left), axis(node.top, node.bottom, node.margin.top))
}

fn write_offset_axis(node: &mut Node, horizontal: bool, value: f32) {
    let relative = node.position_type == PositionType::Relative;
    let (start, end, margin) = if horizontal {
        (&mut node.left, &mut node.right, &mut node.margin.left)
    } else {
        (&mut node.top, &mut node.bottom, &mut node.margin.top)
    };
    if relative {
        *start = Val::Px(value);
        *end = Val::Auto;
        return;
    }
    match edge_of(*start, *end) {
        Edge::Start => *start = Val::Px(value),
        Edge::End => *end = Val::Px(-value),
        Edge::Center => *margin = Val::Px(value),
        Edge::Stretch => {
            *start = Val::Px(value);
            *end = Val::Px(-value);
        }
    }
}

fn apply_anchor(world: &mut World, e: Entity, name: &str) -> Result<(), String> {
    let logical = world.get::<ComputedNode>(e).map(|c| c.size() * c.inverse_scale_factor).unwrap_or(Vec2::ZERO);
    let mut node = world.get::<Node>(e).cloned().ok_or("Not a UI node")?;
    let mut xf = world.get::<UiTransform>(e).cloned().unwrap_or_default();
    if name == "none" || name == "flow" {
        node.position_type = PositionType::Relative;
        node.left = Val::Auto;
        node.right = Val::Auto;
        node.top = Val::Auto;
        node.bottom = Val::Auto;
        node.margin.left = Val::Px(0.0);
        node.margin.top = Val::Px(0.0);
        xf.translation = Val2::ZERO;
    } else {
        let (h, v) = anchor_edges(name).ok_or_else(|| format!("Unknown anchor '{name}' (use {})", ANCHORS.join(", ")))?;
        node.position_type = PositionType::Absolute;
        let mut translate = [Val::Px(0.0); 2];
        for (i, edge) in [h, v].into_iter().enumerate() {
            let (start, end, margin, size, computed) = if i == 0 {
                (&mut node.left, &mut node.right, &mut node.margin.left, &mut node.width, logical.x)
            } else {
                (&mut node.top, &mut node.bottom, &mut node.margin.top, &mut node.height, logical.y)
            };
            if edge != Edge::Stretch && *size == Val::Auto && *start != Val::Auto && *end != Val::Auto && computed > 0.0 {
                // Leaving a stretch anchor: keep the current visual size.
                *size = Val::Px(computed.round());
            }
            match edge {
                Edge::Start => {
                    *start = Val::Px(0.0);
                    *end = Val::Auto;
                }
                Edge::End => {
                    *start = Val::Auto;
                    *end = Val::Px(0.0);
                }
                Edge::Center => {
                    *start = Val::Percent(50.0);
                    *end = Val::Auto;
                    *margin = Val::Px(0.0);
                    translate[i] = Val::Percent(-50.0);
                }
                Edge::Stretch => {
                    *start = Val::Px(0.0);
                    *end = Val::Px(0.0);
                    *size = Val::Auto;
                }
            }
            if edge != Edge::Center && matches!(*margin, Val::Px(_)) {
                // Offsets of a former center anchor lived in the margin.
                *margin = Val::Px(0.0);
            }
        }
        xf.translation = Val2::new(translate[0], translate[1]);
    }
    world.entity_mut(e).insert((node, xf));
    Ok(())
}


// ---------------------------------------------------------------------------------------------
// Property API
// ---------------------------------------------------------------------------------------------

/// Every key accepted by [`set_prop`] / returned by [`get_props`].
pub const PROP_KEYS: &[&str] = &[
    "name", "anchor", "x", "y", "width", "height", "min_width", "min_height", "max_width", "max_height",
    "left", "right", "top", "bottom", "position", "display", "direction", "justify", "align", "align_self",
    "wrap", "gap", "grow", "shrink", "padding", "margin", "background", "border_width", "border_color",
    "radius", "clip", "visible", "text", "font_size", "text_color", "text_align", "action", "action_arg", "value", "min", "max",
    "step", "checked", "fill_color", "image", "tint",
];

/// Applies `anchor` before offsets/sizes so a single call can set both.
fn prop_priority(key: &str) -> u8 {
    match key {
        "anchor" | "position" => 0,
        "x" | "y" => 2,
        _ => 1,
    }
}

fn text_target(world: &World, e: Entity) -> Option<Entity> {
    if world.get::<Text>(e).is_some() {
        return Some(e);
    }
    fn find(world: &World, e: Entity) -> Option<Entity> {
        for c in world.get::<Children>(e).map(|c| c.to_vec()).unwrap_or_default() {
            if world.get::<UiWidgetPart>(c).is_some_and(|p| p.role == UiPartRole::Label) && world.get::<Text>(c).is_some() {
                return Some(c);
            }
            if is_part(world, c) {
                if let Some(t) = find(world, c) {
                    return Some(t);
                }
            }
        }
        None
    }
    find(world, e).or_else(|| {
        // Buttons spawned by the Hierarchy menu have a plain Text child.
        world.get::<Children>(e)?.iter().find(|c| world.get::<Text>(*c).is_some() && !is_game_ui_element(world, *c))
    })
}

fn is_game_ui_element(world: &World, e: Entity) -> bool {
    world.get::<UiElement>(e).is_some()
}

fn action_text(a: &UiButtonClickAction) -> String {
    match a {
        UiButtonClickAction::LogMessage(m) => format!("log:{m}"),
        UiButtonClickAction::ToggleLight => "toggle_light".into(),
        UiButtonClickAction::SpawnCube => "spawn_cube".into(),
        UiButtonClickAction::PrintStats => "print_stats".into(),
        UiButtonClickAction::CustomAction(n) => format!("custom:{n}"),
    }
}

fn parse_action(s: &str, previous: Option<&UiButtonClickAction>) -> Result<UiButtonClickAction, String> {
    let (kind, arg) = match s.split_once(':') {
        Some((k, a)) => (k.trim().to_ascii_lowercase(), Some(a.to_string())),
        None => (s.trim().to_ascii_lowercase(), None),
    };
    let prev_arg = match previous {
        Some(UiButtonClickAction::LogMessage(m)) | Some(UiButtonClickAction::CustomAction(m)) => Some(m.clone()),
        _ => None,
    };
    Ok(match kind.replace([' ', '-'], "_").as_str() {
        "log" | "log_message" | "logmessage" => UiButtonClickAction::LogMessage(arg.or(prev_arg).unwrap_or_else(|| "Button Clicked!".into())),
        "toggle_light" | "togglelight" => UiButtonClickAction::ToggleLight,
        "spawn_cube" | "spawncube" => UiButtonClickAction::SpawnCube,
        "print_stats" | "printstats" | "stats" => UiButtonClickAction::PrintStats,
        "custom" | "custom_action" | "customaction" => UiButtonClickAction::CustomAction(arg.or(prev_arg).unwrap_or_else(|| "my_action".into())),
        _ => return Err(format!("Unknown action '{s}' (log:msg, toggle_light, spawn_cube, print_stats, custom:name)")),
    })
}

fn enum_name<T: std::fmt::Debug>(v: &T) -> String {
    let s = format!("{v:?}");
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out.replace("flex_start", "start").replace("flex_end", "end")
}

fn parse_justify(s: &str) -> Result<JustifyContent, String> {
    Ok(match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "default" => JustifyContent::Default,
        "start" | "flex_start" => JustifyContent::FlexStart,
        "end" | "flex_end" => JustifyContent::FlexEnd,
        "center" => JustifyContent::Center,
        "stretch" => JustifyContent::Stretch,
        "space_between" | "between" => JustifyContent::SpaceBetween,
        "space_around" | "around" => JustifyContent::SpaceAround,
        "space_evenly" | "evenly" => JustifyContent::SpaceEvenly,
        _ => return Err(format!("Unknown justify '{s}'")),
    })
}

fn parse_align(s: &str) -> Result<AlignItems, String> {
    Ok(match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "default" => AlignItems::Default,
        "start" | "flex_start" => AlignItems::FlexStart,
        "end" | "flex_end" => AlignItems::FlexEnd,
        "center" => AlignItems::Center,
        "stretch" => AlignItems::Stretch,
        "baseline" => AlignItems::Baseline,
        _ => return Err(format!("Unknown align '{s}'")),
    })
}

fn parse_align_self(s: &str) -> Result<AlignSelf, String> {
    Ok(match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "auto" | "default" => AlignSelf::Auto,
        "start" | "flex_start" => AlignSelf::FlexStart,
        "end" | "flex_end" => AlignSelf::FlexEnd,
        "center" => AlignSelf::Center,
        "stretch" => AlignSelf::Stretch,
        "baseline" => AlignSelf::Baseline,
        _ => return Err(format!("Unknown align_self '{s}'")),
    })
}

/// Reads one property as display text; `None` when the key does not apply to the element.
pub fn get_prop(world: &World, e: Entity, key: &str) -> Option<String> {
    let node = world.get::<Node>(e)?;
    let text_e = text_target(world, e);
    Some(match key {
        "name" => display_name(world, e),
        "kind" => kind_of(world, e).id().into(),
        "anchor" => read_anchor(node),
        "x" => fmt_num(read_offset(node).x),
        "y" => fmt_num(read_offset(node).y),
        "width" => fmt_val(node.width),
        "height" => fmt_val(node.height),
        "min_width" => fmt_val(node.min_width),
        "min_height" => fmt_val(node.min_height),
        "max_width" => fmt_val(node.max_width),
        "max_height" => fmt_val(node.max_height),
        "left" => fmt_val(node.left),
        "right" => fmt_val(node.right),
        "top" => fmt_val(node.top),
        "bottom" => fmt_val(node.bottom),
        "position" => enum_name(&node.position_type),
        "display" => enum_name(&node.display),
        "direction" => enum_name(&node.flex_direction),
        "justify" => enum_name(&node.justify_content),
        "align" => enum_name(&node.align_items),
        "align_self" => enum_name(&node.align_self),
        "wrap" => (node.flex_wrap != FlexWrap::NoWrap).to_string(),
        "gap" => fmt_val(node.column_gap),
        "grow" => fmt_num(node.flex_grow),
        "shrink" => fmt_num(node.flex_shrink),
        "padding" => fmt_rect(node.padding),
        "margin" => fmt_rect(node.margin),
        "background" => fmt_color(world.get::<BackgroundColor>(e).map(|b| b.0).unwrap_or(Color::NONE)),
        "border_width" => fmt_val(node.border.top),
        "border_color" => fmt_color(world.get::<BorderColor>(e).map(|b| b.top).unwrap_or(Color::NONE)),
        "radius" => fmt_val(node.border_radius.top_left.x),
        "clip" => (node.overflow.x == OverflowAxis::Clip || node.overflow.x == OverflowAxis::Hidden).to_string(),
        "visible" => (world.get::<Visibility>(e).copied().unwrap_or_default() != Visibility::Hidden).to_string(),
        "text" => world.get::<Text>(text_e?)?.0.clone(),
        "font_size" => fmt_num(font_px(world.get::<TextFont>(text_e?)?)),
        "text_color" => fmt_color(world.get::<TextColor>(text_e?)?.0),
        "text_align" => enum_name(&world.get::<TextLayout>(text_e?).map(|l| l.justify).unwrap_or_default()),
        "action" => action_text(world.get::<UiButtonClickAction>(e)?),
        "action_arg" => match world.get::<UiButtonClickAction>(e)? {
            UiButtonClickAction::LogMessage(m) | UiButtonClickAction::CustomAction(m) => m.clone(),
            _ => String::new(),
        },
        "value" => fmt_num(world.get::<UiProgressBar>(e).map(|p| p.value).or_else(|| world.get::<UiSlider>(e).map(|s| s.value))?),
        "min" => fmt_num(world.get::<UiProgressBar>(e).map(|p| p.min).or_else(|| world.get::<UiSlider>(e).map(|s| s.min))?),
        "max" => fmt_num(world.get::<UiProgressBar>(e).map(|p| p.max).or_else(|| world.get::<UiSlider>(e).map(|s| s.max))?),
        "step" => fmt_num(world.get::<UiSlider>(e)?.step),
        "checked" => world.get::<UiCheckbox>(e)?.checked.to_string(),
        "fill_color" => fmt_color(
            world.get::<UiProgressBar>(e).map(|p| p.fill_color)
                .or_else(|| world.get::<UiSlider>(e).map(|s| s.fill_color))
                .or_else(|| world.get::<UiCheckbox>(e).map(|c| c.mark_color))?,
        ),
        "image" => world.get::<ImageNode>(e)?.image.path().map(|p| p.to_string()).unwrap_or_default(),
        "tint" => fmt_color(world.get::<ImageNode>(e)?.color),
        _ => return None,
    })
}

/// All applicable properties of an element.
pub fn get_props(world: &World, e: Entity) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    for key in std::iter::once(&"kind").chain(PROP_KEYS.iter()) {
        if let Some(v) = get_prop(world, e, key) {
            map.insert(key.to_string(), serde_json::Value::String(v));
        }
    }
    map
}

/// Writes one property (no undo; see [`apply_props_undoable`]).
pub fn set_prop(world: &mut World, e: Entity, key: &str, value: &str) -> Result<(), String> {
    if world.get::<Node>(e).is_none() {
        return Err("Entity is not a UI node".into());
    }
    macro_rules! node {
        (|$n:ident| $body:expr) => {{
            let mut $n = world.get_mut::<Node>(e).ok_or("Not a UI node")?;
            $body;
        }};
    }
    match key {
        "name" => {
            world.entity_mut(e).insert(Name::new(value.trim().to_string()));
        }
        "anchor" => apply_anchor(world, e, &value.trim().to_ascii_lowercase())?,
        "x" | "y" => {
            let v = parse_f32(value)?;
            node!(|n| write_offset_axis(&mut n, key == "x", v));
        }
        "width" => { let v = parse_val(value)?; node!(|n| n.width = v) }
        "height" => { let v = parse_val(value)?; node!(|n| n.height = v) }
        "min_width" => { let v = parse_val(value)?; node!(|n| n.min_width = v) }
        "min_height" => { let v = parse_val(value)?; node!(|n| n.min_height = v) }
        "max_width" => { let v = parse_val(value)?; node!(|n| n.max_width = v) }
        "max_height" => { let v = parse_val(value)?; node!(|n| n.max_height = v) }
        "left" => { let v = parse_val(value)?; node!(|n| n.left = v) }
        "right" => { let v = parse_val(value)?; node!(|n| n.right = v) }
        "top" => { let v = parse_val(value)?; node!(|n| n.top = v) }
        "bottom" => { let v = parse_val(value)?; node!(|n| n.bottom = v) }
        "position" => {
            let p = match value.trim().to_ascii_lowercase().as_str() {
                "absolute" => PositionType::Absolute,
                "relative" | "flow" => PositionType::Relative,
                _ => return Err("position is relative or absolute".into()),
            };
            node!(|n| n.position_type = p)
        }
        "display" => {
            let d = match value.trim().to_ascii_lowercase().as_str() {
                "flex" => Display::Flex,
                "grid" => Display::Grid,
                "block" => Display::Block,
                "none" => Display::None,
                _ => return Err("display is flex, grid, block or none".into()),
            };
            node!(|n| n.display = d)
        }
        "direction" => {
            let d = match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
                "row" | "horizontal" => FlexDirection::Row,
                "column" | "vertical" => FlexDirection::Column,
                "row_reverse" => FlexDirection::RowReverse,
                "column_reverse" => FlexDirection::ColumnReverse,
                _ => return Err("direction is row, column, row_reverse or column_reverse".into()),
            };
            node!(|n| n.flex_direction = d)
        }
        "justify" => { let v = parse_justify(value)?; node!(|n| n.justify_content = v) }
        "align" => { let v = parse_align(value)?; node!(|n| n.align_items = v) }
        "align_self" => { let v = parse_align_self(value)?; node!(|n| n.align_self = v) }
        "wrap" => { let v = parse_bool(value)?; node!(|n| n.flex_wrap = if v { FlexWrap::Wrap } else { FlexWrap::NoWrap }) }
        "gap" => {
            let v = parse_val(value)?;
            node!(|n| { n.row_gap = v; n.column_gap = v; })
        }
        "grow" => { let v = parse_f32(value)?; node!(|n| n.flex_grow = v) }
        "shrink" => { let v = parse_f32(value)?; node!(|n| n.flex_shrink = v) }
        "padding" => { let v = parse_rect(value)?; node!(|n| n.padding = v) }
        "margin" => { let v = parse_rect(value)?; node!(|n| n.margin = v) }
        "background" => {
            let c = parse_color(value)?;
            world.entity_mut(e).insert(BackgroundColor(c));
        }
        "border_width" => { let v = parse_val(value)?; node!(|n| n.border = UiRect::all(v)) }
        "border_color" => {
            let c = parse_color(value)?;
            world.entity_mut(e).insert(BorderColor::all(c));
        }
        "radius" => { let v = parse_val(value)?; node!(|n| n.border_radius = BorderRadius::all(v)) }
        "clip" => {
            let v = parse_bool(value)?;
            node!(|n| n.overflow = if v { Overflow::clip() } else { Overflow::visible() })
        }
        "visible" => {
            let v = parse_bool(value)?;
            world.entity_mut(e).insert(if v { Visibility::Inherited } else { Visibility::Hidden });
        }
        "text" | "font_size" | "text_color" | "text_align" => {
            let t = text_target(world, e).ok_or("This element has no text")?;
            match key {
                "text" => {
                    world.entity_mut(t).insert(Text::new(value.replace("\\n", "\n")));
                }
                "font_size" => {
                    let v = parse_f32(value)?.max(1.0);
                    let mut font = world.get::<TextFont>(t).cloned().unwrap_or_default();
                    font.font_size = FontSize::Px(v);
                    world.entity_mut(t).insert(font);
                }
                "text_color" => {
                    let c = parse_color(value)?;
                    world.entity_mut(t).insert(TextColor(c));
                }
                _ => {
                    let j = match value.trim().to_ascii_lowercase().as_str() {
                        "left" | "start" => Justify::Left,
                        "center" => Justify::Center,
                        "right" | "end" => Justify::Right,
                        "justified" => Justify::Justified,
                        _ => return Err("text_align is left, center, right or justified".into()),
                    };
                    let mut layout = world.get::<TextLayout>(t).cloned().unwrap_or_default();
                    layout.justify = j;
                    world.entity_mut(t).insert(layout);
                }
            }
        }
        "action" => {
            let a = parse_action(value, world.get::<UiButtonClickAction>(e))?;
            world.entity_mut(e).insert(a);
        }
        "action_arg" => {
            let mut a = world.get::<UiButtonClickAction>(e).cloned().ok_or("Not a button")?;
            match &mut a {
                UiButtonClickAction::LogMessage(m) | UiButtonClickAction::CustomAction(m) => *m = value.to_string(),
                _ => return Err("This action takes no argument".into()),
            }
            world.entity_mut(e).insert(a);
        }
        "value" | "min" | "max" | "step" => {
            let v = parse_f32(value)?;
            if let Some(mut p) = world.get_mut::<UiProgressBar>(e) {
                match key {
                    "value" => p.value = v,
                    "min" => p.min = v,
                    "max" => p.max = v,
                    _ => return Err("Progress bars have no step".into()),
                }
            } else if let Some(mut s) = world.get_mut::<UiSlider>(e) {
                match key {
                    "value" => s.value = v,
                    "min" => s.min = v,
                    "max" => s.max = v,
                    _ => s.step = v.max(0.0),
                }
            } else {
                return Err(format!("'{key}' applies to progress bars and sliders"));
            }
        }
        "checked" => {
            let v = parse_bool(value)?;
            world.get_mut::<UiCheckbox>(e).ok_or("Not a checkbox")?.checked = v;
        }
        "fill_color" => {
            let c = parse_color(value)?;
            if let Some(mut p) = world.get_mut::<UiProgressBar>(e) {
                p.fill_color = c;
            } else if let Some(mut s) = world.get_mut::<UiSlider>(e) {
                s.fill_color = c;
            } else if let Some(mut cb) = world.get_mut::<UiCheckbox>(e) {
                cb.mark_color = c;
            } else {
                return Err("fill_color applies to progress bars, sliders and checkboxes".into());
            }
        }
        "image" => {
            let path = value.trim().replace('\\', "/");
            let path = path.strip_prefix("assets/").unwrap_or(&path).to_string();
            let mut img = world.get::<ImageNode>(e).cloned().unwrap_or_default();
            img.image = if path.is_empty() { Handle::default() } else { world.resource::<AssetServer>().load(path) };
            world.entity_mut(e).insert(img);
        }
        "tint" => {
            let c = parse_color(value)?;
            let mut img = world.get::<ImageNode>(e).cloned().ok_or("Not an image")?;
            img.color = c;
            world.entity_mut(e).insert(img);
        }
        _ => return Err(format!("Unknown property '{key}' (known: {})", PROP_KEYS.join(", "))),
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Undo: component snapshots
// ---------------------------------------------------------------------------------------------

macro_rules! comp_snap {
    ($($f:ident : $t:ty),* $(,)?) => {
        /// Clone of every authoring component the UI editor can change on one entity.
        #[derive(Clone, Default)]
        pub struct CompSnap { $(pub $f: Option<$t>,)* }
        impl CompSnap {
            pub fn capture(world: &World, e: Entity) -> Self {
                Self { $($f: world.get::<$t>(e).cloned(),)* }
            }
            pub fn restore(&self, world: &mut World, e: Entity) {
                let Ok(mut em) = world.get_entity_mut(e) else { return };
                $(
                    match &self.$f {
                        Some(v) => { em.insert(v.clone()); }
                        None => { if em.contains::<$t>() { em.remove::<$t>(); } }
                    }
                )*
            }
        }
    };
}

comp_snap!(
    node: Node,
    background: BackgroundColor,
    border: BorderColor,
    text: Text,
    font: TextFont,
    text_color: TextColor,
    text_layout: TextLayout,
    visibility: Visibility,
    transform: UiTransform,
    image: ImageNode,
    name: Name,
    action: UiButtonClickAction,
    progress: UiProgressBar,
    checkbox: UiCheckbox,
    slider: UiSlider,
);

/// The element plus its widget parts (label, fill, thumb...), which property edits may touch.
fn snapshot_targets(world: &World, e: Entity) -> Vec<Entity> {
    fn walk(world: &World, e: Entity, out: &mut Vec<Entity>) {
        for c in world.get::<Children>(e).map(|c| c.to_vec()).unwrap_or_default() {
            if is_part(world, c) || (world.get::<Text>(c).is_some() && !is_game_ui_element(world, c) && world.get::<UiButtonClickAction>(e).is_some()) {
                out.push(c);
                walk(world, c, out);
            }
        }
    }
    let mut out = vec![e];
    walk(world, e, &mut out);
    out
}

pub struct SnapshotCommand {
    pub label: String,
    pub items: Vec<(Entity, CompSnap, CompSnap)>,
}

impl EditorCommand for SnapshotCommand {
    fn name(&self) -> &str {
        &self.label
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        for (e, _, after) in &self.items {
            after.restore(world, *e);
        }
        Ok(())
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        for (e, before, _) in &self.items {
            before.restore(world, *e);
        }
        Ok(())
    }
}

/// Applies several properties as one undo step. On error nothing is changed.
pub fn apply_props_undoable(world: &mut World, e: Entity, props: &[(String, String)]) -> Result<(), String> {
    let targets = snapshot_targets(world, e);
    let before: Vec<CompSnap> = targets.iter().map(|t| CompSnap::capture(world, *t)).collect();
    let mut sorted: Vec<&(String, String)> = props.iter().collect();
    sorted.sort_by_key(|(k, _)| prop_priority(k));
    for (k, v) in sorted {
        if let Err(err) = set_prop(world, e, k, v) {
            for (t, snap) in targets.iter().zip(&before) {
                snap.restore(world, *t);
            }
            return Err(format!("{k}: {err}"));
        }
    }
    let items: Vec<_> = targets.iter().zip(before).map(|(t, b)| (*t, b, CompSnap::capture(world, *t))).collect();
    let label = if props.len() == 1 { format!("UI: set {}", props[0].0) } else { "UI: edit properties".to_string() };
    reflect_fields::execute_command(world, SnapshotCommand { label, items })
}

/// Current anchor offset (logical px) of an element.
pub fn offset_of(world: &World, e: Entity) -> Vec2 {
    world.get::<Node>(e).map(read_offset).unwrap_or_default()
}

/// Sets an element's anchor offset (canvas drag).
pub fn set_offset(world: &mut World, e: Entity, off: Vec2) {
    if let Some(mut node) = world.get_mut::<Node>(e) {
        write_offset_axis(&mut node, true, off.x.round());
        write_offset_axis(&mut node, false, off.y.round());
    }
}

/// Records a change already applied to the world (e.g. a canvas drag) as one undo step.
pub fn record_change(world: &mut World, e: Entity, before: CompSnap, label: &str) {
    let after = CompSnap::capture(world, e);
    let _ = reflect_fields::execute_command(world, SnapshotCommand { label: label.into(), items: vec![(e, before, after)] });
}

// ---------------------------------------------------------------------------------------------
// Undo: structural changes (add / delete / duplicate / move)
// ---------------------------------------------------------------------------------------------

/// Reflection copy of an element subtree that can be re-spawned.
pub struct Subtree {
    dynamic: DynamicWorld,
    /// (child, parent) pairs in pre-order, so restoring keeps child order.
    links: Vec<(Entity, Entity)>,
    root: Entity,
    parent: Option<Entity>,
    index: usize,
}

fn descendants_preorder(world: &World, e: Entity, out: &mut Vec<Entity>) {
    out.push(e);
    for c in world.get::<Children>(e).map(|c| c.to_vec()).unwrap_or_default() {
        descendants_preorder(world, c, out);
    }
}

pub fn child_index(world: &World, parent: Entity, child: Entity) -> usize {
    world.get::<Children>(parent).and_then(|c| c.iter().position(|x| x == child)).unwrap_or(0)
}

impl Subtree {
    pub fn capture(world: &World, root: Entity) -> Self {
        let mut all = Vec::new();
        descendants_preorder(world, root, &mut all);
        let links = all
            .iter()
            .skip(1)
            .filter_map(|e| world.get::<ChildOf>(*e).map(|c| (*e, c.parent())))
            .collect();
        let parent = world.get::<ChildOf>(root).map(|c| c.parent());
        let index = parent.map(|p| child_index(world, p, root)).unwrap_or(0);
        let registry = world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        let dynamic = DynamicWorldBuilder::from_world(world, &registry)
            .deny_component::<ChildOf>()
            .deny_component::<Children>()
            .deny_component::<EditorInternal>()
            .deny_component::<PreviewEntity>()
            .deny_component::<ComputedNode>()
            .deny_component::<ComputedUiTargetCamera>()
            .deny_component::<ComputedUiRenderTargetInfo>()
            .deny_component::<UiGlobalTransform>()
            .deny_component::<InheritedVisibility>()
            .deny_component::<bevy::camera::visibility::ViewVisibility>()
            .deny_component::<bevy::camera::visibility::VisibilityClass>()
            .deny_component::<bevy::gizmos::transform_gizmo::TransformGizmoFocus>()
            .deny_component::<bevy::ui::Pressed>()
            .extract_entities(all.into_iter())
            .build();
        Self { dynamic, links, root, parent, index }
    }

    /// Spawns a fresh copy; returns the new root.
    pub fn spawn(&self, world: &mut World, parent: Option<Entity>, index: Option<usize>) -> Result<Entity, String> {
        let mut map = EntityHashMap::default();
        self.dynamic.write_to_world(world, &mut map).map_err(|e| format!("Cannot restore UI: {e}"))?;
        for (child, p) in &self.links {
            if let (Some(c), Some(p)) = (map.get(child), map.get(p)) {
                world.entity_mut(*c).insert(ChildOf(*p));
            }
        }
        let root = *map.get(&self.root).ok_or("Snapshot lost its root")?;
        if let Some(p) = parent.filter(|p| world.get_entity(*p).is_ok()) {
            let idx = index.unwrap_or(usize::MAX).min(world.get::<Children>(p).map(|c| c.len()).unwrap_or(0));
            world.entity_mut(p).insert_child(idx, root);
        }
        Ok(root)
    }

    pub fn restore(&self, world: &mut World) -> Result<Entity, String> {
        self.spawn(world, self.parent, Some(self.index))
    }
}

fn select(world: &mut World, e: Option<Entity>) {
    if let Some(mut sel) = world.get_resource_mut::<InspectorSelection>() {
        sel.0 = e;
    }
}

/// Undoable creation (add or duplicate). The entity already exists when the command runs first.
pub struct CreateCommand {
    pub entity: Entity,
    pub snapshot: Option<Subtree>,
}

impl EditorCommand for CreateCommand {
    fn name(&self) -> &str {
        "UI: add element"
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        if let Some(snap) = self.snapshot.take() {
            self.entity = snap.restore(world)?;
            select(world, Some(self.entity));
        }
        Ok(())
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        if world.get_entity(self.entity).is_err() {
            return Err("Element no longer exists".into());
        }
        self.snapshot = Some(Subtree::capture(world, self.entity));
        world.entity_mut(self.entity).despawn();
        select(world, None);
        Ok(())
    }
}

pub struct DeleteCommand {
    pub entity: Entity,
    pub snapshot: Option<Subtree>,
}

impl EditorCommand for DeleteCommand {
    fn name(&self) -> &str {
        "UI: delete element"
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        if world.get_entity(self.entity).is_err() {
            return Err("Element no longer exists".into());
        }
        self.snapshot = Some(Subtree::capture(world, self.entity));
        world.entity_mut(self.entity).despawn();
        if world.get_resource::<InspectorSelection>().and_then(|s| s.0) == Some(self.entity) {
            select(world, None);
        }
        Ok(())
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        let snap = self.snapshot.take().ok_or("Nothing to restore")?;
        self.entity = snap.restore(world)?;
        select(world, Some(self.entity));
        Ok(())
    }
}

/// Undoable reparent / reorder. `None` parent makes the element a root.
pub struct MoveCommand {
    pub entity: Entity,
    pub from: (Option<Entity>, usize),
    pub to: (Option<Entity>, usize),
}

fn place(world: &mut World, e: Entity, parent: Option<Entity>, index: usize) -> Result<(), String> {
    match parent {
        Some(p) => {
            if world.get_entity(p).is_err() {
                return Err("Parent no longer exists".into());
            }
            let len = world.get::<Children>(p).map(|c| c.len()).unwrap_or(0);
            let same = world.get::<ChildOf>(e).is_some_and(|c| c.parent() == p);
            let idx = index.min(if same { len.saturating_sub(1) } else { len });
            world.entity_mut(p).insert_child(idx, e);
        }
        None => {
            world.entity_mut(e).remove::<ChildOf>();
        }
    }
    Ok(())
}

impl EditorCommand for MoveCommand {
    fn name(&self) -> &str {
        "UI: move element"
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        place(world, self.entity, self.to.0, self.to.1)
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        place(world, self.entity, self.from.0, self.from.1)
    }
}

/// Moves `e` under `parent` (or to the root level) at `index`, as one undo step.
pub fn move_element(world: &mut World, e: Entity, parent: Option<Entity>, index: Option<usize>) -> Result<(), String> {
    if let Some(p) = parent {
        if p == e {
            return Err("An element cannot be its own parent".into());
        }
        let mut cur = Some(p);
        while let Some(c) = cur {
            if c == e {
                return Err("Cannot move an element into its own child".into());
            }
            cur = world.get::<ChildOf>(c).map(|x| x.parent());
        }
        if world.get::<Node>(p).is_none() {
            return Err("Parent is not a UI node".into());
        }
    }
    let old_parent = world.get::<ChildOf>(e).map(|c| c.parent());
    let from = (old_parent, old_parent.map(|p| child_index(world, p, e)).unwrap_or(0));
    let to_index = index.unwrap_or(usize::MAX);
    reflect_fields::execute_command(world, MoveCommand { entity: e, from, to: (parent, to_index) })
}

/// Moves `e` one slot up (-1) or down (+1) among its parent's children (widget parts are skipped).
pub fn shift_element(world: &mut World, e: Entity, delta: i32) -> Result<(), String> {
    let parent = world.get::<ChildOf>(e).map(|c| c.parent()).ok_or("Root elements have no sibling order")?;
    let siblings = ui_children(world, parent);
    let pos = siblings.iter().position(|x| *x == e).ok_or("Element not found under its parent")? as i32;
    let target = pos + delta;
    if target < 0 || target >= siblings.len() as i32 {
        return Ok(());
    }
    let other = siblings[target as usize];
    let raw = child_index(world, parent, other);
    move_element(world, e, Some(parent), Some(raw))
}

pub fn delete_element(world: &mut World, e: Entity) -> Result<(), String> {
    reflect_fields::execute_command(world, DeleteCommand { entity: e, snapshot: None })
}

pub fn duplicate_element(world: &mut World, e: Entity) -> Result<Entity, String> {
    let snap = Subtree::capture(world, e);
    let parent = world.get::<ChildOf>(e).map(|c| c.parent());
    let index = parent.map(|p| child_index(world, p, e) + 1);
    let copy = snap.spawn(world, parent, index)?;
    let base = display_name(world, e);
    let name = unique_name(world, &base);
    world.entity_mut(copy).insert(Name::new(name));
    if parent.is_none() {
        // Offset duplicated roots so the copy is visible.
        if let Some(mut node) = world.get_mut::<Node>(copy) {
            let off = read_offset(&node) + Vec2::splat(16.0);
            write_offset_axis(&mut node, true, off.x);
            write_offset_axis(&mut node, false, off.y);
        }
    }
    reflect_fields::execute_command(world, CreateCommand { entity: copy, snapshot: None })?;
    select(world, Some(copy));
    Ok(copy)
}

// ---------------------------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------------------------

fn part(world: &mut World, parent: Entity, role: UiPartRole, name: &str, bundle: impl Bundle) -> Entity {
    world.spawn((SceneEntity, Name::new(name.to_string()), UiWidgetPart { role }, ChildOf(parent), bundle)).id()
}

fn label_bundle(text: &str, size: f32) -> impl Bundle {
    (
        Pickable::IGNORE,
        Text::new(text.to_string()),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(Color::WHITE),
    )
}

/// Spawns an element with editor defaults. Roots are centered; children join their parent's flow.
pub fn spawn_element(world: &mut World, kind: UiElementKind, parent: Option<Entity>, text: Option<&str>, name: Option<&str>) -> Result<Entity, String> {
    if let Some(p) = parent {
        if world.get::<Node>(p).is_none() {
            return Err("Parent is not a UI node".into());
        }
    }
    let name = unique_name(world, name.unwrap_or(kind.label()));
    let mut node = Node::default();
    let mut bg = Color::NONE;
    match kind {
        UiElementKind::Panel => {
            node.width = Val::Px(320.0);
            node.height = Val::Px(200.0);
            node.flex_direction = FlexDirection::Column;
            node.padding = UiRect::all(Val::Px(12.0));
            node.row_gap = Val::Px(8.0);
            node.column_gap = Val::Px(8.0);
            node.border = UiRect::all(Val::Px(1.0));
            node.border_radius = BorderRadius::all(Val::Px(8.0));
            bg = Color::srgba(0.07, 0.09, 0.13, 0.88);
        }
        UiElementKind::Row | UiElementKind::Column => {
            node.flex_direction = if kind == UiElementKind::Row { FlexDirection::Row } else { FlexDirection::Column };
            node.align_items = AlignItems::Center;
            node.row_gap = Val::Px(8.0);
            node.column_gap = Val::Px(8.0);
            node.padding = UiRect::all(Val::Px(4.0));
            if parent.is_none() {
                node.min_width = Val::Px(120.0);
                node.min_height = Val::Px(40.0);
            }
        }
        UiElementKind::Text => {}
        UiElementKind::Button => {
            node.width = Val::Px(160.0);
            node.height = Val::Px(40.0);
            node.justify_content = JustifyContent::Center;
            node.align_items = AlignItems::Center;
            node.border_radius = BorderRadius::all(Val::Px(6.0));
            bg = Color::srgb(0.15, 0.45, 0.75);
        }
        UiElementKind::Image => {
            node.width = Val::Px(96.0);
            node.height = Val::Px(96.0);
        }
        UiElementKind::ProgressBar => {
            node.width = Val::Px(220.0);
            node.height = Val::Px(16.0);
            node.border_radius = BorderRadius::all(Val::Px(8.0));
            node.overflow = Overflow::clip();
            bg = Color::srgba(1.0, 1.0, 1.0, 0.12);
        }
        UiElementKind::Checkbox => {
            node.flex_direction = FlexDirection::Row;
            node.align_items = AlignItems::Center;
            node.column_gap = Val::Px(8.0);
        }
        UiElementKind::Slider => {
            node.width = Val::Px(220.0);
            node.height = Val::Px(20.0);
        }
        UiElementKind::Spacer => {
            node.flex_grow = 1.0;
            node.min_width = Val::Px(8.0);
            node.min_height = Val::Px(8.0);
        }
    }
    let mut ent = world.spawn((SceneEntity, Name::new(name), UiElement { kind }, node, BackgroundColor(bg)));
    if kind == UiElementKind::Panel {
        ent.insert(BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.14)));
    }
    let e = ent.id();
    match kind {
        UiElementKind::Text => {
            world.entity_mut(e).insert((
                Text::new(text.unwrap_or("Text").to_string()),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(Color::WHITE),
            ));
        }
        UiElementKind::Button => {
            world.entity_mut(e).insert(UiButtonClickAction::default());
            part(world, e, UiPartRole::Label, "Label", label_bundle(text.unwrap_or("Button"), 16.0));
        }
        UiElementKind::Image => {
            world.entity_mut(e).insert(ImageNode::default());
            if let Some(path) = text.filter(|t| !t.trim().is_empty()) {
                set_prop(world, e, "image", path)?;
            } else {
                world.entity_mut(e).insert(BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.15)));
            }
        }
        UiElementKind::ProgressBar => {
            let bar = UiProgressBar::default();
            let fill = Node { width: Val::Percent(bar.fraction() * 100.0), height: Val::Percent(100.0), border_radius: BorderRadius::all(Val::Px(8.0)), ..default() };
            part(world, e, UiPartRole::Fill, "Fill", (Pickable::IGNORE, fill, BackgroundColor(bar.fill_color)));
            world.entity_mut(e).insert(bar);
        }
        UiElementKind::Checkbox => {
            let cb = UiCheckbox::default();
            let bx = part(world, e, UiPartRole::Box, "Box", (
                Pickable::IGNORE,
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
                BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.6)),
            ));
            part(world, bx, UiPartRole::CheckMark, "Check", (
                Pickable::IGNORE,
                Node { width: Val::Px(10.0), height: Val::Px(10.0), border_radius: BorderRadius::all(Val::Px(2.0)), ..default() },
                BackgroundColor(cb.mark_color),
            ));
            part(world, e, UiPartRole::Label, "Label", label_bundle(text.unwrap_or("Checkbox"), 16.0));
            world.entity_mut(e).insert(cb);
        }
        UiElementKind::Slider => {
            let s = UiSlider::default();
            let track = part(world, e, UiPartRole::Track, "Track", (
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(7.0),
                    height: Val::Px(6.0),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.15)),
            ));
            part(world, track, UiPartRole::Fill, "Fill", (
                Pickable::IGNORE,
                Node { width: Val::Percent(s.fraction() * 100.0), height: Val::Percent(100.0), ..default() },
                BackgroundColor(s.fill_color),
            ));
            part(world, e, UiPartRole::Thumb, "Thumb", (
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(s.fraction() * 100.0),
                    top: Val::Px(2.0),
                    width: Val::Px(16.0),
                    height: Val::Px(16.0),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                UiTransform::from_translation(Val2::percent(-50.0, 0.0)),
                BackgroundColor(Color::WHITE),
            ));
            world.entity_mut(e).insert(s);
        }
        _ => {}
    }
    match parent {
        Some(p) => {
            world.entity_mut(e).insert(ChildOf(p));
        }
        None => {
            apply_anchor(world, e, "center")?;
        }
    }
    Ok(e)
}

/// Spawns as one undo step and selects the new element.
pub fn add_element(world: &mut World, kind: UiElementKind, parent: Option<Entity>, text: Option<&str>, name: Option<&str>) -> Result<Entity, String> {
    let e = spawn_element(world, kind, parent, text, name)?;
    reflect_fields::execute_command(world, CreateCommand { entity: e, snapshot: None })?;
    select(world, Some(e));
    Ok(e)
}

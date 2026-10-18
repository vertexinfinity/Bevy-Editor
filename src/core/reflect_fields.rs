//! Reflection-driven field access shared by the generic Inspector card and the remote API.
//!
//! Components are walked through `bevy_reflect`; `bevy_annotations` attributes
//! (`DisplayName`, `Tooltip`, `ValueRange`, `Step`, `Unit`, `ReadOnly`) shape labels and
//! limits. Bevy's own components carry few annotations yet, so a small built-in hint table
//! fills in units and ranges for well-known rendering and lighting fields.

use std::any::TypeId;

use bevy::{
    annotations::{DisplayName, ReadOnly, Step, Tooltip, Unit, ValueRange},
    ecs::{component::ComponentId, reflect::ReflectComponent},
    prelude::*,
    reflect::{
        attributes::CustomAttributes, enums::{DynamicEnum, DynamicVariant, VariantInfo},
        PartialReflect, ReflectPath, ReflectRef, TypeInfo, TypeRegistry,
    },
};

const MAX_DEPTH: usize = 4;
const MAX_FIELDS: usize = 64;

/// How a leaf field is edited.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    Float,
    Int,
    Bool,
    Text,
    Color,
    /// Unit-only enum; holds every variant name in declaration order.
    Enum(Vec<String>),
    /// Nested struct heading (no value of its own).
    Group,
    /// Opaque value edited as JSON through its serde implementation (e.g. `Exposure`).
    Json,
    /// Shown but not editable (handles, entities, collections, data-carrying enums).
    Opaque,
}

#[derive(Clone, Debug)]
pub struct FieldMeta {
    /// Reflect path relative to the component root; empty for the root itself.
    pub path: String,
    pub label: String,
    pub kind: FieldKind,
    pub depth: usize,
    pub unit: Option<&'static str>,
    pub tooltip: Option<&'static str>,
    pub range: Option<(f64, f64)>,
    pub step: Option<f64>,
    pub read_only: bool,
}

#[derive(Clone, Debug)]
pub struct ComponentMeta {
    pub type_path: String,
    pub name: String,
    pub tooltip: Option<&'static str>,
    pub fields: Vec<FieldMeta>,
    pub component_id: ComponentId,
}

/// Components that have a dedicated card, are editor plumbing, or are computed every frame.
const HIDDEN_SHORT_NAMES: &[&str] = &[
    "Transform", "GlobalTransform", "TransformTreeChanged", "Name", "ChildOf", "Children",
    "InheritedVisibility", "ViewVisibility", "VisibilityClass", "Mesh3d", "MeshMaterial3d<StandardMaterial>",
    "Aabb", "Frustum", "CascadesFrusta", "CubemapFrusta", "Cascades", "VisibleMeshEntities",
    "CubemapVisibleEntities", "CascadesVisibleEntities", "VisibleEntities", "EditorInternal",
    "SceneEntity", "PrimitiveMeta", "ScriptComponent", "PreviewEntity", "TransformGizmoFocus",
    "MeshPickingCamera", "TransformGizmoCamera", "SyncToRenderWorld", "RenderEntity", "MainEntity",
    "TemporaryRenderEntity", "CameraMainTextureUsages", "Lightmap", "LightBakeMode", "PreviewCamera",
    "MeshTag", "ComputedNode", "ComputedUiTargetCamera", "ComputedUiRenderTargetInfo",
    "CameraRenderGraph", "RenderVisibleEntities", "NoAutomaticBatching", "PickingInteraction",
    "Hovered", "DynamicWorldRoot", "NotShadowCaster", "NotShadowReceiver", "Wireframe", "ShowAabbGizmo", "WorldSettingsEntity", "WorldAssetRoot", "ManualTextureViewHandle",
];

/// ::b::Foo<c::Bar> -> Foo<Bar>.
fn short_name(type_path: &str) -> String {
    let mut out = String::new();
    let mut segment = String::new();
    for ch in type_path.chars() {
        match ch {
            ':' => segment.clear(),
            '<' | '>' | ',' | ' ' | '(' | ')' | '[' | ']' | ';' | '&' => {
                out.push_str(&segment);
                segment.clear();
                out.push(ch);
            }
            _ => segment.push(ch),
        }
    }
    out.push_str(&segment);
    out
}

pub fn is_hidden(type_path: &str) -> bool {
    let short = short_name(type_path);
    HIDDEN_SHORT_NAMES.contains(&short.as_str())
        || type_path.contains("::extract")
}

/// Turns `snake_case` or `CamelCase` into "Title Case".
pub fn prettify(name: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for (i, ch) in name.chars().enumerate() {
        if ch == '_' {
            out.push(' ');
            prev_lower = false;
            continue;
        }
        if ch.is_uppercase() && prev_lower {
            out.push(' ');
        }
        if i == 0 || out.ends_with(' ') {
            out.extend(ch.to_uppercase());
        } else {
            out.push(ch);
        }
        prev_lower = ch.is_lowercase() || ch.is_ascii_digit();
    }
    out
}

/// Fallback annotations for Bevy types that do not carry `bevy_annotations` yet.
struct Hint {
    unit: Option<&'static str>,
    range: Option<(f64, f64)>,
    step: Option<f64>,
    tooltip: Option<&'static str>,
}

fn builtin_hint(component: &str, path: &str) -> Option<Hint> {
    let h = |unit, range, step, tooltip| Some(Hint { unit, range, step, tooltip });
    match (component, path) {
        ("PointLight", "intensity") | ("SpotLight", "intensity") => h(Some("lm"), Some((0.0, 1.0e8)), Some(10_000.0), Some("Luminous power")),
        ("PointLight", "range") | ("SpotLight", "range") => h(Some("m"), Some((0.0, 1000.0)), Some(1.0), Some("Distance at which the light stops affecting surfaces")),
        ("PointLight", "radius") | ("SpotLight", "radius") => h(Some("m"), Some((0.0, 100.0)), Some(0.05), Some("Size of the light source; softens highlights")),
        ("SpotLight", "inner_angle") | ("SpotLight", "outer_angle") => h(Some("rad"), Some((0.0, std::f64::consts::FRAC_PI_2)), Some(0.05), None),
        ("DirectionalLight", "illuminance") => h(Some("lx"), Some((0.0, 200_000.0)), Some(1000.0), Some("Sunlight is roughly 100,000 lx, overcast 1,000 lx")),
        (_, "shadow_depth_bias") | (_, "shadow_normal_bias") => h(None, Some((0.0, 10.0)), Some(0.01), None),
        ("Exposure", "") => h(Some("EV"), Some((-10.0, 20.0)), Some(0.25), Some("Exposure value at ISO 100; Bevy's physical default is 9.7")),
        ("Bloom", "intensity") => h(None, Some((0.0, 1.0)), Some(0.01), Some("Strength of the glow around bright pixels")),
        ("Bloom", "low_frequency_boost") | ("Bloom", "low_frequency_boost_curvature") | ("Bloom", "high_pass_frequency") => h(None, Some((0.0, 1.0)), Some(0.05), None),
        ("ScreenSpaceAmbientOcclusion", "constant_object_thickness") => h(Some("m"), Some((0.0, 4.0)), Some(0.05), None),
        ("DepthOfField", "focal_distance") => h(Some("m"), Some((0.0, 1000.0)), Some(0.25), Some("Distance that stays in sharp focus")),
        ("DepthOfField", "aperture_f_stops") => h(Some("f"), Some((0.5, 64.0)), Some(0.5), Some("Lower values give a shallower depth of field")),
        ("DepthOfField", "sensor_height") => h(Some("m"), Some((0.001, 0.1)), Some(0.001), None),
        ("MotionBlur", "shutter_angle") => h(None, Some((0.0, 2.0)), Some(0.05), Some("0.5 matches a 180° film shutter")),
        ("MotionBlur", "samples") => h(None, Some((0.0, 64.0)), Some(1.0), None),
        ("ChromaticAberration", "intensity") | ("Vignette", "intensity") => h(None, Some((0.0, 1.0)), Some(0.01), None),
        ("ContrastAdaptiveSharpening", "sharpening_strength") => h(None, Some((0.0, 1.0)), Some(0.05), None),
        ("DistanceFog", _) if path.ends_with("start") || path.ends_with("end") => h(Some("m"), Some((0.0, 10_000.0)), Some(1.0), None),
        ("DistanceFog", _) if path.ends_with("density") => h(None, Some((0.0, 1.0)), Some(0.005), None),
        ("Projection", p) if p.ends_with("fov") => h(Some("rad"), Some((0.05, 3.1)), Some(0.05), Some("Vertical field of view")),
        ("Projection", p) if p.ends_with("near") || p.ends_with("far") => h(Some("m"), Some((0.0001, 100_000.0)), Some(0.1), None),
        ("Camera", "order") => h(None, Some((-100.0, 100.0)), Some(1.0), Some("Higher orders render later, on top")),
        ("VolumetricFog", "step_count") => h(None, Some((1.0, 256.0)), Some(1.0), None),
        ("GlobalAmbientLight", "brightness") | ("AmbientLight", "brightness") => h(Some("cd/m²"), Some((0.0, 100_000.0)), Some(50.0), None),
        ("ColorGrading", p) if p.ends_with("exposure") => h(Some("EV"), Some((-8.0, 8.0)), Some(0.1), None),
        ("ColorGrading", p) if p.ends_with("temperature") || p.ends_with("tint") || p.ends_with("hue") => h(None, Some((-1.0, 1.0)), Some(0.02), None),
        ("ColorGrading", p) if p.ends_with("saturation") || p.ends_with("contrast") || p.ends_with("gamma") || p.ends_with("gain") || p.ends_with("lift") => h(None, Some((0.0, 4.0)), Some(0.05), None),
        _ => None,
    }
}

fn attributes_meta(attrs: Option<&CustomAttributes>) -> (Option<&'static str>, Option<&'static str>, Option<(f64, f64)>, Option<f64>, Option<&'static str>, bool) {
    let Some(attrs) = attrs else {
        return (None, None, None, None, None, false);
    };
    (
        attrs.get::<DisplayName>().map(|d| d.0),
        attrs.get::<Tooltip>().map(|t| t.0),
        ValueRange::from_attributes(attrs).map(|r| (r.min(), r.max())),
        attrs.get::<Step>().map(|s| s.0),
        attrs.get::<Unit>().map(|u| u.0),
        attrs.contains::<ReadOnly>(),
    )
}

fn join(path: &str, segment: &str) -> String {
    if segment.starts_with('.') || segment.starts_with('[') {
        format!("{path}{segment}")
    } else if path.is_empty() {
        segment.to_string()
    } else {
        format!("{path}.{segment}")
    }
}

fn leaf_kind(value: &dyn PartialReflect) -> Option<FieldKind> {
    if value.try_downcast_ref::<f32>().is_some() || value.try_downcast_ref::<f64>().is_some() {
        return Some(FieldKind::Float);
    }
    macro_rules! ints { ($($t:ty),*) => { $( if value.try_downcast_ref::<$t>().is_some() { return Some(FieldKind::Int); } )* }; }
    ints!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);
    if value.try_downcast_ref::<bool>().is_some() {
        return Some(FieldKind::Bool);
    }
    if value.try_downcast_ref::<String>().is_some() {
        return Some(FieldKind::Text);
    }
    if value.try_downcast_ref::<Color>().is_some() {
        return Some(FieldKind::Color);
    }
    let path = value.reflect_type_path();
    if path.starts_with("bevy_asset::handle::Handle") || path.starts_with("bevy_ecs::entity::Entity") {
        return Some(FieldKind::Opaque);
    }
    None
}

struct Walker<'a> {
    registry: Option<&'a TypeRegistry>,
    component: String,
    out: &'a mut Vec<FieldMeta>,
}

impl Walker<'_> {
    #[allow(clippy::too_many_arguments)]
    fn push(&mut self, path: String, label: String, kind: FieldKind, depth: usize, attrs: Option<&CustomAttributes>, parent_read_only: bool) {
        if self.out.len() >= MAX_FIELDS {
            return;
        }
        let (display, tooltip, range, step, unit, read_only) = attributes_meta(attrs);
        let hint = builtin_hint(&self.component, &path);
        self.out.push(FieldMeta {
            label: display.map(str::to_string).unwrap_or(label),
            kind,
            depth,
            unit: unit.or_else(|| hint.as_ref().and_then(|h| h.unit)),
            tooltip: tooltip.or_else(|| hint.as_ref().and_then(|h| h.tooltip)),
            range: range.or_else(|| hint.as_ref().and_then(|h| h.range)),
            step: step.or_else(|| hint.as_ref().and_then(|h| h.step)),
            read_only: read_only || parent_read_only,
            path,
        });
    }

    fn walk(&mut self, value: &dyn PartialReflect, path: String, label: String, depth: usize, attrs: Option<&CustomAttributes>, read_only: bool) {
        if let Some(kind) = leaf_kind(value) {
            let ro = read_only || kind == FieldKind::Opaque;
            self.push(path, label, kind, depth, attrs, ro);
            return;
        }
        let read_only = read_only || attrs.is_some_and(|a| a.contains::<ReadOnly>());
        if opaque_number(value).is_some() {
            let label = if path.is_empty() { "EV100".to_string() } else { label };
            self.push(path, label, FieldKind::Float, depth, attrs, read_only);
            return;
        }
        if matches!(value.reflect_ref(), ReflectRef::Opaque(_)) && self.registry.is_some_and(|r| json_codec(r, value).is_some()) {
            self.push(path, label, FieldKind::Json, depth, attrs, read_only);
            return;
        }
        let info = value.get_represented_type_info();
        match value.reflect_ref() {
            ReflectRef::Struct(s) => {
                if depth >= MAX_DEPTH {
                    self.push(path, label, FieldKind::Opaque, depth, attrs, true);
                    return;
                }
                let struct_info = match info {
                    Some(TypeInfo::Struct(i)) => Some(i),
                    _ => None,
                };
                if depth > 0 || !path.is_empty() {
                    self.push(path.clone(), label, FieldKind::Group, depth, attrs, read_only);
                }
                for i in 0..s.field_len() {
                    let (Some(name), Some(field)) = (s.name_at(i), s.field_at(i)) else { continue };
                    // Engine-computed state (e.g. Camera::computed) is noise in an editor.
                    if name == "computed" {
                        continue;
                    }
                    let field_attrs = struct_info.and_then(|si| si.field_at(i)).map(|f| f.custom_attributes());
                    let child_depth = if path.is_empty() { depth } else { depth + 1 };
                    self.walk(field, join(&path, name), prettify(name), child_depth, field_attrs, read_only);
                }
            }
            ReflectRef::TupleStruct(ts) => {
                let ts_info = match info {
                    Some(TypeInfo::TupleStruct(i)) => Some(i),
                    _ => None,
                };
                if ts.field_len() == 1 {
                    let field_attrs = ts_info.and_then(|i| i.field_at(0)).map(|f| f.custom_attributes()).or(attrs);
                    if let Some(field) = ts.field(0) {
                        self.walk(field, join(&path, ".0"), label, depth, field_attrs, read_only);
                    }
                    return;
                }
                if !path.is_empty() {
                    self.push(path.clone(), label, FieldKind::Group, depth, attrs, read_only);
                }
                for i in 0..ts.field_len() {
                    let Some(field) = ts.field(i) else { continue };
                    let field_attrs = ts_info.and_then(|ti| ti.field_at(i)).map(|f| f.custom_attributes());
                    self.walk(field, join(&path, &format!(".{i}")), format!("{i}"), depth + 1, field_attrs, read_only);
                }
            }
            ReflectRef::Enum(e) => {
                let enum_info = match info {
                    Some(TypeInfo::Enum(i)) => Some(i),
                    _ => None,
                };
                let all_unit = enum_info.is_some_and(|i| i.iter().all(|v| matches!(v, VariantInfo::Unit(_))));
                if all_unit {
                    let names = enum_info.map(|i| i.variant_names().iter().map(|n| n.to_string()).collect()).unwrap_or_default();
                    self.push(path, label, FieldKind::Enum(names), depth, attrs, read_only);
                    return;
                }
                if e.field_len() == 0 {
                    self.push(path, format!("{label}: {}", e.variant_name()), FieldKind::Opaque, depth, attrs, true);
                    return;
                }
                if depth >= MAX_DEPTH {
                    self.push(path, label, FieldKind::Opaque, depth, attrs, true);
                    return;
                }
                // Option<T> and similar newtype variants read best inline.
                if e.field_len() == 1 && e.name_at(0).is_none() {
                    if let Some(field) = e.field_at(0) {
                        self.walk(field, join(&path, ".0"), label, depth, attrs, read_only);
                    }
                    return;
                }
                self.push(path.clone(), format!("{label} ({})", e.variant_name()), FieldKind::Group, depth, attrs, read_only);
                let variant_info = enum_info.and_then(|i| i.variant(e.variant_name()));
                for i in 0..e.field_len() {
                    let Some(field) = e.field_at(i) else { continue };
                    let field_attrs = match variant_info {
                        Some(VariantInfo::Struct(v)) => v.field_at(i).map(|f| f.custom_attributes()),
                        Some(VariantInfo::Tuple(v)) => v.field_at(i).map(|f| f.custom_attributes()),
                        _ => None,
                    };
                    let (segment, name) = match e.name_at(i) {
                        Some(n) => (n.to_string(), prettify(n)),
                        None => (format!(".{i}"), format!("{i}")),
                    };
                    self.walk(field, join(&path, &segment), name, depth + 1, field_attrs, read_only);
                }
            }
            ReflectRef::List(l) => self.push(path, format!("{label} [{}]", l.len()), FieldKind::Opaque, depth, attrs, true),
            ReflectRef::Array(a) => self.push(path, format!("{label} [{}]", a.len()), FieldKind::Opaque, depth, attrs, true),
            ReflectRef::Map(m) => self.push(path, format!("{label} {{{}}}", m.len()), FieldKind::Opaque, depth, attrs, true),
            ReflectRef::Set(s) => self.push(path, format!("{label} {{{}}}", s.len()), FieldKind::Opaque, depth, attrs, true),
            _ => self.push(path, label, FieldKind::Opaque, depth, attrs, true),
        }
    }
}

fn type_attrs(value: &dyn PartialReflect) -> Option<&'static CustomAttributes> {
    match value.get_represented_type_info()? {
        TypeInfo::Struct(i) => Some(i.custom_attributes()),
        TypeInfo::TupleStruct(i) => Some(i.custom_attributes()),
        TypeInfo::Enum(i) => Some(i.custom_attributes()),
        _ => None,
    }
}

/// Lists the editable fields of one reflected component value.
pub fn describe_value(registry: Option<&TypeRegistry>, type_path: &str, value: &dyn PartialReflect) -> Vec<FieldMeta> {
    let mut out = Vec::new();
    let mut walker = Walker { registry, component: short_name(type_path), out: &mut out };
    walker.walk(value, String::new(), short_name(type_path), 0, None, type_attrs(value).is_some_and(|a| a.contains::<ReadOnly>()));
    out
}

/// Reflected components on `entity`, in archetype order, minus hidden plumbing.
pub fn reflected_components(world: &World, entity: Entity) -> Vec<(ComponentId, TypeId, String)> {
    let Ok(entity_ref) = world.get_entity(entity) else { return Vec::new() };
    let registry = world.resource::<AppTypeRegistry>().read();
    let mut out = Vec::new();
    for id in entity_ref.archetype().components() {
        let Some(info) = world.components().get_info(*id) else { continue };
        let Some(type_id) = info.type_id() else { continue };
        let Some(registration) = registry.get(type_id) else { continue };
        if registration.data::<ReflectComponent>().is_none() {
            continue;
        }
        let type_path = registration.type_info().type_path().to_string();
        if is_hidden(&type_path) {
            continue;
        }
        out.push((*id, type_id, type_path));
    }
    out
}

/// Full description of every visible reflected component on `entity`.
pub fn describe_entity(world: &World, entity: Entity) -> Vec<ComponentMeta> {
    let registry = world.resource::<AppTypeRegistry>().read();
    let Ok(entity_ref) = world.get_entity(entity) else { return Vec::new() };
    reflected_components(world, entity)
        .into_iter()
        .filter_map(|(component_id, type_id, type_path)| {
            let reflect = registry.get_type_data::<ReflectComponent>(type_id)?;
            let value = reflect.reflect(entity_ref)?;
            let tooltip = type_attrs(value.as_partial_reflect()).and_then(|a| a.get::<Tooltip>()).map(|t| t.0);
            let name = type_attrs(value.as_partial_reflect())
                .and_then(|a| a.get::<DisplayName>())
                .map(|d| d.0.to_string())
                .unwrap_or_else(|| prettify(&short_name(&type_path)));
            Some(ComponentMeta {
                fields: describe_value(Some(&registry), &type_path, value.as_partial_reflect()),
                type_path,
                name,
                tooltip,
                component_id,
            })
        })
        .collect()
}

/// Finds a registered component type by full or short type path.
pub fn find_component_type(registry: &TypeRegistry, name: &str) -> Option<TypeId> {
    registry
        .iter()
        .filter(|r| r.data::<ReflectComponent>().is_some())
        .find(|r| {
            let path = r.type_info().type_path();
            path == name || short_name(path) == name || r.type_info().type_path_table().ident() == Some(name)
        })
        .map(|r| r.type_id())
}

/// Resolves a component name against the entity's own components first, so short names like
/// `Exposure` pick the type actually present rather than the first registry match.
fn resolve_on_entity(world: &World, registry: &TypeRegistry, entity: Entity, name: &str) -> Option<TypeId> {
    let on_entity = world.get_entity(entity).ok().and_then(|entity_ref| {
        entity_ref.archetype().components().iter().find_map(|id| {
            let type_id = world.components().get_info(*id)?.type_id()?;
            let registration = registry.get(type_id)?;
            registration.data::<ReflectComponent>()?;
            let path = registration.type_info().type_path();
            (path == name || short_name(path) == name).then_some(type_id)
        })
    });
    on_entity.or_else(|| find_component_type(registry, name))
}

/// Numeric views of opaque Bevy types that reflect no fields (read, write).
fn opaque_number(value: &dyn PartialReflect) -> Option<f64> {
    value.try_downcast_ref::<bevy::camera::Exposure>().map(|e| e.ev100 as f64)
}

fn set_opaque_number(value: &mut dyn PartialReflect, v: f64) -> bool {
    match value.try_downcast_mut::<bevy::camera::Exposure>() {
        Some(e) => {
            e.ev100 = v as f32;
            true
        }
        None => false,
    }
}

/// Serde codec for an opaque reflected value, when its type registers both directions.
fn json_codec<'r>(registry: &'r TypeRegistry, value: &dyn PartialReflect) -> Option<(&'r bevy::reflect::ReflectSerialize, &'r bevy::reflect::ReflectDeserialize)> {
    let type_id = value.get_represented_type_info()?.type_id();
    Some((registry.get_type_data(type_id)?, registry.get_type_data(type_id)?))
}

fn field_ref<'a>(value: &'a dyn PartialReflect, path: &str) -> Result<&'a dyn PartialReflect, String> {
    if path.is_empty() {
        return Ok(value);
    }
    path.reflect_element(value).map_err(|e| format!("Invalid field path '{path}': {e}"))
}

pub fn format_value(value: &dyn PartialReflect) -> String {
    if let Some(v) = value.try_downcast_ref::<f32>() {
        return trim_float(*v as f64);
    }
    if let Some(v) = value.try_downcast_ref::<f64>() {
        return trim_float(*v);
    }
    if let Some(v) = value.try_downcast_ref::<bool>() {
        return v.to_string();
    }
    if let Some(v) = value.try_downcast_ref::<String>() {
        return v.clone();
    }
    if let Some(c) = value.try_downcast_ref::<Color>() {
        return color_hex(*c);
    }
    macro_rules! ints { ($($t:ty),*) => { $( if let Some(v) = value.try_downcast_ref::<$t>() { return v.to_string(); } )* }; }
    ints!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);
    if let ReflectRef::Enum(e) = value.reflect_ref() {
        if e.field_len() == 0 {
            return e.variant_name().to_string();
        }
    }
    let text = format!("{value:?}");
    if text.chars().count() > 48 {
        format!("{}…", text.chars().take(47).collect::<String>())
    } else {
        text
    }
}

fn trim_float(v: f64) -> String {
    if v.abs() >= 1.0e9 || (v != 0.0 && v.abs() < 1.0e-4) {
        return format!("{v:e}");
    }
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

pub fn color_hex(color: Color) -> String {
    let c = color.to_srgba();
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        (c.red.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.green.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.alpha.clamp(0.0, 1.0) * 255.0).round() as u8
    )
}

pub fn parse_color(value: &str) -> Result<Color, String> {
    let hex = value.trim().trim_start_matches('#');
    if !matches!(hex.len(), 6 | 8) || !hex.is_ascii() {
        return Err("Use #RRGGBB or #RRGGBBAA".into());
    }
    let mut c = [255u8; 4];
    for (i, channel) in c.iter_mut().enumerate().take(hex.len() / 2) {
        *channel = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| "Invalid hexadecimal color")?;
    }
    Ok(Color::srgba_u8(c[0], c[1], c[2], c[3]))
}

/// Reads one field of a component as display text.
pub fn read_field(world: &World, entity: Entity, type_path: &str, path: &str) -> Option<String> {
    let registry = world.resource::<AppTypeRegistry>().read();
    let type_id = resolve_on_entity(world, &registry, entity, type_path)?;
    let reflect = registry.get_type_data::<ReflectComponent>(type_id)?;
    let value = reflect.reflect(world.get_entity(entity).ok()?)?;
    let field = field_ref(value.as_partial_reflect(), path).ok()?;
    if let Some(v) = opaque_number(field) {
        return Some(trim_float(v));
    }
    if leaf_kind(field).is_none() && matches!(field.reflect_ref(), ReflectRef::Opaque(_)) {
        if let Some(text) = json_codec(&registry, field).and_then(|(ser, _)| {
            let mut buf = Vec::new();
            ser.serialize(field.try_as_reflect()?, &mut serde_json::Serializer::new(&mut buf)).ok()?;
            String::from_utf8(buf).ok()
        }) {
            return Some(text);
        }
    }
    Some(format_value(field))
}

/// Metadata for one field, used to validate and clamp edits.
pub fn field_meta(world: &World, entity: Entity, type_path: &str, path: &str) -> Option<FieldMeta> {
    let registry = world.resource::<AppTypeRegistry>().read();
    let type_id = resolve_on_entity(world, &registry, entity, type_path)?;
    let reflect = registry.get_type_data::<ReflectComponent>(type_id)?;
    let value = reflect.reflect(world.get_entity(entity).ok()?)?;
    let full_path = registry.get(type_id)?.type_info().type_path();
    describe_value(Some(&registry), full_path, value.as_partial_reflect()).into_iter().find(|f| f.path == path)
}

fn clamp(meta: Option<&FieldMeta>, v: f64) -> f64 {
    match meta.and_then(|m| m.range) {
        Some((lo, hi)) => v.clamp(lo, hi),
        None => v,
    }
}

/// Parses `value` into the field's type and writes it, with range clamping from annotations.
pub fn write_field(world: &mut World, entity: Entity, type_path: &str, path: &str, value: &str) -> Result<(), String> {
    let meta = field_meta(world, entity, type_path, path);
    if meta.as_ref().is_some_and(|m| m.read_only) {
        return Err("This field is read-only".into());
    }
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    let type_id = resolve_on_entity(world, &registry, entity, type_path).ok_or_else(|| format!("Unknown component '{type_path}'"))?;
    let reflect = registry.get_type_data::<ReflectComponent>(type_id).ok_or("Component is not reflectable")?;
    let mut entity_mut = world.get_entity_mut(entity).map_err(|_| "Entity no longer exists")?;
    let mut component = reflect.reflect_mut(&mut entity_mut).ok_or("Entity does not have this component")?;
    let root = component.as_partial_reflect_mut();
    let field: &mut dyn PartialReflect = if path.is_empty() {
        root
    } else {
        path.reflect_element_mut(root).map_err(|e| format!("Invalid field path '{path}': {e}"))?
    };
    let trimmed = value.trim();
    let number = || -> Result<f64, String> {
        trimmed.parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| "Enter a finite number".to_string())
    };
    if let Some(v) = field.try_downcast_mut::<f32>() {
        *v = clamp(meta.as_ref(), number()?) as f32;
        return Ok(());
    }
    if opaque_number(field).is_some() {
        set_opaque_number(field, clamp(meta.as_ref(), number()?));
        return Ok(());
    }
    if let Some(v) = field.try_downcast_mut::<f64>() {
        *v = clamp(meta.as_ref(), number()?);
        return Ok(());
    }
    macro_rules! ints {
        ($($t:ty),*) => { $(
            if let Some(v) = field.try_downcast_mut::<$t>() {
                let n = clamp(meta.as_ref(), number()?.round());
                *v = <$t>::try_from(n as i128).map_err(|_| format!("Value out of range for {}", stringify!($t)))?;
                return Ok(());
            }
        )* };
    }
    ints!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);
    if let Some(v) = field.try_downcast_mut::<bool>() {
        *v = trimmed.parse().map_err(|_| "Enter true or false")?;
        return Ok(());
    }
    if let Some(v) = field.try_downcast_mut::<String>() {
        *v = value.to_string();
        return Ok(());
    }
    if let Some(v) = field.try_downcast_mut::<Color>() {
        *v = parse_color(trimmed)?;
        return Ok(());
    }
    if let Some(FieldKind::Enum(variants)) = meta.as_ref().map(|m| &m.kind) {
        let variant = variants
            .iter()
            .find(|v| v.eq_ignore_ascii_case(trimmed))
            .ok_or_else(|| format!("Choose one of: {}", variants.join(", ")))?;
        let dynamic = DynamicEnum::new(variant.clone(), DynamicVariant::Unit);
        return field.try_apply(&dynamic).map_err(|e| format!("Cannot set variant: {e}"));
    }
    if let Some((_, de)) = json_codec(&registry, field) {
        let parsed = de.deserialize(&mut serde_json::Deserializer::from_str(trimmed)).map_err(|e| format!("Invalid JSON value: {e}"))?;
        return field.try_apply(parsed.as_partial_reflect()).map_err(|e| format!("Cannot apply value: {e}"));
    }
    Err("This field type cannot be edited as text".into())
}

/// For toggle-style fields (bool, unit enum) returns the value a single click should set.
pub fn next_toggle_value(world: &World, entity: Entity, type_path: &str, path: &str) -> Option<String> {
    let meta = field_meta(world, entity, type_path, path)?;
    if meta.read_only {
        return None;
    }
    let current = read_field(world, entity, type_path, path)?;
    match meta.kind {
        FieldKind::Bool => Some((current != "true").to_string()),
        FieldKind::Enum(variants) if !variants.is_empty() => {
            let i = variants.iter().position(|v| *v == current).map(|i| (i + 1) % variants.len()).unwrap_or(0);
            Some(variants[i].clone())
        }
        _ => None,
    }
}

/// Adds `steps` increments to a numeric field, honouring `Step` and `ValueRange`.
pub fn nudge_field(world: &mut World, entity: Entity, type_path: &str, path: &str, steps: f64) -> Result<(String, String), String> {
    let meta = field_meta(world, entity, type_path, path).ok_or("Field not found")?;
    let before = read_field(world, entity, type_path, path).ok_or("Field not found")?;
    let current: f64 = before.parse().map_err(|_| "Field is not numeric")?;
    let step = meta.step.unwrap_or(match meta.kind {
        FieldKind::Int => 1.0,
        _ => {
            let magnitude = current.abs();
            if magnitude >= 1000.0 { 100.0 } else if magnitude >= 10.0 { 1.0 } else { 0.1 }
        }
    });
    let after = trim_float(clamp(Some(&meta), current + step * steps));
    Ok((before, after))
}

/// Inserts a component by type path using its reflected `Default`.
pub fn insert_default_component(world: &mut World, entity: Entity, type_path: &str) -> Result<String, String> {
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    let type_id = find_component_type(&registry, type_path).ok_or_else(|| format!("Unknown component '{type_path}'"))?;
    let registration = registry.get(type_id).ok_or("Type is not registered")?;
    let default = registration
        .data::<ReflectDefault>()
        .ok_or_else(|| format!("'{type_path}' has no reflected Default"))?
        .default();
    let reflect = registration.data::<ReflectComponent>().ok_or("Type is not a component")?;
    let mut entity_mut = world.get_entity_mut(entity).map_err(|_| "Entity no longer exists")?;
    reflect.insert(&mut entity_mut, default.as_partial_reflect(), &registry);
    Ok(registration.type_info().type_path().to_string())
}

/// Removes a component by type path, returning a clone of the removed value for undo.
pub fn remove_component(world: &mut World, entity: Entity, type_path: &str) -> Result<Box<dyn PartialReflect>, String> {
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    let type_id = resolve_on_entity(world, &registry, entity, type_path).ok_or_else(|| format!("Unknown component '{type_path}'"))?;
    let reflect = registry.get_type_data::<ReflectComponent>(type_id).ok_or("Type is not a component")?;
    let snapshot = {
        let entity_ref = world.get_entity(entity).map_err(|_| "Entity no longer exists")?;
        let value = reflect.reflect(entity_ref).ok_or("Entity does not have this component")?;
        value.reflect_clone().map_err(|e| format!("Cannot snapshot component: {e}"))?.into_partial_reflect()
    };
    let mut entity_mut = world.get_entity_mut(entity).map_err(|_| "Entity no longer exists")?;
    reflect.remove(&mut entity_mut);
    Ok(snapshot)
}

/// Re-inserts a previously removed component snapshot.
pub fn restore_component(world: &mut World, entity: Entity, type_path: &str, value: &dyn PartialReflect) -> Result<(), String> {
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    let type_id = find_component_type(&registry, type_path).ok_or_else(|| format!("Unknown component '{type_path}'"))?;
    let reflect = registry.get_type_data::<ReflectComponent>(type_id).ok_or("Type is not a component")?;
    let mut entity_mut = world.get_entity_mut(entity).map_err(|_| "Entity no longer exists")?;
    reflect.insert(&mut entity_mut, value, &registry);
    Ok(())
}

/// Undoable edit of one reflected field, stored as text so it replays through the same parser.
pub struct SetReflectField {
    pub entity: Entity,
    pub type_path: String,
    pub path: String,
    pub before: String,
    pub after: String,
}

impl super::commands::EditorCommand for SetReflectField {
    fn name(&self) -> &str {
        "Edit component field"
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        write_field(world, self.entity, &self.type_path, &self.path, &self.after)
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        write_field(world, self.entity, &self.type_path, &self.path, &self.before)
    }
}

/// Undoable insert (with reflected `Default`) or removal of a component.
pub struct ToggleReflectComponent {
    pub entity: Entity,
    pub type_path: String,
    pub insert: bool,
    snapshot: Option<Box<dyn PartialReflect>>,
}

impl ToggleReflectComponent {
    pub fn insert(entity: Entity, type_path: impl Into<String>) -> Self {
        Self { entity, type_path: type_path.into(), insert: true, snapshot: None }
    }
    pub fn remove(entity: Entity, type_path: impl Into<String>) -> Self {
        Self { entity, type_path: type_path.into(), insert: false, snapshot: None }
    }
    fn add(&mut self, world: &mut World) -> Result<(), String> {
        match &self.snapshot {
            Some(value) => restore_component(world, self.entity, &self.type_path, value.as_ref()),
            None => insert_default_component(world, self.entity, &self.type_path).map(|_| ()),
        }
    }
    fn take(&mut self, world: &mut World) -> Result<(), String> {
        self.snapshot = Some(remove_component(world, self.entity, &self.type_path)?);
        Ok(())
    }
}

impl super::commands::EditorCommand for ToggleReflectComponent {
    fn name(&self) -> &str {
        if self.insert { "Add component" } else { "Remove component" }
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        if self.insert { self.add(world) } else { self.take(world) }
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        if self.insert { self.take(world) } else { self.add(world) }
    }
}

/// Runs a command through the shared undo history.
pub fn execute_command(world: &mut World, command: impl super::commands::EditorCommand) -> Result<(), String> {
    if !world.contains_resource::<super::commands::CommandHistory>() {
        let mut command = command;
        return command.execute(world);
    }
    world.resource_scope(|world, mut history: Mut<super::commands::CommandHistory>| history.execute(Box::new(command), world))
}

/// Sets a field through the undo history, skipping no-op edits.
pub fn set_field_with_undo(world: &mut World, entity: Entity, type_path: &str, path: &str, value: &str) -> Result<(), String> {
    let before = read_field(world, entity, type_path, path).ok_or("Field not found")?;
    let registry = world.resource::<AppTypeRegistry>().clone();
    let full = {
        let registry = registry.read();
        let type_id = resolve_on_entity(world, &registry, entity, type_path).ok_or_else(|| format!("Unknown component '{type_path}'"))?;
        registry.get(type_id).map(|r| r.type_info().type_path().to_string()).unwrap_or_else(|| type_path.to_string())
    };
    if before == value.trim() {
        return Ok(());
    }
    execute_command(world, SetReflectField { entity, type_path: full, path: path.to_string(), before, after: value.to_string() })
}

/// JSON view of an entity's reflected components for the remote API.
pub fn describe_entity_json(world: &World, entity: Entity) -> serde_json::Value {
    let components: Vec<_> = describe_entity(world, entity)
        .into_iter()
        .map(|component| {
            let fields: Vec<_> = component
                .fields
                .iter()
                .filter(|f| f.kind != FieldKind::Group)
                .map(|f| {
                    serde_json::json!({
                        "path": f.path,
                        "label": f.label,
                        "kind": match &f.kind {
                            FieldKind::Float => "float", FieldKind::Int => "int", FieldKind::Bool => "bool",
                            FieldKind::Text => "text", FieldKind::Color => "color", FieldKind::Enum(_) => "enum",
                            FieldKind::Group => "group", FieldKind::Json => "json", FieldKind::Opaque => "opaque",
                        },
                        "variants": if let FieldKind::Enum(v) = &f.kind { Some(v.clone()) } else { None },
                        "value": read_field(world, entity, &component.type_path, &f.path),
                        "unit": f.unit,
                        "tooltip": f.tooltip,
                        "range": f.range.map(|(a, b)| [a, b]),
                        "step": f.step,
                        "read_only": f.read_only,
                    })
                })
                .collect();
            serde_json::json!({
                "type_path": component.type_path,
                "name": component.name,
                "tooltip": component.tooltip,
                "fields": fields,
            })
        })
        .collect();
    serde_json::json!({ "entity": entity.to_bits(), "components": components })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    struct Mover {
        #[reflect(@ValueRange(0.0..=20.0), @Step(0.5), @Unit("m/s"))]
        speed: f32,
        #[reflect(@DisplayName("Jump Height"))]
        jump: f32,
        #[reflect(@ReadOnly)]
        grounded: bool,
        mode: MoveMode,
    }

    #[derive(Reflect, Default, Clone, Copy, PartialEq, Debug)]
    enum MoveMode {
        #[default]
        Walk,
        Run,
    }

    fn world() -> (World, Entity) {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        world.resource::<AppTypeRegistry>().write().register::<Mover>();
        let e = world.spawn(Mover::default()).id();
        (world, e)
    }

    #[test]
    fn annotations_drive_metadata_and_clamping() {
        let (mut world, e) = world();
        let path = std::any::type_name::<Mover>();
        let speed = field_meta(&world, e, "Mover", "speed").unwrap();
        assert_eq!(speed.unit, Some("m/s"));
        assert_eq!(speed.range, Some((0.0, 20.0)));
        assert_eq!(field_meta(&world, e, path, "jump").unwrap().label, "Jump Height");
        write_field(&mut world, e, "Mover", "speed", "99").unwrap();
        assert_eq!(world.get::<Mover>(e).unwrap().speed, 20.0);
        assert!(write_field(&mut world, e, "Mover", "grounded", "true").is_err());
        assert_eq!(next_toggle_value(&world, e, "Mover", "mode").as_deref(), Some("Run"));
        write_field(&mut world, e, "Mover", "mode", "Run").unwrap();
        assert_eq!(world.get::<Mover>(e).unwrap().mode, MoveMode::Run);
        let (before, after) = nudge_field(&mut world, e, "Mover", "speed", -1.0).unwrap();
        assert_eq!((before.as_str(), after.as_str()), ("20", "19.5"));
    }
}

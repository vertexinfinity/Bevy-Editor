//! Camera "Rendering" card: one place for the post-process stack, AA and tonemapping.
//!
//! In Bevy these are all components on the camera entity, so the card only toggles them
//! (with undo). Their parameters then show up in the generic Components card, driven by
//! reflection. "Preview in viewport" mirrors the selected camera's rendering setup onto the
//! editor viewport camera, since scene cameras do not render while editing.
use bevy::{
    anti_alias::{fxaa::Fxaa, smaa::Smaa, taa::TemporalAntiAliasing},
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    ecs::reflect::ReflectComponent,
    inspector::InspectorSelection,
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
    camera::Hdr,
    render::camera::{MipBias, TemporalJitter},
};

use super::property_editor::{PropertyPath, spawn_property_value_for};
use crate::camera::EditorCamera;
use crate::core::play_mode::PlayState;
use crate::core::reflect_fields::{self, ToggleReflectComponent};
use crate::scene::EditorInternal;
use crate::ui::theme::*;

/// Toggleable rendering features: (label, component type, tooltip, needs MSAA off).
pub const CAMERA_FEATURES: &[(&str, &str, &str, bool)] = &[
    ("HDR", "Hdr", "High dynamic range rendering; required by bloom and auto exposure", false),
    ("Bloom", "Bloom", "Glow around bright pixels", false),
    ("Auto Exposure", "AutoExposure", "Adapts exposure to scene brightness over time", false),
    ("Ambient Occlusion (SSAO)", "ScreenSpaceAmbientOcclusion", "Contact shadows in creases; turns MSAA off", true),
    ("Screen-Space Reflections", "ScreenSpaceReflections", "Reflections of on-screen geometry; needs deferred rendering, turns MSAA off", true),
    ("Depth of Field", "DepthOfField", "Blurs what is outside the focal distance", false),
    ("Motion Blur", "MotionBlur", "Blur from camera and object motion; turns MSAA off", true),
    ("Chromatic Aberration", "ChromaticAberration", "Color fringing towards the screen edges", false),
    ("Vignette", "Vignette", "Darkens the screen edges", false),
    ("Sharpening (CAS)", "ContrastAdaptiveSharpening", "Contrast-adaptive sharpening, pairs well with TAA/FXAA", false),
];

/// Anti-aliasing choices shown as one cycling control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AntiAliasing {
    Off,
    Msaa4,
    Fxaa,
    Smaa,
    Taa,
}

impl AntiAliasing {
    pub const ALL: [Self; 5] = [Self::Off, Self::Msaa4, Self::Fxaa, Self::Smaa, Self::Taa];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Msaa4 => "MSAA 4x",
            Self::Fxaa => "FXAA",
            Self::Smaa => "SMAA",
            Self::Taa => "TAA",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.to_ascii_lowercase().replace([' ', '_'], "");
        Self::ALL.into_iter().find(|m| m.label().to_ascii_lowercase().replace(' ', "") == s || (s == "msaa" && *m == Self::Msaa4))
    }
}

/// Selected-camera rendering preview on the editor viewport camera.
#[derive(Resource, Default)]
pub struct ViewportPreview {
    pub source: Option<Entity>,
}

#[derive(Component)]
pub struct CameraCardRoot;

#[derive(Component)]
struct FeatureChip(&'static str);

#[derive(Component)]
struct AaChip;

#[derive(Component)]
struct PreviewChip;

#[derive(Component)]
struct TonemappingRow;

pub fn spawn_camera_card(commands: &mut Commands, body: Entity) {
    commands.entity(body).insert(CameraCardRoot);
    preview_row(commands, body);
    let aa = toggle_row(commands, body, "Anti-aliasing", "Off / MSAA 4x / FXAA / SMAA / TAA");
    commands.entity(aa).insert(AaChip).observe(|click: On<PointerClick>, mut commands: Commands| {
        if click.button == PointerButton::Primary {
            commands.queue(|world: &mut World| with_camera(world, |world, camera| {
                let next = AntiAliasing::ALL[(AntiAliasing::ALL.iter().position(|m| *m == anti_aliasing(world, camera)).unwrap_or(0) + 1) % AntiAliasing::ALL.len()];
                set_anti_aliasing(world, camera, next);
                Ok(())
            }));
        }
    });
    let tonemap = commands
        .spawn((EditorInternal, TonemappingRow, Node { align_items: AlignItems::Center, min_height: px(28), column_gap: px(8), ..default() }, ChildOf(body)))
        .id();
    label(commands, tonemap, "Tonemapping");
    // Bound to the selected camera; the value field cycles unit enums on click.
    spawn_property_value_for(commands, tonemap, PropertyPath::Reflect { type_path: "Tonemapping".into(), field: String::new() }, None);
    for (name, type_path, tooltip, _) in CAMERA_FEATURES {
        let chip = toggle_row(commands, body, name, tooltip);
        commands.entity(chip).insert(FeatureChip(type_path)).observe(move |click: On<PointerClick>, mut commands: Commands| {
            if click.button == PointerButton::Primary {
                commands.queue(move |world: &mut World| with_camera(world, |world, camera| toggle_feature(world, camera, type_path)));
            }
        });
    }
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Text::new("Feature parameters appear in the Components card below."),
        TextFont { font_size: bevy::text::FontSize::Px(FONT_SIZE_SMALL), ..default() },
        TextColor(TEXT_MUTED),
        ChildOf(body),
    ));
}

fn label(commands: &mut Commands, row: Entity, text: &str) {
    commands.spawn((
        EditorInternal,
        Pickable::IGNORE,
        Node { width: px(150), flex_shrink: 0.0, ..default() },
        Text::new(text),
        TextFont { font_size: bevy::text::FontSize::Px(12.5), ..default() },
        TextColor(TEXT_SECONDARY),
        ChildOf(row),
    ));
}

/// A label plus a clickable chip; returns the chip.
pub(crate) fn toggle_row(commands: &mut Commands, parent: Entity, name: &str, _tooltip: &str) -> Entity {
    let row = commands
        .spawn((EditorInternal, Node { align_items: AlignItems::Center, min_height: px(26), column_gap: px(8), ..default() }, ChildOf(parent)))
        .id();
    label(commands, row, name);
    let chip = commands
        .spawn((
            EditorInternal,
            Node { padding: UiRect::axes(px(10), px(3)), border_radius: radius_pill(), min_width: px(64), justify_content: JustifyContent::Center, ..default() },
            BackgroundColor(BG_INPUT),
            ChildOf(row),
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
        Text::new("Off"),
        TextFont { font_size: bevy::text::FontSize::Px(12.0), weight: FontWeight::SEMIBOLD, ..default() },
        TextColor(TEXT_MUTED),
        ChildOf(chip),
    ));
    chip
}

fn preview_row(commands: &mut Commands, body: Entity) {
    let chip = toggle_row(commands, body, "Preview in viewport", "Mirror this camera's rendering setup onto the editor viewport");
    commands.entity(chip).insert(PreviewChip).observe(|click: On<PointerClick>, mut commands: Commands| {
        if click.button == PointerButton::Primary {
            commands.queue(|world: &mut World| with_camera(world, |world, camera| {
                let mut preview = world.resource_mut::<ViewportPreview>();
                preview.source = if preview.source == Some(camera) { None } else { Some(camera) };
                Ok(())
            }));
        }
    });
}

fn with_camera(world: &mut World, f: impl FnOnce(&mut World, Entity) -> Result<(), String>) {
    if *world.resource::<PlayState>() != PlayState::Edit {
        return;
    }
    let Some(camera) = world.resource::<InspectorSelection>().0.filter(|e| world.get::<Camera3d>(*e).is_some()) else { return };
    if let Err(error) = f(world, camera) {
        warn!("{error}");
    }
}

pub fn has_component(world: &World, entity: Entity, type_path: &str) -> bool {
    let registry = world.resource::<AppTypeRegistry>().read();
    let Some(type_id) = reflect_fields::find_component_type(&registry, type_path) else { return false };
    let (Some(reflect), Ok(entity_ref)) = (registry.get_type_data::<ReflectComponent>(type_id), world.get_entity(entity)) else { return false };
    reflect.contains(entity_ref)
}

/// Toggles a rendering feature with undo, applying the MSAA constraint some features need.
pub fn toggle_feature(world: &mut World, camera: Entity, type_path: &str) -> Result<(), String> {
    let needs_msaa_off = CAMERA_FEATURES.iter().find(|f| f.1 == type_path).is_some_and(|f| f.3);
    let enabled = has_component(world, camera, type_path);
    set_feature(world, camera, type_path, !enabled, needs_msaa_off)
}

/// Features whose Bevy shaders fail to compile on the active GPU backend.
pub fn feature_blocked(world: &World, type_path: &str) -> Option<&'static str> {
    let info = world.get_resource::<bevy::render::renderer::RenderAdapterInfo>()?;
    let dx12 = bevy::render::settings::Backends::from(info.backend) == bevy::render::settings::Backends::DX12;
    match (type_path, dx12) {
        ("MotionBlur", true) => {
            Some("Motion blur's shader does not compile with DirectX 12 (FXC); launch with WGPU_BACKEND=vulkan to use it")
        }
        _ => None,
    }
}

pub fn set_feature(world: &mut World, camera: Entity, type_path: &str, on: bool, needs_msaa_off: bool) -> Result<(), String> {
    if has_component(world, camera, type_path) == on {
        return Ok(());
    }
    if on {
        if let Some(reason) = feature_blocked(world, type_path) {
            return Err(reason.to_string());
        }
    }
    if on && needs_msaa_off && world.get::<Msaa>(camera).is_some_and(|m| *m != Msaa::Off) {
        world.entity_mut(camera).insert(Msaa::Off);
    }
    if on && type_path == "ScreenSpaceReflections" {
        world.entity_mut(camera).insert((DepthPrepass, DeferredPrepass));
    }
    let command = if on { ToggleReflectComponent::insert(camera, type_path) } else { ToggleReflectComponent::remove(camera, type_path) };
    reflect_fields::execute_command(world, command)
}

pub fn anti_aliasing(world: &World, camera: Entity) -> AntiAliasing {
    if world.get::<TemporalAntiAliasing>(camera).is_some() {
        AntiAliasing::Taa
    } else if world.get::<Smaa>(camera).is_some() {
        AntiAliasing::Smaa
    } else if world.get::<Fxaa>(camera).is_some_and(|f| f.enabled) {
        AntiAliasing::Fxaa
    } else if world.get::<Msaa>(camera).is_some_and(|m| *m != Msaa::Off) {
        AntiAliasing::Msaa4
    } else {
        AntiAliasing::Off
    }
}

pub fn set_anti_aliasing(world: &mut World, camera: Entity, mode: AntiAliasing) {
    let mut e = world.entity_mut(camera);
    e.remove::<(Fxaa, Smaa, TemporalAntiAliasing, TemporalJitter, MipBias)>();
    match mode {
        AntiAliasing::Off => { e.insert(Msaa::Off); }
        AntiAliasing::Msaa4 => { e.insert(Msaa::Sample4); }
        AntiAliasing::Fxaa => { e.insert((Msaa::Off, Fxaa::default())); }
        AntiAliasing::Smaa => { e.insert((Msaa::Off, Smaa::default())); }
        AntiAliasing::Taa => { e.insert((Msaa::Off, TemporalAntiAliasing::default())); }
    }
    // Features that need MSAA off stay compatible; switching to MSAA drops them.
    if mode == AntiAliasing::Msaa4 {
        for (_, type_path, _, needs_off) in CAMERA_FEATURES {
            if *needs_off && has_component(world, camera, type_path) {
                let _ = reflect_fields::remove_component(world, camera, type_path);
            }
        }
    }
}

/// Shows the card for scene cameras and refreshes chip labels.
#[allow(clippy::type_complexity)]
pub fn update_camera_card(world: &mut World) {
    let selected = world.resource::<InspectorSelection>().0;
    let camera = selected.filter(|e| world.get::<Camera3d>(*e).is_some() && world.get::<EditorInternal>(*e).is_none());
    let Some(card) = world.query_filtered::<&ChildOf, With<CameraCardRoot>>().iter(world).next().map(|c| c.parent()) else { return };
    if let Some(mut node) = world.get_mut::<Node>(card) {
        let display = if camera.is_some() { Display::Flex } else { Display::None };
        if node.display != display {
            crate::ui::set_display(&mut node, display);
        }
    }
    let Some(camera) = camera else { return };
    let chips: Vec<(Entity, String, bool)> = {
        let mut out = Vec::new();
        let features: Vec<(Entity, &'static str)> = world.query::<(Entity, &FeatureChip)>().iter(world).map(|(e, f)| (e, f.0)).collect();
        for (chip, type_path) in features {
            let on = has_component(world, camera, type_path);
            let text = if feature_blocked(world, type_path).is_some() { "N/A" } else if on { "On" } else { "Off" };
            out.push((chip, text.into(), on));
        }
        let aa = anti_aliasing(world, camera);
        for chip in world.query_filtered::<Entity, With<AaChip>>().iter(world).collect::<Vec<_>>() {
            out.push((chip, aa.label().into(), aa != AntiAliasing::Off));
        }
        let previewing = world.resource::<ViewportPreview>().source == Some(camera);
        for chip in world.query_filtered::<Entity, With<PreviewChip>>().iter(world).collect::<Vec<_>>() {
            out.push((chip, if previewing { "On".into() } else { "Off".into() }, previewing));
        }
        out
    };
    for (chip, text, on) in chips {
        let Some(child) = world.get::<Children>(chip).and_then(|c| c.first().copied()) else { continue };
        if let Some(mut t) = world.get_mut::<Text>(child) {
            if t.0 != text {
                t.0 = text;
            }
        }
        if let Some(mut color) = world.get_mut::<TextColor>(child) {
            let next = TextColor(if on { ACCENT_CYAN } else { TEXT_MUTED });
            if *color != next {
                *color = next;
            }
        }
    }
}

/// Components mirrored from a previewed camera onto the editor viewport camera.
const MIRRORED: &[&str] = &[
    "Hdr", "Bloom", "AutoExposure", "ScreenSpaceAmbientOcclusion", "ScreenSpaceReflections", "DepthOfField",
    "MotionBlur", "ChromaticAberration", "Vignette", "ContrastAdaptiveSharpening", "Tonemapping", "Exposure",
    "ColorGrading", "DebandDither", "Msaa", "Fxaa", "TemporalAntiAliasing", "DepthPrepass", "NormalPrepass",
    "MotionVectorPrepass", "DeferredPrepass", "TemporalJitter", "MipBias",
];

/// Copies rendering components from the previewed camera to the editor camera when they change.
pub fn sync_viewport_preview(world: &mut World, mut last: Local<Option<(Option<Entity>, bevy::ecs::change_detection::Tick)>>) {
    let source = world.resource::<ViewportPreview>().source.filter(|e| world.get::<Camera3d>(*e).is_some());
    let Some(editor) = world.query_filtered::<Entity, With<EditorCamera>>().iter(world).next() else { return };
    let now = world.read_change_tick();
    let (previous, since) = last.unwrap_or((None, now));
    // Removing a feature bumps no component tick, so also compare which features are present.
    let changed = previous != source
        || source.is_some_and(|s| {
            reflect_fields::reflected_components(world, s).iter().any(|(id, _, _)| {
                world.entity(s).get_change_ticks_by_id(*id).is_some_and(|t| t.is_changed(since, now))
            }) || MIRRORED.iter().any(|t| has_component(world, s, t) != has_component(world, editor, t))
                || world.get::<Smaa>(s).is_some() != world.get::<Smaa>(editor).is_some()
        });
    *last = Some((source, now));
    if !changed {
        return;
    }
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    for type_path in MIRRORED {
        if feature_blocked(world, type_path).is_some() {
            continue;
        }
        let Some(type_id) = reflect_fields::find_component_type(&registry, type_path) else { continue };
        let Some(reflect) = registry.get_type_data::<ReflectComponent>(type_id) else { continue };
        let value = source.and_then(|s| reflect.reflect(world.entity(s)).and_then(|v| v.reflect_clone().ok()));
        let mut editor_mut = world.entity_mut(editor);
        match value {
            Some(value) => reflect.insert(&mut editor_mut, value.as_partial_reflect(), &registry),
            None => reflect.remove(&mut editor_mut),
        }
    }
    let smaa = source.and_then(|s| world.get::<Smaa>(s).cloned());
    let mut editor_mut = world.entity_mut(editor);
    match smaa {
        Some(smaa) => { editor_mut.insert(smaa); }
        None => { editor_mut.remove::<Smaa>(); }
    }
    if source.is_none() {
        // Restore the plain viewport: re-inserting Camera3d re-adds its required defaults.
        editor_mut.remove::<(Hdr, DepthPrepass, NormalPrepass, MotionVectorPrepass, DeferredPrepass, TemporalJitter, MipBias)>();
        editor_mut.insert((Camera3d::default(), Msaa::default()));
    }
}

/// Overlay cameras on the editor window (e.g. the transform gizmo) must match the viewport's
/// HDR and MSAA settings, otherwise they composite from a separate texture and wipe the frame.
/// Preview, atmosphere and fog can all change those settings, so this runs every frame.
#[allow(clippy::type_complexity)]
pub fn sync_overlay_cameras(
    mut commands: Commands,
    editor: Query<(Has<Hdr>, &Msaa), With<EditorCamera>>,
    mut overlays: Query<(Entity, &mut Camera, Has<Hdr>, Option<&Msaa>), (Without<EditorCamera>, Without<crate::camera::EditorUiCamera>, Without<crate::scene::SceneEntity>, Without<crate::core::play_mode::PreviewEntity>)>,
) {
    let Ok((hdr, msaa)) = editor.single() else { return };
    for (entity, mut camera, overlay_hdr, overlay_msaa) in &mut overlays {
        // Without MSAA writeback a clearing overlay would erase the viewport underneath it.
        if !matches!(camera.clear_color, ClearColorConfig::None) {
            camera.clear_color = ClearColorConfig::None;
        }
        if hdr && !overlay_hdr {
            commands.entity(entity).insert(Hdr);
        } else if !hdr && overlay_hdr {
            commands.entity(entity).remove::<Hdr>();
        }
        if overlay_msaa != Some(msaa) {
            commands.entity(entity).insert(*msaa);
        }
    }
}

/// Play-mode copies of scene cameras render for real, so drop features the backend can't compile.
pub fn strip_blocked_runtime_features(
    mut commands: Commands,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    cameras: Query<Entity, (With<bevy::post_process::motion_blur::MotionBlur>, With<crate::core::play_mode::PreviewEntity>)>,
) {
    if adapter.is_none_or(|a| bevy::render::settings::Backends::from(a.backend) != bevy::render::settings::Backends::DX12) {
        return;
    }
    for camera in &cameras {
        warn!("Motion blur is not supported on DirectX 12; disabled for the Game View camera");
        commands.entity(camera).remove::<bevy::post_process::motion_blur::MotionBlur>();
    }
}
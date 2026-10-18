//! Game View runs a scene copy. Stop only deletes preview-owned entities.
use super::play_input::GameInput;
use crate::{
    camera::EditorCamera,
    scene::{EditorInternal, SceneEntity},
};
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::entity::EntityHashMap,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowRef},
    world_serialization::{DynamicWorldBuilder, DynamicWorldRoot, WorldAssetRoot},
};
use std::collections::HashMap;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Edit,
    Playing,
    Paused,
}
#[derive(Component)]
pub struct PreviewWindow;
#[derive(Component)]
pub struct PreviewCamera;
/// Gameplay queries must target runtime copies, never the authoring scene.
#[derive(Component)]
pub struct PreviewEntity;
#[derive(Component)]
struct PreviewHud;
#[derive(Resource, Default)]
pub struct PlaySession {
    pub window: Option<Entity>,
    pub camera: Option<Entity>,
    pub entity_map: EntityHashMap<Entity>,
    step_pending: bool,
}
#[derive(Component, Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayModeAction {
    Play,
    PlayInWindow,
    Pause,
    Stop,
    Step,
}
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaySystems {
    Lifecycle,
    Simulation,
}
pub fn in_edit_mode(state: Res<PlayState>) -> bool {
    *state == PlayState::Edit
}

/// Bound idle redraw latency so deferred layout/text changes appear without another input event.
pub fn editor_winit_settings() -> bevy::winit::WinitSettings {
    use bevy::winit::{UpdateMode, WinitSettings};
    use std::time::Duration;
    WinitSettings {
        focused_mode: UpdateMode::reactive(Duration::from_secs_f64(1.0 / 60.0)),
        unfocused_mode: UpdateMode::reactive_low_power(Duration::from_millis(250)),
    }
}

pub struct PlayModePlugin;
impl Plugin for PlayModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayState>()
            .init_resource::<PlaySession>()
            .add_message::<PlayModeAction>()
            .configure_sets(
                Update,
                (PlaySystems::Lifecycle, PlaySystems::Simulation).chain(),
            )
            .add_systems(
                Update,
                handle_play_mode_actions.in_set(PlaySystems::Lifecycle),
            )
            .add_systems(PostUpdate, (finish_single_step, update_preview_hud).chain());
        super::play_input::install(app);
    }
}

fn start_play_session(world: &mut World) -> Result<(), String> {
    let sources: Vec<Entity> = world
        .query_filtered::<Entity, (
            With<SceneEntity>,
            Without<Window>,
            Without<PrimaryWindow>,
            Without<EditorInternal>,
            Without<EditorCamera>,
            Without<PreviewEntity>,
        )>()
        .iter(world)
        .collect();
    let parents: Vec<_> = sources
        .iter()
        .filter_map(|entity| {
            world
                .get::<ChildOf>(*entity)
                .map(|parent| (*entity, parent.parent()))
        })
        .collect();
    let dynamic = {
        let registry = world.resource::<AppTypeRegistry>().read();
        DynamicWorldBuilder::from_world(world, &registry)
            .deny_component::<SceneEntity>()
            .deny_component::<EditorInternal>()
            .deny_component::<ChildOf>()
            .deny_component::<Children>()
            .deny_component::<DynamicWorldRoot>()
            .deny_component::<WorldAssetRoot>()
            .deny_component::<bevy::gizmos::transform_gizmo::TransformGizmoFocus>()
            .deny_component::<IsDefaultUiCamera>()
            .deny_component::<bevy::camera::visibility::VisibilityClass>()
            .deny_component::<bevy::camera::visibility::ViewVisibility>()
            .extract_entities(sources.into_iter())
            .build()
    };
    let mut map = EntityHashMap::default();
    if let Err(error) = dynamic.write_to_world(world, &mut map) {
        for entity in map.values() {
            if world.get_entity(*entity).is_ok() {
                world.despawn(*entity);
            }
        }
        return Err(format!("Cannot create runtime scene: {error}"));
    }
    // Only recreate relationships within the scene copy; never attach it to a window or editor UI.
    for (source, parent) in parents {
        if let (Some(entity), Some(parent)) = (map.get(&source), map.get(&parent)) {
            world.entity_mut(*entity).insert(ChildOf(*parent));
        }
    }
    let mut material_copies: HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>> =
        HashMap::new();
    for entity in map.values() {
        world
            .entity_mut(*entity)
            .insert((PreviewEntity, RenderLayers::layer(31)));
        if let Some(mut camera) = world.get_mut::<Camera>(*entity) {
            camera.is_active = false;
        }
        if let Some(handle) = world
            .get::<MeshMaterial3d<StandardMaterial>>(*entity)
            .map(|material| material.0.clone())
        {
            let copy = if let Some(copy) = material_copies.get(&handle.id()) {
                Some(copy.clone())
            } else {
                let value = world
                    .get_resource::<Assets<StandardMaterial>>()
                    .and_then(|materials| materials.get(&handle).cloned());
                value.map(|value| {
                    let copy = world.resource_mut::<Assets<StandardMaterial>>().add(value);
                    material_copies.insert(handle.id(), copy.clone());
                    copy
                })
            };
            if let Some(copy) = copy {
                world.entity_mut(*entity).insert(MeshMaterial3d(copy));
            }
        }
    }
    let window = world
        .spawn((
            PreviewWindow,
            EditorInternal,
            Window {
                title:
                    "Bevylite — Game View | Space/E: gate · RMB: orbit · Esc: release · F8: Stop"
                        .into(),
                resolution: (960, 640).into(),
                focused: true,
                ..default()
            },
            CursorOptions::default(),
        ))
        .id();
    let mut cameras: Vec<_> = map
        .values()
        .copied()
        .filter(|entity| world.get::<Camera>(*entity).is_some())
        .collect();
    cameras.sort();
    let camera = if let Some(camera) = cameras.first() {
        *camera
    } else {
        let transform = world
            .query_filtered::<&Transform, With<EditorCamera>>()
            .iter(world)
            .next()
            .copied()
            .unwrap_or_else(|| Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y));
        world
            .spawn((
                PreviewEntity,
                Camera3d::default(),
                transform,
                RenderLayers::layer(31),
            ))
            .id()
    };
    // Serialization does not guarantee runtime camera pipeline components are reflected.
    // Reinstall the camera kind so Bevy supplies its required render graph and view state.
    let is_2d = map
        .iter()
        .any(|(source, runtime)| *runtime == camera && world.get::<Camera2d>(*source).is_some());
    if is_2d {
        world.entity_mut(camera).insert(Camera2d);
    } else {
        world.entity_mut(camera).insert(Camera3d::default());
    }
    world.entity_mut(camera).insert((
        PreviewCamera,
        Camera {
            is_active: true,
            ..default()
        },
        RenderTarget::Window(WindowRef::Entity(window)),
        bevy::picking::mesh_picking::MeshPickingCamera,
    ));
    for entity in map.values() {
        if world.get::<Node>(*entity).is_some() {
            world.entity_mut(*entity).insert(UiTargetCamera(camera));
        }
    }
    world.spawn((
        PreviewEntity,
        PreviewHud,
        EditorInternal,
        Pickable::IGNORE,
        UiTargetCamera(camera),
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            bottom: px(12),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.88)),
        Text::new("Game View · Space/E: gate · Click switch · RMB: orbit · Wheel: zoom · F8: Stop"),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::WHITE),
    ));
    *world.resource_mut::<PlaySession>() = PlaySession {
        window: Some(window),
        camera: Some(camera),
        entity_map: map,
        step_pending: false,
    };
    *world.resource_mut::<PlayState>() = PlayState::Playing;
    *world.resource_mut::<GameInput>() = default();
    if let Some(mut gizmo) =
        world.get_resource_mut::<bevy::gizmos::transform_gizmo::TransformGizmoState>()
    {
        *gizmo = default();
    }
    let focused: Vec<_> = world
        .query_filtered::<Entity, With<bevy::gizmos::transform_gizmo::TransformGizmoFocus>>()
        .iter(world)
        .collect();
    for entity in focused {
        world
            .entity_mut(entity)
            .remove::<bevy::gizmos::transform_gizmo::TransformGizmoFocus>();
    }
    if let Some(mut settings) = world.get_resource_mut::<bevy::winit::WinitSettings>() {
        settings.focused_mode = bevy::winit::UpdateMode::Continuous;
    }
    info!("Game View started; authoring scene preserved.");
    Ok(())
}

fn stop_play_session(world: &mut World) {
    let runtime: Vec<_> = world
        .query_filtered::<Entity, (
            With<PreviewEntity>,
            Without<Window>,
            Without<PrimaryWindow>,
            Without<EditorCamera>,
        )>()
        .iter(world)
        .collect();
    for entity in runtime {
        if world.get_entity(entity).is_ok() {
            world.despawn(entity);
        }
    }
    let window = world.resource::<PlaySession>().window;
    if let Some(window) = window {
        if world.get::<PreviewWindow>(window).is_some()
            && world.get::<PrimaryWindow>(window).is_none()
        {
            world.despawn(window);
        }
    }
    for mut cursor in world.query::<&mut CursorOptions>().iter_mut(world) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
    *world.resource_mut::<PlaySession>() = default();
    *world.resource_mut::<GameInput>() = default();
    *world.resource_mut::<PlayState>() = PlayState::Edit;
    if let Some(mut settings) = world.get_resource_mut::<bevy::winit::WinitSettings>() {
        *settings = editor_winit_settings();
    }
    info!("Game View stopped; authoring scene unchanged.");
}

pub fn handle_play_mode_actions(world: &mut World) {
    let actions: Vec<_> = world
        .resource_mut::<Messages<PlayModeAction>>()
        .drain()
        .collect();
    if *world.resource::<PlayState>() != PlayState::Edit
        && world
            .resource::<PlaySession>()
            .window
            .is_some_and(|window| world.get::<Window>(window).is_none())
    {
        stop_play_session(world);
    }
    for action in actions {
        match action {
            PlayModeAction::Play => match *world.resource::<PlayState>() {
                PlayState::Edit => {
                    if let Err(error) = start_play_session(world) {
                        error!("{error}");
                    }
                }
                PlayState::Paused => *world.resource_mut::<PlayState>() = PlayState::Playing,
                PlayState::Playing => {}
            },
            PlayModeAction::Pause => match *world.resource::<PlayState>() {
                PlayState::Playing => *world.resource_mut::<PlayState>() = PlayState::Paused,
                PlayState::Paused => *world.resource_mut::<PlayState>() = PlayState::Playing,
                _ => {}
            },
            PlayModeAction::Stop => {
                if *world.resource::<PlayState>() != PlayState::Edit {
                    stop_play_session(world);
                }
            }
            PlayModeAction::Step => {
                if *world.resource::<PlayState>() == PlayState::Edit {
                    if let Err(error) = start_play_session(world) {
                        error!("{error}");
                        continue;
                    }
                }
                *world.resource_mut::<PlayState>() = PlayState::Playing;
                world.resource_mut::<PlaySession>().step_pending = true;
            }
            PlayModeAction::PlayInWindow => super::standalone::launch(world),
        }
    }
}
fn finish_single_step(mut state: ResMut<PlayState>, mut session: ResMut<PlaySession>) {
    if session.step_pending {
        session.step_pending = false;
        *state = PlayState::Paused;
    }
}
fn update_preview_hud(
    state: Res<PlayState>,
    input: Res<GameInput>,
    mut labels: Query<&mut Text, With<PreviewHud>>,
) {
    let label = if *state == PlayState::Paused {
        "PAUSED · Resume or Step in the editor · Close Game View to Stop"
    } else if !input.active {
        "Click Game View for game input · Space/E: gate · Esc: release · F8: Stop"
    } else {
        "PLAYING · Space/E: gate · Click switch · RMB: orbit · Wheel: zoom · Esc: release · F8: Stop"
    };
    for mut text in &mut labels {
        if text.0 != label {
            text.0 = label.into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::components::ScriptComponent;

    fn fixture() -> (App, Entity, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<PlayState>()
            .init_resource::<PlaySession>()
            .init_resource::<GameInput>()
            .init_resource::<Assets<StandardMaterial>>()
            .register_type::<Name>()
            .register_type::<Transform>()
            .register_type::<ScriptComponent>()
            .register_type::<MeshMaterial3d<StandardMaterial>>();
        let primary = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow, EditorInternal))
            .id();
        let root = app
            .world_mut()
            .spawn((SceneEntity, Name::new("Root"), Transform::default()))
            .id();
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::default());
        let child = app
            .world_mut()
            .spawn((
                SceneEntity,
                Name::new("Child"),
                Transform::from_xyz(1.0, 2.0, 3.0),
                ChildOf(root),
                MeshMaterial3d(material),
            ))
            .id();
        app.world_mut().entity_mut(root).insert(ScriptComponent {
            entity_refs: HashMap::from([("child".into(), child)]),
            ..default()
        });
        (app, primary, root, child)
    }

    #[test]
    fn repeated_play_stop_preserves_editor_and_authoring_scene() {
        let (mut app, primary, root, child) = fixture();
        let world = app.world_mut();
        let original = *world.get::<Transform>(child).unwrap();
        for _ in 0..3 {
            start_play_session(world).unwrap();
            let map = world.resource::<PlaySession>().entity_map.clone();
            assert_eq!(
                world.get::<ChildOf>(map[&child]).unwrap().parent(),
                map[&root]
            );
            assert_eq!(
                world
                    .get::<ScriptComponent>(map[&root])
                    .unwrap()
                    .entity_refs["child"],
                map[&child]
            );
            let source_material = world
                .get::<MeshMaterial3d<StandardMaterial>>(child)
                .unwrap()
                .0
                .clone();
            let runtime_material = world
                .get::<MeshMaterial3d<StandardMaterial>>(map[&child])
                .unwrap()
                .0
                .clone();
            assert_ne!(source_material, runtime_material);
            world.get_mut::<Transform>(map[&child]).unwrap().translation = Vec3::splat(99.0);
            stop_play_session(world);
            assert!(world.get::<PrimaryWindow>(primary).is_some());
            assert_eq!(world.get::<Transform>(child).unwrap(), &original);
            assert_eq!(world.get::<ChildOf>(child).unwrap().parent(), root);
            assert_eq!(
                world
                    .query_filtered::<Entity, With<PreviewEntity>>()
                    .iter(world)
                    .count(),
                0
            );
            assert_eq!(
                world
                    .query_filtered::<Entity, With<PreviewWindow>>()
                    .iter(world)
                    .count(),
                0
            );
        }
    }

    #[test]
    fn reflected_camera_reinstalls_required_render_pipeline() {
        let (mut app, _, _, _) = fixture();
        app.register_type::<Camera>();
        app.world_mut()
            .register_required_components_with::<Camera3d, bevy::render::camera::CameraRenderGraph>(
                || {
                    bevy::render::camera::CameraRenderGraph::new(
                        bevy::core_pipeline::schedule::Core3d,
                    )
                },
            );
        let source = app
            .world_mut()
            .spawn((
                SceneEntity,
                Camera3d::default(),
                Transform::from_xyz(0.0, 5.0, 10.0),
            ))
            .id();
        start_play_session(app.world_mut()).unwrap();
        let runtime = app.world().resource::<PlaySession>().entity_map[&source];
        assert_eq!(app.world().resource::<PlaySession>().camera, Some(runtime));
        assert!(app.world().get::<Camera3d>(runtime).is_some());
        assert!(
            app.world()
                .get::<bevy::render::camera::CameraRenderGraph>(runtime)
                .is_some()
        );
        assert!(app.world().get::<RenderTarget>(runtime).is_some());
    }

    #[test]
    fn closing_game_view_and_single_step_do_not_close_editor() {
        let (mut app, primary, _, _) = fixture();
        app.add_message::<PlayModeAction>()
            .add_systems(PostUpdate, finish_single_step);
        let world = app.world_mut();
        world
            .resource_mut::<Messages<PlayModeAction>>()
            .write(PlayModeAction::Step);
        handle_play_mode_actions(world);
        assert_eq!(*world.resource::<PlayState>(), PlayState::Playing);
        app.world_mut().run_schedule(PostUpdate);
        assert_eq!(*app.world().resource::<PlayState>(), PlayState::Paused);
        let preview = app.world().resource::<PlaySession>().window.unwrap();
        app.world_mut().despawn(preview);
        handle_play_mode_actions(app.world_mut());
        assert_eq!(*app.world().resource::<PlayState>(), PlayState::Edit);
        assert!(app.world().get::<PrimaryWindow>(primary).is_some());
    }
}

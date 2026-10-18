//! Scene-wide World / Environment settings: background, ambient light, sky, fog.
//!
//! The settings live as a reflected component on one hidden `World Settings` entity so the
//! generic reflected-field UI and remote API can edit them like any other component.
//! They are applied to the `ClearColor` / `GlobalAmbientLight` resources and to every 3D
//! camera (editor viewport, scene cameras and the Play-mode game camera), and persisted in
//! the scene's `.bevylite.json` sidecar under the `"world"` key.

use std::path::{Path, PathBuf};

use bevy::{
    annotations::{DisplayName, Step, Tooltip, Unit, ValueRange},
    light::{
        atmosphere::ScatteringMedium, Atmosphere, AtmosphereEnvironmentMapLight, GlobalAmbientLight,
        VolumetricFog,
    },
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
};
use serde::{Deserialize, Serialize};

use super::{CurrentScene, EditorInternal};

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkyMode {
    /// Flat background color.
    #[default]
    SolidColor,
    /// Bevy's physically based procedural atmosphere (sky driven by the sun's direction).
    Atmosphere,
}

#[derive(Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FogMode {
    #[default]
    Off,
    Linear,
    Exponential,
    ExponentialSquared,
    Atmospheric,
}

/// Scene environment, edited from the Inspector's World card (`editor/world` over RPC).
#[derive(Component, Reflect, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[reflect(Component, Default, @DisplayName("World Environment"), @Tooltip("Scene-wide sky, ambient light and fog"))]
#[serde(default)]
pub struct WorldEnvironment {
    #[reflect(@DisplayName("Background"), @Tooltip("Clear color behind everything when the sky is a solid color"))]
    pub clear_color: Color,
    #[reflect(@DisplayName("Sky"), @Tooltip("Solid color, or the physical atmosphere lit by the first directional light"))]
    pub sky: SkyMode,
    #[reflect(@DisplayName("Sky Reflections"), @Tooltip("Let the atmosphere drive reflections and ambient light on surfaces"))]
    pub sky_reflections: bool,
    #[reflect(@DisplayName("Ambient Color"))]
    pub ambient_color: Color,
    #[reflect(@DisplayName("Ambient Brightness"), @ValueRange(0.0..=20000.0), @Step(25.0), @Unit("cd/m²"), @Tooltip("Uniform light reaching every surface; Bevy's default is 80"))]
    pub ambient_brightness: f32,
    #[reflect(@DisplayName("Fog"), @Tooltip("Distance fog falloff applied to every 3D camera"))]
    pub fog: FogMode,
    #[reflect(@DisplayName("Fog Color"))]
    pub fog_color: Color,
    #[reflect(@DisplayName("Fog Start"), @ValueRange(0.0..=10000.0), @Step(1.0), @Unit("m"))]
    pub fog_start: f32,
    #[reflect(@DisplayName("Fog End"), @ValueRange(0.0..=10000.0), @Step(5.0), @Unit("m"))]
    pub fog_end: f32,
    #[reflect(@DisplayName("Fog Density"), @ValueRange(0.0..=1.0), @Step(0.005), @Tooltip("Used by exponential fog modes"))]
    pub fog_density: f32,
    #[reflect(@DisplayName("Volumetric Fog"), @Tooltip("Ray-marched light shafts; lights need a VolumetricLight component to scatter"))]
    pub volumetric_fog: bool,
}

impl Default for WorldEnvironment {
    fn default() -> Self {
        Self {
            clear_color: Color::srgb(0.11, 0.12, 0.15),
            sky: SkyMode::SolidColor,
            sky_reflections: true,
            ambient_color: Color::WHITE,
            ambient_brightness: 80.0,
            fog: FogMode::Off,
            fog_color: Color::srgb(0.55, 0.6, 0.7),
            fog_start: 20.0,
            fog_end: 120.0,
            fog_density: 0.02,
            volumetric_fog: false,
        }
    }
}

/// The hidden entity holding [`WorldEnvironment`].
#[derive(Component)]
pub struct WorldSettingsEntity;

/// The planet entity spawned while the sky is in Atmosphere mode.
#[derive(Component)]
struct EnvironmentAtmosphere;

#[derive(Resource, Default)]
struct EarthMedium(Option<Handle<ScatteringMedium>>);

/// Scene file the current world settings were last loaded from or saved to.
#[derive(Resource, Default)]
struct LoadedEnvironmentFor(Option<PathBuf>);

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WorldEnvironment>()
            .init_resource::<EarthMedium>()
            .init_resource::<LoadedEnvironmentFor>()
            .add_systems(Startup, spawn_world_settings)
            .add_systems(Update, (load_environment_on_scene_change, apply_environment).chain());
    }
}

fn spawn_world_settings(mut commands: Commands) {
    commands.spawn((EditorInternal, WorldSettingsEntity, Name::new("World Settings"), WorldEnvironment::default()));
}

pub fn world_settings_entity(world: &mut World) -> Option<Entity> {
    world.query_filtered::<Entity, With<WorldSettingsEntity>>().iter(world).next()
}

pub fn sidecar_path(scene_path: &Path) -> PathBuf {
    let mut s = scene_path.as_os_str().to_os_string();
    s.push(".bevylite.json");
    PathBuf::from(s)
}

/// Writes the `"world"` key into the scene's sidecar, keeping any other keys already there.
pub fn save_environment(world: &World, scene_path: &Path) -> Result<(), String> {
    let Some(mut state) = bevy::ecs::query::QueryState::<&WorldEnvironment, With<WorldSettingsEntity>>::try_new(world) else { return Ok(()) };
    let Some(settings) = state.iter(world).next().cloned() else { return Ok(()) };
    let path = sidecar_path(scene_path);
    let mut doc = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    doc["world"] = serde_json::to_value(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn read_environment(scene_path: &Path) -> Option<WorldEnvironment> {
    let raw = std::fs::read_to_string(sidecar_path(scene_path)).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&raw).ok()?;
    serde_json::from_value(doc.get("world")?.clone()).ok()
}

/// When the current scene path changes (open, new, save as), load that scene's environment.
fn load_environment_on_scene_change(
    scene: Option<Res<CurrentScene>>,
    mut loaded: ResMut<LoadedEnvironmentFor>,
    mut settings: Query<&mut WorldEnvironment, With<WorldSettingsEntity>>,
) {
    let Some(scene) = scene else { return };
    if !scene.is_changed() || loaded.0 == scene.file_path {
        return;
    }
    loaded.0 = scene.file_path.clone();
    let next = scene.file_path.as_deref().and_then(read_environment);
    if let (Some(next), Ok(mut current)) = (next, settings.single_mut()) {
        if *current != next {
            *current = next;
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn apply_environment(
    mut commands: Commands,
    settings: Query<Ref<WorldEnvironment>, With<WorldSettingsEntity>>,
    new_cameras: Query<(), Added<Camera3d>>,
    cameras: Query<Entity, With<Camera3d>>,
    atmospheres: Query<Entity, With<EnvironmentAtmosphere>>,
    mut clear_color: ResMut<ClearColor>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut media: ResMut<Assets<ScatteringMedium>>,
    mut earth: ResMut<EarthMedium>,
) {
    let Ok(env) = settings.single() else { return };
    if !env.is_changed() && new_cameras.is_empty() {
        return;
    }
    if clear_color.0 != env.clear_color {
        clear_color.0 = env.clear_color;
    }
    if ambient.color != env.ambient_color || ambient.brightness != env.ambient_brightness {
        ambient.color = env.ambient_color;
        ambient.brightness = env.ambient_brightness;
    }

    let atmosphere_on = env.sky == SkyMode::Atmosphere;
    if atmosphere_on && atmospheres.is_empty() {
        let medium = earth.0.get_or_insert_with(|| media.add(ScatteringMedium::earth(256, 256))).clone();
        commands.spawn((EditorInternal, EnvironmentAtmosphere, Name::new("Environment Atmosphere"), Atmosphere::earth(medium)));
    } else if !atmosphere_on {
        for e in &atmospheres {
            commands.entity(e).despawn();
        }
    }

    let fog = (env.fog != FogMode::Off).then(|| DistanceFog {
        color: env.fog_color,
        falloff: match env.fog {
            FogMode::Linear => FogFalloff::Linear { start: env.fog_start, end: env.fog_end.max(env.fog_start + 0.01) },
            FogMode::Exponential => FogFalloff::Exponential { density: env.fog_density },
            FogMode::ExponentialSquared => FogFalloff::ExponentialSquared { density: env.fog_density },
            _ => FogFalloff::from_visibility_colors(env.fog_end.max(1.0), Color::srgb(0.35, 0.5, 0.66), Color::srgb(0.8, 0.844, 1.0)),
        },
        ..default()
    });
    for camera in &cameras {
        let mut entity = commands.entity(camera);
        match &fog {
            Some(fog) => entity.insert(fog.clone()),
            None => entity.remove::<DistanceFog>(),
        };
        // Atmosphere rendering is per camera (and pulls in HDR).
        if atmosphere_on {
            entity.insert(bevy::pbr::AtmosphereSettings::default());
        } else {
            entity.remove::<bevy::pbr::AtmosphereSettings>();
        }
        if atmosphere_on && env.sky_reflections {
            entity.insert(AtmosphereEnvironmentMapLight::default());
        } else {
            entity.remove::<AtmosphereEnvironmentMapLight>();
        }
        if env.volumetric_fog {
            entity.insert(VolumetricFog::default());
        } else {
            entity.remove::<VolumetricFog>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_roundtrip_preserves_other_keys() {
        let dir = std::env::temp_dir().join(format!("bevylite_env_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let scene = dir.join("test.scn.ron");
        std::fs::write(sidecar_path(&scene), r#"{"entities": []}"#).unwrap();
        let mut world = World::new();
        let env = WorldEnvironment { fog: FogMode::Linear, ambient_brightness: 250.0, ..default() };
        world.spawn((WorldSettingsEntity, env.clone()));
        save_environment(&world, &scene).unwrap();
        assert_eq!(read_environment(&scene), Some(env));
        let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(sidecar_path(&scene)).unwrap()).unwrap();
        assert!(doc.get("entities").is_some());
        let _ = std::fs::remove_dir_all(dir);
    }
}

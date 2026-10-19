//! Bundled sample games (`samples/` workspace) and their levels inside the editor.
//!
//! Each sample keeps its static level in `src/level.rs`, written against plain Bevy so the
//! editor can compile the very same file here and show the level for editing. Gameplay
//! stays in the sample crate and runs with `cargo run -p <crate>`.
use std::path::PathBuf;

use bevy::prelude::*;

use crate::scene::SceneEntity;

#[path = "../../samples/third_person/src/level.rs"]
pub mod third_person;
#[path = "../../samples/first_person/src/level.rs"]
pub mod first_person;
#[path = "../../samples/endless_runner/src/level.rs"]
pub mod endless_runner;
#[path = "../../samples/arena_shooter/src/level.rs"]
pub mod arena_shooter;

/// One entry in the hub's Samples section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleGame {
    /// Cargo package name and folder under `samples/`.
    pub crate_name: &'static str,
    pub title: &'static str,
    pub genre: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

pub const SAMPLE_GAMES: &[SampleGame] = &[
    SampleGame {
        crate_name: "third_person",
        title: "Coin Courtyard",
        genre: "Third Person",
        description: "Run and jump around a walled courtyard with an orbit camera; grab 12 coins before the timer ends.",
        icon: "🏃",
    },
    SampleGame {
        crate_name: "first_person",
        title: "Target Range",
        genre: "First Person",
        description: "Mouse-look shooting gallery with hitscan, ammo and reloads, moving targets and a 60 second round.",
        icon: "🎯",
    },
    SampleGame {
        crate_name: "endless_runner",
        title: "Neon Runner",
        genre: "Endless Runner",
        description: "Three-lane runner: switch lanes, jump barriers, collect coins while the speed keeps climbing.",
        icon: "⚡",
    },
    SampleGame {
        crate_name: "arena_shooter",
        title: "Arena Survivor",
        genre: "Top-Down Shooter",
        description: "Twin-stick arena: aim with the mouse, dash, and survive escalating waves of chasers.",
        icon: "🛸",
    },
];

impl SampleGame {
    /// Label stored in the recent-projects registry, used to recognise samples on reopen.
    pub fn registry_label(&self) -> String {
        format!("Sample · {}", self.genre)
    }

    pub fn find_by_label(label: &str) -> Option<&'static SampleGame> {
        SAMPLE_GAMES.iter().find(|s| s.registry_label() == label)
    }

    /// Folder of this sample's crate.
    pub fn dir(&self) -> PathBuf {
        samples_root().join(self.crate_name)
    }

    /// Spawns the sample's level as authorable scene entities and returns them.
    pub fn spawn_level(&self, commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Vec<Entity> {
        let all = match self.crate_name {
            "third_person" => third_person::spawn_level(commands, meshes, materials).all,
            "first_person" => first_person::spawn_level(commands, meshes, materials).all,
            "endless_runner" => endless_runner::spawn_level(commands, meshes, materials).all,
            "arena_shooter" => arena_shooter::spawn_level(commands, meshes, materials).all,
            _ => Vec::new(),
        };
        for entity in &all {
            commands.entity(*entity).insert(SceneEntity);
        }
        all
    }

    /// Builds and runs the sample in its own window (`cargo run -p <crate>` in `samples/`).
    pub fn launch(&self) -> std::io::Result<std::process::Child> {
        std::process::Command::new("cargo")
            .args(["run", "-p", self.crate_name])
            .current_dir(samples_root())
            .spawn()
    }
}

/// The `samples/` workspace next to the editor's manifest.
pub fn samples_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sample_level_spawns_named_scene_entities_and_a_camera() {
        for sample in SAMPLE_GAMES {
            let mut world = World::new();
            world.init_resource::<Assets<Mesh>>();
            world.init_resource::<Assets<StandardMaterial>>();
            let all = world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                world.resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
                    let mut commands = world.commands();
                    let all = sample.spawn_level(&mut commands, &mut meshes, &mut materials);
                    world.flush();
                    all
                })
            });
            assert!(!all.is_empty(), "{} spawned nothing", sample.crate_name);
            assert!(all.iter().all(|e| world.get::<SceneEntity>(*e).is_some() && world.get::<Name>(*e).is_some()));
            assert!(all.iter().any(|e| world.get::<Camera3d>(*e).is_some()), "{} has no camera", sample.crate_name);
            assert!(sample.dir().join("Cargo.toml").is_file());
        }
    }
}

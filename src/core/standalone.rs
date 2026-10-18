//! Explicit Cargo launch for the independent user project.
use bevy::prelude::*;
pub(super) fn launch(world: &mut World) {

let project_dir = world
    .get_resource::<crate::ui::hub::ActiveProject>()
    .and_then(|p| p.project_dir.clone());

if let Some(proj_dir) = project_dir {
    info!("Launching Standalone Application via 'cargo run' for {}...", proj_dir.display());
    info!("(Note: Standalone runner builds the independent Rust binary with Cargo. For instant 0s play on loaded models, click '▶ Play'!)");

    // Save active scene as default_scene.bsn so standalone game runs current edits
    let scenes_dir = proj_dir.join("assets").join("scenes");
    let _ = std::fs::create_dir_all(&scenes_dir);
    let default_scene_path = scenes_dir.join("default_scene.bsn");
    if let Err(e) = crate::scene::save_load::save_world_to_scene(world, &default_scene_path) {
        warn!("Could not save active scene before launching window: {e}");
    }

    // Sync models to user project if needed
    let editor_models = std::path::Path::new("assets/models");
    let project_models = proj_dir.join("assets").join("models");
    if editor_models.is_dir() {
        let _ = std::fs::create_dir_all(&project_models);
        if let Ok(entries) = std::fs::read_dir(editor_models) {
            for entry in entries.flatten() {
                let dest = project_models.join(entry.file_name());
                if !dest.exists() {
                    let _ = std::fs::copy(entry.path(), dest);
                }
            }
        }
    }

    // Spawn standalone game process (zero DX12 swapchain sharing issues, pure 60FPS)
    let child_res = std::process::Command::new("cargo")
        .arg("run")
        .current_dir(&proj_dir)
        .spawn();

    match child_res {
        Ok(_) => info!("Spawned standalone game window in external process!"),
        Err(e) => warn!("Failed to launch standalone game process: {e}"),
    }
} else {
    warn!("Cannot open standalone window: No active project loaded. Open or create a project first.");
}

}

//! Core Editor Framework Subsystems.
//!
//! Provides the 6 core architectural pillars:
//! 1. Command & Undo/Redo System ([`commands`])
//! 2. Extensible Tool System ([`tools`])
//! 3. Extensible Panel System ([`panels`])
//! 4. Selection Subsystem ([`selection`])
//! 5. Play-In-Editor (PIE) & Simulation Rollback ([`play_mode`])

pub mod commands;
pub mod panels;
pub mod play_mode;
pub mod play_input;
pub mod reflect_fields;
mod standalone;
pub mod selection;
pub mod tools;

use bevy::prelude::*;

pub struct CoreEditorPlugin;

impl Plugin for CoreEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            commands::CommandPlugin,
            tools::ToolPlugin,
            panels::PanelPlugin,
            selection::SelectionPlugin,
            play_mode::PlayModePlugin,
        ));
    }
}

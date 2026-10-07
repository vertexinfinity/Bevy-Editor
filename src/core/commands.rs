//! Command & Undo/Redo System.
//!
//! Every mutating editor operation is encapsulated in an [`EditorCommand`],
//! allowing reliable Undo/Redo, macro recording, and future automation.

use bevy::prelude::*;

/// Trait implemented by any reversible editor command.
pub trait EditorCommand: Send + Sync + 'static {
    /// Human-readable description of the command (e.g. "Move Entity", "Add Script").
    fn name(&self) -> &str;
    /// Executes the command, mutating the world.
    fn execute(&mut self, world: &mut World) -> Result<(), String>;
    /// Reverses the changes made by [`execute`](EditorCommand::execute).
    fn undo(&mut self, world: &mut World) -> Result<(), String>;
}

/// Stores command history for Undo and Redo operations.
#[derive(Resource)]
#[allow(dead_code)]
pub struct CommandHistory {
    undo_stack: Vec<Box<dyn EditorCommand>>,
    redo_stack: Vec<Box<dyn EditorCommand>>,
    pub max_history: usize,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 100,
        }
    }
}

#[allow(dead_code)]
impl CommandHistory {
    /// Executes a new command and pushes it onto the undo stack.
    pub fn execute(&mut self, mut cmd: Box<dyn EditorCommand>, world: &mut World) -> Result<(), String> {
        cmd.execute(world)?;
        info!("Command executed: {}", cmd.name());
        self.undo_stack.push(cmd);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        Ok(())
    }

    /// Undoes the most recent command.
    pub fn undo(&mut self, world: &mut World) -> Result<(), String> {
        let Some(mut cmd) = self.undo_stack.pop() else {
            return Err("Nothing to undo".to_string());
        };
        cmd.undo(world)?;
        info!("Command undone: {}", cmd.name());
        self.redo_stack.push(cmd);
        Ok(())
    }

    /// Redoes the previously undone command.
    pub fn redo(&mut self, world: &mut World) -> Result<(), String> {
        let Some(mut cmd) = self.redo_stack.pop() else {
            return Err("Nothing to redo".to_string());
        };
        cmd.execute(world)?;
        info!("Command redone: {}", cmd.name());
        self.undo_stack.push(cmd);
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

// Built-in reversible commands

/// Reversible translation command.
#[allow(dead_code)]
pub struct MoveEntityCommand {
    pub entity: Entity,
    pub old_pos: Vec3,
    pub new_pos: Vec3,
}

impl EditorCommand for MoveEntityCommand {
    fn name(&self) -> &str {
        "Move Entity"
    }

    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        let mut transform = world
            .get_mut::<Transform>(self.entity)
            .ok_or_else(|| "Entity or Transform not found".to_string())?;
        transform.translation = self.new_pos;
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        let mut transform = world
            .get_mut::<Transform>(self.entity)
            .ok_or_else(|| "Entity or Transform not found".to_string())?;
        transform.translation = self.old_pos;
        Ok(())
    }
}

/// System listening for Ctrl+Z (Undo) and Ctrl+Y / Ctrl+Shift+Z (Redo) shortcuts.
pub fn handle_undo_redo_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    world: &mut World,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if ctrl && keys.just_pressed(KeyCode::KeyZ) {
        if shift {
            world.resource_scope(|w, mut hist: Mut<CommandHistory>| {
                let _ = hist.redo(w);
            });
        } else {
            world.resource_scope(|w, mut hist: Mut<CommandHistory>| {
                let _ = hist.undo(w);
            });
        }
    } else if ctrl && keys.just_pressed(KeyCode::KeyY) {
        world.resource_scope(|w, mut hist: Mut<CommandHistory>| {
            let _ = hist.redo(w);
        });
    }
}

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandHistory>()
            .add_systems(Update, handle_undo_redo_shortcuts);
    }
}

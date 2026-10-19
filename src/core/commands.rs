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
    /// Bumped whenever history applies a change (execute, undo, redo, record), so
    /// change trackers can tell history-driven edits from direct ones.
    pub epoch: u64,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 100,
            epoch: 0,
        }
    }
}

#[allow(dead_code)]
impl CommandHistory {
    /// Executes a new command and pushes it onto the undo stack.
    pub fn execute(&mut self, mut cmd: Box<dyn EditorCommand>, world: &mut World) -> Result<(), String> {
        cmd.execute(world)?;
        info!("Command executed: {}", cmd.name());
        self.epoch += 1;
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
        self.epoch += 1;
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
        self.epoch += 1;
        self.undo_stack.push(cmd);
        Ok(())
    }

    /// Records a change that has already been applied (e.g. a finished gizmo drag).
    pub fn record(&mut self, cmd: Box<dyn EditorCommand>) {
        info!("Command recorded: {}", cmd.name());
        self.epoch += 1;
        self.undo_stack.push(cmd);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
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
#[derive(Default)]
pub struct EditorModifiers { ctrl: bool, shift: bool }

pub fn handle_undo_redo_shortcuts(
    world: &mut World,
    mut cursor: Local<bevy::ecs::message::MessageCursor<bevy::input::keyboard::KeyboardInput>>,
    mut modifiers: Local<EditorModifiers>,
) {
    let events = cursor.read(world.resource::<Messages<bevy::input::keyboard::KeyboardInput>>()).cloned().collect::<Vec<_>>();
    let primary = world.query_filtered::<(Entity,&Window), With<bevy::window::PrimaryWindow>>().iter(world).find(|(_,window)|window.focused).map(|(entity,_)|entity);
    if primary.is_none() { *modifiers=default(); return; }
    if world.get_resource::<crate::core::play_mode::PlayState>().is_some_and(|state|*state!=crate::core::play_mode::PlayState::Edit) { *modifiers=default(); return; }
    for event in events.into_iter().filter(|event|Some(event.window)==primary) {
        match event.key_code {
            KeyCode::ControlLeft | KeyCode::ControlRight => modifiers.ctrl=event.state.is_pressed(),
            KeyCode::ShiftLeft | KeyCode::ShiftRight => modifiers.shift=event.state.is_pressed(),
            key if event.state.is_pressed() && !event.repeat && modifiers.ctrl && matches!(key,KeyCode::KeyZ|KeyCode::KeyY) => {
                if world.get_resource::<crate::ui::inspector::property_editor::PropertyEditState>().is_some_and(|edit|edit.is_editing()) { continue; }
                let redo=key==KeyCode::KeyY || modifiers.shift;
                world.resource_scope(|world,mut history:Mut<CommandHistory>| { let _=if redo {history.redo(world)} else {history.undo(world)}; });
            }
            _=>{}
        }
    }
}

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandHistory>()
            .add_systems(Update, handle_undo_redo_shortcuts);
    }
}

//! Selection Management Subsystem.
//!
//! Manages active entity selection (single and multi-select), entity focus,
//! and notifies listeners via [`SelectionChangedMessage`].

#![allow(dead_code)]

use bevy::{
    ecs::message::{Message, MessageWriter},
    inspector::InspectorSelection,
    prelude::*,
};

/// Tracks selected entities in the editor.
#[derive(Resource, Default, Debug, Clone)]
pub struct EditorSelection {
    pub selected: Vec<Entity>,
}

impl EditorSelection {
    pub fn primary(&self) -> Option<Entity> {
        self.selected.first().copied()
    }

    pub fn set_single(&mut self, entity: Entity) {
        self.selected.clear();
        self.selected.push(entity);
    }

    pub fn clear(&mut self) {
        self.selected.clear();
    }

    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected.contains(&entity)
    }

    pub fn toggle(&mut self, entity: Entity) {
        if let Some(pos) = self.selected.iter().position(|e| *e == entity) {
            self.selected.remove(pos);
        } else {
            self.selected.push(entity);
        }
    }
}

#[derive(Message, Clone, Debug)]
pub struct SelectionChangedMessage {
    pub selected: Vec<Entity>,
}

/// Keeps `EditorSelection` and `InspectorSelection` synchronized.
pub fn sync_editor_selection(
    mut editor_sel: ResMut<EditorSelection>,
    inspector_sel: Res<InspectorSelection>,
    mut writer: MessageWriter<SelectionChangedMessage>,
) {
    if inspector_sel.is_changed() {
        if let Some(ent) = inspector_sel.0 {
            if editor_sel.primary() != Some(ent) {
                editor_sel.set_single(ent);
                writer.write(SelectionChangedMessage {
                    selected: editor_sel.selected.clone(),
                });
            }
        } else if !editor_sel.selected.is_empty() {
            editor_sel.clear();
            writer.write(SelectionChangedMessage {
                selected: Vec::new(),
            });
        }
    }
}

pub struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorSelection>()
            .add_message::<SelectionChangedMessage>()
            .add_systems(Update, sync_editor_selection);
    }
}

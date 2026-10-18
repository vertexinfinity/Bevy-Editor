//! Rust plugins can register inspector sections without modifying the dock builder.
use crate::ui::panel::CardBuilder;
use bevy::prelude::*;

/// The callback receives its section body. Plugin systems own data binding and visibility.
pub struct InspectorSection {
    pub id: &'static str,
    pub title: &'static str,
    pub order: i32,
    pub spawn: fn(&mut Commands, Entity),
}
#[derive(Resource, Default)]
pub struct InspectorSectionRegistry {
    sections: Vec<InspectorSection>,
}
impl InspectorSectionRegistry {
    /// Register before entering the editor. Duplicate IDs replace earlier registrations.
    pub fn register(&mut self, section: InspectorSection) {
        self.sections.retain(|existing| existing.id != section.id);
        self.sections.push(section);
        self.sections
            .sort_by_key(|section| (section.order, section.id));
    }
    pub(crate) fn spawn_sections(&self, commands: &mut Commands, parent: Entity) {
        for section in &self.sections {
            let (_, _, body) = CardBuilder::new(section.title).spawn_shell(commands, parent);
            (section.spawn)(commands, body);
        }
    }
}

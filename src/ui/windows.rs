//! Registry for editor tool windows (Shader Editor, UI Editor, ...) listed in the File menu.
//!
//! Feature plugins register an entry in `Plugin::build`; the menu bar shows one item per
//! entry under "Windows", and clicking it runs the entry's `toggle` with world access.
use bevy::{
    picking::{
        events::{PointerClick, PointerOut, PointerOver},
        pointer::PointerButton,
    },
    prelude::*,
};

use crate::scene::EditorInternal;
use crate::ui::theme::*;

#[derive(Clone)]
pub struct EditorWindowEntry {
    pub id: &'static str,
    pub label: &'static str,
    /// Opens or closes the window.
    pub toggle: fn(&mut World),
}

#[derive(Resource, Default, Clone)]
pub struct EditorWindowRegistry {
    pub entries: Vec<EditorWindowEntry>,
}

impl EditorWindowRegistry {
    /// Registers a window; a duplicate `id` replaces the earlier entry.
    pub fn register(&mut self, entry: EditorWindowEntry) {
        self.entries.retain(|e| e.id != entry.id);
        self.entries.push(entry);
    }

    pub fn toggle(world: &mut World, id: &str) -> bool {
        let toggle = world.resource::<Self>().entries.iter().find(|e| e.id == id).map(|e| e.toggle);
        if let Some(toggle) = toggle {
            toggle(world);
        }
        toggle.is_some()
    }
}

/// Registers a window from a plugin's `build`.
pub fn register_window(app: &mut App, entry: EditorWindowEntry) {
    app.init_resource::<EditorWindowRegistry>();
    app.world_mut().resource_mut::<EditorWindowRegistry>().register(entry);
}

/// Adds one dropdown item per registered window under `parent` (the File menu popup).
pub fn spawn_window_menu_items(commands: &mut Commands, parent: Entity, registry: &EditorWindowRegistry) {
    for entry in &registry.entries {
        let id = entry.id;
        let button = commands
            .spawn((
                EditorInternal,
                Node { width: percent(100.0), padding: UiRect::axes(px(8), px(5)), border_radius: radius_button(), align_items: AlignItems::Center, ..default() },
                BackgroundColor(BG_CARD),
                ChildOf(parent),
            ))
            .observe(|t: On<PointerOver>, mut bg: Query<&mut BackgroundColor>| {
                if let Ok(mut bg) = bg.get_mut(t.entity) { *bg = BackgroundColor(BG_HOVER); }
            })
            .observe(|t: On<PointerOut>, mut bg: Query<&mut BackgroundColor>| {
                if let Ok(mut bg) = bg.get_mut(t.entity) { *bg = BackgroundColor(BG_CARD); }
            })
            .observe(move |click: On<PointerClick>, mut commands: Commands, mut menu: ResMut<crate::ui::menu::FileMenuState>| {
                if click.button == PointerButton::Primary {
                    menu.0 = false;
                    commands.queue(move |world: &mut World| {
                        EditorWindowRegistry::toggle(world, id);
                    });
                }
            })
            .id();
        commands.spawn((
            EditorInternal,
            Pickable::IGNORE,
            Text::new(entry.label),
            TextFont { font_size: bevy::text::FontSize::Px(11.5), ..default() },
            TextColor(TEXT_PRIMARY),
            ChildOf(button),
        ));
    }
}

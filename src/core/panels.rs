//! Extensible Panel Management System.
//!
//! Enables modular registration and toggling of editor panels (Hierarchy, Inspector,
//! Asset Browser, Console, Animation Editor, etc.) without hardcoding layout references.

#![allow(dead_code)]

use std::collections::HashMap;
use bevy::prelude::*;

/// Trait defining a modular editor panel.
pub trait EditorPanel: Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn is_open(&self) -> bool;
    fn set_open(&mut self, open: bool);
    fn toggle(&mut self) {
        let current = self.is_open();
        self.set_open(!current);
    }
}

/// Dynamic panel instance wrapper.
#[derive(Clone)]
pub struct PanelState {
    pub id: &'static str,
    pub title: &'static str,
    pub is_open: bool,
}

/// Registry managing all available editor panels.
#[derive(Resource, Default)]
pub struct PanelRegistry {
    panels: HashMap<&'static str, PanelState>,
    order: Vec<&'static str>,
}

impl PanelRegistry {
    pub fn register(&mut self, id: &'static str, title: &'static str, default_open: bool) {
        self.panels.insert(id, PanelState { id, title, is_open: default_open });
        if !self.order.contains(&id) {
            self.order.push(id);
        }
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.panels.get(id).map(|p| p.is_open).unwrap_or(false)
    }

    pub fn set_open(&mut self, id: &str, open: bool) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.is_open = open;
        }
    }

    pub fn toggle(&mut self, id: &str) {
        if let Some(panel) = self.panels.get_mut(id) {
            panel.is_open = !panel.is_open;
        }
    }

    pub fn list(&self) -> Vec<&PanelState> {
        self.order.iter().filter_map(|id| self.panels.get(id)).collect()
    }
}

pub struct PanelPlugin;

impl Plugin for PanelPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = PanelRegistry::default();
        registry.register("inspector", "Inspector", true);
        registry.register("assets", "Project Explorer", true);
        registry.register("hierarchy", "Hierarchy", true);
        registry.register("console", "Console / Log", false);

        app.insert_resource(registry);
    }
}

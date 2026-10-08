//! Panel types, dock zones, states, and messages.

use bevy::{ecs::message::Message, prelude::*};

/// Unique identifier for editor panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub enum PanelId {
    Hierarchy,
    Inspector,
    AssetBrowser,
    Console,
    SceneSummary,
}

/// Docking placement zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelDockZone {
    Left,
    #[default]
    Right,
    Bottom,
    Floating,
}

/// Reusable panel metadata and configuration.
#[derive(Component, Debug, Clone)]
pub struct PanelMetadata {
    pub id: PanelId,
    pub title: String,
    pub icon: String,
    pub dock_zone: PanelDockZone,
    pub is_collapsed: bool,
    pub is_open: bool,
    pub default_width: f32,
    pub default_height: f32,
}

/// Marker for the panel root node.
#[derive(Component)]
pub struct PanelRoot(pub PanelId);

/// Marker for the panel draggable header bar.
#[derive(Component)]
pub struct PanelHeaderBar(pub PanelId);

/// Marker for the panel title text.
#[derive(Component)]
pub struct PanelTitleText(pub PanelId);

/// Marker for the panel collapse toggle button.
#[derive(Component)]
pub struct PanelCollapseButton(pub PanelId);

/// Marker for the panel collapse toggle text.
#[derive(Component)]
pub struct PanelCollapseLabel(pub PanelId);

/// Marker for the panel close button.
#[derive(Component)]
pub struct PanelCloseButton(pub PanelId);

/// Marker for the scrollable or flexible content body.
#[derive(Component)]
pub struct PanelBodyContainer(pub PanelId);

/// Message to toggle a panel's open/closed state.
#[derive(Message, Clone, Copy, Debug)]
pub struct TogglePanelMessage(pub PanelId);

/// Message to toggle a panel's collapsed/expanded state.
#[derive(Message, Clone, Copy, Debug)]
pub struct TogglePanelCollapseMessage(pub PanelId);

/// Message to change a panel's docking zone.
#[derive(Message, Clone, Copy, Debug)]
pub struct SetPanelDockMessage {
    pub id: PanelId,
    pub zone: PanelDockZone,
}

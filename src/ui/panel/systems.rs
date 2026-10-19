//! Panel runtime systems: collapse toggling, visibility open/close, and docking layout updates.

use bevy::{
    ecs::message::MessageReader,
    prelude::*,
};
use super::types::*;

pub fn handle_panel_collapse_messages(
    mut messages: MessageReader<TogglePanelCollapseMessage>,
    mut panel_query: Query<(&PanelRoot, &mut PanelMetadata, &mut Node)>,
    mut body_query: Query<(&PanelBodyContainer, &mut Node), Without<PanelRoot>>,
    mut label_query: Query<(&PanelCollapseLabel, &mut Text)>,
) {
    for msg in messages.read() {
        let target_id = msg.0;

        for (root, mut meta, mut root_node) in &mut panel_query {
            if root.0 == target_id {
                meta.is_collapsed = !meta.is_collapsed;

                if meta.is_collapsed {
                    root_node.height = px(32);
                } else {
                    root_node.height = px(meta.default_height);
                }

                // Update collapse arrow label
                for (label, mut text) in &mut label_query {
                    if label.0 == target_id {
                        text.0 = if meta.is_collapsed { "^".to_string() } else { "v".to_string() };
                    }
                }

                // Show / hide body content
                for (body, mut body_node) in &mut body_query {
                    if body.0 == target_id {
                        crate::ui::set_display(&mut body_node, if meta.is_collapsed { Display::None } else { Display::Flex });
                    }
                }
            }
        }
    }
}

pub fn handle_panel_visibility_messages(
    mut messages: MessageReader<TogglePanelMessage>,
    mut panel_query: Query<(&PanelRoot, &mut PanelMetadata, &mut Node)>,
) {
    for msg in messages.read() {
        let target_id = msg.0;

        for (root, mut meta, mut root_node) in &mut panel_query {
            if root.0 == target_id {
                meta.is_open = !meta.is_open;
                crate::ui::set_display(&mut root_node, if meta.is_open { Display::Flex } else { Display::None });
            }
        }
    }
}

//! Feathers-based Entity Inspector and Tree Panel.

use bevy::{
    feathers::controls::{FeathersPane, FeathersSplitPane, FeathersSplitPaneHandle},
    inspector::{
        details_panel::details_panel,
        entity_tree::{entity_tree_panel, InspectorUi},
    },
    prelude::*,
    ui_widgets::split_pane_self_update,
};
use crate::scene::EditorInternal;

/// Spawns the inspector UI docking split pane on the right side of the viewport.
pub fn spawn_inspector_panel() -> impl Scene {
    bsn! {
        InspectorUi
        EditorInternal
        Name::new("InspectorDock")
        Node {
            position_type: PositionType::Absolute,
            top: px(36), // below top menu bar
            right: px(0),
            bottom: px(28), // above bottom status bar
            width: px(520),
            padding: px(8),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
        }
        Pickable::IGNORE
        Children [
            @FeathersSplitPane
            Node {
                width: percent(100),
                height: percent(100),
            }
            on(split_pane_self_update)
            Children [
                @FeathersPane { @size: 1.0, @min_size: 180.0 }
                Children [
                    @entity_tree_panel()
                    Node {
                        width: Val::Auto,
                    }
                ]
                --
                @FeathersSplitPaneHandle
                --
                @FeathersPane { @size: 1.5, @min_size: 220.0 }
                Children [
                    @details_panel()
                    Node {
                        width: Val::Auto,
                    }
                ]
            ]
        ]
    }
}

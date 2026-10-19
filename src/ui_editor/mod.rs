//! UI Editor: author in-game UI (bevy_ui) visually, saved with the scene.
//!
//! Open it from **File → 🧱 UI Editor** (or `editor/ui_editor`). The window has an element
//! palette (Panel, Row, Column, Text, Button, Image, Progress Bar, Checkbox, Slider, Spacer),
//! a tree of the game UI with select / duplicate / delete / move up-down / move-to-root, and a
//! property sheet (anchor presets, offsets, size in px/%/auto, flex layout, gap, padding, margin,
//! colors, border, radius, text, font size, visibility, button click action, widget values).
//! Every edit goes through the shared undo history (Ctrl+Z / Ctrl+Y).
//!
//! While the window is open the selected element is outlined in the viewport; clicking game UI
//! selects the deepest element under the cursor, and dragging moves it (updates its anchor
//! offset). "Game UI: shown/hidden" hides authored UI in the editor viewport only.
//!
//! Game UI elements are ordinary scene entities (`SceneEntity` + `Node` + reflected
//! components), so they are saved/loaded with the scene and copied into Play mode, where
//! progress bars, checkboxes and sliders react to input ([`model::UiProgressBar`],
//! [`model::UiCheckbox`], [`model::UiSlider`] hold the values scripts can read) and buttons run
//! their [`UiButtonClickAction`](crate::scene::components::UiButtonClickAction).
//!
//! Anchors: an anchored element is `position_type: Absolute`. Left/top/right/bottom anchors keep
//! the offset in that inset; center anchors use `left/top: 50%` plus a `UiTransform` of -50% and
//! keep the offset in `margin.left/top`; `stretch` pins all four insets; `none` is normal flow.
//!
//! # Remote methods (JSON-RPC / MCP `bevylite_editor_rpc`)
//!
//! | Method | Params | Purpose |
//! | :-- | :-- | :-- |
//! | `editor/ui_editor` | `open`?, `show_game_ui`? | Open/close (toggles without params); lists kinds, properties, anchors |
//! | `editor/ui_tree` | `props`? (default true) | Nested tree of authored game UI with every property |
//! | `editor/ui_add` | `kind`, `parent`? (id, name or null), `text`?, `name`?, `props`? or any property keys | Add an element (root elements are centered) |
//! | `editor/ui_set` | `entity`\|`name`, then property keys (or `props: {}`; `rename` renames) | Undoable property edit |
//! | `editor/ui_get` | `entity`\|`name` | One element with properties and children |
//! | `editor/ui_delete` | `entity`\|`name` | Undoable delete of the element and its children |
//! | `editor/ui_move` | `entity`\|`name`, `parent`? (id, name, null/"root"), `index`?, or `delta` (-1/+1) | Reparent / reorder |
//! | `editor/ui_duplicate` | `entity`\|`name` | Duplicate an element subtree next to itself |
//! | `editor/ui_pointer` | `x`, `y`, `to_x`?, `to_y`?, `steps`?, `window`? | Inject a real mouse click (or drag to `to_*`) in logical px, for automated testing (`window`: id or `"game_view"`) |
//! | `editor/ui_screenshot` | `path`, `window`? (`"game_view"`) | Screenshot the editor or the Game View window |
//!
//! Property keys (all values may be strings; numbers/bools are accepted too): `name`, `anchor`
//! (`top_left` `top` `top_right` `left` `center` `right` `bottom_left` `bottom` `bottom_right`
//! `stretch` `none`), `x`, `y` (offset px), `width`, `height`, `min_*`/`max_*`, `left` `right`
//! `top` `bottom` (sizes: `120`, `120px`, `50%`, `10vw`, `auto`), `position`, `display`,
//! `direction`, `justify`, `align`, `align_self`, `wrap`, `gap`, `grow`, `shrink`, `padding`,
//! `margin` (CSS shorthand `8` / `8 16` / `t r b l`), `background`, `border_width`,
//! `border_color`, `radius`, `clip`, `visible`, `text`, `font_size`, `text_color`, `text_align`,
//! `action` (`log:msg`, `toggle_light`, `spawn_cube`, `print_stats`, `custom:name`),
//! `action_arg`, `value`, `min`, `max`, `step`, `checked`, `fill_color`, `image` (asset path),
//! `tint`. Colors are `#RRGGBB[AA]` or `none`.
use bevy::{camera::visibility::VisibilitySystems, input::InputSystems, prelude::*};
use serde_json::Value;

pub mod canvas;
pub mod model;
pub mod panel;
pub mod pointer;
pub mod rpc;
pub mod widgets;

use crate::scene::{EditorInternal, SceneEntity};
use crate::ui::windows::{register_window, EditorWindowEntry};

/// UI Editor window and canvas state.
#[derive(Resource)]
pub struct UiEditorState {
    pub open: bool,
    /// When false, authored game UI is hidden in the editor viewport (not in Play mode).
    pub show_game_ui: bool,
    /// Image path used by the palette's Image button.
    pub image_path: String,
    pub status: String,
    pub(crate) status_sel: Option<Entity>,
    pub(crate) ui_hidden_applied: bool,
    pub(crate) window: Option<Entity>,
    pub(crate) outline: Option<Entity>,
    pub(crate) editing: Option<panel::TextEdit>,
    pub(crate) drag: Option<canvas::DragState>,
    pub(crate) tree_sig: Vec<(Entity, usize, String)>,
    pub(crate) tree_sel: Option<Entity>,
    pub(crate) tree_built: bool,
    pub(crate) props_for: Option<(Entity, model::UiElementKind)>,
    pub(crate) props_built: bool,
}

impl Default for UiEditorState {
    fn default() -> Self {
        Self {
            open: false,
            show_game_ui: true,
            image_path: String::new(),
            status: String::new(),
            status_sel: None,
            ui_hidden_applied: false,
            window: None,
            outline: None,
            editing: None,
            drag: None,
            tree_sig: Vec::new(),
            tree_sel: None,
            tree_built: false,
            props_for: None,
            props_built: false,
        }
    }
}

fn toggle_window(world: &mut World) {
    let mut st = world.resource_mut::<UiEditorState>();
    st.open = !st.open;
}

pub struct UiEditorPlugin;

impl Plugin for UiEditorPlugin {
    fn build(&self, app: &mut App) {
        model::register_types(app);
        app.init_resource::<UiEditorState>();
        register_window(app, EditorWindowEntry { id: "ui_editor", label: "🧱 UI Editor", toggle: toggle_window });
        widgets::install(app);
        pointer::install(app);
        app.add_systems(PreUpdate, panel::swallow_keys_while_editing.after(InputSystems))
            .add_systems(
                Update,
                (panel::text_input, panel::update_window, canvas::canvas_interaction, canvas::update_outline)
                    .chain()
                    .run_if(in_state(crate::ui::EditorAppState::InEditor)),
            )
            .add_systems(Update, detach_ui_from_non_ui_parents)
            .add_systems(PostUpdate, canvas::apply_game_ui_visibility.after(VisibilitySystems::VisibilityPropagate));
    }
}

/// A scene file spawns its top-level entities under the scene root entity, which is not a UI
/// node; a UI node under a non-UI parent is never laid out, so authored UI roots are detached.
#[allow(clippy::type_complexity)]
fn detach_ui_from_non_ui_parents(
    mut commands: Commands,
    moved: Query<(Entity, &ChildOf), (With<Node>, With<SceneEntity>, Without<EditorInternal>, Changed<ChildOf>)>,
    nodes: Query<(), With<Node>>,
) {
    for (e, child_of) in &moved {
        if !nodes.contains(child_of.parent()) {
            commands.entity(e).remove::<ChildOf>();
        }
    }
}

/// Remote methods owned by this module; `None` means the method is not ours.
pub fn dispatch(world: &mut World, method: &str, params: Option<Value>) -> Option<Result<Value, (i32, String)>> {
    rpc::dispatch(world, method, params)
}

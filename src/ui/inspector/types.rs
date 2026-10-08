//! Inspector types, state, and message declarations.
use bevy::{ecs::message::Message, prelude::*};
/// Docking zones for panels (Left, Right, or Floating).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelDockZone {
    #[default]
    Right,
    Left,
    Floating,
}

/// State managing the inspector's display mode, coordinate frame, and docking.
#[derive(Resource, Debug, Clone)]
pub struct InspectorState {
    pub is_debug_view: bool,
    pub is_global_coords: bool,
    pub is_collapsed: bool,
    pub dock_zone: PanelDockZone,
    pub width: f32,
    #[allow(dead_code)]
    pub is_dragging: bool,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            is_debug_view: false, // Default is clean Transform view
            is_global_coords: false, // Default is Local coordinates
            is_collapsed: false, // Default is expanded dock
            dock_zone: PanelDockZone::Right,
            width: 380.0,
            is_dragging: false,
        }
    }
}

// Markers for UI elements
#[derive(Component)]
pub struct InspectorDockRoot;

#[derive(Component)]
pub struct CleanViewRoot;

#[derive(Component)]
pub struct DebugViewRoot;

#[derive(Component)]
pub struct SelectedEntityLabel;

#[derive(Component)]
pub struct CoordModeLabel;

#[derive(Component)]
pub struct ModeToggleLabel;

#[derive(Component)]
pub struct DockToggleLabel;

#[derive(Component)]
pub struct InspectorHeaderTitle;

#[derive(Component)]
pub struct InspectorModeBtn;

#[derive(Component)]
pub struct ComponentsBadgesLabel;

#[derive(Component)]
pub struct TransformAxisValue {
    pub(crate) property: TransformProperty,
    pub(crate) axis: TransformAxis,
}

#[derive(Component)]
pub struct InspectorAddMenu;

#[derive(Component)]
pub struct ScriptPropsLabel;

// Messages for Inspector Actions
#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleDebugModeMessage;

#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleDockMessage;

#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleCoordModeMessage;

#[derive(Message, Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct SelectEntityMessage(pub Entity);

#[derive(Message, Clone, Copy, Debug)]
pub enum TransformNudgeAction {
    AdjustAxis {
        property: TransformProperty,
        axis: TransformAxis,
        delta: f32,
    },
    TranslateX(f32),
    TranslateY(f32),
    TranslateZ(f32),
    RotateX(f32),
    RotateY(f32),
    #[allow(dead_code)]
    RotateZ(f32),
    ScaleUniform(f32),
    ResetPosition,
    ResetRotation,
    ResetScale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformProperty {
    Position,
    Rotation,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformAxis {
    X,
    Y,
    Z,
}

#[derive(Message, Clone, Debug)]
pub enum AddComponentKind {
    Script,
    ScriptAsset(String),
    MeshCube,
    MeshSphere,
    MeshPlane,
    SpinAnimation,
    Velocity,
    PointLight,
    DirectionalLight,
    SpotLight,
    Camera3d,
    GameplayTag,
}

pub(crate) struct ScriptDirectory {
    pub(crate) name: String,
    pub(crate) scripts: Vec<(String, String)>,
    pub(crate) directories: Vec<ScriptDirectory>,
}

#[derive(Message, Clone, Debug)]
pub enum ScriptPropAction {
    NudgeFloat { key: String, delta: f32 },
    ToggleBool { key: String },
}

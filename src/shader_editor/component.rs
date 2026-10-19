//! Scene-facing shader assignment: [`ShaderMaterialRef`] is the saved source of truth and
//! [`sync_shader_materials`] keeps every entity's `MeshMaterial3d<EditorShaderMaterial>` in
//! step with it (scene load, Play-mode copies, undo/redo and Inspector edits included).
use bevy::{
    annotations::{DisplayName, Step, Tooltip, ValueRange},
    prelude::*,
    shader::Shader,
};

use super::library::ShaderLibrary;
use super::material::{EditorShaderMaterial, EditorShaderUniform};
use crate::core::commands::EditorCommand;

/// Assigns a WGSL shader from `assets/shaders` to this mesh, with its parameter values.
#[derive(Component, Reflect, Clone, Debug, PartialEq)]
#[reflect(Component, Default, @Tooltip("Renders this mesh with a WGSL shader from the Shader Editor"))]
pub struct ShaderMaterialRef {
    #[reflect(@DisplayName("Shader"), @Tooltip("Asset path of the .wgsl file, e.g. shaders/templates/toon.wgsl"))]
    pub shader: String,
    #[reflect(@DisplayName("Base Color"), @Tooltip("material.color in the shader"))]
    pub color: Color,
    #[reflect(@DisplayName("Param 0"), @ValueRange(-1000.0..=1000.0), @Step(0.05), @Tooltip("p0() in the shader; see the Shader Editor for its name and range"))]
    pub p0: f32,
    #[reflect(@DisplayName("Param 1"), @ValueRange(-1000.0..=1000.0), @Step(0.05), @Tooltip("p1() in the shader"))]
    pub p1: f32,
    #[reflect(@DisplayName("Param 2"), @ValueRange(-1000.0..=1000.0), @Step(0.05), @Tooltip("p2() in the shader"))]
    pub p2: f32,
    #[reflect(@DisplayName("Param 3"), @ValueRange(-1000.0..=1000.0), @Step(0.05), @Tooltip("p3() in the shader"))]
    pub p3: f32,
}

impl Default for ShaderMaterialRef {
    fn default() -> Self {
        Self {
            shader: "shaders/templates/unlit_color.wgsl".into(),
            color: Color::WHITE,
            p0: 0.25,
            p1: 1.0,
            p2: 0.0,
            p3: 0.0,
        }
    }
}

impl ShaderMaterialRef {
    pub fn params(&self) -> [f32; 4] {
        [self.p0, self.p1, self.p2, self.p3]
    }
    pub fn set_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.p0 = value,
            1 => self.p1 = value,
            2 => self.p2 = value,
            _ => self.p3 = value,
        }
    }
    pub fn uniform(&self) -> EditorShaderUniform {
        let c = self.color.to_linear();
        EditorShaderUniform { color: Vec4::new(c.red, c.green, c.blue, c.alpha), params: Vec4::from_array(self.params()) }
    }
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "shader": self.shader,
            "color": crate::core::reflect_fields::color_hex(self.color),
            "params": self.params(),
        })
    }
}

/// Runtime link between an entity and the material built for it (never saved or copied,
/// so Play-mode copies and loaded scenes get their own material).
#[derive(Component)]
pub struct ShaderMaterialLink {
    pub material: Handle<EditorShaderMaterial>,
    pub shader: String,
    pub revision: u64,
}

/// The standard material the mesh had before a shader was applied (restored on revert).
#[derive(Component)]
pub struct StashedStandardMaterial(pub Handle<StandardMaterial>);

/// Builds, updates and tears down shader materials to match [`ShaderMaterialRef`].
pub fn sync_shader_materials(world: &mut World) {
    // 1. Entities that need a (new) material or an update.
    type Wanted = (Entity, ShaderMaterialRef, bool, Option<(Handle<EditorShaderMaterial>, String, u64)>, bool);
    let wanted: Vec<Wanted> = world
        .query_filtered::<(Entity, Ref<ShaderMaterialRef>, Option<&ShaderMaterialLink>, Option<&MeshMaterial3d<EditorShaderMaterial>>), With<Mesh3d>>()
        .iter(world)
        .map(|(entity, shader_ref, link, mesh_material)| {
            let attached = match (link, mesh_material) {
                (Some(l), Some(m)) => l.material.id() == m.0.id(),
                _ => false,
            };
            let link_data = link.map(|l| (l.material.clone(), l.shader.clone(), l.revision));
            (entity, (*shader_ref).clone(), shader_ref.is_changed(), link_data, attached)
        })
        .collect();
    for (entity, shader_ref, changed, link, attached) in wanted {
        let Some((handle, handle_no_uv, alpha, revision)) = world.resource_scope(|world, mut library: Mut<ShaderLibrary>| {
            let mut shaders = world.resource_mut::<Assets<Shader>>();
            library
                .ensure(&mut shaders, &shader_ref.shader)
                .map(|e| (e.handle.clone(), e.handle_no_uv.clone(), e.meta.alpha.alpha_mode(), e.revision))
        }) else {
            continue;
        };
        // Shader edits bump `revision` (metadata such as the alpha mode may have changed).
        let up_to_date = attached && !changed && link.as_ref().is_some_and(|(_, shader, rev)| *shader == shader_ref.shader && *rev == revision);
        if up_to_date {
            continue;
        }
        let material = EditorShaderMaterial { uniform: shader_ref.uniform(), shader: handle, shader_no_uv: handle_no_uv, alpha };
        let material_handle = {
            let mut materials = world.resource_mut::<Assets<EditorShaderMaterial>>();
            match link.as_ref().filter(|_| attached).map(|(h, _, _)| h.clone()) {
                Some(existing) if materials.get(&existing).is_some() => {
                    if let Some(mut current) = materials.get_mut(&existing) {
                        *current = material;
                    }
                    existing
                }
                _ => materials.add(material),
            }
        };
        let mut entity_mut = world.entity_mut(entity);
        if let Some(standard) = entity_mut.take::<MeshMaterial3d<StandardMaterial>>() {
            if !entity_mut.contains::<StashedStandardMaterial>() {
                entity_mut.insert(StashedStandardMaterial(standard.0));
            }
        }
        entity_mut.insert((
            MeshMaterial3d(material_handle.clone()),
            ShaderMaterialLink { material: material_handle, shader: shader_ref.shader.clone(), revision },
        ));
    }

    // 2. Entities whose assignment was removed (revert / undo): restore the standard material.
    let removed: Vec<Entity> = world
        .query_filtered::<Entity, (With<ShaderMaterialLink>, Without<ShaderMaterialRef>)>()
        .iter(world)
        .collect();
    for entity in removed {
        let stashed = world.get::<StashedStandardMaterial>(entity).map(|s| s.0.clone());
        let standard = stashed.unwrap_or_else(|| {
            world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial { base_color: Color::srgb(0.8, 0.8, 0.8), ..default() })
        });
        world
            .entity_mut(entity)
            .remove::<(ShaderMaterialLink, StashedStandardMaterial, MeshMaterial3d<EditorShaderMaterial>)>()
            .insert(MeshMaterial3d(standard));
    }
}

/// Undoable change of an entity's shader assignment (`None` = standard material).
pub struct SetShaderRefCommand {
    pub entity: Entity,
    pub before: Option<ShaderMaterialRef>,
    pub after: Option<ShaderMaterialRef>,
    pub label: &'static str,
}

fn set_ref(world: &mut World, entity: Entity, value: &Option<ShaderMaterialRef>) -> Result<(), String> {
    let mut entity_mut = world.get_entity_mut(entity).map_err(|_| "Entity no longer exists".to_string())?;
    match value {
        Some(value) => {
            entity_mut.insert(value.clone());
        }
        None => {
            entity_mut.remove::<ShaderMaterialRef>();
        }
    }
    Ok(())
}

impl EditorCommand for SetShaderRefCommand {
    fn name(&self) -> &str {
        self.label
    }
    fn execute(&mut self, world: &mut World) -> Result<(), String> {
        set_ref(world, self.entity, &self.after)
    }
    fn undo(&mut self, world: &mut World) -> Result<(), String> {
        set_ref(world, self.entity, &self.before)
    }
}

/// Applies a new assignment through the undo history (no-op if unchanged).
pub fn set_shader_ref_with_undo(world: &mut World, entity: Entity, after: Option<ShaderMaterialRef>, label: &'static str) -> Result<(), String> {
    let before = world.get::<ShaderMaterialRef>(entity).cloned();
    if before == after {
        return Ok(());
    }
    crate::core::reflect_fields::execute_command(world, SetShaderRefCommand { entity, before, after, label })
}

/// Records an edit that was already applied live (e.g. a slider drag) as one undo step.
pub fn record_shader_ref_edit(world: &mut World, entity: Entity, before: Option<ShaderMaterialRef>, label: &'static str) -> Result<(), String> {
    let after = world.get::<ShaderMaterialRef>(entity).cloned();
    if before == after {
        return Ok(());
    }
    crate::core::reflect_fields::execute_command(world, SetShaderRefCommand { entity, before, after, label })
}

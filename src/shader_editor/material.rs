//! `EditorShaderMaterial`: one material type whose fragment shader is chosen per instance.
//!
//! `Material::fragment_shader` is static in Bevy, so the shader handle travels in the
//! material's bind-group data (the pipeline specialization key) and `specialize` swaps it into
//! the main-pass descriptor. Different handles produce different pipelines, so any number of
//! shaders render at once; replacing a `Shader` asset in place recompiles every user of it.
use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    material::OpaqueRendererMethod,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError},
    shader::Shader,
};

/// GPU layout of `material` in the prelude (`ShaderParams`).
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct EditorShaderUniform {
    /// Base color in linear space.
    pub color: Vec4,
    /// `p0()..p3()`.
    pub params: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(EditorShaderKey)]
pub struct EditorShaderMaterial {
    #[uniform(0)]
    pub uniform: EditorShaderUniform,
    /// Fragment shader for meshes with UVs.
    pub shader: Handle<Shader>,
    /// Same shader compiled for meshes without `ATTRIBUTE_UV_0` (`in.uv` is zero).
    pub shader_no_uv: Handle<Shader>,
    pub alpha: AlphaMode,
}

/// Specialization key: which compiled shader the pipeline uses.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EditorShaderKey {
    pub shader: Handle<Shader>,
    pub shader_no_uv: Handle<Shader>,
}

impl From<&EditorShaderMaterial> for EditorShaderKey {
    fn from(material: &EditorShaderMaterial) -> Self {
        Self { shader: material.shader.clone(), shader_no_uv: material.shader_no_uv.clone() }
    }
}

impl Material for EditorShaderMaterial {
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha
    }

    fn opaque_render_method(&self) -> OpaqueRendererMethod {
        OpaqueRendererMethod::Forward
    }

    /// The default prepass shader reads `StandardMaterial` bindings, which this layout lacks.
    fn enable_prepass() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Shadow / prepass pipelines keep Bevy's depth-only shaders.
        if descriptor.label.as_deref() == Some("prepass_pipeline") {
            return Ok(());
        }
        if let Some(fragment) = descriptor.fragment.as_mut() {
            let has_uv = layout.0.contains(Mesh::ATTRIBUTE_UV_0);
            fragment.shader = if has_uv { key.bind_group_data.shader.clone() } else { key.bind_group_data.shader_no_uv.clone() };
        }
        Ok(())
    }
}

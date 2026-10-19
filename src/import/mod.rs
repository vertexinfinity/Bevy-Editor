//! Model import for formats Bevy does not load natively: OBJ (+MTL), FBX, STL and PLY.
//!
//! Files are parsed on the async compute pool into plain vertex data, then turned into
//! Bevy meshes and `StandardMaterial`s on the main thread. An imported model is one scene
//! entity carrying [`ImportedModel`] (the asset path); its mesh parts are rebuilt from
//! the file whenever that component appears, so scenes save just the reference and reload
//! (and Play-mode copies) rebuild the geometry.
//!
//! Remote: `editor/import_model {path, position?, name?}` (path relative to `assets/` or
//! absolute; absolute files are copied into `assets/models/`) and
//! `editor/import_formats`.
use std::path::{Path, PathBuf};

use bevy::{
    annotations::{ReadOnly, Tooltip},
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future},
};

/// Extensions handled by this module (lower case, without the dot).
pub const IMPORT_EXTENSIONS: &[&str] = &["obj", "fbx", "stl", "ply"];

pub fn is_importable(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMPORT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// A model imported from an OBJ/FBX/STL/PLY file under `assets/`.
#[derive(Component, Reflect, Clone, Debug, Default)]
#[reflect(Component, Default, @Tooltip("Geometry is rebuilt from this file when the scene loads"))]
pub struct ImportedModel {
    #[reflect(@ReadOnly)]
    pub path: String,
}

/// Marks generated mesh parts so they are rebuilt (not saved) with the scene.
#[derive(Component)]
pub struct ImportedPart;

/// Raw geometry for one mesh part, produced off the main thread.
pub struct PartData {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub base_color: [f32; 4],
    /// Texture path relative to `assets/`, if one was found next to the model.
    pub texture: Option<String>,
}

#[derive(Component)]
struct ImportTask(Task<Result<Vec<PartData>, String>>);

pub struct ImportPlugin;

impl Plugin for ImportPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ImportedModel>()
            .add_systems(Update, (start_imports, finish_imports));
    }
}

/// Resolves an asset-relative or absolute path to a file on disk.
fn disk_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() { p.to_path_buf() } else { Path::new("assets").join(p) }
}

fn start_imports(mut commands: Commands, added: Query<(Entity, &ImportedModel), Added<ImportedModel>>) {
    for (entity, model) in &added {
        let path = model.path.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move { load_parts(&disk_path(&path), &path) });
        commands.entity(entity).insert(ImportTask(task));
    }
}

fn finish_imports(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut ImportTask, &ImportedModel, Option<&Children>, Option<&bevy::camera::visibility::RenderLayers>)>,
    parts: Query<(), With<ImportedPart>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut task, model, children, layers) in &mut tasks {
        let Some(result) = block_on(future::poll_once(&mut task.0)) else { continue };
        commands.entity(entity).remove::<ImportTask>();
        // Replace parts from an earlier build (e.g. after undo/redo or a reload).
        for child in children.into_iter().flatten() {
            if parts.contains(*child) {
                commands.entity(*child).despawn();
            }
        }
        match result {
            Ok(data) => {
                let count = data.len();
                for part in data {
                    let material = materials.add(StandardMaterial {
                        base_color: Color::srgba(part.base_color[0], part.base_color[1], part.base_color[2], part.base_color[3]),
                        base_color_texture: part.texture.as_ref().map(|t| asset_server.load(t.clone())),
                        alpha_mode: if part.base_color[3] < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
                        perceptual_roughness: 0.6,
                        ..default()
                    });
                    let mut part_entity = commands.spawn((
                        ImportedPart,
                        Name::new(part.name.clone()),
                        Mesh3d(meshes.add(build_mesh(part))),
                        MeshMaterial3d(material),
                        Transform::IDENTITY,
                        ChildOf(entity),
                    ));
                    // Play-mode copies render on their own layer; parts must follow.
                    if let Some(layers) = layers {
                        part_entity.insert(layers.clone());
                    }
                }
                info!("Imported {} ({count} parts)", model.path);
            }
            Err(error) => warn!("Could not import {}: {error}", model.path),
        }
    }
}

/// Builds a Bevy mesh; computes smooth normals and a planar UV fallback when missing.
pub fn build_mesh(part: PartData) -> Mesh {
    let count = part.positions.len();
    let uvs = if part.uvs.len() == count {
        part.uvs
    } else {
        part.positions.iter().map(|p| [p[0], p[2]]).collect()
    };
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, part.positions);
    // Lightmap baking expects a second UV channel.
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uvs.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(part.indices));
    if part.normals.len() == count {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, part.normals);
    } else {
        mesh.compute_smooth_normals();
    }
    mesh
}

/// Parses a model file into mesh parts. `asset_path` is used to resolve textures.
pub fn load_parts(file: &Path, asset_path: &str) -> Result<Vec<PartData>, String> {
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    let parts = match ext.as_str() {
        "obj" => load_obj(file, asset_path)?,
        "fbx" => load_fbx(file, asset_path)?,
        "stl" => load_stl(file)?,
        "ply" => load_ply(file)?,
        other => return Err(format!("Unsupported model format '.{other}'")),
    };
    if parts.is_empty() {
        return Err("The file contains no triangle meshes".into());
    }
    Ok(parts)
}

/// Texture path relative to `assets/`, looked up next to the model file.
fn texture_near(asset_path: &str, texture: &str) -> Option<String> {
    let texture = texture.replace('\\', "/");
    let file_name = Path::new(&texture).file_name()?.to_string_lossy().to_string();
    let model_dir = Path::new(asset_path).parent().map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
    [format!("{model_dir}/{texture}"), format!("{model_dir}/{file_name}"), format!("{model_dir}/textures/{file_name}")]
        .into_iter()
        .map(|p| p.trim_start_matches('/').to_string())
        .find(|p| disk_path(p).is_file())
}

fn load_obj(file: &Path, asset_path: &str) -> Result<Vec<PartData>, String> {
    let options = tobj::LoadOptions { triangulate: true, single_index: true, ..default() };
    let (models, materials) = tobj::load_obj(file, &options).map_err(|e| e.to_string())?;
    let materials = materials.unwrap_or_default();
    Ok(models
        .into_iter()
        .map(|model| {
            let m = model.mesh;
            let material = m.material_id.and_then(|id| materials.get(id));
            let diffuse = material.and_then(|mat| mat.diffuse).unwrap_or([0.8, 0.8, 0.8]);
            let alpha = material.and_then(|mat| mat.dissolve).unwrap_or(1.0);
            PartData {
                name: if model.name.is_empty() { "Mesh".into() } else { model.name },
                positions: m.positions.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect(),
                normals: m.normals.chunks_exact(3).map(|n| [n[0], n[1], n[2]]).collect(),
                // OBJ's V axis points up; Bevy's points down.
                uvs: m.texcoords.chunks_exact(2).map(|t| [t[0], 1.0 - t[1]]).collect(),
                indices: m.indices,
                base_color: [diffuse[0], diffuse[1], diffuse[2], alpha],
                texture: material.and_then(|mat| mat.diffuse_texture.as_deref()).and_then(|t| texture_near(asset_path, t)),
            }
        })
        .collect())
}

fn load_stl(file: &Path) -> Result<Vec<PartData>, String> {
    let mut reader = std::fs::File::open(file).map_err(|e| e.to_string())?;
    let mesh = stl_io::read_stl(&mut reader).map_err(|e| e.to_string())?;
    // STL is usually Z-up; flat-shade by giving each triangle its own vertices.
    let to_y_up = |v: [f32; 3]| [v[0], v[2], -v[1]];
    let mut positions = Vec::with_capacity(mesh.faces.len() * 3);
    let mut normals = Vec::with_capacity(mesh.faces.len() * 3);
    for face in &mesh.faces {
        let n = to_y_up(face.normal.into());
        for &i in &face.vertices {
            positions.push(to_y_up(mesh.vertices[i].into()));
            normals.push(n);
        }
    }
    let indices = (0..positions.len() as u32).collect();
    Ok(vec![PartData { name: "STL Mesh".into(), positions, normals, uvs: Vec::new(), indices, base_color: [0.75, 0.75, 0.78, 1.0], texture: None }])
}

fn load_ply(file: &Path) -> Result<Vec<PartData>, String> {
    use ply_rs_bw::{parser::Parser, ply::{DefaultElement, Property}};
    let mut reader = std::io::BufReader::new(std::fs::File::open(file).map_err(|e| e.to_string())?);
    let ply = Parser::<DefaultElement>::new().read_ply(&mut reader).map_err(|e| e.to_string())?;
    let float = |e: &DefaultElement, key: &str| -> Option<f32> {
        match e.get(key)? {
            Property::Float(v) => Some(*v),
            Property::Double(v) => Some(*v as f32),
            Property::Int(v) => Some(*v as f32),
            Property::UInt(v) => Some(*v as f32),
            Property::Short(v) => Some(*v as f32),
            Property::UShort(v) => Some(*v as f32),
            Property::Char(v) => Some(*v as f32),
            Property::UChar(v) => Some(*v as f32 / 255.0),
            _ => None,
        }
    };
    let vertices = ply.payload.get("vertex").ok_or("PLY file has no vertex element")?;
    let positions: Vec<[f32; 3]> = vertices
        .iter()
        .map(|v| [float(v, "x").unwrap_or(0.0), float(v, "y").unwrap_or(0.0), float(v, "z").unwrap_or(0.0)])
        .collect();
    let normals: Vec<[f32; 3]> = vertices.iter().filter_map(|v| Some([float(v, "nx")?, float(v, "ny")?, float(v, "nz")?])).collect();
    let uvs: Vec<[f32; 2]> = vertices
        .iter()
        .filter_map(|v| Some([float(v, "s").or_else(|| float(v, "u"))?, 1.0 - float(v, "t").or_else(|| float(v, "v"))?]))
        .collect();
    let color = vertices.first().and_then(|v| Some([float(v, "red")?, float(v, "green")?, float(v, "blue")?, 1.0])).unwrap_or([0.75, 0.75, 0.78, 1.0]);
    let mut indices = Vec::new();
    for face in ply.payload.get("face").map(|f| f.as_slice()).unwrap_or_default() {
        let list: Vec<u32> = match face.get("vertex_indices").or_else(|| face.get("vertex_index")) {
            Some(Property::ListInt(l)) => l.iter().map(|i| *i as u32).collect(),
            Some(Property::ListUInt(l)) => l.clone(),
            Some(Property::ListUChar(l)) => l.iter().map(|i| *i as u32).collect(),
            Some(Property::ListShort(l)) => l.iter().map(|i| *i as u32).collect(),
            Some(Property::ListUShort(l)) => l.iter().map(|i| *i as u32).collect(),
            _ => continue,
        };
        for k in 1..list.len().saturating_sub(1) {
            indices.extend([list[0], list[k], list[k + 1]]);
        }
    }
    if indices.is_empty() {
        return Err("PLY file has no faces (point clouds are not supported)".into());
    }
    Ok(vec![PartData { name: "PLY Mesh".into(), positions, normals, uvs, indices, base_color: color, texture: None }])
}

fn load_fbx(file: &Path, asset_path: &str) -> Result<Vec<PartData>, String> {
    let opts = ufbx::LoadOpts {
        target_axes: ufbx::CoordinateAxes::right_handed_y_up(),
        target_unit_meters: 1.0,
        generate_missing_normals: true,
        ..Default::default()
    };
    let scene = ufbx::load_file(&file.to_string_lossy(), opts).map_err(|e| format!("{:?}", e.description))?;
    let mut parts = Vec::new();
    for node in scene.nodes.iter() {
        let Some(mesh) = node.mesh.as_ref() else { continue };
        let to_world = &node.geometry_to_world;
        let mut tri = Vec::new();
        // One part per material so each keeps its color/texture.
        let groups: Vec<(Option<usize>, Vec<u32>)> = if mesh.material_parts.is_empty() {
            vec![(None, (0..mesh.faces.len() as u32).collect())]
        } else {
            mesh.material_parts.iter().map(|p| (Some(p.index as usize), p.face_indices.to_vec())).collect()
        };
        for (material_index, faces) in groups {
            let (mut positions, mut normals, mut uvs) = (Vec::new(), Vec::new(), Vec::new());
            for face_index in faces {
                let face = mesh.faces[face_index as usize];
                ufbx::triangulate_face_vec(&mut tri, mesh, face);
                for &ix in &tri {
                    let ix = ix as usize;
                    let p = ufbx::transform_position(to_world, mesh.vertex_position[ix]);
                    positions.push([p.x as f32, p.y as f32, p.z as f32]);
                    if mesh.vertex_normal.exists {
                        let n = ufbx::transform_direction(to_world, mesh.vertex_normal[ix]);
                        let n = Vec3::new(n.x as f32, n.y as f32, n.z as f32).normalize_or_zero();
                        normals.push(n.to_array());
                    }
                    if mesh.vertex_uv.exists {
                        let uv = mesh.vertex_uv[ix];
                        uvs.push([uv.x as f32, 1.0 - uv.y as f32]);
                    }
                }
            }
            if positions.is_empty() {
                continue;
            }
            let material = material_index.and_then(|i| mesh.materials.get(i));
            let (base_color, texture) = material
                .map(|m| {
                    let map = if m.pbr.base_color.has_value { &m.pbr.base_color } else { &m.fbx.diffuse_color };
                    let v = map.value_vec4;
                    let texture = map
                        .texture
                        .as_ref()
                        .and_then(|t| texture_near(asset_path, &t.relative_filename).or_else(|| texture_near(asset_path, &t.filename)));
                    ([v.x as f32, v.y as f32, v.z as f32, if map.value_components >= 4 { v.w as f32 } else { 1.0 }], texture)
                })
                .unwrap_or(([0.8, 0.8, 0.8, 1.0], None));
            let indices = (0..positions.len() as u32).collect();
            let name = if node.element.name.is_empty() { "FBX Mesh".to_string() } else { node.element.name.to_string() };
            parts.push(PartData { name, positions, normals, uvs, indices, base_color, texture });
        }
    }
    Ok(parts)
}

/// Copies `source` into `assets/models/` when it is outside `assets/`, returning the
/// asset-relative path.
pub fn ensure_in_assets(source: &Path) -> Result<String, String> {
    let assets = std::fs::canonicalize("assets").map_err(|e| e.to_string())?;
    let canonical = std::fs::canonicalize(source).map_err(|e| format!("{}: {e}", source.display()))?;
    if let Ok(relative) = canonical.strip_prefix(&assets) {
        return Ok(relative.to_string_lossy().replace('\\', "/"));
    }
    let target_dir = assets.join("models");
    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    let file_name = source.file_name().ok_or("Invalid file name")?;
    std::fs::copy(source, target_dir.join(file_name)).map_err(|e| e.to_string())?;
    // OBJ materials live in a sibling .mtl; copy it (and its textures) along.
    if source.extension().is_some_and(|e| e.eq_ignore_ascii_case("obj")) {
        let mtl = source.with_extension("mtl");
        if mtl.is_file() {
            let _ = std::fs::copy(&mtl, target_dir.join(mtl.file_name().unwrap_or_default()));
            if let Ok(text) = std::fs::read_to_string(&mtl) {
                for line in text.lines().filter(|l| l.trim_start().starts_with("map_")) {
                    if let Some(tex) = line.split_whitespace().last() {
                        let from = source.parent().unwrap_or(Path::new(".")).join(tex);
                        if from.is_file() {
                            let _ = std::fs::copy(&from, target_dir.join(Path::new(tex).file_name().unwrap_or_default()));
                        }
                    }
                }
            }
        }
    }
    Ok(format!("models/{}", file_name.to_string_lossy()))
}

/// Spawns an imported model as a scene entity; its parts are built asynchronously.
pub fn spawn_imported(commands: &mut Commands, asset_path: &str, transform: Transform, name: Option<String>) -> Entity {
    let name = name.unwrap_or_else(|| Path::new(asset_path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Model".into()));
    commands
        .spawn((crate::scene::SceneEntity, Name::new(name), ImportedModel { path: asset_path.to_string() }, transform, Visibility::default()))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str, contents: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bevylite_import_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn obj_quad_with_material_color() {
        temp("quad.mtl", b"newmtl red\nKd 1 0 0\n");
        let obj = temp("quad.obj", b"mtllib quad.mtl\no Quad\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nusemtl red\nf 1/1 2/2 3/3 4/4\n");
        let parts = load_parts(&obj, "models/quad.obj").unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].indices.len(), 6);
        assert_eq!(parts[0].base_color, [1.0, 0.0, 0.0, 1.0]);
        let mesh = build_mesh(parts.into_iter().next().unwrap());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }

    #[test]
    fn ascii_stl_and_ply_triangles() {
        let stl = temp("tri.stl", b"solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n");
        assert_eq!(load_parts(&stl, "tri.stl").unwrap()[0].positions.len(), 3);
        let ply = temp("quad.ply", b"ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n1 1 0\n0 1 0\n4 0 1 2 3\n");
        assert_eq!(load_parts(&ply, "quad.ply").unwrap()[0].indices.len(), 6);
    }
}

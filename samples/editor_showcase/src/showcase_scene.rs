//! Shared procedural sample, also copied into standalone showcase projects.
//! Keep this module independent of the editor so both scenes use identical geometry.
use bevy::prelude::*;

pub struct ShowcaseEntities {
    pub all: Vec<Entity>,
    pub root: Entity,
    pub gate: Entity,
    pub switch: Entity,
    pub trigger: Entity,
    pub beacon: Entity,
    pub camera: Entity,
    pub stress_group: Entity,
}

fn group(commands: &mut Commands, all: &mut Vec<Entity>, name: &str, parent: Option<Entity>) -> Entity {
    let mut entity = commands.spawn((Name::new(name.to_string()), Transform::default(), Visibility::Inherited));
    if let Some(parent) = parent { entity.insert(ChildOf(parent)); }
    let id = entity.id();
    all.push(id);
    id
}

fn prop(commands: &mut Commands, all: &mut Vec<Entity>, parent: Entity, name: &str, mesh: &Handle<Mesh>, material: &Handle<StandardMaterial>, position: Vec3, scale: Vec3) -> Entity {
    let id = commands.spawn((Name::new(name.to_string()), Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), Transform::from_translation(position).with_scale(scale), ChildOf(parent))).id();
    all.push(id);
    id
}

pub fn spawn_showcase(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> ShowcaseEntities {
    let mut all = Vec::new();
    let root = group(commands, &mut all, "Courtyard - Editor Showcase", None);
    let environment = group(commands, &mut all, "01 - Environment", Some(root));
    let assembly = group(commands, &mut all, "02 - Gate Assembly", Some(root));
    let gallery = group(commands, &mut all, "03 - PBR Material Gallery", Some(root));
    let lighting = group(commands, &mut all, "04 - Lighting", Some(root));
    let stress_group = group(commands, &mut all, "05 - Hierarchy Stress (64 hidden props)", Some(root));
    commands.entity(stress_group).insert(Visibility::Hidden);

    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let sphere = meshes.add(Sphere::new(0.45));
    let stone = materials.add(StandardMaterial { base_color: Color::srgb(0.48, 0.52, 0.59), perceptual_roughness: 0.9, ..default() });
    let floor = materials.add(StandardMaterial { base_color: Color::srgb(0.17, 0.21, 0.26), perceptual_roughness: 0.8, ..default() });
    let wood = materials.add(StandardMaterial { base_color: Color::srgb(0.37, 0.16, 0.06), perceptual_roughness: 0.65, ..default() });
    let brass = materials.add(StandardMaterial { base_color: Color::srgb(0.85, 0.54, 0.14), metallic: 0.9, perceptual_roughness: 0.25, ..default() });
    let glow = materials.add(StandardMaterial { base_color: Color::srgb(0.05, 0.7, 0.85), emissive: LinearRgba::rgb(0.0, 2.0, 3.0), ..default() });
    prop(commands, &mut all, environment, "Ground - Slate", &cube, &floor, Vec3::new(0.0, -0.2, 0.0), Vec3::new(24.0, 0.4, 24.0));
    for side in [-1.0, 1.0] {
        for tier in 0..4 {
            prop(commands, &mut all, environment, &format!("Tower {} - Stone {}", if side < 0.0 { "West" } else { "East" }, tier), &cube, &stone, Vec3::new(side * 3.2, 0.6 + tier as f32 * 1.2, -3.0), Vec3::new(1.8, 1.15, 1.8));
        }
    }
    prop(commands, &mut all, assembly, "Gate Lintel", &cube, &stone, Vec3::new(0.0, 4.3, -3.0), Vec3::new(5.0, 0.65, 1.1));
    let gate = prop(commands, &mut all, assembly, "GateDoor", &cube, &wood, Vec3::new(0.0, 1.8, -3.0), Vec3::new(3.0, 3.6, 0.35));
    let switch = prop(commands, &mut all, assembly, "Switch_01", &cube, &brass, Vec3::new(4.0, 1.0, 0.5), Vec3::new(0.6, 1.6, 0.6));
    let trigger = group(commands, &mut all, "TriggerZone - Gate Approach", Some(assembly));
    commands.entity(trigger).insert(Transform::from_xyz(0.0, 0.5, -0.5));
    let beacon = prop(commands, &mut all, assembly, "Beacon - Animated Emissive", &cube, &glow, Vec3::new(-4.0, 1.2, 0.5), Vec3::splat(0.65));
    for metallic in 0..5 {
        for roughness in 0..5 {
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.15 + metallic as f32 * 0.16, 0.42, 0.76 - roughness as f32 * 0.11),
                metallic: metallic as f32 / 4.0,
                perceptual_roughness: 0.05 + roughness as f32 * 0.225,
                ..default()
            });
            prop(commands, &mut all, gallery, &format!("Sphere - Metallic {:.2} Roughness {:.2}", metallic as f32 / 4.0, 0.05 + roughness as f32 * 0.225), &sphere, &material, Vec3::new(-3.0 + roughness as f32 * 1.5, 0.5, 3.0 + metallic as f32 * 1.4), Vec3::ONE);
        }
    }
    let sun = commands.spawn((Name::new("Sun - Mixed Shadows"), DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() }, Transform::from_xyz(8.0, 12.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y), ChildOf(lighting))).id();
    all.push(sun);
    let point = commands.spawn((Name::new("Point - Warm Lantern"), PointLight { color: Color::srgb(1.0, 0.55, 0.2), intensity: 250000.0, range: 12.0, shadow_maps_enabled: true, ..default() }, Transform::from_xyz(-2.0, 3.0, -1.0), ChildOf(lighting))).id();
    all.push(point);
    let spot = commands.spawn((Name::new("Spot - Cool Gallery"), SpotLight { color: Color::srgb(0.3, 0.65, 1.0), intensity: 350000.0, range: 20.0, outer_angle: 0.7, ..default() }, Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::new(0.0, 0.0, 4.0), Vec3::Y), ChildOf(lighting))).id();
    all.push(spot);
    for index in 0..64 {
        prop(commands, &mut all, stress_group, &format!("Scroll Test Prop {index:02}"), &cube, &stone, Vec3::new(-7.0 + (index % 8) as f32 * 2.0, 0.25, 12.0 + (index / 8) as f32 * 2.0), Vec3::splat(0.5));
    }
    let camera = commands.spawn((Name::new("Camera - Showcase Orbit"), Camera3d::default(), Camera { is_active: false, ..default() }, Transform::from_xyz(12.0, 9.0, 16.0).looking_at(Vec3::new(0.0, 1.0, 1.5), Vec3::Y), ChildOf(root))).id();
    all.push(camera);
    ShowcaseEntities { all, root, gate, switch, trigger, beacon, camera, stress_group }
}

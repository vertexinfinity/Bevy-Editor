//! Editor-specific authoring data layered over the standalone showcase geometry.
use bevy::prelude::*;
use std::collections::HashMap;
use super::{components::*, SceneEntity};
use crate::project::showcase_scene::{spawn_showcase, ShowcaseEntities};

pub fn spawn_editor_showcase(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> ShowcaseEntities {
    let sample = spawn_showcase(commands, meshes, materials);
    for entity in &sample.all { commands.entity(*entity).insert((SceneEntity, LightBakeMode::Mixed)); }
    commands.entity(sample.gate).insert(ScriptComponent {
        script_file: "scripts/showcase/gate_controller.lua".to_string(),
        properties: HashMap::from([
            ("open_angle".to_string(), ScriptPropertyVal::Float(90.0)),
            ("move_speed".to_string(), ScriptPropertyVal::Float(2.0)),
            ("jump_force".to_string(), ScriptPropertyVal::Float(10.0)),
            ("is_active".to_string(), ScriptPropertyVal::Bool(true)),
            ("interaction_count".to_string(), ScriptPropertyVal::Int(0)),
            ("display_name".to_string(), ScriptPropertyVal::String("Courtyard Gate".to_string())),
            ("approach_offset".to_string(), ScriptPropertyVal::Vec3(Vec3::new(0.0, 0.0, 2.0))),
        ]),
        inspector_sections: vec![
            ScriptInspectorSection { title: "Door Controller".to_string(), property_keys: vec!["open_angle".into(), "move_speed".into(), "is_active".into()] },
            ScriptInspectorSection { title: "Diagnostics / All Property Types".to_string(), property_keys: vec!["interaction_count".into(), "display_name".into(), "approach_offset".into(), "jump_force".into()] },
        ],
        entity_refs: HashMap::from([("switch".to_string(), sample.switch), ("camera".to_string(), sample.camera)]),
        enabled: true,
    });
    commands.entity(sample.switch).insert(ScriptComponent {
        script_file: "scripts/showcase/switch.lua".to_string(),
        properties: HashMap::from([("is_active".into(), ScriptPropertyVal::Bool(true))]),
        inspector_sections: vec![],
        entity_refs: HashMap::from([("gate".into(), sample.gate)]),
        enabled: true,
    });
    commands.entity(sample.trigger).insert(ScriptComponent {
        script_file: "scripts/showcase/trigger.lua".to_string(),
        properties: HashMap::from([("radius".into(), ScriptPropertyVal::Float(2.5))]),
        inspector_sections: vec![],
        entity_refs: HashMap::from([("target".into(), sample.gate), ("activator".into(), sample.switch)]),
        enabled: false,
    });
    commands.entity(sample.beacon).insert(SpinAnimation { speed: 1.0, axis: Vec3::Y });
    commands.entity(sample.gate).insert(ShowcaseGate { open: false, closed_y: 1.8 });
    commands.entity(sample.camera).insert(OrbitController { distance: 18.0, yaw: 0.6, pitch: 0.45, sensitivity: 0.004 });
    let (root, gate) = (sample.root, sample.gate);
    commands.queue(move |world: &mut World| {
        let gate_parent = world.get::<ChildOf>(gate).map(ChildOf::parent);
        let collapsed = world.query_filtered::<Entity, (With<SceneEntity>, With<Children>)>()
            .iter(world).filter(|entity| *entity != root && Some(*entity) != gate_parent).collect();
        if let Some(mut hierarchy) = world.get_resource_mut::<crate::ui::hierarchy::HierarchyState>() {
            hierarchy.collapsed_entities = collapsed;
            hierarchy.has_initialized_tree = true;
            hierarchy.filter.clear();
        }
        if let Some(mut selection) = world.get_resource_mut::<bevy::inspector::InspectorSelection>() { selection.0 = Some(gate); }
        let position = Vec3::new(11.0, 10.0, 18.0);
        let target = Vec3::new(0.0, 1.0, 1.5);
        let direction = (target - position).normalize();
        for (mut transform, mut camera) in world.query_filtered::<(&mut Transform, &mut crate::camera::FlyCamera), With<crate::camera::EditorCamera>>().iter_mut(world) {
            *transform = Transform::from_translation(position).looking_at(target, Vec3::Y);
            camera.pitch = direction.y.asin();
            camera.yaw = (-direction.x).atan2(-direction.z);
        }
    });
    sample
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn showcase_has_real_hierarchy_valid_links_and_material_variation() {
        let mut world = World::new();
        world.insert_resource(Assets::<Mesh>::default());
        world.insert_resource(Assets::<StandardMaterial>::default());
        let sample = world.run_system_once(|mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>| {
            spawn_editor_showcase(&mut commands, &mut meshes, &mut materials)
        }).unwrap();
        assert!(sample.all.len() >= 100, "Keep enough entities to exercise hierarchy scrolling");
        assert_eq!(world.query_filtered::<Entity, With<SceneEntity>>().iter(&world).count(), sample.all.len());
        assert_eq!(world.query::<&DirectionalLight>().iter(&world).count(), 1);
        assert_eq!(world.get::<Visibility>(sample.stress_group), Some(&Visibility::Hidden));
        assert_eq!(world.get::<Children>(sample.stress_group).unwrap().len(), 64);
        let gate = world.get::<ScriptComponent>(sample.gate).unwrap();
        assert_eq!(gate.entity_refs["switch"], sample.switch);
        let mut incoming = 0;
        for script in world.query::<&ScriptComponent>().iter(&world) {
            for target in script.entity_refs.values() {
                assert!(world.get::<SceneEntity>(*target).is_some(), "No dangling script entity references");
                incoming += usize::from(*target == sample.gate);
            }
        }
        assert_eq!(incoming, 2, "Gate must demonstrate incoming references from switch and trigger");
        for entity in &sample.all {
            if *entity != sample.root { assert!(world.get::<ChildOf>(*entity).is_some()); }
        }
        let materials = world.resource::<Assets<StandardMaterial>>();
        assert!(materials.iter().any(|(_, material)| material.metallic == 0.0));
        assert!(materials.iter().any(|(_, material)| material.metallic == 1.0));
        assert!(!world.get::<Camera>(sample.camera).unwrap().is_active, "Editor controls which camera is active");
    }
}

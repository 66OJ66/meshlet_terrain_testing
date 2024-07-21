use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TerrainColliderNode {
    pub colliders: Vec<Collider>,
    pub transform: Transform,
    pub children: Vec<TerrainColliderNode>,
}

impl TerrainColliderNode {
    pub fn spawn(&self, parent: &mut ChildBuilder) {
        for collider in &self.colliders {
            parent.spawn((
                TransformBundle {
                    local: self.transform,
                    ..default()
                },
                collider.clone(),
                RigidBody::Fixed,
            ));
        }

        parent.spawn(()).with_children(|inner_parent| {
            for child in &self.children {
                child.spawn(inner_parent)
            }
        });
    }
}

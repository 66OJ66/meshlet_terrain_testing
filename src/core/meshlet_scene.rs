use super::serialised_meshlet_scene::*;
use bevy::asset::*;
use bevy::pbr::experimental::meshlet::*;
use bevy::prelude::*;

pub struct MeshletScene {
    pub nodes: Vec<MeshletNode>,
}

impl MeshletScene {
    pub fn load<'a>(
        serialised_nodes: Vec<SerialisedMeshletNode>,
        gltf: &Gltf,
        load_context: &'a mut LoadContext<'_>,
    ) -> Self {
        Self {
            nodes: serialised_nodes
                .into_iter()
                .map(|node| MeshletNode::load(node, gltf, load_context, 0))
                .collect(),
        }
    }

    pub fn spawn(&self, commands: &mut Commands) -> Entity {
        commands
            .spawn(SpatialBundle::default())
            .with_children(|parent| {
                for node in &self.nodes {
                    node.spawn(parent);
                }
            })
            .id()
    }
}

/// Meshlet version of GltfNode (with un-needed fields removed)
pub struct MeshletNode {
    pub meshlets: Vec<MeshletMaterialPair>,
    pub transform: Transform,
    pub children: Vec<MeshletNode>,
}

impl MeshletNode {
    pub fn load<'a>(
        node: SerialisedMeshletNode,
        gltf: &Gltf,
        load_context: &'a mut LoadContext<'_>,
        level: usize,
    ) -> Self {
        Self {
            meshlets: node
                .meshes
                .into_iter()
                .enumerate()
                .map(|(index, mesh)| MeshletMaterialPair {
                    meshlet_handle: load_context
                        .add_labeled_asset(format!("meshlet{0}-{1}", level, index), mesh.mesh),
                    material_handle: gltf.materials[mesh.material_index].clone(),
                })
                .collect(),
            transform: node.transform,
            children: node
                .children
                .into_iter()
                .map(|child| MeshletNode::load(child, gltf, load_context, level.saturating_add(1)))
                .collect(),
        }
    }

    pub fn spawn(&self, parent: &mut ChildBuilder) {
        for meshlet in &self.meshlets {
            parent.spawn(MaterialMeshletMeshBundle {
                meshlet_mesh: meshlet.meshlet_handle.clone(),
                material: meshlet.material_handle.clone(),
                transform: self.transform,
                ..default()
            });
        }

        parent
            .spawn(SpatialBundle::default())
            .with_children(|inner_parent| {
                for child in &self.children {
                    child.spawn(inner_parent)
                }
            });
    }
}

pub struct MeshletMaterialPair {
    meshlet_handle: Handle<MeshletMesh>,
    material_handle: Handle<StandardMaterial>,
}

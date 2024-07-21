use bevy::pbr::experimental::meshlet::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SerialisedMeshletNode {
    pub meshes: Vec<SerialisedMeshlet>,
    pub transform: Transform,
    pub children: Vec<SerialisedMeshletNode>,
}

#[derive(Serialize, Deserialize)]
pub struct SerialisedMeshlet {
    pub mesh: MeshletMesh,
    pub material_index: usize,
}

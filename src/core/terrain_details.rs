use super::colliders::*;
use super::meshlet_scene::*;
use bevy::asset::*;
use bevy::prelude::*;

/// Stores terrain meshlets and colliders
#[derive(Asset, TypePath)]
pub struct TerrainDetails {
    pub _gltf_handle: Handle<Gltf>,
    pub meshlet_scene: MeshletScene,
    pub colliders: Vec<SectorColliderNode>,
}

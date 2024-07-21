use crate::core::*;
use crate::errors::*;
use bevy::asset::io::*;
use bevy::asset::saver::*;
use bevy::asset::*;
use bevy::gltf::*;
use bevy::pbr::experimental::meshlet::*;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::utils::HashMap;
use bevy_rapier3d::prelude::*;
use const_format::formatcp;
use serde::{Deserialize, Serialize};

pub const TERRAIN_PATH: &str = formatcp!("default.{}", TERRAIN_DETAILS_FILE_EXTENSION);
pub const TERRAIN_DETAILS_FILE_EXTENSION: &str = "terrain.bin";

//****************************************************************************
// ASSETS
//****************************************************************************

#[derive(Serialize, Deserialize)]
pub struct SerialisedTerrainDetails {
    pub gltf_path: String,
}

#[derive(Asset, TypePath, Serialize, Deserialize)]
pub struct ProcessedTerrainDetails {
    pub gltf_path: String,
    pub meshlet_nodes: Vec<SerialisedMeshletNode>,
    pub colliders: Vec<TerrainColliderNode>,
}

//****************************************************************************
// ASSET LOADERS
//****************************************************************************

#[derive(Default)]
pub struct ProcessedTerrainDetailsAssetLoader;

impl AssetLoader for ProcessedTerrainDetailsAssetLoader {
    type Asset = ProcessedTerrainDetails;
    type Settings = ();
    type Error = LoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let ron: SerialisedTerrainDetails = ron::de::from_bytes(&bytes)?;

        // Generate the meshlets & colliders
        let mut meshlet_nodes = Vec::new();
        let mut colliders = Vec::new();

        if !ron.gltf_path.is_empty() {
            let model_asset = load_context
                .loader()
                .direct()
                .load::<Gltf>(&ron.gltf_path)
                .await?;
            let gltf: &Gltf = model_asset.get();

            debug!("Terrain gLTF directly loaded");

            // Generate the meshlets and colliders for each Mesh in this GLTF file
            let mut processed_meshlets: HashMap<
                Handle<GltfMesh>,
                Vec<Option<(MeshletMesh, usize)>>,
            > = HashMap::with_capacity(gltf.meshes.len());

            let mut processed_colliders: HashMap<Handle<GltfMesh>, Vec<Collider>> =
                HashMap::with_capacity(gltf.meshes.len());

            for mesh_index in 0..gltf.meshes.len() {
                // Unwraps should be safe
                let gltf_mesh_asset = model_asset
                    .get_labeled(format!("Mesh{}", mesh_index))
                    .unwrap();

                let gltf_mesh = gltf_mesh_asset.get::<GltfMesh>().unwrap();

                // Get the handle as well (necessary for lookups below)
                let gltf_mesh_handle: Handle<GltfMesh> =
                    load_context.load(format!("{0}#Mesh{1}", &ron.gltf_path, mesh_index));

                let mut meshlets: Vec<Option<(MeshletMesh, usize)>> =
                    Vec::with_capacity(gltf_mesh.primitives.len());
                let mut colliders: Vec<Collider> = Vec::with_capacity(gltf_mesh.primitives.len());

                for (primitive_index, primitive) in gltf_mesh.primitives.iter().enumerate() {
                    let mesh_asset = model_asset
                        .get_labeled(format!("Mesh{0}/Primitive{1}", mesh_index, primitive_index))
                        .unwrap();

                    let mut mesh: Mesh = mesh_asset.get::<Mesh>().unwrap().clone();

                    debug!("Generating collider...");

                    let Some(collider) =
                        Collider::from_bevy_mesh(&mesh, &ComputedColliderShape::TriMesh)
                    else {
                        return Err(LoaderError::Other(
                            "Unable to generate collider for terrain mesh".to_string(),
                        ));
                    };

                    debug!("Collider generated");
                    colliders.push(collider);

                    if let Some(material_handle) = &primitive.material {
                        if !mesh.contains_attribute(Mesh::ATTRIBUTE_TANGENT) {
                            debug!("Generating tangents...");

                            mesh.generate_tangents().map_err(|e| {
                                LoaderError::Other(format!(
                                    "Unable to generate tangent for terrain mesh [{0}]",
                                    e
                                ))
                            })?;

                            debug!("Tangents generated");
                        }

                        debug!("Generating meshlets...");

                        let meshlet = MeshletMesh::from_mesh(&mesh).map_err(|e| {
                            LoaderError::Other(format!(
                                "Unable to generate meshlet for terrain mesh [{0}]",
                                e
                            ))
                        })?;

                        let material_index = gltf
                            .materials
                            .iter()
                            .position(|m| m == material_handle)
                            .unwrap();

                        debug!("Meshlets generated");

                        meshlets.push(Some((meshlet, material_index)));
                    } else {
                        meshlets.push(None);
                    }
                }

                processed_meshlets.insert(gltf_mesh_handle.clone(), meshlets);
                processed_colliders.insert(gltf_mesh_handle, colliders);
            }

            for node_index in 0..gltf.nodes.len() {
                // Unwraps should be safe (otherwise there's a bug in the GltfLoader)
                let gltf_node_asset = model_asset
                    .get_labeled(format!("Node{}", node_index))
                    .unwrap();

                let gltf_node = gltf_node_asset.get::<GltfNode>().unwrap();

                let serialised_meshlet_node =
                    gltf_node_to_meshlet_node(gltf_node, &processed_meshlets);

                meshlet_nodes.push(serialised_meshlet_node);

                let mesh_collider = gltf_node_to_collider_node(gltf_node, &processed_colliders);

                colliders.push(mesh_collider);
            }

            fn gltf_node_to_meshlet_node(
                gltf_node: &GltfNode,
                processed_meshlets: &HashMap<Handle<GltfMesh>, Vec<Option<(MeshletMesh, usize)>>>,
            ) -> SerialisedMeshletNode {
                let children = gltf_node
                    .children
                    .iter()
                    .map(|child_gltf_node| {
                        gltf_node_to_meshlet_node(child_gltf_node, processed_meshlets)
                    })
                    .collect();

                let meshes = if let Some(gltf_mesh_handle) = &gltf_node.mesh {
                    let meshlets = processed_meshlets.get(gltf_mesh_handle).unwrap();

                    meshlets
                        .iter()
                        .filter_map(|inner_meshlet| {
                            inner_meshlet
                                .clone()
                                .map(|inner_meshlet| SerialisedMeshlet {
                                    mesh: inner_meshlet.0,
                                    material_index: inner_meshlet.1,
                                })
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                SerialisedMeshletNode {
                    meshes,
                    transform: gltf_node.transform,
                    children,
                }
            }

            fn gltf_node_to_collider_node(
                gltf_node: &GltfNode,
                processed_colliders: &HashMap<Handle<GltfMesh>, Vec<Collider>>,
            ) -> TerrainColliderNode {
                let children = gltf_node
                    .children
                    .iter()
                    .map(|child_gltf_node| {
                        gltf_node_to_collider_node(child_gltf_node, processed_colliders)
                    })
                    .collect();

                let colliders = if let Some(gltf_mesh_handle) = &gltf_node.mesh {
                    let colliders = processed_colliders.get(gltf_mesh_handle).unwrap();

                    colliders.clone()
                } else {
                    Vec::new()
                };

                TerrainColliderNode {
                    colliders,
                    transform: gltf_node.transform,
                    children,
                }
            }
        } else {
            return Err(LoaderError::Other(
                StartupError::MissingGltfPath.to_string(),
            ));
        };

        Ok(ProcessedTerrainDetails {
            gltf_path: ron.gltf_path,
            meshlet_nodes,
            colliders,
        })
    }

    fn extensions(&self) -> &[&str] {
        &[TERRAIN_DETAILS_FILE_EXTENSION]
    }
}

#[derive(Default)]
pub struct TerrainDetailsAssetLoader;

impl AssetLoader for TerrainDetailsAssetLoader {
    type Asset = TerrainDetails;
    type Settings = ();
    type Error = LoaderError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut compressed_bytes = Vec::new();
        reader.read_to_end(&mut compressed_bytes).await?;
        let bytes = zstd::stream::decode_all(compressed_bytes.as_slice())?;
        let bin: ProcessedTerrainDetails = postcard::from_bytes(&bytes)?;

        fn meshlet_gltf_load_settings(settings: &mut GltfLoaderSettings) {
            settings.load_meshes = RenderAssetUsages::empty();
            settings.load_materials = RenderAssetUsages::RENDER_WORLD;
        }

        // Get a handle to the gLTF (seems to be necessary to keep material handles working after cloning below)
        let gltf_handle = load_context
            .loader()
            .with_settings(meshlet_gltf_load_settings)
            .load(&bin.gltf_path);

        // Load the gLTF directly so the material handles can be retrieved
        // Uses the above settings to avoid loading the mesh data
        let model_asset = load_context
            .loader()
            .with_settings(meshlet_gltf_load_settings)
            .direct()
            .load::<Gltf>(bin.gltf_path)
            .await?;

        let gltf: &Gltf = model_asset.get();

        let meshlet_scene = MeshletScene::load(bin.meshlet_nodes, gltf, load_context);

        Ok(TerrainDetails {
            _gltf_handle: gltf_handle,
            meshlet_scene,
            colliders: bin.colliders,
        })
    }

    fn extensions(&self) -> &[&str] {
        &[TERRAIN_DETAILS_FILE_EXTENSION]
    }
}

//****************************************************************************
// ASSET SAVERS
//****************************************************************************

pub struct ProcessedTerrainSaver;

impl AssetSaver for ProcessedTerrainSaver {
    type Asset = ProcessedTerrainDetails;
    type Settings = ();
    type OutputLoader = TerrainDetailsAssetLoader;
    type Error = SaverError;

    async fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, Self::Asset>,
        _settings: &'a Self::Settings,
    ) -> Result<(), Self::Error> {
        let bytes = postcard::to_allocvec(asset.get())?;

        // Compress using ZSTD
        let compressed_bytes = zstd::encode_all(bytes.as_slice(), 19)?;
        writer.write_all(&compressed_bytes).await?;
        debug!("Processed terrain asset written to disk");
        Ok(())
    }
}

//****************************************************************************
// RESOURCES
//****************************************************************************

#[derive(Resource)]
pub struct TerrainStartupManager {
    pub state: AssetLoadState,
    pub terrain_detail_handle: Handle<TerrainDetails>,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum AssetLoadState {
    Loading,
    Loaded,
    Failed,
}

//****************************************************************************
// ENTER SYSTEMS - GAMESTATE:STARTUP
//****************************************************************************

/// Starts loading the terrain.
pub fn asset_startup_enter_system(
    mut commands: Commands,
    // Resources
    asset_server: Res<AssetServer>,
) {
    debug!("Starting to load terrain");

    commands.insert_resource(TerrainStartupManager {
        state: AssetLoadState::Loading,
        terrain_detail_handle: asset_server.load(TERRAIN_PATH),
    });
}

//****************************************************************************
// UPDATE SYSTEMS - GAMESTATE:STARTUP
//****************************************************************************

pub fn finalise_startup_system(
    mut commands: Commands,
    // Resources
    mut manager: ResMut<TerrainStartupManager>,
    asset_server: Res<AssetServer>,
    // Assets
    terrain_details_assets: Res<Assets<TerrainDetails>>,
) {
    if manager.state != AssetLoadState::Loading {
        return;
    }

    // Check the status of the Terrain detail asset
    let terrain_details = match asset_server
        .recursive_dependency_load_state(&manager.terrain_detail_handle)
    {
        RecursiveDependencyLoadState::Loaded => {
            let Some(terrain_details) = terrain_details_assets.get(&manager.terrain_detail_handle)
            else {
                return;
            };

            terrain_details
        }

        RecursiveDependencyLoadState::Failed => {
            error!("Error whilst loading terrain details");
            manager.state = AssetLoadState::Failed;
            return;
        }

        _ => return,
    };

    let terrain_entity = terrain_details.meshlet_scene.spawn(&mut commands);

    commands
        .entity(terrain_entity)
        .insert(Name::from("Terrain"))
        .with_children(|parent| {
            for collider_node in &terrain_details.colliders {
                collider_node.spawn(parent);
            }
        });

    debug!("Terrain loaded successfully");

    manager.state = AssetLoadState::Loaded;
}

mod assets;
mod components;
mod core;
mod errors;
mod systems;

use crate::assets::*;
use crate::core::*;
use crate::systems::*;
use bevy::asset::processor::*;
use bevy::core::TaskPoolThreadAssignmentPolicy;
use bevy::pbr::experimental::meshlet::MeshletPlugin;
use bevy::prelude::*;
use bevy::tasks::available_parallelism;
use bevy::window::PresentMode;
use bevy_atmosphere::prelude::*;
use bevy_mod_wanderlust::*;
use bevy_rapier3d::prelude::*;
use bevy_water::WaterPlugin;

const LOG_LEVEL: &str =
    "naga::back::spv::writer=warn,bevy_ecs::world=error,bevy_gltf::loader=error,bevy_asset::server::loaders=error,meshlet_terrain_testing=debug";

#[derive(States, PartialEq, Eq, Debug, Default, Hash, Copy, Clone)]
/// The overall state enum for this game.
pub enum GameState {
    /// Loads the terrain details asset
    #[default]
    Startup,
    InGame,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(bevy::log::LogPlugin {
                    level: bevy::log::Level::INFO,
                    filter: LOG_LEVEL.to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    mode: AssetMode::Processed,
                    ..default()
                })
                .set(TaskPoolPlugin {
                    task_pool_options: TaskPoolOptions {
                        compute: TaskPoolThreadAssignmentPolicy {
                            // Over-provision compute threads
                            min_threads: available_parallelism(),
                            max_threads: usize::MAX,
                            percent: 1.0,
                        },
                        ..default()
                    },
                }),
            MeshletPlugin,
            // Atmosphere
            AtmospherePlugin,
            // Physics
            RapierPhysicsPlugin::<NoUserData>::default(),
            // Water
            WaterPlugin,
            // Wanderlust
            WanderlustPlugin::default()
        ))
        // Atmosphere
        .insert_resource(AtmosphereModel::default())
        // GameState
        .init_state::<GameState>()
        // Assets
        .init_asset::<ProcessedTerrainDetails>()
        .init_asset::<TerrainDetails>()
        .register_asset_loader(ProcessedTerrainDetailsAssetLoader)
        .register_asset_loader(TerrainDetailsAssetLoader)
        .register_asset_processor::<LoadAndSave<ProcessedTerrainDetailsAssetLoader, ProcessedTerrainSaver>>(
            LoadAndSave::from(ProcessedTerrainSaver),
        )
        .set_default_asset_processor::<LoadAndSave<ProcessedTerrainDetailsAssetLoader, ProcessedTerrainSaver>>(
            TERRAIN_DETAILS_FILE_EXTENSION,
        )
        // Systems - OnEnter GameState::Startup
        .add_systems(OnEnter(GameState::Startup), asset_startup_enter_system)
        // Systems - Update GameState::Startup
        .add_systems(
            Update,
            (finalise_startup_system, startup_load_complete_system)
                .distributive_run_if(in_state(GameState::Startup)),
        )
        // Systems - OnEnter GameState::InGame
        .add_systems(
            OnEnter(GameState::InGame),
            (spawn_player_enter_system, spawn_sun_enter_system, hide_mouse_enter_system),
        )
        // Systems - Update GameState::InGame
        .add_systems(
            Update,
            (movement_input_system, toggle_mouse_visibility_system).distributive_run_if(in_state(GameState::InGame)),
        )
        // Systems - Update
        .add_systems(Update, mouse_look)
        .run();
}

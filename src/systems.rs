use std::f32::consts::FRAC_2_PI;

use crate::assets::*;
use crate::components::*;
use crate::GameState;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::view::GpuCulling;
use bevy::render::view::NoCpuCulling;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_atmosphere::prelude::*;
use bevy_mod_wanderlust::*;
use bevy_rapier3d::prelude::*;

const DEFAULT_PLAYER_SPEED: f32 = 10.0;
const CAMERA_VERTICAL_OFFSET: f32 = 1.5;

//****************************************************************************
// ENTER SYSTEMS - GAMESTATE:STARTUP
//****************************************************************************

/// Transitions the GameState to InGame.
/// Runs in the Startup GameState.
pub fn startup_load_complete_system(
    // Resources
    terrain_manager: Res<TerrainStartupManager>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if terrain_manager.state == AssetLoadState::Loaded {
        info!("All startup assets loaded. Progressing to InGame");
        next_state.set(GameState::InGame);
    }
}

//****************************************************************************
// ENTER SYSTEMS - GAMESTATE:INGAME
//****************************************************************************

pub fn spawn_player_enter_system(mut commands: Commands) {
    commands
        .spawn((
            Name::from("Player"),
            Player,
            ControllerBundle {
                controller: Controller {
                    movement: Movement {
                        max_speed: DEFAULT_PLAYER_SPEED,
                        ..default()
                    },
                    ..default()
                },
                rapier_physics: RapierPhysicsBundle {
                    // Lock the axes to prevent camera shake whilst moving up slopes
                    locked_axes: LockedAxes::ROTATION_LOCKED,
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 20.0, 0.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::from("Player Camera"),
                PlayerCamera,
                Camera3dBundle {
                    camera: Camera {
                        hdr: true,
                        ..default()
                    },
                    projection: Projection::Perspective(PerspectiveProjection {
                        far: 2000.0,
                        ..default()
                    }),
                    transform: Transform::from_xyz(0.0, CAMERA_VERTICAL_OFFSET, 0.0),
                    ..default()
                },
                GpuCulling,
                NoCpuCulling,
                AtmosphereCamera::default(),
            ));
        });
}

/// Spawn a directional light.
/// Runs upon entering the InGame GameState.
pub fn spawn_sun_enter_system(mut commands: Commands) {
    commands.spawn((
        Name::from("Sun"),
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Color::srgba_u8(250, 255, 230, 0),
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
    ));
}

pub fn hide_mouse_enter_system(
    // Queries
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = window_query.single_mut();
    set_cursor_visible(&mut window, false);
}

//****************************************************************************
// UPDATE SYSTEMS - GAMESTATE:INGAME
//****************************************************************************

/// Handles WASD movement.
/// Runs in the InGame GameState.
/// Has run condition: ui_state_is_none.
pub fn movement_input_system(
    // Resources
    input: Res<ButtonInput<KeyCode>>,
    // Queries
    mut player_query: Query<(&mut ControllerInput, &mut Movement), With<Player>>,
    camera_query: Query<&GlobalTransform, With<PlayerCamera>>,
) {
    let (mut player_input, mut movement_settings) = player_query.single_mut();
    let global_camera_transform = camera_query.single();
    let mut dir = Vec3::ZERO;
    if input.pressed(KeyCode::KeyA) {
        dir += *global_camera_transform.left();
    }
    if input.pressed(KeyCode::KeyD) {
        dir += *global_camera_transform.right();
    }
    if input.pressed(KeyCode::KeyS) {
        dir += *global_camera_transform.back();
    }
    if input.pressed(KeyCode::KeyW) {
        dir += *global_camera_transform.forward();
    }

    if input.just_pressed(KeyCode::ShiftLeft) {
        movement_settings.max_speed *= 4.0;
    } else if input.just_released(KeyCode::ShiftLeft) {
        movement_settings.max_speed /= 4.0;
    }

    player_input.movement = dir;
    player_input.jumping = input.pressed(KeyCode::Space);
}

pub fn toggle_mouse_visibility_system(
    // Resources
    input: Res<ButtonInput<KeyCode>>,
    // Queries
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        let mut window = window_query.single_mut();
        let current_visibility = window.cursor.visible;
        set_cursor_visible(&mut window, !current_visibility);
    }
}

//****************************************************************************
// UPDATE SYSTEMS
//****************************************************************************

/// Handles changing the camera direction via mouse.
/// Runs in all states (otherwise MouseMotion accumulates when not in-game).
pub fn mouse_look(
    // Resources
    current_state: Res<State<GameState>>,
    time: Res<Time<Fixed>>,
    // Queries
    mut camera_query: Query<&mut Transform, With<PlayerCamera>>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<PlayerCamera>)>,
    // Events
    mut mouse_input_events: EventReader<MouseMotion>,
    // Local
    mut target_y_transform: Local<Transform>,
    mut target_x_transform: Local<Transform>,
) {
    if *current_state.get() != GameState::InGame {
        mouse_input_events.clear();
        return;
    }

    let mut camera_transform = camera_query.single_mut();
    let mut body_transform = player_query.single_mut();

    let cumulative: Vec2 = -mouse_input_events
        .read()
        .map(|motion| motion.delta)
        .sum::<Vec2>();

    // Vertical
    let camera_rotation = camera_transform.rotation;

    if (camera_rotation.x < FRAC_2_PI || cumulative.y.is_sign_negative())
        && (camera_rotation.x > -FRAC_2_PI || cumulative.y.is_sign_positive())
    {
        target_y_transform.rotate(Quat::from_scaled_axis(
            camera_rotation * Vec3::X * cumulative.y / 360.0,
        ));

        camera_transform.rotation = camera_transform.rotation.slerp(
            target_y_transform.rotation,
            (time.delta_seconds() * 19.0).min(1.0),
        );
    }

    // Horizontal
    target_x_transform.rotate(Quat::from_scaled_axis(
        body_transform.rotation * Vec3::Y * cumulative.x / 360.0,
    ));

    body_transform.rotation = body_transform.rotation.slerp(
        target_x_transform.rotation,
        (time.delta_seconds() * 19.0).min(1.0),
    );
}

//****************************************************************************
// UTILITY
//****************************************************************************

/// Used to toggle whether the cursor is visible or not.
pub fn set_cursor_visible(window: &mut Window, visible: bool) {
    window.cursor.visible = visible;
    window.cursor.grab_mode = match visible {
        true => CursorGrabMode::None,
        false => CursorGrabMode::Locked,
    };
}

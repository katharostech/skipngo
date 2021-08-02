use std::time::Duration;

use bevy_retrograde::{
    physics::heron::rapier_plugin::rapier2d::prelude::IntegrationParameters,
    prelude::heron::PhysicsSteps,
};

use super::*;

mod start_menu_ui;

//
// Game Loading and initialization systems
//

/// Wait for the game info to load and spawn the map
pub fn await_init(
    mut commands: Commands,
    game_info_assets: Res<Assets<GameInfo>>,
    asset_server: Res<AssetServer>,
    mut state: ResMut<State<GameState>>,
    mut ui_tree: ResMut<UiTree>,
    #[cfg(not(wasm))] mut windows: ResMut<Windows>,
    mut physics_params: ResMut<IntegrationParameters>,
) {
    debug!("Awaiting game info load...");
    let game_info: Handle<GameInfo> = asset_server.load_cached("default.game.yaml");

    // Spawn the map and camera once the game info loads
    if let Some(game_info) = game_info_assets.get(game_info) {
        debug!("Game info loaded: spawning camera and map");

        // Tweak the physics parameters
        *physics_params = IntegrationParameters {
            // Adjust for a "16 pixels equals 1 meter" scale
            erp: 0.1,
            allowed_linear_error: 0.05,
            prediction_distance: 1.,
            max_linear_correction: 3.,
            ..(*physics_params)
        };
        commands.insert_resource(PhysicsSteps::from_max_delta_time(Duration::from_secs_f64(
            1.0 / 24.,
        )));

        // Spawn the camera
        commands.spawn().insert_bundle(CameraBundle {
            camera: Camera {
                size: game_info.camera_size.clone(),
                ..Default::default()
            },
            ..Default::default()
        });

        // Update the window title
        #[cfg(not(wasm))]
        windows
            .get_primary_mut()
            .unwrap()
            .set_title(game_info.title.clone());
        #[cfg(wasm)]
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .set_title(&game_info.title);

        // Spawn the map
        commands.spawn().insert_bundle(LdtkMapBundle {
            map: asset_server.load_cached(game_info.map.as_str()),
            ..Default::default()
        });

        // Add the game info as a resource
        commands.insert_resource(game_info.clone());
        // Add the current level resource
        commands.insert_resource(CurrentLevel(
            game_info.splash_screen.background_level.clone(),
        ));

        // Set the UI tree to the start menu
        *ui_tree = UiTree(widget! {
            (start_menu_ui::start_menu)
        });

        // Transition to map loading state
        state.push(GameState::StartMenu).unwrap();
    }
}

pub struct StartMenuMusicHandle(pub Sound);
/// Position the camera on the start menu
pub fn setup_start_menu(
    mut completed: Local<bool>,
    mut cameras: Query<&mut Transform, With<Camera>>,
    mut maps_query: Query<&Handle<LdtkMap>>,
    current_level: Res<CurrentLevel>,
    mut map_layers: Query<(&LdtkMapLayer, &mut Visible)>,
    map_assets: Res<Assets<LdtkMap>>,
    game_info: Res<GameInfo>,
    asset_server: Res<AssetServer>,
    mut sound_controller: SoundController,
    mut commands: Commands,
    state: Res<State<GameState>>,
) {
    // If the game state has just changed, reset the completed flag
    if state.is_changed() && *completed {
        *completed = false;
        return;
    }

    // Run only once
    if *completed {
        return;
    }

    // Get our camera and map information
    let mut camera_transform = if let Ok(pos) = cameras.single_mut() {
        pos
    } else {
        return;
    };
    let map_handle = if let Ok(handle) = maps_query.single_mut() {
        handle
    } else {
        return;
    };
    let map = if let Some(map) = map_assets.get(map_handle) {
        map
    } else {
        return;
    };

    // Get the current level from the map
    let level = map
        .project
        .levels
        .iter()
        .find(|x| x.identifier == current_level.as_str())
        .unwrap();

    // Hide all other map layers
    let mut hid_layers = false;
    for (layer, mut visible) in map_layers.iter_mut() {
        if layer.level_identifier != current_level.as_str() {
            hid_layers = true;
            *visible = Visible(false);
        }
    }

    // Center the camera on the map level
    *camera_transform = Transform::from_xyz(
        level.world_x as f32 + level.px_wid as f32 / 2.,
        level.world_y as f32 + level.px_hei as f32 / 2.,
        0.,
    );

    let sound_data = asset_server.load_cached(game_info.splash_screen.music.as_str());
    let sound = sound_controller.create_sound(&sound_data);

    // Play music on loop
    sound_controller.play_sound_with_settings(
        sound,
        PlaySoundSettings::new().loop_start(LoopStart::Custom(0.0)),
    );

    commands.insert_resource(StartMenuMusicHandle(sound));

    // Mark completed so we don't run this system again
    if !*completed && hid_layers {
        *completed = true;
    }
}

pub fn spawn_player_and_setup_level(
    mut commands: Commands,
    map_query: Query<&Handle<LdtkMap>>,
    map_assets: Res<Assets<LdtkMap>>,
    mut state: ResMut<State<GameState>>,
    asset_server: Res<AssetServer>,
    game_info: Res<GameInfo>,
    current_level: Res<CurrentLevel>,
    mut sound_controller: SoundController,
    mut ui_tree: ResMut<UiTree>,
    start_menu_music_handle: Res<StartMenuMusicHandle>,
) {
    if let Ok(map_handle) = map_query.single() {
        if let Some(map) = map_assets.get(map_handle) {
            debug!("Map loaded: spawning player");

            // Stop the menu music
            sound_controller.stop_sound(start_menu_music_handle.0);

            let level = &map
                .project
                .levels
                .iter()
                .find(|x| x.identifier == **current_level)
                .unwrap();

            let entities_layer = level
                .layer_instances
                .as_ref()
                .unwrap()
                .iter()
                .find(|&x| !x.entity_instances.is_empty())
                .unwrap();

            let player_start = entities_layer
                .entity_instances
                .iter()
                .find(|x| {
                    x.__identifier == "SpawnPoint"
                        && x.field_instances
                            .iter()
                            .any(|x| x.__identifier == "name" && x.__value == "PlayerStart")
                })
                .unwrap();

            let character_handle: Handle<Character> =
                asset_server.load_cached(game_info.player_character.as_str());

            let character_image_handle =
                asset_server.load_cached(format!("{}#atlas", game_info.player_character).as_str());
            let character_spritesheet_handle = asset_server
                .load_cached(format!("{}#spritesheet", game_info.player_character).as_str());

            // Layers are 2 units away from each-other, so put the player at the top
            let player_z = level.layer_instances.as_ref().unwrap().len() as f32 * 2.0;

            // Spawn the player
            commands.spawn().insert_bundle(CharacterBundle {
                character: character_handle,
                sprite_bundle: SpriteBundle {
                    image: character_image_handle,
                    transform: Transform::from_xyz(
                        player_start.px[0] as f32 + level.world_x as f32,
                        player_start.px[1] as f32 + level.world_y as f32,
                        player_z,
                    ),
                    sprite: Sprite {
                        pixel_perfect: false,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                sprite_sheet: character_spritesheet_handle,
                ..Default::default()
            });

            // Get the level background music
            let background_music_field = level
                .field_instances
                .iter()
                .find(|x| x.__identifier == "music")
                .unwrap();

            // Play the music if it is set
            if let Some(music) = background_music_field.__value.as_str() {
                if music != "none" {
                    debug!("Starting level music");
                    let sound_data = asset_server.load_cached(music);
                    let sound = sound_controller.create_sound(&sound_data);

                    // Play music on loop
                    sound_controller.play_sound_with_settings(
                        sound,
                        PlaySoundSettings::new().loop_start(LoopStart::Custom(0.0)),
                    );

                    commands.insert_resource(CurrentLevelMusic { sound_data, sound });
                }
            }

            // Pre-load all other background music for the map
            for level in &map.project.levels {
                let background_music_field = level
                    .field_instances
                    .iter()
                    .find(|x| x.__identifier == "music")
                    .unwrap();

                if let Some(music) = background_music_field.__value.as_str() {
                    if music != "none" {
                        // Cache the music data
                        asset_server.load_cached::<SoundData, _>(music);
                    }
                }
            }

            // Remove the start menu
            *ui_tree = UiTree(widget! {
                ()
            });

            // Go to the running state
            debug!("Going into running state");
            state.push(GameState::Playing).unwrap();
        }
    }
}

use bevy_retrograde::{
    core::image::{DynamicImage, GenericImageView},
    prelude::rapier_plugin::rapier::prelude::IntegrationParameters,
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
        commands.insert_resource(PhysicsSteps::variable_timestep());

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

pub struct LdtkMapLayerFinishedLoadingCollisions;
/// Get any maps that have not had their tile collisions spawned yet and spawn them
pub fn update_map_collisions(
    mut commands: Commands,
    map_layers: Query<
        (Entity, &LdtkMapLayer, &Handle<Image>),
        Without<LdtkMapLayerFinishedLoadingCollisions>,
    >,
    image_assets: Res<Assets<Image>>,
) {
    // Spawn collision shapes for the collision layers
    for (layer_ent, map_layer, image_handle) in map_layers.iter() {
        // Skip non-collision layers
        if !map_layer
            .layer_instance
            .__identifier
            .to_lowercase()
            .contains("collision")
        {
            // Mark this layer as loaded so we don't check it again
            commands
                .entity(layer_ent)
                .insert(LdtkMapLayerFinishedLoadingCollisions);
            continue;
        }

        // Get the layer image
        let image = if let Some(image) = image_assets.get(image_handle) {
            image
        } else {
            continue;
        };

        // Get the tile size of the map
        let tile_size = map_layer.layer_instance.__grid_size as u32;

        let mut layer_commands = commands.entity(layer_ent);

        // For every tile grid
        for tile_x in 0u32..map_layer.layer_instance.__c_wid as u32 {
            for tile_y in 0u32..map_layer.layer_instance.__c_hei as u32 {
                // Get the tile image
                let tile_img = image
                    .view(tile_x * tile_size, tile_y * tile_size, tile_size, tile_size)
                    .to_image();

                // Try to generate a convex collision mesh from the tile
                let mesh = create_convex_collider(
                    DynamicImage::ImageRgba8(tile_img.clone()),
                    &TesselatedColliderConfig {
                        vertice_separation: 1.,
                        ..Default::default()
                    },
                );

                // If mesh generation was successful ( wouldn't be fore empty tiles, etc. )
                if let Some(mesh) = mesh {
                    // Spawn a collider as a child of the map layer
                    layer_commands.with_children(|layer| {
                        layer.spawn().insert_bundle((
                            mesh,
                            Transform::from_xyz(
                                (tile_x * tile_size + tile_size / 2) as f32,
                                (tile_y * tile_size + tile_size / 2) as f32,
                                0.,
                            ),
                            GlobalTransform::default(),
                        ));
                    });
                }
            }
        }

        layer_commands
            // Make layer a static body
            .insert(RigidBody::Static)
            // Mark as loaded
            .insert(LdtkMapLayerFinishedLoadingCollisions);
    }
}

pub struct LdtkMapEntrancesLoaded;
pub fn update_map_entrances(
    mut commands: Commands,
    maps: Query<(Entity, &Handle<LdtkMap>), Without<LdtkMapEntrancesLoaded>>,
    map_assets: Res<Assets<LdtkMap>>,
) {
    for (ent, map_handle) in maps.iter() {
        let map = if let Some(map) = map_assets.get(map_handle) {
            map
        } else {
            continue;
        };

        let mut map_commands = commands.entity(ent);

        for level in &map.project.levels {
            for layer in level
                .layer_instances
                .as_ref()
                .expect("Map has no layers")
                .iter()
                .filter(|x| x.__type == "Entities")
            {
                // Spawn collision sensors for the entrances
                for entrance in layer
                    .entity_instances
                    .iter()
                    .filter(|x| x.__identifier == "Entrance")
                {
                    map_commands.with_children(|map| {
                        map.spawn_bundle((
                            Entrance {
                                map_handle: map_handle.clone(),
                                level: level.identifier.clone(),
                                id: entrance
                                    .field_instances
                                    .iter()
                                    .find(|x| x.__identifier == "id")
                                    .expect("Could not find entrance `id` field")
                                    .__value
                                    .as_str()
                                    .expect("Entrance `id` field is not a string")
                                    .into(),
                                to_level: entrance
                                    .field_instances
                                    .iter()
                                    .find(|x| x.__identifier == "to")
                                    .expect("Could not find entrance `to` field")
                                    .__value
                                    .as_str()
                                    .expect("Entrance `to` field is not a string")
                                    .into(),
                                spawn_at: entrance
                                    .field_instances
                                    .iter()
                                    .find(|x| x.__identifier == "spawn_at")
                                    .expect("Could not find entrance `spawn_at` field")
                                    .__value
                                    .as_str()
                                    .expect("Entrance `spawn_at` field is not a string")
                                    .into(),
                            },
                            CollisionShape::Cuboid {
                                half_extends: Vec3::new(
                                    entrance.width as f32 / 2.0,
                                    entrance.height as f32 / 2.0,
                                    0.,
                                ),
                                border_radius: None,
                            },
                            RigidBody::Sensor,
                            Transform::from_xyz(
                                (level.world_x
                                    + layer.__px_total_offset_x
                                    + entrance.px[0]
                                    + entrance.width / 2) as f32,
                                (level.world_y
                                    + layer.__px_total_offset_y
                                    + entrance.px[1]
                                    + entrance.height / 2) as f32,
                                100.,
                            ),
                            GlobalTransform::default(),
                        ));
                    });
                }
            }
        }

        map_commands.insert(LdtkMapEntrancesLoaded);
    }
}
pub fn reload_changed_map_entrances(
    mut commands: Commands,
    maps: Query<(Entity, &Handle<LdtkMap>)>,
    entrances: Query<(Entity, &Entrance)>,
    mut events: EventReader<AssetEvent<LdtkMap>>,
) {
    for event in events.iter() {
        if let AssetEvent::Modified { handle } = event {
            // Remove the `LdtkMapEntrancesLoaded` flag from the map
            for (ent, map) in maps.iter() {
                if map == handle {
                    commands.entity(ent).remove::<LdtkMapEntrancesLoaded>();
                }
            }
            // Despawn all entrances for that map
            for (ent, entrance) in entrances.iter() {
                if &entrance.map_handle == handle {
                    commands.entity(ent).despawn();
                }
            }
        }
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
) {
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

use std::path::{Path, PathBuf};

use bevy::utils::HashMap;
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

/// Component that caches map tileset collisions based on the tilesetid and the tile id
pub struct LdtkMapTilesetTileCollisions(pub HashMap<(i32, i32), CollisionShape>);
/// Component used to mark map collision shapes
pub struct LdtkMapTileCollisionShape;
/// Component used to mark the map as having had its collisions loaded
pub struct LdtkMapTileCollisionsLoaded;
/// Get any maps that have not had their tile collisions spawned yet and spawn them
pub fn spawn_map_collisions(
    mut commands: Commands,
    maps: Query<
        (
            Entity,
            &Handle<LdtkMap>,
            Option<&LdtkMapTilesetTileCollisions>,
        ),
        Without<LdtkMapTileCollisionsLoaded>,
    >,
    map_assets: Res<Assets<LdtkMap>>,
    image_assets: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
    game_info: Option<Res<GameInfo>>,
) {
    // Load game info or wait until it is loaded
    let game_info = if let Some(game_info) = game_info {
        game_info
    } else {
        return;
    };

    'map_load: for (map_ent, map_handle, tileset_tile_collisions_component) in maps.iter() {
        // Get map commands
        let mut map_commands = commands.entity(map_ent);

        // Get the loaded map
        let map = if let Some(map) = map_assets.get(map_handle) {
            map
        } else {
            continue;
        };

        // Load all tilesets and skip if any are missing
        let tileset_images = if let Some(tile_sets) = map
            .tile_sets
            .iter()
            .map(|(name, handle)| {
                if let Some(image) = image_assets.get(handle) {
                    Some((name, image))
                } else {
                    None
                }
            })
            .collect::<Option<HashMap<_, _>>>()
        {
            tile_sets
        } else {
            continue;
        };

        // Tilemap tile collisions indexed by (tileset_uid, tile_id)
        let mut tileset_tile_collisions = tileset_tile_collisions_component
            .map(|x| x.0.clone())
            .unwrap_or_default();

        // Generate collision shapes for all of the tiles in each tileset
        for tileset_def in &map.project.defs.tilesets {
            // For all tiles with custom data
            for tile_data in &tileset_def.custom_data {
                // Get tile ID and custom data
                let tile_id = tile_data
                    .get("tileId")
                    .expect("Tile data missing `tileId` field")
                    .as_i64()
                    .expect("Tile `tileId` field not an int") as i32;
                let data = tile_data
                    .get("data")
                    .expect("Tile data missing `data` field")
                    .as_str()
                    .expect("Tile `data` field not a string");

                // If we already have the collision calculated for this tile, skip it
                if tileset_tile_collisions.contains_key(&(tileset_def.uid, tile_id)) {
                    continue;
                }

                // Parse tile metadata as YAML
                let tileset_tile_metadata: TilesetTileMetadata = match serde_yaml::from_str(data) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        warn!(
                            %error,
                            %tile_id,
                            tileset_id=%tileset_def.identifier,
                            "Could not parse tileset tile metadata, ignoring"
                        );
                        continue;
                    }
                };

                // Get the image for this tileset
                let tileset_image = *tileset_images
                    .get(&tileset_def.identifier)
                    .expect("Tileset image not loaded");

                // Helper for generating alpha-based collision shapes
                macro_rules! create_alpha_based_collision {
                    ($image:ident) => {
                        {
                            // Get the tile pixel x and y positions from the tile ID
                            let tile_grid_y = tile_id / tileset_def.__c_wid;
                            let tile_grid_x = tile_id - (tile_grid_y * tileset_def.__c_wid);
                            let tile_x = tile_grid_x * tileset_def.tile_grid_size;
                            let tile_y = tile_grid_y * tileset_def.tile_grid_size;

                            // Get the portion of the tilemap image for this tile
                            let tile_image = $image.view(
                                tile_x as u32,
                                tile_y as u32,
                                tileset_def.tile_grid_size as u32,
                                tileset_def.tile_grid_size as u32,
                            );

                            // Generate a collision shape from the tile image
                            let collision_shape = if let Some(collision) =
                                physics::create_convex_collider(
                                    DynamicImage::ImageRgba8(tile_image.to_image()),
                                    &TesselatedColliderConfig {
                                        vertice_separation: 1.,
                                        ..Default::default()
                                    },
                            ) {
                                collision
                            } else {
                                warn!(
                                    %tile_id,
                                    tileset_id=%tileset_def.identifier,
                                    "Could not create collision shape for tile"
                                );
                                continue;
                            };

                            collision_shape
                        }
                    }
                }

                match tileset_tile_metadata.collision {
                    // Create a cuboid collision for this block
                    TilesetTileCollisionMode::Full => {
                        tileset_tile_collisions.insert(
                            (tileset_def.uid, tile_id),
                            CollisionShape::Cuboid {
                                half_extends: Vec3::new(
                                    tileset_def.tile_grid_size as f32 / 2.0,
                                    tileset_def.tile_grid_size as f32 / 2.0,
                                    0.,
                                ),
                                border_radius: None,
                            },
                        );
                    }
                    // Spawn a tesselated collision shape generated from
                    TilesetTileCollisionMode::FromAlpha => {
                        let collision_shape = create_alpha_based_collision!(tileset_image);

                        // Add the collision to the list
                        tileset_tile_collisions.insert((tileset_def.uid, tile_id), collision_shape);
                    }
                    // Create a collision from the alpha of a corresponding tile in a reference tilesheet
                    TilesetTileCollisionMode::FromAlphaReference {
                        tileset: tileset_relative_path,
                    } => {
                        // Load the reference tileset image
                        let map_path = PathBuf::from(game_info.map.clone());
                        let tileset_reference_handle: Handle<Image> = asset_server.load_cached(
                            map_path
                                .parent()
                                .unwrap_or_else(|| Path::new("./"))
                                .join(tileset_relative_path),
                        );

                        // Get the reference tilesheet image
                        let tileset_reference_image = if let Some(tileset_image) =
                            image_assets.get(tileset_reference_handle)
                        {
                            tileset_image
                        // If the tilesheet image cannot be loaded
                        } else {
                            // Store the collisions we have currently and wait to try again next
                            // frame
                            map_commands
                                .insert(LdtkMapTilesetTileCollisions(tileset_tile_collisions));
                            continue 'map_load;
                        };

                        let collision_shape =
                            create_alpha_based_collision!(tileset_reference_image);

                        // Add the collision to the list
                        tileset_tile_collisions.insert((tileset_def.uid, tile_id), collision_shape);
                    }
                    // Don't do anything for empty collisions
                    TilesetTileCollisionMode::None => (),
                }
            }
        }

        // For every level in the map
        for level in &map.project.levels {
            // Get the level offset
            let level_offset = Vec3::new(level.world_x as f32, level.world_y as f32, 0.);

            // For every layer in the level
            for layer in level
                .layer_instances
                .as_ref()
                .expect("Map level has no layers")
                .iter()
            {
                // Get the layer offset
                let layer_offset = level_offset
                    + Vec3::new(
                        layer.__px_total_offset_x as f32,
                        layer.__px_total_offset_y as f32,
                        0.,
                    );

                // Get layer tile size
                let tile_size = layer.__grid_size as f32;

                // Get the NoCollision hlper layer for this layer if it exists
                let no_collision_layer = level
                    .layer_instances
                    .as_ref()
                    .expect("Level has no layers")
                    .iter()
                    .find(|x| x.__identifier == format!("{}NoCollision", layer.__identifier));

                // Get the layer tileset uid, or skip the layer if it doesn't have a tileset
                let tileset_uid = if let Some(uid) = layer.__tileset_def_uid {
                    uid
                } else {
                    continue;
                };

                // For every tile in the layer
                for tile in layer.grid_tiles.iter().chain(layer.auto_layer_tiles.iter()) {
                    // Skip this tile if it has a representative in the NoCollision layer
                    if let Some(no_collision_layer) = no_collision_layer {
                        let tile_index = (tile.px[0] / layer.__grid_size)
                            + (tile.px[1] / layer.__grid_size * layer.__c_wid);

                        // If the NoCollision layer has a tile in a position corresponding to this
                        // tile
                        if no_collision_layer.int_grid_csv[tile_index as usize] != 0 {
                            // Skip the tile
                            continue;
                        }
                    }

                    // Get the tile position
                    let tile_pos =
                        layer_offset + Vec3::new(tile.px[0] as f32, tile.px[1] as f32, 0.);

                    // Offset the tile position to get the center of the tile
                    let half_tile_size = Vec3::new(tile_size / 2.0, tile_size / 2.0, 0.);

                    // Spawn a collision shape for this tile if one exists
                    if let Some(collision_shape) =
                        tileset_tile_collisions.get(&(tileset_uid, tile.t))
                    {
                        map_commands.with_children(|map| {
                            map.spawn_bundle((
                                LdtkMapTileCollisionShape,
                                collision_shape.clone(),
                                Transform::from_translation(tile_pos + half_tile_size),
                                GlobalTransform::default(),
                            ));
                        });
                    }
                }
            }
        }

        map_commands
            // Mark map collsions as loaded
            .insert(LdtkMapTileCollisionsLoaded)
            // Make the map a static body
            .insert(RigidBody::Static);
    }
}
pub fn hot_reload_map_collisions(
    mut commands: Commands,
    maps: Query<(Entity, &Handle<LdtkMap>)>,
    tile_collisions: Query<(Entity, &Parent), With<LdtkMapTileCollisionShape>>,
    mut events: EventReader<AssetEvent<LdtkMap>>,
) {
    for event in events.iter() {
        if let AssetEvent::Modified { handle } = event {
            // For every map
            for (map_ent, map) in maps.iter() {
                // If this map's handle has been updated
                if map == handle {
                    // Unmark the map collisions as loaded and emove cached tileset collisions
                    commands
                        .entity(map_ent)
                        .remove::<LdtkMapTileCollisionsLoaded>()
                        .remove::<LdtkMapTilesetTileCollisions>();

                    // For every tile collision
                    for (tile_ent, parent) in tile_collisions.iter() {
                        // If this tile is a child of the map that changed
                        if parent.0 == map_ent {
                            // Despawn it
                            commands.entity(tile_ent).despawn();
                        }
                    }
                }
            }
        }
    }
}

pub struct LdtkMapEntrancesLoaded;
pub fn spawn_map_entrances(
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
            let level_offset = Vec3::new(level.world_x as f32, level.world_y as f32, 0.);

            for layer in level
                .layer_instances
                .as_ref()
                .expect("Map has no layers")
                .iter()
                .filter(|x| x.__type == "Entities")
            {
                let layer_offset = Vec3::new(
                    layer.__px_total_offset_x as f32,
                    layer.__px_total_offset_y as f32,
                    0.,
                );
                // Spawn collision sensors for the entrances
                for entrance in layer
                    .entity_instances
                    .iter()
                    .filter(|x| x.__identifier == "Entrance")
                {
                    let entrance_position = Vec3::new(
                        entrance.px[0] as f32 + layer.__grid_size as f32 / 2.,
                        entrance.px[1] as f32 + layer.__grid_size as f32 / 2.,
                        0.,
                    );

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
                                    // Shrink the entrance slightly by dividing by 2.2 to prevent
                                    // the collision from being hit past walls.
                                    entrance.width as f32 / 2.2,
                                    entrance.height as f32 / 2.2,
                                    0.,
                                ),
                                border_radius: None,
                            },
                            RigidBody::Sensor,
                            Transform::from_translation(
                                level_offset + layer_offset + entrance_position,
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
pub fn hot_reload_map_entrances(
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
            // Despawn all entrances for the modified map
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

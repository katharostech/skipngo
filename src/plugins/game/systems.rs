use bevy::{core::FixedTimestep, ecs::schedule::ShouldRun, prelude::*, utils::HashSet};
use bevy_retro::*;

use super::*;

/// The game states
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameState {
    /// The game is loading initial game data and spawning the initial items
    LoadingGameInfo,
    /// The game is loading the map and spawning the player
    LoadingMap,
    /// The game is running!
    Running,
}

// System labels
#[derive(Clone, Debug, PartialEq, Eq, Hash, SystemLabel)]
pub enum GameStage {
    PreUpdate,
    Update,
    PostUpdate,
}

pub fn add_systems(app: &mut AppBuilder) {
    debug!("Configuring game systems");
    app
        // // Set the inital game state
        .add_state(GameState::LoadingGameInfo)
        // Game init
        .add_system_set(
            SystemSet::on_update(GameState::LoadingGameInfo).with_system(await_init.system()),
        )
        // Player spawn
        .add_system_set(
            SystemSet::on_update(GameState::LoadingMap).with_system(spawn_player.system()),
        )
        // Add pre-update systems
        .add_system_set(
            SystemSet::on_update(GameState::Running)
                .label(GameStage::PreUpdate)
                .with_system(touch_control_input.system())
                .with_system(keyboard_control_input.system())
                .with_system(finish_spawning_character.system()),
        )
        // Add update systems
        .add_system_set(
            SystemSet::new()
                .after(GameStage::PreUpdate)
                .label(GameStage::Update)
                .with_run_criteria(
                    FixedTimestep::step(0.012).chain(
                        // Workaround: https://github.com/bevyengine/bevy/issues/1839
                        (|In(input): In<ShouldRun>, state: Res<State<GameState>>| {
                            if state.current() == &GameState::Running {
                                input
                            } else {
                                ShouldRun::No
                            }
                        })
                        .system(),
                    ),
                ) // Run with fixed timestep
                .with_system(control_character.system())
                .with_system(animate_sprites.system())
                .with_system(change_level.system()),
        )
        // Add post-update sytems
        .add_system_set(
            SystemSet::on_update(GameState::Running)
                .after(GameStage::Update)
                .label(GameStage::PostUpdate)
                .with_system(camera_follow_system.system()),
        );
}

//
// Game Loading and initialization systems
//

/// Wait for the game info to load and spawn the map
pub fn await_init(
    mut commands: Commands,
    game_info_assets: Res<Assets<GameInfo>>,
    asset_server: Res<AssetServer>,
    mut state: ResMut<State<GameState>>,
    engine_config: Res<EngineConfig>,
    #[cfg(not(wasm))] mut windows: ResMut<Windows>,
) {
    debug!("Awaiting game info load...");
    let game_info: Handle<GameInfo> = asset_server.load("default.game.yaml");

    // Spawn the map once the game info loads
    if let Some(game_info) = game_info_assets.get(game_info) {
        debug!("Game info loaded, spawning camera and map");

        // Spawn the camera
        commands.spawn().insert_bundle(CameraBundle {
            camera: Camera {
                size: game_info.camera_size.clone(),
                custom_shader: if engine_config.enable_crt {
                    Some(CrtShader::default().get_shader())
                } else {
                    None
                },
                pixel_aspect_ratio: engine_config.pixel_aspect_ratio,
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
            map: asset_server.load(game_info.map.as_str()),
            ..Default::default()
        });

        // Add the game info as a resource
        commands.insert_resource(game_info.clone());
        // Add the current level resource
        commands.insert_resource(CurrentLevel(game_info.starting_level.clone()));

        // Transition to map loading state
        state.push(GameState::LoadingMap).unwrap();
    }
}

pub fn spawn_player(
    mut commands: Commands,
    map_query: Query<&Handle<LdtkMap>>,
    map_assets: Res<Assets<LdtkMap>>,
    mut state: ResMut<State<GameState>>,
    asset_server: Res<AssetServer>,
    game_info: Res<GameInfo>,
    current_level: Res<CurrentLevel>,
) {
    for map_handle in map_query.iter() {
        if let Some(map) = map_assets.get(map_handle) {
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
                asset_server.load(game_info.player_character.as_str());

            let character_image_handle =
                asset_server.load(format!("{}#atlas", game_info.player_character).as_str());
            let character_spritesheet_handle =
                asset_server.load(format!("{}#spritesheet", game_info.player_character).as_str());

            // Layers are 2 units away from each-other, so put the player at the top
            let player_z = level.layer_instances.as_ref().unwrap().len() as i32 * 2;

            commands.spawn().insert_bundle(CharacterBundle {
                character: character_handle,
                sprite_bundle: SpriteBundle {
                    image: character_image_handle,
                    position: Position::new(
                        player_start.px[0] + level.world_x,
                        player_start.px[1] + level.world_y,
                        player_z,
                    ),
                    ..Default::default()
                },
                sprite_sheet: character_spritesheet_handle,
                ..Default::default()
            });

            // Go to the running state
            state.push(GameState::Running).unwrap();
        }
    }
}

//
// Game play systems
//

pub fn touch_control_input(
    mut tracked_touch: Local<Option<u64>>,
    mut touch_events: EventReader<TouchInput>,
    mut control_events: EventWriter<ControlEvent>,
    touches: Res<Touches>,
) {
    for touch in touch_events.iter() {
        if let Some(&id) = tracked_touch.as_ref() {
            if touch.id == id {
                match touch.phase {
                    bevy::input::touch::TouchPhase::Ended
                    | bevy::input::touch::TouchPhase::Cancelled => *tracked_touch = None,
                    _ => (),
                }
            }
        } else {
            *tracked_touch = Some(touch.id);
        }
    }

    if let Some(&id) = tracked_touch.as_ref() {
        if let Some(touch) = touches.get_pressed(id) {
            // Get the difference in the positions
            let diff = touch.position() - touch.start_position();

            if diff.x > 0. {
                control_events.send(ControlEvent::MoveRight);
            }

            if diff.x < 0. {
                control_events.send(ControlEvent::MoveLeft);
            }

            if diff.y > 0. {
                control_events.send(ControlEvent::MoveDown);
            }

            if diff.y < 0. {
                control_events.send(ControlEvent::MoveUp);
            }
        } else {
            *tracked_touch = None;
        }
    }
}

pub fn keyboard_control_input(
    mut control_events: EventWriter<ControlEvent>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.pressed(KeyCode::Left) {
        control_events.send(ControlEvent::MoveLeft);
    }

    if keyboard_input.pressed(KeyCode::Right) {
        control_events.send(ControlEvent::MoveRight);
    }

    if keyboard_input.pressed(KeyCode::Up) {
        control_events.send(ControlEvent::MoveUp);
    }

    if keyboard_input.pressed(KeyCode::Down) {
        control_events.send(ControlEvent::MoveDown);
    }
}

/// Marker component for loaded characters
pub struct CharacterLoaded;

/// Add the sprite image and sprite sheet handles to the spawned character
pub fn finish_spawning_character(
    mut commands: Commands,
    mut characters: Query<
        (
            Entity,
            &Handle<Character>,
            &mut Handle<Image>,
            &mut Handle<SpriteSheet>,
        ),
        Without<CharacterLoaded>,
    >,
    character_assets: Res<Assets<Character>>,
) {
    for (ent, character_handle, mut image_handle, mut sprite_sheet_handle) in characters.iter_mut()
    {
        if let Some(character) = character_assets.get(character_handle) {
            *image_handle = character.sprite_image.clone();
            *sprite_sheet_handle = character.sprite_sheet.clone();
            commands.entity(ent).insert(CharacterLoaded);
        }
    }
}

/// Walk the character in response to input
pub fn control_character(
    mut world_positions: WorldPositionsQuery,
    mut characters: Query<
        (Entity, &Handle<Character>, &mut CharacterState, &Sprite),
        With<Handle<Character>>,
    >,
    map_layers: Query<(Entity, &LdtkMapLayer, &Handle<Image>, &Sprite)>,
    character_assets: Res<Assets<Character>>,
    mut scene_graph: ResMut<SceneGraph>,
    image_assets: Res<Assets<Image>>,
    mut control_events: EventReader<ControlEvent>,
) {
    // Synchronize world positions before checking for collisions
    world_positions.sync_world_positions(&mut scene_graph);

    // Loop through characters
    for (character_ent, character_handle, mut character_state, character_sprite) in
        characters.iter_mut()
    {
        let character = if let Some(character) = character_assets.get(character_handle) {
            character
        } else {
            continue;
        };
        let character_collision = if let Some(image) = image_assets.get(&character.collision_shape)
        {
            image
        } else {
            continue;
        };

        let mut movement = IVec3::default();

        // Determine movement direction
        let mut directions = HashSet::default();
        for control_event in control_events.iter() {
            if directions.insert(control_event) {
                match control_event {
                    ControlEvent::MoveUp => movement += IVec3::new(0, -1, 0),
                    ControlEvent::MoveDown => movement += IVec3::new(0, 1, 0),
                    ControlEvent::MoveLeft => movement += IVec3::new(-1, 0, 0),
                    ControlEvent::MoveRight => movement += IVec3::new(1, 0, 0),
                }
            }
        }

        // Determine animation and direction
        let new_action;
        let mut new_direction = character_state.direction;

        if movement.x == 0 && movement.y == 0 {
            new_action = CharacterStateAction::Idle;
        } else {
            new_action = CharacterStateAction::Walk;

            if movement.y.abs() > 0 && movement.x.abs() > 0 {
                // We are moving diagnally, so the new direction should be the same as the
                // previous direction and we don't do anything.
            } else if movement.y > 0 {
                new_direction = CharacterStateDirection::Down;
            } else if movement.y < 0 {
                new_direction = CharacterStateDirection::Up;
            } else if movement.x > 0 {
                new_direction = CharacterStateDirection::Right;
            } else if movement.x < 0 {
                new_direction = CharacterStateDirection::Left;
            }
        }

        // Reset character animation frame if direction or action changes
        if new_direction != character_state.direction || new_action != character_state.action {
            character_state.tileset_index = 0;
            character_state.animation_frame = 0;
        }
        // Update character action
        if new_action != character_state.action {
            character_state.action = new_action;
        }
        // Update character direction
        if new_direction != character_state.direction {
            character_state.direction = new_direction;
        }

        // Check for collisions with map collision layers
        for (layer_ent, layer, layer_image, layer_sprite) in map_layers.iter() {
            // Skip non-collision layers
            if !layer
                .layer_instance
                .__identifier
                .to_lowercase()
                .contains("collision")
            {
                continue;
            }

            // Get the layer image
            if let Some(layer_image) = image_assets.get(layer_image) {
                // Get the world position of the player
                let base_character_world_position = **world_positions
                    .get_world_position_mut(character_ent)
                    .unwrap();

                // Create the collider info for the layer image
                let layer_collider = PixelColliderInfo {
                    image: layer_image,
                    world_position: &world_positions.get_world_position_mut(layer_ent).unwrap(),
                    sprite: layer_sprite,
                    sprite_sheet: None,
                };

                // Create the character collider information
                let collides = |movement| {
                    let character_collider = PixelColliderInfo {
                        image: character_collision,
                        // Add our movement vector to the world position
                        world_position: &(base_character_world_position + movement),
                        sprite: character_sprite,
                        sprite_sheet: None,
                    };
                    pixels_collide_with_pixels(character_collider, layer_collider)
                };

                // Perform ritual to check for collisions ( in a closure to make it easy to return early )
                let has_collided = (|| {
                    // If our current movement would cause a collision
                    if collides(movement) {
                        // Try setting x movement to nothing and check again
                        if movement.x != 0 {
                            let mut new_movement = movement;
                            new_movement.x = 0;

                            if !collides(new_movement) {
                                *movement = *new_movement;
                                return false;
                            }
                        }

                        // Try setting y movement to nothing and check again
                        if movement.y != 0 {
                            let mut new_movement = movement;
                            new_movement.y = 0;

                            if !collides(new_movement) {
                                *movement = *new_movement;
                                return false;
                            }
                        }

                        // If we are still colliding, just set movement to nothing and break out of this loop
                        *movement = *IVec3::ZERO;

                        true

                    // If movement would not cause a collision just return false
                    } else {
                        false
                    }
                })();

                if has_collided {
                    break;
                }
            }
        }

        // Make sure moving diagonal does not make us go faster
        if movement.x != 0 && movement.y != 0 && character_state.animation_frame % 2 == 0 {
            movement.y = 0;
            movement.x = 0;
        }

        // Move the player
        let mut pos = world_positions
            .get_local_position_mut(character_ent)
            .unwrap();
        **pos += movement;
    }
}

/// Play the character's sprite animation
pub fn animate_sprites(
    characters: Res<Assets<Character>>,
    mut query: Query<(
        &Handle<SpriteSheet>,
        &mut Sprite,
        &mut CharacterState,
        &Handle<Character>,
    )>,
    mut sprite_sheet_assets: ResMut<Assets<SpriteSheet>>,
) {
    for (sprite_sheet, mut sprite, mut state, character_handle) in query.iter_mut() {
        if state.animation_frame % 10 == 0 {
            state.animation_frame = 0;

            if let Some(sprite_sheet) = sprite_sheet_assets.get_mut(sprite_sheet) {
                let character = characters.get(character_handle).unwrap();

                let action = match state.action {
                    CharacterStateAction::Walk => &character.actions.walk,
                    CharacterStateAction::Idle => &character.actions.idle,
                };

                let direction = match state.direction {
                    CharacterStateDirection::Up => &action.animations.up,
                    CharacterStateDirection::Down => &action.animations.down,
                    CharacterStateDirection::Left => &action.animations.left,
                    CharacterStateDirection::Right => &action.animations.right,
                };

                if direction.flip {
                    sprite.flip_x = true;
                } else {
                    sprite.flip_x = false;
                }

                let idx = direction.frames[state.tileset_index as usize % direction.frames.len()];

                sprite_sheet.tile_index = idx;

                state.tileset_index = state.tileset_index.wrapping_add(1);
            }
        }

        state.animation_frame = state.animation_frame.wrapping_add(1);
    }
}

// Make the camera follow the character
pub fn camera_follow_system(
    mut cameras: Query<(&Camera, &mut Position)>,
    characters: Query<&Position, (With<Handle<Character>>, Without<Camera>)>,
    mut map_layers: Query<
        (&mut LdtkMapLayer, &mut Visible, &Handle<Image>, &Position),
        Without<Camera>,
    >,
    windows: Res<Windows>,
    image_assets: Res<Assets<Image>>,
    current_level: Option<Res<CurrentLevel>>,
) {
    let current_level = if let Some(level) = current_level {
        level
    } else {
        return;
    };

    if let Some((camera, mut camera_pos)) = cameras.iter_mut().next() {
        // Start by making the camera stick to the player
        if let Some(character_pos) = characters.iter().next() {
            camera_pos.x = character_pos.x;
            camera_pos.y = character_pos.y;
        }

        // If there is a spawned map layer we can find, we want to make sure the camera doesn't show
        // outside the edges of the map. ( we don't really care which layer because they should all
        // be the same size )
        let mut has_constrained_camera = false;
        for (layer, mut layer_visible, layer_image_handle, layer_pos) in map_layers.iter_mut() {
            // If this layer is a part of the current level
            if layer.level_identifier == **current_level {
                // Make sure the layer is visible ( if it's supposed to be )
                if !**layer_visible {
                    **layer_visible = layer.layer_instance.visible;
                }

                if !has_constrained_camera {
                    // Get the layer image
                    let layer_image = if let Some(image) = image_assets.get(layer_image_handle) {
                        image
                    } else {
                        return;
                    };

                    // Get the layer bounds
                    let (layer_width, layer_height) = layer_image.dimensions();
                    let layer_min_x = layer_pos.x;
                    let layer_max_x = layer_pos.x + layer_width as i32;
                    let layer_min_y = layer_pos.y;
                    let layer_max_y = layer_pos.y + layer_height as i32;

                    // Get the camera target size
                    let camera_size = camera.get_target_size(windows.get_primary().unwrap());
                    let camera_min_x = camera_pos.x - camera_size.x as i32 / 2;
                    let camera_max_x =
                        (camera_pos.x - camera_size.x as i32 / 2) + camera_size.x as i32;
                    let camera_min_y = camera_pos.y - camera_size.y as i32 / 2;
                    let camera_max_y =
                        (camera_pos.y - camera_size.y as i32 / 2) + camera_size.y as i32;

                    // Constrain the camera to the layer size
                    if layer_width > camera_size.x {
                        if layer_min_x > camera_min_x {
                            camera_pos.x += layer_min_x - camera_min_x;
                        }

                        if layer_max_x < camera_max_x {
                            camera_pos.x -= camera_max_x - layer_max_x;
                        }
                    }

                    if layer_height > camera_size.y {
                        if layer_min_y > camera_min_y {
                            camera_pos.y += layer_min_y - camera_min_y;
                        }

                        if layer_max_y < camera_max_y {
                            camera_pos.y -= camera_max_y - layer_max_y;
                        }
                    }

                    has_constrained_camera = true;
                }

            // If the layer is not a part of the current level
            } else {
                // Make sure it is invisible
                if **layer_visible {
                    **layer_visible = false;
                }
            }
        }
    }
}

pub fn change_level(
    mut cameras: Query<&mut Camera>,
    mut characters: Query<(Entity, &Handle<Character>, &Sprite)>,
    mut world_positions: WorldPositionsQuery,
    maps: Query<&Handle<LdtkMap>>,
    map_assets: Res<Assets<LdtkMap>>,
    mut scene_graph: ResMut<SceneGraph>,
    image_assets: Res<Assets<Image>>,
    character_assets: Res<Assets<Character>>,
    mut current_level: ResMut<CurrentLevel>,
) {
    // Synchronize world positions before checking for collisions
    world_positions.sync_world_positions(&mut scene_graph);

    // Get the map
    let map_handle = if let Some(map) = maps.iter().next() {
        map
    } else {
        return;
    };
    let map = if let Some(map) = map_assets.get(map_handle) {
        map
    } else {
        return;
    };

    // Get the current map level
    let level = map
        .project
        .levels
        .iter()
        .find(|x| x.identifier == **current_level)
        .unwrap();

    // Loop through the characters
    for (character_ent, character_handle, character_sprite) in characters.iter_mut() {
        let character = if let Some(character) = character_assets.get(character_handle) {
            character
        } else {
            continue;
        };
        let character_collision = if let Some(image) = image_assets.get(&character.collision_shape)
        {
            image
        } else {
            continue;
        };

        // For every entity layer in the level
        for layer in level
            .layer_instances
            .as_ref()
            .unwrap()
            .iter()
            .filter(|x| x.__type == "Entities")
        {
            // For every entrance entity
            for entrance in layer
                .entity_instances
                .iter()
                .filter(|x| x.__identifier == "Entrance")
            {
                // Get the pixel collider for the character
                let character_collider = PixelColliderInfo {
                    image: character_collision,
                    // Add our movement vector to the world position
                    world_position: &world_positions
                        .get_world_position_mut(character_ent)
                        .unwrap(),
                    sprite: character_sprite,
                    sprite_sheet: None,
                };

                // Get the bounding box for the entrance
                let entrance_bounds = BoundingBox {
                    min: IVec2::new(
                        entrance.px[0] + level.world_x,
                        entrance.px[1] + level.world_y,
                    ),
                    max: IVec2::new(
                        entrance.px[0] + level.world_x + entrance.width,
                        entrance.px[1] + level.world_y + entrance.height,
                    ),
                };

                // If we have collided with the entrance
                if pixels_collide_with_bounding_box(character_collider, entrance_bounds) {
                    // Figure out where to teleport to
                    let to_level_id = entrance
                        .field_instances
                        .iter()
                        .find(|x| x.__identifier == "to")
                        .unwrap()
                        .__value
                        .as_str()
                        .unwrap();
                    let to_spawn_point = entrance
                        .field_instances
                        .iter()
                        .find(|x| x.__identifier == "spawn_at")
                        .unwrap()
                        .__value
                        .as_str()
                        .unwrap();

                    // Get the level that we will be teleporting to
                    let to_level = map
                        .project
                        .levels
                        .iter()
                        .find(|x| x.identifier == to_level_id)
                        .unwrap();

                    // Get the spawn point we will be teleporting to
                    let spawn_point = to_level
                        .layer_instances
                        .as_ref()
                        .unwrap()
                        .iter()
                        .find_map(|x| {
                            x.entity_instances.iter().find(|x| {
                                x.__identifier == "SpawnPoint"
                                    && x.field_instances.iter().any(|x| {
                                        x.__identifier == "name" && x.__value == to_spawn_point
                                    })
                            })
                        })
                        .unwrap();

                    // Set the current level to the new level
                    *current_level = CurrentLevel(to_level_id.into());

                    // Set the camera background to the level background color
                    for mut camera in cameras.iter_mut() {
                        let decoded = hex::decode(
                            to_level
                                .bg_color
                                .as_ref()
                                .unwrap_or(&map.project.default_level_bg_color)
                                .strip_prefix("#")
                                .expect("Invalid background color"),
                        )
                        .expect("Invalid background color");

                        camera.background_color =
                            Color::from_rgba8(decoded[0], decoded[1], decoded[2], 1);
                    }

                    // Get the character's position
                    let mut character_pos = world_positions
                        .get_local_position_mut(character_ent)
                        .unwrap();

                    *character_pos = Position::new(
                        to_level.world_x + spawn_point.px[0],
                        to_level.world_y + spawn_point.px[1],
                        level.layer_instances.as_ref().unwrap().len() as i32 * 2,
                    );
                }
            }
        }
    }
}

use bevy::{
    core::FixedTimestep,
    ecs::{component::ComponentDescriptor, schedule::ShouldRun},
    prelude::*,
    utils::HashSet,
};
use bevy_retro::{prelude::*, ui::raui::prelude::widget};
use kira::parameter::tween::Tween;

use super::*;

/// The game states
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameState {
    /// The game is loading initial game data, spawning the map, and displaying the start menu
    Init,
    /// The game is showing the start menu
    StartMenu,
    /// The game is loading the map and spawning the player
    LoadingGame,
    /// The game is playing the main game
    Playing,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, StageLabel)]
enum GameStage {
    Init,
    StartMenu,
    LoadingGame,
    Playing,
}

#[derive(Clone, Hash, PartialEq, Eq, Debug, SystemLabel, AmbiguitySetLabel)]
struct InputLabel;
#[derive(Clone, Hash, PartialEq, Eq, Debug, SystemLabel, AmbiguitySetLabel)]
struct CharacterControlLabel;
#[derive(Clone, Hash, PartialEq, Eq, Debug, SystemLabel, AmbiguitySetLabel)]
struct CameraFollowLabel;
#[derive(Clone, Hash, PartialEq, Eq, Debug, SystemLabel, AmbiguitySetLabel)]
struct AnimateSpritesLabel;

pub fn add_systems(app: &mut AppBuilder) {
    debug!("Configuring game systems");
    app
        // Use sparse storage for marker component
        .register_component(ComponentDescriptor::new::<CharacterLoaded>(
            bevy::ecs::component::StorageType::SparseSet,
        ))
        // Set the inital game state
        .add_state(GameState::Init)
        // Loading initial game data
        .add_stage_after(
            CoreStage::Update,
            GameStage::Init,
            SystemStage::parallel().with_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        (|state: Res<State<GameState>>| {
                            if state.current() == &GameState::Init {
                                ShouldRun::Yes
                            } else {
                                ShouldRun::No
                            }
                        })
                        .system(),
                    )
                    .with_system(await_init.system()),
            ),
        )
        // Showing the start menu
        .add_stage_after(
            GameStage::Init,
            GameStage::StartMenu,
            SystemStage::parallel().with_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        (|state: Res<State<GameState>>| {
                            if state.current() == &GameState::StartMenu {
                                ShouldRun::Yes
                            } else {
                                ShouldRun::No
                            }
                        })
                        .system(),
                    )
                    .with_system(setup_start_menu.system()),
            ),
        )
        // Spawning player and loading game level
        .add_stage_after(
            GameStage::Init,
            GameStage::LoadingGame,
            SystemStage::parallel().with_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        (|state: Res<State<GameState>>| {
                            if state.current() == &GameState::LoadingGame {
                                ShouldRun::Yes
                            } else {
                                ShouldRun::No
                            }
                        })
                        .system(),
                    )
                    .with_system(spawn_player_and_setup_level.system()),
            ),
        )
        // Playing the game
        .add_stage_after(
            GameStage::LoadingGame,
            GameStage::Playing,
            SystemStage::parallel().with_system_set(
                SystemSet::new()
                    .with_run_criteria(
                        FixedTimestep::step(0.012).chain(
                            // Workaround: https://github.com/bevyengine/bevy/issues/1839
                            (|In(input): In<ShouldRun>, state: Res<State<GameState>>| {
                                if state.current() == &GameState::Playing {
                                    input
                                } else {
                                    ShouldRun::No
                                }
                            })
                            .system(),
                        ),
                    ) // Run with fixed timestep
                    .with_system(finish_spawning_character.system().before(InputLabel))
                    .with_system(
                        touch_control_input
                            .system()
                            .label(InputLabel)
                            .in_ambiguity_set(InputLabel),
                    )
                    .with_system(
                        keyboard_control_input
                            .system()
                            .label(InputLabel)
                            .in_ambiguity_set(InputLabel),
                    )
                    .with_system(
                        control_character
                            .system()
                            .label(CharacterControlLabel)
                            .after(InputLabel),
                    )
                    .with_system(
                        animate_sprites
                            .system()
                            .label(AnimateSpritesLabel)
                            .after(CharacterControlLabel),
                    )
                    .with_system(
                        camera_follow_system
                            .system()
                            .label(CameraFollowLabel)
                            .after(CharacterControlLabel),
                    )
                    .with_system(
                        change_level
                            .system()
                            .after(CameraFollowLabel)
                            .after(AnimateSpritesLabel),
                    ),
            ),
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
    mut ui_tree: ResMut<UiTree>,
    #[cfg(not(wasm))] mut windows: ResMut<Windows>,
) {
    debug!("Awaiting game info load...");
    let game_info: Handle<GameInfo> = asset_server.load_cached("default.game.yaml");

    // Spawn the map and camera once the game info loads
    if let Some(game_info) = game_info_assets.get(game_info) {
        debug!("Game info loaded: spawning camera and map");

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
            (ui::start_menu)
        });

        // Transition to map loading state
        state.push(GameState::StartMenu).unwrap();
    }
}

/// Position the camera on the start menu
pub fn setup_start_menu(
    mut completed: Local<bool>,
    mut cameras: Query<&mut Position, With<Camera>>,
    mut maps_query: Query<&Handle<LdtkMap>>,
    current_level: Res<CurrentLevel>,
    mut map_layers: Query<(&LdtkMapLayer, &mut Visible)>,
    map_assets: Res<Assets<LdtkMap>>,
) {
    // Run only once
    if *completed {
        return;
    }

    // Get our camera and map information
    let mut camera_pos = if let Ok(pos) = cameras.single_mut() {
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
    *camera_pos = Position::new(
        level.world_x + level.px_wid / 2,
        level.world_y + level.px_hei / 2,
        0,
    );

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
) {
    if let Ok(map_handle) = map_query.single() {
        if let Some(map) = map_assets.get(map_handle) {
            debug!("Map loaded: spawning player");
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
            let player_z = level.layer_instances.as_ref().unwrap().len() as i32 * 2;

            // Spawn the player
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

    if let Ok((camera, mut camera_pos)) = cameras.single_mut() {
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
    mut commands: Commands,
    mut cameras: Query<&mut Camera>,
    mut characters: Query<(Entity, &Handle<Character>, &Sprite)>,
    mut world_positions: WorldPositionsQuery,
    maps: Query<&Handle<LdtkMap>>,
    map_assets: Res<Assets<LdtkMap>>,
    mut scene_graph: ResMut<SceneGraph>,
    image_assets: Res<Assets<Image>>,
    character_assets: Res<Assets<Character>>,
    mut current_level: ResMut<CurrentLevel>,
    mut current_level_music: Option<ResMut<CurrentLevelMusic>>,
    mut sound_controller: SoundController,
    asset_server: Res<AssetServer>,
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

                    // Play the level music
                    let music_field = to_level
                        .field_instances
                        .iter()
                        .find(|x| x.__identifier == "music")
                        .unwrap();

                    // Create helper to stop the music that is already playing
                    let stop_music = |controller: &mut SoundController, sound| {
                        controller.stop_sound_with_settings(
                            sound,
                            StopSoundSettings::new().fade_tween(Some(Tween {
                                duration: 1.0,
                                easing: Default::default(),
                                ease_direction: Default::default(),
                            })),
                        );
                    };

                    // If there is a music setting for this level
                    if let Some(new_music) = music_field.__value.as_str() {
                        // If the new music is the special value "none"
                        if new_music == "none" {
                            // Stop playing any music that might already be playing
                            if let Some(current_music) = current_level_music.as_ref() {
                                stop_music(&mut sound_controller, current_music.sound);
                            }

                            // And unset the current music
                            commands.remove_resource::<CurrentLevelMusic>();

                        // If there is new music we should play
                        } else {
                            // Get the new music file data
                            let new_sound_data = asset_server.load_cached(new_music);

                            // Create helper to play the new music
                            let play_music = |controller: &mut SoundController, new_sound_data| {
                                let sound = controller.create_sound(&new_sound_data);

                                controller.play_sound_with_settings(
                                    sound,
                                    PlaySoundSettings::new()
                                        .fade_in_tween(Tween {
                                            duration: 1.0,
                                            easing: Default::default(),
                                            ease_direction: Default::default(),
                                        })
                                        .loop_start(LoopStart::Custom(0.0)),
                                );

                                // Return the current level music data
                                CurrentLevelMusic {
                                    sound_data: new_sound_data,
                                    sound,
                                }
                            };

                            // If there is music currently playing
                            if let Some(current_music) = current_level_music.as_mut() {
                                // If the music currently playing is not already the music we want to play
                                if current_music.sound_data != new_sound_data {
                                    // Stop the old music
                                    stop_music(&mut sound_controller, current_music.sound);

                                    // And play new new music
                                    **current_music =
                                        play_music(&mut sound_controller, new_sound_data);
                                }

                            // If there is no music already playing, just play the new music
                            } else {
                                commands.insert_resource(play_music(
                                    &mut sound_controller,
                                    new_sound_data,
                                ));
                            }
                        }
                    }

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

mod ui {
    use bevy::prelude::World;
    use bevy_retro::ui::raui::prelude::*;

    use super::{CurrentLevel, GameInfo, GameState, State};

    fn use_start_menu(ctx: &mut WidgetContext) {
        ctx.life_cycle.change(|ctx| {
            let world: &mut World = ctx.process_context.get_mut().unwrap();

            for msg in ctx.messenger.messages {
                if let Some(msg) = msg.as_any().downcast_ref::<GameButtonMessage>() {
                    if &msg.0 == "start" {
                        let start_level = world
                            .get_resource::<GameInfo>()
                            .unwrap()
                            .game_start_level
                            .clone();

                        {
                            let mut current_level =
                                world.get_resource_mut::<CurrentLevel>().unwrap();
                            *current_level = CurrentLevel(start_level);
                        }
                        {
                            let mut state = world.get_resource_mut::<State<GameState>>().unwrap();
                            if state.current() != &GameState::LoadingGame {
                                state.push(GameState::LoadingGame).unwrap();
                            }
                        }
                    } else if &msg.0 == "show_settings" {
                        let mut query = world.query::<&super::Camera>();
                        let camera = query.iter_mut(world).next().expect("Expected one camera");

                        let previous_crt_filter_enabled = camera.custom_shader.is_some();
                        let previous_pixel_aspect_4_3_enabled =
                            camera.pixel_aspect_ratio.abs() - 1.0 > f32::EPSILON;

                        ctx.state
                            .write(StartMenuState {
                                show_settings: true,
                                previous_crt_filter_enabled,
                                previous_pixel_aspect_4_3_enabled,
                            })
                            .unwrap();
                    } else if &msg.0 == "cancel_settings" {
                        let mut query = world.query::<&mut super::Camera>();
                        let mut camera = query.iter_mut(world).next().expect("Expected one camera");

                        ctx.state
                            .mutate_cloned(|state: &mut StartMenuState| {
                                camera.pixel_aspect_ratio =
                                    if state.previous_pixel_aspect_4_3_enabled {
                                        4. / 3.
                                    } else {
                                        1.
                                    };

                                camera.custom_shader = if state.previous_crt_filter_enabled {
                                    Some(super::CrtShader::default().get_shader())
                                } else {
                                    None
                                };

                                state.show_settings = false;
                            })
                            .unwrap();
                    } else if &msg.0 == "save_settings" {
                        ctx.state
                            .mutate_cloned(|state: &mut StartMenuState| {
                                state.show_settings = false;
                            })
                            .unwrap();
                    }
                }
            }
        })
    }

    #[derive(PropsData, Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
    struct StartMenuState {
        show_settings: bool,
        previous_crt_filter_enabled: bool,
        previous_pixel_aspect_4_3_enabled: bool,
    }

    /// The UI tree used for the start menu
    #[pre_hooks(use_start_menu)]
    pub fn start_menu(mut ctx: WidgetContext) -> WidgetNode {
        let WidgetContext {
            id,
            process_context,
            ..
        } = ctx;

        let StartMenuState { show_settings, .. } = ctx.state.read_cloned_or_default();

        // Get the game info from the world
        let world: &mut World = process_context.get_mut().unwrap();
        let game_info = world.get_resource::<GameInfo>().unwrap();

        // Create shared props containing the theme
        let shared_props = Props::default()
            // Add the theme properties
            .with({
                let mut theme = ThemeProps::default();

                theme.content_backgrounds.insert(
                    String::from("panel"),
                    ThemedImageMaterial::Image(ImageBoxImage {
                        id: game_info.ui_theme.panel.image.clone(),
                        scaling: ImageBoxImageScaling::Frame(
                            (
                                game_info.ui_theme.panel.border_size as f32,
                                game_info.ui_theme.panel.only_frame,
                            )
                                .into(),
                        ),
                        ..Default::default()
                    }),
                );

                theme.content_backgrounds.insert(
                    String::from("button-up"),
                    ThemedImageMaterial::Image(ImageBoxImage {
                        id: game_info.ui_theme.button_up.image.clone(),
                        scaling: ImageBoxImageScaling::Frame(
                            (
                                game_info.ui_theme.button_up.border_size as f32,
                                game_info.ui_theme.button_up.only_frame,
                            )
                                .into(),
                        ),
                        ..Default::default()
                    }),
                );

                theme.content_backgrounds.insert(
                    String::from("button-down"),
                    ThemedImageMaterial::Image(ImageBoxImage {
                        id: game_info.ui_theme.button_down.image.clone(),
                        scaling: ImageBoxImageScaling::Frame(
                            (
                                game_info.ui_theme.button_down.border_size as f32,
                                game_info.ui_theme.button_down.only_frame,
                            )
                                .into(),
                        ),
                        ..Default::default()
                    }),
                );

                theme.switch_variants.insert(
                    "checkbox".to_owned(),
                    ThemedSwitchMaterial {
                        on: ThemedImageMaterial::Image(ImageBoxImage {
                            id: game_info.ui_theme.checkbox.checked.clone(),
                            ..Default::default()
                        }),
                        off: ThemedImageMaterial::Image(ImageBoxImage {
                            id: game_info.ui_theme.checkbox.unchecked.clone(),
                            ..Default::default()
                        }),
                    },
                );

                theme.text_variants.insert(
                    String::new(),
                    ThemedTextMaterial {
                        font: TextBoxFont {
                            name: game_info.ui_theme.default_font.clone(),
                            // Font's in Bevy Retro don't really have sizes so we can just set this to
                            // one
                            size: 1.0,
                        },
                        ..Default::default()
                    },
                );

                theme.icons_level_sizes = vec![8., 12., 16.];

                theme
            })
            .with(game_info.clone());

        let vertical_box_props = VerticalBoxProps {
            separation: 0.,
            ..Default::default()
        };

        // The title image props
        let title_image_props = Props::new(ImageBoxProps {
            material: ImageBoxMaterial::Image(ImageBoxImage {
                id: game_info.splash_screen.splash_image.path.clone(),
                ..Default::default()
            }),
            width: ImageBoxSizeValue::Exact(game_info.splash_screen.splash_image.size.x as f32),
            height: ImageBoxSizeValue::Exact(game_info.splash_screen.splash_image.size.y as f32),
            ..Default::default()
        })
        .with(FlexBoxItemLayout {
            align: 0.5,
            grow: 0.0,
            margin: Rect {
                top: 10.,
                ..Default::default()
            },
            ..Default::default()
        });

        let start_button_props = Props::new(FlexBoxItemLayout {
            align: 0.5,
            grow: 0.0,
            margin: Rect {
                top: 10.,
                ..Default::default()
            },
            ..Default::default()
        })
        .with(GameButtonProps {
            text: "Start Game".into(),
            notify_id: id.to_owned(),
            message_name: "start".into(),
        });

        let settings_button_props = Props::new(FlexBoxItemLayout {
            align: 0.5,
            grow: 0.0,
            margin: Rect {
                top: 10.,
                ..Default::default()
            },
            ..Default::default()
        })
        .with(GameButtonProps {
            text: "Settings".into(),
            notify_id: id.to_owned(),
            message_name: "show_settings".into(),
        });

        let content = if show_settings {
            let props = Props::new(SettingsPanelProps {
                cancel_notify_id: ctx.id.to_owned(),
                cancel_notify_message: "cancel_settings".into(),
                save_notify_id: ctx.id.to_owned(),
                save_notify_message: "save_settings".into(),
            });

            widget! {
                (#{"settings"} settings_panel: {props})
            }
        } else {
            widget! {
                // The main content
                (nav_vertical_box: {vertical_box_props} [
                    (image_box: {title_image_props})
                    (game_button: {start_button_props})
                    (game_button: {settings_button_props})
                ])
            }
        };

        widget! {
            (content_box | {shared_props} [
                {content}
            ])
        }
    }

    #[derive(PropsData, Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
    struct GameButtonProps {
        text: String,
        notify_id: WidgetId,
        message_name: String,
    }

    #[derive(MessageData, Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
    struct GameButtonMessage(String);

    fn use_game_button(ctx: &mut WidgetContext) {
        ctx.life_cycle.change(|ctx| {
            let ButtonProps { trigger, .. } = ctx.state.read_cloned_or_default();
            let GameButtonProps {
                notify_id,
                message_name: message,
                ..
            } = ctx.props.read_cloned_or_default();

            if trigger {
                ctx.messenger.write(notify_id, GameButtonMessage(message));
            }
        });
    }

    #[pre_hooks(
        // This allows us to get a `ButtonProps` instance from our widget state which will keep
        // track of whether or not we are clicked, hovered over, etc.
        use_game_button,
        use_button_notified_state,
    )]
    fn game_button(mut ctx: WidgetContext) -> WidgetNode {
        // Get our button state
        let ButtonProps {
            selected: hover,
            trigger: clicked,
            ..
        } = ctx.state.read_cloned_or_default();

        let GameButtonProps {
            text: button_text, ..
        } = ctx.props.read_cloned_or_default();

        let button_props = ctx
            .props
            .clone()
            .with(NavItemActive)
            .with(ButtonNotifyProps(ctx.id.to_owned().into()));

        let button_panel_props = Props::new(PaperProps {
            frame: None,
            variant: if clicked {
                // TODO: Somehow pre-load the button-up image so that it doesn't flash
                // blank for a second the first time a button is clicked
                String::from("button-down")
            } else {
                String::from("button-up")
            },
        });

        let scale = if hover { 1.1 } else { 1. };

        let label_props = Props::new(TextBoxProps {
            text: button_text,
            width: TextBoxSizeValue::Fill,
            height: TextBoxSizeValue::Fill,
            horizontal_align: TextBoxHorizontalAlign::Center,
            vertical_align: TextBoxVerticalAlign::Middle,
            font: TextBoxFont {
                name: "fonts/cozette.bdf".to_string(),
                size: 1.,
            },
            transform: Transform {
                translation: Vec2 {
                    x: 0.,
                    y: if clicked { 1. } else { 0. },
                },
                // scale: Vec2::from(1.0 / scale), // Undo button scale to make sure text stays same size
                ..Default::default()
            },
            ..Default::default()
        });

        let size_box_props = Props::new(SizeBoxProps {
            width: SizeBoxSizeValue::Exact(70.),
            height: SizeBoxSizeValue::Exact(18.),
            transform: Transform {
                scale: Vec2::from(scale),
                translation: Vec2 {
                    x: if hover { (-75. * scale + 75.) / 2. } else { 0. },
                    y: if hover { (-20. * scale + 20.) / 2. } else { 0. },
                },
                ..Default::default()
            },
            ..Default::default()
        });

        widget! {
            (button: {button_props} {
                content = (size_box: {size_box_props} {
                    content = (horizontal_paper: {button_panel_props} [
                        (text_box: {label_props})
                    ])
                })
            })
        }
    }

    #[derive(PropsData, Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
    struct SettingsPanelProps {
        cancel_notify_id: WidgetId,
        cancel_notify_message: String,
        save_notify_id: WidgetId,
        save_notify_message: String,
    }

    fn use_settings_panel(ctx: &mut WidgetContext) {
        ctx.life_cycle.change(|ctx| {
            let world: &mut World = ctx.process_context.get_mut().unwrap();
            let mut query = world.query::<&mut super::Camera>();
            let mut camera = query.iter_mut(world).next().expect("Expected one camera");

            for msg in ctx.messenger.messages {
                // Respond to click settings change messages
                if let Some(msg) = msg.as_any().downcast_ref::<ButtonNotifyMessage>() {
                    if msg.trigger_start() && msg.sender.ends_with("pixel_aspect") {
                        if (camera.pixel_aspect_ratio - 1.0).abs() < f32::EPSILON {
                            camera.pixel_aspect_ratio = 4.0 / 3.0;
                        } else {
                            camera.pixel_aspect_ratio = 1.0;
                        }
                    } else if msg.trigger_start() && msg.sender.ends_with("crt_filter") {
                        if camera.custom_shader == None {
                            camera.custom_shader = Some(super::CrtShader::default().get_shader())
                        } else {
                            camera.custom_shader = None;
                        }
                    }
                }
            }
        });
    }

    #[pre_hooks(use_settings_panel)]
    fn settings_panel(mut ctx: WidgetContext) -> WidgetNode {
        let game_info: GameInfo = ctx.shared_props.read_cloned().unwrap();
        let SettingsPanelProps {
            cancel_notify_id,
            cancel_notify_message,
            save_notify_id,
            save_notify_message,
        } = ctx.props.read_cloned_or_default();

        // Get the camera info from the world
        let world: &mut World = ctx.process_context.get_mut().unwrap();
        let mut query = world.query::<&super::Camera>();
        let camera = query.iter_mut(world).next().expect("Expected one camera");
        // Get the values for the checkboxes
        let crt_filter = camera.custom_shader.is_some();
        let pixel_aspect_4_3 = camera.pixel_aspect_ratio.abs() - 1.0 > f32::EPSILON;

        // Settings panel
        let panel_props = Props::new(ContentBoxItemLayout {
            // TODO: Open RAUI bug, margin somehow applies to both the inside and outside of the panel
            margin: Rect {
                left: 13.,
                right: 13.,
                top: 7.,
                bottom: 7.,
            },
            ..Default::default()
        })
        .with(PaperProps {
            variant: "panel".into(),
            frame: None,
        });

        // "Settings" title
        let title_props = Props::new(TextBoxProps {
            text: "Settings".into(),
            font: TextBoxFont {
                name: game_info.ui_theme.default_font.clone(),
                size: 1.0,
            },
            horizontal_align: TextBoxHorizontalAlign::Center,
            color: Color {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 1.,
            },
            ..Default::default()
        })
        .with(FlexBoxItemLayout {
            grow: 0.,
            basis: Some(16.),
            ..Default::default()
        });

        // Cancel button
        let cancel_button_props = Props::new(FlexBoxItemLayout {
            align: 0.5,
            grow: 0.0,
            margin: Rect {
                top: 10.,
                ..Default::default()
            },
            ..Default::default()
        })
        .with(GameButtonProps {
            text: "Cancel".into(),
            notify_id: cancel_notify_id,
            message_name: cancel_notify_message,
        });

        // Save button
        let save_button_props = Props::new(FlexBoxItemLayout {
            align: 0.5,
            grow: 0.0,
            margin: Rect {
                top: 10.,
                ..Default::default()
            },
            ..Default::default()
        })
        .with(GameButtonProps {
            text: "Save".into(),
            notify_id: save_notify_id,
            message_name: save_notify_message,
        });

        // Container for buttons
        let button_box_props = Props::new(())
            .with(FlexBoxProps {
                wrap: true,
                direction: FlexBoxDirection::HorizontalLeftToRight,
                separation: 17.,
                ..Default::default()
            })
            .with(FlexBoxItemLayout {
                grow: 0.0,
                align: 0.5,
                margin: Rect {
                    top: 8.,
                    bottom: 8.,
                    ..Default::default()
                },
                ..Default::default()
            });

        // "Graphics" title
        let graphics_settings_title_props = Props::new(TextBoxProps {
            text: "Graphics".into(),
            font: TextBoxFont {
                name: game_info.ui_theme.default_font.clone(),
                size: 1.0,
            },
            color: Color {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 1.,
            },
            ..Default::default()
        })
        .with(FlexBoxItemLayout {
            grow: 0.0,
            align: 0.0,
            basis: Some(16.),
            margin: Rect {
                left: 5.,
                ..Default::default()
            },
            ..Default::default()
        });

        // Wrapper for check box settings
        let check_box_wrapper_props = Props::new(FlexBoxItemLayout {
            grow: 0.0,
            basis: Some(17.),
            margin: Rect {
                top: 5.,
                left: 10.,
                ..Default::default()
            },
            ..Default::default()
        });

        // CRT Filter check box
        let crt_filter_check_props = Props::new(SwitchPaperProps {
            on: crt_filter,
            variant: "checkbox".into(),
            size_level: 1,
        })
        .with(NavItemActive)
        .with(ButtonNotifyProps(ctx.id.to_owned().into()))
        .with(ThemedWidgetProps {
            color: ThemeColor::Primary,
            variant: ThemeVariant::ContentOnly,
        })
        .with(FlexBoxItemLayout {
            grow: 0.0,
            ..Default::default()
        });

        // CRT Filter text
        let crt_filter_text_props = Props::new(TextBoxProps {
            text: "CRT Filter".into(),
            font: TextBoxFont {
                name: game_info.ui_theme.default_font.clone(),
                size: 1.0,
            },
            color: Color {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 1.,
            },
            ..Default::default()
        })
        .with(FlexBoxItemLayout {
            margin: Rect {
                left: 10.,
                ..Default::default()
            },
            ..Default::default()
        });

        // 4/3 Pixel Aspect Ratio checkbox
        let pixel_aspect_check_props = Props::new(SwitchPaperProps {
            on: pixel_aspect_4_3,
            variant: "checkbox".into(),
            size_level: 1,
        })
        .with(NavItemActive)
        .with(ButtonNotifyProps(ctx.id.to_owned().into()))
        .with(ThemedWidgetProps {
            color: ThemeColor::Primary,
            variant: ThemeVariant::ContentOnly,
        })
        .with(FlexBoxItemLayout {
            grow: 0.0,
            ..Default::default()
        });

        // 4/3 Pixel Aspect Ratio text
        let pixel_aspect_text_props = Props::new(TextBoxProps {
            text: "4/3 Pixel Aspect Ratio".into(),
            font: TextBoxFont {
                name: game_info.ui_theme.default_font,
                size: 1.0,
            },
            color: Color {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 1.,
            },
            ..Default::default()
        })
        .with(FlexBoxItemLayout {
            margin: Rect {
                left: 10.,
                ..Default::default()
            },
            ..Default::default()
        });

        widget! {
            (nav_content_box [
                (nav_vertical_paper: {panel_props} [
                    (text_box: {title_props})
                    (vertical_box [
                        (text_box: {graphics_settings_title_props})
                        (horizontal_box: {check_box_wrapper_props.clone()} [
                            (#{"crt_filter"} switch_button_paper: {crt_filter_check_props})
                            (text_box: {crt_filter_text_props})
                        ])
                        (horizontal_box: {check_box_wrapper_props} [
                            (#{"pixel_aspect"} switch_button_paper: {pixel_aspect_check_props})
                            (text_box: {pixel_aspect_text_props})
                        ])
                    ])
                    (flex_box: {button_box_props} [
                        (game_button: {cancel_button_props})
                        (game_button: {save_button_props})
                    ])
                ])
            ])
        }
    }
}

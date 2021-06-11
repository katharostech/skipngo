use bevy_retrograde::prelude::raui::core::make_widget;

use super::*;

mod hud;

/// The amount of health an object that can die or be destroyed has
pub struct Health {
    /// The current health of the entity
    pub current: u32,
    /// The maximum amount of health the entity can have
    pub max: u32,
}

//
// Game play systems
//

pub fn spawn_hud(state: Res<State<GameState>>, mut ui: ResMut<UiTree>) {
    // If we have just changed to gameplay state
    if state.is_changed() {
        // Spawn the HUD
        *ui = UiTree(make_widget!(hud::hud).into());
    }
}

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
    mut pause_was_pressed: Local<bool>,
    mut control_events: EventWriter<ControlEvent>,
    keyboard_input: Res<Input<KeyCode>>,
    mut state: ResMut<State<GameState>>,
) {
    if keyboard_input.pressed(KeyCode::Escape) && !*pause_was_pressed {
        debug!("Pausing game");
        state
            .push(GameState::Paused)
            .expect("Could not transition to paused state");
        *pause_was_pressed = true;
    } else if !keyboard_input.pressed(KeyCode::Escape) {
        *pause_was_pressed = false;
    }

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
            commands.entity(ent).insert(CharacterLoaded).insert(Health {
                max: character.max_health,
                current: character.max_health,
            });
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

/// Helper macro to unwrap an option or return the function
macro_rules! unwrap_or_return {
    ($e:expr) => {
        if let Some(e) = $e {
            e
        } else {
            return;
        }
    };
}

/// Enumerates different states the entrance transition logic can be in
#[derive(PartialEq, Eq)]
pub enum EntranceStatus {
    /// Totally outside of any entrance
    Outside,
    /// Teleporting and waiting to reach the other entrance
    TeleportingTo { entrance_id: String },
    /// Waiting for the player to walk out of the entrance after having teleported
    AwatingLeave,
}

impl Default for EntranceStatus {
    fn default() -> Self {
        EntranceStatus::Outside
    }
}

pub fn change_level(
    mut status: Local<EntranceStatus>,
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
    let map_handle = unwrap_or_return!(maps.iter().next());
    let map = unwrap_or_return!(map_assets.get(map_handle));

    // Get the current map level
    let level = map
        .project
        .levels
        .iter()
        .find(|x| x.identifier == **current_level)
        .expect("Current level not found");

    // Detect character collision
    if let Ok((character_ent, character_handle, character_sprite)) = characters.single_mut() {
        let character = unwrap_or_return!(character_assets.get(character_handle));
        let character_collision = unwrap_or_return!(image_assets.get(&character.collision_shape));

        // For every entity layer in the level
        for layer in level
            .layer_instances
            .as_ref()
            .expect("Level has no layers")
            .iter()
            .filter(|x| x.__type == "Entities")
        {
            let mut has_collided = false;

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

                // If we have collided with an entrance
                if pixels_collide_with_bounding_box(character_collider, entrance_bounds) {
                    has_collided = true;

                    // Get the entrance id
                    let entrance_id = entrance
                        .field_instances
                        .iter()
                        .find(|x| x.__identifier == "id")
                        .expect("Could not find `id` field of entrance")
                        .__value
                        .as_str()
                        .expect("`id` field of entrance is null");

                    // Figure out where to teleport to
                    let to_level_id = entrance
                        .field_instances
                        .iter()
                        .find(|x| x.__identifier == "to")
                        .expect("Entrance missing `to` property")
                        .__value
                        .as_str()
                        .expect("Entrance `to` property is null");
                    let to_entrance_id = entrance
                        .field_instances
                        .iter()
                        .find(|x| x.__identifier == "spawn_at")
                        .expect("Entrance missing `spawn_at` value")
                        .__value
                        .as_str()
                        .expect("Entrance `spawn_at` property is null");

                    match &*status {
                        // If we are in the middle of teleporting to an entrance
                        EntranceStatus::TeleportingTo {
                            entrance_id: target_entrance_id,
                        } => {
                            // If we have collided with the entrance we are trying to teleport to
                            if entrance_id == target_entrance_id.as_str() {
                                // Transition into an awaiting leave state
                                *status = EntranceStatus::AwatingLeave;
                            }

                            // And skip all tasks below
                            return;
                        }
                        // If we are waiting to leave an entrance we have just gotten to, we just
                        // skip everything below. We don't respond to collision with the entrance
                        // until we leave the entrance.
                        EntranceStatus::AwatingLeave => {
                            return;
                        }

                        // We are outside of an entrance and walking into it for the first time
                        EntranceStatus::Outside => {
                            // Move to teleporting state and continue on with the logic below to
                            // teleport to the target entrance
                            *status = EntranceStatus::TeleportingTo {
                                entrance_id: to_entrance_id.into(),
                            };
                        }
                    }

                    // Get the level that we will be teleporting to
                    let to_level = map
                        .project
                        .levels
                        .iter()
                        .find(|x| x.identifier == to_level_id)
                        .unwrap_or_else(|| {
                            panic!(
                                "Level `{}` does not exist. Could not teleport there.",
                                to_level_id
                            )
                        });

                    // Get the spawn point we will be teleporting to
                    let to_entrance = to_level
                        .layer_instances
                        .as_ref()
                        .expect("Teleport `to` level does not have any layers")
                        .iter()
                        .find_map(|x| {
                            x.entity_instances.iter().find(|x| {
                                x.__identifier == "Entrance"
                                    && x.field_instances.iter().any(|x| {
                                        x.__identifier == "id" && x.__value == to_entrance_id
                                    })
                            })
                        })
                        .unwrap_or_else(|| {
                            panic!(
                                "Could not find entrance `{}` in level `{}` to teleport to",
                                to_entrance_id, to_level_id
                            )
                        });

                    // Set the current level to the new level
                    *current_level = CurrentLevel(to_level_id.into());

                    // Play the level music
                    let music_field = to_level
                        .field_instances
                        .iter()
                        .find(|x| x.__identifier == "music")
                        .expect("Level missing field `music`");

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
                        .expect("Character missing position component");

                    *character_pos = Position::new(
                        to_level.world_x + to_entrance.px[0] + to_entrance.width / 2,
                        to_level.world_y + to_entrance.px[1] + to_entrance.height / 2,
                        level
                            .layer_instances
                            .as_ref()
                            .expect("Level does not have any layers")
                            .len() as i32
                            * 2,
                    );
                }
            }

            if !has_collided {
                // If we are waiting to leave an entrance
                if *status == EntranceStatus::AwatingLeave {
                    // We're outside again!
                    *status = EntranceStatus::Outside;
                }
            }
        }
    }
}

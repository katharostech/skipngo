use bevy::prelude::*;
use bevy_retro::*;
use bevy_retro_ldtk::*;

use crate::plugins::game::CurrentLevel;

use super::*;

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
pub fn control_character<'a>(
    mut world_positions: WorldPositionsQuery,
    mut characters: Query<
        (Entity, &Handle<Character>, &mut CharacterState, &Sprite),
        With<Handle<Character>>,
    >,
    map_layers: Query<(Entity, &LdtkMapLayer, &Handle<Image>, &Sprite)>,
    character_assets: Res<Assets<Character>>,
    input: Res<Input<KeyCode>>,
    mut scene_graph: ResMut<SceneGraph>,
    image_assets: Res<Assets<Image>>,
) {
    // Synchronize world positions before checking for collisions
    world_positions.sync_world_positions(&mut scene_graph);

    // Check for collisions and record all the characters that collided
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
        if input.pressed(KeyCode::Right) {
            movement += IVec3::new(1, 0, 0);
        }
        if input.pressed(KeyCode::Left) {
            movement += IVec3::new(-1, 0, 0);
        }
        if input.pressed(KeyCode::Down) {
            movement += IVec3::new(0, 1, 0);
        }
        if input.pressed(KeyCode::Up) {
            movement += IVec3::new(0, -1, 0);
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
                let base_character_world_position = *world_positions
                    .get_world_position_mut(character_ent)
                    .unwrap()
                    .clone();

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
                    pixels_collide_with(character_collider, layer_collider)
                };

                // Perform ritual to check for collisions ( in a closure to make it easy to return early )
                let has_collided = (|| {
                    // If our current movement would cause a collision
                    if collides(movement) {
                        // Try setting x movement to nothing and check again
                        if movement.x != 0 {
                            let mut new_movement = movement.clone();
                            new_movement.x = 0;

                            if !collides(new_movement) {
                                *movement = *new_movement;
                                return false;
                            }
                        }

                        // Try setting y movement to nothing and check again
                        if movement.y != 0 {
                            let mut new_movement = movement.clone();
                            new_movement.y = 0;

                            if !collides(new_movement) {
                                *movement = *new_movement;
                                return false;
                            }
                        }

                        // If we are still colliding, just set movement to nothing and break out of this loop
                        *movement = *IVec3::ZERO;
                        return true;

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
        if movement.x != 0 && movement.y != 0 {
            if character_state.animation_frame % 2 == 0 {
                movement.y = 0;
                movement.x = 0;
            }
        }

        // Move the player
        let mut pos = world_positions
            .get_local_position_mut(character_ent)
            .unwrap();
        **pos += movement;
    }
}

/// Play the character's sprite animation
pub fn animate_sprite_system(
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

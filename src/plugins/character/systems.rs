use bevy::prelude::*;
use bevy_retro::*;
use bevy_retro_ldtk::*;

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
    // Loop through characters and move them
    for (character_ent, character_handle, mut state, _) in characters.iter_mut() {
        let mut pos = world_positions.get_local_position_mut(character_ent).unwrap();

        if character_assets.get(character_handle).is_some() {
            let mut direction = IVec2::default();

            // Determine movement direction
            if input.pressed(KeyCode::Right) {
                direction += IVec2::new(1, 0);
            }
            if input.pressed(KeyCode::Left) {
                direction += IVec2::new(-1, 0);
            }
            if input.pressed(KeyCode::Down) {
                direction += IVec2::new(0, 1);
            }
            if input.pressed(KeyCode::Up) {
                direction += IVec2::new(0, -1);
            }

            // Determine animation and direction
            let new_action;
            let mut new_direction = state.direction;

            if direction.x == 0 && direction.y == 0 {
                new_action = CharacterStateAction::Idle;
            } else {
                new_action = CharacterStateAction::Walk;

                if direction.y.abs() > 0 && direction.x.abs() > 0 {
                    // We are moving diagnally, so the new direction should be the same as the
                    // previous direction and we don't do anything.
                } else if direction.y > 0 {
                    new_direction = CharacterStateDirection::Down;
                } else if direction.y < 0 {
                    new_direction = CharacterStateDirection::Up;
                } else if direction.x > 0 {
                    new_direction = CharacterStateDirection::Right;
                } else if direction.x < 0 {
                    new_direction = CharacterStateDirection::Left;
                }
            }

            // Update the character action
            if new_action != state.action {
                state.action = new_action;
                state.tileset_index = 0;
                state.animation_frame = 0;
            }

            // Make sure movement speed is normalized
            if direction.x != 0 && direction.y != 0 {
                if state.animation_frame % 2 == 0 {
                    direction.y = 0;
                    direction.x = 0;
                }
            }

            if new_direction != state.direction {
                state.direction = new_direction;
                state.tileset_index = 0;
                state.animation_frame = 0;
            }

            // Record the previous position so that we can move the player back in the collision
            // detection system.
            state.previous_position = pos.clone();

            // Move the sprite
            pos.x += direction.x;
            pos.y += direction.y;
        }
    }

    // Synchronize world positions before checking for collisions
    world_positions.sync_world_positions(&mut scene_graph);

    // Check for collisions and record all the characters that collided
    for (character_ent, character_handle, character_state, character_sprite) in
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
                if pixels_collide_with(
                    PixelColliderInfo {
                        image: character_collision,
                        position: &world_positions
                            .get_world_position_mut(character_ent)
                            .unwrap()
                            .clone(),
                        sprite: character_sprite,
                        sprite_sheet: None,
                    },
                    PixelColliderInfo {
                        image: layer_image,
                        position: &world_positions.get_world_position_mut(layer_ent).unwrap(),
                        sprite: layer_sprite,
                        sprite_sheet: None,
                    },
                ) {
                    let mut character_local_pos =
                        world_positions.get_local_position_mut(character_ent).unwrap();

                    *character_local_pos = character_state.previous_position;
                }
            }
        }
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

/// Make the camera follow the character
pub fn camera_follow(
    mut cameras: Query<&mut Position, With<Camera>>,
    characters: Query<&Position, (With<Handle<Character>>, Without<Camera>)>,
) {
    if let Some(mut camera_trans) = cameras.iter_mut().next() {
        if let Some(character_trans) = characters.iter().next() {
            camera_trans.x = character_trans.x;
            camera_trans.y = character_trans.y;
        }
    }
}

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
    mut queries: QuerySet<(
        // Character mutation query
        Query<(&mut Position, &mut CharacterState, &Handle<Character>)>,
        // World positions so that we can synchronize them
        WorldPositions,
        // Character collision read query
        Query<
            (
                Entity,
                &Handle<Image>,
                &Sprite,
                &Handle<SpriteSheet>,
                &WorldPosition,
            ),
            With<Handle<Character>>,
        >,
        // Query map layers
        Query<(&LdtkMapLayer, &Handle<Image>, &Sprite, &WorldPosition)>,
    )>,
    character_assets: Res<Assets<Character>>,
    input: Res<Input<KeyCode>>,
    mut scene_graph: ResMut<SceneGraph>,
    image_assets: Res<Assets<Image>>,
    sprite_sheet_assets: Res<Assets<SpriteSheet>>,
) {
    // Loop through characters and move them
    for (mut pos, mut state, character_handle) in queries.q0_mut().iter_mut() {
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
    queries.q1_mut().sync_world_positions(&mut scene_graph);

    // Check for collisions and record all the characters that collided
    let mut collided_characters = Vec::new();
    for (character_ent, character_image, character_sprite, character_sprite_sheet, character_pos) in
        queries.q2().iter()
    {
        let character_image = if let Some(i) = image_assets.get(character_image) {
            i
        } else {
            continue;
        };
        let character_sprite_sheet =
            if let Some(i) = sprite_sheet_assets.get(character_sprite_sheet) {
                i
            } else {
                continue;
            };

        for (layer, layer_image, layer_sprite, layer_pos) in queries.q3().iter() {
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
                        image: character_image,
                        position: character_pos,
                        sprite: character_sprite,
                        sprite_sheet: Some(character_sprite_sheet),
                    },
                    PixelColliderInfo {
                        image: layer_image,
                        position: layer_pos,
                        sprite: layer_sprite,
                        sprite_sheet: None,
                    },
                ) {
                    collided_characters.push(character_ent);
                }
            }
        }
    }

    // Move all collided characters back to their original locations
    for collided_character_ent in collided_characters {
        let (mut pos, state, _) = queries.q0_mut().get_mut(collided_character_ent).unwrap();
        *pos = state.previous_position;
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

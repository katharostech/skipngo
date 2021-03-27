use bevy::prelude::*;
use bevy_retro::*;

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

pub struct CharacterAnimationFrame(pub u16);

/// Walk the character in response to input
pub fn control_character(
    mut query: Query<(&mut Position, &mut CharacterState, &Handle<Character>)>,
    characters: Res<Assets<Character>>,
    input: Res<Input<KeyCode>>,
) {
    for (mut pos, mut state, handle) in query.iter_mut() {
        if characters.get(handle).is_some() {
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

            // Move the sprite
            pos.x += direction.x;
            pos.y += direction.y;
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

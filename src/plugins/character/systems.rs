use bevy::prelude::*;
use bevy_retro::*;

use super::{
    Character, CharacterCurrentTilesetIndex, CurrentCharacterAction, CurrentCharacterDirection,
};

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
    mut query: Query<(
        &mut Position,
        &mut CurrentCharacterAction,
        &mut CurrentCharacterDirection,
        &mut CharacterCurrentTilesetIndex,
        &mut CharacterAnimationFrame,
        &Handle<Character>,
    )>,
    characters: Res<Assets<Character>>,
    input: Res<Input<KeyCode>>,
) {
    for (
        mut trans,
        mut current_action,
        mut current_direction,
        mut current_tileset_index,
        mut character_animation_frame,
        handle,
    ) in query.iter_mut()
    {
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
            let mut new_direction = *current_direction;

            if direction.x == 0 && direction.y == 0 {
                new_action = CurrentCharacterAction::Idle;
            } else {
                new_action = CurrentCharacterAction::Walk;

                if direction.y.abs() > 0 && direction.x.abs() > 0 {
                    // We are moving diagnally, so the new direction should be the same as the
                    // previous direction and we don't do anything.
                } else if direction.y > 0 {
                    new_direction = CurrentCharacterDirection::Down;
                } else if direction.y < 0 {
                    new_direction = CurrentCharacterDirection::Up;
                } else if direction.x > 0 {
                    new_direction = CurrentCharacterDirection::Right;
                } else if direction.x < 0 {
                    new_direction = CurrentCharacterDirection::Left;
                }
            }

            // Update the character action
            if new_action != *current_action {
                *current_action = new_action;
                current_tileset_index.0 = 0;
                character_animation_frame.0 = 0;
            }

            // Make sure movement speed is normalized
            if direction.x != 0 && direction.y != 0 {
                if character_animation_frame.0 % 2 == 0 {
                    direction.y = 0;
                    direction.x = 0;
                }
            }

            if new_direction != *current_direction {
                *current_direction = new_direction;
                current_tileset_index.0 = 0;
                character_animation_frame.0 = 0;
            }

            // Move the sprite
            trans.x += direction.x;
            trans.y += direction.y;
        }
    }
}

/// Play the character's sprite animation
pub fn animate_sprite_system(
    characters: Res<Assets<Character>>,
    mut query: Query<(
        &Handle<SpriteSheet>,
        &mut Sprite,
        &mut CharacterCurrentTilesetIndex,
        &CurrentCharacterAction,
        &CurrentCharacterDirection,
        &mut CharacterAnimationFrame,
        &Handle<Character>,
    )>,
    mut sprite_sheet_assets: ResMut<Assets<SpriteSheet>>,
) {
    for (
        sprite_sheet,
        mut sprite,
        mut current_anim_index,
        current_action,
        current_direction,
        mut character_animation_frame,
        character_handle,
    ) in query.iter_mut()
    {
        if character_animation_frame.0 % 10 == 0 {
            if let Some(sprite_sheet) = sprite_sheet_assets.get_mut(sprite_sheet) {
                let character = characters.get(character_handle).unwrap();

                let action = match *current_action {
                    CurrentCharacterAction::Walk => &character.actions.walk,
                    CurrentCharacterAction::Idle => &character.actions.idle,
                };

                let direction = match *current_direction {
                    CurrentCharacterDirection::Up => &action.animations.up,
                    CurrentCharacterDirection::Down => &action.animations.down,
                    CurrentCharacterDirection::Left => &action.animations.left,
                    CurrentCharacterDirection::Right => &action.animations.right,
                };

                if direction.flip {
                    sprite.flip_x = true;
                } else {
                    sprite.flip_x = false;
                }

                let idx = direction
                    .frames
                    .iter()
                    .cycle()
                    .nth(current_anim_index.0 as usize)
                    .unwrap();

                sprite_sheet.tile_index = *idx;

                current_anim_index.0 = current_anim_index.0.wrapping_add(1);
            }
        }

        character_animation_frame.0 = character_animation_frame.0.wrapping_add(1);
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

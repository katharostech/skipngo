use std::time::Duration;

use bevy_retrograde::physics::heron::rapier_plugin::PhysicsWorld;
use bevy_retrograde::prelude::{kira::parameter::tween::Tween, raui::core::make_widget};
use itertools::Itertools;

use crate::utils::{IntoBevy, IntoNav};

use super::map_loading::LdtkMapLevelNavigationMeshes;
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

/// Switch to game over when the player runs out of health
pub fn check_for_game_over(
    characters: Query<&Health, With<Handle<Character>>>,
    mut state: ResMut<State<GameState>>,
) {
    for character_health in characters.iter() {
        // If player health is 0, then go to game over
        if character_health.current == 0 {
            state
                .push(GameState::GameOver)
                .expect("Could not transition to game over state");
        }
    }
}

/// Listen for touch events and send character control events in response
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

/// Listen for keyboard events and send character control events in response
pub fn keyboard_control_input(
    mut pause_was_pressed: Local<bool>,
    mut control_events: EventWriter<ControlEvent>,
    keyboard_input: Res<Input<KeyCode>>,
    mut state: ResMut<State<GameState>>,
    mut physics_time: ResMut<PhysicsTime>,
) {
    if keyboard_input.pressed(KeyCode::Escape) && !*pause_was_pressed {
        debug!("Pausing game");
        state
            .push(GameState::Paused)
            .expect("Could not transition to paused state");
        *pause_was_pressed = true;
        physics_time.pause();
    } else if !keyboard_input.pressed(KeyCode::Escape) {
        *pause_was_pressed = false;
    }

    if keyboard_input.pressed(KeyCode::A) {
        control_events.send(ControlEvent::MoveLeft);
    }

    if keyboard_input.pressed(KeyCode::D) {
        control_events.send(ControlEvent::MoveRight);
    }

    if keyboard_input.pressed(KeyCode::W) {
        control_events.send(ControlEvent::MoveUp);
    }

    if keyboard_input.pressed(KeyCode::S) {
        control_events.send(ControlEvent::MoveDown);
    }
}

/// Marker component for loaded characters
pub struct CharacterLoaded;
pub struct CharacterAnimationTimer(pub Timer);
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
            // Set the players sprite image and sheet
            *image_handle = character.sprite_image.clone();
            *sprite_sheet_handle = character.sprite_sheet.clone();

            commands
                .entity(ent)
                // Add the character loaded marker so we don't do this again
                .insert(CharacterLoaded)
                // Set the players health and max health
                .insert(Health {
                    max: character.max_health,
                    current: character.max_health,
                })
                // Set the character's collision shape to it's tesselated collider image
                .insert(TesselatedCollider {
                    image: character.collision_shape.clone(),
                    tesselator_config: TesselatedColliderConfig {
                        vertice_separation: 0.,
                        ..Default::default()
                    },
                })
                // Start them off not moving
                .insert(Velocity::from_linear(Vec3::new(0., -12., 0.)))
                // Lock rotations
                .insert(RotationConstraints::lock())
                // Make him not bouncy and remove friction
                .insert(PhysicMaterial {
                    friction: 0.,
                    restitution: 0.,
                    ..Default::default()
                })
                // And make it a dynamic body
                .insert(RigidBody::Dynamic)
                // Add a timer that will be used for calculating animation frames
                .insert(CharacterAnimationTimer(Timer::new(
                    Duration::from_millis(100),
                    true,
                )))
                .insert(CollisionLayers::from_bits(
                    // Put in the player group
                    PhysicsGroup::Player.to_bits(),
                    // Interact with all other groups
                    PhysicsGroup::all_bits(),
                ));
        }
    }
}

/// Move the character in response to character control events
pub fn control_character(
    mut characters: Query<
        (
            &Handle<Character>,
            &Transform,
            &mut CharacterState,
            &mut Velocity,
        ),
        With<Handle<Character>>,
    >,
    character_assets: Res<Assets<Character>>,
    mut control_events: EventReader<ControlEvent>,
    time: Res<Time>,
) {
    // Loop through characters
    for (character_handle, character_transform, mut character_state, mut character_velocity) in
        characters.iter_mut()
    {
        let character = if let Some(character) = character_assets.get(character_handle) {
            character
        } else {
            continue;
        };

        let mut movement = Vec3::default();

        // Check for damage knock-back state
        //
        // We do a check for the enum variant first so we avoid mutably borrowing and triggering the
        // is_changed() detection.
        if matches!(
            &character_state.action,
            CharacterStateAction::DamageKnockBack { .. }
        ) {
            if let CharacterStateAction::DamageKnockBack {
                force_timer,
                freeze_timer,
            } = &mut character_state.action
            {
                // Tick the knock-back timers
                force_timer.tick(time.delta());
                freeze_timer.tick(time.delta());

                let mut skip_controls = false;

                // If the freeze timer has **not** finished
                if !freeze_timer.finished() {
                    // Skip running the normal character controller behaviour to freeze the controls
                    skip_controls = true;
                } else {
                }

                // If the force timer has finished
                if force_timer.finished() {
                    // Set the velocity to 0
                    *character_velocity = Velocity::from_linear(Vec3::default());
                }

                if skip_controls {
                    continue;
                }
            }
        }

        // Determine movement direction
        let mut directions = HashSet::default();
        for control_event in control_events.iter() {
            let z = character_transform.translation.z;
            if directions.insert(control_event) {
                match control_event {
                    ControlEvent::MoveUp => movement += Vec3::new(0., -1., z),
                    ControlEvent::MoveDown => movement += Vec3::new(0., 1., z),
                    ControlEvent::MoveLeft => movement += Vec3::new(-1., 0., z),
                    ControlEvent::MoveRight => movement += Vec3::new(1., 0., z),
                }
            }
        }

        // Determine animation and direction
        let new_action;
        let mut new_direction = character_state.direction;

        if movement.x == 0. && movement.y == 0. {
            new_action = CharacterStateAction::Idle;
        } else {
            new_action = CharacterStateAction::Walk;

            if movement.y.abs() > 0. && movement.x.abs() > 0. {
                // We are moving diagnally, so the new direction should be the same as the
                // previous direction and we don't do anything.
            } else if movement.y > 0. {
                new_direction = CharacterStateDirection::Down;
            } else if movement.y < 0. {
                new_direction = CharacterStateDirection::Up;
            } else if movement.x > 0. {
                new_direction = CharacterStateDirection::Right;
            } else if movement.x < 0. {
                new_direction = CharacterStateDirection::Left;
            }
        }

        // Reset character animation frame if direction or action changes
        if new_direction != character_state.direction || new_action != character_state.action {
            character_state.anim_frame_idx = 0;
        }
        // Update character action
        if new_action != character_state.action {
            character_state.action = new_action;
        }
        // Update character direction
        if new_direction != character_state.direction {
            character_state.direction = new_direction;
        }

        if movement.length() > f32::EPSILON {
            // Set player speed
            movement = movement.normalize() * character.walk_speed;
        }

        // Update player velocity
        *character_velocity = Velocity::from_linear(movement);
    }
}

/// Handles damaging characters
pub fn damage_character(
    mut characters: Query<
        (
            &mut Velocity,
            &mut CharacterState,
            &mut Health,
            &GlobalTransform,
        ),
        With<Handle<Character>>,
    >,
    damage_regions: Query<(&DamageRegion, &GlobalTransform)>,
    mut collision_events: EventReader<CollisionEvent>,
) {
    // Check characters colliding with entrances
    for event in collision_events.iter() {
        let (ent1, ent2) = event.collision_shape_entities();

        // If this is not a started event, skip it
        if !event.is_started() {
            continue;
        }

        // Get the character from the collision or skip the event
        let (mut character_velocity, mut character_state, mut character_health, character_location) =
            if let Ok(character) = characters.get_mut(ent1) {
                character
            } else if let Ok(character) = characters.get_mut(ent2) {
                character
            } else {
                continue;
            };

        // Get the damage region of the collision or skip the event
        let (damage_region, damage_region_location) = if let Ok(region) = damage_regions
            .get(ent1)
            .or_else(|_| damage_regions.get(ent2))
        {
            region
        } else {
            continue;
        };

        // Damage the player
        character_health.current -= damage_region.damage.min(character_health.current);

        // Put the player into knock-back frames
        character_state.action = CharacterStateAction::DamageKnockBack {
            force_timer: Timer::new(
                Duration::from_secs_f32(damage_region.knock_back.force_duration),
                false,
            ),
            freeze_timer: Timer::new(
                Duration::from_secs_f32(damage_region.knock_back.freeze_duration),
                false,
            ),
        };

        // FIXME: We need to make sure the location is properly updated, right now we use
        // GlobalTransform, but maybe we need to run this system after the transform propagation
        // system.

        // Get the push direction
        let push_direction = (character_location.translation - damage_region_location.translation)
            .normalize_or_zero();

        // Set the character velocity
        *character_velocity =
            Velocity::from_linear(push_direction * damage_region.knock_back.speed);
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
        &mut CharacterAnimationTimer,
    )>,
    mut sprite_sheet_assets: ResMut<Assets<SpriteSheet>>,
    time: Res<Time>,
) {
    // For every character and their sprites
    for (sprite_sheet, mut sprite, mut state, character_handle, mut timer) in query.iter_mut() {
        // Tick their animation timer
        timer.0.tick(time.delta());

        // If the timer has finished or if our animation state has changed
        if timer.0.just_finished() || state.is_changed() {
            // Reset the timer
            timer.0.set_elapsed(Duration::from_millis(0));

            // If the spritesheet info is loaded
            if let Some(sprite_sheet) = sprite_sheet_assets.get_mut(sprite_sheet) {
                let character = characters.get(character_handle).unwrap();

                // Get the character info for our current action
                let action = match state.action {
                    CharacterStateAction::Idle | CharacterStateAction::DamageKnockBack { .. } => {
                        &character.actions.idle
                    }
                    CharacterStateAction::Walk => &character.actions.walk,
                };

                // Get the animation frames for the direction we are facing
                let direction = match state.direction {
                    CharacterStateDirection::Up => &action.animations.up,
                    CharacterStateDirection::Down => &action.animations.down,
                    CharacterStateDirection::Left => &action.animations.left,
                    CharacterStateDirection::Right => &action.animations.right,
                };

                // Flip the sprite if necessary
                if direction.flip {
                    sprite.flip_x = true;
                } else {
                    sprite.flip_x = false;
                }

                // Get the index of the current animation frame
                let idx = direction.frames[state.anim_frame_idx as usize % direction.frames.len()];

                // Set the current tile in sprite sheet
                sprite_sheet.tile_index = idx;

                // Set
                state.anim_frame_idx = state.anim_frame_idx.wrapping_add(1);
            }
        }
    }
}

// Make the camera follow the character
pub fn camera_follow_system(
    mut cameras: Query<(&Camera, &mut Transform)>,
    characters: Query<&GlobalTransform, (With<Handle<Character>>, Without<Camera>)>,
    mut map_layers: Query<
        (&mut LdtkMapLayer, &mut Visible, &Handle<Image>, &Transform),
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

    if let Ok((camera, mut camera_transform)) = cameras.single_mut() {
        let camera_pos = &mut camera_transform.translation;

        // Start by making the camera stick to the player
        if let Some(character_transform) = characters.iter().next() {
            camera_pos.x = character_transform.translation.x;
            camera_pos.y = character_transform.translation.y;
        }

        // If there is a spawned map layer we can find, we want to make sure the camera doesn't show
        // outside the edges of the map. ( we don't really care which layer because they should all
        // be the same size )
        let mut has_constrained_camera = false;
        for (layer, mut layer_visible, layer_image_handle, layer_transform) in map_layers.iter_mut()
        {
            let layer_pos = layer_transform.translation;

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
                    let layer_max_x = layer_pos.x + layer_width as f32;
                    let layer_min_y = layer_pos.y;
                    let layer_max_y = layer_pos.y + layer_height as f32;

                    // Get the camera target size
                    let camera_size = camera.get_target_sizes(windows.get_primary().unwrap()).low;
                    let camera_min_x = camera_pos.x - camera_size.x as f32 / 2.;
                    let camera_max_x =
                        (camera_pos.x - camera_size.x as f32 / 2.) + camera_size.x as f32;
                    let camera_min_y = camera_pos.y - camera_size.y as f32 / 2.;
                    let camera_max_y =
                        (camera_pos.y - camera_size.y as f32 / 2.) + camera_size.y as f32;

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

/// Enumerates different states the entrance transition logic can be in
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum EntranceStatus {
    /// Totally outside of any entrance
    Outside,
    /// Teleporting and waiting to reach the other entrance
    TeleportingTo {
        entrance_id: String,
        level_id: String,
    },
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
    maps: Query<&Handle<LdtkMap>>,
    map_assets: Res<Assets<LdtkMap>>,
    mut current_level: ResMut<CurrentLevel>,
    mut current_level_music: Option<ResMut<CurrentLevelMusic>>,
    mut sound_controller: SoundController,
    asset_server: Res<AssetServer>,
    entrances: Query<&Entrance>,
    mut characters: Query<&mut Transform, With<Handle<Character>>>,
    mut collision_events: EventReader<CollisionEvent>,
) {
    // Get the map
    let map = if let Ok(map) = maps.single() {
        if let Some(map) = map_assets.get(map) {
            map
        } else {
            return;
        }
    } else {
        return;
    };

    // Check characters colliding with entrances
    for event in collision_events.iter() {
        let (ent1, ent2) = event.collision_shape_entities();

        // Skip non-character collisions
        let mut character_transform = if let Ok(character) = characters.get_mut(ent1) {
            character
        } else if let Ok(character) = characters.get_mut(ent2) {
            character
        } else {
            continue;
        };

        // Get the entrance of the collision or skip this event
        let entrance = if let Ok(entrance) = entrances.get(ent1).or_else(|_| entrances.get(ent2)) {
            entrance
        } else {
            continue;
        };

        match &*status {
            // If we are in the middle of teleporting to an entrance
            EntranceStatus::TeleportingTo {
                entrance_id: target_entrance_id,
                level_id: target_level_id,
            } => {
                // If we have stopped collided with the entrance we are trying to teleport to
                if entrance.id == target_entrance_id.as_str()
                    && &entrance.level == target_level_id
                    && event.is_stopped()
                {
                    // Transition into an awaiting leave state
                    *status = EntranceStatus::Outside;
                }

                // And skip all tasks below
                return;
            }

            // We are outside of an entrance and walking into it for the first time
            EntranceStatus::Outside if event.is_started() => {
                // Move to teleporting state and continue on with the logic below to
                // teleport to the target entrance
                *status = EntranceStatus::TeleportingTo {
                    level_id: entrance.to_level.clone(),
                    entrance_id: entrance.spawn_at.clone(),
                };
            }
            EntranceStatus::Outside => (),
        }

        // Get the level that we will be teleporting to
        let to_level = map
            .project
            .levels
            .iter()
            .find(|x| x.identifier == entrance.to_level)
            .unwrap_or_else(|| {
                panic!(
                    "Level `{}` does not exist. Could not teleport there.",
                    entrance.to_level
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
                        && x.field_instances
                            .iter()
                            .any(|x| x.__identifier == "id" && x.__value == entrance.spawn_at)
                })
            })
            .unwrap_or_else(|| {
                panic!(
                    "Could not find entrance `{}` in level `{}` to teleport to",
                    entrance.spawn_at, entrance.to_level
                )
            });

        // Set the current level to the new level
        *current_level = CurrentLevel(entrance.to_level.clone());

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
                        **current_music = play_music(&mut sound_controller, new_sound_data);
                    }

                // If there is no music already playing, just play the new music
                } else {
                    commands.insert_resource(play_music(&mut sound_controller, new_sound_data));
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
                    .strip_prefix('#')
                    .expect("Invalid background color"),
            )
            .expect("Invalid background color");

            camera.background_color = Color::from_rgba8(decoded[0], decoded[1], decoded[2], 1);
        }

        // Move the character to the other entrance
        *character_transform = Transform::from_xyz(
            // FIXME: We subtract 0.1 pixels to push the sprite very slightly to the left because
            // there were issues when teleporting where we were just enough to the right that we
            // could somehow go through the first block of doorpost.
            //
            // Not sure why, but this is the easiest place to fix for now.
            to_level.world_x as f32 + to_entrance.px[0] as f32 + to_entrance.width as f32 / 2.
                - 0.1,
            to_level.world_y as f32 + to_entrance.px[1] as f32 + to_entrance.height as f32 / 2.
                - 0.1,
            to_level
                .layer_instances
                .as_ref()
                .expect("Level does not have any layers")
                .len() as f32
                * 2.,
        );
    }
}

pub struct EnemyPathfindingDebugViz {
    pub enemy_ent: Entity,
}

pub fn enemy_follow_player(
    mut commands: Commands,
    mut enemies: Query<(Entity, &Transform, &mut Velocity, &Enemy)>,
    characters: Query<(Entity, &Transform), With<Handle<Character>>>,
    maps: Query<&LdtkMapLevelNavigationMeshes, With<Handle<LdtkMap>>>,
    enemy_pathfinding_debug_vizes: Query<Entity, With<EnemyPathfindingDebugViz>>,
    current_level: Option<Res<CurrentLevel>>,
    physics_world: PhysicsWorld,
    game_info: Res<GameInfo>,
) {
    const ENEMY_SPEED: f32 = 40.;

    let current_level = if let Some(level) = current_level {
        level
    } else {
        return;
    };

    let (character_ent, character_transform) = if let Ok(character) = characters.single() {
        character
    } else {
        return;
    };

    // For the sake of pathfinding we set the z position to 0.
    let character_pos = character_transform.translation.truncate().extend(0.);

    let map_nav_meshes = if let Ok(meshes) = maps.single() {
        meshes
    } else {
        return;
    };

    let mesh = if let Some(mesh) = map_nav_meshes.get(&current_level.0) {
        mesh
    } else {
        return;
    };

    'enemy: for (enemy_ent, enemy_transform, mut enemy_velocity, enemy) in enemies.iter_mut() {
        let enemy_pos = enemy_transform.translation.truncate().extend(0.);

        // Skip the enemy if he is not from the current level
        if enemy.level != current_level.0 {
            continue;
        }

        if game_info.debug_rendering.navmesh {
            // Clean up navigation debug viz from previous frame
            for entity in enemy_pathfinding_debug_vizes.iter() {
                commands.entity(entity).despawn();
            }
        }

        // Try to plot a path straight to the player
        let straight_path = if let Some(collision) = physics_world.shape_cast_with_filter(
            &CollisionShape::Sphere { radius: 8. },
            enemy_pos,
            Quat::default(),
            character_pos - enemy_pos,
            CollisionLayers::default(),
            |entity| entity != enemy_ent,
        ) {
            if collision.entity == character_ent {
                // Spawn debug rendering if enabled
                if game_info.debug_rendering.navmesh {
                    commands
                        .spawn_bundle(ShapeBundle {
                            shape: Shape::line_segment(
                                [
                                    epaint::pos2(enemy_pos.x, enemy_pos.y),
                                    epaint::pos2(character_pos.x, character_pos.y),
                                ],
                                (2., epaint::Color32::RED),
                            ),
                            transform: Transform::from_xyz(0., 0., 1024.),
                            ..Default::default()
                        })
                        .insert(EnemyPathfindingDebugViz { enemy_ent });
                }

                Some(vec![character_pos.into_nav()])
            } else {
                None
            }
        } else {
            None
        };

        // Use navigation mesh to plot a path to the character if the straight path doesn't work
        if let Some(path) = straight_path.or_else(|| {
            mesh.find_path(
                enemy_pos.into_nav(),
                character_pos.into_nav(),
                navmesh::NavQuery::Accuracy,
                navmesh::NavPathMode::Accuracy,
            )
        }) {
            // Display debug visualization if enabled
            if game_info.debug_rendering.navmesh {
                for (v1, v2) in path.iter().tuple_windows() {
                    commands
                        .spawn_bundle(ShapeBundle {
                            shape: Shape::line_segment(
                                [epaint::pos2(v1.x, v1.y), epaint::pos2(v2.x, v2.y)],
                                (2., epaint::Color32::GREEN),
                            ),
                            transform: Transform::from_xyz(0., 0., 1024.),
                            ..Default::default()
                        })
                        .insert(EnemyPathfindingDebugViz { enemy_ent });
                }
            }

            for node in path {
                let vel = (node.into_bevy() - enemy_pos).normalize_or_zero() * ENEMY_SPEED;
                if vel.length() > 0.5 {
                    *enemy_velocity = vel.into();
                    break 'enemy;
                }
            }

            *enemy_velocity = Velocity::default()
        } else {
            *enemy_velocity = Velocity::default()
        }
    }
}

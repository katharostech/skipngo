use bevy::{prelude::*, reflect::TypeUuid};
use bevy_retro::*;
use serde::Deserialize;

pub mod loader;
pub mod systems;

use loader::CharacterLoader;

pub struct CharacterPlugin;

#[derive(Eq, PartialEq, StageLabel, Clone, Hash, Debug)]
pub enum CharacterStages {
    Game,
    CameraFollow,
}

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.add_asset::<Character>()
            .init_asset_loader::<CharacterLoader>()
            .add_stage(CharacterStages::Game, SystemStage::parallel())
            .add_stage_after(
                CharacterStages::Game,
                CharacterStages::CameraFollow,
                SystemStage::parallel(),
            )
            .add_system_to_stage(
                CharacterStages::CameraFollow,
                systems::camera_follow.system(),
            )
            .add_system_to_stage(
                CharacterStages::Game,
                systems::finish_spawning_character.system(),
            )
            .add_system_to_stage(CharacterStages::Game, systems::control_character.system())
            .add_system_to_stage(
                CharacterStages::Game,
                systems::animate_sprite_system.system(),
            );
    }
}

#[derive(TypeUuid)]
#[uuid = "9fa5febb-1a7b-4864-9534-2d5df8df82f4"]
pub struct Character {
    pub name: String,
    pub sprite_sheet_info: CharacterSpriteSheet,
    pub actions: CharacterActions,
    pub walk_speed: u32,
    pub sprite_image: Handle<Image>,
    pub sprite_sheet: Handle<SpriteSheet>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct CharacterYmlData {
    pub name: String,
    pub sprite_sheet: CharacterSpriteSheet,
    pub actions: CharacterActions,
    pub walk_speed: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CharacterSpriteSheet {
    pub path: String,
    pub grid_size: (u32, u32),
    pub tiles: (u32, u32),
}

#[derive(Deserialize)]
pub struct CharacterActions {
    pub walk: CharacterAction,
    pub idle: CharacterAction,
}

#[derive(Deserialize)]
pub struct CharacterAction {
    pub sound: Option<String>,
    pub animations: CharacterAnimations,
}

#[derive(Deserialize)]
pub struct CharacterAnimations {
    pub up: CharacterAnimation,
    pub down: CharacterAnimation,
    pub right: CharacterAnimation,
    pub left: CharacterAnimation,
}

#[derive(Deserialize)]
pub struct CharacterAnimation {
    #[serde(default)]
    pub flip: bool,
    pub frames: Vec<u32>,
}

#[derive(Clone)]
pub struct CharacterCurrentTilesetIndex(pub u32);

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum CurrentCharacterAction {
    Walk,
    Idle,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum CurrentCharacterDirection {
    Up,
    Down,
    Left,
    Right,
}

/// A bundle for spawning a character
// Copied mostly from the SpriteSheetBundle bundle
#[derive(Bundle)]
pub struct CharacterBundle {
    pub character: Handle<Character>,
    pub current_action: CurrentCharacterAction,
    pub current_direction: CurrentCharacterDirection,
    pub current_tileset_index: CharacterCurrentTilesetIndex,
    pub animation_frame_timer: Timer,
    #[bundle]
    pub sprite_bundle: SpriteBundle,
    pub sprite_sheet: Handle<SpriteSheet>,
}

impl Default for CharacterBundle {
    fn default() -> Self {
        Self {
            character: Default::default(),
            current_tileset_index: CharacterCurrentTilesetIndex(0),
            current_action: CurrentCharacterAction::Idle,
            current_direction: CurrentCharacterDirection::Down,
            animation_frame_timer: Timer::from_seconds(0.1, true),
            sprite_bundle: Default::default(),
            sprite_sheet: Default::default(),
        }
    }
}

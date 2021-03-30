use bevy::{core::FixedTimestep, prelude::*, reflect::TypeUuid};
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
            .add_stage(
                CharacterStages::Game,
                SystemStage::parallel().with_run_criteria(FixedTimestep::step(0.012)),
            )
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
    pub collision_shape: Handle<Image>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct CharacterYmlData {
    pub name: String,
    pub sprite_sheet: CharacterSpriteSheet,
    pub actions: CharacterActions,
    pub walk_speed: u32,
    pub collision_shape: String,
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
pub enum CharacterStateAction {
    Walk,
    Idle,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum CharacterStateDirection {
    Up,
    Down,
    Left,
    Right,
}

pub struct CharacterState {
    pub action: CharacterStateAction,
    pub direction: CharacterStateDirection,
    pub tileset_index: u32,
    pub animation_frame: u16,
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            action: CharacterStateAction::Idle,
            direction: CharacterStateDirection::Down,
            tileset_index: 0,
            animation_frame: 0,
        }
    }
}

/// A bundle for spawning a character
// Copied mostly from the SpriteSheetBundle bundle
#[derive(Bundle, Default)]
pub struct CharacterBundle {
    pub character: Handle<Character>,
    pub state: CharacterState,
    #[bundle]
    pub sprite_bundle: SpriteBundle,
    pub sprite_sheet: Handle<SpriteSheet>,
}

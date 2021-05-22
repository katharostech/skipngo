use serde::{Deserialize, Serialize};

use bevy::{
    asset::{AssetLoader, AssetPath, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
};
use bevy_retro::prelude::{
    ui::raui::prelude::{Prefab, PropsData},
    *,
};

use super::*;

/// Add all assets and their loaders to the Bevy app
pub fn add_assets(app: &mut AppBuilder) {
    app.add_asset::<GameInfo>()
        .add_asset_loader(GameInfoLoader::default())
        .add_asset::<Character>()
        .add_asset_loader(CharacterLoader::default());
}

#[derive(thiserror::Error, Debug)]
pub enum AssetLoaderError {
    #[error("Could not parse game info: {0}")]
    DeserializationError(#[from] serde_yaml::Error),
}

/// The core info about the game provided by the .game.yaml file
#[derive(PropsData, Deserialize, TypeUuid, Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
#[uuid = "c19826f5-e474-4ad0-a0fc-c24f144a1b79"]
pub struct GameInfo {
    /// The title of the game
    pub title: String,
    /// The path to the game map
    pub map: String,
    /// The name of the level to start the game in
    pub game_start_level: String,
    /// The path to the character that you will play as
    pub player_character: String,
    /// The camera size
    #[serde(with = "CameraSizeDef")]
    pub camera_size: CameraSize,
    /// Splash screen configuration
    pub splash_screen: SplashScreen,
    pub ui_theme: UiTheme,
}

/// Splash screen settings
#[derive(Deserialize, Clone, Serialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct SplashScreen {
    pub splash_image: SplashImage,
    pub background_level: String,
    pub music: String,
}

/// The splash image to use for the game
#[derive(Deserialize, Clone, Serialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct SplashImage {
    pub path: String,
    pub size: UVec2,
}

/// The definition of the UI theme
#[derive(Deserialize, Clone, Serialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct UiTheme {
    pub default_font: String,
    pub panel: UiBoxImage,
    pub button_up: UiBoxImage,
    pub button_down: UiBoxImage,
    pub checkbox: UiCheckboxImages,
}

/// The theme for a checkbox
#[derive(Deserialize, Clone, Serialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct UiCheckboxImages {
    pub checked: String,
    pub unchecked: String,
}

// Settings for a 9-patch UI image
#[derive(Deserialize, Clone, Serialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct UiBoxImage {
    pub image: String,
    pub border_size: u32,
    #[serde(default)]
    pub only_frame: bool,
}

/// A serializable version of the bevy_retro [`CameraSize`]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(remote = "CameraSize")]
#[serde(rename_all = "kebab-case")]
pub enum CameraSizeDef {
    FixedHeight(u32),
    FixedWidth(u32),
    LetterBoxed { width: u32, height: u32 },
}

//
// Game info loader
//

#[derive(Default)]
pub struct GameInfoLoader;

impl AssetLoader for GameInfoLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move { Ok(load_game_info(bytes, load_context).await?) })
    }

    fn extensions(&self) -> &[&str] {
        &["game.yml", "game.yaml"]
    }
}

async fn load_game_info<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut bevy::asset::LoadContext<'b>,
) -> Result<(), AssetLoaderError> {
    let game_info: GameInfo = serde_yaml::from_slice(bytes)?;
    load_context.set_default_asset(LoadedAsset::new(game_info));
    Ok(())
}

//
// Character loader
//

#[derive(Default)]
pub struct CharacterLoader;

impl AssetLoader for CharacterLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move { Ok(load_character(bytes, load_context).await?) })
    }

    fn extensions(&self) -> &[&str] {
        &["character.yml", "character.yaml"]
    }
}

async fn load_character<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut bevy::asset::LoadContext<'b>,
) -> Result<(), AssetLoaderError> {
    // Load the character
    let character: CharacterYmlData = serde_yaml::from_slice(bytes)?;

    // Get the path to the tileset image asset
    let atlas_file_path = load_context
        .path()
        .parent()
        .unwrap()
        .join(&character.sprite_sheet.path);

    // Get the path to the tileset image asset
    let collision_file_path = load_context
        .path()
        .parent()
        .unwrap()
        .join(&character.collision_shape);

    // Convert that to an asset path for the texture
    let sprite_image_path = AssetPath::new(atlas_file_path, None);
    // Get the texture handle
    let sprite_image_handle: Handle<Image> = load_context.get_handle(sprite_image_path.clone());
    // Add it as a labled asset
    let sprite_sheet_handle = load_context.set_labeled_asset(
        "SpriteSheet",
        LoadedAsset::new(SpriteSheet {
            grid_size: UVec2::splat(character.sprite_sheet.grid_size.0),
            tile_index: 0,
        }),
    );

    // Convert that to an asset path for the texture
    let collision_image_path = AssetPath::new(collision_file_path, None);
    // Get the texture handle
    let collision_image_handle: Handle<Image> =
        load_context.get_handle(collision_image_path.clone());

    // Set the character asset
    load_context.set_default_asset(
        LoadedAsset::new(Character {
            name: character.name,
            sprite_sheet_info: character.sprite_sheet,
            collision_shape: collision_image_handle,
            actions: character.actions,
            walk_speed: character.walk_speed,
            sprite_image: sprite_image_handle,
            sprite_sheet: sprite_sheet_handle,
        })
        .with_dependency(collision_image_path)
        .with_dependency(sprite_image_path),
    );

    Ok(())
}

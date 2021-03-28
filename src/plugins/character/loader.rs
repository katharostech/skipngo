use bevy::{
    asset::{AssetLoader, AssetPath, LoadedAsset},
    math::UVec2,
    prelude::Handle,
};
use bevy_retro::*;

use super::{Character, CharacterYmlData};

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

#[derive(thiserror::Error, Debug)]
enum CharacterLoaderError {
    #[error("Could not parse character file: {0}")]
    DeserializationError(#[from] serde_yaml::Error),
}

async fn load_character<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut bevy::asset::LoadContext<'b>,
) -> Result<(), CharacterLoaderError> {
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
    let sprite_image_path = AssetPath::new(atlas_file_path.clone(), None);
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
    let collision_image_path = AssetPath::new(collision_file_path.clone(), None);
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

use bevy::{
    asset::{AssetLoader, LoadedAsset},
    reflect::TypeUuid,
};
use bevy_retro::CameraSize;
use serde::Deserialize;

#[derive(thiserror::Error, Debug)]
pub enum AssetLoaderError {
    #[error("Could not parse game info: {0}")]
    DeserializationError(#[from] serde_yaml::Error),
}

#[derive(Deserialize, TypeUuid, Clone)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
#[uuid = "c19826f5-e474-4ad0-a0fc-c24f144a1b79"]
pub struct GameInfo {
    /// The title of the game
    pub title: String,
    /// The path to the game map
    pub map: String,
    /// The name of the level to start the game in
    pub starting_level: String,
    /// The path to the character that you will play as
    pub player_character: String,
    /// The camera size
    #[serde(with = "CameraSizeDef")]
    pub camera_size: CameraSize,
}

#[derive(Deserialize)]
#[serde(remote = "CameraSize")]
#[serde(rename_all = "kebab-case")]
pub enum CameraSizeDef {
    FixedHeight(u32),
    FixedWidth(u32),
    LetterBoxed { width: u32, height: u32 },
}

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

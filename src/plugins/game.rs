use crate::{plugins::character::*, EngineConfig};
use bevy::prelude::*;
use bevy_retro::*;
use bevy_retro_ldtk::*;

/// The game states
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameState {
    /// The game is loading initial game data and spawning the initial items
    LoadingGameInfo,
    /// The game is loading the map and spawning the player
    LoadingMap,
    /// The game is running!
    Running,
}

/// Plugin responsible for booting and handling core game stuff
pub struct GamePlugin;

/// The current map level the player is in
#[derive(Clone)]
pub struct CurrentLevel(pub String);
impl_deref!(CurrentLevel, String);

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            // Add game info asset and loader
            .add_asset::<GameInfo>()
            .add_asset_loader(GameInfoLoader::default())
            // Add game state
            .add_state(GameState::LoadingGameInfo)
            // Add the system to wait for game initialization
            .add_system_set(
                SystemSet::on_update(GameState::LoadingGameInfo).with_system(await_init.system()),
            )
            // Add system to spawn players when the map loads
            .add_system_set(
                SystemSet::on_update(GameState::LoadingMap).with_system(spawn_players.system()),
            );
    }
}

/// Wait for the game info to load and spawn the map
fn await_init(
    mut commands: Commands,
    game_info_assets: Res<Assets<GameInfo>>,
    asset_server: Res<AssetServer>,
    mut state: ResMut<State<GameState>>,
    engine_config: Res<EngineConfig>,
) {
    let game_info: Handle<GameInfo> = asset_server.load("default.game.yaml");

    // Spawn the map once the game info loads
    if let Some(game_info) = game_info_assets.get(game_info) {
        // Spawn the camera
        commands.spawn().insert_bundle(CameraBundle {
            camera: Camera {
                size: CameraSize::FixedHeight(game_info.viewport_height),
                custom_shader: if engine_config.enable_crt {
                    Some(CrtShader::default().get_shader())
                } else {
                    None
                },
                pixel_aspect_ratio: engine_config.pixel_aspect_ratio,
                ..Default::default()
            },
            ..Default::default()
        });

        // Spawn the map
        commands.spawn().insert_bundle(LdtkMapBundle {
            map: asset_server.load(game_info.map.as_str()),
            ..Default::default()
        });

        // Add the game info as a resource
        commands.insert_resource(game_info.clone());
        // Add the current level resource
        commands.insert_resource(CurrentLevel(game_info.starting_level.clone()));

        // Transition to running state
        state.set_push(GameState::LoadingMap).unwrap();
    }
}

fn spawn_players(
    mut commands: Commands,
    map_query: Query<&Handle<LdtkMap>>,
    map_assets: Res<Assets<LdtkMap>>,
    mut state: ResMut<State<GameState>>,
    asset_server: Res<AssetServer>,
    game_info: Res<GameInfo>,
    current_level: Res<CurrentLevel>,
) {
    for map_handle in map_query.iter() {
        if let Some(map) = map_assets.get(map_handle) {
            let level = &map
                .project
                .levels
                .iter()
                .filter(|x| x.identifier == **current_level)
                .next()
                .unwrap();

            let entities_layer = level
                .layer_instances
                .as_ref()
                .unwrap()
                .iter()
                .filter(|&x| x.entity_instances.len() != 0)
                .next()
                .unwrap();

            let player_start = entities_layer
                .entity_instances
                .iter()
                .filter(|x| {
                    x.__identifier == "SpawnPoint"
                        && x.field_instances
                            .iter()
                            .filter(|x| x.__identifier == "name" && x.__value == "PlayerStart")
                            .next()
                            .is_some()
                })
                .next()
                .unwrap();

            let character_handle: Handle<Character> =
                asset_server.load(game_info.player_character.as_str());

            let character_image_handle =
                asset_server.load(format!("{}#atlas", game_info.player_character).as_str());
            let character_spritesheet_handle =
                asset_server.load(format!("{}#spritesheet", game_info.player_character).as_str());

            // Layers are 2 units away from each-other, so put the player at the top
            let player_z = level.layer_instances.as_ref().unwrap().len() as i32 * 2;

            commands.spawn().insert_bundle(CharacterBundle {
                character: character_handle,
                sprite_bundle: SpriteBundle {
                    image: character_image_handle,
                    position: Position::new(
                        player_start.px[0] + level.world_x,
                        player_start.px[1] + level.world_y,
                        player_z,
                    ),
                    ..Default::default()
                },
                sprite_sheet: character_spritesheet_handle,
                ..Default::default()
            });

            // Go to the running state
            state.set_push(GameState::Running).unwrap();
        }
    }
}

use asset::*;

mod asset {
    use bevy::{
        asset::{AssetLoader, LoadedAsset},
        reflect::TypeUuid,
    };
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize, TypeUuid, Clone)]
    #[serde(deny_unknown_fields)]
    #[serde(rename_all = "kebab-case")]
    #[uuid = "c19826f5-e474-4ad0-a0fc-c24f144a1b79"]
    pub struct GameInfo {
        pub map: String,
        pub starting_level: String,
        pub player_character: String,
        pub viewport_height: u32,
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

    #[derive(thiserror::Error, Debug)]
    enum CharacterLoaderError {
        #[error("Could not parse game info: {0}")]
        DeserializationError(#[from] serde_yaml::Error),
    }

    async fn load_game_info<'a, 'b>(
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext<'b>,
    ) -> Result<(), CharacterLoaderError> {
        let game_info: GameInfo = serde_yaml::from_slice(bytes)?;
        load_context.set_default_asset(LoadedAsset::new(game_info));
        Ok(())
    }
}

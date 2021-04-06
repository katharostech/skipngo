use crate::{plugins::character::*, EngineConfig};
use bevy::prelude::*;
use bevy_retro::*;
use bevy_retro_ldtk::*;

use assets::*;
mod assets;

/// The game states
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    #[cfg(not(wasm))] mut windows: ResMut<Windows>,
) {
    let game_info: Handle<GameInfo> = asset_server.load("default.game.yaml");

    // Spawn the map once the game info loads
    if let Some(game_info) = game_info_assets.get(game_info) {
        // Spawn the camera
        commands.spawn().insert_bundle(CameraBundle {
            camera: Camera {
                size: game_info.camera_size.clone(),
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

        // Update the window title
        #[cfg(not(wasm))]
        windows
            .get_primary_mut()
            .unwrap()
            .set_title(game_info.title.clone());
        #[cfg(wasm)]
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .set_title(&game_info.title);

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
        state.push(GameState::LoadingMap).unwrap();
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
                .find(|x| x.identifier == **current_level)
                .unwrap();

            let entities_layer = level
                .layer_instances
                .as_ref()
                .unwrap()
                .iter()
                .find(|&x| !x.entity_instances.is_empty())
                .unwrap();

            let player_start = entities_layer
                .entity_instances
                .iter()
                .find(|x| {
                    x.__identifier == "SpawnPoint"
                        && x.field_instances
                            .iter()
                            .any(|x| x.__identifier == "name" && x.__value == "PlayerStart")
                })
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
            state.push(GameState::Running).unwrap();
        }
    }
}

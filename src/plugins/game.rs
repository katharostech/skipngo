use bevy::prelude::*;
use bevy_retro::*;

/// The game states
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameState {
    /// The game is initializing, loading initial data necessary to start up
    Initializing,
    /// The game is playing
    Running,
}

/// Plugin responsible for booting and handling core game stuff
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            // Add game state resource
            .add_state(GameState::Initializing)
            // Add game init sysem
            .add_system_set(SystemSet::on_enter(GameState::Initializing).with_system(init.system()))
            // Add the system to wait for game initialization
            .add_system_set(
                SystemSet::on_update(GameState::Initializing).with_system(await_game_init.system()),
            );
    }
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    let sprite_image = asset_server.load("sprite.png");

    commands
        // spawn camera
        .spawn(CameraBundle {
            camera: Camera {
                size: CameraSize::FixedHeight(50),
                ..Default::default()
            },
            ..Default::default()
        })
        // spawn sprite
        .spawn(SpriteBundle {
            image: sprite_image,
            ..Default::default()
        });
}

fn await_game_init() {}

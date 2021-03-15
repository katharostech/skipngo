use bevy::prelude::*;

/// The main ECS stage for the game
#[derive(StageLabel, Clone, Debug, Hash, PartialEq, Eq)]
pub enum GameStage {
    Update,
}

/// The game states
#[derive(Clone, Debug)]
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
            .insert_resource(State::new(GameState::Initializing))
            // Add GameUpdate stage
            .add_stage_after(
                CoreStage::Update,
                GameStage::Update,
                StateStage::<GameState>::default(),
            )
            // Add game init sysem
            .on_state_enter(GameStage::Update, GameState::Initializing, init.system())
            // Add the system to wait for game initialization
            .on_state_update(
                GameStage::Update,
                GameState::Running,
                await_game_init.system(),
            );
    }
}

fn init() {}

fn await_game_init() {}

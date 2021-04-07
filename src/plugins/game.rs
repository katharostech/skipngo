use crate::EngineConfig;
use bevy::prelude::*;
use bevy_retro::*;

use assets::*;
mod assets;

mod systems;
use systems::*;

mod components;
use components::*;

mod events;
use events::*;

/// Plugin responsible for booting and handling core game stuff
pub struct GamePlugin;

/// The current map level the player is in
#[derive(Clone)]
pub struct CurrentLevel(pub String);
impl_deref!(CurrentLevel, String);

impl Plugin for GamePlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Add assets
        add_assets(app);

        // Add events
        add_events(app);

        // Add systems
        add_systems(app);
    }
}

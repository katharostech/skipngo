use bevy::prelude::*;

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

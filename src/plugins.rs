use bevy::prelude::*;

pub mod game;
use game::*;

pub mod character;
use character::*;

pub struct SkipnGoPlugins;

impl PluginGroup for SkipnGoPlugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        group.add(GamePlugin).add(CharacterPlugin);
    }
}

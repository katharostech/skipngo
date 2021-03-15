use bevy::prelude::*;
use bevy_retro::*;
use bevy_retro_ldtk::*;

pub mod plugins;

pub fn run() {
    App::build()
        .add_plugins(RetroPlugins)
        .add_plugin(LdtkPlugin)
        .add_plugins(plugins::SkipnGoPlugins)
        .run();
}
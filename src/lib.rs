use bevy::{asset::AssetServerSettings, prelude::*};
use bevy_retro::*;
use bevy_retro_ldtk::*;

#[cfg(not(wasm))]
use structopt::StructOpt;

pub mod plugins;

#[cfg(wasm)]
pub mod wasm_utils;

pub fn run() {
    let log_config = get_log_config();
    let engine_config = EngineConfig::get_config();

    App::build()
        // Configure the asset directory
        .insert_resource(AssetServerSettings {
            asset_folder: engine_config.asset_path,
        })
        // Add the logging config
        .insert_resource(log_config)
        // Install Bevy Retro
        .add_plugins(RetroPlugins)
        // Install Bevy Retro LDtk
        .add_plugin(LdtkPlugin)
        // Add our SkipnGo plugins
        .add_plugins(plugins::SkipnGoPlugins)
        // Start the game!
        .run();
}

#[cfg(not(wasm))]
use bevy::log::LogSettings;
/// Get logging config for desktop
#[cfg(not(wasm))]
fn get_log_config() -> LogSettings {
    // Default settings are fine, just use RUST_LOG env to tweak
    Default::default()
}

#[cfg(wasm)]
use crate::wasm_utils::parse_url_query_string;
#[cfg(wasm)]
use wasm_utils::get_log_config;

/// Game configuration provided exertnally i.e. commandline/URL query string
#[derive(Debug)]
#[cfg_attr(not(wasm), derive(StructOpt))]
#[cfg_attr(
    not(wasm),
    structopt(
        name = "Skip'n Go",
        about = "A game engine to help you skip the hard stuff and go make a game!",
        setting(structopt::clap::AppSettings::ColoredHelp)
    )
)]
pub struct EngineConfig {
    /// The path to the game asset directory
    #[cfg_attr(
        not(wasm),
        structopt(short = "a", long = "asset_dir", default_value = "assets", parse(from_str = parse_asset_path))
    )]
    asset_path: String,
}

#[cfg(not(wasm))]
fn parse_asset_path(s: &str) -> String {
    std::env::current_dir()
        .unwrap()
        .join(s)
        .to_str()
        .unwrap()
        .to_owned()
}

#[cfg(not(wasm))]
impl EngineConfig {
    pub fn get_config() -> Self {
        EngineConfig::from_args()
    }
}

#[cfg(wasm)]
impl EngineConfig {
    pub fn get_config() -> Self {
        use web_sys::*;

        // Get the query string
        let asset_url: String = window().unwrap().location().search().unwrap();

        Self {
            asset_path: parse_url_query_string(&asset_url, "asset_url")
                .map(String::from)
                .unwrap_or("/assets".into()),
        }
    }
}

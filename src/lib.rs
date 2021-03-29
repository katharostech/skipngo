use bevy::{
    asset::AssetServerSettings,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_retro::*;
use bevy_retro_ldtk::*;

#[cfg(not(wasm))]
use structopt::StructOpt;

pub mod plugins;

#[cfg(wasm)]
pub mod wasm_utils;

pub fn run() {
    // Get logging config
    let log_config = get_log_config();
    // Get the engine config
    let engine_config = EngineConfig::get_config();

    // Create an app builder
    let mut builder = App::build();

    // Build the app
    builder
        .insert_resource(WindowDescriptor {
            title: "Skip'n Go".into(),
            ..Default::default()
        })
        // Configure the asset directory
        .insert_resource(AssetServerSettings {
            asset_folder: engine_config.asset_path.clone(),
        })
        .insert_resource(engine_config.clone())
        // Add the logging config
        .insert_resource(log_config)
        // Install Bevy Retro
        .add_plugins(RetroPlugins)
        // Install Bevy Retro LDtk
        .add_plugin(LdtkPlugin)
        // Add our SkipnGo plugins
        .add_plugins(plugins::SkipnGoPlugins);

    if engine_config.frame_time_diagnostics {
        builder
            .add_plugin(FrameTimeDiagnosticsPlugin)
            .add_plugin(LogDiagnosticsPlugin::default());
    }

    // Start the game!
    builder.run();
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

/// Game configuration provided externally i.e. commandline/URL query string
#[derive(Debug, Clone)]
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
        structopt(short = "a", long = "asset-dir", default_value = "assets", parse(from_str = parse_asset_path))
    )]
    asset_path: String,
    /// Enable frame time diagnostics to the console
    #[cfg_attr(not(wasm), structopt(short = "d", long = "frame-time-diagnostics"))]
    frame_time_diagnostics: bool,
    /// Enable CRT screen filter
    #[cfg_attr(not(wasm), structopt(short = "C", long = "enable-crt"))]
    enable_crt: bool,
    /// Set the pixel aspect ratio
    #[cfg_attr(
        not(wasm),
        structopt(short = "A", long = "pixel-aspect-ratio", default_value = "1.0")
    )]
    pixel_aspect_ratio: f32,
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
            frame_time_diagnostics: parse_url_query_string(&asset_url, "frame_time_diagnostics")
                .map(|x| x == "true")
                .unwrap_or(false),
            enable_crt: parse_url_query_string(&asset_url, "enable_crt")
                .map(|x| x == "true")
                .unwrap_or(false),
            pixel_aspect_ratio: parse_url_query_string(&asset_url, "pixel_aspect_ratio")
                .map(|x| x.parse().expect("Pixel aspect ratio not a number"))
                .unwrap_or(1.0),
        }
    }
}

use bevy::{
    core::FixedTimestep,
    ecs::{component::ComponentDescriptor, schedule::ShouldRun},
    prelude::*,
    utils::HashSet,
    window::WindowMode,
};
use bevy_retro::{prelude::*, ui::raui::prelude::widget};
use kira::parameter::tween::Tween;

use super::*;

mod game_init;
mod pause_menu;

mod gameplay;
use gameplay::{
    animate_sprites, camera_follow_system, change_level, control_character,
    finish_spawning_character, keyboard_control_input, spawn_hud, touch_control_input,
};

/// The game states
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameState {
    /// The game is loading initial game data, spawning the map, and displaying the start menu
    Init,
    /// The game is showing the start menu
    StartMenu,
    /// The game is loading the map and spawning the player
    LoadingGame,
    /// The game is playing the main game
    Playing,
    /// The game is paused during the main game
    Paused,
}

pub fn add_systems(app: &mut AppBuilder) {
    debug!("Configuring game systems");

    app
        // Use sparse storage for marker component
        .register_component(ComponentDescriptor::new::<gameplay::CharacterLoaded>(
            bevy::ecs::component::StorageType::SparseSet,
        ))
        .add_system(switch_fullscreen.system())
        // Game init state
        .add_state(GameState::Init)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(
                    (|state: Res<State<GameState>>| {
                        if state.current() == &GameState::Init {
                            ShouldRun::Yes
                        } else {
                            ShouldRun::No
                        }
                    })
                    .system(),
                )
                .with_system(game_init::await_init.system()),
        )
        // Game start menu state
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(
                    (|state: Res<State<GameState>>| {
                        if state.current() == &GameState::StartMenu {
                            ShouldRun::Yes
                        } else {
                            ShouldRun::No
                        }
                    })
                    .system(),
                )
                .with_system(game_init::setup_start_menu.system()),
        )
        // Loading main game state
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(
                    (|state: Res<State<GameState>>| {
                        if state.current() == &GameState::LoadingGame {
                            ShouldRun::Yes
                        } else {
                            ShouldRun::No
                        }
                    })
                    .system(),
                )
                .with_system(game_init::spawn_player_and_setup_level.system()),
        )
        // Main gameplay
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(
                    FixedTimestep::step(0.012).chain(
                        // Workaround: https://github.com/bevyengine/bevy/issues/1839
                        (|In(input): In<ShouldRun>, state: Res<State<GameState>>| {
                            if state.current() == &GameState::Playing {
                                input
                            } else {
                                ShouldRun::No
                            }
                        })
                        .system(),
                    ),
                )
                .with_system(spawn_hud.system())
                .with_system(finish_spawning_character.system().label("finish_spawn"))
                .with_system(
                    touch_control_input
                        .system()
                        .label("input")
                        .after("finish_spawn"),
                )
                .with_system(
                    keyboard_control_input
                        .system()
                        .label("input")
                        .after("finish_spawn"),
                )
                .with_system(
                    control_character
                        .system()
                        .label("control_character")
                        .after("input"),
                )
                .with_system(animate_sprites.system().after("control_character"))
                .with_system(camera_follow_system.system().after("control_character"))
                .with_system(change_level.system().after("control_character")),
        )
        //
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(
                    (|state: Res<State<GameState>>| {
                        if state.current() == &GameState::Paused {
                            ShouldRun::Yes
                        } else {
                            ShouldRun::No
                        }
                    })
                    .system(),
                )
                .with_system(pause_menu::handle_pause_menu.system()),
        );
}

fn switch_fullscreen(mut windows: ResMut<Windows>, keyboard_input: Res<Input<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::F11) {
        if let Some(window) = windows.get_primary_mut() {
            window.set_mode(match window.mode() {
                WindowMode::BorderlessFullscreen => WindowMode::Windowed,
                _ => WindowMode::BorderlessFullscreen,
            });
        }
    }
}

mod ui_utils {
    use crate::plugins::game::assets::GameInfo;
    use bevy_retro::ui::raui::prelude::*;

    pub fn get_ui_theme(game_info: &GameInfo) -> ThemeProps {
        let mut theme = ThemeProps::default();

        theme.content_backgrounds.insert(
            String::from("panel"),
            ThemedImageMaterial::Image(ImageBoxImage {
                id: game_info.ui_theme.panel.image.clone(),
                scaling: ImageBoxImageScaling::Frame(
                    (
                        game_info.ui_theme.panel.border_size as f32,
                        game_info.ui_theme.panel.only_frame,
                    )
                        .into(),
                ),
                ..Default::default()
            }),
        );

        theme.content_backgrounds.insert(
            String::from("button-up"),
            ThemedImageMaterial::Image(ImageBoxImage {
                id: game_info.ui_theme.button_up.image.clone(),
                scaling: ImageBoxImageScaling::Frame(
                    (
                        game_info.ui_theme.button_up.border_size as f32,
                        game_info.ui_theme.button_up.only_frame,
                    )
                        .into(),
                ),
                ..Default::default()
            }),
        );

        theme.content_backgrounds.insert(
            String::from("button-down"),
            ThemedImageMaterial::Image(ImageBoxImage {
                id: game_info.ui_theme.button_down.image.clone(),
                scaling: ImageBoxImageScaling::Frame(
                    (
                        game_info.ui_theme.button_down.border_size as f32,
                        game_info.ui_theme.button_down.only_frame,
                    )
                        .into(),
                ),
                ..Default::default()
            }),
        );

        theme.switch_variants.insert(
            "checkbox".to_owned(),
            ThemedSwitchMaterial {
                on: ThemedImageMaterial::Image(ImageBoxImage {
                    id: game_info.ui_theme.checkbox.checked.clone(),
                    ..Default::default()
                }),
                off: ThemedImageMaterial::Image(ImageBoxImage {
                    id: game_info.ui_theme.checkbox.unchecked.clone(),
                    ..Default::default()
                }),
            },
        );

        theme.text_variants.insert(
            String::new(),
            ThemedTextMaterial {
                font: TextBoxFont {
                    name: game_info.ui_theme.default_font.clone(),
                    // Font's in Bevy Retro don't really have sizes so we can just set this to
                    // one
                    size: 1.0,
                },
                ..Default::default()
            },
        );

        theme.icons_level_sizes = vec![8., 12., 16.];

        theme
    }
}

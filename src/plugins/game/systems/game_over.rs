use std::time::Duration;

use bevy::prelude::*;
use bevy_retrograde::prelude::{
    kira::parameter::tween::Tween,
    raui::core::{make_widget, widget},
    *,
};

use crate::plugins::game::{
    assets::GameInfo,
    components::{CurrentLevel, CurrentLevelMusic},
};

use super::GameState;

pub fn run_game_over_screen(
    mut has_shown_game_over: Local<bool>,
    mut display_screen_timer: Local<Timer>,
    mut commands: Commands,
    all_entities: Query<Entity>,
    mut state: ResMut<State<GameState>>,
    mut ui_tree: ResMut<UiTree>,
    current_level_music: Option<Res<CurrentLevelMusic>>,
    mut sound_controller: SoundController,
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    time: Res<Time>,
) {
    // If we haven't shown the game over screen
    if !*has_shown_game_over {
        *has_shown_game_over = true;
        debug!("Game over! Showing game over menu.");

        // Show the game over screen
        *ui_tree = UiTree(make_widget!(ui::game_over_screen).into());

        // Stop the music
        if let Some(current_level_music) = current_level_music {
            sound_controller.stop_sound_with_settings(
                current_level_music.sound,
                StopSoundSettings::new().fade_tween(Some(Tween {
                    duration: 1.0,
                    easing: Default::default(),
                    ease_direction: Default::default(),
                })),
            );
        }
        commands.remove_resource::<CurrentLevelMusic>();

        // Clear the current level
        commands.remove_resource::<CurrentLevel>();

        // Set the timer for how long we display the game over screen
        display_screen_timer.set_duration(Duration::from_secs(5));
        display_screen_timer.set_repeating(false);
        display_screen_timer.reset();

    // If we are currently showing the game over screen
    } else {
        // Tick the display timer
        display_screen_timer.tick(time.delta());

        // If the timer is finished
        if display_screen_timer.finished()
            || keyboard_input.just_pressed(KeyCode::Escape)
            || mouse_input.just_pressed(MouseButton::Left)
        {
            // Clear the game info
            commands.remove_resource::<GameInfo>();

            // Despawn all entities
            for entity in all_entities.iter() {
                commands.entity(entity).despawn();
            }

            // Transition to the game init state to restart the game
            state
                .replace(GameState::Init)
                .expect("Could not transition to game init state");

            // Reset game over display state
            *has_shown_game_over = false;
            *ui_tree = UiTree(widget!(()));
        }
    }
}

mod ui {
    use bevy::prelude::World;
    use bevy_retrograde::prelude::raui::prelude::*;

    use crate::plugins::game::assets::GameInfo;

    pub fn game_over_screen(ctx: WidgetContext) -> WidgetNode {
        let world: &mut World = ctx.process_context.get_mut().unwrap();

        let game_info = world.get_resource::<GameInfo>().unwrap();

        make_widget!(content_box)
            // Add a black background
            .listed_slot(make_widget!(image_box).with_props(ImageBoxProps {
                material: ImageBoxMaterial::Color(ImageBoxColor {
                    color: Color {
                        r: 0.,
                        g: 0.,
                        b: 0.,
                        a: 1.,
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }))
            // The "Game Over" text centered in the screen
            .listed_slot(make_widget!(text_box).with_props(TextBoxProps {
                color: Color {
                    r: 1.,
                    g: 1.,
                    b: 1.,
                    a: 1.,
                },
                text: "Game Over".into(),
                font: TextBoxFont {
                    name: game_info.ui_theme.default_font.clone(),
                    size: 1.,
                },
                horizontal_align: TextBoxHorizontalAlign::Center,
                vertical_align: TextBoxVerticalAlign::Middle,
                ..Default::default()
            }))
            .into()
    }
}

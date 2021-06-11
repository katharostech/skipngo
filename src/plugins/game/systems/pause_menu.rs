use bevy::prelude::*;
use bevy_retro::prelude::{
    raui::{core::make_widget, prelude::WidgetNode},
    UiTree,
};

use super::GameState;

pub fn handle_pause_menu(
    mut pause_menu_visible: Local<bool>,
    mut ui: ResMut<UiTree>,
    keyboard_input: Res<Input<KeyCode>>,
    mut state: ResMut<State<GameState>>,
) {
    if !*pause_menu_visible {
        debug!("Showing pause menu");
        *pause_menu_visible = true;
        *ui = UiTree(make_widget!(ui::pause_menu).into());
    }

    if keyboard_input.just_pressed(KeyCode::Escape) {
        debug!("Unpausing and hiding pause menu");
        state.pop().expect("Could not transition game state");
        *ui = UiTree(WidgetNode::None);
        *pause_menu_visible = false;
    }
}

mod ui {
    use bevy::prelude::World;
    use bevy_retro::ui::raui::prelude::*;

    use crate::plugins::game::{assets::GameInfo, systems::ui_utils::get_ui_theme};

    pub fn pause_menu(ctx: WidgetContext) -> WidgetNode {
        let WidgetContext {
            process_context, ..
        } = ctx;

        // Get the game info from the world
        let world: &mut World = process_context.get_mut().unwrap();
        let game_info = world.get_resource::<GameInfo>().unwrap();

        // Content box
        make_widget!(content_box)
            .with_shared_props(get_ui_theme(game_info))
            .listed_slot(
                // Size box
                make_widget!(size_box)
                    .with_props(SizeBoxProps {
                        width: SizeBoxSizeValue::Exact(50.),
                        height: SizeBoxSizeValue::Exact(20.),
                        ..Default::default()
                    })
                    .with_props(ContentBoxItemLayout {
                        align: 0.5.into(),
                        ..Default::default()
                    })
                    .named_slot(
                        "content",
                        // Horizontal paper
                        make_widget!(horizontal_paper)
                            .with_props(PaperProps {
                                variant: "button-up".into(),
                                ..Default::default()
                            })
                            // Text box
                            .listed_slot(make_widget!(text_box).with_props(TextBoxProps {
                                text: "Paused".into(),
                                font: TextBoxFont {
                                    name: game_info.ui_theme.default_font.clone(),
                                    ..Default::default()
                                },
                                horizontal_align: TextBoxHorizontalAlign::Center,
                                vertical_align: TextBoxVerticalAlign::Middle,
                                ..Default::default()
                            })),
                    ),
            )
            .into()
    }
}

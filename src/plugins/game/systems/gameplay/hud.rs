use bevy::prelude::{Handle, With, World};
// use bevy::prelude::{debug, Handle, With, World};
use bevy_retro::ui::raui::prelude::*;

use crate::plugins::game::{assets::GameInfo, components::Character, systems::gameplay::Health};

pub fn hud(ctx: WidgetContext) -> WidgetNode {
    let WidgetContext {
        process_context, ..
    } = ctx;

    let world: &mut World = process_context.get_mut().unwrap();

    // Get the health of the player
    let player_health = {
        let mut q = world.query_filtered::<&Health, With<Handle<Character>>>();
        if let Some(health) = q.iter(&world).next() {
            health.current
        } else {
            return WidgetNode::None;
        }
    };

    // Get the game info from the world
    let game_info = world.get_resource::<GameInfo>().unwrap();
    let health_background = &game_info.ui_theme.hud.health_background;
    let full_heart = &game_info.ui_theme.hud.full_heart;
    let half_heart = &game_info.ui_theme.hud.half_heart;

    make_widget!(content_box)
        .listed_slot(
            make_widget!(size_box)
                .with_props(SizeBoxProps {
                    width: SizeBoxSizeValue::Exact(health_background.size.0 as f32),
                    height: SizeBoxSizeValue::Exact(health_background.size.1 as f32),
                    ..Default::default()
                })
                .with_props(ContentBoxItemLayout {
                    margin: 5.0.into(),
                    ..Default::default()
                })
                .named_slot(
                    "content",
                    make_widget!(content_box)
                        .listed_slot(make_widget!(image_box).with_props(ImageBoxProps {
                            material: ImageBoxMaterial::Image(ImageBoxImage {
                                id: health_background.image.clone(),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }))
                        .listed_slot({
                            let mut horizontal = make_widget!(horizontal_box)
                                .with_props(HorizontalBoxProps {
                                    separation: 2.,
                                    ..Default::default()
                                })
                                .with_props(ContentBoxItemLayout {
                                    margin: 1.0.into(),
                                    ..Default::default()
                                });

                            let full_hearts = player_health / 2;
                            let half_hearts = player_health - full_hearts * 2;

                            for _ in 0..full_hearts {
                                horizontal = horizontal.listed_slot(
                                    make_widget!(image_box)
                                        .with_props(ImageBoxProps {
                                            material: ImageBoxMaterial::Image(ImageBoxImage {
                                                id: full_heart.image.clone(),
                                                ..Default::default()
                                            }),
                                            width: ImageBoxSizeValue::Exact(
                                                full_heart.size.0 as f32,
                                            ),
                                            height: ImageBoxSizeValue::Exact(
                                                full_heart.size.1 as f32,
                                            ),
                                            ..Default::default()
                                        })
                                        .with_props(FlexBoxItemLayout {
                                            grow: 0.0,
                                            ..Default::default()
                                        }),
                                );
                            }

                            for _ in 0..half_hearts {
                                horizontal = horizontal.listed_slot(
                                    make_widget!(image_box)
                                        .with_props(ImageBoxProps {
                                            material: ImageBoxMaterial::Image(ImageBoxImage {
                                                id: half_heart.image.clone(),
                                                ..Default::default()
                                            }),
                                            width: ImageBoxSizeValue::Exact(
                                                half_heart.size.0 as f32,
                                            ),
                                            height: ImageBoxSizeValue::Exact(
                                                half_heart.size.1 as f32,
                                            ),
                                            ..Default::default()
                                        })
                                        .with_props(FlexBoxItemLayout {
                                            grow: 0.0,
                                            ..Default::default()
                                        }),
                                );
                            }

                            horizontal
                        }),
                ),
        )
        .into()
}

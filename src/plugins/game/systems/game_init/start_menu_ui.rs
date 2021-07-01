use bevy::prelude::World;
use bevy_retrograde::ui::raui::prelude::*;

use super::{ui_utils::get_ui_theme, CurrentLevel, GameInfo, GameState, State};

fn use_start_menu(ctx: &mut WidgetContext) {
    ctx.life_cycle.change(|ctx| {
        let world: &mut World = ctx.process_context.get_mut().unwrap();

        for msg in ctx.messenger.messages {
            if let Some(msg) = msg.as_any().downcast_ref::<GameButtonMessage>() {
                if &msg.0 == "start" {
                    let start_level = world
                        .get_resource::<GameInfo>()
                        .unwrap()
                        .game_start_level
                        .clone();

                    {
                        let mut current_level = world.get_resource_mut::<CurrentLevel>().unwrap();
                        *current_level = CurrentLevel(start_level);
                    }
                    {
                        let mut state = world.get_resource_mut::<State<GameState>>().unwrap();
                        if state.current() != &GameState::LoadingGame {
                            state.push(GameState::LoadingGame).unwrap();
                        }
                    }
                } else if &msg.0 == "show_settings" {
                    let mut query = world.query::<&super::Camera>();
                    let camera = query.iter_mut(world).next().expect("Expected one camera");

                    let previous_crt_filter_enabled = camera.custom_shader.is_some();
                    let previous_pixel_aspect_4_3_enabled =
                        camera.pixel_aspect_ratio.abs() - 1.0 > f32::EPSILON;

                    ctx.state
                        .write(StartMenuState {
                            show_settings: true,
                            previous_crt_filter_enabled,
                            previous_pixel_aspect_4_3_enabled,
                        })
                        .unwrap();
                } else if &msg.0 == "cancel_settings" {
                    let mut query = world.query::<&mut super::Camera>();
                    let mut camera = query.iter_mut(world).next().expect("Expected one camera");

                    ctx.state
                        .mutate_cloned(|state: &mut StartMenuState| {
                            camera.pixel_aspect_ratio = if state.previous_pixel_aspect_4_3_enabled {
                                4. / 3.
                            } else {
                                1.
                            };

                            camera.custom_shader = if state.previous_crt_filter_enabled {
                                Some(super::CrtShader::default().get_shader())
                            } else {
                                None
                            };

                            state.show_settings = false;
                        })
                        .unwrap();
                } else if &msg.0 == "save_settings" {
                    ctx.state
                        .mutate_cloned(|state: &mut StartMenuState| {
                            state.show_settings = false;
                        })
                        .unwrap();
                }
            }
        }
    })
}

#[derive(PropsData, Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
struct StartMenuState {
    show_settings: bool,
    previous_crt_filter_enabled: bool,
    previous_pixel_aspect_4_3_enabled: bool,
}

/// The UI tree used for the start menu
#[pre_hooks(use_start_menu)]
pub fn start_menu(mut ctx: WidgetContext) -> WidgetNode {
    let WidgetContext {
        id,
        process_context,
        ..
    } = ctx;

    let StartMenuState { show_settings, .. } = ctx.state.read_cloned_or_default();

    // Get the game info from the world
    let world: &mut World = process_context.get_mut().unwrap();
    let game_info = world.get_resource::<GameInfo>().unwrap();

    // Create shared props containing the theme
    let shared_props = Props::default()
        // Add the theme properties
        .with(get_ui_theme(game_info))
        .with(game_info.clone());

    let vertical_box_props = VerticalBoxProps {
        separation: 0.,
        ..Default::default()
    };

    // The title image props
    let title_image_props = Props::new(ImageBoxProps {
        material: ImageBoxMaterial::Image(ImageBoxImage {
            id: game_info.splash_screen.splash_image.path.clone(),
            ..Default::default()
        }),
        width: ImageBoxSizeValue::Exact(game_info.splash_screen.splash_image.size.x as f32),
        height: ImageBoxSizeValue::Exact(game_info.splash_screen.splash_image.size.y as f32),
        ..Default::default()
    })
    .with(FlexBoxItemLayout {
        align: 0.5,
        grow: 0.0,
        margin: Rect {
            top: 10.,
            ..Default::default()
        },
        ..Default::default()
    });

    let start_button_props = Props::new(FlexBoxItemLayout {
        align: 0.5,
        grow: 0.0,
        margin: Rect {
            top: 10.,
            ..Default::default()
        },
        ..Default::default()
    })
    .with(GameButtonProps {
        text: "Start Game".into(),
        notify_id: id.to_owned(),
        message_name: "start".into(),
    });

    let settings_button_props = Props::new(FlexBoxItemLayout {
        align: 0.5,
        grow: 0.0,
        margin: Rect {
            top: 10.,
            ..Default::default()
        },
        ..Default::default()
    })
    .with(GameButtonProps {
        text: "Settings".into(),
        notify_id: id.to_owned(),
        message_name: "show_settings".into(),
    });

    let copyright_props = Props::new(TextBoxProps {
        text: game_info.splash_screen.copyright.text.clone(),
        color: Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.,
        },
        font: TextBoxFont {
            name: game_info.splash_screen.copyright.font.clone(),
            size: 1.0,
        },
        horizontal_align: TextBoxHorizontalAlign::Center,
        vertical_align: TextBoxVerticalAlign::Bottom,
        ..Default::default()
    })
    .with(ContentBoxItemLayout {
        margin: 5.0.into(),
        ..Default::default()
    });

    let content = if show_settings {
        let props = Props::new(SettingsPanelProps {
            cancel_notify_id: ctx.id.to_owned(),
            cancel_notify_message: "cancel_settings".into(),
            save_notify_id: ctx.id.to_owned(),
            save_notify_message: "save_settings".into(),
        });

        widget! {
            (#{"settings"} settings_panel: {props})
        }
    } else {
        widget! {
            // The main content
            (content_box [
                (nav_vertical_box: {vertical_box_props} [
                    (image_box: {title_image_props})
                    (game_button: {start_button_props})
                    (game_button: {settings_button_props})
                ])
                (text_box: {copyright_props})
            ])
        }
    };

    widget! {
        (content_box | {shared_props} [
            {content}
        ])
    }
}

#[derive(PropsData, Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
struct GameButtonProps {
    text: String,
    notify_id: WidgetId,
    message_name: String,
}

#[derive(MessageData, Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
struct GameButtonMessage(String);

fn use_game_button(ctx: &mut WidgetContext) {
    ctx.life_cycle.change(|ctx| {
        let ButtonProps { trigger, .. } = ctx.state.read_cloned_or_default();
        let GameButtonProps {
            notify_id,
            message_name: message,
            ..
        } = ctx.props.read_cloned_or_default();

        if trigger {
            ctx.messenger.write(notify_id, GameButtonMessage(message));
        }
    });
}

#[pre_hooks(
    // This allows us to get a `ButtonProps` instance from our widget state which will keep
    // track of whether or not we are clicked, hovered over, etc.
    use_game_button,
    use_button_notified_state,
)]
fn game_button(mut ctx: WidgetContext) -> WidgetNode {
    let world: &mut World = ctx.process_context.get_mut().unwrap();
    let game_info = world.get_resource::<GameInfo>().unwrap();

    // Get our button state
    let ButtonProps {
        selected: hover,
        trigger: clicked,
        ..
    } = ctx.state.read_cloned_or_default();

    let GameButtonProps {
        text: button_text, ..
    } = ctx.props.read_cloned_or_default();

    let button_props = ctx
        .props
        .clone()
        .with(NavItemActive)
        .with(ButtonNotifyProps(ctx.id.to_owned().into()));

    let button_panel_props = Props::new(PaperProps {
        frame: None,
        variant: if clicked {
            // TODO: Somehow pre-load the button-up image so that it doesn't flash
            // blank for a second the first time a button is clicked
            String::from("button-down")
        } else {
            String::from("button-up")
        },
    });

    let scale = if hover { 1.1 } else { 1. };

    let label_props = Props::new(TextBoxProps {
        text: button_text,
        width: TextBoxSizeValue::Fill,
        height: TextBoxSizeValue::Fill,
        horizontal_align: TextBoxHorizontalAlign::Center,
        vertical_align: TextBoxVerticalAlign::Middle,
        font: TextBoxFont {
            name: game_info.ui_theme.default_font.clone(),
            size: 1.,
        },
        transform: Transform {
            translation: Vec2 {
                x: 0.,
                y: if clicked { 1. } else { 0. },
            },
            // scale: Vec2::from(1.0 / scale), // Undo button scale to make sure text stays same size
            ..Default::default()
        },
        ..Default::default()
    });

    let size_box_props = Props::new(SizeBoxProps {
        width: SizeBoxSizeValue::Exact(70.),
        height: SizeBoxSizeValue::Exact(18.),
        transform: Transform {
            scale: Vec2::from(scale),
            translation: Vec2 {
                x: if hover { (-75. * scale + 75.) / 2. } else { 0. },
                y: if hover { (-20. * scale + 20.) / 2. } else { 0. },
            },
            ..Default::default()
        },
        ..Default::default()
    });

    widget! {
        (button: {button_props} {
            content = (size_box: {size_box_props} {
                content = (horizontal_paper: {button_panel_props} [
                    (text_box: {label_props})
                ])
            })
        })
    }
}

#[derive(PropsData, Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
struct SettingsPanelProps {
    cancel_notify_id: WidgetId,
    cancel_notify_message: String,
    save_notify_id: WidgetId,
    save_notify_message: String,
}

fn use_settings_panel(ctx: &mut WidgetContext) {
    ctx.life_cycle.change(|ctx| {
        let world: &mut World = ctx.process_context.get_mut().unwrap();
        let mut query = world.query::<&mut super::Camera>();
        let mut camera = query.iter_mut(world).next().expect("Expected one camera");

        for msg in ctx.messenger.messages {
            // Respond to click settings change messages
            if let Some(msg) = msg.as_any().downcast_ref::<ButtonNotifyMessage>() {
                if msg.trigger_start() && msg.sender.ends_with("pixel_aspect") {
                    if (camera.pixel_aspect_ratio - 1.0).abs() < f32::EPSILON {
                        camera.pixel_aspect_ratio = 4.0 / 3.0;
                    } else {
                        camera.pixel_aspect_ratio = 1.0;
                    }
                } else if msg.trigger_start() && msg.sender.ends_with("crt_filter") {
                    if camera.custom_shader == None {
                        camera.custom_shader = Some(super::CrtShader::default().get_shader())
                    } else {
                        camera.custom_shader = None;
                    }
                }
            }
        }
    });
}

#[pre_hooks(use_settings_panel)]
fn settings_panel(mut ctx: WidgetContext) -> WidgetNode {
    let game_info: GameInfo = ctx.shared_props.read_cloned().unwrap();
    let SettingsPanelProps {
        cancel_notify_id,
        cancel_notify_message,
        save_notify_id,
        save_notify_message,
    } = ctx.props.read_cloned_or_default();

    // Get the camera info from the world
    let world: &mut World = ctx.process_context.get_mut().unwrap();
    let mut query = world.query::<&super::Camera>();
    let camera = query.iter_mut(world).next().expect("Expected one camera");
    // Get the values for the checkboxes
    let crt_filter = camera.custom_shader.is_some();
    let pixel_aspect_4_3 = camera.pixel_aspect_ratio.abs() - 1.0 > f32::EPSILON;

    // Settings panel
    let panel_props = Props::new(ContentBoxItemLayout {
        margin: Rect {
            left: 13.,
            right: 13.,
            top: 7.,
            bottom: 7.,
        },
        ..Default::default()
    })
    .with(PaperProps {
        variant: "panel".into(),
        frame: None,
    });

    // "Settings" title
    let title_props = Props::new(TextBoxProps {
        text: "Settings".into(),
        font: TextBoxFont {
            name: game_info.ui_theme.default_font.clone(),
            size: 1.0,
        },
        horizontal_align: TextBoxHorizontalAlign::Center,
        color: Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.,
        },
        ..Default::default()
    })
    .with(FlexBoxItemLayout {
        grow: 0.,
        basis: Some(16.),
        ..Default::default()
    });

    // Cancel button
    let cancel_button_props = Props::new(FlexBoxItemLayout {
        align: 0.5,
        grow: 0.0,
        ..Default::default()
    })
    .with(GameButtonProps {
        text: "Cancel".into(),
        notify_id: cancel_notify_id,
        message_name: cancel_notify_message,
    });

    // Save button
    let save_button_props = Props::new(FlexBoxItemLayout {
        align: 0.5,
        grow: 0.0,
        ..Default::default()
    })
    .with(GameButtonProps {
        text: "Save".into(),
        notify_id: save_notify_id,
        message_name: save_notify_message,
    });

    // Container for buttons
    let button_box_props = Props::new(())
        .with(FlexBoxProps {
            wrap: true,
            direction: FlexBoxDirection::HorizontalLeftToRight,
            separation: 10.,
            ..Default::default()
        })
        .with(FlexBoxItemLayout {
            grow: 0.,
            align: 0.5,
            ..Default::default()
        });

    // "Graphics" title
    let graphics_settings_title_props = Props::new(TextBoxProps {
        text: "Graphics".into(),
        font: TextBoxFont {
            name: game_info.ui_theme.default_font.clone(),
            size: 1.0,
        },
        color: Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.,
        },
        ..Default::default()
    })
    .with(FlexBoxItemLayout {
        grow: 0.0,
        align: 0.0,
        basis: Some(16.),
        margin: Rect {
            left: 5.,
            ..Default::default()
        },
        ..Default::default()
    });

    // Wrapper for check box settings
    let check_box_wrapper_props = Props::new(FlexBoxItemLayout {
        grow: 0.0,
        basis: Some(17.),
        margin: Rect {
            top: 5.,
            left: 10.,
            ..Default::default()
        },
        ..Default::default()
    });

    // CRT Filter check box
    let crt_filter_check_props = Props::new(SwitchPaperProps {
        on: crt_filter,
        variant: "checkbox".into(),
        size_level: 1,
    })
    .with(NavItemActive)
    .with(ButtonNotifyProps(ctx.id.to_owned().into()))
    .with(ThemedWidgetProps {
        color: ThemeColor::Primary,
        variant: ThemeVariant::ContentOnly,
    })
    .with(FlexBoxItemLayout {
        grow: 0.0,
        ..Default::default()
    });

    // CRT Filter text
    let crt_filter_text_props = Props::new(TextBoxProps {
        text: "CRT Filter".into(),
        font: TextBoxFont {
            name: game_info.ui_theme.default_font.clone(),
            size: 1.0,
        },
        color: Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.,
        },
        ..Default::default()
    })
    .with(FlexBoxItemLayout {
        margin: Rect {
            left: 10.,
            ..Default::default()
        },
        ..Default::default()
    });

    // 4/3 Pixel Aspect Ratio checkbox
    let pixel_aspect_check_props = Props::new(SwitchPaperProps {
        on: pixel_aspect_4_3,
        variant: "checkbox".into(),
        size_level: 1,
    })
    .with(NavItemActive)
    .with(ButtonNotifyProps(ctx.id.to_owned().into()))
    .with(ThemedWidgetProps {
        color: ThemeColor::Primary,
        variant: ThemeVariant::ContentOnly,
    })
    .with(FlexBoxItemLayout {
        grow: 0.0,
        ..Default::default()
    });

    // 4/3 Pixel Aspect Ratio text
    let pixel_aspect_text_props = Props::new(TextBoxProps {
        text: "4/3 Pixel Aspect Ratio".into(),
        font: TextBoxFont {
            name: game_info.ui_theme.default_font,
            size: 1.0,
        },
        color: Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 1.,
        },
        ..Default::default()
    })
    .with(FlexBoxItemLayout {
        margin: Rect {
            left: 10.,
            ..Default::default()
        },
        ..Default::default()
    });

    let margin_box_props = FlexBoxItemLayout {
        margin: Rect {
            top: 10.,
            bottom: 10.,
            left: 15.,
            right: 15.,
        },
        ..Default::default()
    };

    widget! {
        (nav_content_box [
            (nav_vertical_paper: {panel_props} [
                (vertical_box: {margin_box_props} [
                    (text_box: {title_props})
                    (vertical_box [
                        (text_box: {graphics_settings_title_props})
                        (horizontal_box: {check_box_wrapper_props.clone()} [
                            (#{"crt_filter"} switch_button_paper: {crt_filter_check_props})
                            (text_box: {crt_filter_text_props})
                        ])
                        (horizontal_box: {check_box_wrapper_props} [
                            (#{"pixel_aspect"} switch_button_paper: {pixel_aspect_check_props})
                            (text_box: {pixel_aspect_text_props})
                        ])
                    ])
                    (flex_box: {button_box_props} [
                        (game_button: {cancel_button_props})
                        (game_button: {save_button_props})
                    ])
                ])
            ])
        ])
    }
}

use bevy::prelude::*;

pub fn add_events(app: &mut AppBuilder) {
    app.add_event::<ControlEvent>();
}

/// A user control event, used to control the character
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum ControlEvent {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
}

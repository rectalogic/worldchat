use bevy::prelude::*;

use crate::{app::AppState, irc::IrcError};

pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_error);
    }
}

fn scene(error: String) -> impl Scene {
    bsn! {
        #Error
        DespawnOnExit<AppState>(AppState::Error)
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        BorderColor::from(Color::srgb(0.9, 0.9, 0.9))
        BackgroundColor::from(Color::BLACK)
        Children [
            Node {
                width: percent(90),
            }
            Children [
                Text(error)
                TextFont {
                    font_size: px(33.0),
                }
                TextColor(Color::srgb(0.9, 0.9, 0.9))
            ]
        ]
    }
}

#[expect(clippy::needless_pass_by_value)]
fn on_error(error: On<IrcError>, mut commands: Commands) {
    commands.spawn_scene(scene(error.0.clone()));
}

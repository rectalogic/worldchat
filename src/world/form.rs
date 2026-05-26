use bevy::{
    input_focus::{AutoFocus, InputFocus},
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::Pressed,
    ui_widgets::{Activate, Button},
};

pub struct FormPlugin;

impl Plugin for FormPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(button_highlight)
            .add_observer(button_unhighlight)
            .add_systems(Update, text_submission);
    }
}

#[derive(Component, FromTemplate)]
struct ButtonFor(Entity);

#[derive(EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct SubmitTextEvent {
    pub entity: Entity,
    pub value: String,
}

pub fn ui(label: &str) -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Center,
            column_gap: px(0),
        }
        Children [
            (
                :text_input
                #TextInput
                Node {
                    flex_grow: 1.0,
                }
            ),
            (
                button(label)
                ButtonFor(#TextInput)
                on(|event: On<Activate>, mut commands: Commands, buttons: Query<&ButtonFor>, mut texts: Query<&mut EditableText>| {
                    if let Ok(button_for ) = buttons.get(event.entity) && let Ok(mut text) = texts.get_mut(button_for.0) {
                        let value = text.value().to_string();
                        if !value.is_empty() {
                            commands.trigger(SubmitTextEvent {
                                entity: button_for.0,
                                value,
                            });
                            text.clear();
                        }
                    }
                })
            )
        ]
    }
}

const BUTTON_COLOR: Color = Color::srgb(0.003, 0.447, 0.678);
const BUTTON_PRESSED_COLOR: Color = Color::srgb(0.003 * 0.8, 0.447 * 0.8, 0.678 * 0.8);

fn button(label: &str) -> impl Scene {
    bsn! {
        Button
        Node {
            padding: UiRect::axes(px(24), px(0)),
            min_height: px(65),
            border: px(2),
            border_radius: BorderRadius::all(px(10)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BorderColor::from(Color::BLACK)
        BackgroundColor(BUTTON_COLOR)
        Children [(
            Text(label)
            TextFont {
                font_size: px(33.0),
            }
            TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        )]
    }
}

fn text_input() -> impl Scene {
    bsn! {
        EditableText
        Node {
            border: px(1),
            padding: UiRect::axes(px(5), px(10)),
        }
        TextLayout::no_wrap()
        AutoFocus
        TextCursorStyle
        TextFont {
            font_size: px(33.0),
        }
        TextColor(Color::srgb(0.878, 0.890, 0.905))
        BackgroundColor(Color::srgb(0.101, 0.121, 0.156))
        BorderColor::all(Color::srgb(0.1647, 0.192, 0.250))
    }
}

#[expect(clippy::needless_pass_by_value)]
fn text_submission(
    mut commands: Commands,
    input_focus: Res<InputFocus>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut text_input: Query<&mut EditableText>,
) {
    if keyboard_input.just_pressed(KeyCode::Enter)
        && let Some(focused_entity) = input_focus.get()
        && let Ok(mut text_input) = text_input.get_mut(focused_entity)
    {
        let value = text_input.value().to_string();
        if !value.is_empty() {
            commands.trigger(SubmitTextEvent {
                entity: focused_entity,
                value,
            });
            text_input.clear();
        }
    }
}

#[expect(clippy::needless_pass_by_value)]
fn button_highlight(
    pressed: On<Add, Pressed>,
    mut buttons: Query<&mut BackgroundColor, With<Button>>,
) {
    if let Ok(mut color) = buttons.get_mut(pressed.entity) {
        color.0 = BUTTON_PRESSED_COLOR;
    }
}

#[expect(clippy::needless_pass_by_value)]
fn button_unhighlight(
    removed: On<Remove, Pressed>,
    mut buttons: Query<&mut BackgroundColor, With<Button>>,
) {
    if let Ok(mut color) = buttons.get_mut(removed.entity) {
        color.0 = BUTTON_COLOR;
    }
}

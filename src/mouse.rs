use iced::event::Event::Mouse;
use iced::mouse;
use iced::{Subscription, event};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    LeftPressed,
    LeftReleased,
    Moved { x: f32, y: f32 },
}

#[derive(Default)]
pub struct State {
    is_left_pressed: bool,
    pressed_at: Option<(f32, f32)>,
    released_at: Option<(f32, f32)>,
    position: (f32, f32)
}


pub fn update(state: &mut State, message: Message) {
    match message {
        Message::LeftPressed => {
            state.is_left_pressed = true;
            state.pressed_at = Some(state.position);
            println!(
                "Left button pressed at: ({}, {})",
                state.position.0, state.position.1
            );
        }
        Message::LeftReleased => {
            state.is_left_pressed = false;
            state.pressed_at = None;
            state.released_at = Some(state.position);
            println!(
                "Left button released at: ({}, {})",
                state.position.0, state.position.1
            );
        }
        Message::Moved { x, y } => {
            state.position = (x, y);
            println!("Mouse moved to: ({}, {})", x, y);
        }
    }
}

pub fn subscription(state: &State) -> Subscription<Message> {
    event::listen_with(|event, status, window| match event {
        Mouse(mouse::Event::CursorMoved { position }) => Some(Message::Moved {
            x: position.x,
            y: position.y,
        }),
        Mouse(mouse::Event::ButtonPressed(button)) => match button {
            mouse::Button::Left => Some(Message::LeftPressed),
            _ => None,
        },
        Mouse(mouse::Event::ButtonReleased(button)) => match button {
            mouse::Button::Left => Some(Message::LeftReleased),
            _ => None,
        },
        _ => None,
    })
}

use crate::mouse;
use crate::models::Grave;

struct MapEditor {
    graves: Vec<Grave>,
    mouse_state: mouse::State,
}

enum Message {
    MouseMessage(mouse::Message),
}


impl Default for MapEditor {
    fn default() -> Self {
        Self { graves: vec![], mouse_state: Default::default() }
    }
}

impl MapEditor {
    fn update(&mut self, message: Message) {
        match message {
            Message::MouseMessage(mouse_msg) => mouse::update(&mut self.mouse_state, mouse_msg),
        }
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        mouse::subscription(&self.mouse_state).map(Message::MouseMessage)
    }
}
mod mouse;
mod models;
mod screens;

use iced::widget::Space;
use iced::{Element, Task};
use iced;

use mouse::subscription;

fn main() -> iced::Result {
    iced::application(mouse::State::default, mouse::update, view)
     .subscription(subscription)
     .run()
        
}

fn update(_state: &mut mouse::State, _message: mouse::Message) -> Task<mouse::Message> {
    Task::none()
}

fn view(_state: &mouse::State) -> Element<mouse::Message> {
    Space::new().into()
}
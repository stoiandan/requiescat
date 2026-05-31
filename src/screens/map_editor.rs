use iced::widget::{Action, canvas, container};
use iced::{Element, Point, Renderer, Theme};

use crate::models::Grave;
pub struct MapEditor {
    graves: Vec<Grave>,
}

pub enum Message {
    GraveCreated(Grave),
}


impl Default for MapEditor {
    fn default() -> Self {
        Self { graves: vec![] }
    }
}

impl MapEditor {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::GraveCreated(grave) => self.graves.push(grave),
        }
    }

    pub fn view(&self) -> Element<'_, Message>{
        container(
        canvas(self)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
        )
        .style(|_| container::Style {
            border: iced::Border {
                color: iced::Color::WHITE,
                width: 2.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
    }
}

#[derive(Default)]
pub struct CanvasState {
    left_pressed_at: Option<Point>,
}


impl canvas::Program<Message> for MapEditor {
    type State = CanvasState;

    fn draw(&self,
            _state: &Self::State,
            renderer: &Renderer,
            _: &Theme,
            bounds: iced::Rectangle,
            _: iced::mouse::Cursor
        ) -> Vec<canvas::Geometry> {

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        print!("graves: {}", self.graves.len());
        for grave in &self.graves {
            let rect = iced::Rectangle {
                x: grave.coordinate.top_left_x(),
                y: grave.coordinate.top_left_y(),
                width: grave.coordinate.width(),
                height: grave.coordinate.height(),
            };
            println!("Filling a grave");
            frame.fill_rectangle(rect.position(), rect.size(), iced::Color::from_rgb(0.65, 0.121, 0.157));
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &iced::Event,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>>
    {
        match _event {
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                _state.left_pressed_at = Some(_cursor.position().unwrap());
                println!("set left_pressed_at to: {:?}", _state.left_pressed_at);
                None
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                let p1 = _state.left_pressed_at?;
                let p2 = _cursor.position()?;
                if is_worth_drawing(p1, p2 ) {
                    let m = Message::GraveCreated((p1, p2).into());
                    return Some(Action::publish(m));
                }
                None
            }
            _ => None,
        }
    }
}


fn is_worth_drawing(staring_point: iced::Point, ending_point: iced::Point) -> bool {
         (staring_point.x - ending_point.x).abs() >= 5.0 &&
         (staring_point.y - ending_point.y).abs() >= 5.0
    }
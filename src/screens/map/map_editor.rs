use super::Camera;
use super::is_worth_drawing;
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

    pub fn view(&self) -> Element<'_, Message> {
        container(
            canvas(self)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
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
    camera: Camera,
}

impl canvas::Program<Message> for MapEditor {
    type State = CanvasState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _: &Theme,
        bounds: iced::Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let zoom = _state.camera.zoom;
        for grave in &self.graves {

            let grave_to_screen = _state.camera.world_to_screen(Point::new(grave.coordinate.top_left_x(), grave.coordinate.top_left_y()));
            let rect = iced::Rectangle {
                x: grave_to_screen.x,
                y: grave_to_screen.y,
                width: grave.coordinate.width() * zoom,
                height: grave.coordinate.height() * zoom,
            };
            frame.fill_rectangle(
                rect.position(),
                rect.size(),
                iced::Color::from_rgb(0.65, 0.121, 0.157),
            );
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &iced::Event,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match _event {
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                let cursor = _cursor.position_in(_bounds)?;

                let current_position_to_world = _state.camera.screen_to_world(cursor);
                _state.left_pressed_at = Some(current_position_to_world);
                None
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                let cursor = _cursor.position_in(_bounds)?;

                let p1 = _state.left_pressed_at.take()?;
                let p2 = _state.camera.screen_to_world(cursor);

                if is_worth_drawing(p1, p2) {
                    let m = Message::GraveCreated((p1, p2).into());
                    return Some(Action::publish(m));
                }
                None
            }
            iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                let zoom_amount = match delta {
                    iced::mouse::ScrollDelta::Lines { y, .. } => *y as f32 * 0.1,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => *y as f32 * 0.001,
                };
                _state.camera.zoom = (_state.camera.zoom + zoom_amount).clamp(0.1, 10.0);
                return Some(Action::request_redraw());
            }
            _ => None,
        }
    }
}

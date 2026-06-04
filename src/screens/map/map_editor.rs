use super::Camera;
use super::{ToolbarAction, ToolBar};
use super::is_worth_drawing;
use iced::widget::{Action, canvas, container};
use iced::widget::{column, row};
use iced::{Element, Point, Renderer, Theme};

use crate::models::Grave;
pub struct MapEditor {
    graves: Vec<Grave>,
    toolbar: ToolBar,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    GraveCreated(Grave),
    ToolBarAction(ToolbarAction)
}

impl Default for MapEditor {
    fn default() -> Self {
        Self {
            graves: vec![],
            toolbar: ToolBar::default(),
        }
    }
}

impl MapEditor {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::GraveCreated(grave) => self.graves.push(grave),
            Message::ToolBarAction(action) => self.toolbar.update(action),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(column![
            canvas(self)
                .height(iced::Length::Fill)
                .width(iced::Length::Fill),
            self.toolbar.view().map(Message::ToolBarAction)
        ])
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
    current_drag_position: Option<Point>,
    grab_started_at: Option<Point>,
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

        if let Some(current_drag) = _state.current_drag_position {
            let ghost_grave: Grave =
                (_state.left_pressed_at.unwrap_or(current_drag), current_drag).into();
            let ghost_to_screen = _state.camera.world_to_screen(Point::new(
                ghost_grave.coordinate.top_left_x(),
                ghost_grave.coordinate.top_left_y(),
            ));
            let rect = iced::Rectangle {
                x: ghost_to_screen.x,
                y: ghost_to_screen.y,
                width: ghost_grave.coordinate.width() * zoom,
                height: ghost_grave.coordinate.height() * zoom,
            };
            let path = canvas::Path::rectangle(rect.position(), rect.size());

            frame.stroke(
                &path,
                canvas::Stroke {
                    width: 2.0,
                    style: canvas::Style::Solid(iced::Color::WHITE),
                    line_dash: canvas::LineDash {
                        segments: &[6.0, 4.0],
                        offset: 0,
                    },
                    ..Default::default()
                },
            );
        }
        for grave in &self.graves {
            let grave_to_screen = _state.camera.world_to_screen(Point::new(
                grave.coordinate.top_left_x(),
                grave.coordinate.top_left_y(),
            ));
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

                match self.toolbar.selected_action {
                    ToolbarAction::Draw => {
                        let current_position_to_world = _state.camera.screen_to_world(cursor);
                        _state.left_pressed_at = Some(current_position_to_world);
                    }
                    ToolbarAction::Grab => {
                        _state.grab_started_at = Some(cursor);
                    }
                }

                None
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                let cursor = _cursor.position_in(_bounds)?;

                match self.toolbar.selected_action {
                    ToolbarAction::Draw => {
                        _state.current_drag_position = None;

                        let p1 = _state.left_pressed_at.take()?;
                        let p2 = _state.camera.screen_to_world(cursor);

                        if is_worth_drawing(p1, p2) {
                            let m = Message::GraveCreated((p1, p2).into());
                            return Some(Action::publish(m));
                        }
                    }
                    ToolbarAction::Grab => {
                        _state.grab_started_at = None;
                    }
                }

                None
            }
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position: _ }) => {
                let cursor = _cursor.position_in(_bounds)?;

                match self.toolbar.selected_action {
                    ToolbarAction::Draw => {
                        if _state.left_pressed_at.is_some() {
                            let current_position_to_world = _state.camera.screen_to_world(cursor);
                            _state.current_drag_position = Some(current_position_to_world);
                            return Some(Action::request_redraw());
                        }
                    }
                    ToolbarAction::Grab => {
                        if let Some(previous_cursor) = _state.grab_started_at {
                            let delta = cursor - previous_cursor;

                            _state.camera.offset.x -= delta.x / _state.camera.zoom;
                            _state.camera.offset.y -= delta.y / _state.camera.zoom;
                            _state.grab_started_at = Some(cursor);

                            return Some(Action::request_redraw());
                        }
                    }
                }

                None
            }
            iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                let cursor = _cursor.position_in(_bounds)?;
                let before_zoom_cursor = _state.camera.screen_to_world(cursor);
                let zoom_amount = match delta {
                    iced::mouse::ScrollDelta::Lines { y, .. } => *y as f32 * 0.1,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => *y as f32 * 0.001,
                };
                _state.camera.zoom = (_state.camera.zoom + zoom_amount).clamp(0.1, 10.0);
                let after_zoom_curor = _state.camera.screen_to_world(cursor);
                _state.camera.offset.x += before_zoom_cursor.x - after_zoom_curor.x;
                _state.camera.offset.y += before_zoom_cursor.y - after_zoom_curor.y;
                return Some(Action::request_redraw());
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        match self.toolbar.selected_action {
            ToolbarAction::Draw => iced::mouse::Interaction::Crosshair,
            ToolbarAction::Grab => {
                if state.grab_started_at.is_some() {
                    iced::mouse::Interaction::Grabbing
                } else {
                    iced::mouse::Interaction::Grab
                }
            }
        }
    }
}

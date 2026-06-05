use super::Camera;
use super::{is_worth_drawing, find_grave_at};
use super::{ToolBar, ToolbarAction};
use iced::widget::{Action, canvas, container};
use iced::widget::{column};
use iced::{Element, Point, Renderer, Theme};

use crate::models::Grave;
use crate::screens::map::map_editor::Message::EraseAt;
pub struct MapEditor {
    graves: Vec<Grave>,
    toolbar: ToolBar,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    GraveCreated(Grave),
    ToolBarAction(ToolbarAction),
    EraseAt(iced::Point)
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
            Message::EraseAt(point) => {
                let grave_to_remove_idx = find_grave_at(&self.graves, point);
                if let Some(idx) = grave_to_remove_idx {
                    self.graves.remove(idx);
                }
            }
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

        if self.toolbar.show_grid {
            draw_grid(&mut frame, &_state.camera, bounds);
        }

        draw_grave_preview(&mut frame, zoom, _state);
        
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
                    _ => {}
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
                    ToolbarAction::StampGrave => {
                        let top_left = _state.camera.screen_to_world(cursor);
                        let bottom_right = Point::new(top_left.x + 10.0, top_left.y + 30.0);

                        return Some(Action::publish(Message::GraveCreated(
                            (top_left, bottom_right).into(),
                        )));
                    }
                    ToolbarAction::Grab => {
                        _state.grab_started_at = None;
                    }
                    ToolbarAction::Erase => {
                        let to_world = _state.camera.screen_to_world(cursor);
                        return Some(Action::publish(EraseAt(to_world)))
                    }
                    _  => {}
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
                    ToolbarAction::StampGrave => {}
                    ToolbarAction::Grab => {
                        if let Some(previous_cursor) = _state.grab_started_at {
                            let delta = cursor - previous_cursor;

                            _state.camera.offset.x -= delta.x / _state.camera.zoom;
                            _state.camera.offset.y -= delta.y / _state.camera.zoom;
                            _state.grab_started_at = Some(cursor);

                            return Some(Action::request_redraw());
                        }
                    }
                    _ => {}
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
            ToolbarAction::Draw | ToolbarAction::StampGrave => iced::mouse::Interaction::Crosshair,
            ToolbarAction::Grab => {
                if state.grab_started_at.is_some() {
                    iced::mouse::Interaction::Grabbing
                } else {
                    iced::mouse::Interaction::Grab
                }
            }
            ToolbarAction::Erase => iced::mouse::Interaction::NoDrop,
            _ => iced::mouse::Interaction::Idle,
        }
    }
}

fn draw_grid(frame: &mut canvas::Frame, camera: &Camera, bounds: iced::Rectangle) {
    const SQUARE_SIZE: f32 = 50.0;
    const LABEL_INTERVAL: i32 = 2;

    let top_left = camera.screen_to_world(Point::ORIGIN);
    let bottom_right = camera.screen_to_world(Point::new(bounds.width, bounds.height));
    let line_color = iced::Color::from_rgba(0.55, 0.72, 0.72, 0.25);
    let axis_color = iced::Color::from_rgba(0.65, 0.90, 0.88, 0.55);
    let label_color = iced::Color::from_rgba(0.75, 0.90, 0.88, 0.75);

    let start_x = (top_left.x / SQUARE_SIZE).floor() as i32;
    let end_x = (bottom_right.x / SQUARE_SIZE).ceil() as i32;
    let start_y = (top_left.y / SQUARE_SIZE).floor() as i32;
    let end_y = (bottom_right.y / SQUARE_SIZE).ceil() as i32;

    for index in start_x..=end_x {
        let world_x = index as f32 * SQUARE_SIZE;
        let screen_x = camera.world_to_screen(Point::new(world_x, 0.0)).x;
        let path = canvas::Path::line(
            Point::new(screen_x, 0.0),
            Point::new(screen_x, bounds.height),
        );

        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(if index == 0 { axis_color } else { line_color })
                .with_width(if index == 0 { 2.0 } else { 1.0 }),
        );

        if index % LABEL_INTERVAL == 0 {
            frame.fill_text(canvas::Text {
                content: format!("{world_x:.0}"),
                position: Point::new(screen_x + 4.0, 4.0),
                color: label_color,
                size: iced::Pixels(11.0),
                ..Default::default()
            });
        }
    }

    for index in start_y..=end_y {
        let world_y = index as f32 * SQUARE_SIZE;
        let screen_y = camera.world_to_screen(Point::new(0.0, world_y)).y;
        let path = canvas::Path::line(
            Point::new(0.0, screen_y),
            Point::new(bounds.width, screen_y),
        );

        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(if index == 0 { axis_color } else { line_color })
                .with_width(if index == 0 { 2.0 } else { 1.0 }),
        );

        if index % LABEL_INTERVAL == 0 {
            frame.fill_text(canvas::Text {
                content: format!("{world_y:.0}"),
                position: Point::new(4.0, screen_y + 4.0),
                color: label_color,
                size: iced::Pixels(11.0),
                ..Default::default()
            });
        }
    }
}


fn draw_grave_preview(frame: &mut canvas::Frame, zoom: f32, _state: &CanvasState) {
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
}
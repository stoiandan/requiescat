use super::Camera;
use super::drawing;
use super::geometry::find_grave_at;
use super::interaction;
use super::{Tool, Toolbar, ToolbarAction};
use iced::widget::column;
use iced::widget::{canvas, container};
use iced::{Element, Point, Renderer, Theme};

use crate::models::Grave;
pub struct MapEditor {
    graves: Vec<Grave>,
    toolbar: Toolbar,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    GraveCreated(Grave),
    ToolBarAction(ToolbarAction),
    EraseAt(iced::Point),
    MoveGrave { index: usize, delta: iced::Vector },
}

impl Default for MapEditor {
    fn default() -> Self {
        Self {
            graves: vec![],
            toolbar: Toolbar::default(),
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
            Message::MoveGrave { index, delta } => {
                if let Some(selected_grave) = self.graves.get_mut(index) {
                    selected_grave.translate(delta);
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

    pub(super) fn graves(&self) -> &[Grave] {
        &self.graves
    }

    pub(super) fn selected_tool(&self) -> Tool {
        self.toolbar.selected_tool()
    }

    pub(super) fn show_grid(&self) -> bool {
        self.toolbar.show_grid()
    }
}

#[derive(Default)]
pub struct CanvasState {
    pub(super) drag: DragState,
    pub(super) camera: Camera,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) enum DragState {
    #[default]
    None,
    Drawing {
        start: Point,
        current: Point,
    },
    Panning {
        previous_cursor: Point,
    },
    MovingGrave {
        index: usize,
        previous_cursor: Point,
    },
}

impl CanvasState {
    pub(super) fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn left_pressed_at(&self) -> Option<Point> {
        match self.drag {
            DragState::Drawing { start, .. } => Some(start),
            _ => None,
        }
    }

    pub fn current_drag_position(&self) -> Option<Point> {
        match self.drag {
            DragState::Drawing { current, .. } => Some(current),
            _ => None,
        }
    }
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

        if self.show_grid() {
            drawing::grid(&mut frame, &_state.camera, bounds);
        }

        drawing::grave_preview(&mut frame, _state);
        drawing::graves(&mut frame, &self.graves, &_state.camera);

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &iced::Event,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        interaction::handle_event(self, _state, _event, _bounds, _cursor)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        match self.selected_tool() {
            Tool::Draw | Tool::StampGrave => iced::mouse::Interaction::Crosshair,
            Tool::Grab => match state.drag {
                DragState::Panning { .. } | DragState::MovingGrave { .. } => {
                    iced::mouse::Interaction::Grabbing
                }
                _ => iced::mouse::Interaction::Grab,
            },
            Tool::Erase => iced::mouse::Interaction::NoDrop,
        }
    }
}

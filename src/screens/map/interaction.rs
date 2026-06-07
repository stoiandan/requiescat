use iced::widget::{Action, canvas};
use iced::{Point, Size, Vector};

use super::Tool;
use super::geometry::is_worth_drawing;
use super::map_editor::{CanvasState, DragState, MapEditor, Message};
use crate::models::GraveRectangle;

pub fn handle_event(
    editor: &MapEditor,
    state: &mut CanvasState,
    event: &iced::Event,
    bounds: iced::Rectangle,
    cursor: iced::mouse::Cursor,
) -> Option<canvas::Action<Message>> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
            let cursor = cursor.position_in(bounds)?;
            handle_left_press(editor, state, cursor);
            None
        }
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            let cursor = cursor.position_in(bounds)?;
            handle_left_release(editor, state, cursor)
        }
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
            let cursor = cursor.position_in(bounds)?;
            handle_cursor_moved(editor, state, cursor)
        }
        iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
            let cursor = cursor.position_in(bounds)?;
            let zoom_amount = match delta {
                iced::mouse::ScrollDelta::Lines { y, .. } => *y as f32 * 0.1,
                iced::mouse::ScrollDelta::Pixels { y, .. } => *y as f32 * 0.001,
            };

            state.camera.zoom_at(cursor, zoom_amount);

            Some(Action::request_redraw())
        }
        _ => None,
    }
}

fn handle_left_press(editor: &MapEditor, state: &mut CanvasState, cursor: Point) {
    match editor.selected_tool() {
        Tool::Select => {}
        Tool::Draw => {
            let current_position_to_world = state.camera.screen_to_world(cursor);
            state.drag = DragState::Drawing {
                start: current_position_to_world,
                current: current_position_to_world,
            };
        }
        Tool::Grab => {
            let world_cursor = state.camera.screen_to_world(cursor);

            state.drag = if let Some(id) = editor.cemetery().grave_at(world_cursor) {
                DragState::MovingGrave {
                    id,
                    previous_cursor: cursor,
                }
            } else {
                DragState::Panning {
                    previous_cursor: cursor,
                }
            };
        }
        Tool::StampGrave | Tool::Erase => {}
    }
}

fn handle_left_release(
    editor: &MapEditor,
    state: &mut CanvasState,
    cursor: Point,
) -> Option<canvas::Action<Message>> {
    match editor.selected_tool() {
        Tool::Select => {
            let to_world = state.camera.screen_to_world(cursor);
            return Some(Action::publish(Message::SelectGrave(
                editor.cemetery().grave_at(to_world),
            )));
        }
        Tool::Draw => {
            let DragState::Drawing { start, current } = state.drag else {
                return None;
            };

            state.drag = DragState::None;

            if is_worth_drawing(start, current) {
                return Some(Action::publish(Message::CreateGrave(
                    GraveRectangle::from_corners(start, current),
                )));
            }
        }
        Tool::StampGrave => {
            let top_left = state.camera.screen_to_world(cursor);

            return Some(Action::publish(Message::CreateGrave(
                GraveRectangle::from_top_left_size(top_left, Size::new(100.0, 200.0)),
            )));
        }
        Tool::Grab => {
            state.drag = DragState::None;
        }
        Tool::Erase => {
            let to_world = state.camera.screen_to_world(cursor);
            if let Some(id) = editor.cemetery().grave_at(to_world) {
                return Some(Action::publish(Message::EraseGrave(id)));
            }
        }
    }

    None
}

fn handle_cursor_moved(
    editor: &MapEditor,
    state: &mut CanvasState,
    cursor: Point,
) -> Option<canvas::Action<Message>> {
    match editor.selected_tool() {
        Tool::Select => {}
        Tool::Draw => {
            if let DragState::Drawing { start, .. } = state.drag {
                state.drag = DragState::Drawing {
                    start,
                    current: state.camera.screen_to_world(cursor),
                };

                return Some(Action::request_redraw());
            }
        }
        Tool::StampGrave => {}
        Tool::Grab => match state.drag {
            DragState::MovingGrave {
                id,
                previous_cursor,
            } => {
                let delta = cursor - previous_cursor;
                let world_delta = state.camera.canvas_delta_to_world(delta);

                state.drag = DragState::MovingGrave {
                    id,
                    previous_cursor: cursor,
                };

                return Some(Action::publish(Message::MoveGrave {
                    id,
                    delta: world_delta,
                }));
            }
            DragState::Panning { previous_cursor } => {
                let delta: Vector = cursor - previous_cursor;

                state.camera.pan_by_canvas_delta(delta);
                state.drag = DragState::Panning {
                    previous_cursor: cursor,
                };

                return Some(Action::request_redraw());
            }
            DragState::None | DragState::Drawing { .. } => {}
        },
        Tool::Erase => {}
    }

    None
}

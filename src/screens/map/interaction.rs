use iced::widget::{Action, canvas};
use iced::{Point, Size, Vector};

use super::Tool;
use super::geometry::is_worth_drawing;
use super::map_canvas::{CanvasState, DragState};
use super::map_editor::{MapEditor, Message};
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
            let cursor_in_bounds = cursor.position_in(bounds);
            let cursor_from_canvas = cursor.position_from(Point::new(bounds.x, bounds.y));

            handle_left_release(editor, state, cursor_in_bounds, cursor_from_canvas)
        }
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
            let Some(cursor) = cursor.position_in(bounds) else {
                return Some(Action::publish(Message::CanvasCursorChanged(None)));
            };

            handle_cursor_moved(editor, state, cursor).or_else(|| {
                Some(Action::publish(Message::CanvasCursorChanged(Some(
                    editor.camera().screen_to_world(cursor),
                ))))
            })
        }
        iced::Event::Mouse(iced::mouse::Event::CursorLeft) => {
            Some(Action::publish(Message::CanvasCursorChanged(None)))
        }
        iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
            let cursor = cursor.position_in(bounds)?;
            let zoom_amount = match delta {
                iced::mouse::ScrollDelta::Lines { y, .. } => *y * 0.1,
                iced::mouse::ScrollDelta::Pixels { y, .. } => *y * 0.001,
            };

            Some(Action::publish(Message::ZoomCamera {
                cursor,
                amount: zoom_amount,
            }))
        }
        _ => None,
    }
}

fn handle_left_press(editor: &MapEditor, state: &mut CanvasState, cursor: Point) {
    if let Some(id) = rotation_handle_at(editor, cursor) {
        state.drag = DragState::RotatingObject { id };
        return;
    }

    match editor.selected_tool() {
        Tool::Select => {}
        Tool::Draw | Tool::DrawDelimiter => {
            let current_position_to_world = editor.camera().screen_to_world(cursor);
            state.drag = DragState::Drawing {
                start: current_position_to_world,
                current: current_position_to_world,
            };
        }
        Tool::Grab => {
            let world_cursor = editor.camera().screen_to_world(cursor);

            state.drag = if let Some(id) = editor.object_at(world_cursor) {
                DragState::MovingObject {
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
    cursor_in_bounds: Option<Point>,
    cursor_from_canvas: Option<Point>,
) -> Option<canvas::Action<Message>> {
    let drag = std::mem::take(&mut state.drag);

    match drag {
        DragState::Drawing { start, current } => {
            let selected_tool = editor.selected_tool();
            let end = cursor_from_canvas
                .map(|cursor| editor.camera().screen_to_world(cursor))
                .unwrap_or(current);

            if !is_worth_drawing(start, end) {
                return Some(Action::request_redraw());
            }

            match selected_tool {
                Tool::Draw | Tool::DrawDelimiter => {
                    let rectangle = GraveRectangle::from_corners(start, end);
                    let message = match selected_tool {
                        Tool::Draw => Message::CreateGrave(rectangle),
                        Tool::DrawDelimiter => Message::CreateDelimiter(rectangle),
                        _ => unreachable!(),
                    };
                    return Some(Action::publish(message));
                }
                _ => {
                    return Some(Action::request_redraw());
                }
            }
        }
        DragState::Panning { .. } => {
            return None;
        }
        DragState::MovingObject { .. } | DragState::RotatingObject { .. } => {
            return Some(Action::publish(Message::CommitPendingChanges));
        }
        DragState::None => {}
    }

    let cursor = cursor_in_bounds?;

    match editor.selected_tool() {
        Tool::Select => {
            let to_world = editor.camera().screen_to_world(cursor);
            return Some(Action::publish(Message::SelectGrave(
                editor.cemetery().grave_at(to_world),
            )));
        }
        Tool::Draw => {}
        Tool::DrawDelimiter => {}
        Tool::StampGrave => {
            let top_left = editor.camera().screen_to_world(cursor);

            return Some(Action::publish(Message::CreateGrave(
                GraveRectangle::from_top_left_size(top_left, Size::new(100.0, 200.0)),
            )));
        }
        Tool::Grab => {}
        Tool::Erase => {
            let to_world = editor.camera().screen_to_world(cursor);
            if let Some(id) = editor.cemetery().grave_at(to_world) {
                return Some(Action::publish(Message::EraseGrave(id)));
            }
            if let Some(id) = editor.cemetery().delimiter_at(to_world) {
                return Some(Action::publish(Message::EraseDelimiter(id)));
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
    if let DragState::RotatingObject { id } = state.drag {
        let world_cursor = editor.camera().screen_to_world(cursor);
        let rotation_degrees = editor.rotation_degrees_for_cursor(id, world_cursor)?;

        return Some(Action::publish(Message::RotateMapObject {
            id,
            rotation_degrees,
        }));
    }

    match editor.selected_tool() {
        Tool::Select => {}
        Tool::Draw | Tool::DrawDelimiter => {
            if let DragState::Drawing { start, .. } = state.drag {
                state.drag = DragState::Drawing {
                    start,
                    current: editor.camera().screen_to_world(cursor),
                };

                return Some(Action::request_redraw());
            }
        }
        Tool::StampGrave => {}
        Tool::Grab => match state.drag {
            DragState::MovingObject {
                id,
                previous_cursor,
            } => {
                let delta = cursor - previous_cursor;
                let world_delta = editor.camera().canvas_delta_to_world(delta);

                state.drag = DragState::MovingObject {
                    id,
                    previous_cursor: cursor,
                };

                return Some(Action::publish(Message::MoveMapObject {
                    id,
                    delta: world_delta,
                }));
            }
            DragState::Panning { previous_cursor } => {
                let delta: Vector = cursor - previous_cursor;

                state.drag = DragState::Panning {
                    previous_cursor: cursor,
                };

                return Some(Action::publish(Message::PanCamera(delta)));
            }
            DragState::None | DragState::Drawing { .. } | DragState::RotatingObject { .. } => {}
        },
        Tool::Erase => {}
    }

    None
}

fn rotation_handle_at(editor: &MapEditor, cursor: Point) -> Option<super::map_editor::MapObjectId> {
    const HIT_RADIUS: f32 = 10.0;

    editor.rotation_targets().into_iter().rev().find(|id| {
        let Some(handle) = editor
            .rotation_handle_position(*id)
            .map(|point| editor.camera().world_to_screen(point))
        else {
            return false;
        };
        let delta = cursor - handle;

        delta.x.hypot(delta.y) <= HIT_RADIUS
    })
}

#[cfg(test)]
mod tests {
    use iced::mouse;

    use super::*;
    use crate::screens::map::ToolbarAction;

    const BOUNDS: iced::Rectangle = iced::Rectangle {
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 80.0,
    };

    fn editor_with_tool(tool: Tool) -> MapEditor {
        let mut editor = MapEditor::default();
        editor.update(Message::ToolBarAction(ToolbarAction::SelectTool(tool)));
        editor
    }

    #[test]
    fn release_outside_canvas_finishes_drawing_and_clears_drag() {
        let editor = editor_with_tool(Tool::Draw);
        let mut state = CanvasState::default();

        handle_event(
            &editor,
            &mut state,
            &iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            BOUNDS,
            mouse::Cursor::Available(Point::new(30.0, 40.0)),
        );

        let action = handle_event(
            &editor,
            &mut state,
            &iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            BOUNDS,
            mouse::Cursor::Available(Point::new(140.0, 130.0)),
        );

        assert!(action.is_some());
        assert!(matches!(state.drag, DragState::None));
    }

    #[test]
    fn unavailable_cursor_still_clears_an_active_drag() {
        let editor = editor_with_tool(Tool::Grab);
        let mut state = CanvasState::default();
        state.drag = DragState::Panning {
            previous_cursor: Point::new(20.0, 20.0),
        };

        handle_event(
            &editor,
            &mut state,
            &iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            BOUNDS,
            mouse::Cursor::Unavailable,
        );

        assert!(matches!(state.drag, DragState::None));
    }
}

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
            let cursor_in_bounds = cursor.position_in(bounds);
            let cursor_from_canvas = cursor.position_from(Point::new(bounds.x, bounds.y));

            handle_left_release(editor, state, cursor_in_bounds, cursor_from_canvas)
        }
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
            let cursor = cursor.position_in(bounds)?;
            handle_cursor_moved(editor, state, cursor)
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
    match editor.selected_tool() {
        Tool::Select => {}
        Tool::Draw => {
            let current_position_to_world = editor.camera().screen_to_world(cursor);
            state.drag = DragState::Drawing {
                start: current_position_to_world,
                current: current_position_to_world,
            };
        }
        Tool::Grab => {
            let world_cursor = editor.camera().screen_to_world(cursor);

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
    cursor_in_bounds: Option<Point>,
    cursor_from_canvas: Option<Point>,
) -> Option<canvas::Action<Message>> {
    let drag = std::mem::take(&mut state.drag);

    match drag {
        DragState::Drawing { start, current } => {
            if editor.selected_tool() != Tool::Draw {
                return Some(Action::request_redraw());
            }

            let end = cursor_from_canvas
                .map(|cursor| editor.camera().screen_to_world(cursor))
                .unwrap_or(current);

            if is_worth_drawing(start, end) {
                return Some(Action::publish(Message::CreateGrave(
                    GraveRectangle::from_corners(start, end),
                )));
            }

            return Some(Action::request_redraw());
        }
        DragState::Panning { .. } => {
            return None;
        }
        DragState::MovingGrave { .. } => {
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
                    current: editor.camera().screen_to_world(cursor),
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
                let world_delta = editor.camera().canvas_delta_to_world(delta);

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

                state.drag = DragState::Panning {
                    previous_cursor: cursor,
                };

                return Some(Action::publish(Message::PanCamera(delta)));
            }
            DragState::None | DragState::Drawing { .. } => {}
        },
        Tool::Erase => {}
    }

    None
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
        let mut state = CanvasState {
            drag: DragState::Panning {
                previous_cursor: Point::new(20.0, 20.0),
            },
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

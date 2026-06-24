use std::cell::Cell;

use iced::widget::canvas;
use iced::{Point, Renderer, Theme};

use super::Tool;
use super::delimiter_drawing;
use super::drawing;
use super::interaction;
use super::map_editor::{MapEditor, MapObjectId, Message};
use crate::localization::{Language, Localizer, MessageId};
use crate::models::GraveId;

pub struct CanvasState {
    pub(super) drag: DragState,
    map_cache: canvas::Cache<Renderer>,
    map_cache_key: Cell<Option<MapCacheKey>>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            drag: DragState::default(),
            map_cache: canvas::Cache::new(),
            map_cache_key: Cell::new(None),
        }
    }
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
    MovingObject {
        id: MapObjectId,
        previous_cursor: Point,
    },
    RotatingObject {
        id: MapObjectId,
    },
}

impl CanvasState {
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

pub(super) struct LocalizedMapCanvas<'a> {
    pub(super) editor: &'a MapEditor,
    pub(super) localizer: &'a Localizer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MapCacheKey {
    render_revision: u64,
    zoom: u32,
    offset_x: u32,
    offset_y: u32,
    selected_grave: Option<GraveId>,
    show_grid: bool,
    language: Language,
}

impl MapCacheKey {
    fn new(editor: &MapEditor, language: Language) -> Self {
        let camera = editor.camera();

        Self {
            render_revision: editor.render_revision(),
            zoom: camera.zoom.to_bits(),
            offset_x: camera.offset.x.to_bits(),
            offset_y: camera.offset.y.to_bits(),
            selected_grave: editor.selected_grave(),
            show_grid: editor.show_grid(),
            language,
        }
    }
}

impl canvas::Program<Message> for LocalizedMapCanvas<'_> {
    type State = CanvasState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _: &Theme,
        bounds: iced::Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let cache_key = MapCacheKey::new(self.editor, self.localizer.language());
        if state.map_cache_key.get() != Some(cache_key) {
            state.map_cache.clear();
            state.map_cache_key.set(Some(cache_key));
        }

        let map = state.map_cache.draw(renderer, bounds.size(), |frame| {
            if self.editor.show_grid() {
                drawing::grid(frame, &self.editor.camera(), bounds);
            }

            delimiter_drawing::all(frame, self.editor.cemetery(), &self.editor.camera(), bounds);

            drawing::graves(
                frame,
                self.editor.cemetery(),
                &self.editor.camera(),
                bounds,
                self.editor.selected_grave(),
                |grave_id| {
                    self.localizer
                        .value(MessageId::GraveCanvas, "grave", grave_id.to_string())
                },
            );
        });

        if state.current_drag_position().is_some() || !self.editor.rotation_targets().is_empty() {
            let mut preview = canvas::Frame::new(renderer, bounds.size());
            match self.editor.selected_tool() {
                Tool::Draw => drawing::grave_preview(&mut preview, state, &self.editor.camera()),
                Tool::DrawDelimiter => delimiter_drawing::preview(
                    &mut preview,
                    state,
                    &self.editor.camera(),
                    self.editor.selected_delimiter_type(),
                ),
                _ => {}
            }
            drawing::rotation_handles(&mut preview, self.editor);
            vec![map, preview.into_geometry()]
        } else {
            vec![map]
        }
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        interaction::handle_event(self.editor, state, event, bounds, cursor)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        match self.editor.selected_tool() {
            Tool::Select => iced::mouse::Interaction::Pointer,
            Tool::Draw | Tool::StampGrave | Tool::DrawDelimiter => {
                iced::mouse::Interaction::Crosshair
            }
            Tool::Grab => match state.drag {
                DragState::Panning { .. }
                | DragState::MovingObject { .. }
                | DragState::RotatingObject { .. } => iced::mouse::Interaction::Grabbing,
                _ => iced::mouse::Interaction::Grab,
            },
            Tool::Erase => iced::mouse::Interaction::NoDrop,
        }
    }
}

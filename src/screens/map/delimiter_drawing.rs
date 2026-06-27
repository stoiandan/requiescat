use iced::widget::canvas;
use iced::{Point, Rectangle, Vector};

use super::Camera;
use super::map_canvas::CanvasState;
use crate::models::{Cemetery, Delimiter, DelimiterType};

const MIN_SCREEN_LENGTH: f32 = 10.0;
const WALL_AMPLITUDE: f32 = 10.0;
const WALL_STEP: f32 = 20.0;
const ROAD_HALF_WIDTH: f32 = 8.0;
const VISIBILITY_PADDING: f32 = WALL_AMPLITUDE.max(ROAD_HALF_WIDTH);

pub fn preview(
    frame: &mut canvas::Frame,
    state: &CanvasState,
    camera: &Camera,
    delimiter_type: DelimiterType,
    color: iced::Color,
) {
    let Some(current_drag) = state.current_drag_position() else {
        return;
    };

    let start = state.left_pressed_at().unwrap_or(current_drag);

    draw_world_delimiter(
        frame,
        start,
        current_drag,
        camera,
        color,
        delimiter_type,
        true,
    );
}

pub fn all(frame: &mut canvas::Frame, cemetery: &Cemetery, camera: &Camera, bounds: Rectangle) {
    for delimiter in cemetery.delimiters() {
        let (start, end) = delimiter_line_points(delimiter);
        let Some(line) = ScreenLine::from_world(start, end, camera) else {
            continue;
        };

        if !line.is_visible(bounds) {
            continue;
        }

        draw_delimiter(
            frame,
            line,
            delimiter.color().to_iced(),
            delimiter.delimiter_type(),
            false,
        );
    }
}

fn draw_world_delimiter(
    frame: &mut canvas::Frame,
    start: iced::Point,
    current_drag: iced::Point,
    camera: &Camera,
    color: iced::Color,
    delimiter_type: DelimiterType,
    preview: bool,
) {
    let Some(line) = ScreenLine::from_world(start, current_drag, camera) else {
        return;
    };

    draw_delimiter(frame, line, color, delimiter_type, preview)
}

fn draw_delimiter(
    frame: &mut canvas::Frame,
    line: ScreenLine,
    color: iced::Color,
    delimiter_type: DelimiterType,
    preview: bool,
) {
    match delimiter_type {
        DelimiterType::Wall => draw_wall(frame, line, color, preview),
        DelimiterType::Road => draw_road(frame, line, color, preview),
    }
}

fn draw_wall(frame: &mut canvas::Frame, line: ScreenLine, color: iced::Color, preview: bool) {
    frame.stroke(
        &wall_zig_zag(line),
        canvas::Stroke::default()
            .with_color(color)
            .with_width(if preview { 1.5 } else { 2.0 }),
    );
}

fn draw_road(frame: &mut canvas::Frame, line: ScreenLine, color: iced::Color, preview: bool) {
    let dash = canvas::LineDash {
        segments: &[10.0, 6.0],
        offset: 0,
    };

    for offset in [
        -line.normal * ROAD_HALF_WIDTH,
        line.normal * ROAD_HALF_WIDTH,
    ] {
        frame.stroke(
            &canvas::Path::line(line.start + offset, line.end + offset),
            canvas::Stroke {
                width: if preview { 1.5 } else { 2.0 },
                style: canvas::Style::Solid(color),
                line_dash: dash,
                ..Default::default()
            },
        );
    }
}

fn wall_zig_zag(line: ScreenLine) -> canvas::Path {
    let amplitude = WALL_AMPLITUDE.min(line.length / 3.0);

    canvas::Path::new(|builder| {
        builder.move_to(line.start);

        let mut distance = 0.0;
        let mut zig = true;
        while distance < line.length {
            distance = (distance + WALL_STEP).min(line.length);
            let center = line.point_at(distance);
            let offset = line.normal * if zig { -amplitude } else { amplitude };
            builder.line_to(center + offset);
            zig = !zig;
        }
    })
}

fn delimiter_line_points(delimiter: &Delimiter) -> (Point, Point) {
    let rectangle = delimiter.rectangle();
    let rotation_degrees = delimiter.rotation_degrees();
    let width = rectangle.size().width;

    (
        rectangle.point_at_rotated(width / 2.0, 0.0, rotation_degrees),
        rectangle.point_at_rotated(width / 2.0, rectangle.size().height, rotation_degrees),
    )
}

#[derive(Debug, Clone, Copy)]
struct ScreenLine {
    start: Point,
    end: Point,
    length: f32,
    normal: Vector,
}

impl ScreenLine {
    fn from_world(start: Point, end: Point, camera: &Camera) -> Option<Self> {
        Self::new(camera.world_to_screen(start), camera.world_to_screen(end))
    }

    fn new(start: Point, end: Point) -> Option<Self> {
        let delta = end - start;
        let length = (delta.x.powi(2) + delta.y.powi(2)).sqrt();
        if length < MIN_SCREEN_LENGTH {
            return None;
        }

        Some(Self {
            start,
            end,
            length,
            normal: Vector::new(-delta.y / length, delta.x / length),
        })
    }

    fn point_at(self, distance: f32) -> Point {
        let progress = distance / self.length;
        Point::new(
            self.start.x + (self.end.x - self.start.x) * progress,
            self.start.y + (self.end.y - self.start.y) * progress,
        )
    }

    fn is_visible(self, bounds: Rectangle) -> bool {
        let min_x = self.start.x.min(self.end.x) - VISIBILITY_PADDING;
        let max_x = self.start.x.max(self.end.x) + VISIBILITY_PADDING;
        let min_y = self.start.y.min(self.end.y) - VISIBILITY_PADDING;
        let max_y = self.start.y.max(self.end.y) + VISIBILITY_PADDING;

        max_x >= 0.0 && max_y >= 0.0 && min_x <= bounds.width && min_y <= bounds.height
    }
}

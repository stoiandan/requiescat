use iced::widget::canvas;
use iced::{Point, Rectangle};

use super::Camera;
use super::map_canvas::CanvasState;
use crate::models::{Cemetery, DelimiterType, GraveRectangle};

pub fn preview(
    frame: &mut canvas::Frame,
    state: &CanvasState,
    camera: &Camera,
    delimiter_type: DelimiterType,
) {
    let Some(current_drag) = state.current_drag_position() else {
        return;
    };

    let start = state.left_pressed_at().unwrap_or(current_drag);
    let rectangle = GraveRectangle::from_corners(start, current_drag);

    draw_shape(
        frame,
        rectangle,
        0.0,
        camera,
        iced::Color::WHITE,
        delimiter_type,
        true,
    );
}

pub fn all(frame: &mut canvas::Frame, cemetery: &Cemetery, camera: &Camera, bounds: Rectangle) {
    for delimiter in cemetery.delimiters().iter().filter(|delimiter| {
        rectangle_is_visible(
            delimiter.rectangle(),
            delimiter.rotation_degrees(),
            camera,
            bounds,
        )
    }) {
        draw_shape(
            frame,
            delimiter.rectangle(),
            delimiter.rotation_degrees(),
            camera,
            delimiter.color().to_iced(),
            delimiter.delimiter_type(),
            false,
        );
    }
}

fn draw_shape(
    frame: &mut canvas::Frame,
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    camera: &Camera,
    color: iced::Color,
    delimiter_type: DelimiterType,
    preview: bool,
) {
    match delimiter_type {
        DelimiterType::Wall => {
            draw_wall(frame, rectangle, rotation_degrees, camera, color, preview)
        }
        DelimiterType::Road => {
            draw_road(frame, rectangle, rotation_degrees, camera, color, preview)
        }
    }
}

fn draw_wall(
    frame: &mut canvas::Frame,
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    camera: &Camera,
    color: iced::Color,
    preview: bool,
) {
    let screen = ScreenRectangle::from_map(rectangle, rotation_degrees, camera);
    if screen.min_dimension() < 10.0 {
        return;
    }

    frame.stroke(
        &wall_zig_zag(screen),
        canvas::Stroke::default()
            .with_color(color)
            .with_width(if preview { 1.5 } else { 2.0 }),
    );
}

fn draw_road(
    frame: &mut canvas::Frame,
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    camera: &Camera,
    color: iced::Color,
    preview: bool,
) {
    let screen = ScreenRectangle::from_map(rectangle, rotation_degrees, camera);
    let dash = canvas::LineDash {
        segments: &[10.0, 6.0],
        offset: 0,
    };

    for quarter in [1, 4] {
        frame.stroke(
            &screen.quarter_line(quarter),
            canvas::Stroke {
                width: if preview { 1.5 } else { 2.0 },
                style: canvas::Style::Solid(color),
                line_dash: dash,
                ..Default::default()
            },
        );
    }
}

fn wall_zig_zag(screen: ScreenRectangle) -> canvas::Path {
    let x_center = screen.width / 2.0;
    let amplitude = (screen.width / 5.0).clamp(3.0, 10.0);
    let step = 20.0;
    let mut y = 0.0;
    let mut zig = true;

    canvas::Path::new(|builder| {
        builder.move_to(screen.point(x_center, 0.0));
        while y < screen.height {
            y = (y + step).min(screen.height);
            builder.line_to(screen.point(
                if zig {
                    x_center - amplitude
                } else {
                    x_center + amplitude
                },
                y,
            ));
            zig = !zig;
        }
    })
}

fn rectangle_is_visible(
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    camera: &Camera,
    bounds: Rectangle,
) -> bool {
    let corners = rectangle
        .corners_rotated(rotation_degrees)
        .map(|corner| camera.world_to_screen(corner));
    let min_x = corners
        .iter()
        .map(|corner| corner.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = corners
        .iter()
        .map(|corner| corner.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = corners
        .iter()
        .map(|corner| corner.y)
        .fold(f32::INFINITY, f32::min);
    let max_y = corners
        .iter()
        .map(|corner| corner.y)
        .fold(f32::NEG_INFINITY, f32::max);

    max_x >= 0.0 && max_y >= 0.0 && min_x <= bounds.width && min_y <= bounds.height
}

#[derive(Debug, Clone, Copy)]
struct ScreenRectangle {
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    width: f32,
    height: f32,
    camera: Camera,
}

impl ScreenRectangle {
    fn from_map(rectangle: GraveRectangle, rotation_degrees: f32, camera: &Camera) -> Self {
        Self {
            rectangle,
            rotation_degrees,
            width: rectangle.size().width,
            height: rectangle.size().height,
            camera: *camera,
        }
    }

    fn min_dimension(self) -> f32 {
        (self.width * self.camera.zoom).min(self.height * self.camera.zoom)
    }

    fn quarter_line(self, quarter: u32) -> canvas::Path {
        let quarter = quarter.clamp(1, 4);
        let x = self.width / quarter as f32;

        canvas::Path::line(self.point(x, 0.0), self.point(x, self.height))
    }

    fn point(self, x: f32, y: f32) -> Point {
        self.camera
            .world_to_screen(self.rectangle.point_at_rotated(x, y, self.rotation_degrees))
    }
}

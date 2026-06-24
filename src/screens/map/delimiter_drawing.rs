use iced::widget::canvas;
use iced::{Point, Rectangle, Size};

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
        camera,
        iced::Color::WHITE,
        delimiter_type,
        true,
    );
}

pub fn all(frame: &mut canvas::Frame, cemetery: &Cemetery, camera: &Camera, bounds: Rectangle) {
    for delimiter in cemetery
        .delimiters()
        .iter()
        .filter(|delimiter| rectangle_is_visible(delimiter.rectangle(), camera, bounds))
    {
        draw_shape(
            frame,
            delimiter.rectangle(),
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
    camera: &Camera,
    color: iced::Color,
    delimiter_type: DelimiterType,
    preview: bool,
) {
    match delimiter_type {
        DelimiterType::Wall => draw_wall(frame, rectangle, camera, color, preview),
        DelimiterType::Road => draw_road(frame, rectangle, camera, color, preview),
    }
}

fn draw_wall(
    frame: &mut canvas::Frame,
    rectangle: GraveRectangle,
    camera: &Camera,
    color: iced::Color,
    preview: bool,
) {
    let screen = ScreenRectangle::from_map(rectangle, camera);
    let outline = canvas::Path::rectangle(screen.top_left, screen.size);

    frame.stroke(
        &outline,
        canvas::Stroke {
            width: if preview { 2.0 } else { 3.0 },
            style: canvas::Style::Solid(color),
            line_dash: preview_dash(preview),
            ..Default::default()
        },
    );

    if screen.min_dimension() < 12.0 {
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
    camera: &Camera,
    color: iced::Color,
    preview: bool,
) {
    let screen = ScreenRectangle::from_map(rectangle, camera);
    let outline = canvas::Path::rectangle(screen.top_left, screen.size);
    let dash = canvas::LineDash {
        segments: &[10.0, 6.0],
        offset: 0,
    };

    frame.stroke(
        &outline,
        canvas::Stroke {
            width: if preview { 2.0 } else { 2.5 },
            style: canvas::Style::Solid(color),
            line_dash: dash,
            ..Default::default()
        },
    );

    frame.stroke(
        &screen.center_line(),
        canvas::Stroke {
            width: if preview { 1.5 } else { 2.0 },
            style: canvas::Style::Solid(color),
            line_dash: dash,
            ..Default::default()
        },
    );
}

fn wall_zig_zag(screen: ScreenRectangle) -> canvas::Path {
    let inset = 5.0_f32.min(screen.min_dimension() / 4.0);
    let left = screen.top_left.x + inset;
    let right = screen.right() - inset;
    let center_y = screen.center_y();
    let amplitude = (screen.size.height / 5.0).clamp(3.0, 10.0);
    let step = 12.0;
    let mut x = left;
    let mut high = true;

    canvas::Path::new(|builder| {
        builder.move_to(Point::new(left, center_y));
        while x < right {
            x = (x + step).min(right);
            builder.line_to(Point::new(
                x,
                if high {
                    center_y - amplitude
                } else {
                    center_y + amplitude
                },
            ));
            high = !high;
        }
    })
}

fn preview_dash(preview: bool) -> canvas::LineDash<'static> {
    if preview {
        canvas::LineDash {
            segments: &[6.0, 4.0],
            offset: 0,
        }
    } else {
        canvas::LineDash::default()
    }
}

fn rectangle_is_visible(rectangle: GraveRectangle, camera: &Camera, bounds: Rectangle) -> bool {
    let screen = ScreenRectangle::from_map(rectangle, camera);

    screen.right() >= 0.0
        && screen.bottom() >= 0.0
        && screen.top_left.x <= bounds.width
        && screen.top_left.y <= bounds.height
}

#[derive(Debug, Clone, Copy)]
struct ScreenRectangle {
    top_left: Point,
    size: Size,
}

impl ScreenRectangle {
    fn from_map(rectangle: GraveRectangle, camera: &Camera) -> Self {
        Self {
            top_left: camera.world_to_screen(rectangle.top_left()),
            size: rectangle.size() * camera.zoom,
        }
    }

    fn right(self) -> f32 {
        self.top_left.x + self.size.width
    }

    fn bottom(self) -> f32 {
        self.top_left.y + self.size.height
    }

    fn center_y(self) -> f32 {
        self.top_left.y + self.size.height / 2.0
    }

    fn min_dimension(self) -> f32 {
        self.size.width.min(self.size.height)
    }

    fn center_line(self) -> canvas::Path {
        canvas::Path::line(
            Point::new(self.top_left.x, self.center_y()),
            Point::new(self.right(), self.center_y()),
        )
    }
}

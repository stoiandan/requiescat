use iced::Point;

use crate::models::{Cemetery, DelimiterType, GraveColor, GraveRectangle};

use super::content::PdfContent;
use super::layout::MapTransform;
use super::map_geometry::PdfRectangle;

pub(super) fn render(content: &mut PdfContent, cemetery: &Cemetery, transform: &MapTransform) {
    for delimiter in cemetery.delimiters() {
        let (red, green, blue) = rgb(delimiter.color());
        content.stroke_color(red, green, blue);

        match delimiter.delimiter_type() {
            DelimiterType::Wall => render_wall(
                content,
                delimiter.rectangle(),
                delimiter.rotation_degrees(),
                transform,
            ),
            DelimiterType::Road => render_road(
                content,
                delimiter.rectangle(),
                delimiter.rotation_degrees(),
                transform,
            ),
        }
    }
}

fn render_wall(
    content: &mut PdfContent,
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    transform: &MapTransform,
) {
    let pdf_rectangle = PdfRectangle::from_map(rectangle, transform);

    if pdf_rectangle.min_dimension() < 12.0 {
        return;
    }

    content.line_width(2.25);
    for (start, end) in wall_zig_zag_segments(rectangle, rotation_degrees, transform) {
        content.line(start.x, start.y, end.x, end.y);
    }
}

fn render_road(
    content: &mut PdfContent,
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    transform: &MapTransform,
) {
    let size = rectangle.size();

    content.line_width(1.75);
    for index in [1.0, 4.0] {
        let x = size.width / 5.0 * index;
        let start = PdfRectangle::point_from_map(rectangle, x, 0.0, rotation_degrees, transform);
        let end =
            PdfRectangle::point_from_map(rectangle, x, size.height, rotation_degrees, transform);

        content.line(start.x, start.y, end.x, end.y);
    }
}

fn wall_zig_zag_segments(
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    transform: &MapTransform,
) -> Vec<(Point, Point)> {
    let size = rectangle.size();
    let inset = 5.0_f32.min(size.width.min(size.height) / 4.0);
    let left = inset;
    let right = size.width - inset;
    let center_y = size.height / 2.0;
    let amplitude = (size.height / 5.0).clamp(3.0, 10.0);
    let step = 12.0;
    let mut x = left;
    let mut previous =
        PdfRectangle::point_from_map(rectangle, left, center_y, rotation_degrees, transform);
    let mut high = true;
    let mut segments = Vec::new();

    while x < right {
        x = (x + step).min(right);
        let next_y = if high {
            center_y - amplitude
        } else {
            center_y + amplitude
        };
        let next = PdfRectangle::point_from_map(rectangle, x, next_y, rotation_degrees, transform);
        segments.push((previous, next));
        previous = next;
        high = !high;
    }

    segments
}

fn rgb(color: GraveColor) -> (f32, f32, f32) {
    let (red, green, blue) = color.to_rgb8();
    (
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
    )
}

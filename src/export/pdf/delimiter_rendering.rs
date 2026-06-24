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
            DelimiterType::Wall => render_wall(content, delimiter.rectangle(), transform),
            DelimiterType::Road => render_road(content, delimiter.rectangle(), transform),
        }
    }
}

fn render_wall(content: &mut PdfContent, rectangle: GraveRectangle, transform: &MapTransform) {
    let pdf_rectangle = PdfRectangle::from_map(rectangle, transform);

    content.line_width(2.25);
    pdf_rectangle.draw(content);
    content.stroke();

    if pdf_rectangle.min_dimension() < 12.0 {
        return;
    }

    content.line_width(1.5);
    for (start, end) in wall_zig_zag_segments(pdf_rectangle) {
        content.line(start.x, start.y, end.x, end.y);
    }
}

fn render_road(content: &mut PdfContent, rectangle: GraveRectangle, transform: &MapTransform) {
    let pdf_rectangle = PdfRectangle::from_map(rectangle, transform);
    let center_y = pdf_rectangle.center_y();

    content.line_width(1.75);
    pdf_rectangle.draw(content);
    content.stroke();
    content.line(pdf_rectangle.x, center_y, pdf_rectangle.right(), center_y);
}

fn wall_zig_zag_segments(rectangle: PdfRectangle) -> Vec<(Point, Point)> {
    let inset = 5.0_f32.min(rectangle.min_dimension() / 4.0);
    let left = rectangle.x + inset;
    let right = rectangle.right() - inset;
    let center_y = rectangle.center_y();
    let amplitude = (rectangle.height / 5.0).clamp(3.0, 10.0);
    let step = 12.0;
    let mut x = left;
    let mut previous = Point::new(left, center_y);
    let mut high = true;
    let mut segments = Vec::new();

    while x < right {
        x = (x + step).min(right);
        let next = Point::new(
            x,
            if high {
                center_y - amplitude
            } else {
                center_y + amplitude
            },
        );
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

use std::collections::HashMap;

use iced::Point;

use super::PdfExportOptions;
use super::content::PdfContent;
use super::delimiter_rendering;
use super::layout::{CemeteryBounds, MapTransform, PageLayout, grid_interval};
use super::map_geometry::PdfRectangle;
use crate::label_layout;
use crate::models::{Cemetery, GraveColor, GraveId};

pub(super) fn header(content: &mut PdfContent, options: &PdfExportOptions, layout: PageLayout) {
    content.text_centered(
        "/F2",
        layout.title_size,
        layout.width / 2.0,
        layout.height - 120.0,
        &options.title,
    );
    content.text_centered(
        "/F1",
        layout.subtitle_size,
        layout.width / 2.0,
        layout.height - 162.0,
        &options.subtitle,
    );
}

pub(super) fn map(
    cemetery: &Cemetery,
    content: &mut PdfContent,
    options: &PdfExportOptions,
    layout: PageLayout,
) {
    content.stroke_color(0.18, 0.34, 0.36);
    content.line_width(2.0);
    content.rectangle(
        layout.map_left,
        layout.map_bottom,
        layout.map_width(),
        layout.map_height(),
    );
    content.stroke();

    let Some(bounds) = CemeteryBounds::from_cemetery(cemetery) else {
        content.text_centered(
            "/F1",
            32.0,
            layout.width / 2.0,
            layout.map_bottom + layout.map_height() / 2.0,
            &options.empty_message,
        );
        return;
    };

    let transform = MapTransform::new(bounds, layout);
    render_grid(content, &bounds, &transform);
    delimiter_rendering::render(content, cemetery, &transform);

    let labels_by_grave = labels_by_grave(cemetery);

    for grave in cemetery.graves() {
        let pdf_rectangle = PdfRectangle::from_map(grave.rectangle(), &transform);
        let corners =
            PdfRectangle::corners_from_map(grave.rectangle(), grave.rotation_degrees(), &transform);
        let (red, green, blue) = rgb(grave.color());

        content.fill_color(red, green, blue);
        content.polygon(&corners);
        content.fill();

        content.stroke_color(0.95, 0.98, 0.98);
        content.line_width(1.25);
        content.polygon(&corners);
        content.stroke();

        render_grave_label(
            content,
            labels_by_grave
                .get(&grave.id())
                .map(Vec::as_slice)
                .unwrap_or_default(),
            &grave.id().to_string(),
            pdf_rectangle.x,
            pdf_rectangle.y,
            pdf_rectangle.width,
            pdf_rectangle.height,
        );
    }
}

pub(super) fn footer(content: &mut PdfContent, options: &PdfExportOptions, layout: PageLayout) {
    content.fill_color(0.22, 0.30, 0.31);
    content.text(
        "/F1",
        layout.footer_size,
        layout.margin,
        72.0,
        &options.footer,
    );
    content.text(
        "/F1",
        layout.footer_size,
        layout.width - layout.margin - 260.0,
        72.0,
        "Requiescat PDF export",
    );
}

fn render_grid(content: &mut PdfContent, bounds: &CemeteryBounds, transform: &MapTransform) {
    let interval = grid_interval(bounds.width().max(bounds.height()));
    let start_x = (bounds.min.x / interval).floor() as i32;
    let end_x = (bounds.max.x / interval).ceil() as i32;
    let start_y = (bounds.min.y / interval).floor() as i32;
    let end_y = (bounds.max.y / interval).ceil() as i32;

    content.stroke_color(0.86, 0.91, 0.91);
    content.line_width(0.35);

    for index in start_x..=end_x {
        let x = index as f32 * interval;
        let a = transform.point(Point::new(x, bounds.min.y));
        let b = transform.point(Point::new(x, bounds.max.y));
        content.line(a.x, a.y, b.x, b.y);
    }

    for index in start_y..=end_y {
        let y = index as f32 * interval;
        let a = transform.point(Point::new(bounds.min.x, y));
        let b = transform.point(Point::new(bounds.max.x, y));
        content.line(a.x, a.y, b.x, b.y);
    }
}

fn render_grave_label(
    content: &mut PdfContent,
    rows: &[String],
    fallback: &str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) {
    const PADDING: f32 = 2.0;
    const MIN_FONT_SIZE: f32 = 16.0;
    const MAX_FONT_SIZE: f32 = 28.0;

    let font_size = (width / 8.5).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
    let row_height = font_size * 1.2;
    let max_rows = ((height - PADDING * 2.0) / row_height).floor().max(0.0) as usize;
    if max_rows == 0 {
        return;
    }

    let rows = label_layout::visible_rows(rows, fallback, max_rows);
    let max_characters = label_layout::character_capacity(width - PADDING * 2.0, font_size);
    let total_height = rows.len() as f32 * row_height;
    let first_baseline = y + height / 2.0 + total_height / 2.0 - font_size;

    content.fill_color(1.0, 1.0, 1.0);
    for (index, row) in rows.into_iter().enumerate() {
        let row = label_layout::truncate(&row, max_characters);
        if row.is_empty() {
            continue;
        }
        content.text_centered(
            "/F2",
            font_size,
            x + width / 2.0,
            first_baseline - index as f32 * row_height,
            &row,
        );
    }
}

fn labels_by_grave(cemetery: &Cemetery) -> HashMap<GraveId, Vec<String>> {
    let mut labels = HashMap::new();

    for person in cemetery.people() {
        if let Some(grave_id) = person.grave_id() {
            labels
                .entry(grave_id)
                .or_insert_with(Vec::new)
                .push(person.display_name());
        }
    }

    labels
}

fn rgb(color: GraveColor) -> (f32, f32, f32) {
    let (red, green, blue) = color.to_rgb8();
    let red = red as f32 / 255.0;
    let green = green as f32 / 255.0;
    let blue = blue as f32 / 255.0;
    (red, green, blue)
}

use iced::widget::canvas;
use iced::{Point, Rectangle};

use super::Camera;
use super::map_editor::CanvasState;
use crate::models::{Cemetery, GraveId, GraveRectangle};

pub fn grid(frame: &mut canvas::Frame, camera: &Camera, bounds: Rectangle) {
    const SQUARE_SIZE: f32 = 50.0;
    const LABEL_INTERVAL: i32 = 2;

    let top_left = camera.screen_to_world(Point::ORIGIN);
    let bottom_right = camera.screen_to_world(Point::new(bounds.width, bounds.height));
    let line_color = iced::Color::from_rgba(0.55, 0.72, 0.72, 0.25);
    let axis_color = iced::Color::from_rgba(0.65, 0.90, 0.88, 0.55);
    let label_color = iced::Color::from_rgba(0.75, 0.90, 0.88, 0.75);

    let start_x = (top_left.x / SQUARE_SIZE).floor() as i32;
    let end_x = (bottom_right.x / SQUARE_SIZE).ceil() as i32;
    let start_y = (top_left.y / SQUARE_SIZE).floor() as i32;
    let end_y = (bottom_right.y / SQUARE_SIZE).ceil() as i32;

    for index in start_x..=end_x {
        let world_x = index as f32 * SQUARE_SIZE;
        let screen_x = camera.world_to_screen(Point::new(world_x, 0.0)).x;
        let path = canvas::Path::line(
            Point::new(screen_x, 0.0),
            Point::new(screen_x, bounds.height),
        );

        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(if index == 0 { axis_color } else { line_color })
                .with_width(if index == 0 { 2.0 } else { 1.0 }),
        );

        if index % LABEL_INTERVAL == 0 {
            frame.fill_text(canvas::Text {
                content: format!("{world_x:.0}"),
                position: Point::new(screen_x + 4.0, 4.0),
                color: label_color,
                size: iced::Pixels(11.0),
                ..Default::default()
            });
        }
    }

    for index in start_y..=end_y {
        let world_y = index as f32 * SQUARE_SIZE;
        let screen_y = camera.world_to_screen(Point::new(0.0, world_y)).y;
        let path = canvas::Path::line(
            Point::new(0.0, screen_y),
            Point::new(bounds.width, screen_y),
        );

        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(if index == 0 { axis_color } else { line_color })
                .with_width(if index == 0 { 2.0 } else { 1.0 }),
        );

        if index % LABEL_INTERVAL == 0 {
            frame.fill_text(canvas::Text {
                content: format!("{world_y:.0}"),
                position: Point::new(4.0, screen_y + 4.0),
                color: label_color,
                size: iced::Pixels(11.0),
                ..Default::default()
            });
        }
    }
}

pub fn grave_preview(frame: &mut canvas::Frame, state: &CanvasState, camera: &Camera) {
    if let Some(current_drag) = state.current_drag_position() {
        let start = state.left_pressed_at().unwrap_or(current_drag);
        let preview = GraveRectangle::from_corners(start, current_drag);
        let top_left = camera.world_to_screen(preview.top_left());
        let size = preview.size() * camera.zoom;
        let path = canvas::Path::rectangle(top_left, size);

        frame.stroke(
            &path,
            canvas::Stroke {
                width: 2.0,
                style: canvas::Style::Solid(iced::Color::WHITE),
                line_dash: canvas::LineDash {
                    segments: &[6.0, 4.0],
                    offset: 0,
                },
                ..Default::default()
            },
        );
    }
}

pub fn graves(
    frame: &mut canvas::Frame,
    cemetery: &Cemetery,
    camera: &Camera,
    selected_grave: Option<GraveId>,
) {
    for grave in cemetery.graves() {
        let rectangle = grave.rectangle();
        let top_left = camera.world_to_screen(rectangle.top_left());
        let size = rectangle.size() * camera.zoom;

        frame.fill_rectangle(top_left, size, iced::Color::from_rgb(0.65, 0.121, 0.157));
        grave_labels(
            frame,
            grave.id(),
            grave_label_rows(cemetery, grave.id()),
            top_left,
            size,
        );

        if Some(grave.id()) == selected_grave {
            let path = canvas::Path::rectangle(top_left, size);

            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(iced::Color::from_rgb8(151, 255, 244))
                    .with_width(3.0),
            );
        }
    }
}

fn grave_labels(
    frame: &mut canvas::Frame,
    grave_id: GraveId,
    rows: Vec<String>,
    top_left: Point,
    size: iced::Size,
) {
    const PADDING: f32 = 5.0;
    const MIN_FONT_SIZE: f32 = 9.0;
    const MAX_FONT_SIZE: f32 = 13.0;
    const APPROX_WORLD_CHARACTER_WIDTH: f32 = 7.0;

    let font_size = label_font_size(size.width);
    let row_height = font_size * 1.25;
    let max_rows = ((size.height - PADDING * 2.0) / row_height).floor() as usize;
    if max_rows == 0 {
        return;
    }

    let rows = visible_label_rows(rows, format!("grave {}", grave_id), max_rows);
    let max_width = size.width - PADDING * 2.0;
    let max_characters = (max_width / (font_size / 12.0 * APPROX_WORLD_CHARACTER_WIDTH))
        .floor()
        .max(0.0) as usize;

    for (index, row) in rows.into_iter().enumerate() {
        let row = truncate_label(&row, max_characters);
        if row.is_empty() {
            continue;
        }

        frame.fill_text(canvas::Text {
            content: row,
            position: Point::new(
                top_left.x + size.width / 2.0,
                top_left.y + PADDING + index as f32 * row_height + row_height / 2.0,
            ),
            max_width,
            color: iced::Color::WHITE,
            size: iced::Pixels(font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Default::default()
        });
    }
}

fn label_font_size(screen_width: f32) -> f32 {
    (screen_width / 8.5).clamp(9.0, 13.0)
}

fn grave_label_rows(cemetery: &Cemetery, grave_id: GraveId) -> Vec<String> {
    cemetery
        .people_in_grave(grave_id)
        .into_iter()
        .map(|person| person.display_name())
        .collect()
}

fn visible_label_rows(rows: Vec<String>, fallback: String, max_rows: usize) -> Vec<String> {
    if max_rows == 0 {
        return Vec::new();
    }

    if rows.is_empty() {
        return vec![fallback];
    }

    if rows.len() <= max_rows {
        return rows;
    }

    if max_rows == 1 {
        return vec!["...".to_owned()];
    }

    rows.into_iter()
        .take(max_rows - 1)
        .chain(std::iter::once("...".to_owned()))
        .collect()
}

fn truncate_label(label: &str, max_characters: usize) -> String {
    if label.chars().count() <= max_characters {
        return label.to_owned();
    }

    if max_characters <= 3 {
        return String::new();
    }

    let mut truncated = label.chars().take(max_characters - 3).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn visible_label_rows_uses_fallback_when_grave_has_no_people() {
        assert_eq!(
            visible_label_rows(Vec::new(), "grave 1".to_owned(), 2),
            rows(&["grave 1"])
        );
    }

    #[test]
    fn visible_label_rows_keeps_all_people_that_fit() {
        assert_eq!(
            visible_label_rows(rows(&["Dan Stoian", "Maria Boto"]), "grave 1".to_owned(), 2),
            rows(&["Dan Stoian", "Maria Boto"])
        );
    }

    #[test]
    fn visible_label_rows_reserves_last_row_for_overflow_marker() {
        assert_eq!(
            visible_label_rows(
                rows(&["Dan Stoian", "Maria Boto", "Ada Lovelace"]),
                "grave 1".to_owned(),
                2
            ),
            rows(&["Dan Stoian", "..."])
        );
    }

    #[test]
    fn label_font_size_is_clamped_to_avoid_zoom_jumps() {
        assert_eq!(label_font_size(20.0), 9.0);
        assert_eq!(label_font_size(500.0), 13.0);
    }

    #[test]
    fn truncate_label_uses_character_capacity() {
        assert_eq!(truncate_label("Dan Stoian", 20), "Dan Stoian");
        assert_eq!(truncate_label("Dan Stoian", 6), "Dan...");
    }
}

use std::collections::{HashMap, HashSet};

use iced::widget::canvas;
use iced::{Point, Rectangle, Size};

use super::Camera;
use super::map_canvas::CanvasState;
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
        let (top_left, size) = screen_rectangle(preview, camera);
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
    bounds: Rectangle,
    selected_grave: Option<GraveId>,
    grave_label: impl Fn(GraveId) -> String,
) {
    let visible_graves = cemetery
        .graves()
        .iter()
        .filter(|grave| grave_is_visible(grave.rectangle(), camera, bounds))
        .collect::<Vec<_>>();
    let visible_grave_ids = visible_graves
        .iter()
        .map(|grave| grave.id())
        .collect::<HashSet<_>>();
    let labels_by_grave = grave_labels_by_grave(cemetery, &visible_grave_ids);

    for grave in visible_graves {
        let rectangle = grave.rectangle();
        let (top_left, size) = screen_rectangle(rectangle, camera);
        let grave_id = grave.id();

        frame.fill_rectangle(top_left, size, grave.color().to_iced());
        grave_labels(
            frame,
            labels_by_grave
                .get(&grave_id)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            &grave_label(grave_id),
            top_left,
            size,
        );

        if Some(grave_id) == selected_grave {
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

fn grave_is_visible(rectangle: GraveRectangle, camera: &Camera, bounds: Rectangle) -> bool {
    let (top_left, size) = screen_rectangle(rectangle, camera);
    let right = top_left.x + size.width;
    let bottom = top_left.y + size.height;

    right >= 0.0 && bottom >= 0.0 && top_left.x <= bounds.width && top_left.y <= bounds.height
}

fn screen_rectangle(rectangle: GraveRectangle, camera: &Camera) -> (Point, Size) {
    (
        camera.world_to_screen(rectangle.top_left()),
        rectangle.size() * camera.zoom,
    )
}

fn grave_labels(
    frame: &mut canvas::Frame,
    rows: &[String],
    fallback: &str,
    top_left: Point,
    size: iced::Size,
) {
    const PADDING: f32 = 1.0;
    const MIN_FONT_SIZE: f32 = 9.0;
    const MAX_FONT_SIZE: f32 = 13.0;

    let font_size = label_font_size(size.width);
    let row_height = font_size * 1.25;
    let max_rows = ((size.height - PADDING * 2.0) / row_height).floor() as usize;
    if max_rows == 0 {
        return;
    }

    let rows = visible_label_rows(rows, fallback, max_rows);
    let max_width = (size.width - PADDING * 2.0).max(0.0);
    let max_characters = label_character_capacity(max_width, font_size);

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

fn label_character_capacity(available_width: f32, font_size: f32) -> usize {
    const APPROX_CHARACTER_WIDTH_RATIO: f32 = 0.52;

    (available_width / (font_size * APPROX_CHARACTER_WIDTH_RATIO))
        .floor()
        .max(0.0) as usize
}

fn grave_labels_by_grave(
    cemetery: &Cemetery,
    visible_grave_ids: &HashSet<GraveId>,
) -> HashMap<GraveId, Vec<String>> {
    let mut labels = HashMap::new();

    for person in cemetery.people() {
        if let Some(grave_id) = person
            .grave_id()
            .filter(|grave_id| visible_grave_ids.contains(grave_id))
        {
            labels
                .entry(grave_id)
                .or_insert_with(Vec::new)
                .push(person.display_name());
        }
    }

    labels
}

fn visible_label_rows(rows: &[String], fallback: &str, max_rows: usize) -> Vec<String> {
    if max_rows == 0 {
        return Vec::new();
    }

    if rows.is_empty() {
        return vec![fallback.to_owned()];
    }

    if rows.len() <= max_rows {
        return rows.to_vec();
    }

    if max_rows == 1 {
        return vec!["...".to_owned()];
    }

    rows.iter()
        .take(max_rows - 1)
        .cloned()
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
            visible_label_rows(&Vec::new(), "grave 1", 2),
            rows(&["grave 1"])
        );
    }

    #[test]
    fn visible_label_rows_keeps_all_people_that_fit() {
        assert_eq!(
            visible_label_rows(&rows(&["Dan Stoian", "Maria Boto"]), "grave 1", 2),
            rows(&["Dan Stoian", "Maria Boto"])
        );
    }

    #[test]
    fn visible_label_rows_reserves_last_row_for_overflow_marker() {
        assert_eq!(
            visible_label_rows(
                &rows(&["Dan Stoian", "Maria Boto", "Ada Lovelace"]),
                "grave 1",
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
    fn label_character_capacity_expands_with_available_width() {
        assert_eq!(label_character_capacity(46.8, 9.0), 10);
        assert_eq!(label_character_capacity(93.6, 9.0), 20);
    }

    #[test]
    fn truncate_label_uses_character_capacity() {
        assert_eq!(truncate_label("Dan Stoian", 20), "Dan Stoian");
        assert_eq!(truncate_label("Dan Stoian", 6), "Dan...");
    }
}

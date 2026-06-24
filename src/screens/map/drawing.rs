use std::collections::{HashMap, HashSet};

use iced::widget::canvas;
use iced::{Point, Rectangle, Vector};

use super::Camera;
use super::map_canvas::CanvasState;
use super::map_editor::MapEditor;
use crate::label_layout;
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
        let path = rotated_rectangle_path(preview, 0.0, camera);

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

pub fn rotation_handles(frame: &mut canvas::Frame, editor: &MapEditor) {
    for id in editor.rotation_targets() {
        let Some(position) = editor.rotation_handle_position(id) else {
            continue;
        };

        let screen_position = editor.camera().world_to_screen(position);
        let dot = canvas::Path::circle(screen_position, 6.0);

        frame.fill(&dot, iced::Color::WHITE);
        frame.stroke(
            &dot,
            canvas::Stroke::default()
                .with_color(iced::Color::from_rgb8(16, 132, 122))
                .with_width(2.0),
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
        .filter(|grave| {
            grave_is_visible(grave.rectangle(), grave.rotation_degrees(), camera, bounds)
        })
        .collect::<Vec<_>>();
    let visible_grave_ids = visible_graves
        .iter()
        .map(|grave| grave.id())
        .collect::<HashSet<_>>();
    let labels_by_grave = grave_labels_by_grave(cemetery, &visible_grave_ids);

    for grave in visible_graves {
        let rectangle = grave.rectangle();
        let path = rotated_rectangle_path(rectangle, grave.rotation_degrees(), camera);
        let grave_id = grave.id();

        frame.fill(&path, grave.color().to_iced());
        grave_labels(
            frame,
            labels_by_grave
                .get(&grave_id)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            &grave_label(grave_id),
            rectangle,
            grave.rotation_degrees(),
            camera,
        );

        if Some(grave_id) == selected_grave {
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(iced::Color::from_rgb8(151, 255, 244))
                    .with_width(3.0),
            );
        }
    }
}

fn grave_is_visible(
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

fn rotated_rectangle_path(
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    camera: &Camera,
) -> canvas::Path {
    let corners = rectangle
        .corners_rotated(rotation_degrees)
        .map(|corner| camera.world_to_screen(corner));

    canvas::Path::new(|builder| {
        builder.move_to(corners[0]);
        builder.line_to(corners[1]);
        builder.line_to(corners[2]);
        builder.line_to(corners[3]);
        builder.close();
    })
}

fn grave_labels(
    frame: &mut canvas::Frame,
    rows: &[String],
    fallback: &str,
    rectangle: GraveRectangle,
    rotation_degrees: f32,
    camera: &Camera,
) {
    const PADDING: f32 = 1.0;
    const MIN_FONT_SIZE: f32 = 9.0;
    const MAX_FONT_SIZE: f32 = 13.0;

    let top_left = camera.world_to_screen(rectangle.top_left());
    let center = camera.world_to_screen(rectangle.center());
    let size = rectangle.size() * camera.zoom;
    let font_size = label_font_size(size.width);
    let row_height = font_size * 1.25;
    let max_rows = ((size.height - PADDING * 2.0) / row_height).floor() as usize;
    if max_rows == 0 {
        return;
    }

    let rows = label_layout::visible_rows(rows, fallback, max_rows);
    let max_width = (size.width - PADDING * 2.0).max(0.0);
    let max_characters = label_layout::character_capacity(max_width, font_size);

    frame.with_save(|frame| {
        frame.translate(Vector::new(center.x, center.y));
        frame.rotate(rotation_degrees.to_radians());
        frame.translate(Vector::new(-center.x, -center.y));

        for (index, row) in rows.into_iter().enumerate() {
            let row = label_layout::truncate(&row, max_characters);
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
    });
}

fn label_font_size(screen_width: f32) -> f32 {
    (screen_width / 8.5).clamp(9.0, 13.0)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_font_size_is_clamped_to_avoid_zoom_jumps() {
        assert_eq!(label_font_size(20.0), 9.0);
        assert_eq!(label_font_size(500.0), 13.0);
    }
}

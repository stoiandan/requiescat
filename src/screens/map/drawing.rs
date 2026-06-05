use iced::widget::canvas;
use iced::{Point, Rectangle};

use super::Camera;
use super::map_editor::CanvasState;
use crate::models::Grave;

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

pub fn grave_preview(frame: &mut canvas::Frame, state: &CanvasState) {
    if let Some(current_drag) = state.current_drag_position() {
        let start = state.left_pressed_at().unwrap_or(current_drag);
        let ghost_grave: Grave = (start, current_drag).into();
        let top_left = state.camera().world_to_screen(ghost_grave.top_left());
        let size = ghost_grave.size() * state.camera().zoom;
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

pub fn graves(frame: &mut canvas::Frame, graves: &[Grave], camera: &Camera) {
    for grave in graves {
        let top_left = camera.world_to_screen(grave.top_left());
        let size = grave.size() * camera.zoom;

        frame.fill_rectangle(top_left, size, iced::Color::from_rgb(0.65, 0.121, 0.157));
    }
}

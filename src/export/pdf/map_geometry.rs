use crate::models::GraveRectangle;

use super::layout::MapTransform;

#[derive(Debug, Clone, Copy)]
pub(super) struct PdfRectangle {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

impl PdfRectangle {
    pub(super) fn from_map(rectangle: GraveRectangle, transform: &MapTransform) -> Self {
        let top_left = transform.point(rectangle.top_left());
        let size = transform.size(rectangle.size());

        Self {
            x: top_left.x,
            y: top_left.y - size.height,
            width: size.width,
            height: size.height,
        }
    }

    pub(super) fn corners_from_map(
        rectangle: GraveRectangle,
        rotation_degrees: f32,
        transform: &MapTransform,
    ) -> [iced::Point; 4] {
        rectangle
            .corners_rotated(rotation_degrees)
            .map(|corner| transform.point(corner))
    }

    pub(super) fn point_from_map(
        rectangle: GraveRectangle,
        x: f32,
        y: f32,
        rotation_degrees: f32,
        transform: &MapTransform,
    ) -> iced::Point {
        transform.point(rectangle.point_at_rotated(x, y, rotation_degrees))
    }

    pub(super) fn min_dimension(self) -> f32 {
        self.width.min(self.height)
    }
}

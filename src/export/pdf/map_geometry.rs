use crate::models::GraveRectangle;

use super::content::PdfContent;
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

    pub(super) fn draw(self, content: &mut PdfContent) {
        content.rectangle(self.x, self.y, self.width, self.height);
    }

    pub(super) fn center_y(self) -> f32 {
        self.y + self.height / 2.0
    }

    pub(super) fn right(self) -> f32 {
        self.x + self.width
    }

    pub(super) fn min_dimension(self) -> f32 {
        self.width.min(self.height)
    }
}

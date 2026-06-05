use iced::{Point, Size, Vector};

use super::grave_rectangle::GraveRectangle;

#[derive(Debug, Clone, Copy)]
pub struct Grave {
    rectangle: GraveRectangle,
}

impl Grave {
    pub fn from_corners(a: Point, b: Point) -> Self {
        Self {
            rectangle: GraveRectangle::from_corners(a, b),
        }
    }

    pub fn from_top_left_size(top_left: Point, size: Size) -> Self {
        Self {
            rectangle: GraveRectangle::from_top_left_size(top_left, size),
        }
    }

    pub fn top_left(&self) -> Point {
        self.rectangle.top_left()
    }

    pub fn size(&self) -> Size {
        self.rectangle.size()
    }

    pub fn contains(&self, point: Point) -> bool {
        self.rectangle.contains(point)
    }

    pub fn translate(&mut self, delta: Vector) {
        self.rectangle.translate(delta);
    }
}

impl Into<Grave> for (Point, Point) {
    fn into(self) -> Grave {
        Grave::from_corners(self.0, self.1)
    }
}

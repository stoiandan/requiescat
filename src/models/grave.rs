use iced::{Point, Vector};

use super::{GraveId, GraveRectangle};

#[derive(Debug, Clone, Copy)]
pub struct Grave {
    id: GraveId,
    rectangle: GraveRectangle,
}

impl Grave {
    pub fn new(id: GraveId, rectangle: GraveRectangle) -> Self {
        Self { id, rectangle }
    }

    pub fn id(&self) -> GraveId {
        self.id
    }

    pub fn rectangle(&self) -> GraveRectangle {
        self.rectangle
    }

    pub fn contains(&self, point: Point) -> bool {
        self.rectangle.contains(point)
    }

    pub fn translate(&mut self, delta: Vector) {
        self.rectangle.translate(delta);
    }
}

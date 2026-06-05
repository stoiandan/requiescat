use iced::{Point, Size, Vector};

#[derive(Debug, Clone, Copy)]
pub struct GraveRectangle {
    top_left: Point,
    size: Size,
}

impl GraveRectangle {
    pub fn from_corners(a: Point, b: Point) -> Self {
        Self {
            top_left: Point::new(a.x.min(b.x), a.y.min(b.y)),
            size: Size::new((a.x - b.x).abs(), (a.y - b.y).abs()),
        }
    }

    pub fn from_top_left_size(top_left: Point, size: Size) -> Self {
        Self { top_left, size }
    }

    pub fn top_left(&self) -> Point {
        self.top_left
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.top_left.x
            && point.x <= self.top_left.x + self.size.width
            && point.y >= self.top_left.y
            && point.y <= self.top_left.y + self.size.height
    }

    pub fn translate(&mut self, delta: Vector) {
        self.top_left.x += delta.x;
        self.top_left.y += delta.y;
    }
}

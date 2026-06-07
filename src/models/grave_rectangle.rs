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

    pub fn center(&self) -> Point {
        Point::new(
            self.top_left.x + self.size.width / 2.0,
            self.top_left.y + self.size.height / 2.0,
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_corners_normalizes_top_left_and_size() {
        let rectangle = GraveRectangle::from_corners(Point::new(8.0, 12.0), Point::new(2.0, 4.0));

        assert_eq!(rectangle.top_left(), Point::new(2.0, 4.0));
        assert_eq!(rectangle.size(), Size::new(6.0, 8.0));
    }

    #[test]
    fn contains_includes_edges_and_rejects_points_outside() {
        let rectangle =
            GraveRectangle::from_top_left_size(Point::new(10.0, 20.0), Size::new(30.0, 40.0));

        assert!(rectangle.contains(Point::new(10.0, 20.0)));
        assert!(rectangle.contains(Point::new(40.0, 60.0)));
        assert!(rectangle.contains(Point::new(25.0, 45.0)));
        assert!(!rectangle.contains(Point::new(9.9, 45.0)));
        assert!(!rectangle.contains(Point::new(25.0, 60.1)));
    }

    #[test]
    fn center_returns_middle_of_rectangle() {
        let rectangle =
            GraveRectangle::from_top_left_size(Point::new(10.0, 20.0), Size::new(30.0, 40.0));

        assert_eq!(rectangle.center(), Point::new(25.0, 40.0));
    }

    #[test]
    fn translate_moves_only_the_top_left_corner() {
        let mut rectangle =
            GraveRectangle::from_top_left_size(Point::new(10.0, 20.0), Size::new(30.0, 40.0));

        rectangle.translate(Vector::new(-5.0, 7.5));

        assert_eq!(rectangle.top_left(), Point::new(5.0, 27.5));
        assert_eq!(rectangle.size(), Size::new(30.0, 40.0));
    }
}

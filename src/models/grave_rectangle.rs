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

    pub fn corners_rotated(&self, rotation_degrees: f32) -> [Point; 4] {
        let top_left = self.top_left();
        let top_right = Point::new(top_left.x + self.size.width, top_left.y);
        let bottom_right = Point::new(top_left.x + self.size.width, top_left.y + self.size.height);
        let bottom_left = Point::new(top_left.x, top_left.y + self.size.height);
        let center = self.center();

        [
            rotate_point(top_left, center, rotation_degrees),
            rotate_point(top_right, center, rotation_degrees),
            rotate_point(bottom_right, center, rotation_degrees),
            rotate_point(bottom_left, center, rotation_degrees),
        ]
    }

    pub fn point_at_rotated(&self, x: f32, y: f32, rotation_degrees: f32) -> Point {
        rotate_point(
            Point::new(self.top_left.x + x, self.top_left.y + y),
            self.center(),
            rotation_degrees,
        )
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.top_left.x
            && point.x <= self.top_left.x + self.size.width
            && point.y >= self.top_left.y
            && point.y <= self.top_left.y + self.size.height
    }

    pub fn contains_rotated(&self, point: Point, rotation_degrees: f32) -> bool {
        self.contains(rotate_point(point, self.center(), -rotation_degrees))
    }

    pub fn translated(self, delta: Vector) -> Self {
        Self {
            top_left: Point::new(self.top_left.x + delta.x, self.top_left.y + delta.y),
            ..self
        }
    }
}

pub fn rotate_point(point: Point, center: Point, rotation_degrees: f32) -> Point {
    let radians = rotation_degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    let x = point.x - center.x;
    let y = point.y - center.y;

    Point::new(center.x + x * cos - y * sin, center.y + x * sin + y * cos)
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
    fn contains_rotated_checks_points_in_rotated_space() {
        let rectangle =
            GraveRectangle::from_top_left_size(Point::new(0.0, 0.0), Size::new(20.0, 10.0));

        assert!(rectangle.contains_rotated(rectangle.center(), 45.0));
        assert!(!rectangle.contains_rotated(Point::new(20.0, 10.0), 45.0));
    }

    #[test]
    fn center_returns_middle_of_rectangle() {
        let rectangle =
            GraveRectangle::from_top_left_size(Point::new(10.0, 20.0), Size::new(30.0, 40.0));

        assert_eq!(rectangle.center(), Point::new(25.0, 40.0));
    }

    #[test]
    fn translated_returns_a_moved_rectangle_without_changing_the_original() {
        let rectangle =
            GraveRectangle::from_top_left_size(Point::new(10.0, 20.0), Size::new(30.0, 40.0));

        let translated = rectangle.translated(Vector::new(-5.0, 7.5));

        assert_eq!(rectangle.top_left(), Point::new(10.0, 20.0));
        assert_eq!(translated.top_left(), Point::new(5.0, 27.5));
        assert_eq!(translated.size(), Size::new(30.0, 40.0));
    }
}

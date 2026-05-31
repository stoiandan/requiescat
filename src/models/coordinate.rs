use iced::Point;
use std::num;

#[derive(Debug, Clone, Copy)]
pub struct Coordinate {
    pub p1: Point,
    pub p2: Point
}


impl Coordinate {
    pub fn new(p1: Point, p2: Point) -> Self {
        Self { 
            p1,
            p2
        }
    }

    pub fn height(&self) -> f32 {
       (self.p1.y - self.p2.y).abs()
    }

    pub fn width(&self) -> f32 {
        (self.p1.x - self.p2.x).abs()
    }

    pub fn top_left_x(&self) -> f32 {
        self.p1.x.min(self.p2.x)
    }

    pub fn top_left_y(&self) -> f32 {
        self.p1.y.min(self.p2.y)
    }
}
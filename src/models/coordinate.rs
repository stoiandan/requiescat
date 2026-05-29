#[derive(Debug, Clone, Copy)]
pub struct Coordinate {
    pub x: f32,
    pub y: f32,
    pub width: i32,
    pub height: i32,
}


impl Coordinate {
    pub fn new(x: f32, y: f32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }
}
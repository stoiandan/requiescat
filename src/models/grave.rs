use super::coordinate::Coordinate;


#[derive(Debug, Clone, Copy)]
pub struct Grave {
    coordinate: Coordinate,
}


impl Grave {
    pub fn new(x: f32, y: f32, width: i32, height: i32) -> Self {
        Self { coordinate: Coordinate::new(x, y, width, height) }
    }
}
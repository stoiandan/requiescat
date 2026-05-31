use super::coordinate::Coordinate;


#[derive(Debug, Clone, Copy)]
pub struct Grave {
    pub coordinate: Coordinate,
}


impl Grave {
    pub fn new(top_left: iced::Point, bottom_right: iced::Point) -> Self {
        Self { coordinate: Coordinate::new(top_left, bottom_right) }
    }
}

impl Into<Grave> for (iced::Point, iced::Point) {
    fn into(self) -> Grave {
        Grave { coordinate: Coordinate { p1: self.0, p2: self.1 } }
    }
}
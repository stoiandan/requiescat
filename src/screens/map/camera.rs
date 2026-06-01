use iced::Point;

pub(super) struct Camera {
    pub zoom: f32,
    pub offset: Point,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: Point::new(0.0, 0.0),
        }
    }
}

impl Camera {
    pub fn world_to_screen(&self, world: Point) -> Point {
        Point::new(
            (world.x - self.offset.x) * self.zoom,
            (world.y - self.offset.y) * self.zoom,
        )
    }

    pub fn screen_to_world(&self, screen: Point) -> Point {
        Point::new(
            (screen.x / self.zoom) + self.offset.x,
            (screen.y / self.zoom) + self.offset.y,
        )
    }    
}
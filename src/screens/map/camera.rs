use iced::{Point, Vector};

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

    pub fn canvas_delta_to_world(&self, delta: Vector) -> Vector {
        Vector::new(delta.x / self.zoom, delta.y / self.zoom)
    }

    pub fn pan_by_canvas_delta(&mut self, delta: Vector) {
        let delta = self.canvas_delta_to_world(delta);

        self.offset.x -= delta.x;
        self.offset.y -= delta.y;
    }

    pub fn zoom_at(&mut self, cursor: Point, amount: f32) {
        let before = self.screen_to_world(cursor);

        self.zoom = (self.zoom + amount).clamp(0.1, 10.0);

        let after = self.screen_to_world(cursor);

        self.offset.x += before.x - after.x;
        self.offset.y += before.y - after.y;
    }
}

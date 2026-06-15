use iced::Vector;

use super::{Grave, GraveId, GraveRectangle};

#[derive(Debug, Default)]
pub struct CemeteryMap {
    graves: Vec<Grave>,
    next_grave_id: i64,
}

impl CemeteryMap {
    pub fn from_graves(graves: Vec<Grave>) -> Self {
        let next_grave_id = graves
            .iter()
            .map(|grave| grave.id().value())
            .max()
            .unwrap_or_default();

        Self {
            graves,
            next_grave_id,
        }
    }

    pub fn add_grave(&mut self, rectangle: GraveRectangle) -> GraveId {
        let id = self.next_id();
        self.graves.push(Grave::new(id, rectangle));
        id
    }

    pub fn erase_grave(&mut self, id: GraveId) {
        if let Some(index) = self.index_of(id) {
            self.graves.remove(index);
        }
    }

    pub fn move_grave(&mut self, id: GraveId, delta: Vector) {
        if let Some(grave) = self.grave_mut(id) {
            grave.translate(delta);
        }
    }

    pub fn grave_at(&self, point: iced::Point) -> Option<GraveId> {
        self.graves
            .iter()
            .rev()
            .find(|grave| grave.contains(point))
            .map(Grave::id)
    }

    pub fn graves(&self) -> &[Grave] {
        &self.graves
    }

    pub fn grave(&self, id: GraveId) -> Option<&Grave> {
        self.graves.iter().find(|grave| grave.id() == id)
    }

    fn grave_mut(&mut self, id: GraveId) -> Option<&mut Grave> {
        self.graves.iter_mut().find(|grave| grave.id() == id)
    }

    fn index_of(&self, id: GraveId) -> Option<usize> {
        self.graves.iter().position(|grave| grave.id() == id)
    }

    fn next_id(&mut self) -> GraveId {
        self.next_grave_id += 1;
        GraveId::new(self.next_grave_id)
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size, Vector};

    use super::*;

    fn rectangle_at(x: f32, y: f32) -> GraveRectangle {
        GraveRectangle::from_top_left_size(Point::new(x, y), Size::new(10.0, 20.0))
    }

    #[test]
    fn add_grave_assigns_incrementing_ids_and_stores_graves() {
        let mut map = CemeteryMap::default();

        let first = map.add_grave(rectangle_at(0.0, 0.0));
        let second = map.add_grave(rectangle_at(20.0, 0.0));

        assert_eq!(first, GraveId::new(1));
        assert_eq!(second, GraveId::new(2));
        assert_eq!(map.graves().len(), 2);
        assert_eq!(map.grave(first).map(Grave::id), Some(first));
    }

    #[test]
    fn grave_at_returns_the_matching_grave_id() {
        let mut map = CemeteryMap::default();
        let first = map.add_grave(rectangle_at(0.0, 0.0));
        let second = map.add_grave(rectangle_at(20.0, 0.0));

        assert_eq!(map.grave_at(Point::new(5.0, 5.0)), Some(first));
        assert_eq!(map.grave_at(Point::new(25.0, 5.0)), Some(second));
        assert_eq!(map.grave_at(Point::new(100.0, 100.0)), None);
    }

    #[test]
    fn grave_at_returns_the_topmost_overlapping_grave() {
        let mut map = CemeteryMap::default();
        map.add_grave(rectangle_at(0.0, 0.0));
        let topmost = map.add_grave(rectangle_at(5.0, 5.0));

        assert_eq!(map.grave_at(Point::new(7.0, 7.0)), Some(topmost));
    }

    #[test]
    fn move_grave_translates_only_the_requested_grave() {
        let mut map = CemeteryMap::default();
        let moved = map.add_grave(rectangle_at(0.0, 0.0));
        let stationary = map.add_grave(rectangle_at(20.0, 0.0));

        map.move_grave(moved, Vector::new(5.0, -3.0));

        assert_eq!(
            map.grave(moved).map(|grave| grave.rectangle().top_left()),
            Some(Point::new(5.0, -3.0))
        );
        assert_eq!(
            map.grave(stationary)
                .map(|grave| grave.rectangle().top_left()),
            Some(Point::new(20.0, 0.0))
        );
    }

    #[test]
    fn erase_grave_removes_only_the_requested_grave() {
        let mut map = CemeteryMap::default();
        let removed = map.add_grave(rectangle_at(0.0, 0.0));
        let remaining = map.add_grave(rectangle_at(20.0, 0.0));

        map.erase_grave(removed);

        assert!(map.grave(removed).is_none());
        assert!(map.grave(remaining).is_some());
        assert_eq!(map.graves().len(), 1);
    }
}

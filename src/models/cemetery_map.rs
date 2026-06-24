use iced::Vector;

use super::{
    Delimiter, DelimiterId, DelimiterType, Grave, GraveColor, GraveGps, GraveId, GraveRectangle,
};

#[derive(Debug, Clone, Default)]
pub struct CemeteryMap {
    graves: Vec<Grave>,
    delimiters: Vec<Delimiter>,
    next_grave_id: i64,
    next_delimiter_id: i64,
}

impl CemeteryMap {
    pub fn from_records(graves: Vec<Grave>, delimiters: Vec<Delimiter>) -> Self {
        let next_grave_id = graves
            .iter()
            .map(|grave| grave.id().value())
            .max()
            .unwrap_or_default();
        let next_delimiter_id = delimiters
            .iter()
            .map(|delimiter| delimiter.id().value())
            .max()
            .unwrap_or_default();

        Self {
            graves,
            delimiters,
            next_grave_id,
            next_delimiter_id,
        }
    }

    pub fn add_grave_with_color(
        &mut self,
        rectangle: GraveRectangle,
        color: GraveColor,
    ) -> GraveId {
        let id = self.next_id();
        self.graves.push(Grave::with_color(id, rectangle, color));
        id
    }

    pub fn add_delimiter_with_color_and_type(
        &mut self,
        rectangle: GraveRectangle,
        color: GraveColor,
        delimiter_type: DelimiterType,
    ) -> DelimiterId {
        let id = self.next_delimiter_id();
        self.delimiters.push(Delimiter::with_color_and_type(
            id,
            rectangle,
            color,
            0.0,
            delimiter_type,
        ));
        id
    }

    pub fn erase_grave(&mut self, id: GraveId) {
        if let Some(index) = self.index_of(id) {
            self.graves.remove(index);
        }
    }

    pub fn erase_delimiter(&mut self, id: DelimiterId) {
        if let Some(index) = self
            .delimiters
            .iter()
            .position(|delimiter| delimiter.id() == id)
        {
            self.delimiters.remove(index);
        }
    }

    pub fn move_grave(&mut self, id: GraveId, delta: Vector) {
        self.update_grave(id, |grave| grave.translated(delta));
    }

    pub fn move_delimiter(&mut self, id: DelimiterId, delta: Vector) {
        self.update_delimiter(id, |delimiter| delimiter.translated(delta));
    }

    pub fn rotate_grave(&mut self, id: GraveId, rotation_degrees: f32) -> bool {
        self.update_grave(id, |grave| grave.with_rotation(rotation_degrees))
    }

    pub fn rotate_delimiter(&mut self, id: DelimiterId, rotation_degrees: f32) -> bool {
        self.update_delimiter(id, |delimiter| delimiter.with_rotation(rotation_degrees))
    }

    pub fn update_grave_gps(&mut self, id: GraveId, gps: Option<GraveGps>) -> bool {
        self.update_grave(id, |grave| grave.with_gps(gps))
    }

    pub fn grave_at(&self, point: iced::Point) -> Option<GraveId> {
        self.graves
            .iter()
            .rev()
            .find(|grave| grave.contains(point))
            .map(Grave::id)
    }

    pub fn delimiter_at(&self, point: iced::Point) -> Option<DelimiterId> {
        self.delimiters
            .iter()
            .rev()
            .find(|delimiter| delimiter.contains(point))
            .map(Delimiter::id)
    }

    pub fn graves(&self) -> &[Grave] {
        &self.graves
    }

    pub fn delimiters(&self) -> &[Delimiter] {
        &self.delimiters
    }

    pub fn grave(&self, id: GraveId) -> Option<&Grave> {
        self.graves.iter().find(|grave| grave.id() == id)
    }

    pub fn delimiter(&self, id: DelimiterId) -> Option<&Delimiter> {
        self.delimiters
            .iter()
            .find(|delimiter| delimiter.id() == id)
    }

    fn update_grave(&mut self, id: GraveId, update: impl FnOnce(Grave) -> Grave) -> bool {
        let Some(index) = self.index_of(id) else {
            return false;
        };

        self.graves[index] = update(self.graves[index]);
        true
    }

    fn index_of(&self, id: GraveId) -> Option<usize> {
        self.graves.iter().position(|grave| grave.id() == id)
    }

    fn update_delimiter(
        &mut self,
        id: DelimiterId,
        update: impl FnOnce(Delimiter) -> Delimiter,
    ) -> bool {
        let Some(index) = self
            .delimiters
            .iter()
            .position(|delimiter| delimiter.id() == id)
        else {
            return false;
        };

        self.delimiters[index] = update(self.delimiters[index]);
        true
    }

    fn next_id(&mut self) -> GraveId {
        self.next_grave_id += 1;
        GraveId::new(self.next_grave_id)
    }

    fn next_delimiter_id(&mut self) -> DelimiterId {
        self.next_delimiter_id += 1;
        DelimiterId::new(self.next_delimiter_id)
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

        let first = map.add_grave_with_color(rectangle_at(0.0, 0.0), GraveColor::default());
        let second = map.add_grave_with_color(rectangle_at(20.0, 0.0), GraveColor::default());

        assert_eq!(first, GraveId::new(1));
        assert_eq!(second, GraveId::new(2));
        assert_eq!(map.graves().len(), 2);
        assert_eq!(map.grave(first).map(Grave::id), Some(first));
    }

    #[test]
    fn add_delimiter_assigns_incrementing_ids_and_stores_delimiters() {
        let mut map = CemeteryMap::default();

        let first = map.add_delimiter_with_color_and_type(
            rectangle_at(0.0, 0.0),
            GraveColor::default(),
            DelimiterType::Wall,
        );
        let second = map.add_delimiter_with_color_and_type(
            rectangle_at(20.0, 0.0),
            GraveColor::default(),
            DelimiterType::Road,
        );

        assert_eq!(first, DelimiterId::new(1));
        assert_eq!(second, DelimiterId::new(2));
        assert_eq!(map.delimiters().len(), 2);
    }

    #[test]
    fn grave_at_returns_the_matching_grave_id() {
        let mut map = CemeteryMap::default();
        let first = map.add_grave_with_color(rectangle_at(0.0, 0.0), GraveColor::default());
        let second = map.add_grave_with_color(rectangle_at(20.0, 0.0), GraveColor::default());

        assert_eq!(map.grave_at(Point::new(5.0, 5.0)), Some(first));
        assert_eq!(map.grave_at(Point::new(25.0, 5.0)), Some(second));
        assert_eq!(map.grave_at(Point::new(100.0, 100.0)), None);
    }

    #[test]
    fn grave_at_returns_the_topmost_overlapping_grave() {
        let mut map = CemeteryMap::default();
        map.add_grave_with_color(rectangle_at(0.0, 0.0), GraveColor::default());
        let topmost = map.add_grave_with_color(rectangle_at(5.0, 5.0), GraveColor::default());

        assert_eq!(map.grave_at(Point::new(7.0, 7.0)), Some(topmost));
    }

    #[test]
    fn delimiter_at_returns_the_topmost_matching_delimiter() {
        let mut map = CemeteryMap::default();
        map.add_delimiter_with_color_and_type(
            rectangle_at(0.0, 0.0),
            GraveColor::default(),
            DelimiterType::Wall,
        );
        let topmost = map.add_delimiter_with_color_and_type(
            rectangle_at(5.0, 5.0),
            GraveColor::default(),
            DelimiterType::Road,
        );

        assert_eq!(map.delimiter_at(iced::Point::new(7.0, 7.0)), Some(topmost));
    }

    #[test]
    fn move_grave_translates_only_the_requested_grave() {
        let mut map = CemeteryMap::default();
        let moved = map.add_grave_with_color(rectangle_at(0.0, 0.0), GraveColor::default());
        let stationary = map.add_grave_with_color(rectangle_at(20.0, 0.0), GraveColor::default());

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
    fn move_delimiter_translates_only_the_requested_delimiter() {
        let mut map = CemeteryMap::default();
        let moved = map.add_delimiter_with_color_and_type(
            rectangle_at(0.0, 0.0),
            GraveColor::default(),
            DelimiterType::Wall,
        );
        let stationary = map.add_delimiter_with_color_and_type(
            rectangle_at(20.0, 0.0),
            GraveColor::default(),
            DelimiterType::Road,
        );

        map.move_delimiter(moved, Vector::new(5.0, -2.0));

        assert_eq!(
            map.delimiter(moved)
                .map(|delimiter| delimiter.rectangle().top_left()),
            Some(Point::new(5.0, -2.0))
        );
        assert_eq!(
            map.delimiter(stationary)
                .map(|delimiter| delimiter.rectangle().top_left()),
            Some(Point::new(20.0, 0.0))
        );
    }

    #[test]
    fn rotating_objects_normalizes_their_angle() {
        let mut map = CemeteryMap::default();
        let grave = map.add_grave_with_color(rectangle_at(0.0, 0.0), GraveColor::default());
        let delimiter = map.add_delimiter_with_color_and_type(
            rectangle_at(20.0, 0.0),
            GraveColor::default(),
            DelimiterType::Road,
        );

        assert!(map.rotate_grave(grave, 450.0));
        assert!(map.rotate_delimiter(delimiter, -90.0));

        assert_eq!(
            map.grave(grave).map(|grave| grave.rotation_degrees()),
            Some(90.0)
        );
        assert_eq!(
            map.delimiter(delimiter)
                .map(|delimiter| delimiter.rotation_degrees()),
            Some(270.0)
        );
    }

    #[test]
    fn erase_grave_removes_only_the_requested_grave() {
        let mut map = CemeteryMap::default();
        let removed = map.add_grave_with_color(rectangle_at(0.0, 0.0), GraveColor::default());
        let remaining = map.add_grave_with_color(rectangle_at(20.0, 0.0), GraveColor::default());

        map.erase_grave(removed);

        assert!(map.grave(removed).is_none());
        assert!(map.grave(remaining).is_some());
        assert_eq!(map.graves().len(), 1);
    }

    #[test]
    fn erase_delimiter_removes_only_the_requested_delimiter() {
        let mut map = CemeteryMap::default();
        let removed = map.add_delimiter_with_color_and_type(
            rectangle_at(0.0, 0.0),
            GraveColor::default(),
            DelimiterType::Wall,
        );
        let remaining = map.add_delimiter_with_color_and_type(
            rectangle_at(20.0, 0.0),
            GraveColor::default(),
            DelimiterType::Road,
        );

        map.erase_delimiter(removed);

        assert_eq!(map.delimiters().len(), 1);
        assert_eq!(map.delimiters()[0].id(), remaining);
    }
}

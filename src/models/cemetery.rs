use iced::Vector;

use super::{
    CemeteryMap, Grave, GraveColor, GraveId, GraveRectangle, Person, PersonDate, PersonDirectory,
    PersonId,
};

#[derive(Debug, Clone, Default)]
pub struct Cemetery {
    map: CemeteryMap,
    people: PersonDirectory,
}

impl Cemetery {
    pub fn from_records(graves: Vec<Grave>, mut people: Vec<Person>) -> Self {
        for person in &mut people {
            if person
                .grave_id()
                .is_some_and(|grave_id| !graves.iter().any(|grave| grave.id() == grave_id))
            {
                person.unassign_from_grave();
            }
        }

        Self {
            map: CemeteryMap::from_graves(graves),
            people: PersonDirectory::from_people(people),
        }
    }

    pub fn add_grave_with_color(
        &mut self,
        rectangle: GraveRectangle,
        color: GraveColor,
    ) -> GraveId {
        self.map.add_grave_with_color(rectangle, color)
    }

    pub fn erase_grave(&mut self, id: GraveId) {
        self.map.erase_grave(id);
        self.people.unassign_all_from_grave(id);
    }

    pub fn move_grave(&mut self, id: GraveId, delta: Vector) {
        self.map.move_grave(id, delta);
    }

    pub fn grave_at(&self, point: iced::Point) -> Option<GraveId> {
        self.map.grave_at(point)
    }

    pub fn grave(&self, id: GraveId) -> Option<&Grave> {
        self.map.grave(id)
    }

    pub fn graves(&self) -> &[Grave] {
        self.map.graves()
    }

    pub fn create_person_with_details(
        &mut self,
        first_name: String,
        last_name: String,
        date_of_birth: PersonDate,
        date_of_decease: Option<PersonDate>,
        grave_id: Option<GraveId>,
    ) -> PersonId {
        self.people.create_person_with_details(
            first_name,
            last_name,
            date_of_birth,
            date_of_decease,
            grave_id.filter(|id| self.grave(*id).is_some()),
        )
    }

    pub fn assign_person_to_grave(&mut self, person_id: PersonId, grave_id: GraveId) {
        if self.grave(grave_id).is_some() {
            self.people.assign_to_grave(person_id, grave_id);
        }
    }

    pub fn unassign_person_from_grave(&mut self, person_id: PersonId) {
        self.people.unassign_from_grave(person_id);
    }

    pub fn people_in_grave(&self, grave_id: GraveId) -> Vec<&Person> {
        self.people.people_in_grave(grave_id)
    }

    pub fn search_people(&self, query: &str) -> Vec<&Person> {
        self.people.search(query)
    }

    pub fn people(&self) -> impl Iterator<Item = &Person> {
        self.people.people()
    }

    pub fn person(&self, id: PersonId) -> Option<&Person> {
        self.people.person(id)
    }

    pub fn person_mut(&mut self, id: PersonId) -> Option<&mut Person> {
        self.people.person_mut(id)
    }

    pub fn grave_for_person(&self, person_id: PersonId) -> Option<&Grave> {
        let grave_id = self.person(person_id)?.grave_id()?;
        self.grave(grave_id)
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};

    use super::*;

    fn rectangle() -> GraveRectangle {
        GraveRectangle::from_top_left_size(Point::new(0.0, 0.0), Size::new(10.0, 20.0))
    }

    fn create_person(cemetery: &mut Cemetery, grave_id: Option<GraveId>) -> PersonId {
        cemetery.create_person_with_details(
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            None,
            grave_id,
        )
    }

    #[test]
    fn assign_person_to_grave_ignores_missing_graves() {
        let mut cemetery = Cemetery::default();
        let person_id = create_person(&mut cemetery, None);

        cemetery.assign_person_to_grave(person_id, GraveId::new(99));

        assert_eq!(cemetery.person(person_id).and_then(Person::grave_id), None);
    }

    #[test]
    fn assign_person_to_existing_grave_links_person_and_grave() {
        let mut cemetery = Cemetery::default();
        let grave_id = cemetery.add_grave_with_color(rectangle(), GraveColor::default());
        let person_id = create_person(&mut cemetery, None);

        cemetery.assign_person_to_grave(person_id, grave_id);

        assert_eq!(
            cemetery.person(person_id).and_then(Person::grave_id),
            Some(grave_id)
        );
        assert_eq!(
            cemetery.grave_for_person(person_id).map(Grave::id),
            Some(grave_id)
        );
    }

    #[test]
    fn erase_grave_unassigns_people_in_that_grave() {
        let mut cemetery = Cemetery::default();
        let removed_grave = cemetery.add_grave_with_color(rectangle(), GraveColor::default());
        let remaining_grave = cemetery.add_grave_with_color(rectangle(), GraveColor::default());
        let removed_person = create_person(&mut cemetery, Some(removed_grave));
        let remaining_person = create_person(&mut cemetery, Some(remaining_grave));

        cemetery.erase_grave(removed_grave);

        assert!(cemetery.grave(removed_grave).is_none());
        assert_eq!(
            cemetery.person(removed_person).and_then(Person::grave_id),
            None
        );
        assert_eq!(
            cemetery.person(remaining_person).and_then(Person::grave_id),
            Some(remaining_grave)
        );
    }

    #[test]
    fn from_records_removes_dangling_grave_assignments() {
        let person = Person::from_parts(
            PersonId::new(1),
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            None,
            Some(GraveId::new(99)),
        );

        let cemetery = Cemetery::from_records(Vec::new(), vec![person]);

        assert_eq!(
            cemetery.person(PersonId::new(1)).and_then(Person::grave_id),
            None
        );
    }
}

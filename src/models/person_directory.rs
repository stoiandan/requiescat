use super::{GraveId, Person, PersonDate, PersonId};

#[derive(Debug, Default)]
pub struct PersonDirectory {
    people: Vec<Person>,
    next_person_id: i64,
}

impl PersonDirectory {
    pub fn create_person_with_details(
        &mut self,
        first_name: String,
        last_name: String,
        date_of_birth: PersonDate,
        date_of_decease: Option<PersonDate>,
        grave_id: Option<GraveId>,
    ) -> PersonId {
        let id = self.next_id();

        self.people.push(Person::from_parts(
            id,
            first_name,
            last_name,
            date_of_birth,
            date_of_decease,
            grave_id,
        ));

        id
    }

    pub fn assign_to_grave(&mut self, person_id: PersonId, grave_id: GraveId) {
        if let Some(person) = self.person_mut(person_id) {
            person.assign_to_grave(grave_id);
        }
    }

    pub fn unassign_from_grave(&mut self, person_id: PersonId) {
        if let Some(person) = self.person_mut(person_id) {
            person.unassign_from_grave();
        }
    }

    pub fn unassign_all_from_grave(&mut self, grave_id: GraveId) {
        for person in &mut self.people {
            if person.grave_id() == Some(grave_id) {
                person.unassign_from_grave();
            }
        }
    }

    pub fn people_in_grave(&self, grave_id: GraveId) -> Vec<&Person> {
        self.people
            .iter()
            .filter(|person| person.grave_id() == Some(grave_id))
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&Person> {
        let query = query.trim().to_lowercase();

        self.people
            .iter()
            .filter(|person| person.matches_query(&query))
            .collect()
    }

    pub fn person(&self, id: PersonId) -> Option<&Person> {
        self.people.iter().find(|person| person.id() == id)
    }

    pub fn person_mut(&mut self, id: PersonId) -> Option<&mut Person> {
        self.people.iter_mut().find(|person| person.id() == id)
    }

    fn next_id(&mut self) -> PersonId {
        self.next_person_id += 1;
        PersonId::new(self.next_person_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_person(directory: &mut PersonDirectory, grave_id: Option<GraveId>) -> PersonId {
        directory.create_person_with_details(
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            None,
            grave_id,
        )
    }

    #[test]
    fn create_person_assigns_incrementing_ids_and_optional_grave() {
        let mut directory = PersonDirectory::default();
        let grave_id = GraveId::new(3);

        let first = create_person(&mut directory, Some(grave_id));
        let second = create_person(&mut directory, None);

        assert_eq!(first, PersonId::new(1));
        assert_eq!(second, PersonId::new(2));
        assert_eq!(
            directory.person(first).and_then(Person::grave_id),
            Some(grave_id)
        );
        assert_eq!(directory.person(second).and_then(Person::grave_id), None);
    }

    #[test]
    fn people_in_grave_returns_only_assigned_people() {
        let mut directory = PersonDirectory::default();
        let target_grave = GraveId::new(1);
        let other_grave = GraveId::new(2);
        let assigned = create_person(&mut directory, Some(target_grave));
        create_person(&mut directory, Some(other_grave));
        create_person(&mut directory, None);

        let people = directory.people_in_grave(target_grave);

        assert_eq!(people.len(), 1);
        assert_eq!(people[0].id(), assigned);
    }

    #[test]
    fn unassign_all_from_grave_keeps_other_assignments() {
        let mut directory = PersonDirectory::default();
        let target_grave = GraveId::new(1);
        let other_grave = GraveId::new(2);
        let removed = create_person(&mut directory, Some(target_grave));
        let kept = create_person(&mut directory, Some(other_grave));

        directory.unassign_all_from_grave(target_grave);

        assert_eq!(directory.person(removed).and_then(Person::grave_id), None);
        assert_eq!(
            directory.person(kept).and_then(Person::grave_id),
            Some(other_grave)
        );
    }

    #[test]
    fn search_trims_query_and_matches_people() {
        let mut directory = PersonDirectory::default();
        let ada = create_person(&mut directory, None);
        let grace = create_person(&mut directory, None);

        directory
            .person_mut(ada)
            .expect("person should exist")
            .set_first_name("Ada".to_owned());
        directory
            .person_mut(grace)
            .expect("person should exist")
            .set_last_name("Hopper".to_owned());

        let people = directory.search("  hop  ");

        assert_eq!(people.len(), 1);
        assert_eq!(people[0].id(), grace);
    }
}

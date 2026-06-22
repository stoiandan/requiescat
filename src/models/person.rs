use super::{GraveId, PersonDate, PersonId};

#[derive(Debug, Clone)]
pub struct Person {
    id: PersonId,
    first_name: String,
    last_name: String,
    date_of_birth: PersonDate,
    date_of_decease: Option<PersonDate>,
    grave_id: Option<GraveId>,
}

impl Person {
    pub fn from_parts(
        id: PersonId,
        first_name: String,
        last_name: String,
        date_of_birth: PersonDate,
        date_of_decease: Option<PersonDate>,
        grave_id: Option<GraveId>,
    ) -> Self {
        Self {
            id,
            first_name,
            last_name,
            date_of_birth,
            date_of_decease,
            grave_id,
        }
    }

    pub fn id(&self) -> PersonId {
        self.id
    }

    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn date_of_birth(&self) -> &str {
        self.date_of_birth.as_str()
    }

    pub fn date_of_decease_text(&self) -> &str {
        self.date_of_decease
            .as_ref()
            .map(PersonDate::as_str)
            .unwrap_or_default()
    }

    pub fn grave_id(&self) -> Option<GraveId> {
        self.grave_id
    }

    pub fn with_first_name(mut self, value: String) -> Self {
        self.first_name = value;
        self
    }

    pub fn set_first_name(&mut self, value: String) {
        *self = self.clone().with_first_name(value);
    }

    pub fn with_last_name(mut self, value: String) -> Self {
        self.last_name = value;
        self
    }

    pub fn set_last_name(&mut self, value: String) {
        *self = self.clone().with_last_name(value);
    }

    pub fn with_date_of_birth(mut self, value: PersonDate) -> Self {
        self.date_of_birth = value;
        self
    }

    pub fn set_date_of_birth(&mut self, value: PersonDate) {
        *self = self.clone().with_date_of_birth(value);
    }

    pub fn with_date_of_decease(mut self, value: Option<PersonDate>) -> Self {
        self.date_of_decease = value;
        self
    }

    pub fn set_date_of_decease(&mut self, value: Option<PersonDate>) {
        *self = self.clone().with_date_of_decease(value);
    }

    pub fn assigned_to_grave(mut self, grave_id: GraveId) -> Self {
        self.grave_id = Some(grave_id);
        self
    }

    pub fn assign_to_grave(&mut self, grave_id: GraveId) {
        *self = self.clone().assigned_to_grave(grave_id);
    }

    pub fn unassigned_from_grave(mut self) -> Self {
        self.grave_id = None;
        self
    }

    pub fn unassign_from_grave(&mut self) {
        *self = self.clone().unassigned_from_grave();
    }

    pub fn display_name(&self) -> String {
        format!("{} {}", self.first_name.trim(), self.last_name.trim())
    }

    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();

        if query.is_empty() {
            return true;
        }

        self.first_name.to_lowercase().contains(&query)
            || self.last_name.to_lowercase().contains(&query)
            || self.date_of_birth.as_str().to_lowercase().contains(&query)
            || self.date_of_decease_text().to_lowercase().contains(&query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_trims_first_and_last_name() {
        let person = Person::from_parts(
            PersonId::new(1),
            " Ada ".to_owned(),
            " Lovelace ".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            None,
            None,
        );

        assert_eq!(person.display_name(), "Ada Lovelace");
    }

    #[test]
    fn matches_query_searches_identity_and_dates_case_insensitively() {
        let person = Person::from_parts(
            PersonId::new(1),
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            Some(PersonDate::parse("27-11-1852").unwrap()),
            None,
        );

        assert!(person.matches_query(""));
        assert!(person.matches_query("lovelace"));
        assert!(person.matches_query("ADA"));
        assert!(person.matches_query("27-11-1852"));
        assert!(!person.matches_query("hopper"));
    }

    #[test]
    fn assign_and_unassign_grave() {
        let mut person = Person::from_parts(
            PersonId::new(1),
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            None,
            None,
        );
        let grave_id = GraveId::new(7);

        person.assign_to_grave(grave_id);
        assert_eq!(person.grave_id(), Some(grave_id));

        person.unassign_from_grave();
        assert_eq!(person.grave_id(), None);
    }

    #[test]
    fn assigned_and_unassigned_return_updated_people_without_changing_the_original() {
        let person = Person::from_parts(
            PersonId::new(1),
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            None,
            None,
        );
        let grave_id = GraveId::new(7);

        let assigned = person.clone().assigned_to_grave(grave_id);
        let unassigned = assigned.clone().unassigned_from_grave();

        assert_eq!(person.grave_id(), None);
        assert_eq!(assigned.grave_id(), Some(grave_id));
        assert_eq!(unassigned.grave_id(), None);
    }
}

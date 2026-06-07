use super::{GraveId, PersonId};

#[derive(Debug, Clone)]
pub struct Person {
    id: PersonId,
    first_name: String,
    last_name: String,
    date_of_birth: String,
    date_of_decease: String,
    grave_id: Option<GraveId>,
}

impl Person {
    pub fn from_parts(
        id: PersonId,
        first_name: String,
        last_name: String,
        date_of_birth: String,
        date_of_decease: String,
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
        &self.date_of_birth
    }

    pub fn date_of_decease(&self) -> &str {
        &self.date_of_decease
    }

    pub fn grave_id(&self) -> Option<GraveId> {
        self.grave_id
    }

    pub fn set_first_name(&mut self, value: String) {
        self.first_name = value;
    }

    pub fn set_last_name(&mut self, value: String) {
        self.last_name = value;
    }

    pub fn set_date_of_birth(&mut self, value: String) {
        self.date_of_birth = value;
    }

    pub fn set_date_of_decease(&mut self, value: String) {
        self.date_of_decease = value;
    }

    pub fn assign_to_grave(&mut self, grave_id: GraveId) {
        self.grave_id = Some(grave_id);
    }

    pub fn unassign_from_grave(&mut self) {
        self.grave_id = None;
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
            || self.date_of_birth.to_lowercase().contains(&query)
            || self.date_of_decease.to_lowercase().contains(&query)
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
            String::new(),
            String::new(),
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
            "1815".to_owned(),
            "1852".to_owned(),
            None,
        );

        assert!(person.matches_query(""));
        assert!(person.matches_query("lovelace"));
        assert!(person.matches_query("ADA"));
        assert!(person.matches_query("1852"));
        assert!(!person.matches_query("hopper"));
    }

    #[test]
    fn assign_and_unassign_grave() {
        let mut person = Person::from_parts(
            PersonId::new(1),
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            "1815".to_owned(),
            String::new(),
            None,
        );
        let grave_id = GraveId::new(7);

        person.assign_to_grave(grave_id);
        assert_eq!(person.grave_id(), Some(grave_id));

        person.unassign_from_grave();
        assert_eq!(person.grave_id(), None);
    }
}

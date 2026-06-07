use crate::models::{Cemetery, Grave, GraveId, GraveRectangle, Person, PersonDate, PersonId};
use iced::{Point, Size};

pub trait CemeteryRepository {
    fn load(&self) -> Result<Cemetery, PersistenceError>;
    fn save(&mut self, cemetery: &Cemetery) -> Result<(), PersistenceError>;
}

#[derive(Debug)]
pub enum PersistenceError {
    StorageUnavailable,
}

#[derive(Debug, Clone, Copy)]
pub struct GraveRow {
    pub id: i64,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct PersonRow {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub date_of_decease: String,
    pub grave_id: Option<i64>,
}

impl From<Grave> for GraveRow {
    fn from(grave: Grave) -> Self {
        let rectangle = grave.rectangle();
        let top_left = rectangle.top_left();
        let size = rectangle.size();

        Self {
            id: grave.id().value(),
            x: top_left.x,
            y: top_left.y,
            width: size.width,
            height: size.height,
        }
    }
}

impl From<GraveRow> for Grave {
    fn from(row: GraveRow) -> Self {
        Grave::new(
            GraveId::new(row.id),
            GraveRectangle::from_top_left_size(
                Point::new(row.x, row.y),
                Size::new(row.width, row.height),
            ),
        )
    }
}

impl From<Person> for PersonRow {
    fn from(person: Person) -> Self {
        Self {
            id: person.id().value(),
            first_name: person.first_name().to_owned(),
            last_name: person.last_name().to_owned(),
            date_of_birth: person.date_of_birth().to_owned(),
            date_of_decease: person.date_of_decease_text().to_owned(),
            grave_id: person.grave_id().map(GraveId::value),
        }
    }
}

impl From<PersonRow> for Person {
    fn from(row: PersonRow) -> Self {
        Person::from_parts(
            PersonId::new(row.id),
            row.first_name,
            row.last_name,
            PersonDate::parse(&row.date_of_birth)
                .expect("persisted person birth date should be a valid dd-mm-yyyy date"),
            if row.date_of_decease.trim().is_empty() {
                None
            } else {
                Some(
                    PersonDate::parse(&row.date_of_decease)
                        .expect("persisted person decease date should be a valid dd-mm-yyyy date"),
                )
            },
            row.grave_id.map(GraveId::new),
        )
    }
}

use std::cell::Cell;
use std::collections::HashMap;

use super::Camera;
use super::drawing;
use super::interaction;
use super::{Tool, Toolbar, ToolbarAction};
use iced::widget::{
    button, canvas, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Background, Border, Color, Element, Length, Point, Renderer, Shadow, Theme, Vector};

use crate::localization::{Language, Localizer, MessageId};
use crate::models::{Cemetery, GraveId, GraveRectangle, Person, PersonDate, PersonId};

#[derive(Default)]
pub struct MapEditor {
    cemetery: Cemetery,
    toolbar: Toolbar,
    camera: Camera,
    selected_grave: Option<GraveId>,
    selected_person: Option<PersonId>,
    person_search: String,
    new_person: NewPersonDraft,
    person_edits: HashMap<PersonId, PersonEditDraft>,
    render_revision: u64,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenPersonDetails(PersonId),
    SubmitNewPerson,
    NewPersonFirstNameChanged(String),
    NewPersonLastNameChanged(String),
    NewPersonDateOfBirthChanged(String),
    NewPersonDateOfDeceaseChanged(String),
    CreateGrave(GraveRectangle),
    ToolBarAction(ToolbarAction),
    EraseGrave(GraveId),
    MoveGrave { id: GraveId, delta: iced::Vector },
    PanCamera(iced::Vector),
    ZoomCamera { cursor: Point, amount: f32 },
    SelectGrave(Option<GraveId>),
    SelectPerson(PersonId),
    PersonSearchChanged(String),
    AssignPersonToSelectedGrave(PersonId),
    UnassignPersonFromGrave(PersonId),
    UpdatePersonFirstName(PersonId, String),
    UpdatePersonLastName(PersonId, String),
    UpdatePersonDateOfBirth(PersonId, String),
    UpdatePersonDateOfDecease(PersonId, String),
    CommitPendingChanges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcome {
    Unchanged,
    Changed,
    DeferredChange,
    Commit,
}

impl MapEditor {
    pub fn from_cemetery(cemetery: Cemetery) -> Self {
        Self {
            cemetery,
            ..Default::default()
        }
    }

    pub fn update(&mut self, message: Message) -> UpdateOutcome {
        match message {
            Message::OpenPersonDetails(_) => UpdateOutcome::Unchanged,
            Message::SubmitNewPerson => {
                if self.submit_new_person() {
                    UpdateOutcome::Changed
                } else {
                    UpdateOutcome::Unchanged
                }
            }
            Message::NewPersonFirstNameChanged(value) => {
                self.new_person.first_name = value;
                UpdateOutcome::Unchanged
            }
            Message::NewPersonLastNameChanged(value) => {
                self.new_person.last_name = value;
                UpdateOutcome::Unchanged
            }
            Message::NewPersonDateOfBirthChanged(value) => {
                self.new_person.date_of_birth = value;
                UpdateOutcome::Unchanged
            }
            Message::NewPersonDateOfDeceaseChanged(value) => {
                self.new_person.date_of_decease = value;
                UpdateOutcome::Unchanged
            }
            Message::CreateGrave(rectangle) => {
                self.cemetery
                    .add_grave_with_color(rectangle, self.toolbar.selected_grave_color());
                self.invalidate_map();
                UpdateOutcome::Changed
            }
            Message::ToolBarAction(action) => {
                let previous_show_grid = self.toolbar.show_grid();
                self.toolbar.update(action);
                if self.toolbar.show_grid() != previous_show_grid {
                    self.invalidate_map();
                }
                UpdateOutcome::Unchanged
            }
            Message::EraseGrave(id) => {
                self.cemetery.erase_grave(id);
                if self.selected_grave == Some(id) {
                    self.selected_grave = None;
                }
                self.invalidate_map();
                UpdateOutcome::Changed
            }
            Message::MoveGrave { id, delta } => {
                self.cemetery.move_grave(id, delta);
                self.invalidate_map();
                UpdateOutcome::DeferredChange
            }
            Message::PanCamera(delta) => {
                self.camera.pan_by_canvas_delta(delta);
                self.invalidate_map();
                UpdateOutcome::Unchanged
            }
            Message::ZoomCamera { cursor, amount } => {
                self.camera.zoom_at(cursor, amount);
                self.invalidate_map();
                UpdateOutcome::Unchanged
            }
            Message::SelectGrave(id) => {
                if self.selected_grave != id {
                    self.selected_grave = id;
                    self.invalidate_map();
                }
                UpdateOutcome::Unchanged
            }
            Message::SelectPerson(id) => {
                self.selected_person = Some(id);
                let previous_grave = self.selected_grave;
                self.selected_grave = self.cemetery.grave_for_person(id).map(|grave| grave.id());
                if let Some(grave_id) = self.selected_grave {
                    self.center_camera_on_grave(grave_id);
                }
                if self.selected_grave != previous_grave {
                    self.invalidate_map();
                }
                UpdateOutcome::Unchanged
            }
            Message::PersonSearchChanged(value) => {
                self.person_search = value;
                UpdateOutcome::Unchanged
            }
            Message::AssignPersonToSelectedGrave(person_id) => {
                if let Some(grave_id) = self.selected_grave {
                    self.cemetery.assign_person_to_grave(person_id, grave_id);
                    self.invalidate_map();
                    UpdateOutcome::Changed
                } else {
                    UpdateOutcome::Unchanged
                }
            }
            Message::UnassignPersonFromGrave(person_id) => {
                self.cemetery.unassign_person_from_grave(person_id);
                self.invalidate_map();
                UpdateOutcome::Changed
            }
            Message::UpdatePersonFirstName(id, value) => {
                self.person_edits.entry(id).or_default().first_name = Some(value.clone());

                if !value.trim().is_empty()
                    && let Some(person) = self.cemetery.person_mut(id)
                {
                    person.set_first_name(value);
                    self.invalidate_map();
                    UpdateOutcome::Changed
                } else {
                    UpdateOutcome::Unchanged
                }
            }
            Message::UpdatePersonLastName(id, value) => {
                self.person_edits.entry(id).or_default().last_name = Some(value.clone());

                if !value.trim().is_empty()
                    && let Some(person) = self.cemetery.person_mut(id)
                {
                    person.set_last_name(value);
                    self.invalidate_map();
                    UpdateOutcome::Changed
                } else {
                    UpdateOutcome::Unchanged
                }
            }
            Message::UpdatePersonDateOfBirth(id, value) => {
                self.person_edits.entry(id).or_default().date_of_birth = Some(value.clone());

                if let Ok(date) = PersonDate::parse(&value)
                    && let Some(person) = self.cemetery.person_mut(id)
                {
                    person.set_date_of_birth(date);
                    self.invalidate_map();
                    UpdateOutcome::Changed
                } else {
                    UpdateOutcome::Unchanged
                }
            }
            Message::UpdatePersonDateOfDecease(id, value) => {
                self.person_edits.entry(id).or_default().date_of_decease = Some(value.clone());

                let date = if value.trim().is_empty() {
                    Some(None)
                } else {
                    PersonDate::parse(&value).ok().map(Some)
                };

                if let Some(date) = date
                    && let Some(person) = self.cemetery.person_mut(id)
                {
                    person.set_date_of_decease(date);
                    self.invalidate_map();
                    UpdateOutcome::Changed
                } else {
                    UpdateOutcome::Unchanged
                }
            }
            Message::CommitPendingChanges => UpdateOutcome::Commit,
        }
    }

    pub fn view<'a>(
        &'a self,
        localizer: &'a Localizer,
        save_status: Option<String>,
    ) -> Element<'a, Message> {
        let selected_grave = self.selected_grave();
        let mut map_row = row![
            canvas(LocalizedMapCanvas {
                editor: self,
                localizer,
            })
            .height(Length::Fill)
            .width(Length::Fill)
        ];

        if let Some(grave_id) = selected_grave {
            map_row = map_row.push(self.side_panel(localizer, grave_id));
        }

        let mut footer = row![self.toolbar.view().map(Message::ToolBarAction)]
            .spacing(8)
            .padding([8, 12]);

        if let Some(status) = save_status {
            footer = footer.push(
                container(text(status).size(12))
                    .padding([6, 10])
                    .align_y(iced::Alignment::Center),
            );
        }

        container(column![map_row.height(Length::Fill), footer])
            .style(|_| container::Style {
                border: iced::Border {
                    color: iced::Color::WHITE,
                    width: 2.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    pub fn person_directory_view<'a>(&'a self, localizer: &'a Localizer) -> Element<'a, Message> {
        container(scrollable(
            column![self.person_directory(localizer)]
                .spacing(16)
                .padding(14),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb8(15, 45, 48))),
            ..Default::default()
        })
        .into()
    }

    pub fn person_details_view<'a>(
        &'a self,
        localizer: &'a Localizer,
        person_id: PersonId,
    ) -> Element<'a, Message> {
        let content = if let Some(person) = self.cemetery.person(person_id) {
            column![
                text(localizer.text(MessageId::Person)).size(20),
                self.person_editor(localizer, person)
            ]
            .spacing(12)
            .padding(16)
        } else {
            column![text(localizer.text(MessageId::PersonNotFound)).size(20)].padding(16)
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb8(15, 45, 48))),
                ..Default::default()
            })
            .into()
    }

    pub fn new_person_view<'a>(&'a self, localizer: &'a Localizer) -> Element<'a, Message> {
        let grave_label = self
            .new_person
            .grave_id
            .map(|grave_id| {
                localizer.value(MessageId::WillAddToGrave, "grave", grave_id.to_string())
            })
            .unwrap_or_else(|| localizer.text(MessageId::WillCreateUnassigned));

        let submit = button(text(localizer.text(MessageId::AddPerson))).width(Length::Fill);
        let submit = if self.can_submit_new_person() {
            submit.on_press(Message::SubmitNewPerson)
        } else {
            submit
        };

        container(
            column![
                text(localizer.text(MessageId::NewPersonTitle)).size(20),
                text(grave_label).size(12).style(|_| text::Style {
                    color: Some(iced::Color::from_rgb8(190, 220, 218)),
                }),
                text_input(
                    &localizer.text(MessageId::FirstName),
                    &self.new_person.first_name
                )
                .on_input(Message::NewPersonFirstNameChanged)
                .padding(8),
                text_input(
                    &localizer.text(MessageId::LastName),
                    &self.new_person.last_name
                )
                .on_input(Message::NewPersonLastNameChanged)
                .padding(8),
                text_input(
                    &localizer.text(MessageId::DateOfBirthExample),
                    &self.new_person.date_of_birth
                )
                .on_input(Message::NewPersonDateOfBirthChanged)
                .padding(8),
                text_input(
                    &localizer.text(MessageId::DateOfDeceaseExample),
                    &self.new_person.date_of_decease
                )
                .on_input(Message::NewPersonDateOfDeceaseChanged)
                .padding(8),
                submit
            ]
            .spacing(10)
            .padding(16),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb8(15, 45, 48))),
            ..Default::default()
        })
        .into()
    }

    fn side_panel<'a>(
        &'a self,
        localizer: &'a Localizer,
        grave_id: GraveId,
    ) -> Element<'a, Message> {
        container(scrollable(
            column![self.grave_details(localizer, grave_id)]
                .spacing(16)
                .padding(14),
        ))
        .width(Length::Fixed(340.0))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb8(15, 45, 48))),
            border: iced::Border {
                color: iced::Color::from_rgb8(42, 139, 143),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn grave_details<'a>(
        &'a self,
        localizer: &'a Localizer,
        grave_id: GraveId,
    ) -> Element<'a, Message> {
        let mut content = column![
            text(localizer.value(MessageId::Grave, "grave", grave_id.to_string())).size(18)
        ]
        .spacing(8);

        let people = self.cemetery.people_in_grave(grave_id);

        if people.is_empty() {
            content = content.push(text(localizer.text(MessageId::NoPersonsAssociated)).style(
                |_| text::Style {
                    color: Some(iced::Color::from_rgb8(190, 220, 218)),
                },
            ));
        } else {
            for person in people {
                content = content.push(self.person_editor(localizer, person));
            }
        }

        container(content).into()
    }

    fn person_directory<'a>(&'a self, localizer: &'a Localizer) -> Element<'a, Message> {
        let mut content = column![
            text(localizer.text(MessageId::Persons)).size(18),
            text_input(
                &localizer.text(MessageId::SearchPeople),
                &self.person_search
            )
            .on_input(Message::PersonSearchChanged)
            .padding(8)
        ]
        .spacing(8);

        for person in self.cemetery.search_people(&self.person_search) {
            content = content.push(self.person_result(localizer, person));
        }

        container(content).into()
    }

    fn person_result<'a>(
        &'a self,
        localizer: &'a Localizer,
        person: &'a Person,
    ) -> Element<'a, Message> {
        let id = person.id();
        let selected_grave = self.selected_grave();

        let info = mouse_area(
            container(
                column![
                    text(person.display_name()).size(16),
                    text(person.date_of_birth().to_owned())
                        .size(12)
                        .style(|_| text::Style {
                            color: Some(iced::Color::from_rgb8(190, 220, 218)),
                        })
                ]
                .spacing(2),
            )
            .width(Length::Fill),
        )
        .on_double_click(Message::OpenPersonDetails(id));

        let mut content = row![info].spacing(8);

        if person.grave_id().is_some() {
            content = content.push(
                button(text(localizer.text(MessageId::GoToGrave)))
                    .on_press(Message::SelectPerson(id)),
            );
        }

        if selected_grave.is_some() {
            let assignment_action = if person.grave_id().is_some() {
                button(text(localizer.text(MessageId::Unassign)))
                    .on_press(Message::UnassignPersonFromGrave(id))
                    .style(danger_button)
            } else {
                button(text(localizer.text(MessageId::Assign)))
                    .on_press(Message::AssignPersonToSelectedGrave(id))
            };

            content = content.push(assignment_action);
        }

        container(content)
            .width(Length::Fill)
            .padding([10, 12])
            .style(|_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb8(24, 64, 68))),
                border: iced::Border {
                    color: iced::Color::from_rgb8(42, 139, 143),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn person_editor<'a>(
        &'a self,
        localizer: &'a Localizer,
        person: &'a Person,
    ) -> Element<'a, Message> {
        let id = person.id();
        let edits = self.person_edits.get(&id);
        let first_name = edits
            .and_then(|edit| edit.first_name.as_deref())
            .unwrap_or_else(|| person.first_name());
        let last_name = edits
            .and_then(|edit| edit.last_name.as_deref())
            .unwrap_or_else(|| person.last_name());
        let date_of_birth = edits
            .and_then(|edit| edit.date_of_birth.as_deref())
            .unwrap_or_else(|| person.date_of_birth());
        let date_of_decease = edits
            .and_then(|edit| edit.date_of_decease.as_deref())
            .unwrap_or_else(|| person.date_of_decease_text());

        let details = column![
            text(person.display_name()).size(16),
            text(localizer.value(MessageId::Born, "date", person.date_of_birth()))
                .size(12)
                .style(|_| text::Style {
                    color: Some(iced::Color::from_rgb8(190, 220, 218)),
                }),
        ]
        .spacing(2);

        let details = if let Some(grave_id) = person.grave_id() {
            details.push(
                text(localizer.value(MessageId::Grave, "grave", grave_id.to_string()))
                    .size(12)
                    .style(|_| text::Style {
                        color: Some(iced::Color::from_rgb8(190, 220, 218)),
                    }),
            )
        } else {
            details
        };

        container(
            column![
                details,
                text_input(&localizer.text(MessageId::FirstName), first_name)
                    .on_input(move |value| Message::UpdatePersonFirstName(id, value))
                    .padding(7),
                text_input(&localizer.text(MessageId::LastName), last_name)
                    .on_input(move |value| Message::UpdatePersonLastName(id, value))
                    .padding(7),
                text_input(&localizer.text(MessageId::DateOfBirth), date_of_birth)
                    .on_input(move |value| Message::UpdatePersonDateOfBirth(id, value))
                    .padding(7),
                text_input(&localizer.text(MessageId::DateOfDecease), date_of_decease)
                    .on_input(move |value| Message::UpdatePersonDateOfDecease(id, value))
                    .padding(7)
            ]
            .spacing(6),
        )
        .padding(10)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb8(24, 64, 68))),
            border: iced::Border {
                color: iced::Color::from_rgb8(42, 139, 143),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    pub fn cemetery(&self) -> &Cemetery {
        &self.cemetery
    }

    pub(super) fn camera(&self) -> Camera {
        self.camera
    }

    fn selected_grave(&self) -> Option<GraveId> {
        self.selected_grave
            .filter(|id| self.cemetery.grave(*id).is_some())
    }

    pub fn prepare_new_person(&mut self) {
        self.new_person = NewPersonDraft {
            grave_id: self.selected_grave(),
            ..Default::default()
        };
    }

    pub fn can_submit_new_person(&self) -> bool {
        self.new_person.is_valid()
    }

    pub fn submit_new_person(&mut self) -> bool {
        if !self.can_submit_new_person() {
            return false;
        }

        let id = self.cemetery.create_person_with_details(
            self.new_person.first_name.trim().to_owned(),
            self.new_person.last_name.trim().to_owned(),
            self.new_person
                .date_of_birth()
                .expect("new person should be valid before submit"),
            self.new_person.date_of_decease(),
            self.new_person.grave_id,
        );

        self.selected_person = Some(id);
        self.selected_grave = self.new_person.grave_id;
        self.new_person = NewPersonDraft::default();
        self.invalidate_map();

        true
    }

    fn center_camera_on_grave(&mut self, grave_id: GraveId) {
        if let Some(grave) = self.cemetery.grave(grave_id) {
            self.camera.center_on(grave.rectangle().center());
            self.invalidate_map();
        }
    }

    pub(super) fn selected_tool(&self) -> Tool {
        self.toolbar.selected_tool()
    }

    pub(super) fn show_grid(&self) -> bool {
        self.toolbar.show_grid()
    }

    fn invalidate_map(&mut self) {
        self.render_revision = self.render_revision.saturating_add(1);
    }
}

#[derive(Debug, Clone, Default)]
struct NewPersonDraft {
    first_name: String,
    last_name: String,
    date_of_birth: String,
    date_of_decease: String,
    grave_id: Option<GraveId>,
}

#[derive(Debug, Clone, Default)]
struct PersonEditDraft {
    first_name: Option<String>,
    last_name: Option<String>,
    date_of_birth: Option<String>,
    date_of_decease: Option<String>,
}

impl NewPersonDraft {
    fn is_valid(&self) -> bool {
        !self.first_name.trim().is_empty()
            && !self.last_name.trim().is_empty()
            && self.date_of_birth().is_some()
            && (self.date_of_decease.trim().is_empty() || self.date_of_decease().is_some())
    }

    fn date_of_birth(&self) -> Option<PersonDate> {
        PersonDate::parse(&self.date_of_birth).ok()
    }

    fn date_of_decease(&self) -> Option<PersonDate> {
        if self.date_of_decease.trim().is_empty() {
            None
        } else {
            PersonDate::parse(&self.date_of_decease).ok()
        }
    }
}

fn danger_button(_: &Theme, status: button::Status) -> button::Style {
    let pressed = status == button::Status::Pressed;
    let hovered = status == button::Status::Hovered;

    button::Style {
        background: Some(Background::Color(if pressed {
            Color::from_rgb8(122, 25, 36)
        } else if hovered {
            Color::from_rgb8(178, 45, 58)
        } else {
            Color::from_rgb8(151, 34, 47)
        })),
        text_color: Color::WHITE,
        border: Border {
            color: Color::from_rgb8(225, 91, 105),
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.35),
            offset: if pressed {
                Vector::new(0.0, 1.0)
            } else {
                Vector::new(0.0, 2.0)
            },
            blur_radius: 2.0,
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use iced::Size;

    use crate::models::GraveColor;

    use super::*;

    fn rectangle_at(x: f32, y: f32) -> GraveRectangle {
        GraveRectangle::from_top_left_size(Point::new(x, y), Size::new(40.0, 20.0))
    }

    fn create_person(editor: &mut MapEditor, grave_id: Option<GraveId>) -> PersonId {
        editor.cemetery.create_person_with_details(
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            None,
            grave_id,
        )
    }

    #[test]
    fn assign_person_to_selected_grave_requires_a_selected_grave() {
        let mut editor = MapEditor::default();
        let grave_id = editor
            .cemetery
            .add_grave_with_color(rectangle_at(0.0, 0.0), GraveColor::default());
        let person_id = create_person(&mut editor, None);

        editor.update(Message::AssignPersonToSelectedGrave(person_id));

        assert_eq!(
            editor.cemetery.person(person_id).and_then(Person::grave_id),
            None
        );

        editor.update(Message::SelectGrave(Some(grave_id)));
        editor.update(Message::AssignPersonToSelectedGrave(person_id));

        assert_eq!(
            editor.cemetery.person(person_id).and_then(Person::grave_id),
            Some(grave_id)
        );
    }

    #[test]
    fn unassign_person_from_grave_clears_their_grave() {
        let mut editor = MapEditor::default();
        let grave_id = editor
            .cemetery
            .add_grave_with_color(rectangle_at(0.0, 0.0), GraveColor::default());
        let person_id = create_person(&mut editor, Some(grave_id));

        editor.update(Message::UnassignPersonFromGrave(person_id));

        assert_eq!(
            editor.cemetery.person(person_id).and_then(Person::grave_id),
            None
        );
    }

    #[test]
    fn select_person_goes_to_their_grave() {
        let mut editor = MapEditor::default();
        let grave_id = editor
            .cemetery
            .add_grave_with_color(rectangle_at(100.0, 200.0), GraveColor::default());
        let person_id = create_person(&mut editor, Some(grave_id));

        editor.update(Message::SelectPerson(person_id));

        assert_eq!(editor.selected_person, Some(person_id));
        assert_eq!(editor.selected_grave, Some(grave_id));
        assert_eq!(editor.camera.offset, Point::new(-180.0, -40.0));
    }

    #[test]
    fn new_person_requires_name_and_valid_birth_date() {
        let mut editor = MapEditor::default();

        editor.update(Message::NewPersonFirstNameChanged("Ada".to_owned()));
        editor.update(Message::NewPersonLastNameChanged("Lovelace".to_owned()));
        editor.update(Message::NewPersonDateOfBirthChanged(
            "not a date".to_owned(),
        ));

        assert!(!editor.can_submit_new_person());

        editor.update(Message::NewPersonDateOfBirthChanged(
            "10-12-1815".to_owned(),
        ));
        editor.update(Message::NewPersonDateOfDeceaseChanged(
            "also bad".to_owned(),
        ));

        assert!(!editor.can_submit_new_person());

        editor.update(Message::NewPersonDateOfDeceaseChanged(
            "27-11-1852".to_owned(),
        ));

        assert!(editor.can_submit_new_person());
    }

    #[test]
    fn person_date_edit_preserves_intermediate_text_until_valid() {
        let mut editor = MapEditor::default();
        let person_id = create_person(&mut editor, None);

        editor.update(Message::UpdatePersonDateOfBirth(
            person_id,
            "10-12-181".to_owned(),
        ));

        assert_eq!(
            editor
                .person_edits
                .get(&person_id)
                .and_then(|draft| draft.date_of_birth.as_deref()),
            Some("10-12-181")
        );
        assert_eq!(
            editor.cemetery.person(person_id).map(Person::date_of_birth),
            Some("10-12-1815")
        );

        editor.update(Message::UpdatePersonDateOfBirth(
            person_id,
            "10-12-1816".to_owned(),
        ));

        assert_eq!(
            editor.cemetery.person(person_id).map(Person::date_of_birth),
            Some("10-12-1816")
        );
    }

    #[test]
    fn danger_button_uses_red_background() {
        let style = danger_button(&Theme::Dark, button::Status::Active);

        assert_eq!(
            style.background,
            Some(Background::Color(Color::from_rgb8(151, 34, 47)))
        );
        assert_eq!(style.text_color, Color::WHITE);
    }

    #[test]
    fn created_graves_use_selected_toolbar_color() {
        let mut editor = MapEditor::default();
        let color = crate::models::GraveColor::from_rgb8(122, 77, 161);

        editor.update(Message::ToolBarAction(ToolbarAction::SelectGraveColor(
            color,
        )));
        editor.update(Message::CreateGrave(rectangle_at(0.0, 0.0)));

        assert_eq!(editor.cemetery.graves()[0].color(), color);
    }

    #[test]
    fn update_reports_only_persistent_model_changes() {
        let mut editor = MapEditor::default();

        assert_eq!(
            editor.update(Message::PersonSearchChanged("Ada".to_owned())),
            UpdateOutcome::Unchanged
        );
        assert_eq!(
            editor.update(Message::PanCamera(Vector::new(4.0, 2.0))),
            UpdateOutcome::Unchanged
        );
        assert_eq!(
            editor.update(Message::CreateGrave(rectangle_at(0.0, 0.0))),
            UpdateOutcome::Changed
        );
        assert_eq!(
            editor.update(Message::CommitPendingChanges),
            UpdateOutcome::Commit
        );
    }
}

pub struct CanvasState {
    pub(super) drag: DragState,
    map_cache: canvas::Cache<Renderer>,
    map_cache_key: Cell<Option<MapCacheKey>>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            drag: DragState::default(),
            map_cache: canvas::Cache::new(),
            map_cache_key: Cell::new(None),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) enum DragState {
    #[default]
    None,
    Drawing {
        start: Point,
        current: Point,
    },
    Panning {
        previous_cursor: Point,
    },
    MovingGrave {
        id: GraveId,
        previous_cursor: Point,
    },
}

impl CanvasState {
    pub fn left_pressed_at(&self) -> Option<Point> {
        match self.drag {
            DragState::Drawing { start, .. } => Some(start),
            _ => None,
        }
    }

    pub fn current_drag_position(&self) -> Option<Point> {
        match self.drag {
            DragState::Drawing { current, .. } => Some(current),
            _ => None,
        }
    }
}

struct LocalizedMapCanvas<'a> {
    editor: &'a MapEditor,
    localizer: &'a Localizer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MapCacheKey {
    render_revision: u64,
    zoom: u32,
    offset_x: u32,
    offset_y: u32,
    selected_grave: Option<GraveId>,
    show_grid: bool,
    language: Language,
}

impl MapCacheKey {
    fn new(editor: &MapEditor, language: Language) -> Self {
        Self {
            render_revision: editor.render_revision,
            zoom: editor.camera.zoom.to_bits(),
            offset_x: editor.camera.offset.x.to_bits(),
            offset_y: editor.camera.offset.y.to_bits(),
            selected_grave: editor.selected_grave,
            show_grid: editor.show_grid(),
            language,
        }
    }
}

impl canvas::Program<Message> for LocalizedMapCanvas<'_> {
    type State = CanvasState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _: &Theme,
        bounds: iced::Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let cache_key = MapCacheKey::new(self.editor, self.localizer.language());
        if state.map_cache_key.get() != Some(cache_key) {
            state.map_cache.clear();
            state.map_cache_key.set(Some(cache_key));
        }

        let map = state.map_cache.draw(renderer, bounds.size(), |frame| {
            if self.editor.show_grid() {
                drawing::grid(frame, &self.editor.camera, bounds);
            }

            drawing::graves(
                frame,
                &self.editor.cemetery,
                &self.editor.camera,
                self.editor.selected_grave,
                |grave_id| {
                    self.localizer
                        .value(MessageId::GraveCanvas, "grave", grave_id.to_string())
                },
            );
        });

        if state.current_drag_position().is_some() {
            let mut preview = canvas::Frame::new(renderer, bounds.size());
            drawing::grave_preview(&mut preview, state, &self.editor.camera);
            vec![map, preview.into_geometry()]
        } else {
            vec![map]
        }
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        interaction::handle_event(self.editor, state, event, bounds, cursor)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        match self.editor.selected_tool() {
            Tool::Select => iced::mouse::Interaction::Pointer,
            Tool::Draw | Tool::StampGrave => iced::mouse::Interaction::Crosshair,
            Tool::Grab => match state.drag {
                DragState::Panning { .. } | DragState::MovingGrave { .. } => {
                    iced::mouse::Interaction::Grabbing
                }
                _ => iced::mouse::Interaction::Grab,
            },
            Tool::Erase => iced::mouse::Interaction::NoDrop,
        }
    }
}

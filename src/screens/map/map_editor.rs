use std::collections::HashMap;

use super::Camera;
use super::map_canvas::LocalizedMapCanvas;
use super::{Tool, Toolbar, ToolbarAction};
use iced::widget::{
    button, canvas, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Background, Border, Color, Element, Length, Point, Shadow, Theme, Vector};

use crate::localization::{Localizer, MessageId};
use crate::models::{
    Cemetery, DelimiterId, GraveGps, GraveId, GraveRectangle, Person, PersonDate, PersonId, Tags,
};
use crate::screens::ConfirmationDialog;
use crate::theme;

#[derive(Default)]
pub struct MapEditor {
    cemetery: Cemetery,
    toolbar: Toolbar,
    camera: Camera,
    selected_grave: Option<GraveId>,
    last_created_grave: Option<GraveId>,
    canvas_cursor: Option<Point>,
    selected_person: Option<PersonId>,
    person_search: String,
    grave_search: String,
    new_person: NewPersonDraft,
    person_edits: HashMap<PersonId, PersonEditDraft>,
    grave_gps_edits: HashMap<GraveId, GraveGpsEditDraft>,
    grave_tag_inputs: HashMap<GraveId, String>,
    pending_grave_tag_removal: Option<PendingGraveTagRemoval>,
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
    NewPersonTagsChanged(String),
    CreateGrave(GraveRectangle),
    CreateDelimiter(GraveRectangle),
    ToolBarAction(ToolbarAction),
    CanvasCursorChanged(Option<Point>),
    DuplicateLastGraveAtCursor,
    EraseGrave(GraveId),
    EraseDelimiter(DelimiterId),
    MoveMapObject {
        id: MapObjectId,
        delta: iced::Vector,
    },
    RotateMapObject {
        id: MapObjectId,
        rotation_degrees: f32,
    },
    PanCamera(iced::Vector),
    ZoomCamera {
        cursor: Point,
        amount: f32,
    },
    SelectGrave(Option<GraveId>),
    UpdateGraveLatitude(GraveId, String),
    UpdateGraveLongitude(GraveId, String),
    GraveTagInputChanged(GraveId, String),
    AddGraveTags(GraveId),
    RequestRemoveGraveTag(GraveId, String),
    CancelRemoveGraveTag,
    ConfirmRemoveGraveTag(GraveId, String),
    SelectPerson(PersonId),
    PersonSearchChanged(String),
    GraveSearchChanged(String),
    AssignPersonToSelectedGrave(PersonId),
    UnassignPersonFromGrave(PersonId),
    UpdatePersonFirstName(PersonId, String),
    UpdatePersonLastName(PersonId, String),
    UpdatePersonDateOfBirth(PersonId, String),
    UpdatePersonDateOfDecease(PersonId, String),
    UpdatePersonTags(PersonId, String),
    CommitPendingChanges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectId {
    Grave(GraveId),
    Delimiter(DelimiterId),
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
            Message::NewPersonTagsChanged(value) => {
                self.new_person.tags = value;
                UpdateOutcome::Unchanged
            }
            Message::CreateGrave(rectangle) => {
                let id = self
                    .cemetery
                    .add_grave_with_color(rectangle, self.toolbar.selected_grave_color());
                self.selected_grave = Some(id);
                self.remember_created_grave(id);
                self.invalidate_map();
                UpdateOutcome::Changed
            }
            Message::CreateDelimiter(rectangle) => {
                self.cemetery.add_delimiter_with_color_and_type(
                    rectangle,
                    self.toolbar.selected_grave_color(),
                    self.toolbar.selected_delimiter_type(),
                );
                self.invalidate_map();
                UpdateOutcome::Changed
            }
            Message::ToolBarAction(action) => {
                let previous_show_grid = self.toolbar.show_grid();
                let previous_rotation_targets = self.rotation_targets();
                self.toolbar.update(action);
                if self.toolbar.show_grid() != previous_show_grid
                    || self.rotation_targets() != previous_rotation_targets
                {
                    self.invalidate_map();
                }
                UpdateOutcome::Unchanged
            }
            Message::CanvasCursorChanged(cursor) => {
                self.canvas_cursor = cursor;
                UpdateOutcome::Unchanged
            }
            Message::DuplicateLastGraveAtCursor => self.duplicate_last_grave_at_cursor(),
            Message::EraseGrave(id) => {
                self.cemetery.erase_grave(id);
                if self.selected_grave == Some(id) {
                    self.selected_grave = None;
                }
                self.invalidate_map();
                UpdateOutcome::Changed
            }
            Message::EraseDelimiter(id) => {
                self.cemetery.erase_delimiter(id);
                self.invalidate_map();
                UpdateOutcome::Changed
            }
            Message::MoveMapObject { id, delta } => {
                self.move_map_object(id, delta);
                self.invalidate_map();
                UpdateOutcome::DeferredChange
            }
            Message::RotateMapObject {
                id,
                rotation_degrees,
            } => {
                if self.rotate_map_object(id, rotation_degrees) {
                    self.invalidate_map();
                    UpdateOutcome::DeferredChange
                } else {
                    UpdateOutcome::Unchanged
                }
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
            Message::UpdateGraveLatitude(id, value) => self.update_grave_latitude(id, value),
            Message::UpdateGraveLongitude(id, value) => self.update_grave_longitude(id, value),
            Message::GraveTagInputChanged(id, value) => {
                self.grave_tag_inputs.insert(id, value);
                UpdateOutcome::Unchanged
            }
            Message::AddGraveTags(id) => self.add_grave_tags(id),
            Message::RequestRemoveGraveTag(id, tag) => {
                if self.cemetery.grave(id).is_some() {
                    self.pending_grave_tag_removal =
                        Some(PendingGraveTagRemoval { grave_id: id, tag });
                }
                UpdateOutcome::Unchanged
            }
            Message::CancelRemoveGraveTag => {
                self.pending_grave_tag_removal = None;
                UpdateOutcome::Unchanged
            }
            Message::ConfirmRemoveGraveTag(id, tag) => self.remove_grave_tag(id, &tag),
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
            Message::GraveSearchChanged(value) => {
                self.grave_search = value;
                self.invalidate_map();
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
            Message::UpdatePersonFirstName(id, value) => self.update_person_draft(
                id,
                value,
                PersonEditDraft::with_first_name,
                Person::with_first_name,
            ),
            Message::UpdatePersonLastName(id, value) => self.update_person_draft(
                id,
                value,
                PersonEditDraft::with_last_name,
                Person::with_last_name,
            ),
            Message::UpdatePersonDateOfBirth(id, value) => self.update_person_draft(
                id,
                value,
                PersonEditDraft::with_date_of_birth,
                Person::with_date_of_birth,
            ),
            Message::UpdatePersonDateOfDecease(id, value) => self.update_person_draft(
                id,
                value,
                PersonEditDraft::with_date_of_decease,
                Person::with_date_of_decease,
            ),
            Message::UpdatePersonTags(id, value) => {
                self.update_person_draft(id, value, PersonEditDraft::with_tags, Person::with_tags)
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

        let mut footer = row![self.toolbar.view(localizer).map(Message::ToolBarAction)]
            .spacing(8)
            .padding([8, 12]);

        if let Some(status) = save_status {
            footer = footer.push(
                container(text(status).size(12))
                    .padding([6, 10])
                    .align_y(iced::Alignment::Center),
            );
        }

        let content = container(column![map_row.height(Length::Fill), footer])
            .style(|_| container::Style {
                border: iced::Border {
                    color: theme::TEXT_PRIMARY,
                    width: 2.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into();

        if let Some(pending) = &self.pending_grave_tag_removal {
            return ConfirmationDialog::new(
                localizer.text(MessageId::ConfirmRemoveTagTitle),
                localizer.value(
                    MessageId::ConfirmRemoveTagDescription,
                    "tag",
                    pending.tag.as_str(),
                ),
                localizer.text(MessageId::Cancel),
                localizer.text(MessageId::Delete),
                Message::CancelRemoveGraveTag,
                Message::ConfirmRemoveGraveTag(pending.grave_id, pending.tag.clone()),
            )
            .overlay(content);
        }

        content
    }

    pub fn person_directory_view<'a>(&'a self, localizer: &'a Localizer) -> Element<'a, Message> {
        scrolling_surface(self.person_directory(localizer))
    }

    pub fn grave_directory_view<'a>(&'a self, localizer: &'a Localizer) -> Element<'a, Message> {
        scrolling_surface(self.grave_directory(localizer))
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

        full_surface(content)
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

        full_surface(
            column![
                text(localizer.text(MessageId::NewPersonTitle)).size(20),
                text(grave_label).size(12).style(|_| text::Style {
                    color: Some(theme::TEXT_MUTED),
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
                text_input(&localizer.text(MessageId::Tags), &self.new_person.tags)
                    .on_input(Message::NewPersonTagsChanged)
                    .padding(8),
                submit
            ]
            .spacing(10)
            .padding(16),
        )
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
            background: Some(iced::Background::Color(theme::SURFACE)),
            text_color: Some(theme::TEXT_PRIMARY),
            border: iced::Border {
                color: theme::BORDER,
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

        if let Some(grave) = self.cemetery.grave(grave_id) {
            let gps = GraveGpsEditDraft::for_grave(grave, self.grave_gps_edits.get(&grave_id));

            content = content
                .push(section_heading(localizer.text(MessageId::GraveGps)))
                .push(
                    row![
                        text(localizer.text(MessageId::Latitude))
                            .width(Length::Fixed(82.0))
                            .size(13),
                        text_input(&localizer.text(MessageId::Latitude), &gps.latitude)
                            .on_input(move |value| Message::UpdateGraveLatitude(grave_id, value))
                            .padding(8)
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                )
                .push(
                    row![
                        text(localizer.text(MessageId::Longitude))
                            .width(Length::Fixed(82.0))
                            .size(13),
                        text_input(&localizer.text(MessageId::Longitude), &gps.longitude)
                            .on_input(move |value| Message::UpdateGraveLongitude(grave_id, value))
                            .padding(8)
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                );
        }

        let people = self.cemetery.people_in_grave(grave_id);

        if people.is_empty() {
            content = content.push(text(localizer.text(MessageId::NoPersonsAssociated)).style(
                |_| text::Style {
                    color: Some(theme::TEXT_MUTED),
                },
            ));
        } else {
            for person in people {
                content = content.push(self.person_editor(localizer, person));
            }
        }

        if let Some(grave) = self.cemetery.grave(grave_id) {
            content = content.push(self.grave_tag_editor(localizer, grave_id, grave.tags()));
        }

        container(content).into()
    }

    fn grave_tag_editor<'a>(
        &'a self,
        localizer: &'a Localizer,
        grave_id: GraveId,
        tags: &'a Tags,
    ) -> Element<'a, Message> {
        let input = self
            .grave_tag_inputs
            .get(&grave_id)
            .map(String::as_str)
            .unwrap_or_default();
        let add_button = button(text(localizer.text(MessageId::AddTag)));
        let add_button = if input.trim().is_empty() {
            add_button
        } else {
            add_button.on_press(Message::AddGraveTags(grave_id))
        };

        let mut content = column![
            section_heading(localizer.text(MessageId::Tags)),
            row![
                text_input(&localizer.text(MessageId::Tags), input)
                    .on_input(move |value| Message::GraveTagInputChanged(grave_id, value))
                    .on_submit(Message::AddGraveTags(grave_id))
                    .padding(8),
                add_button.padding([8, 12])
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
        ]
        .spacing(8);

        if !tags.is_empty() {
            content = content.push(tag_badges(Some(grave_id), tags.values()));
        }

        container(content).padding(10).into()
    }

    fn person_directory<'a>(&'a self, localizer: &'a Localizer) -> Element<'a, Message> {
        directory(
            localizer.text(MessageId::Persons),
            localizer.text(MessageId::SearchPeople),
            &self.person_search,
            Message::PersonSearchChanged,
            self.cemetery
                .search_people(&self.person_search)
                .into_iter()
                .map(|person| self.person_result(localizer, person)),
        )
    }

    fn grave_directory<'a>(&'a self, localizer: &'a Localizer) -> Element<'a, Message> {
        directory(
            localizer.text(MessageId::Graves),
            localizer.text(MessageId::SearchGraves),
            &self.grave_search,
            Message::GraveSearchChanged,
            self.cemetery
                .search_graves(&self.grave_search)
                .into_iter()
                .map(|grave| self.grave_result(localizer, grave.id(), grave.tags())),
        )
    }

    fn grave_result<'a>(
        &'a self,
        localizer: &'a Localizer,
        grave_id: GraveId,
        tags: &'a Tags,
    ) -> Element<'a, Message> {
        let mut details = column![
            text(localizer.value(MessageId::Grave, "grave", grave_id.to_string())).size(16)
        ]
        .spacing(6);

        if !tags.is_empty() {
            details = details.push(tag_badges(None, tags.values()));
        }

        container(
            row![
                details.width(Length::Fill),
                button(text(localizer.text(MessageId::GoToGrave)))
                    .on_press(Message::SelectGrave(Some(grave_id)))
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding([10, 12])
        .style(|_| card_style())
        .into()
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
                            color: Some(theme::TEXT_MUTED),
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
            .style(|_| card_style())
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
        let tags = edits
            .and_then(|edit| edit.tags.as_deref())
            .map(str::to_owned)
            .unwrap_or_else(|| person.tags_text());

        let details = column![
            text(person.display_name()).size(16),
            text(localizer.value(MessageId::Born, "date", person.date_of_birth()))
                .size(12)
                .style(|_| text::Style {
                    color: Some(theme::TEXT_MUTED),
                }),
        ]
        .spacing(2);

        let details = if let Some(grave_id) = person.grave_id() {
            details.push(
                text(localizer.value(MessageId::Grave, "grave", grave_id.to_string()))
                    .size(12)
                    .style(|_| text::Style {
                        color: Some(theme::TEXT_MUTED),
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
                    .padding(7),
                text_input(&localizer.text(MessageId::Tags), &tags)
                    .on_input(move |value| Message::UpdatePersonTags(id, value))
                    .padding(7)
            ]
            .spacing(6),
        )
        .padding(10)
        .style(|_| card_style())
        .into()
    }

    pub fn cemetery(&self) -> &Cemetery {
        &self.cemetery
    }

    fn update_person_draft<Value>(
        &mut self,
        id: PersonId,
        value: String,
        draft_update: impl FnOnce(PersonEditDraft, String) -> (PersonEditDraft, Option<Value>),
        person_update: impl FnOnce(Person, Value) -> Person,
    ) -> UpdateOutcome {
        let draft = self.person_edits.remove(&id).unwrap_or_default();
        let (draft, parsed) = draft_update(draft, value);
        self.person_edits.insert(id, draft);

        let Some(parsed) = parsed else {
            return UpdateOutcome::Unchanged;
        };

        self.update_person(id, |person| person_update(person, parsed))
    }

    fn update_person(
        &mut self,
        id: PersonId,
        update: impl FnOnce(Person) -> Person,
    ) -> UpdateOutcome {
        if self.cemetery.update_person(id, update) {
            self.invalidate_map();
            UpdateOutcome::Changed
        } else {
            UpdateOutcome::Unchanged
        }
    }

    fn update_grave_latitude(&mut self, id: GraveId, value: String) -> UpdateOutcome {
        self.update_grave_gps(id, |draft| draft.latitude = value)
    }

    fn update_grave_longitude(&mut self, id: GraveId, value: String) -> UpdateOutcome {
        self.update_grave_gps(id, |draft| draft.longitude = value)
    }

    fn update_grave_gps(
        &mut self,
        id: GraveId,
        update: impl FnOnce(&mut GraveGpsEditDraft),
    ) -> UpdateOutcome {
        let Some(grave) = self.cemetery.grave(id) else {
            return UpdateOutcome::Unchanged;
        };

        let mut draft = GraveGpsEditDraft::for_grave(grave, self.grave_gps_edits.get(&id));
        update(&mut draft);
        self.update_grave_gps_draft(id, draft)
    }

    fn update_grave_gps_draft(&mut self, id: GraveId, draft: GraveGpsEditDraft) -> UpdateOutcome {
        self.grave_gps_edits.insert(id, draft.clone());

        let Some(gps) = draft.gps() else {
            return UpdateOutcome::Unchanged;
        };

        self.cemetery.update_grave_gps(id, gps);
        self.grave_gps_edits.remove(&id);
        self.invalidate_map();
        UpdateOutcome::Changed
    }

    fn add_grave_tags(&mut self, id: GraveId) -> UpdateOutcome {
        let Some(grave) = self.cemetery.grave(id) else {
            return UpdateOutcome::Unchanged;
        };

        let input = self.grave_tag_inputs.get(&id).cloned().unwrap_or_default();
        let new_tags = Tags::parse(&input);
        if new_tags.is_empty() {
            return UpdateOutcome::Unchanged;
        }

        self.grave_tag_inputs.remove(&id);
        self.update_grave_tags(id, grave.tags().merged(&new_tags))
    }

    fn remove_grave_tag(&mut self, id: GraveId, tag: &str) -> UpdateOutcome {
        let Some(grave) = self.cemetery.grave(id) else {
            self.pending_grave_tag_removal = None;
            return UpdateOutcome::Unchanged;
        };

        let tags = grave.tags().without(tag);
        self.pending_grave_tag_removal = None;
        self.update_grave_tags(id, tags)
    }

    fn update_grave_tags(&mut self, id: GraveId, tags: Tags) -> UpdateOutcome {
        if self.cemetery.update_grave_tags(id, tags) {
            self.invalidate_map();
            UpdateOutcome::Changed
        } else {
            UpdateOutcome::Unchanged
        }
    }

    pub(super) fn camera(&self) -> Camera {
        self.camera
    }

    pub(super) fn selected_grave(&self) -> Option<GraveId> {
        self.selected_grave
            .filter(|id| self.cemetery.grave(*id).is_some())
    }

    pub(super) fn highlighted_graves(&self) -> Vec<GraveId> {
        if self.grave_search.trim().is_empty() {
            Vec::new()
        } else {
            self.cemetery
                .search_graves(&self.grave_search)
                .into_iter()
                .map(|grave| grave.id())
                .collect()
        }
    }

    pub fn clear_grave_search(&mut self) {
        if !self.grave_search.is_empty() {
            self.grave_search.clear();
            self.invalidate_map();
        }
    }

    pub fn prepare_new_person(&mut self) {
        self.new_person = NewPersonDraft {
            grave_id: self.selected_grave(),
            ..Default::default()
        };
    }

    pub fn can_submit_new_person(&self) -> bool {
        self.new_person.details().is_some()
    }

    pub fn submit_new_person(&mut self) -> bool {
        let Some(details) = self.new_person.details() else {
            return false;
        };

        let id = self.cemetery.create_person_with_details(
            details.first_name,
            details.last_name,
            details.date_of_birth,
            details.date_of_decease,
            self.new_person.grave_id,
            details.tags,
        );

        self.selected_person = Some(id);
        self.selected_grave = self.new_person.grave_id;
        self.new_person = NewPersonDraft::default();
        self.invalidate_map();

        true
    }

    pub fn can_duplicate_last_grave(&self) -> bool {
        self.last_created_grave().is_some()
    }

    fn center_camera_on_grave(&mut self, grave_id: GraveId) {
        if let Some(grave) = self.cemetery.grave(grave_id) {
            self.camera.center_on(grave.rectangle().center());
            self.invalidate_map();
        }
    }

    fn move_map_object(&mut self, id: MapObjectId, delta: Vector) {
        match id {
            MapObjectId::Grave(id) => self.cemetery.move_grave(id, delta),
            MapObjectId::Delimiter(id) => self.cemetery.move_delimiter(id, delta),
        }
    }

    fn rotate_map_object(&mut self, id: MapObjectId, rotation_degrees: f32) -> bool {
        match id {
            MapObjectId::Grave(id) => self.cemetery.rotate_grave(id, rotation_degrees),
            MapObjectId::Delimiter(id) => self.cemetery.rotate_delimiter(id, rotation_degrees),
        }
    }

    fn duplicate_last_grave_at_cursor(&mut self) -> UpdateOutcome {
        let (Some(last_created_grave), Some(top_left)) =
            (self.last_created_grave(), self.canvas_cursor)
        else {
            return UpdateOutcome::Unchanged;
        };

        let rectangle =
            GraveRectangle::from_top_left_size(top_left, last_created_grave.rectangle().size());
        let id = self
            .cemetery
            .add_grave_with_color(rectangle, last_created_grave.color());
        self.cemetery
            .rotate_grave(id, last_created_grave.rotation_degrees());
        self.selected_grave = Some(id);
        self.remember_created_grave(id);
        self.invalidate_map();

        UpdateOutcome::Changed
    }

    fn remember_created_grave(&mut self, id: GraveId) {
        self.last_created_grave = Some(id);
    }

    fn last_created_grave(&self) -> Option<crate::models::Grave> {
        self.last_created_grave
            .and_then(|id| self.cemetery.grave(id))
            .cloned()
    }

    pub(super) fn selected_tool(&self) -> Tool {
        self.toolbar.selected_tool()
    }

    pub(super) fn show_grid(&self) -> bool {
        self.toolbar.show_grid()
    }

    pub(super) fn selected_delimiter_type(&self) -> crate::models::DelimiterType {
        self.toolbar.selected_delimiter_type()
    }

    pub(super) fn render_revision(&self) -> u64 {
        self.render_revision
    }

    pub(super) fn object_at(&self, point: Point) -> Option<MapObjectId> {
        self.cemetery
            .grave_at(point)
            .map(MapObjectId::Grave)
            .or_else(|| {
                self.cemetery
                    .delimiter_at(point)
                    .map(MapObjectId::Delimiter)
            })
    }

    pub(super) fn rotation_targets(&self) -> Vec<MapObjectId> {
        if !matches!(self.selected_tool(), Tool::Grab) {
            return Vec::new();
        }

        self.cemetery
            .graves()
            .iter()
            .map(|grave| MapObjectId::Grave(grave.id()))
            .chain(
                self.cemetery
                    .delimiters()
                    .iter()
                    .map(|delimiter| MapObjectId::Delimiter(delimiter.id())),
            )
            .collect()
    }

    pub(super) fn rotation_handle_position(&self, id: MapObjectId) -> Option<Point> {
        let rectangle = self.object_rectangle(id)?;
        let rotation_degrees = self.object_rotation_degrees(id)?;
        let handle_offset = 24.0 / self.camera.zoom.max(0.1);

        Some(rectangle.point_at_rotated(
            rectangle.size().width / 2.0,
            -handle_offset,
            rotation_degrees,
        ))
    }

    pub(super) fn rotation_degrees_for_cursor(
        &self,
        id: MapObjectId,
        world_cursor: Point,
    ) -> Option<f32> {
        let center = self.object_rectangle(id)?.center();
        let delta = world_cursor - center;

        Some(delta.y.atan2(delta.x).to_degrees() + 90.0)
    }

    fn object_rectangle(&self, id: MapObjectId) -> Option<GraveRectangle> {
        match id {
            MapObjectId::Grave(id) => self.cemetery.grave(id).map(|grave| grave.rectangle()),
            MapObjectId::Delimiter(id) => self
                .cemetery
                .delimiter(id)
                .map(|delimiter| delimiter.rectangle()),
        }
    }

    fn object_rotation_degrees(&self, id: MapObjectId) -> Option<f32> {
        match id {
            MapObjectId::Grave(id) => self
                .cemetery
                .grave(id)
                .map(|grave| grave.rotation_degrees()),
            MapObjectId::Delimiter(id) => self
                .cemetery
                .delimiter(id)
                .map(|delimiter| delimiter.rotation_degrees()),
        }
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
    tags: String,
    grave_id: Option<GraveId>,
}

struct NewPersonDetails {
    first_name: String,
    last_name: String,
    date_of_birth: PersonDate,
    date_of_decease: Option<PersonDate>,
    tags: Tags,
}

#[derive(Debug, Clone, Default)]
struct PersonEditDraft {
    first_name: Option<String>,
    last_name: Option<String>,
    date_of_birth: Option<String>,
    date_of_decease: Option<String>,
    tags: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct GraveGpsEditDraft {
    latitude: String,
    longitude: String,
}

#[derive(Debug, Clone)]
struct PendingGraveTagRemoval {
    grave_id: GraveId,
    tag: String,
}

impl NewPersonDraft {
    fn details(&self) -> Option<NewPersonDetails> {
        let first_name = non_empty_trimmed(&self.first_name)?;
        let last_name = non_empty_trimmed(&self.last_name)?;
        let date_of_birth = PersonDate::parse(&self.date_of_birth).ok()?;
        let date_of_decease = parse_optional_person_date(&self.date_of_decease)?;

        Some(NewPersonDetails {
            first_name,
            last_name,
            date_of_birth,
            date_of_decease,
            tags: Tags::parse(&self.tags),
        })
    }
}

impl PersonEditDraft {
    fn with_first_name(self, value: String) -> (Self, Option<String>) {
        let parsed = (!value.trim().is_empty()).then(|| value.clone());
        (
            Self {
                first_name: Some(value),
                ..self
            },
            parsed,
        )
    }

    fn with_last_name(self, value: String) -> (Self, Option<String>) {
        let parsed = (!value.trim().is_empty()).then(|| value.clone());
        (
            Self {
                last_name: Some(value),
                ..self
            },
            parsed,
        )
    }

    fn with_date_of_birth(self, value: String) -> (Self, Option<PersonDate>) {
        let parsed = PersonDate::parse(&value).ok();
        (
            Self {
                date_of_birth: Some(value),
                ..self
            },
            parsed,
        )
    }

    fn with_date_of_decease(self, value: String) -> (Self, Option<Option<PersonDate>>) {
        let parsed = parse_optional_person_date(&value);
        (
            Self {
                date_of_decease: Some(value),
                ..self
            },
            parsed,
        )
    }

    fn with_tags(self, value: String) -> (Self, Option<Tags>) {
        let parsed = Tags::parse(&value);
        (
            Self {
                tags: Some(value),
                ..self
            },
            Some(parsed),
        )
    }
}

impl GraveGpsEditDraft {
    fn for_grave(grave: &crate::models::Grave, draft: Option<&Self>) -> Self {
        if let Some(draft) = draft {
            return draft.clone();
        }

        grave.gps().map_or_else(Self::default, |gps| Self {
            latitude: gps.latitude_text(),
            longitude: gps.longitude_text(),
        })
    }

    fn gps(&self) -> Option<Option<GraveGps>> {
        if self.latitude.trim().is_empty() && self.longitude.trim().is_empty() {
            Some(None)
        } else {
            GraveGps::parse_parts(&self.latitude, &self.longitude)
                .ok()
                .map(Some)
        }
    }
}

fn non_empty_trimmed(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn parse_optional_person_date(value: &str) -> Option<Option<PersonDate>> {
    if value.trim().is_empty() {
        Some(None)
    } else {
        PersonDate::parse(value).ok().map(Some)
    }
}

fn full_surface<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| surface_style())
        .into()
}

fn scrolling_surface<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    full_surface(scrollable(column![content].spacing(16).padding(14)))
}

fn surface_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::SURFACE)),
        text_color: Some(theme::TEXT_PRIMARY),
        ..Default::default()
    }
}

fn card_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::SURFACE_RAISED)),
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..surface_style()
    }
}

fn directory<'a>(
    title: String,
    placeholder: String,
    search: &'a str,
    on_search: impl Fn(String) -> Message + 'a,
    results: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            text(title).size(18),
            text_input(&placeholder, search)
                .on_input(on_search)
                .padding(8)
        ]
        .spacing(8)
        .extend(results),
    )
    .into()
}

fn section_heading<'a>(value: String) -> iced::widget::Text<'a> {
    text(value).size(14).style(|_| text::Style {
        color: Some(theme::TEXT_MUTED),
    })
}

fn tag_badges<'a>(grave_id: Option<GraveId>, tags: &'a [String]) -> Element<'a, Message> {
    row(tags.iter().map(|tag| tag_badge(grave_id, tag)))
        .spacing(6)
        .wrap()
        .vertical_spacing(6)
        .into()
}

fn tag_badge<'a>(grave_id: Option<GraveId>, tag: &'a str) -> Element<'a, Message> {
    let content: Element<'a, Message> = if let Some(grave_id) = grave_id {
        row![
            text(tag).size(12),
            button(text("x").size(11))
                .padding([0, 4])
                .style(tag_remove_button)
                .on_press(Message::RequestRemoveGraveTag(grave_id, tag.to_owned()))
        ]
        .spacing(4)
        .align_y(iced::Alignment::Start)
        .into()
    } else {
        text(tag).size(12).into()
    };

    container(content)
        .padding([5, if grave_id.is_some() { 6 } else { 8 }])
        .style(|_| tag_badge_style())
        .into()
}

fn tag_badge_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::ACCENT_REST)),
        text_color: Some(theme::TEXT_PRIMARY),
        border: Border {
            color: theme::BORDER_STRONG,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn tag_remove_button(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => theme::DANGER,
            button::Status::Pressed => theme::DANGER_PRESSED,
            button::Status::Disabled | button::Status::Active => Color::TRANSPARENT,
        })),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn danger_button(_: &Theme, status: button::Status) -> button::Style {
    let pressed = status == button::Status::Pressed;
    let hovered = status == button::Status::Hovered;

    button::Style {
        background: Some(Background::Color(if pressed {
            theme::DANGER_PRESSED
        } else if hovered {
            theme::DANGER_HOVER
        } else {
            theme::DANGER
        })),
        text_color: Color::WHITE,
        border: Border {
            color: theme::DANGER_BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow {
            color: theme::SHADOW,
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
            Tags::default(),
        )
    }

    fn create_grave(editor: &mut MapEditor, x: f32, y: f32) -> GraveId {
        editor
            .cemetery
            .add_grave_with_color(rectangle_at(x, y), GraveColor::default())
    }

    fn grave_tags(editor: &MapEditor, grave_id: GraveId) -> Option<String> {
        editor
            .cemetery
            .grave(grave_id)
            .map(|grave| grave.tags_text())
    }

    #[test]
    fn assign_person_to_selected_grave_requires_a_selected_grave() {
        let mut editor = MapEditor::default();
        let grave_id = create_grave(&mut editor, 0.0, 0.0);
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
        let grave_id = create_grave(&mut editor, 0.0, 0.0);
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
        let grave_id = create_grave(&mut editor, 100.0, 200.0);
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

        assert_eq!(style.background, Some(Background::Color(theme::DANGER)));
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
    fn duplicate_last_grave_requires_a_created_grave_and_canvas_cursor() {
        let mut editor = MapEditor::default();

        assert!(editor.last_created_grave().is_none());
        assert_eq!(
            editor.update(Message::DuplicateLastGraveAtCursor),
            UpdateOutcome::Unchanged
        );

        editor.update(Message::CreateGrave(rectangle_at(0.0, 0.0)));
        assert!(editor.last_created_grave().is_some());
        assert_eq!(
            editor.update(Message::DuplicateLastGraveAtCursor),
            UpdateOutcome::Unchanged
        );
        assert_eq!(editor.cemetery.graves().len(), 1);
    }

    #[test]
    fn duplicate_last_grave_uses_current_size_color_and_rotation_with_a_new_id() {
        let mut editor = MapEditor::default();
        let color = crate::models::GraveColor::from_rgb8(122, 77, 161);

        editor.update(Message::ToolBarAction(ToolbarAction::SelectGraveColor(
            color,
        )));
        editor.update(Message::CreateGrave(rectangle_at(0.0, 0.0)));
        editor.update(Message::RotateMapObject {
            id: MapObjectId::Grave(GraveId::new(1)),
            rotation_degrees: 35.0,
        });
        editor.update(Message::CanvasCursorChanged(Some(Point::new(100.0, 200.0))));

        assert_eq!(
            editor.update(Message::DuplicateLastGraveAtCursor),
            UpdateOutcome::Changed
        );

        let duplicated = editor.cemetery.grave(GraveId::new(2)).unwrap();
        assert_eq!(duplicated.rectangle().top_left(), Point::new(100.0, 200.0));
        assert_eq!(duplicated.rectangle().size(), rectangle_at(0.0, 0.0).size());
        assert_eq!(duplicated.color(), color);
        assert_eq!(duplicated.rotation_degrees(), 35.0);
    }

    #[test]
    fn created_delimiters_use_selected_toolbar_color_and_type() {
        let mut editor = MapEditor::default();
        let color = crate::models::GraveColor::from_rgb8(50, 123, 171);

        editor.update(Message::ToolBarAction(ToolbarAction::SelectGraveColor(
            color,
        )));
        editor.update(Message::ToolBarAction(ToolbarAction::SelectDelimiterType(
            crate::models::DelimiterType::Road,
        )));
        editor.update(Message::CreateDelimiter(rectangle_at(0.0, 0.0)));

        assert_eq!(editor.cemetery.delimiters()[0].color(), color);
        assert_eq!(
            editor.cemetery.delimiters()[0].delimiter_type(),
            crate::models::DelimiterType::Road
        );
    }

    #[test]
    fn grave_gps_edit_preserves_intermediate_text_until_valid() {
        let mut editor = MapEditor::default();
        let grave_id = create_grave(&mut editor, 0.0, 0.0);

        assert_eq!(
            editor.update(Message::UpdateGraveLatitude(grave_id, "51° 30′".to_owned())),
            UpdateOutcome::Unchanged
        );
        assert_eq!(
            editor
                .grave_gps_edits
                .get(&grave_id)
                .map(|draft| draft.latitude.as_str()),
            Some("51° 30′")
        );
        assert_eq!(
            editor
                .cemetery
                .grave(grave_id)
                .and_then(|grave| grave.gps()),
            None
        );

        assert_eq!(
            editor.update(Message::UpdateGraveLatitude(
                grave_id,
                "51° 30′ 26.64″ N".to_owned()
            )),
            UpdateOutcome::Unchanged
        );
        assert_eq!(
            editor.update(Message::UpdateGraveLongitude(
                grave_id,
                "0° 7′ 40.08″ W".to_owned()
            )),
            UpdateOutcome::Changed
        );

        assert!(!editor.grave_gps_edits.contains_key(&grave_id));
        assert_eq!(
            editor
                .cemetery
                .grave(grave_id)
                .and_then(|grave| grave.gps())
                .map(|gps| gps.to_string()),
            Some("51° 30′ 26.64″ N, 0° 7′ 40.08″ W".to_owned())
        );
    }

    #[test]
    fn empty_grave_gps_clears_saved_coordinates() {
        let mut editor = MapEditor::default();
        let grave_id = create_grave(&mut editor, 0.0, 0.0);
        editor.update(Message::UpdateGraveLatitude(
            grave_id,
            "51° 30′ 26.64″ N".to_owned(),
        ));
        editor.update(Message::UpdateGraveLongitude(
            grave_id,
            "0° 7′ 40.08″ W".to_owned(),
        ));

        assert_eq!(
            editor.update(Message::UpdateGraveLatitude(grave_id, " ".to_owned())),
            UpdateOutcome::Unchanged
        );
        assert_eq!(
            editor.update(Message::UpdateGraveLongitude(grave_id, " ".to_owned())),
            UpdateOutcome::Changed
        );

        assert_eq!(
            editor
                .cemetery
                .grave(grave_id)
                .and_then(|grave| grave.gps()),
            None
        );
    }

    #[test]
    fn grave_tags_are_added_from_input_and_removed_after_confirmation() {
        let mut editor = MapEditor::default();
        let grave_id = create_grave(&mut editor, 0.0, 0.0);
        editor.update(Message::GraveTagInputChanged(
            grave_id,
            "family plot, veteran".to_owned(),
        ));

        assert_eq!(
            editor.update(Message::AddGraveTags(grave_id)),
            UpdateOutcome::Changed
        );
        assert!(!editor.grave_tag_inputs.contains_key(&grave_id));
        assert_eq!(
            grave_tags(&editor, grave_id),
            Some("family plot, veteran".to_owned())
        );
        assert_eq!(
            editor.update(Message::RequestRemoveGraveTag(
                grave_id,
                "family plot".to_owned()
            )),
            UpdateOutcome::Unchanged
        );
        assert!(editor.pending_grave_tag_removal.is_some());
        assert_eq!(
            grave_tags(&editor, grave_id),
            Some("family plot, veteran".to_owned())
        );

        assert_eq!(
            editor.update(Message::ConfirmRemoveGraveTag(
                grave_id,
                "family plot".to_owned()
            )),
            UpdateOutcome::Changed
        );
        assert!(editor.pending_grave_tag_removal.is_none());
        assert_eq!(grave_tags(&editor, grave_id), Some("veteran".to_owned()));
    }

    #[test]
    fn grave_search_highlights_matching_graves_without_selecting_them() {
        let mut editor = MapEditor::default();
        let matching = create_grave(&mut editor, 0.0, 0.0);
        let other = create_grave(&mut editor, 100.0, 0.0);
        editor
            .cemetery
            .update_grave_tags(matching, Tags::parse("family plot, veteran"));
        editor
            .cemetery
            .update_grave_tags(other, Tags::parse("unmarked"));

        assert_eq!(
            editor.update(Message::GraveSearchChanged("veteran".to_owned())),
            UpdateOutcome::Unchanged
        );

        assert_eq!(editor.highlighted_graves(), vec![matching]);
        assert_eq!(editor.selected_grave(), None);

        editor.clear_grave_search();

        assert!(editor.highlighted_graves().is_empty());
    }

    #[test]
    fn person_tag_edits_are_searchable() {
        let mut editor = MapEditor::default();
        let grave_id = create_grave(&mut editor, 0.0, 0.0);
        let person_id = create_person(&mut editor, Some(grave_id));

        assert_eq!(
            editor.update(Message::UpdatePersonTags(
                person_id,
                "first programmer, countess".to_owned()
            )),
            UpdateOutcome::Changed
        );

        assert_eq!(editor.cemetery.search_people("first programmer").len(), 1);
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

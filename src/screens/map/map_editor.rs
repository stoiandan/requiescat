use super::Camera;
use super::drawing;
use super::interaction;
use super::{Tool, Toolbar, ToolbarAction};
use iced::widget::{
    button, canvas, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Element, Length, Point, Renderer, Theme};

use crate::models::{Cemetery, GraveId, GraveRectangle, Person, PersonId};

pub struct MapEditor {
    cemetery: Cemetery,
    toolbar: Toolbar,
    selected_grave: Option<GraveId>,
    selected_person: Option<PersonId>,
    person_search: String,
    new_person: NewPersonDraft,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenPersonDirectory,
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
    SelectGrave(Option<GraveId>),
    SelectPerson(PersonId),
    PersonSearchChanged(String),
    AssignPersonToSelectedGrave(PersonId),
    UnassignPersonFromGrave(PersonId),
    UpdatePersonFirstName(PersonId, String),
    UpdatePersonLastName(PersonId, String),
    UpdatePersonDateOfBirth(PersonId, String),
    UpdatePersonDateOfDecease(PersonId, String),
}

impl Default for MapEditor {
    fn default() -> Self {
        Self {
            cemetery: Cemetery::default(),
            toolbar: Toolbar::default(),
            selected_grave: None,
            selected_person: None,
            person_search: String::new(),
            new_person: NewPersonDraft::default(),
        }
    }
}

impl MapEditor {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::OpenPersonDirectory => {}
            Message::OpenPersonDetails(_) => {}
            Message::SubmitNewPerson => {
                self.submit_new_person();
            }
            Message::NewPersonFirstNameChanged(value) => {
                self.new_person.first_name = value;
            }
            Message::NewPersonLastNameChanged(value) => {
                self.new_person.last_name = value;
            }
            Message::NewPersonDateOfBirthChanged(value) => {
                self.new_person.date_of_birth = value;
            }
            Message::NewPersonDateOfDeceaseChanged(value) => {
                self.new_person.date_of_decease = value;
            }
            Message::CreateGrave(rectangle) => {
                let id = self.cemetery.add_grave(rectangle);
                self.selected_grave = Some(id);
            }
            Message::ToolBarAction(action) => self.toolbar.update(action),
            Message::EraseGrave(id) => {
                self.cemetery.erase_grave(id);
                if self.selected_grave == Some(id) {
                    self.selected_grave = None;
                }
            }
            Message::MoveGrave { id, delta } => {
                self.cemetery.move_grave(id, delta);
            }
            Message::SelectGrave(id) => {
                self.selected_grave = id;
            }
            Message::SelectPerson(id) => {
                self.selected_person = Some(id);
                self.selected_grave = self.cemetery.grave_for_person(id).map(|grave| grave.id());
            }
            Message::PersonSearchChanged(value) => {
                self.person_search = value;
            }
            Message::AssignPersonToSelectedGrave(person_id) => {
                if let Some(grave_id) = self.selected_grave {
                    self.cemetery.assign_person_to_grave(person_id, grave_id);
                }
            }
            Message::UnassignPersonFromGrave(person_id) => {
                self.cemetery.unassign_person_from_grave(person_id);
            }
            Message::UpdatePersonFirstName(id, value) => {
                if !value.trim().is_empty()
                    && let Some(person) = self.cemetery.person_mut(id)
                {
                    person.set_first_name(value);
                }
            }
            Message::UpdatePersonLastName(id, value) => {
                if !value.trim().is_empty()
                    && let Some(person) = self.cemetery.person_mut(id)
                {
                    person.set_last_name(value);
                }
            }
            Message::UpdatePersonDateOfBirth(id, value) => {
                if !value.trim().is_empty()
                    && let Some(person) = self.cemetery.person_mut(id)
                {
                    person.set_date_of_birth(value);
                }
            }
            Message::UpdatePersonDateOfDecease(id, value) => {
                if let Some(person) = self.cemetery.person_mut(id) {
                    person.set_date_of_decease(value);
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let selected_grave = self.selected_grave();
        let mut map_row = row![canvas(self).height(Length::Fill).width(Length::Fill)];

        if let Some(grave_id) = selected_grave {
            map_row = map_row.push(self.side_panel(grave_id));
        }

        container(column![
            map_row.height(Length::Fill),
            row![
                self.toolbar.view().map(Message::ToolBarAction),
                button(text("Persons"))
                    .on_press(Message::OpenPersonDirectory)
                    .height(44)
            ]
            .spacing(8)
            .padding([8, 12])
        ])
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

    pub fn person_directory_view(&self) -> Element<'_, Message> {
        container(scrollable(
            column![self.person_directory()].spacing(16).padding(14),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb8(15, 45, 48))),
            ..Default::default()
        })
        .into()
    }

    pub fn person_details_view(&self, person_id: PersonId) -> Element<'_, Message> {
        let content = if let Some(person) = self.cemetery.person(person_id) {
            column![
                text("Person").size(20),
                self.person_editor(person, self.selected_grave())
            ]
            .spacing(12)
            .padding(16)
        } else {
            column![text("Person not found").size(20)].padding(16)
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

    pub fn new_person_view(&self) -> Element<'_, Message> {
        let grave_label = self
            .new_person
            .grave_id
            .map(|grave_id| format!("Will be added to grave {}", grave_id))
            .unwrap_or_else(|| "Will be created unassigned".to_owned());

        let submit = button(text("Add person")).width(Length::Fill);
        let submit = if self.can_submit_new_person() {
            submit.on_press(Message::SubmitNewPerson)
        } else {
            submit
        };

        container(
            column![
                text("New Person").size(20),
                text(grave_label).size(12).style(|_| text::Style {
                    color: Some(iced::Color::from_rgb8(190, 220, 218)),
                }),
                text_input("First name", &self.new_person.first_name)
                    .on_input(Message::NewPersonFirstNameChanged)
                    .padding(8),
                text_input("Last name", &self.new_person.last_name)
                    .on_input(Message::NewPersonLastNameChanged)
                    .padding(8),
                text_input("Date of birth", &self.new_person.date_of_birth)
                    .on_input(Message::NewPersonDateOfBirthChanged)
                    .padding(8),
                text_input("Date of decease", &self.new_person.date_of_decease)
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

    fn side_panel(&self, grave_id: GraveId) -> Element<'_, Message> {
        container(scrollable(
            column![self.grave_details(grave_id)]
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

    fn grave_details(&self, grave_id: GraveId) -> Element<'_, Message> {
        let mut content = column![text("Grave").size(18)]
            .spacing(8)
            .push(text(format!("Selected grave {}", grave_id)));

        let people = self.cemetery.people_in_grave(grave_id);

        if people.is_empty() {
            content = content.push(text("No persons associated yet").style(|_| text::Style {
                color: Some(iced::Color::from_rgb8(190, 220, 218)),
            }));
        } else {
            for person in people {
                content = content.push(self.person_editor(person, Some(grave_id)));
            }
        }

        container(content).into()
    }

    fn person_directory(&self) -> Element<'_, Message> {
        let mut content = column![
            text("Persons").size(18),
            text_input("Search names or dates", &self.person_search)
                .on_input(Message::PersonSearchChanged)
                .padding(8)
        ]
        .spacing(8);

        for person in self.cemetery.search_people(&self.person_search) {
            content = content.push(self.person_result(person));
        }

        container(content).into()
    }

    fn person_result(&self, person: &Person) -> Element<'_, Message> {
        let id = person.id();

        mouse_area(
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
            }),
        )
        .on_double_click(Message::OpenPersonDetails(id))
        .into()
    }

    fn person_editor(
        &self,
        person: &Person,
        selected_grave: Option<GraveId>,
    ) -> Element<'_, Message> {
        let id = person.id();
        let mut actions =
            row![button(text("Select")).on_press(Message::SelectPerson(id))].spacing(8);

        if person.grave_id().is_some() {
            actions = actions
                .push(button(text("Unassign")).on_press(Message::UnassignPersonFromGrave(id)));
        }

        if let Some(grave_id) = selected_grave {
            if person.grave_id() != Some(grave_id) {
                actions = actions.push(
                    button(text("Assign")).on_press(Message::AssignPersonToSelectedGrave(id)),
                );
            }
        }

        let details = column![
            text(person.display_name()).size(16),
            text(format!("Born {}", person.date_of_birth()))
                .size(12)
                .style(|_| text::Style {
                    color: Some(iced::Color::from_rgb8(190, 220, 218)),
                }),
        ]
        .spacing(2);

        let details = if let Some(grave_id) = person.grave_id() {
            details.push(
                text(format!("Grave {}", grave_id))
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
                text_input("First name", person.first_name())
                    .on_input(move |value| Message::UpdatePersonFirstName(id, value))
                    .padding(7),
                text_input("Last name", person.last_name())
                    .on_input(move |value| Message::UpdatePersonLastName(id, value))
                    .padding(7),
                text_input("Date of birth", person.date_of_birth())
                    .on_input(move |value| Message::UpdatePersonDateOfBirth(id, value))
                    .padding(7),
                text_input("Date of decease", person.date_of_decease())
                    .on_input(move |value| Message::UpdatePersonDateOfDecease(id, value))
                    .padding(7),
                actions
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

    pub(super) fn cemetery(&self) -> &Cemetery {
        &self.cemetery
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
            self.new_person.date_of_birth.trim().to_owned(),
            self.new_person.date_of_decease.trim().to_owned(),
            self.new_person.grave_id,
        );

        self.selected_person = Some(id);
        self.selected_grave = self.new_person.grave_id;
        self.new_person = NewPersonDraft::default();

        true
    }

    pub(super) fn selected_tool(&self) -> Tool {
        self.toolbar.selected_tool()
    }

    pub(super) fn show_grid(&self) -> bool {
        self.toolbar.show_grid()
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

impl NewPersonDraft {
    fn is_valid(&self) -> bool {
        !self.first_name.trim().is_empty()
            && !self.last_name.trim().is_empty()
            && !self.date_of_birth.trim().is_empty()
    }
}

#[derive(Default)]
pub struct CanvasState {
    pub(super) drag: DragState,
    pub(super) camera: Camera,
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
    pub(super) fn camera(&self) -> &Camera {
        &self.camera
    }

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

impl canvas::Program<Message> for MapEditor {
    type State = CanvasState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _: &Theme,
        bounds: iced::Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        if self.show_grid() {
            drawing::grid(&mut frame, &state.camera, bounds);
        }

        drawing::grave_preview(&mut frame, state);
        drawing::graves(
            &mut frame,
            self.cemetery.graves(),
            &state.camera,
            self.selected_grave,
        );

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        interaction::handle_event(self, state, event, bounds, cursor)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        match self.selected_tool() {
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

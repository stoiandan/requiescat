mod models;
#[allow(dead_code)]
mod persistence;
mod screens;

use iced::widget::{container, text};
use iced::{Element, Length, Size, Subscription, Task, keyboard, window};
use screens::{MapEditor, MapEditorMessage};

fn main() -> iced::Result {
    iced::daemon(Requiesta::boot, Requiesta::update, Requiesta::view)
        .title(Requiesta::title)
        .subscription(Requiesta::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    MainWindowOpened(window::Id),
    PersonDirectoryOpened(window::Id),
    PersonDetailsOpened(window::Id, crate::models::PersonId),
    NewPersonWindowOpened(window::Id),
    WindowClosed(window::Id),
    Keyboard(keyboard::Event),
    Editor(MapEditorMessage),
}

#[derive(Default)]
struct Requiesta {
    editor: MapEditor,
    main_window: Option<window::Id>,
    person_directory_window: Option<window::Id>,
    person_detail_windows: Vec<(window::Id, crate::models::PersonId)>,
    new_person_window: Option<window::Id>,
}

impl Requiesta {
    fn boot() -> (Self, Task<Message>) {
        let (window_id, open) = window::open(window::Settings {
            size: Size::new(1100.0, 760.0),
            min_size: Some(Size::new(760.0, 520.0)),
            ..Default::default()
        });

        (
            Self {
                main_window: Some(window_id),
                ..Default::default()
            },
            open.map(Message::MainWindowOpened),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainWindowOpened(id) => {
                self.main_window = Some(id);
            }
            Message::PersonDirectoryOpened(id) => {
                self.person_directory_window = Some(id);
            }
            Message::PersonDetailsOpened(id, person_id) => {
                if !self
                    .person_detail_windows
                    .iter()
                    .any(|(window_id, _)| *window_id == id)
                {
                    self.person_detail_windows.push((id, person_id));
                }
            }
            Message::NewPersonWindowOpened(id) => {
                self.new_person_window = Some(id);
            }
            Message::WindowClosed(id) => {
                if Some(id) == self.main_window {
                    return iced::exit();
                }

                if Some(id) == self.person_directory_window {
                    self.person_directory_window = None;
                }

                if Some(id) == self.new_person_window {
                    self.new_person_window = None;
                }

                self.person_detail_windows
                    .retain(|(window_id, _)| *window_id != id);
            }
            Message::Keyboard(event) => {
                if should_open_new_person(&event) {
                    return self.open_new_person_dialog();
                }
            }
            Message::Editor(MapEditorMessage::OpenPersonDirectory) => {
                return self.open_person_directory();
            }
            Message::Editor(MapEditorMessage::OpenPersonDetails(person_id)) => {
                return self.open_person_details(person_id);
            }
            Message::Editor(MapEditorMessage::SubmitNewPerson) => {
                if self.editor.submit_new_person() {
                    if let Some(id) = self.new_person_window.take() {
                        return window::close(id);
                    }
                }
            }
            Message::Editor(message) => {
                self.editor.update(message);
            }
        }

        Task::none()
    }

    fn view(&self, window: window::Id) -> Element<'_, Message> {
        if Some(window) == self.person_directory_window {
            self.editor.person_directory_view().map(Message::Editor)
        } else if Some(window) == self.new_person_window {
            self.editor.new_person_view().map(Message::Editor)
        } else if let Some((_, person_id)) = self
            .person_detail_windows
            .iter()
            .find(|(window_id, _)| *window_id == window)
        {
            self.editor
                .person_details_view(*person_id)
                .map(Message::Editor)
        } else if Some(window) == self.main_window {
            self.editor.view().map(Message::Editor)
        } else {
            container(text("Unknown window"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill)
                .into()
        }
    }

    fn title(&self, window: window::Id) -> String {
        if Some(window) == self.person_directory_window {
            "Person Directory".to_owned()
        } else if Some(window) == self.new_person_window {
            "New Person".to_owned()
        } else if self
            .person_detail_windows
            .iter()
            .any(|(window_id, _)| *window_id == window)
        {
            "Person Details".to_owned()
        } else {
            "Requiesta".to_owned()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            window::close_events().map(Message::WindowClosed),
            keyboard::listen().map(Message::Keyboard),
        ])
    }

    fn open_person_directory(&mut self) -> Task<Message> {
        if let Some(id) = self.person_directory_window {
            return window::gain_focus(id);
        }

        let (id, open) = window::open(window::Settings {
            size: Size::new(460.0, 700.0),
            min_size: Some(Size::new(360.0, 420.0)),
            ..Default::default()
        });

        self.person_directory_window = Some(id);

        open.map(Message::PersonDirectoryOpened)
    }

    fn open_person_details(&mut self, person_id: crate::models::PersonId) -> Task<Message> {
        if let Some((id, _)) = self
            .person_detail_windows
            .iter()
            .find(|(_, open_person_id)| *open_person_id == person_id)
        {
            return window::gain_focus(*id);
        }

        let (id, open) = window::open(window::Settings {
            size: Size::new(430.0, 430.0),
            min_size: Some(Size::new(360.0, 360.0)),
            ..Default::default()
        });

        self.person_detail_windows.push((id, person_id));

        open.map(move |window_id| Message::PersonDetailsOpened(window_id, person_id))
    }

    fn open_new_person_dialog(&mut self) -> Task<Message> {
        if let Some(id) = self.new_person_window {
            return window::gain_focus(id);
        }

        self.editor.prepare_new_person();

        let (id, open) = window::open(window::Settings {
            size: Size::new(420.0, 420.0),
            min_size: Some(Size::new(360.0, 360.0)),
            ..Default::default()
        });

        self.new_person_window = Some(id);

        open.map(Message::NewPersonWindowOpened)
    }
}

fn should_open_new_person(event: &keyboard::Event) -> bool {
    let keyboard::Event::KeyPressed {
        key,
        physical_key,
        modifiers,
        repeat,
        ..
    } = event
    else {
        return false;
    };

    !repeat && modifiers.command() && key.to_latin(*physical_key) == Some('n')
}

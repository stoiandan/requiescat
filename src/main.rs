#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod persistence;
mod screens;

use std::path::PathBuf;

use iced::widget::{container, text};
use iced::{Element, Length, Size, Subscription, Task, keyboard, window};
use persistence::{CemeteryFile, CemeteryLibrary, CemeteryRepository, SqliteCemeteryRepository};
use screens::{MapEditor, MapEditorMessage, StartMenuMessage, start_menu_view};

fn main() -> iced::Result {
    iced::daemon(Requiescat::boot, Requiescat::update, Requiescat::view)
        .title(Requiescat::title)
        .subscription(Requiescat::subscription)
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
    StartMenu(StartMenuMessage),
    ImportPathChosen(Option<PathBuf>),
    ExportPathChosen(Option<PathBuf>),
    Editor(MapEditorMessage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainScreen {
    StartMenu,
    MapEditor,
}

struct Requiescat {
    editor: MapEditor,
    main_screen: MainScreen,
    main_window: Option<window::Id>,
    person_directory_window: Option<window::Id>,
    person_detail_windows: Vec<(window::Id, crate::models::PersonId)>,
    new_person_window: Option<window::Id>,
    library: Option<CemeteryLibrary>,
    cemeteries: Vec<CemeteryFile>,
    selected_cemetery: Option<PathBuf>,
    active_database: Option<PathBuf>,
    show_cemeteries: bool,
    show_create_cemetery: bool,
    new_cemetery_name: String,
    status: Option<String>,
}

impl Requiescat {
    fn boot() -> (Self, Task<Message>) {
        let (library, cemeteries, status) = match CemeteryLibrary::for_current_user() {
            Ok(library) => {
                let result = library.cemeteries();
                match result {
                    Ok(cemeteries) => (Some(library), cemeteries, None),
                    Err(error) => (Some(library), Vec::new(), Some(error.to_string())),
                }
            }
            Err(error) => (None, Vec::new(), Some(error.to_string())),
        };

        let (window_id, open) = window::open(window::Settings {
            size: Size::new(760.0, 520.0),
            min_size: Some(Size::new(620.0, 420.0)),
            ..Default::default()
        });

        (
            Self {
                editor: MapEditor::default(),
                main_screen: MainScreen::StartMenu,
                main_window: Some(window_id),
                person_directory_window: None,
                person_detail_windows: Vec::new(),
                new_person_window: None,
                library,
                cemeteries,
                selected_cemetery: None,
                active_database: None,
                show_cemeteries: false,
                show_create_cemetery: false,
                new_cemetery_name: String::new(),
                status,
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
                if self.main_screen != MainScreen::MapEditor {
                    return Task::none();
                }

                if is_command_shortcut(&event, 'n') {
                    return self.open_new_person_dialog();
                }

                if is_command_shortcut(&event, 'p') {
                    return self.open_person_directory();
                }
            }
            Message::StartMenu(message) => {
                return self.update_start_menu(message);
            }
            Message::ImportPathChosen(path) => {
                if let Some(path) = path {
                    self.import_cemetery(path);
                }
            }
            Message::ExportPathChosen(path) => {
                if let Some(destination) = path {
                    self.export_selected_cemetery(destination);
                }
            }
            Message::Editor(MapEditorMessage::OpenPersonDetails(person_id)) => {
                return self.open_person_details(person_id);
            }
            Message::Editor(MapEditorMessage::SubmitNewPerson) => {
                if self.editor.submit_new_person() {
                    self.save_active_cemetery();
                    if let Some(id) = self.new_person_window.take() {
                        return window::close(id);
                    }
                }
            }
            Message::Editor(message) => {
                self.editor.update(message);
                self.save_active_cemetery();
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
            match self.main_screen {
                MainScreen::StartMenu => start_menu_view(
                    &self.cemeteries,
                    self.selected_cemetery.as_deref(),
                    self.show_cemeteries,
                    self.show_create_cemetery,
                    &self.new_cemetery_name,
                    self.status.as_deref(),
                )
                .map(Message::StartMenu),
                MainScreen::MapEditor => self.editor.view().map(Message::Editor),
            }
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
        } else if self.main_screen == MainScreen::MapEditor {
            self.active_database
                .as_deref()
                .and_then(|path| path.file_stem())
                .and_then(|name| name.to_str())
                .map(|name| format!("{name} - Requiescat"))
                .unwrap_or_else(|| "Requiescat".to_owned())
        } else {
            "Requiescat - Cemetery Library".to_owned()
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

    fn update_start_menu(&mut self, message: StartMenuMessage) -> Task<Message> {
        match message {
            StartMenuMessage::ShowCemeteries => {
                self.show_cemeteries = true;
                self.show_create_cemetery = false;
            }
            StartMenuMessage::Back => {
                self.show_cemeteries = false;
                self.show_create_cemetery = false;
                self.new_cemetery_name.clear();
                self.status = None;
            }
            StartMenuMessage::OpenCemetery(path) => {
                self.selected_cemetery = Some(path);
                self.status = None;
                return self.load_selected_cemetery();
            }
            StartMenuMessage::ShowCreateCemetery => {
                self.show_cemeteries = false;
                self.show_create_cemetery = true;
                self.new_cemetery_name.clear();
                self.status = None;
            }
            StartMenuMessage::CemeteryNameChanged(name) => {
                self.new_cemetery_name = name;
                self.status = None;
            }
            StartMenuMessage::SubmitCreateCemetery => {
                if !self.new_cemetery_name.trim().is_empty() {
                    return self.create_cemetery();
                }
            }
            StartMenuMessage::ImportCemetery => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("SQLite cemetery", &["sqlite", "sqlite3", "db"])
                            .pick_file()
                            .await
                            .map(|file| file.path().to_owned())
                    },
                    Message::ImportPathChosen,
                );
            }
            StartMenuMessage::ExportSelected => {
                let file_name = self
                    .selected_cemetery
                    .as_deref()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("Cemetery.sqlite")
                    .to_owned();

                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter("SQLite cemetery", &["sqlite"])
                            .set_file_name(&file_name)
                            .save_file()
                            .await
                            .map(|file| file.path().to_owned())
                    },
                    Message::ExportPathChosen,
                );
            }
        }

        Task::none()
    }

    fn load_selected_cemetery(&mut self) -> Task<Message> {
        let Some(path) = self.selected_cemetery.clone() else {
            return Task::none();
        };

        match SqliteCemeteryRepository::new(path.clone()).load() {
            Ok(cemetery) => {
                self.editor = MapEditor::from_cemetery(cemetery);
                self.active_database = Some(path);
                self.main_screen = MainScreen::MapEditor;
                self.status = None;

                self.main_window
                    .map(|id| window::resize(id, Size::new(1100.0, 760.0)))
                    .unwrap_or_else(Task::none)
            }
            Err(error) => {
                self.status = Some(format!("Could not load cemetery: {error}"));
                Task::none()
            }
        }
    }

    fn import_cemetery(&mut self, source: PathBuf) {
        let Some(library) = &self.library else {
            self.status = Some("The cemetery library is unavailable.".to_owned());
            return;
        };

        match library.import(&source) {
            Ok(imported) => {
                self.selected_cemetery = Some(imported);
                self.show_cemeteries = true;
                self.refresh_cemeteries();
                self.status = Some("Cemetery imported.".to_owned());
            }
            Err(error) => {
                self.status = Some(format!("Could not import cemetery: {error}"));
            }
        }
    }

    fn create_cemetery(&mut self) -> Task<Message> {
        let Some(library) = &self.library else {
            self.status = Some("The cemetery library is unavailable.".to_owned());
            return Task::none();
        };

        match library.create(&self.new_cemetery_name) {
            Ok(path) => {
                self.selected_cemetery = Some(path);
                self.show_create_cemetery = false;
                self.new_cemetery_name.clear();
                self.refresh_cemeteries();
                self.load_selected_cemetery()
            }
            Err(error) => {
                self.status = Some(format!("Could not create cemetery: {error}"));
                Task::none()
            }
        }
    }

    fn export_selected_cemetery(&mut self, destination: PathBuf) {
        let (Some(library), Some(source)) = (&self.library, &self.selected_cemetery) else {
            return;
        };

        self.status = match library.export(source, &destination) {
            Ok(()) => Some("Cemetery exported.".to_owned()),
            Err(error) => Some(format!("Could not export cemetery: {error}")),
        };
    }

    fn refresh_cemeteries(&mut self) {
        let Some(library) = &self.library else {
            return;
        };

        match library.cemeteries() {
            Ok(cemeteries) => self.cemeteries = cemeteries,
            Err(error) => self.status = Some(format!("Could not refresh cemeteries: {error}")),
        }
    }

    fn save_active_cemetery(&mut self) {
        let Some(path) = self.active_database.clone() else {
            return;
        };

        let mut repository = SqliteCemeteryRepository::new(path);
        if let Err(error) = repository.save(self.editor.cemetery()) {
            self.status = Some(format!("Could not save cemetery: {error}"));
        }
    }
}

fn is_command_shortcut(event: &keyboard::Event, character: char) -> bool {
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

    !repeat && modifiers.command() && key.to_latin(*physical_key) == Some(character)
}

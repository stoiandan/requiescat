#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod localization;
mod models;
mod persistence;
mod screens;
mod updater;

use std::path::PathBuf;

use iced::widget::{Space, button, column, container, pick_list, row, text};
use iced::{
    Background, Border, Color, Element, Length, Shadow, Size, Subscription, Task, Theme, Vector,
    keyboard, window,
};
use localization::{Language, Localizer, MessageId};
use persistence::{CemeteryFile, CemeteryLibrary, CemeteryRepository, SqliteCemeteryRepository};
use screens::{
    MapEditor, MapEditorMessage, MapEditorUpdateOutcome, StartMenuMessage, StartMenuViewState,
    UpdateStatusView, start_menu_view,
};
use updater::{AvailableUpdate, StagedUpdate};

fn main() -> iced::Result {
    if let Some(result) = updater::run_installer_mode() {
        if let Err(error) = result {
            updater::record_installer_failure(&error);
            eprintln!("Update installation failed: {error}");
            std::process::exit(1);
        }
        return Ok(());
    }

    updater::remove_stale_helper();

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
    LanguageSelected(Language),
    NewPerson,
    OpenPersonDirectory,
    ExportActiveCemetery,
    StartMenu(StartMenuMessage),
    ImportPathChosen(Option<PathBuf>),
    ExportPathChosen(Option<PathBuf>),
    Editor(MapEditorMessage),
    UpdateCheckFinished(Result<Option<AvailableUpdate>, String>),
    UpdateDownloaded(Result<StagedUpdate, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainScreen {
    StartMenu,
    MapEditor,
}

#[derive(Debug, Clone, Default)]
enum SaveState {
    #[default]
    Clean,
    Dirty,
    Failed(String),
}

#[derive(Debug, Clone, Default)]
enum UpdateState {
    #[default]
    Checking,
    UpToDate,
    Available(AvailableUpdate),
    Downloading(AvailableUpdate),
    Ready(StagedUpdate),
    Failed(String),
}

impl UpdateState {
    fn view(&self, language: Language) -> UpdateStatusView<'_> {
        let language_code = language.code();
        match self {
            Self::Checking => UpdateStatusView::Checking,
            Self::UpToDate => UpdateStatusView::UpToDate,
            Self::Available(update) => UpdateStatusView::Available {
                version: &update.version,
                description: update.description(language_code),
            },
            Self::Downloading(update) => UpdateStatusView::Downloading {
                version: &update.version,
                description: update.description(language_code),
            },
            Self::Ready(update) => UpdateStatusView::Ready {
                version: &update.version,
                description: update.description(language_code),
            },
            Self::Failed(error) => UpdateStatusView::Failed(error),
        }
    }
}

impl SaveState {
    fn label(&self, localizer: &Localizer) -> Option<String> {
        match self {
            Self::Clean => None,
            Self::Dirty => Some(localizer.text(MessageId::UnsavedChanges)),
            Self::Failed(error) => {
                Some(localizer.value(MessageId::SaveFailed, "error", error.as_str()))
            }
        }
    }
}

#[derive(Debug, Clone)]
enum AppStatus {
    RawError(String),
    LibraryUnavailable,
    CemeteryImported,
    CouldNotLoadCemetery(String),
    CouldNotImportCemetery(String),
    CouldNotCreateCemetery(String),
    ExportSaveFailed,
    CemeteryExported,
    CouldNotExportCemetery(String),
    CouldNotRefreshCemeteries(String),
}

impl AppStatus {
    fn localized(&self, localizer: &Localizer) -> String {
        match self {
            Self::RawError(error) => error.clone(),
            Self::LibraryUnavailable => localizer.text(MessageId::LibraryUnavailable),
            Self::CemeteryImported => localizer.text(MessageId::CemeteryImported),
            Self::CouldNotLoadCemetery(error) => {
                localizer.value(MessageId::CouldNotLoadCemetery, "error", error.as_str())
            }
            Self::CouldNotImportCemetery(error) => {
                localizer.value(MessageId::CouldNotImportCemetery, "error", error.as_str())
            }
            Self::CouldNotCreateCemetery(error) => {
                localizer.value(MessageId::CouldNotCreateCemetery, "error", error.as_str())
            }
            Self::ExportSaveFailed => localizer.text(MessageId::ExportSaveFailed),
            Self::CemeteryExported => localizer.text(MessageId::CemeteryExported),
            Self::CouldNotExportCemetery(error) => {
                localizer.value(MessageId::CouldNotExportCemetery, "error", error.as_str())
            }
            Self::CouldNotRefreshCemeteries(error) => localizer.value(
                MessageId::CouldNotRefreshCemeteries,
                "error",
                error.as_str(),
            ),
        }
    }
}

struct Requiescat {
    localizer: Localizer,
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
    status: Option<AppStatus>,
    save_state: SaveState,
    update_state: UpdateState,
}

impl Requiescat {
    fn boot() -> (Self, Task<Message>) {
        let (library, cemeteries, status) = match CemeteryLibrary::for_current_user() {
            Ok(library) => {
                let result = library.cemeteries();
                match result {
                    Ok(cemeteries) => (Some(library), cemeteries, None),
                    Err(error) => (
                        Some(library),
                        Vec::new(),
                        Some(AppStatus::RawError(error.to_string())),
                    ),
                }
            }
            Err(error) => (
                None,
                Vec::new(),
                Some(AppStatus::RawError(error.to_string())),
            ),
        };

        let (window_id, open) = window::open(window::Settings {
            size: Size::new(760.0, 520.0),
            min_size: Some(Size::new(620.0, 420.0)),
            ..Default::default()
        });

        (
            Self {
                localizer: Localizer::default(),
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
                save_state: SaveState::Clean,
                update_state: UpdateState::Checking,
            },
            Task::batch([
                open.map(Message::MainWindowOpened),
                check_for_updates_task(),
            ]),
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
                    if matches!(self.save_state, SaveState::Dirty | SaveState::Failed(_)) {
                        self.save_active_cemetery();
                    }
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
            Message::LanguageSelected(language) => {
                self.localizer.set_language(language);
            }
            Message::NewPerson => {
                if self.main_screen == MainScreen::MapEditor {
                    return self.open_new_person_dialog();
                }
            }
            Message::OpenPersonDirectory => {
                if self.main_screen == MainScreen::MapEditor {
                    return self.open_person_directory();
                }
            }
            Message::ExportActiveCemetery => {
                if self.main_screen == MainScreen::MapEditor {
                    return self.choose_export_path();
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
                    self.save_state = SaveState::Dirty;
                    self.save_active_cemetery();
                    if let Some(id) = self.new_person_window.take() {
                        return window::close(id);
                    }
                }
            }
            Message::Editor(message) => match self.editor.update(message) {
                MapEditorUpdateOutcome::Unchanged => {}
                MapEditorUpdateOutcome::Changed => {
                    self.save_state = SaveState::Dirty;
                    self.save_active_cemetery();
                }
                MapEditorUpdateOutcome::DeferredChange => {
                    self.save_state = SaveState::Dirty;
                }
                MapEditorUpdateOutcome::Commit => {
                    if matches!(self.save_state, SaveState::Dirty | SaveState::Failed(_)) {
                        self.save_active_cemetery();
                    }
                }
            },
            Message::UpdateCheckFinished(result) => {
                self.update_state = match result {
                    Ok(Some(update)) => UpdateState::Available(update),
                    Ok(None) => UpdateState::UpToDate,
                    Err(error) => UpdateState::Failed(error),
                };
            }
            Message::UpdateDownloaded(result) => {
                self.update_state = match result {
                    Ok(update) => UpdateState::Ready(update),
                    Err(error) => UpdateState::Failed(error),
                };
            }
        }

        Task::none()
    }

    fn view(&self, window: window::Id) -> Element<'_, Message> {
        let content = if Some(window) == self.person_directory_window {
            self.editor
                .person_directory_view(&self.localizer)
                .map(Message::Editor)
        } else if Some(window) == self.new_person_window {
            self.editor
                .new_person_view(&self.localizer)
                .map(Message::Editor)
        } else if let Some((_, person_id)) = self
            .person_detail_windows
            .iter()
            .find(|(window_id, _)| *window_id == window)
        {
            self.editor
                .person_details_view(&self.localizer, *person_id)
                .map(Message::Editor)
        } else if Some(window) == self.main_window {
            let status = self
                .status
                .as_ref()
                .map(|status| status.localized(&self.localizer));

            match self.main_screen {
                MainScreen::StartMenu => start_menu_view(
                    &self.localizer,
                    StartMenuViewState {
                        cemeteries: &self.cemeteries,
                        selected: self.selected_cemetery.as_deref(),
                        show_cemeteries: self.show_cemeteries,
                        show_create_cemetery: self.show_create_cemetery,
                        new_cemetery_name: &self.new_cemetery_name,
                        status,
                        update_status: self.update_state.view(self.localizer.language()),
                    },
                )
                .map(Message::StartMenu),
                MainScreen::MapEditor => self
                    .editor
                    .view(&self.localizer, self.save_state.label(&self.localizer))
                    .map(Message::Editor),
            }
        } else {
            container(text(self.localizer.text(MessageId::UnknownWindow)))
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill)
                .into()
        };

        self.with_global_menu(content, Some(window) == self.main_window)
    }

    fn title(&self, window: window::Id) -> String {
        if Some(window) == self.person_directory_window {
            self.localizer.text(MessageId::PersonDirectoryTitle)
        } else if Some(window) == self.new_person_window {
            self.localizer.text(MessageId::NewPersonTitle)
        } else if self
            .person_detail_windows
            .iter()
            .any(|(window_id, _)| *window_id == window)
        {
            self.localizer.text(MessageId::PersonDetailsTitle)
        } else if self.main_screen == MainScreen::MapEditor {
            self.active_database
                .as_deref()
                .and_then(|path| path.file_stem())
                .and_then(|name| name.to_str())
                .map(|name| format!("{name} - Requiescat"))
                .unwrap_or_else(|| "Requiescat".to_owned())
        } else {
            self.localizer.text(MessageId::CemeteryLibraryTitle)
        }
    }

    fn with_global_menu<'a>(
        &'a self,
        content: Element<'a, Message>,
        is_main_window: bool,
    ) -> Element<'a, Message> {
        let language_menu = row![
            text(self.localizer.text(MessageId::LanguageMenu)).size(12),
            pick_list(
                Language::ALL,
                Some(self.localizer.language()),
                Message::LanguageSelected,
            )
            .text_size(12)
            .padding([5, 8]),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let mut menu_bar = row![].spacing(16).align_y(iced::Alignment::Center);

        if is_main_window && self.main_screen == MainScreen::MapEditor {
            menu_bar = menu_bar.push(self.app_menu_group(
                self.localizer.text(MessageId::AppMenuFile),
                [
                    (
                        self.localizer.text(MessageId::AppMenuNewPerson),
                        Message::NewPerson,
                    ),
                    (
                        self.localizer.text(MessageId::AppMenuExportDb),
                        Message::ExportActiveCemetery,
                    ),
                ],
            ));
            menu_bar = menu_bar.push(self.app_menu_group(
                self.localizer.text(MessageId::AppMenuView),
                [(
                    self.localizer.text(MessageId::AppMenuPersonDirectory),
                    Message::OpenPersonDirectory,
                )],
            ));
        }

        menu_bar = menu_bar
            .push(Space::new().width(Length::Fill))
            .push(container(language_menu).align_x(iced::Alignment::End));

        column![
            container(menu_bar)
                .width(Length::Fill)
                .padding([8, 12])
                .style(app_menu_bar),
            container(content).width(Length::Fill).height(Length::Fill),
        ]
        .into()
    }

    fn app_menu_group<'a, const N: usize>(
        &'a self,
        label: String,
        actions: [(String, Message); N],
    ) -> Element<'a, Message> {
        let mut group = row![
            container(text(label).size(12).style(|_| text::Style {
                color: Some(Color::from_rgb8(171, 215, 213)),
            }))
            .padding([0, 4])
            .align_y(iced::Alignment::Center)
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        for (label, message) in actions {
            group = group.push(
                button(text(label).size(12))
                    .padding([5, 10])
                    .style(app_menu_button)
                    .on_press(message),
            );
        }

        container(group)
            .padding([3, 4])
            .style(app_menu_group)
            .into()
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
                let filter = self.localizer.text(MessageId::FileFilterSqliteCemetery);
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter(&filter, &["sqlite", "sqlite3", "db"])
                            .pick_file()
                            .await
                            .map(|file| file.path().to_owned())
                    },
                    Message::ImportPathChosen,
                );
            }
            StartMenuMessage::ExportSelected => {
                return self.choose_export_path();
            }
            StartMenuMessage::CheckForUpdates => {
                self.update_state = UpdateState::Checking;
                return check_for_updates_task();
            }
            StartMenuMessage::DownloadUpdate => {
                let UpdateState::Available(update) = &self.update_state else {
                    return Task::none();
                };
                let update = update.clone();
                self.update_state = UpdateState::Downloading(update.clone());
                return Task::perform(updater::download_update(update), |result| {
                    Message::UpdateDownloaded(result.map_err(|error| error.to_string()))
                });
            }
            StartMenuMessage::InstallUpdate => {
                let UpdateState::Ready(update) = &self.update_state else {
                    return Task::none();
                };
                match updater::install_and_restart(update) {
                    Ok(()) => return iced::exit(),
                    Err(error) => self.update_state = UpdateState::Failed(error.to_string()),
                }
            }
            StartMenuMessage::OpenReleaseNotes => {
                let notes_url = match &self.update_state {
                    UpdateState::Available(update) | UpdateState::Downloading(update) => {
                        Some(update.notes_url.as_str())
                    }
                    UpdateState::Ready(update) => Some(update.notes_url.as_str()),
                    _ => None,
                };
                if let Some(notes_url) = notes_url
                    && let Err(error) = updater::open_release_notes(notes_url)
                {
                    self.update_state = UpdateState::Failed(error.to_string());
                }
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
                self.save_state = SaveState::Clean;

                self.main_window
                    .map(|id| window::resize(id, Size::new(1100.0, 760.0)))
                    .unwrap_or_else(Task::none)
            }
            Err(error) => {
                self.status = Some(AppStatus::CouldNotLoadCemetery(error.to_string()));
                Task::none()
            }
        }
    }

    fn import_cemetery(&mut self, source: PathBuf) {
        let Some(library) = &self.library else {
            self.status = Some(AppStatus::LibraryUnavailable);
            return;
        };

        match library.import(&source) {
            Ok(imported) => {
                self.selected_cemetery = Some(imported);
                self.show_cemeteries = true;
                self.refresh_cemeteries();
                self.status = Some(AppStatus::CemeteryImported);
            }
            Err(error) => {
                self.status = Some(AppStatus::CouldNotImportCemetery(error.to_string()));
            }
        }
    }

    fn create_cemetery(&mut self) -> Task<Message> {
        let Some(library) = &self.library else {
            self.status = Some(AppStatus::LibraryUnavailable);
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
                self.status = Some(AppStatus::CouldNotCreateCemetery(error.to_string()));
                Task::none()
            }
        }
    }

    fn export_selected_cemetery(&mut self, destination: PathBuf) {
        if matches!(self.save_state, SaveState::Dirty | SaveState::Failed(_))
            && !self.save_active_cemetery()
        {
            self.status = Some(AppStatus::ExportSaveFailed);
            return;
        }

        let (Some(library), Some(source)) = (&self.library, &self.selected_cemetery) else {
            return;
        };

        self.status = match library.export(source, &destination) {
            Ok(()) => Some(AppStatus::CemeteryExported),
            Err(error) => Some(AppStatus::CouldNotExportCemetery(error.to_string())),
        };
    }

    fn choose_export_path(&self) -> Task<Message> {
        let file_name = self
            .selected_cemetery
            .as_deref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Cemetery.sqlite")
            .to_owned();

        let filter = self.localizer.text(MessageId::FileFilterSqliteCemetery);
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .add_filter(&filter, &["sqlite"])
                    .set_file_name(&file_name)
                    .save_file()
                    .await
                    .map(|file| file.path().to_owned())
            },
            Message::ExportPathChosen,
        )
    }

    fn refresh_cemeteries(&mut self) {
        let Some(library) = &self.library else {
            return;
        };

        match library.cemeteries() {
            Ok(cemeteries) => self.cemeteries = cemeteries,
            Err(error) => {
                self.status = Some(AppStatus::CouldNotRefreshCemeteries(error.to_string()))
            }
        }
    }

    fn save_active_cemetery(&mut self) -> bool {
        let Some(path) = self.active_database.clone() else {
            return false;
        };

        let mut repository = SqliteCemeteryRepository::new(path);
        match repository.save(self.editor.cemetery()) {
            Ok(()) => {
                self.save_state = SaveState::Clean;
                true
            }
            Err(error) => {
                self.save_state = SaveState::Failed(error.to_string());
                false
            }
        }
    }
}

fn check_for_updates_task() -> Task<Message> {
    Task::perform(updater::check_for_update(), |result| {
        Message::UpdateCheckFinished(result.map_err(|error| error.to_string()))
    })
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

fn app_menu_bar(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(8, 31, 34))),
        border: Border {
            color: Color::from_rgb8(38, 118, 121),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn app_menu_group(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb8(13, 48, 52))),
        border: Border {
            color: Color::from_rgb8(39, 126, 130),
            width: 1.0,
            radius: 7.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.25),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 3.0,
        },
        ..Default::default()
    }
}

fn app_menu_button(_: &Theme, status: button::Status) -> button::Style {
    let pressed = status == button::Status::Pressed;
    let hovered = status == button::Status::Hovered;

    button::Style {
        background: Some(Background::Color(if pressed {
            Color::from_rgb8(34, 125, 128)
        } else if hovered {
            Color::from_rgb8(42, 146, 150)
        } else {
            Color::from_rgb8(21, 78, 82)
        })),
        text_color: Color::WHITE,
        border: Border {
            color: Color::from_rgb8(67, 174, 177),
            width: 1.0,
            radius: 5.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.18),
            offset: if pressed {
                Vector::new(0.0, 0.5)
            } else {
                Vector::new(0.0, 1.0)
            },
            blur_radius: 2.0,
        },
        ..Default::default()
    }
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use iced::widget::{Space, button, column, container, opaque, pick_list, pin, row, stack, text};
use iced::{
    Background, Border, Color, Element, Length, Shadow, Size, Subscription, Task, Theme, Vector,
    keyboard, window,
};
use requiescat::export::pdf::{PdfExportOptions, export_cemetery_map};
use requiescat::localization::{Language, Localizer, MessageId};
use requiescat::models::{Cemetery, PersonId};
use requiescat::persistence::{
    CemeteryFile, CemeteryLibrary, PersistenceError, SqliteCemeteryRepository,
};
use requiescat::screens::{
    MapEditor, MapEditorMessage, MapEditorUpdateOutcome, StartMenuMessage, StartMenuViewState,
    start_menu_view,
};
use requiescat::theme;
use requiescat::windowing;

fn main() -> iced::Result {
    if std::env::args_os().any(|argument| argument == "--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    iced::daemon(Requiescat::boot, Requiescat::update, Requiescat::view)
        .title(Requiescat::title)
        .subscription(Requiescat::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    MainWindowOpened(window::Id),
    PersonDirectoryOpened(window::Id),
    GraveDirectoryOpened(window::Id),
    PersonDetailsOpened(window::Id, PersonId),
    NewPersonWindowOpened(window::Id),
    WindowClosed(window::Id),
    Keyboard(keyboard::Event),
    LanguageSelected(Language),
    ToggleAppMenu(AppMenu),
    NewPerson,
    OpenPersonDirectory,
    OpenGraveDirectory,
    DuplicateLastGrave,
    ExportActiveCemetery,
    ExportActiveCemeteryPdf,
    StartMenu(StartMenuMessage),
    ImportPathChosen(Option<PathBuf>),
    ExportPathChosen(Option<PathBuf>),
    PdfExportPathChosen(Option<PathBuf>),
    Editor(MapEditorMessage),
    SaveFinished {
        revision: u64,
        result: Result<(), String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainScreen {
    StartMenu,
    MapEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMenu {
    File,
    Edit,
    View,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowKind {
    Main,
    PersonDirectory,
    GraveDirectory,
    NewPerson,
    PersonDetails(PersonId),
    Unknown,
}

#[derive(Debug, Clone, Default)]
struct OpenWindows {
    main: Option<window::Id>,
    person_directory: Option<window::Id>,
    grave_directory: Option<window::Id>,
    person_details: Vec<(window::Id, PersonId)>,
    new_person: Option<window::Id>,
}

impl OpenWindows {
    fn with_main(main: window::Id) -> Self {
        Self {
            main: Some(main),
            ..Default::default()
        }
    }

    fn is_main(&self, id: window::Id) -> bool {
        self.main == Some(id)
    }

    fn kind(&self, id: window::Id) -> WindowKind {
        [
            (self.main, WindowKind::Main),
            (self.person_directory, WindowKind::PersonDirectory),
            (self.grave_directory, WindowKind::GraveDirectory),
            (self.new_person, WindowKind::NewPerson),
        ]
        .into_iter()
        .find_map(|(window, kind)| (window == Some(id)).then_some(kind))
        .or_else(|| {
            self.person_detail_for_window(id)
                .map(WindowKind::PersonDetails)
        })
        .unwrap_or(WindowKind::Unknown)
    }

    fn with_person_details(&self, id: window::Id, person_id: PersonId) -> Self {
        if self
            .person_details
            .iter()
            .any(|(window_id, _)| *window_id == id)
        {
            return self.clone();
        }

        Self {
            person_details: self
                .person_details
                .iter()
                .copied()
                .chain(std::iter::once((id, person_id)))
                .collect(),
            ..self.clone()
        }
    }

    fn person_detail_for_window(&self, id: window::Id) -> Option<PersonId> {
        self.person_details
            .iter()
            .find(|(window_id, _)| *window_id == id)
            .map(|(_, person_id)| *person_id)
    }

    fn window_for_person(&self, person_id: PersonId) -> Option<window::Id> {
        self.person_details
            .iter()
            .find(|(_, open_person_id)| *open_person_id == person_id)
            .map(|(window_id, _)| *window_id)
    }

    fn without_window(&self, id: window::Id) -> Self {
        Self {
            person_directory: self.person_directory.filter(|window| *window != id),
            grave_directory: self.grave_directory.filter(|window| *window != id),
            new_person: self.new_person.filter(|window| *window != id),
            person_details: self
                .person_details
                .iter()
                .copied()
                .filter(|(window_id, _)| *window_id != id)
                .collect(),
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, Default)]
enum SaveState {
    #[default]
    Clean,
    Dirty,
    Failed(String),
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

#[derive(Debug, Clone, Default)]
struct CemeterySession {
    active_database: Option<PathBuf>,
    save_state: SaveState,
    save_revision: u64,
    saved_revision: u64,
    save_in_flight: bool,
}

struct SaveRequest {
    path: PathBuf,
    cemetery: Cemetery,
    revision: u64,
}

impl CemeterySession {
    fn open(database: PathBuf) -> Self {
        Self {
            active_database: Some(database),
            ..Default::default()
        }
    }

    fn active_cemetery_title(&self) -> String {
        self.active_cemetery_name().unwrap_or("Cemetery").to_owned()
    }

    fn window_title(&self) -> String {
        self.active_cemetery_name()
            .map(|name| format!("Requiescat - {name}"))
            .unwrap_or_else(|| "Requiescat".to_owned())
    }

    fn active_cemetery_name(&self) -> Option<&str> {
        self.active_database
            .as_deref()
            .and_then(|path| path.file_stem())
            .and_then(|name| name.to_str())
    }

    fn has_unsaved_changes(&self) -> bool {
        matches!(self.save_state, SaveState::Dirty | SaveState::Failed(_))
    }

    fn dirty(&self) -> Self {
        Self {
            save_revision: self.save_revision.saturating_add(1),
            save_state: SaveState::Dirty,
            ..self.clone()
        }
    }

    fn save_now(&self, cemetery: &Cemetery) -> (Self, bool) {
        let Some(path) = self.active_database.clone() else {
            return (self.clone(), false);
        };

        let repository = SqliteCemeteryRepository::new(path);
        match repository.save(cemetery) {
            Ok(()) => (
                Self {
                    save_state: SaveState::Clean,
                    saved_revision: self.save_revision,
                    ..self.clone()
                },
                true,
            ),
            Err(error) => (
                Self {
                    save_state: SaveState::Failed(error.to_string()),
                    ..self.clone()
                },
                false,
            ),
        }
    }

    fn begin_background_save(&self, cemetery: &Cemetery) -> Option<(Self, SaveRequest)> {
        if self.save_in_flight || self.save_revision == self.saved_revision {
            return None;
        }

        let path = self.active_database.clone()?;
        let revision = self.save_revision;

        Some((
            Self {
                save_in_flight: true,
                ..self.clone()
            },
            SaveRequest {
                path,
                cemetery: cemetery.clone(),
                revision,
            },
        ))
    }

    fn finish_background_save(&self, revision: u64, result: Result<(), String>) -> (Self, bool) {
        let session = match result {
            Ok(()) if self.save_revision == revision => Self {
                save_in_flight: false,
                save_state: SaveState::Clean,
                saved_revision: self.saved_revision.max(revision),
                ..self.clone()
            },
            Ok(()) => Self {
                save_in_flight: false,
                saved_revision: self.saved_revision.max(revision),
                ..self.clone()
            },
            Err(error) => Self {
                save_in_flight: false,
                save_state: SaveState::Failed(error),
                ..self.clone()
            },
        };

        let needs_follow_up_save = session.save_revision > revision;
        (session, needs_follow_up_save)
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
    CemeteryDeleted,
    CouldNotDeleteCemetery(String),
    ExportSaveFailed,
    CemeteryExported,
    CouldNotExportCemetery(String),
    CemeteryPdfExported,
    CouldNotExportCemeteryPdf(String),
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
            Self::CemeteryDeleted => localizer.text(MessageId::CemeteryDeleted),
            Self::CouldNotDeleteCemetery(error) => {
                localizer.value(MessageId::CouldNotDeleteCemetery, "error", error.as_str())
            }
            Self::ExportSaveFailed => localizer.text(MessageId::ExportSaveFailed),
            Self::CemeteryExported => localizer.text(MessageId::CemeteryExported),
            Self::CouldNotExportCemetery(error) => {
                localizer.value(MessageId::CouldNotExportCemetery, "error", error.as_str())
            }
            Self::CemeteryPdfExported => localizer.text(MessageId::CemeteryPdfExported),
            Self::CouldNotExportCemeteryPdf(error) => localizer.value(
                MessageId::CouldNotExportCemeteryPdf,
                "error",
                error.as_str(),
            ),
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
    windows: OpenWindows,
    library: Option<CemeteryLibrary>,
    cemeteries: Vec<CemeteryFile>,
    selected_cemetery: Option<PathBuf>,
    session: CemeterySession,
    show_cemeteries: bool,
    show_create_cemetery: bool,
    new_cemetery_name: String,
    pending_delete_cemetery: Option<PathBuf>,
    status: Option<AppStatus>,
    app_menu: Option<AppMenu>,
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
            icon: windowing::application_icon(),
            size: Size::new(760.0, 520.0),
            min_size: Some(Size::new(620.0, 420.0)),
            ..Default::default()
        });

        (
            Self {
                localizer: Localizer::default(),
                editor: MapEditor::default(),
                main_screen: MainScreen::StartMenu,
                windows: OpenWindows::with_main(window_id),
                library,
                cemeteries,
                selected_cemetery: None,
                session: CemeterySession::default(),
                show_cemeteries: false,
                show_create_cemetery: false,
                new_cemetery_name: String::new(),
                pending_delete_cemetery: None,
                status,
                app_menu: None,
            },
            open.map(Message::MainWindowOpened),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainWindowOpened(id) => {
                self.windows.main = Some(id);
            }
            Message::PersonDirectoryOpened(id) => {
                self.windows.person_directory = Some(id);
            }
            Message::GraveDirectoryOpened(id) => {
                self.windows.grave_directory = Some(id);
            }
            Message::PersonDetailsOpened(id, person_id) => {
                self.windows = self.windows.with_person_details(id, person_id);
            }
            Message::NewPersonWindowOpened(id) => {
                self.windows.new_person = Some(id);
            }
            Message::WindowClosed(id) => {
                if self.windows.is_main(id) {
                    if self.session.has_unsaved_changes() {
                        self.save_active_cemetery();
                    }
                    return iced::exit();
                }

                if self.windows.grave_directory == Some(id) {
                    self.editor.clear_grave_search();
                }

                self.windows = self.windows.without_window(id);
            }
            Message::Keyboard(event) => {
                if !self.is_showing_map_editor() {
                    return Task::none();
                }

                if is_command_shortcut(&event, 'n') {
                    return self.open_new_person_dialog();
                }

                if is_command_shortcut(&event, 'p') {
                    return self.open_person_directory();
                }

                if is_command_shortcut(&event, 'g') {
                    return self.open_grave_directory();
                }

                if is_command_shortcut(&event, 'd') {
                    return self.update(Message::Editor(
                        MapEditorMessage::DuplicateLastGraveAtCursor,
                    ));
                }
            }
            Message::LanguageSelected(language) => {
                self.close_app_menu();
                self.localizer.set_language(language);
            }
            Message::ToggleAppMenu(menu) => {
                if self.is_showing_map_editor() {
                    self.app_menu = if self.app_menu == Some(menu) {
                        None
                    } else {
                        Some(menu)
                    };
                }
            }
            Message::NewPerson => {
                return self.run_map_editor_menu_action(Self::open_new_person_dialog);
            }
            Message::OpenPersonDirectory => {
                return self.run_map_editor_menu_action(Self::open_person_directory);
            }
            Message::OpenGraveDirectory => {
                return self.run_map_editor_menu_action(Self::open_grave_directory);
            }
            Message::DuplicateLastGrave => {
                return self.run_map_editor_menu_action(|app| {
                    app.update(Message::Editor(
                        MapEditorMessage::DuplicateLastGraveAtCursor,
                    ))
                });
            }
            Message::ExportActiveCemetery => {
                return self
                    .run_map_editor_menu_action(|app| app.prompt_for_database_export_path());
            }
            Message::ExportActiveCemeteryPdf => {
                return self.run_map_editor_menu_action(|app| app.prompt_for_pdf_export_path());
            }
            Message::StartMenu(message) => {
                self.close_app_menu();
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
            Message::PdfExportPathChosen(path) => {
                if let Some(destination) = path {
                    self.export_selected_cemetery_pdf(destination);
                }
            }
            Message::SaveFinished { revision, result } => {
                let (session, needs_follow_up_save) =
                    self.session.finish_background_save(revision, result);
                self.session = session;

                if needs_follow_up_save {
                    return self.save_active_cemetery_in_background();
                }
            }
            Message::Editor(MapEditorMessage::OpenPersonDetails(person_id)) => {
                return self.open_person_details(person_id);
            }
            Message::Editor(MapEditorMessage::SubmitNewPerson) => {
                if self.editor.submit_new_person() {
                    let save = self.mark_dirty_and_autosave();
                    if let Some(id) = self.windows.new_person.take() {
                        return Task::batch([save, window::close(id)]);
                    }
                    return save;
                }
            }
            Message::Editor(message) => match self.editor.update(message) {
                MapEditorUpdateOutcome::Unchanged => {}
                MapEditorUpdateOutcome::Changed => {
                    return self.mark_dirty_and_autosave();
                }
                MapEditorUpdateOutcome::DeferredChange => {
                    self.session = self.session.dirty();
                }
                MapEditorUpdateOutcome::Commit => {
                    return self.save_active_cemetery_in_background();
                }
            },
        }

        Task::none()
    }

    fn view(&self, window: window::Id) -> Element<'_, Message> {
        let kind = self.windows.kind(window);
        let content = match kind {
            WindowKind::Main => self.main_window_view(),
            WindowKind::PersonDirectory => self
                .editor
                .person_directory_view(&self.localizer)
                .map(Message::Editor),
            WindowKind::GraveDirectory => self
                .editor
                .grave_directory_view(&self.localizer)
                .map(Message::Editor),
            WindowKind::NewPerson => self
                .editor
                .new_person_view(&self.localizer)
                .map(Message::Editor),
            WindowKind::PersonDetails(person_id) => self
                .editor
                .person_details_view(&self.localizer, person_id)
                .map(Message::Editor),
            WindowKind::Unknown => self.unknown_window_view(),
        };

        self.with_global_menu(content, kind == WindowKind::Main)
    }

    fn title(&self, window: window::Id) -> String {
        match self.windows.kind(window) {
            WindowKind::PersonDirectory => self.localizer.text(MessageId::PersonDirectoryTitle),
            WindowKind::GraveDirectory => self.localizer.text(MessageId::GraveDirectoryTitle),
            WindowKind::NewPerson => self.localizer.text(MessageId::NewPersonTitle),
            WindowKind::PersonDetails(_) => self.localizer.text(MessageId::PersonDetailsTitle),
            WindowKind::Main if self.main_screen == MainScreen::MapEditor => {
                self.session.window_title()
            }
            WindowKind::Main | WindowKind::Unknown => {
                self.localizer.text(MessageId::CemeteryLibraryTitle)
            }
        }
    }

    fn main_window_view(&self) -> Element<'_, Message> {
        match self.main_screen {
            MainScreen::StartMenu => self.start_menu_view(),
            MainScreen::MapEditor => self
                .editor
                .view(
                    &self.localizer,
                    self.session.save_state.label(&self.localizer),
                )
                .map(Message::Editor),
        }
    }

    fn start_menu_view(&self) -> Element<'_, Message> {
        let status = self
            .status
            .as_ref()
            .map(|status| status.localized(&self.localizer));
        let pending_delete = self.pending_delete_cemetery.as_deref().and_then(|path| {
            self.cemeteries
                .iter()
                .find(|cemetery| cemetery.path() == path)
        });

        start_menu_view(
            &self.localizer,
            StartMenuViewState {
                cemeteries: &self.cemeteries,
                selected: self.selected_cemetery.as_deref(),
                show_cemeteries: self.show_cemeteries,
                show_create_cemetery: self.show_create_cemetery,
                new_cemetery_name: &self.new_cemetery_name,
                pending_delete,
                status,
            },
        )
        .map(Message::StartMenu)
    }

    fn unknown_window_view(&self) -> Element<'_, Message> {
        container(text(self.localizer.text(MessageId::UnknownWindow)))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into()
    }

    fn build_language_menu(&self, is_main_window: bool) -> Element<'_, Message> {
        if !is_main_window {
            return row![].into();
        }

        row![
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
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn with_global_menu<'a>(
        &'a self,
        content: Element<'a, Message>,
        is_main_window: bool,
    ) -> Element<'a, Message> {
        let language_menu = self.build_language_menu(is_main_window);

        let mut menu_bar = row![].spacing(6).align_y(iced::Alignment::Center);
        let show_app_menu = is_main_window && self.main_screen == MainScreen::MapEditor;

        if show_app_menu {
            menu_bar = menu_bar.push(
                self.app_menu_title(self.localizer.text(MessageId::AppMenuFile), AppMenu::File),
            );
            menu_bar = menu_bar.push(
                self.app_menu_title(self.localizer.text(MessageId::AppMenuEdit), AppMenu::Edit),
            );
            menu_bar = menu_bar.push(
                self.app_menu_title(self.localizer.text(MessageId::AppMenuView), AppMenu::View),
            );
        }

        menu_bar = menu_bar
            .push(Space::new().width(Length::Fill))
            .push(container(language_menu).align_x(iced::Alignment::End));

        let base = column![
            container(menu_bar)
                .width(Length::Fill)
                .padding([4, 10])
                .style(app_menu_bar),
            container(content).width(Length::Fill).height(Length::Fill),
        ];

        if show_app_menu && let Some(menu) = self.app_menu {
            let dropdown_x = match menu {
                AppMenu::File => 10.0,
                AppMenu::Edit => 56.0,
                AppMenu::View => 102.0,
            };

            return stack![base]
                .width(Length::Fill)
                .height(Length::Fill)
                .push(
                    pin(opaque(self.app_menu_dropdown(menu)))
                        .x(dropdown_x)
                        .y(31.0),
                )
                .into();
        }

        base.into()
    }

    fn app_menu_title<'a>(&'a self, label: String, menu: AppMenu) -> Element<'a, Message> {
        button(text(label).size(13))
            .padding([3, 9])
            .style(move |theme, status| {
                app_menu_title_button(theme, status, self.app_menu == Some(menu))
            })
            .on_press(Message::ToggleAppMenu(menu))
            .into()
    }

    fn app_menu_dropdown<'a>(&'a self, menu: AppMenu) -> Element<'a, Message> {
        const FILE_ACTIONS: &[(MessageId, Message, bool)] = &[
            (MessageId::AppMenuNewPerson, Message::NewPerson, true),
            (
                MessageId::AppMenuExportDb,
                Message::ExportActiveCemetery,
                true,
            ),
            (
                MessageId::AppMenuExportPdf,
                Message::ExportActiveCemeteryPdf,
                true,
            ),
        ];
        let edit_actions: &[(MessageId, Message, bool)] = &[(
            MessageId::AppMenuDuplicateLastGrave,
            Message::DuplicateLastGrave,
            self.editor.can_duplicate_last_grave(),
        )];
        const VIEW_ACTIONS: &[(MessageId, Message, bool)] = &[
            (
                MessageId::AppMenuPersonDirectory,
                Message::OpenPersonDirectory,
                true,
            ),
            (
                MessageId::AppMenuGraveDirectory,
                Message::OpenGraveDirectory,
                true,
            ),
        ];

        let mut items = column![]
            .spacing(1)
            .padding([5, 0])
            .width(Length::Fixed(190.0));

        let actions = match menu {
            AppMenu::File => FILE_ACTIONS,
            AppMenu::Edit => edit_actions,
            AppMenu::View => VIEW_ACTIONS,
        };

        for (label, message, enabled) in actions.iter().cloned() {
            let item = button(text(self.localizer.text(label)).size(13))
                .width(Length::Fill)
                .padding([6, 12])
                .style(app_menu_item_button);
            let item = if enabled {
                item.on_press(message)
            } else {
                item
            };

            items = items.push(item);
        }

        container(items)
            .padding([2, 0])
            .style(app_menu_dropdown)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            window::close_events().map(Message::WindowClosed),
            keyboard::listen().map(Message::Keyboard),
        ])
    }

    fn is_showing_map_editor(&self) -> bool {
        self.main_screen == MainScreen::MapEditor
    }

    fn close_app_menu(&mut self) {
        self.app_menu = None;
    }

    fn run_map_editor_menu_action(
        &mut self,
        action: impl FnOnce(&mut Self) -> Task<Message>,
    ) -> Task<Message> {
        if !self.is_showing_map_editor() {
            return Task::none();
        }

        self.close_app_menu();
        action(self)
    }

    fn open_person_directory(&mut self) -> Task<Message> {
        if let Some(id) = self.windows.person_directory {
            return window::gain_focus(id);
        }

        let (id, open) = window::open(window::Settings {
            icon: windowing::application_icon(),
            size: Size::new(460.0, 700.0),
            min_size: Some(Size::new(360.0, 420.0)),
            ..Default::default()
        });

        self.windows.person_directory = Some(id);

        open.map(Message::PersonDirectoryOpened)
    }

    fn open_grave_directory(&mut self) -> Task<Message> {
        if let Some(id) = self.windows.grave_directory {
            return window::gain_focus(id);
        }

        let (id, open) = window::open(window::Settings {
            icon: windowing::application_icon(),
            size: Size::new(460.0, 700.0),
            min_size: Some(Size::new(360.0, 420.0)),
            ..Default::default()
        });

        self.windows.grave_directory = Some(id);

        open.map(Message::GraveDirectoryOpened)
    }

    fn open_person_details(&mut self, person_id: PersonId) -> Task<Message> {
        if let Some(id) = self.windows.window_for_person(person_id) {
            return window::gain_focus(id);
        }

        let (id, open) = window::open(window::Settings {
            icon: windowing::application_icon(),
            size: Size::new(430.0, 430.0),
            min_size: Some(Size::new(360.0, 360.0)),
            ..Default::default()
        });

        self.windows = self.windows.with_person_details(id, person_id);

        open.map(move |window_id| Message::PersonDetailsOpened(window_id, person_id))
    }

    fn open_new_person_dialog(&mut self) -> Task<Message> {
        if let Some(id) = self.windows.new_person {
            return window::gain_focus(id);
        }

        self.editor.prepare_new_person();

        let (id, open) = window::open(window::Settings {
            icon: windowing::application_icon(),
            size: Size::new(420.0, 420.0),
            min_size: Some(Size::new(360.0, 360.0)),
            ..Default::default()
        });

        self.windows.new_person = Some(id);

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
                self.pending_delete_cemetery = None;
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
            StartMenuMessage::RequestDeleteCemetery(path) => {
                self.pending_delete_cemetery = Some(path);
                self.status = None;
            }
            StartMenuMessage::CancelDeleteCemetery => {
                self.pending_delete_cemetery = None;
            }
            StartMenuMessage::ConfirmDeleteCemetery(path) => {
                self.pending_delete_cemetery = None;
                self.delete_cemetery(path);
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
                return self.prompt_for_database_export_path();
            }
        }

        Task::none()
    }

    fn delete_cemetery(&mut self, path: PathBuf) {
        let Some(library) = &self.library else {
            self.status = Some(AppStatus::LibraryUnavailable);
            return;
        };

        match library.delete(&path) {
            Ok(()) => {
                self.selected_cemetery = None;
                self.refresh_cemeteries();
                self.status = Some(AppStatus::CemeteryDeleted);
            }
            Err(error) => {
                self.status = Some(AppStatus::CouldNotDeleteCemetery(error.to_string()));
            }
        }
    }

    fn load_selected_cemetery(&mut self) -> Task<Message> {
        let Some(path) = self.selected_cemetery.clone() else {
            return Task::none();
        };

        match SqliteCemeteryRepository::new(path.clone()).load() {
            Ok(cemetery) => {
                self.editor = MapEditor::from_cemetery(cemetery);
                self.session = CemeterySession::open(path);
                self.main_screen = MainScreen::MapEditor;
                self.status = None;

                self.windows
                    .main
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
        if !self.save_changes_before_export() {
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

    fn export_selected_cemetery_pdf(&mut self, destination: PathBuf) {
        if !self.save_changes_before_export() {
            return;
        }

        let options = PdfExportOptions {
            title: self.active_cemetery_title(),
            subtitle: self.localizer.text(MessageId::PdfExportSubtitle),
            empty_message: self.localizer.text(MessageId::EmptyPdfMap),
            footer: self.localizer.count(
                MessageId::PdfExportFooter,
                self.editor.cemetery().graves().len(),
            ),
        };

        self.status = match export_cemetery_map(self.editor.cemetery(), &destination, &options) {
            Ok(()) => Some(AppStatus::CemeteryPdfExported),
            Err(error) => Some(AppStatus::CouldNotExportCemeteryPdf(error.to_string())),
        };
    }

    fn prompt_for_database_export_path(&self) -> Task<Message> {
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

    fn prompt_for_pdf_export_path(&self) -> Task<Message> {
        let file_name = format!("{}.pdf", self.active_cemetery_title());

        let filter = self.localizer.text(MessageId::FileFilterPdf);
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .add_filter(&filter, &["pdf"])
                    .set_file_name(&file_name)
                    .save_file()
                    .await
                    .map(|file| file.path().to_owned())
            },
            Message::PdfExportPathChosen,
        )
    }

    fn active_cemetery_title(&self) -> String {
        self.session.active_cemetery_title()
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
        let (session, saved) = self.session.save_now(self.editor.cemetery());
        self.session = session;
        saved
    }

    fn save_changes_before_export(&mut self) -> bool {
        if self.session.has_unsaved_changes() && !self.save_active_cemetery() {
            self.status = Some(AppStatus::ExportSaveFailed);
            false
        } else {
            true
        }
    }

    fn mark_dirty_and_autosave(&mut self) -> Task<Message> {
        self.session = self.session.dirty();
        self.save_active_cemetery_in_background()
    }

    fn save_active_cemetery_in_background(&mut self) -> Task<Message> {
        let Some((session, request)) = self.session.begin_background_save(self.editor.cemetery())
        else {
            return Task::none();
        };
        self.session = session;

        Task::perform(
            save_cemetery_snapshot(request.path, request.cemetery),
            move |result| Message::SaveFinished {
                revision: request.revision,
                result,
            },
        )
    }
}

async fn save_cemetery_snapshot(path: PathBuf, cemetery: Cemetery) -> Result<(), String> {
    let repository = SqliteCemeteryRepository::new(path);
    repository
        .save(&cemetery)
        .map_err(|error: PersistenceError| error.to_string())
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
        background: Some(Background::Color(theme::BACKGROUND)),
        border: Border {
            color: theme::BORDER_SUBTLE,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn app_menu_dropdown(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::SURFACE_ALT)),
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow {
            color: theme::HEAVY_SHADOW,
            offset: Vector::new(0.0, 4.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}

fn app_menu_title_button(_: &Theme, status: button::Status, active: bool) -> button::Style {
    let pressed = status == button::Status::Pressed;
    let hovered = status == button::Status::Hovered;

    button::Style {
        background: if active || pressed {
            Some(Background::Color(theme::ACCENT_DARK))
        } else if hovered {
            Some(Background::Color(theme::ACCENT_REST))
        } else {
            None
        },
        text_color: theme::TEXT_PRIMARY,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 3.0.into(),
        },
        ..Default::default()
    }
}

fn app_menu_item_button(_: &Theme, status: button::Status) -> button::Style {
    let pressed = status == button::Status::Pressed;
    let hovered = status == button::Status::Hovered;
    let disabled = status == button::Status::Disabled;

    button::Style {
        background: if pressed {
            Some(Background::Color(theme::ACCENT_ACTIVE))
        } else if hovered && !disabled {
            Some(Background::Color(theme::ACCENT_REST))
        } else {
            None
        },
        text_color: if disabled {
            theme::TEXT_DISABLED
        } else {
            theme::TEXT_PRIMARY
        },
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 2.0.into(),
        },
        ..Default::default()
    }
}

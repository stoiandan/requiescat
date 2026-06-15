use std::borrow::Cow;
use std::fmt;
use std::fs;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::{LanguageIdentifier, langid};

const ENGLISH: &str = include_str!("../locales/en.ftl");
const ROMANIAN: &str = include_str!("../locales/ro.ftl");

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    English,
    Romanian,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::Romanian];

    fn locale(self) -> LanguageIdentifier {
        match self {
            Self::English => langid!("en"),
            Self::Romanian => langid!("ro"),
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::English => ENGLISH,
            Self::Romanian => ROMANIAN,
        }
    }

    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Romanian => "ro",
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        match code.trim() {
            "en" => Some(Self::English),
            "ro" => Some(Self::Romanian),
            _ => None,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::English => "English",
            Self::Romanian => "Română",
        })
    }
}

pub struct Localizer {
    language: Language,
    bundle: FluentBundle<FluentResource>,
}

impl Localizer {
    pub fn new(language: Language) -> Self {
        let resource =
            FluentResource::try_new(language.source().to_owned()).unwrap_or_else(|(_, errors)| {
                panic!("invalid {:?} Fluent catalog: {errors:?}", language)
            });
        let mut bundle = FluentBundle::new(vec![language.locale()]);
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .unwrap_or_else(|errors| panic!("invalid {:?} Fluent messages: {errors:?}", language));

        Self { language, bundle }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn set_language(&mut self, language: Language) {
        if self.language != language {
            *self = Self::new(language);
            save_preferred_language(language);
        }
    }

    pub fn text(&self, id: MessageId) -> String {
        self.format(id, None)
    }

    pub fn count(&self, id: MessageId, count: usize) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count as i64);
        self.format(id, Some(&args))
    }

    pub fn value<'a>(
        &self,
        id: MessageId,
        name: &'static str,
        value: impl Into<FluentValue<'a>>,
    ) -> String {
        let mut args = FluentArgs::new();
        args.set(name, value);
        self.format(id, Some(&args))
    }

    fn format(&self, id: MessageId, args: Option<&FluentArgs<'_>>) -> String {
        let key = id.as_str();
        let Some(message) = self.bundle.get_message(key) else {
            return key.to_owned();
        };
        let Some(pattern) = message.value() else {
            return key.to_owned();
        };

        let mut errors = Vec::new();
        let value: Cow<'_, str> = self.bundle.format_pattern(pattern, args, &mut errors);
        value.into_owned()
    }
}

impl Default for Localizer {
    fn default() -> Self {
        Self::new(preferred_language())
    }
}

fn preferred_language() -> Language {
    let Ok(path) = crate::persistence::application_data_directory() else {
        return Language::default();
    };

    fs::read_to_string(path.join("language"))
        .ok()
        .and_then(|code| Language::from_code(&code))
        .unwrap_or_default()
}

fn save_preferred_language(language: Language) {
    let Ok(directory) = crate::persistence::application_data_directory() else {
        return;
    };

    if fs::create_dir_all(&directory).is_ok() {
        let _ = fs::write(directory.join("language"), language.code());
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MessageId {
    UnsavedChanges,
    LanguageMenu,
    UnknownWindow,
    PersonDirectoryTitle,
    NewPersonTitle,
    PersonDetailsTitle,
    CemeteryLibraryTitle,
    LibraryEmpty,
    LibraryCount,
    BrandTagline,
    BrandDescription,
    SetupLibrary,
    SetupLibraryDescription,
    WelcomeBack,
    WelcomeBackDescription,
    CreateNewCemetery,
    ImportCemetery,
    OpenCemetery,
    ExportCemetery,
    ExportNamedCemetery,
    CreateCemetery,
    CreateCemeteryDescription,
    CemeteryName,
    BackToMenu,
    CemeteryLibrary,
    ChooseCemetery,
    NoCemeteries,
    NoCemeteriesDescription,
    SqliteCemetery,
    Open,
    Person,
    PersonNotFound,
    WillAddToGrave,
    WillCreateUnassigned,
    AddPerson,
    FirstName,
    LastName,
    DateOfBirthExample,
    DateOfDeceaseExample,
    DateOfBirth,
    DateOfDecease,
    Grave,
    GraveCanvas,
    NoPersonsAssociated,
    Persons,
    SearchPeople,
    GoToGrave,
    Unassign,
    Assign,
    Born,
    FileFilterSqliteCemetery,
    CouldNotLoadCemetery,
    LibraryUnavailable,
    CemeteryImported,
    CouldNotImportCemetery,
    CouldNotCreateCemetery,
    ExportSaveFailed,
    CemeteryExported,
    CouldNotExportCemetery,
    CouldNotRefreshCemeteries,
    SaveFailed,
    SoftwareUpdates,
    CheckingForUpdates,
    ApplicationUpToDate,
    CheckAgain,
    UpdateAvailable,
    DownloadUpdate,
    DownloadingUpdate,
    UpdateReady,
    RestartAndInstall,
    ReleaseNotes,
    UpdateCheckFailed,
    TryAgain,
}

impl MessageId {
    fn as_str(self) -> &'static str {
        match self {
            Self::UnsavedChanges => "unsaved-changes",
            Self::LanguageMenu => "language-menu",
            Self::UnknownWindow => "unknown-window",
            Self::PersonDirectoryTitle => "person-directory-title",
            Self::NewPersonTitle => "new-person-title",
            Self::PersonDetailsTitle => "person-details-title",
            Self::CemeteryLibraryTitle => "cemetery-library-title",
            Self::LibraryEmpty => "library-empty",
            Self::LibraryCount => "library-count",
            Self::BrandTagline => "brand-tagline",
            Self::BrandDescription => "brand-description",
            Self::SetupLibrary => "setup-library",
            Self::SetupLibraryDescription => "setup-library-description",
            Self::WelcomeBack => "welcome-back",
            Self::WelcomeBackDescription => "welcome-back-description",
            Self::CreateNewCemetery => "create-new-cemetery",
            Self::ImportCemetery => "import-cemetery",
            Self::OpenCemetery => "open-cemetery",
            Self::ExportCemetery => "export-cemetery",
            Self::ExportNamedCemetery => "export-named-cemetery",
            Self::CreateCemetery => "create-cemetery",
            Self::CreateCemeteryDescription => "create-cemetery-description",
            Self::CemeteryName => "cemetery-name",
            Self::BackToMenu => "back-to-menu",
            Self::CemeteryLibrary => "cemetery-library",
            Self::ChooseCemetery => "choose-cemetery",
            Self::NoCemeteries => "no-cemeteries",
            Self::NoCemeteriesDescription => "no-cemeteries-description",
            Self::SqliteCemetery => "sqlite-cemetery",
            Self::Open => "open",
            Self::Person => "person",
            Self::PersonNotFound => "person-not-found",
            Self::WillAddToGrave => "will-add-to-grave",
            Self::WillCreateUnassigned => "will-create-unassigned",
            Self::AddPerson => "add-person",
            Self::FirstName => "first-name",
            Self::LastName => "last-name",
            Self::DateOfBirthExample => "date-of-birth-example",
            Self::DateOfDeceaseExample => "date-of-decease-example",
            Self::DateOfBirth => "date-of-birth",
            Self::DateOfDecease => "date-of-decease",
            Self::Grave => "grave",
            Self::GraveCanvas => "grave-canvas",
            Self::NoPersonsAssociated => "no-persons-associated",
            Self::Persons => "persons",
            Self::SearchPeople => "search-people",
            Self::GoToGrave => "go-to-grave",
            Self::Unassign => "unassign",
            Self::Assign => "assign",
            Self::Born => "born",
            Self::FileFilterSqliteCemetery => "file-filter-sqlite-cemetery",
            Self::CouldNotLoadCemetery => "could-not-load-cemetery",
            Self::LibraryUnavailable => "library-unavailable",
            Self::CemeteryImported => "cemetery-imported",
            Self::CouldNotImportCemetery => "could-not-import-cemetery",
            Self::CouldNotCreateCemetery => "could-not-create-cemetery",
            Self::ExportSaveFailed => "export-save-failed",
            Self::CemeteryExported => "cemetery-exported",
            Self::CouldNotExportCemetery => "could-not-export-cemetery",
            Self::CouldNotRefreshCemeteries => "could-not-refresh-cemeteries",
            Self::SaveFailed => "save-failed",
            Self::SoftwareUpdates => "software-updates",
            Self::CheckingForUpdates => "checking-for-updates",
            Self::ApplicationUpToDate => "application-up-to-date",
            Self::CheckAgain => "check-again",
            Self::UpdateAvailable => "update-available",
            Self::DownloadUpdate => "download-update",
            Self::DownloadingUpdate => "downloading-update",
            Self::UpdateReady => "update-ready",
            Self::RestartAndInstall => "restart-and-install",
            Self::ReleaseNotes => "release-notes",
            Self::UpdateCheckFailed => "update-check-failed",
            Self::TryAgain => "try-again",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_romanian_plural_categories() {
        let localizer = Localizer::new(Language::Romanian);

        assert_eq!(
            localizer.count(MessageId::LibraryCount, 1),
            "Un cimitir în bibliotecă"
        );
        assert_eq!(
            localizer.count(MessageId::LibraryCount, 2),
            "2 cimitire în bibliotecă"
        );
        assert_eq!(
            localizer.count(MessageId::LibraryCount, 20),
            "20 de cimitire în bibliotecă"
        );
    }

    #[test]
    fn interpolates_named_values() {
        let localizer = Localizer::new(Language::English);

        assert_eq!(
            localizer.value(MessageId::ExportNamedCemetery, "name", "Central"),
            "Export Central"
        );
    }
}

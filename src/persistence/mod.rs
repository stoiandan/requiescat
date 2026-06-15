use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::{Point, Size};
use rusqlite::{Connection, OpenFlags, OptionalExtension, backup::Backup, params};

use crate::models::{
    Cemetery, Grave, GraveColor, GraveId, GraveRectangle, Person, PersonDate, PersonId,
};

const APPLICATION_ID: &str = "requiescat";
const CURRENT_SCHEMA_VERSION: u32 = 1;

pub trait CemeteryRepository {
    fn load(&self) -> Result<Cemetery, PersistenceError>;
    fn save(&mut self, cemetery: &Cemetery) -> Result<(), PersistenceError>;
}

#[derive(Debug)]
pub enum PersistenceError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    InvalidData(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Sqlite(error) => write!(formatter, "{error}"),
            Self::InvalidData(message) => formatter.write_str(message),
        }
    }
}

impl From<std::io::Error> for PersistenceError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for PersistenceError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

#[derive(Debug, Clone)]
pub struct CemeteryFile {
    path: PathBuf,
    name: String,
}

impl CemeteryFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone)]
pub struct CemeteryLibrary {
    directory: PathBuf,
}

impl CemeteryLibrary {
    pub fn for_current_user() -> Result<Self, PersistenceError> {
        let directory = application_data_directory()?.join("Cemeteries");
        Self::new(directory)
    }

    pub fn new(directory: PathBuf) -> Result<Self, PersistenceError> {
        fs::create_dir_all(&directory)?;
        Ok(Self { directory })
    }

    pub fn cemeteries(&self) -> Result<Vec<CemeteryFile>, PersistenceError> {
        let mut cemeteries = fs::read_dir(&self.directory)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| is_sqlite_file(path))
            .map(|path| CemeteryFile {
                name: cemetery_name(&path),
                path,
            })
            .collect::<Vec<_>>();

        cemeteries.sort_by_key(|cemetery| cemetery.name.to_lowercase());
        Ok(cemeteries)
    }

    pub fn import(&self, source: &Path) -> Result<PathBuf, PersistenceError> {
        SqliteCemeteryRepository::new(source.to_owned()).validate()?;

        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("Imported Cemetery.sqlite");
        let destination = unique_destination(&self.directory, file_name);

        fs::copy(source, &destination)?;
        Ok(destination)
    }

    pub fn create(&self, name: &str) -> Result<PathBuf, PersistenceError> {
        let file_name = cemetery_file_name(name)?;
        let destination = unique_destination(&self.directory, &file_name);
        let mut repository = SqliteCemeteryRepository::new(destination.clone());
        repository.save(&Cemetery::default())?;
        Ok(destination)
    }

    pub fn export(&self, source: &Path, destination: &Path) -> Result<(), PersistenceError> {
        if source == destination {
            return Ok(());
        }

        fs::copy(source, destination)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SqliteCemeteryRepository {
    path: PathBuf,
}

impl SqliteCemeteryRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn validate(&self) -> Result<(), PersistenceError> {
        let connection = self.read_only_connection()?;
        validate_current_schema(&connection)
    }

    fn read_only_connection(&self) -> Result<Connection, PersistenceError> {
        let connection = Connection::open_with_flags(&self.path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        configure_connection(&connection)?;
        Ok(connection)
    }

    fn writable_connection(&self) -> Result<Connection, PersistenceError> {
        if self.path.exists() {
            self.migrate_if_needed()?;
        }

        let connection = Connection::open(&self.path)?;
        configure_connection(&connection)?;
        if !table_exists(&connection, "requiescat_metadata")? {
            initialize_schema(&connection)?;
        }
        Ok(connection)
    }

    fn migrate_if_needed(&self) -> Result<Option<PathBuf>, PersistenceError> {
        let connection = self.read_only_connection()?;
        validate_compatible_schema(&connection)?;
        let version = schema_version(&connection)?;
        drop(connection);

        if version == CURRENT_SCHEMA_VERSION {
            return Ok(None);
        }

        ensure_migration_path(version)?;
        let backup_path = migration_backup_path(&self.path, version, CURRENT_SCHEMA_VERSION);
        backup_database(&self.path, &backup_path)?;

        let mut connection = Connection::open(&self.path)?;
        configure_connection(&connection)?;
        migrate_schema(&mut connection, version)?;
        validate_current_schema(&connection)?;

        Ok(Some(backup_path))
    }
}

impl CemeteryRepository for SqliteCemeteryRepository {
    fn load(&self) -> Result<Cemetery, PersistenceError> {
        self.migrate_if_needed()?;
        let connection = self.read_only_connection()?;
        validate_current_schema(&connection)?;

        let graves = {
            let has_color = column_exists(&connection, "graves", "color")?;
            let query = if has_color {
                "SELECT id, x, y, width, height, color FROM graves ORDER BY id"
            } else {
                "SELECT id, x, y, width, height, '' FROM graves ORDER BY id"
            };
            let mut statement = connection.prepare(query)?;
            let rows = statement.query_map([], |row| {
                Ok(GraveRow {
                    id: row.get(0)?,
                    x: row.get(1)?,
                    y: row.get(2)?,
                    width: row.get(3)?,
                    height: row.get(4)?,
                    color: row.get(5)?,
                })
            })?;

            rows.collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(Grave::from)
                .collect()
        };

        let people = {
            let mut statement = connection.prepare(
                "
                SELECT id, first_name, last_name, date_of_birth,
                       COALESCE(date_of_decease, ''), grave_id
                FROM persons
                ORDER BY id
                ",
            )?;
            let rows = statement.query_map([], |row| {
                Ok(PersonRow {
                    id: row.get(0)?,
                    first_name: row.get(1)?,
                    last_name: row.get(2)?,
                    date_of_birth: row.get(3)?,
                    date_of_decease: row.get(4)?,
                    grave_id: row.get(5)?,
                })
            })?;

            rows.collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(Person::try_from)
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(Cemetery::from_records(graves, people))
    }

    fn save(&mut self, cemetery: &Cemetery) -> Result<(), PersistenceError> {
        let mut connection = self.writable_connection()?;
        ensure_grave_color_column(&connection)?;
        let transaction = connection.transaction()?;

        transaction.execute("DELETE FROM persons", [])?;
        transaction.execute("DELETE FROM graves", [])?;

        {
            let mut statement = transaction.prepare(
                "
                INSERT INTO graves (id, x, y, width, height, color)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
            )?;

            for grave in cemetery.graves().iter().copied() {
                let row = GraveRow::from(grave);
                statement.execute(params![
                    row.id, row.x, row.y, row.width, row.height, row.color
                ])?;
            }
        }

        {
            let mut statement = transaction.prepare(
                "
                INSERT INTO persons (
                    id, first_name, last_name, date_of_birth, date_of_decease, grave_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
            )?;

            for person in cemetery.people() {
                let row = PersonRow::from(person.clone());
                let date_of_decease =
                    (!row.date_of_decease.is_empty()).then_some(row.date_of_decease.as_str());
                statement.execute(params![
                    row.id,
                    row.first_name,
                    row.last_name,
                    row.date_of_birth,
                    date_of_decease,
                    row.grave_id
                ])?;
            }
        }

        transaction.commit()?;
        Ok(())
    }
}

fn configure_connection(connection: &Connection) -> Result<(), PersistenceError> {
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(())
}

fn initialize_schema(connection: &Connection) -> Result<(), PersistenceError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS requiescat_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS graves (
            id INTEGER PRIMARY KEY,
            x REAL NOT NULL,
            y REAL NOT NULL,
            width REAL NOT NULL,
            height REAL NOT NULL,
            color TEXT NOT NULL DEFAULT '#a61f28'
        );

        CREATE TABLE IF NOT EXISTS persons (
            id INTEGER PRIMARY KEY,
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL,
            date_of_birth TEXT NOT NULL,
            date_of_decease TEXT,
            grave_id INTEGER REFERENCES graves(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS requiescat_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        ",
    )?;
    connection.execute(
        "
        INSERT INTO requiescat_metadata (key, value)
        VALUES ('application', ?1)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        ",
        [APPLICATION_ID],
    )?;
    connection.execute(
        "
        INSERT INTO requiescat_metadata (key, value)
        VALUES ('schema_version', ?1)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        ",
        [CURRENT_SCHEMA_VERSION.to_string()],
    )?;
    connection.execute(
        "
        INSERT OR IGNORE INTO requiescat_migrations (version)
        VALUES (?1)
        ",
        [CURRENT_SCHEMA_VERSION],
    )?;
    Ok(())
}

fn validate_compatible_schema(connection: &Connection) -> Result<(), PersistenceError> {
    if table_exists(connection, "requiescat_metadata")? {
        let application = metadata_value(connection, "application")?;
        if application.as_deref() != Some(APPLICATION_ID) {
            return Err(PersistenceError::InvalidData(
                "This SQLite file is not a Requiescat cemetery.".to_owned(),
            ));
        }

        let version = schema_version(connection)?;
        if version > CURRENT_SCHEMA_VERSION {
            return Err(PersistenceError::InvalidData(format!(
                "This cemetery uses schema version {version}, but this version of Requiescat supports up to version {CURRENT_SCHEMA_VERSION}."
            )));
        }
    }

    validate_table_columns(connection, "graves", &["id", "x", "y", "width", "height"])?;
    validate_table_columns(
        connection,
        "persons",
        &[
            "id",
            "first_name",
            "last_name",
            "date_of_birth",
            "date_of_decease",
            "grave_id",
        ],
    )?;
    Ok(())
}

fn validate_current_schema(connection: &Connection) -> Result<(), PersistenceError> {
    validate_compatible_schema(connection)?;

    let version = schema_version(connection)?;
    if version != CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::InvalidData(format!(
            "Unsupported cemetery schema version: {version}"
        )));
    }

    validate_table_columns(
        connection,
        "requiescat_migrations",
        &["version", "applied_at"],
    )
}

fn schema_version(connection: &Connection) -> Result<u32, PersistenceError> {
    if !table_exists(connection, "requiescat_metadata")? {
        return Ok(0);
    }

    let version = metadata_value(connection, "schema_version")?.ok_or_else(|| {
        PersistenceError::InvalidData("The cemetery schema version is missing.".to_owned())
    })?;

    version.parse().map_err(|_| {
        PersistenceError::InvalidData(format!("Invalid cemetery schema version: {version}"))
    })
}

fn migrate_schema(connection: &mut Connection, from_version: u32) -> Result<(), PersistenceError> {
    ensure_migration_path(from_version)?;
    let transaction = connection.transaction()?;

    // Add ordered migrations here when CURRENT_SCHEMA_VERSION first increases.
    let migrated_version = from_version;

    if migrated_version != CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::InvalidData(format!(
            "No migration path from schema version {from_version} to {CURRENT_SCHEMA_VERSION}."
        )));
    }

    transaction.commit()?;
    Ok(())
}

fn ensure_migration_path(from_version: u32) -> Result<(), PersistenceError> {
    if from_version == CURRENT_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(PersistenceError::InvalidData(format!(
            "No migration path from schema version {from_version} to {CURRENT_SCHEMA_VERSION}."
        )))
    }
}

fn backup_database(source: &Path, destination: &Path) -> Result<(), PersistenceError> {
    let source = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut destination = Connection::open(destination)?;
    let backup = Backup::new(&source, &mut destination)?;
    backup.run_to_completion(32, Duration::from_millis(10), None)?;
    Ok(())
}

fn migration_backup_path(source: &Path, from_version: u32, to_version: u32) -> PathBuf {
    let directory = source.parent().unwrap_or_else(|| Path::new("."));
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("cemetery");
    let extension = source
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("sqlite");
    let base = format!("{stem}.before-schema-{from_version}-to-{to_version}");
    let mut destination = directory.join(format!("{base}.{extension}"));
    let mut suffix = 2;

    while destination.exists() {
        destination = directory.join(format!("{base}-{suffix}.{extension}"));
        suffix += 1;
    }

    destination
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, PersistenceError> {
    let exists = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )?;
    Ok(exists)
}

fn metadata_value(connection: &Connection, key: &str) -> Result<Option<String>, PersistenceError> {
    Ok(connection
        .query_row(
            "SELECT value FROM requiescat_metadata WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()?)
}

fn ensure_grave_color_column(connection: &Connection) -> Result<(), PersistenceError> {
    if !column_exists(connection, "graves", "color")? {
        connection.execute(
            "ALTER TABLE graves ADD COLUMN color TEXT NOT NULL DEFAULT '#a61f28'",
            [],
        )?;
    }

    Ok(())
}

fn column_exists(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, PersistenceError> {
    Ok(table_columns(connection, table)?
        .iter()
        .any(|existing| existing == column))
}

fn validate_table_columns(
    connection: &Connection,
    table: &str,
    required_columns: &[&str],
) -> Result<(), PersistenceError> {
    if !table_exists(connection, table)? {
        return Err(PersistenceError::InvalidData(format!(
            "The cemetery database is missing the {table} table."
        )));
    }

    let columns = table_columns(connection, table)?;

    if let Some(missing) = required_columns
        .iter()
        .find(|required| !columns.iter().any(|column| column.as_str() == **required))
    {
        return Err(PersistenceError::InvalidData(format!(
            "The cemetery database is missing the {table}.{missing} column."
        )));
    }

    Ok(())
}

fn table_columns(connection: &Connection, table: &str) -> Result<Vec<String>, PersistenceError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    Ok(statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?)
}

#[derive(Debug, Clone)]
pub struct GraveRow {
    pub id: i64,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: String,
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
            color: grave.color().to_hex(),
        }
    }
}

impl From<GraveRow> for Grave {
    fn from(row: GraveRow) -> Self {
        Grave::with_color(
            GraveId::new(row.id),
            GraveRectangle::from_top_left_size(
                Point::new(row.x, row.y),
                Size::new(row.width, row.height),
            ),
            GraveColor::from_hex(&row.color).unwrap_or_default(),
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

impl TryFrom<PersonRow> for Person {
    type Error = PersistenceError;

    fn try_from(row: PersonRow) -> Result<Self, Self::Error> {
        let date_of_birth = PersonDate::parse(&row.date_of_birth).map_err(|_| {
            PersistenceError::InvalidData(format!(
                "Person {} has an invalid birth date: {}",
                row.id, row.date_of_birth
            ))
        })?;
        let date_of_decease = if row.date_of_decease.trim().is_empty() {
            None
        } else {
            Some(PersonDate::parse(&row.date_of_decease).map_err(|_| {
                PersistenceError::InvalidData(format!(
                    "Person {} has an invalid decease date: {}",
                    row.id, row.date_of_decease
                ))
            })?)
        };

        Ok(Person::from_parts(
            PersonId::new(row.id),
            row.first_name,
            row.last_name,
            date_of_birth,
            date_of_decease,
            row.grave_id.map(GraveId::new),
        ))
    }
}

pub(crate) fn application_data_directory() -> Result<PathBuf, PersistenceError> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME").ok_or_else(|| {
            PersistenceError::InvalidData("The HOME directory is unavailable".to_owned())
        })?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Requiescat"))
    }

    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var_os("APPDATA").ok_or_else(|| {
            PersistenceError::InvalidData("The APPDATA directory is unavailable".to_owned())
        })?;
        Ok(PathBuf::from(app_data).join("Requiescat"))
    }

    #[cfg(target_os = "linux")]
    {
        linux_application_data_directory(
            std::env::var_os("XDG_DATA_HOME"),
            std::env::var_os("HOME"),
        )
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(PersistenceError::InvalidData(
            "This operating system is not supported.".to_owned(),
        ))
    }
}

#[cfg(any(target_os = "linux", test))]
fn linux_application_data_directory(
    data_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, PersistenceError> {
    if let Some(data_home) = data_home.filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(data_home).join("requiescat"));
    }

    let home = home.filter(|path| !path.is_empty()).ok_or_else(|| {
        PersistenceError::InvalidData(
            "Neither XDG_DATA_HOME nor HOME is available on Linux.".to_owned(),
        )
    })?;

    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("requiescat"))
}

fn is_sqlite_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "sqlite" | "sqlite3" | "db"
                )
            })
}

fn cemetery_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Unnamed Cemetery")
        .to_owned()
}

fn cemetery_file_name(name: &str) -> Result<String, PersistenceError> {
    let name = name.trim();

    if name.is_empty() {
        return Err(PersistenceError::InvalidData(
            "Enter a cemetery name.".to_owned(),
        ));
    }

    if matches!(name, "." | "..")
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err(PersistenceError::InvalidData(
            "The cemetery name contains invalid characters.".to_owned(),
        ));
    }

    let path = Path::new(name);
    let stem = match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension)
            if matches!(
                extension.to_ascii_lowercase().as_str(),
                "sqlite" | "sqlite3" | "db"
            ) =>
        {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(name)
        }
        _ => name,
    };

    Ok(format!("{stem}.sqlite"))
}

fn unique_destination(directory: &Path, file_name: &str) -> PathBuf {
    let source = Path::new(file_name);
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Imported Cemetery");
    let extension = source
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("sqlite");

    let mut destination = directory.join(format!("{stem}.{extension}"));
    let mut suffix = 2;

    while destination.exists() {
        destination = directory.join(format!("{stem} {suffix}.{extension}"));
        suffix += 1;
    }

    destination
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_destination_adds_a_suffix_when_file_exists() {
        let directory = std::env::temp_dir().join(format!(
            "requiescat-persistence-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).unwrap();
        let existing = directory.join("Central.sqlite");
        fs::write(&existing, []).unwrap();

        let destination = unique_destination(&directory, "Central.sqlite");

        assert_eq!(destination, directory.join("Central 2.sqlite"));

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn library_creates_an_empty_cemetery_with_a_unique_name() {
        let directory =
            std::env::temp_dir().join(format!("requiescat-library-test-{}", std::process::id()));
        let library = CemeteryLibrary::new(directory.clone()).unwrap();

        let first = library.create("Central").unwrap();
        let second = library.create("Central").unwrap();

        assert_eq!(first, directory.join("Central.sqlite"));
        assert_eq!(second, directory.join("Central 2.sqlite"));
        assert!(SqliteCemeteryRepository::new(first.clone()).load().is_ok());
        assert!(SqliteCemeteryRepository::new(second).load().is_ok());

        let connection = Connection::open(first).unwrap();
        assert_eq!(
            metadata_value(&connection, "application")
                .unwrap()
                .as_deref(),
            Some(APPLICATION_ID)
        );
        assert_eq!(schema_version(&connection).unwrap(), CURRENT_SCHEMA_VERSION);
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM requiescat_migrations WHERE version = ?1",
                    [CURRENT_SCHEMA_VERSION],
                    |row| row.get::<_, usize>(0),
                )
                .unwrap(),
            1
        );
        drop(connection);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn import_rejects_unrelated_sqlite_without_modifying_it() {
        let root = std::env::temp_dir().join(format!(
            "requiescat-import-validation-test-{}",
            std::process::id()
        ));
        let library = CemeteryLibrary::new(root.join("library")).unwrap();
        let source = root.join("unrelated.sqlite");
        let connection = Connection::open(&source).unwrap();
        connection
            .execute("CREATE TABLE notes (body TEXT NOT NULL)", [])
            .unwrap();
        drop(connection);

        assert!(library.import(&source).is_err());

        let connection = Connection::open(&source).unwrap();
        assert!(!table_exists(&connection, "graves").unwrap());
        assert!(!table_exists(&connection, "persons").unwrap());
        assert!(!table_exists(&connection, "requiescat_metadata").unwrap());
        drop(connection);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn database_without_version_metadata_is_rejected() {
        let path = std::env::temp_dir().join(format!(
            "requiescat-legacy-validation-test-{}.sqlite",
            std::process::id()
        ));
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "
                CREATE TABLE graves (
                    id INTEGER PRIMARY KEY,
                    x REAL NOT NULL,
                    y REAL NOT NULL,
                    width REAL NOT NULL,
                    height REAL NOT NULL
                );
                CREATE TABLE persons (
                    id INTEGER PRIMARY KEY,
                    first_name TEXT NOT NULL,
                    last_name TEXT NOT NULL,
                    date_of_birth TEXT NOT NULL,
                    date_of_decease TEXT,
                    grave_id INTEGER REFERENCES graves(id) ON DELETE SET NULL
                );
                ",
            )
            .unwrap();
        drop(connection);

        assert!(SqliteCemeteryRepository::new(path.clone()).load().is_err());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn cemetery_file_name_validates_and_normalizes_names() {
        assert_eq!(
            cemetery_file_name("  Central Cemetery  ").unwrap(),
            "Central Cemetery.sqlite"
        );
        assert_eq!(
            cemetery_file_name("Central.sqlite").unwrap(),
            "Central.sqlite"
        );
        assert!(cemetery_file_name(" ").is_err());
        assert!(cemetery_file_name("../Central").is_err());
    }

    #[test]
    fn linux_data_directory_prefers_xdg_data_home() {
        let directory = linux_application_data_directory(
            Some("/home/dan/.data".into()),
            Some("/home/dan".into()),
        )
        .unwrap();

        assert_eq!(directory, PathBuf::from("/home/dan/.data/requiescat"));
    }

    #[test]
    fn linux_data_directory_falls_back_to_local_share() {
        let directory = linux_application_data_directory(None, Some("/home/dan".into())).unwrap();

        assert_eq!(
            directory,
            PathBuf::from("/home/dan/.local/share/requiescat")
        );
    }

    #[test]
    fn linux_data_directory_requires_an_xdg_or_home_directory() {
        assert!(linux_application_data_directory(None, None).is_err());
    }

    #[test]
    fn sqlite_repository_round_trips_cemetery_data() {
        let path = std::env::temp_dir().join(format!(
            "requiescat-round-trip-{}.sqlite",
            std::process::id()
        ));
        let mut cemetery = Cemetery::default();
        let grave_color = GraveColor::from_rgb8(50, 123, 171);
        let grave_id = cemetery.add_grave_with_color(
            GraveRectangle::from_top_left_size(Point::new(12.0, 24.0), Size::new(40.0, 80.0)),
            grave_color,
        );
        cemetery.create_person_with_details(
            "Ada".to_owned(),
            "Lovelace".to_owned(),
            PersonDate::parse("10-12-1815").unwrap(),
            Some(PersonDate::parse("27-11-1852").unwrap()),
            Some(grave_id),
        );

        let mut repository = SqliteCemeteryRepository::new(path.clone());
        repository.save(&cemetery).unwrap();
        let mut loaded = repository.load().unwrap();

        assert_eq!(loaded.graves().len(), 1);
        assert_eq!(loaded.grave(grave_id).map(Grave::color), Some(grave_color));
        assert_eq!(loaded.search_people("").len(), 1);
        assert_eq!(loaded.search_people("Ada")[0].grave_id(), Some(grave_id));
        assert_eq!(
            loaded.add_grave_with_color(
                GraveRectangle::from_top_left_size(
                    Point::new(100.0, 100.0),
                    Size::new(20.0, 40.0),
                ),
                GraveColor::default(),
            ),
            GraveId::new(2)
        );
        assert_eq!(
            loaded.create_person_with_details(
                "Grace".to_owned(),
                "Hopper".to_owned(),
                PersonDate::parse("09-12-1906").unwrap(),
                None,
                None,
            ),
            PersonId::new(2)
        );

        fs::remove_file(path).unwrap();
    }
}

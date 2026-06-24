use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use iced::{Point, Size};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::models::{
    Cemetery, Delimiter, DelimiterId, DelimiterType, Grave, GraveColor, GraveGps, GraveId,
    GraveRectangle, Person, PersonDate, PersonId,
};

const APPLICATION_ID: &str = "requiescat";
const CURRENT_SCHEMA_VERSION: u32 = 3;

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

    pub fn delete(&self, path: &Path) -> Result<(), PersistenceError> {
        let Some(file_name) = path.file_name() else {
            return Err(PersistenceError::InvalidData(
                "Choose a cemetery from the library.".to_owned(),
            ));
        };

        let library_path = self.directory.join(file_name);
        if library_path != path || !is_sqlite_file(&library_path) {
            return Err(PersistenceError::InvalidData(
                "Choose a cemetery from the library.".to_owned(),
            ));
        }

        fs::remove_file(library_path)?;
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
        let connection = Connection::open(&self.path)?;
        configure_connection(&connection)?;
        if !table_exists(&connection, "requiescat_metadata")? {
            initialize_schema(&connection)?;
        }
        migrate_schema(&connection)?;
        Ok(connection)
    }
}

impl CemeteryRepository for SqliteCemeteryRepository {
    fn load(&self) -> Result<Cemetery, PersistenceError> {
        let connection = Connection::open(&self.path)?;
        configure_connection(&connection)?;
        if table_exists(&connection, "requiescat_metadata")? {
            migrate_schema(&connection)?;
        }
        validate_current_schema(&connection)?;

        let graves = {
            let mut statement = connection.prepare(
                "
                SELECT id, x, y, width, height, color, rotation_degrees, gps
                FROM graves
                ORDER BY id
                ",
            )?;
            let rows = statement.query_map([], |row| {
                Ok(GraveRow {
                    id: row.get(0)?,
                    x: row.get(1)?,
                    y: row.get(2)?,
                    width: row.get(3)?,
                    height: row.get(4)?,
                    color: row.get(5)?,
                    rotation_degrees: row.get(6)?,
                    gps: row.get(7)?,
                })
            })?;

            rows.collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(Grave::try_from)
                .collect::<Result<Vec<_>, _>>()?
        };

        let delimiters = {
            let mut statement = connection.prepare(
                "
                SELECT id, x, y, width, height, color, type, rotation_degrees
                FROM delimiters
                ORDER BY id
                ",
            )?;
            let rows = statement.query_map([], |row| {
                Ok(DelimiterRow {
                    id: row.get(0)?,
                    x: row.get(1)?,
                    y: row.get(2)?,
                    width: row.get(3)?,
                    height: row.get(4)?,
                    color: row.get(5)?,
                    delimiter_type: row.get(6)?,
                    rotation_degrees: row.get(7)?,
                })
            })?;

            rows.collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(Delimiter::try_from)
                .collect::<Result<Vec<_>, _>>()?
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

        Ok(Cemetery::from_records(graves, delimiters, people))
    }

    fn save(&mut self, cemetery: &Cemetery) -> Result<(), PersistenceError> {
        let mut connection = self.writable_connection()?;
        validate_current_schema(&connection)?;
        let transaction = connection.transaction()?;

        transaction.execute("DELETE FROM persons", [])?;
        transaction.execute("DELETE FROM graves", [])?;
        transaction.execute("DELETE FROM delimiters", [])?;

        {
            let mut statement = transaction.prepare(
                "
                INSERT INTO graves (id, x, y, width, height, color, rotation_degrees, gps)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ",
            )?;

            for grave in cemetery.graves().iter().copied() {
                let row = GraveRow::from(grave);
                statement.execute(params![
                    row.id,
                    row.x,
                    row.y,
                    row.width,
                    row.height,
                    row.color,
                    row.rotation_degrees,
                    row.gps
                ])?;
            }
        }

        {
            let mut statement = transaction.prepare(
                "
                INSERT INTO delimiters (id, x, y, width, height, color, type, rotation_degrees)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ",
            )?;

            for delimiter in cemetery.delimiters().iter().copied() {
                let row = DelimiterRow::from(delimiter);
                statement.execute(params![
                    row.id,
                    row.x,
                    row.y,
                    row.width,
                    row.height,
                    row.color,
                    row.delimiter_type,
                    row.rotation_degrees
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
            color TEXT NOT NULL DEFAULT '#a61f28',
            rotation_degrees REAL NOT NULL DEFAULT 0,
            gps TEXT
        );

        CREATE TABLE IF NOT EXISTS persons (
            id INTEGER PRIMARY KEY,
            first_name TEXT NOT NULL,
            last_name TEXT NOT NULL,
            date_of_birth TEXT NOT NULL,
            date_of_decease TEXT,
            grave_id INTEGER REFERENCES graves(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS delimiters (
            id INTEGER PRIMARY KEY,
            x REAL NOT NULL,
            y REAL NOT NULL,
            width REAL NOT NULL,
            height REAL NOT NULL,
            color TEXT NOT NULL DEFAULT '#a61f28',
            type TEXT NOT NULL DEFAULT 'wall',
            rotation_degrees REAL NOT NULL DEFAULT 0
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

fn validate_current_schema(connection: &Connection) -> Result<(), PersistenceError> {
    validate_table_columns(connection, "requiescat_metadata", &["key", "value"])?;
    let version = schema_version(connection)?;
    let grave_columns = if version >= 3 {
        &[
            "id",
            "x",
            "y",
            "width",
            "height",
            "color",
            "rotation_degrees",
            "gps",
        ][..]
    } else {
        &["id", "x", "y", "width", "height", "color", "gps"][..]
    };
    validate_table_columns(connection, "graves", grave_columns)?;
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
    validate_table_columns(
        connection,
        "requiescat_migrations",
        &["version", "applied_at"],
    )?;

    let application = metadata_value(connection, "application")?;
    if application.as_deref() != Some(APPLICATION_ID) {
        return Err(PersistenceError::InvalidData(
            "This SQLite file is not a Requiescat cemetery.".to_owned(),
        ));
    }

    if version > CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::InvalidData(format!(
            "Unsupported cemetery schema version: {version}"
        )));
    }
    if version >= 2 {
        let delimiter_columns = if version >= 3 {
            &[
                "id",
                "x",
                "y",
                "width",
                "height",
                "color",
                "type",
                "rotation_degrees",
            ][..]
        } else {
            &["id", "x", "y", "width", "height", "color", "type"][..]
        };
        validate_table_columns(connection, "delimiters", delimiter_columns)?;
    }

    Ok(())
}

fn migrate_schema(connection: &Connection) -> Result<(), PersistenceError> {
    let application = metadata_value(connection, "application")?;
    if application.as_deref() != Some(APPLICATION_ID) {
        return Ok(());
    }

    let mut version = schema_version(connection)?;

    if version < 2 {
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS delimiters (
                id INTEGER PRIMARY KEY,
                x REAL NOT NULL,
                y REAL NOT NULL,
                width REAL NOT NULL,
                height REAL NOT NULL,
                color TEXT NOT NULL DEFAULT '#a61f28',
                type TEXT NOT NULL DEFAULT 'wall'
            );
            INSERT OR IGNORE INTO requiescat_migrations (version) VALUES (2);
            UPDATE requiescat_metadata SET value = '2' WHERE key = 'schema_version';
            ",
        )?;
        version = 2;
    }

    if version < 3 {
        connection.execute_batch(
            "
            ALTER TABLE graves ADD COLUMN rotation_degrees REAL NOT NULL DEFAULT 0;
            ALTER TABLE delimiters ADD COLUMN rotation_degrees REAL NOT NULL DEFAULT 0;
            INSERT OR IGNORE INTO requiescat_migrations (version) VALUES (3);
            UPDATE requiescat_metadata SET value = '3' WHERE key = 'schema_version';
            ",
        )?;
        version = 3;
    }

    if version > CURRENT_SCHEMA_VERSION {
        return Err(PersistenceError::InvalidData(format!(
            "Unsupported cemetery schema version: {version}"
        )));
    }

    Ok(())
}

fn schema_version(connection: &Connection) -> Result<u32, PersistenceError> {
    let version = metadata_value(connection, "schema_version")?.ok_or_else(|| {
        PersistenceError::InvalidData("The cemetery schema version is missing.".to_owned())
    })?;

    version.parse().map_err(|_| {
        PersistenceError::InvalidData(format!("Invalid cemetery schema version: {version}"))
    })
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
struct GraveRow {
    id: i64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: String,
    rotation_degrees: f32,
    gps: Option<String>,
}

#[derive(Debug, Clone)]
struct DelimiterRow {
    id: i64,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: String,
    delimiter_type: String,
    rotation_degrees: f32,
}

#[derive(Debug, Clone)]
struct PersonRow {
    id: i64,
    first_name: String,
    last_name: String,
    date_of_birth: String,
    date_of_decease: String,
    grave_id: Option<i64>,
}

impl From<Delimiter> for DelimiterRow {
    fn from(delimiter: Delimiter) -> Self {
        let rectangle = delimiter.rectangle();
        let top_left = rectangle.top_left();
        let size = rectangle.size();

        Self {
            id: delimiter.id().value(),
            x: top_left.x,
            y: top_left.y,
            width: size.width,
            height: size.height,
            color: delimiter.color().to_hex(),
            delimiter_type: delimiter.delimiter_type().as_str().to_owned(),
            rotation_degrees: delimiter.rotation_degrees(),
        }
    }
}

impl TryFrom<DelimiterRow> for Delimiter {
    type Error = PersistenceError;

    fn try_from(row: DelimiterRow) -> Result<Self, Self::Error> {
        let delimiter_type = DelimiterType::from_str(&row.delimiter_type).ok_or_else(|| {
            PersistenceError::InvalidData(format!(
                "Delimiter {} has invalid type: {}",
                row.id, row.delimiter_type
            ))
        })?;

        Ok(Delimiter::with_color_and_type(
            DelimiterId::new(row.id),
            GraveRectangle::from_top_left_size(
                Point::new(row.x, row.y),
                Size::new(row.width, row.height),
            ),
            GraveColor::from_hex(&row.color).unwrap_or_default(),
            row.rotation_degrees,
            delimiter_type,
        ))
    }
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
            rotation_degrees: grave.rotation_degrees(),
            gps: grave.gps().map(|gps| gps.to_string()),
        }
    }
}

impl TryFrom<GraveRow> for Grave {
    type Error = PersistenceError;

    fn try_from(row: GraveRow) -> Result<Self, Self::Error> {
        let gps = row
            .gps
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                GraveGps::parse(&value).map_err(|_| {
                    PersistenceError::InvalidData(format!(
                        "Grave {} has invalid GPS coordinates: {}",
                        row.id, value
                    ))
                })
            })
            .transpose()?;

        Ok(Grave::from_parts(
            GraveId::new(row.id),
            GraveRectangle::from_top_left_size(
                Point::new(row.x, row.y),
                Size::new(row.width, row.height),
            ),
            GraveColor::from_hex(&row.color).unwrap_or_default(),
            row.rotation_degrees,
            gps,
        ))
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
    fn library_deletes_a_cemetery_file() {
        let directory = std::env::temp_dir().join(format!(
            "requiescat-library-delete-test-{}",
            std::process::id()
        ));
        let library = CemeteryLibrary::new(directory.clone()).unwrap();
        let cemetery = library.create("Central").unwrap();

        library.delete(&cemetery).unwrap();

        assert!(!cemetery.exists());
        assert!(library.cemeteries().unwrap().is_empty());

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
    fn database_missing_current_grave_columns_is_rejected() {
        let path = std::env::temp_dir().join(format!(
            "requiescat-missing-grave-columns-test-{}.sqlite",
            std::process::id()
        ));
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "
                CREATE TABLE requiescat_metadata (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                INSERT INTO requiescat_metadata (key, value)
                VALUES ('application', 'requiescat'), ('schema_version', '1');

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
                CREATE TABLE requiescat_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                INSERT INTO requiescat_migrations (version) VALUES (1);
                ",
            )
            .unwrap();
        drop(connection);

        assert!(SqliteCemeteryRepository::new(path.clone()).load().is_err());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_persisted_grave_gps_is_rejected() {
        let path = std::env::temp_dir().join(format!(
            "requiescat-invalid-gps-test-{}.sqlite",
            std::process::id()
        ));
        let connection = Connection::open(&path).unwrap();
        initialize_schema(&connection).unwrap();
        connection
            .execute(
                "
                INSERT INTO graves (id, x, y, width, height, color, gps)
                VALUES (1, 0.0, 0.0, 10.0, 20.0, '#a61f28', '91° 0′ 0″ N, 0° 0′ 0″ W')
                ",
                [],
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
        let grave_gps = GraveGps::parse("51° 30′ 26.64″ N, 0° 7′ 40.08″ W").unwrap();
        cemetery.update_grave_gps(grave_id, Some(grave_gps));
        cemetery.rotate_grave(grave_id, 45.0);
        let delimiter_id = cemetery.add_delimiter_with_color_and_type(
            GraveRectangle::from_top_left_size(Point::new(2.0, 4.0), Size::new(200.0, 20.0)),
            GraveColor::from_rgb8(71, 141, 86),
            DelimiterType::Road,
        );
        cemetery.rotate_delimiter(delimiter_id, 90.0);
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
        assert_eq!(
            loaded.grave(grave_id).map(Grave::gps),
            Some(Some(grave_gps))
        );
        assert_eq!(
            loaded.grave(grave_id).map(Grave::rotation_degrees),
            Some(45.0)
        );
        assert_eq!(loaded.search_people("").len(), 1);
        assert_eq!(loaded.search_people("Ada")[0].grave_id(), Some(grave_id));
        assert_eq!(loaded.delimiters().len(), 1);
        assert_eq!(loaded.delimiters()[0].delimiter_type(), DelimiterType::Road);
        assert_eq!(loaded.delimiters()[0].rotation_degrees(), 90.0);
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

# Requiescat

Requiescat is a desktop application for preserving cemetery maps and person records in a local, structured library. It helps users create cemetery files, draw and edit grave layouts, associate people with graves, record GPS coordinates, and export printable cemetery maps.

The application is designed as a local-first tool. Cemetery data is stored in SQLite files on the user's machine, with autosave support during editing and explicit export options for both database files and PDF maps.

## Features

- Create, import, open, export, and delete cemetery files from a local library.
- Draw graves and cemetery delimiters such as walls and roads.
- Move, rotate, erase, duplicate, and recolor map objects.
- Add people with birth and decease dates.
- Assign people to graves and browse/search the person directory.
- Store grave GPS coordinates in degrees, minutes, and seconds format.
- Export an A0 landscape PDF cemetery map.
- Use the interface in English or Romanian.
- Build distributable desktop packages for Windows, macOS, and Linux.

## Status

Requiescat is suitable for cautious real use and small trusted-group testing. Keep backups of cemetery files during beta use, especially before schema changes.

The current codebase has passing formatting, unit tests, clippy checks, and release builds, with coverage around domain behavior, persistence, map editing, localization, updater logic, and PDF export.

## Technical Overview

Requiescat is written in Rust and uses `iced` for the desktop interface, `rusqlite` for local persistence, and a custom PDF export pipeline. The codebase is organized around clear module boundaries:

- `models` contains domain data such as cemeteries, graves, people, dates, GPS coordinates, and map geometry.
- `persistence` owns SQLite loading, validation, migration, and saving.
- `screens` contains the start menu and map editor UI.
- `export` contains PDF rendering and writing.
- `localization` provides English and Romanian text through Fluent message catalogs.
- `updater` supports platform-specific update checking and installation flows.

## Development

Run the application:

```sh
cargo run --locked
```

Run the main verification checks:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release --bin requiescat --bin requiescat-updater
```

Print the application version:

```sh
cargo run --locked -- --version
```

## Release Notes

Before broad public distribution, the main hardening work should be:

- Add a regular CI workflow for pull requests and branch pushes.
- Reconcile the database migration documentation with the implemented migration behavior.
- Manually smoke test packaged builds on Windows, macOS, and Linux.

Release packaging details live in `packaging/RELEASING.md`.

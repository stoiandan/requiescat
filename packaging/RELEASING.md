# Releasing Requiescat

## Version and tag

1. Set the application version in `Cargo.toml`.
2. Commit the release.
3. Create and push a matching `vX.Y.Z` tag.

The desktop workflow rejects tags that do not match `Cargo.toml`. A successful
tag build publishes a Windows x86-64 MSI, a macOS Apple Silicon app archive, and
a Linux x86-64 Flatpak bundle, plus `release-manifest.json`, SHA-256 checksums,
and GitHub build-provenance attestations.

The app checks:

`https://github.com/stoiandan/requiescat/releases/latest/download/release-manifest.json`

Set `REQUIESCAT_UPDATE_MANIFEST_URL` at compile time to use another update
channel.

Update `packaging/release-notes/en.md` and
`packaging/release-notes/ro.md` before tagging a release. Their contents are
published with the release artifacts.

Published packages install both `requiescat-updater` and `requiescat`. The
platform launcher starts the updater first. It checks GitHub, verifies an
available package, starts the platform installer when an update is available,
or opens the GUI immediately when current.

On Windows, the updater downloads the MSI and hands it to Windows Installer,
which performs the upgrade and relaunches the installed application. On macOS,
the updater copies itself to Application Support first; that helper waits for
the bundle process to exit before replacing the signed app bundle. On Linux,
the updater opens the Flatpak bundle with the desktop/Flatpak tooling instead
of trying to replace files inside the sandbox.

## Signing secrets

Signing is optional for development builds. Tagged releases require these
GitHub Actions secrets for the macOS package, because unsigned downloaded
`.app` bundles can be rejected by Gatekeeper as damaged or incomplete:

- `WINDOWS_CERTIFICATE`: Base64-encoded Authenticode PFX certificate.
- `WINDOWS_CERTIFICATE_PASSWORD`: PFX password.
- `MACOS_CERTIFICATE`: Base64-encoded Developer ID Application P12 certificate.
- `MACOS_CERTIFICATE_PASSWORD`: P12 password.
- `MACOS_SIGNING_IDENTITY`: Full Developer ID Application identity.
- `APPLE_ID`: Apple account used by `notarytool`.
- `APPLE_APP_PASSWORD`: App-specific Apple password.
- `APPLE_TEAM_ID`: Apple developer team identifier.

For Windows, the `WINDOWS_CERTIFICATE` secret must contain a real
CA-issued Authenticode code-signing certificate exported as a PFX.
Self-signed certificates are only useful for local testing on machines that
explicitly trust that certificate, and they will not remove the
"Unknown publisher" or SmartScreen reputation warnings for normal users.
If you want public Windows installs to avoid those warnings, use either:

- a standard Authenticode certificate from a trusted certificate authority and
  build reputation over time, or
- an EV code-signing certificate / a managed service like Azure Trusted Signing.

When the macOS signing and Apple credentials are present, the workflow signs the
nested executables and app bundle, notarizes, staples, and assesses the app
before publishing it.

## Database migrations

Schema version 1 already includes `requiescat_metadata` and
`requiescat_migrations`.

For the first schema change:

1. Increase `CURRENT_SCHEMA_VERSION` in `src/persistence/mod.rs`.
2. Add the ordered SQL/data transformation to `migrate_schema`.
3. Allow that starting version in `ensure_migration_path`.
4. Record the completed version in both `requiescat_metadata` and
   `requiescat_migrations` inside the migration transaction.
5. Add tests for successful migration, backup creation, rollback, and rejection
   of newer unsupported schemas.

The repository creates a SQLite backup before opening a migration transaction.

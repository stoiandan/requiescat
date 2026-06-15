# Releasing Requiescat

## Version and tag

1. Set the application version in `Cargo.toml`.
2. Commit the release.
3. Create and push a matching `vX.Y.Z` tag.

The desktop workflow rejects tags that do not match `Cargo.toml`. A successful
tag build publishes Windows x86-64, macOS Apple Silicon, and Linux x86-64
archives, `release-manifest.json`, SHA-256 checksums inside that manifest, and
GitHub build-provenance attestations.

The app checks:

`https://github.com/stoiandan/requiescat/releases/latest/download/release-manifest.json`

Set `REQUIESCAT_UPDATE_MANIFEST_URL` at compile time to use another update
channel.

## Signing secrets

Signing is optional for development builds. Configure these GitHub Actions
secrets for trusted production packages:

- `WINDOWS_CERTIFICATE`: Base64-encoded Authenticode PFX certificate.
- `WINDOWS_CERTIFICATE_PASSWORD`: PFX password.
- `MACOS_CERTIFICATE`: Base64-encoded Developer ID Application P12 certificate.
- `MACOS_CERTIFICATE_PASSWORD`: P12 password.
- `MACOS_SIGNING_IDENTITY`: Full Developer ID Application identity.
- `APPLE_ID`: Apple account used by `notarytool`.
- `APPLE_APP_PASSWORD`: App-specific Apple password.
- `APPLE_TEAM_ID`: Apple developer team identifier.

When the macOS signing and Apple credentials are present, the workflow signs,
notarizes, and staples the app before publishing it.

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

//! The `.vex` backup container's own rules — what goes in it, what its
//! metadata says, and which archives this build is willing to restore.
//! database-schema.md §7/§9 (schema-level constraint), user-flows.md §9.
//! Mirrors `vunexo-billing`'s `domain::backup` field-for-field; only the
//! extension, file-name prefix, and "assets" being called `receipts/` differ.
//!
//! Pure: this module decides *what* a backup means, never how bytes reach a
//! file. Archiving and extraction are `infrastructure::filesystem::backup`.

use chrono::{DateTime, NaiveDate, Utc};

/// Bumped only when the archive's *layout* changes. Not the app version.
pub const BACKUP_FORMAT_VERSION: u32 = 1;

/// Archive member paths. Duplicated nowhere else — the writer and the reader
/// both name them from here, so they cannot drift apart.
pub const METADATA_MEMBER: &str = "metadata.json";
pub const DATABASE_MEMBER: &str = "database.sqlite";
pub const RECEIPTS_PREFIX: &str = "receipts/";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BackupMetadata {
    pub format_version: u32,
    pub app_version: String,
    pub created_at: DateTime<Utc>,
    pub platform: String,
}

impl BackupMetadata {
    pub fn new(app_version: &str, platform: &str, created_at: DateTime<Utc>) -> Self {
        Self {
            format_version: BACKUP_FORMAT_VERSION,
            app_version: app_version.to_string(),
            created_at,
            platform: platform.to_string(),
        }
    }
}

/// Why an archive can't be restored by this build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupRejection {
    FormatTooNew { found: u32, supported: u32 },
    Malformed(String),
}

impl std::fmt::Display for BackupRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupRejection::FormatTooNew { found, supported } => write!(
                f,
                "this backup was made by a newer version of Vunexo Expense Manager \
                 (backup format {found}, this app reads up to {supported}) — \
                 update the app, then restore it"
            ),
            BackupRejection::Malformed(detail) => {
                write!(
                    f,
                    "this file isn't a usable Vunexo Expense Manager backup: {detail}"
                )
            }
        }
    }
}

/// The one place that decides whether an archive is restorable here.
pub fn check_restorable(metadata: &BackupMetadata) -> Result<(), BackupRejection> {
    if metadata.format_version > BACKUP_FORMAT_VERSION {
        return Err(BackupRejection::FormatTooNew {
            found: metadata.format_version,
            supported: BACKUP_FORMAT_VERSION,
        });
    }
    Ok(())
}

/// `vunexo-expense-manager-backup-2026-09-05.vex` — offered as the save
/// dialog's default (user-flows.md §9).
pub fn backup_file_name(today: NaiveDate) -> String {
    format!(
        "vunexo-expense-manager-backup-{}.vex",
        today.format("%Y-%m-%d")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(format_version: u32) -> BackupMetadata {
        BackupMetadata {
            format_version,
            app_version: "0.0.0".to_string(),
            created_at: Utc::now(),
            platform: "macos".to_string(),
        }
    }

    #[test]
    fn the_current_format_restores() {
        assert!(check_restorable(&metadata(BACKUP_FORMAT_VERSION)).is_ok());
    }

    #[test]
    fn an_older_format_is_accepted_rather_than_rejected() {
        assert!(check_restorable(&metadata(0)).is_ok());
    }

    #[test]
    fn a_newer_format_is_refused_with_an_actionable_message() {
        let err = check_restorable(&metadata(BACKUP_FORMAT_VERSION + 1)).unwrap_err();
        assert_eq!(
            err,
            BackupRejection::FormatTooNew {
                found: BACKUP_FORMAT_VERSION + 1,
                supported: BACKUP_FORMAT_VERSION,
            }
        );
        assert!(err.to_string().contains("update the app"));
    }

    #[test]
    fn the_backup_file_name_matches_the_spec() {
        assert_eq!(
            backup_file_name(NaiveDate::from_ymd_opt(2026, 9, 5).unwrap()),
            "vunexo-expense-manager-backup-2026-09-05.vex"
        );
    }
}

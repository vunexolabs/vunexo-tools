//! The `.vex` container: a zip holding `metadata.json`, `database.sqlite`,
//! and a `receipts/` directory (database-schema.md §7/§9). Mirrors
//! `vunexo-billing`'s `VbxArchive` — the only real difference is that
//! `receipts/` can hold many files instead of one named logo asset.

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use zip::write::SimpleFileOptions;

use crate::application::ports::backup_archive::{ArchiveContents, BackupArchive};
use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::domain::backup::{BackupMetadata, DATABASE_MEMBER, METADATA_MEMBER, RECEIPTS_PREFIX};

pub struct VexArchive;

impl VexArchive {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VexArchive {
    fn default() -> Self {
        Self::new()
    }
}

fn io(context: &str, err: impl std::fmt::Display) -> InfrastructureError {
    InfrastructureError::Io(format!("{context}: {err}"))
}

impl BackupArchive for VexArchive {
    fn write(
        &self,
        destination: &Path,
        contents: ArchiveContents<'_>,
    ) -> Result<(), InfrastructureError> {
        let file = std::fs::File::create(destination)
            .map_err(|err| io(&format!("could not create {}", destination.display()), err))?;
        let mut zip = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let metadata = serde_json::to_vec_pretty(contents.metadata)
            .map_err(|err| io("could not serialize backup metadata", err))?;
        zip.start_file(METADATA_MEMBER, options)
            .map_err(|err| io("could not write metadata.json", err))?;
        zip.write_all(&metadata)
            .map_err(|err| io("could not write metadata.json", err))?;

        zip.start_file(DATABASE_MEMBER, options)
            .map_err(|err| io("could not write database.sqlite", err))?;
        let database = std::fs::read(contents.database)
            .map_err(|err| io("could not read the database snapshot", err))?;
        zip.write_all(&database)
            .map_err(|err| io("could not write database.sqlite", err))?;

        for (name, path) in contents.receipts {
            // A receipt that has gone missing since it was attached must not
            // sink the whole backup — the database is the irreplaceable part.
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            zip.start_file(format!("{RECEIPTS_PREFIX}{name}"), options)
                .map_err(|err| io("could not write a receipt", err))?;
            zip.write_all(&bytes)
                .map_err(|err| io("could not write a receipt", err))?;
        }

        zip.finish()
            .map_err(|err| io("could not finalize the backup", err))?;
        Ok(())
    }

    fn read_metadata(&self, source: &Path) -> Result<BackupMetadata, InfrastructureError> {
        let file = std::fs::File::open(source)
            .map_err(|err| io(&format!("could not open {}", source.display()), err))?;
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|err| io("this file is not a Vunexo Expense Manager backup", err))?;
        let mut entry = zip
            .by_name(METADATA_MEMBER)
            .map_err(|err| io("this backup has no metadata.json", err))?;
        let mut raw = String::new();
        entry
            .read_to_string(&mut raw)
            .map_err(|err| io("could not read metadata.json", err))?;
        serde_json::from_str(&raw).map_err(|err| io("metadata.json is not readable", err))
    }

    fn extract(
        &self,
        source: &Path,
        database_destination: &Path,
        receipts_directory: &Path,
    ) -> Result<Vec<PathBuf>, InfrastructureError> {
        let file = std::fs::File::open(source)
            .map_err(|err| io(&format!("could not open {}", source.display()), err))?;
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|err| io("this file is not a Vunexo Expense Manager backup", err))?;

        {
            let mut entry = zip
                .by_name(DATABASE_MEMBER)
                .map_err(|err| io("this backup has no database in it", err))?;
            let mut out = std::fs::File::create(database_destination).map_err(|err| {
                io(
                    &format!("could not write {}", database_destination.display()),
                    err,
                )
            })?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|err| io("could not restore the database", err))?;
        }

        let mut written = Vec::new();
        for index in 0..zip.len() {
            let mut entry = zip
                .by_index(index)
                .map_err(|err| io("could not read the backup", err))?;
            let Some(name) = entry
                .name()
                .strip_prefix(RECEIPTS_PREFIX)
                .map(str::to_string)
            else {
                continue;
            };
            // An archive is untrusted input: a member named `../../id_rsa`
            // would otherwise be written outside the data directory.
            let Some(file_name) = safe_file_name(&name) else {
                continue;
            };
            std::fs::create_dir_all(receipts_directory)
                .map_err(|err| io("could not create the receipts directory", err))?;
            let destination = receipts_directory.join(file_name);
            let mut out = std::fs::File::create(&destination)
                .map_err(|err| io("could not restore a receipt", err))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|err| io("could not restore a receipt", err))?;
            written.push(destination);
        }
        Ok(written)
    }
}

/// Reduces an archive member name to a single, ordinary file name, or `None`
/// if it tries to be anything else (absolute, parent-relative, nested, or a
/// directory entry).
fn safe_file_name(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    let mut components = path.components();
    let (Some(Component::Normal(only)), None) = (components.next(), components.next()) else {
        return None;
    };
    Some(PathBuf::from(only))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::backup::{check_restorable, BACKUP_FORMAT_VERSION};
    use chrono::Utc;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "expense-manager-vex-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_backup_round_trips_its_database_and_receipts() {
        let dir = temp_dir("roundtrip");
        let database = dir.join("snapshot.sqlite");
        std::fs::write(&database, b"SQLite format 3\0pretend").unwrap();
        let receipt_a = dir.join("a.jpg");
        std::fs::write(&receipt_a, b"receipt a bytes").unwrap();
        let receipt_b = dir.join("b.png");
        std::fs::write(&receipt_b, b"receipt b bytes").unwrap();

        let metadata = BackupMetadata::new("0.0.0", "macos", Utc::now());
        let archive_path = dir.join("backup.vex");
        let receipts = vec![
            ("a.jpg".to_string(), receipt_a),
            ("b.png".to_string(), receipt_b),
        ];
        VexArchive::new()
            .write(
                &archive_path,
                ArchiveContents {
                    metadata: &metadata,
                    database: &database,
                    receipts: &receipts,
                },
            )
            .unwrap();

        let read_back = VexArchive::new().read_metadata(&archive_path).unwrap();
        assert_eq!(read_back, metadata);
        assert!(check_restorable(&read_back).is_ok());

        let restore_dir = temp_dir("restore");
        let restored_db = restore_dir.join("expense-manager.db");
        let restored_receipts = restore_dir.join("receipts");
        let written = VexArchive::new()
            .extract(&archive_path, &restored_db, &restored_receipts)
            .unwrap();

        assert_eq!(
            std::fs::read(&restored_db).unwrap(),
            b"SQLite format 3\0pretend"
        );
        assert_eq!(written.len(), 2);
        let names: Vec<_> = written
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
            .collect();
        assert!(names.contains(&"a.jpg".to_string()));
        assert!(names.contains(&"b.png".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&restore_dir);
    }

    #[test]
    fn metadata_records_the_format_version_this_build_writes() {
        let metadata = BackupMetadata::new("1.2.3", "linux", Utc::now());
        assert_eq!(metadata.format_version, BACKUP_FORMAT_VERSION);
        assert_eq!(metadata.app_version, "1.2.3");
    }

    #[test]
    fn a_file_that_is_not_an_archive_is_refused_rather_than_half_restored() {
        let dir = temp_dir("garbage");
        let path = dir.join("not-a-backup.vex");
        std::fs::write(&path, b"just some text").unwrap();
        assert!(VexArchive::new().read_metadata(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn receipt_names_that_try_to_escape_the_receipts_directory_are_rejected() {
        assert_eq!(safe_file_name("a.jpg"), Some(PathBuf::from("a.jpg")));
        assert_eq!(safe_file_name("../../.ssh/id_rsa"), None);
        assert_eq!(safe_file_name("/etc/passwd"), None);
        assert_eq!(safe_file_name("nested/a.jpg"), None);
        assert_eq!(safe_file_name(""), None);
    }
}
